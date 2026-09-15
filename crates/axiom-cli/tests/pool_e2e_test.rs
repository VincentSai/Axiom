use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

fn temp_dir(prefix: &str) -> PathBuf {
    let nonce = NEXT_TEMP_DIR.fetch_add(1, Ordering::SeqCst);
    let path = std::env::temp_dir().join(format!(
        "axiom-cli-pool-test-{prefix}-{}-{nonce}",
        std::process::id()
    ));
    fs::create_dir_all(&path).expect("temp dir should be created");
    path
}

fn axiom_bin() -> PathBuf {
    let mut path = std::env::current_exe().expect("test binary path should exist");
    path.pop();
    if path.ends_with("deps") {
        path.pop();
    }
    let exe_name = if cfg!(windows) { "axiom.exe" } else { "axiom" };
    path.push(exe_name);
    assert!(path.exists(), "axiom binary must exist at {:?}", path);
    path
}

#[test]
fn pool_cli_status_submit_and_assemble_flow() {
    let dir = temp_dir("cli-pool");
    let skeleton_path = dir.join("math_skeleton.ax");
    let skeleton_content = r#"
type Point = {
    x: Int,
    y: Int,
}

fn increment(x: Int) -> Int
    requires {
        x >= 0,
        x < 1000000,
    }
    ensures {
        result > x,
    }
{
    ?inc_hole
}
"#;
    fs::write(&skeleton_path, skeleton_content).expect("skeleton write should succeed");

    // 1. Test xiom pool status
    let status_output = Command::new(axiom_bin())
        .args(["pool", "status", skeleton_path.to_str().unwrap()])
        .output()
        .expect("pool status should execute");
    assert!(status_output.status.success(), "status should exit 0");
    let stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(stdout.contains("Axiom Task Pool"));
    assert!(stdout.contains("?inc_hole"));
    assert!(stdout.contains("increment"));

    // 2. Test xiom pool submit with an invalid candidate violating contracts
    let submit_bad = Command::new(axiom_bin())
        .args([
            "pool",
            "submit",
            skeleton_path.to_str().unwrap(),
            "--hole",
            "inc_hole",
            "--expr",
            "x - 1",
        ])
        .output()
        .expect("pool submit bad should execute");
    assert!(
        !submit_bad.status.success(),
        "submit bad should exit with non-zero"
    );
    let stderr_or_stdout = format!(
        "{}\n{}",
        String::from_utf8_lossy(&submit_bad.stdout),
        String::from_utf8_lossy(&submit_bad.stderr)
    );
    assert!(stderr_or_stdout.contains("counter-example"));

    // 3. Test xiom pool submit with a valid candidate satisfying contracts
    let submit_good = Command::new(axiom_bin())
        .args([
            "pool",
            "submit",
            skeleton_path.to_str().unwrap(),
            "--hole",
            "inc_hole",
            "--expr",
            "x + 1",
        ])
        .output()
        .expect("pool submit good should execute");
    assert!(
        submit_good.status.success(),
        "submit good should exit with 0"
    );
    let good_stdout = String::from_utf8_lossy(&submit_good.stdout);
    assert!(good_stdout.contains("[ACCEPTED]"));

    // 4. Test xiom pool assemble with a solutions file
    let sol_file = dir.join("solutions.json");
    fs::write(&sol_file, r#"{"inc_hole": "x + 2"}"#).expect("solutions write should succeed");
    let out_file = dir.join("assembled.ax");

    let assemble_output = Command::new(axiom_bin())
        .args([
            "pool",
            "assemble",
            skeleton_path.to_str().unwrap(),
            "--solutions",
            sol_file.to_str().unwrap(),
            "--output",
            out_file.to_str().unwrap(),
        ])
        .output()
        .expect("pool assemble should execute");
    assert!(
        assemble_output.status.success(),
        "assemble should exit with 0"
    );
    assert!(out_file.exists(), "assembled file should be created");
    let assembled_content =
        fs::read_to_string(&out_file).expect("assembled file should be readable");
    assert!(assembled_content.contains("x + 2"));
    assert!(!assembled_content.contains("?inc_hole"));

    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn pool_cli_auto_heal_solve_flow() {
    let dir = temp_dir("cli-solve");
    let skeleton_path = dir.join("abs_skeleton.ax");
    let skeleton_content = r#"
fn safe_abs(x: Int) -> Int
    requires {
        x > -1000000,
        x < 1000000,
    }
    ensures {
        result >= 0,
    }
{
    ?abs_hole
}
"#;
    fs::write(&skeleton_path, skeleton_content).expect("skeleton write should succeed");

    // 运行 axiom pool solve --file skeleton.ax --hole abs_hole --engine mock --max-steps 3 --json
    let solve_output = Command::new(axiom_bin())
        .args([
            "pool",
            "solve",
            skeleton_path.to_str().unwrap(),
            "--hole",
            "abs_hole",
            "--engine",
            "mock",
            "--max-steps",
            "3",
            "--json",
        ])
        .output()
        .expect("pool solve should execute");

    assert!(
        solve_output.status.success(),
        "solve command should succeed with exit code 0. stderr: {}",
        String::from_utf8_lossy(&solve_output.stderr)
    );

    let stdout = String::from_utf8_lossy(&solve_output.stdout);
    let outcome: axiom_agent_pool::auto_heal::AutoHealOutcome =
        serde_json::from_str(&stdout).expect("output should deserialize into AutoHealOutcome");

    assert!(outcome.success, "auto-heal outcome should be success");
    assert_eq!(outcome.hole_id, "abs_hole");
    assert_eq!(outcome.total_steps, 2);
    assert_eq!(outcome.history.len(), 2);
    assert!(!outcome.history[0].proved);
    assert!(outcome.history[0].counter_example.is_some());
    assert!(outcome.history[1].proved);
    assert!(outcome.history[1].counter_example.is_none());

    let _ = fs::remove_dir_all(&dir);
}
