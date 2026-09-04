use std::path::PathBuf;
use std::process::Command;

#[test]
fn test_survivor_core_verifies_without_regression() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .expect("crates dir")
        .parent()
        .expect("workspace root");

    let survivor_ax = workspace_root.join("examples/game_survivor/ax/survivor_core.ax");
    assert!(survivor_ax.exists(), "survivor_core.ax must exist");

    let bin = env!("CARGO_BIN_EXE_axiom");
    let output = Command::new(bin)
        .arg("verify")
        .arg(&survivor_ax)
        .output()
        .expect("axiom verify should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        output.status.success(),
        "axiom verify on survivor_core.ax must succeed! stdout:\n{}\nstderr:\n{}",
        stdout,
        stderr
    );

    assert!(
        !stdout.contains("[TYPE_ERROR]"),
        "must not contain any type errors"
    );
    assert!(
        !stdout.contains("[PARSE_ERROR]"),
        "must not contain any parse errors"
    );
    assert!(
        !stdout.contains("[VIOLATION]"),
        "must not contain contract violations"
    );
}

#[test]
fn test_strict_mode_rejects_unproved_contract() {
    let bin = env!("CARGO_BIN_EXE_axiom");
    // Under --strict mode, when Z3 is not present, Sampled functions must exit with non-zero
    let temp_dir = std::env::temp_dir().join(format!("axiom_strict_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let ax_file = temp_dir.join("test.ax");
    std::fs::write(
        &ax_file,
        r#"
fn contracted_add(x: Int) -> Int
    requires { x >= 0 }
    ensures { result > x }
{
    x + 1
}
"#,
    )
    .unwrap();

    let output = Command::new(bin)
        .arg("verify")
        .arg(&ax_file)
        .arg("--strict")
        .output()
        .expect("run verify --strict");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // If external Z3 is installed, it is PROVED (exit 0).
    // If external Z3 is NOT installed, it is SAMPLED_FAIL (exit 1).
    // In either case, it must not report false positive or crash!
    if stdout.contains("[PROVED]") {
        assert!(output.status.success());
    } else {
        assert!(!output.status.success(), "Strict mode must fail when Z3 solver is unavailable");
        assert!(stdout.contains("[SAMPLED_FAIL]"));
    }

    let _ = std::fs::remove_dir_all(temp_dir);
}

#[test]
fn test_default_mode_rejects_skipped_contracted_function_fail_closed() {
    let bin = env!("CARGO_BIN_EXE_axiom");
    let temp_dir =
        std::env::temp_dir().join(format!("axiom_fail_closed_test_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let ax_file = temp_dir.join("test_skipped.ax");
    // String parameter is not supported by numeric SMT, so it will be Skipped
    std::fs::write(
        &ax_file,
        r#"
fn contracted_string_fn(s: String) -> Int
    requires { s == "hello" }
    ensures { result >= 0 }
{
    0
}
"#,
    )
    .unwrap();

    let output = Command::new(bin)
        .arg("verify")
        .arg(&ax_file)
        .output()
        .expect("run verify in default mode");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        !output.status.success(),
        "Default mode MUST fail-closed when a contracted function is skipped! stdout:\n{}",
        stdout
    );
    assert!(stdout.contains("[SKIPPED_FAIL]"));

    let _ = std::fs::remove_dir_all(temp_dir);
}

