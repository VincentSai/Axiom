use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn extracts_transfer_implementation_as_schema_compliant_json() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "transfer.ax",
        r#"
        type Account = { id: Int, balance: Decimal }

        fn transfer(from: Account, to: Account, amount: Decimal) -> Account
            requires { amount > 0.0 }
            ensures { result.balance >= 0.0 }
        {
            ?transfer_implementation
        }
        "#,
    );
    let out_dir = temp.path().join("tasks");

    let output = axiom(&[
        "extract-holes",
        path_text(&source),
        "--out-dir",
        path_text(&out_dir),
    ]);
    assert_success(&output);

    let task: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(out_dir.join("transfer_implementation.json"))
            .expect("hole task should be written"),
    )
    .expect("hole task should contain valid JSON");
    assert_eq!(
        task,
        serde_json::json!({
            "$schema": "https://axiom-lang.org/schemas/hole-task.json",
            "hole_id": "transfer_implementation",
            "target_function": "transfer",
            "parameters": [
                { "name": "from", "type": "Account" },
                { "name": "to", "type": "Account" },
                { "name": "amount", "type": "Decimal" }
            ],
            "return_type": "Account",
            "allowed_effects": [],
            "contracts": {
                "requires": ["amount > 0.0"],
                "ensures": ["result.balance >= 0.0"]
            }
        })
    );
}

#[test]
fn filled_axiom_source_emits_rust_that_compiles_and_runs() {
    let temp = TestDirectory::new();
    let source = temp.write("answer.ax", "fn answer() -> Int { 40 + 2 }\n");
    let generated = temp.path().join("generated.rs");

    let output = axiom(&["emit-rust", path_text(&source), "-o", path_text(&generated)]);
    assert_success(&output);

    let mut rust = fs::read_to_string(&generated).expect("generated Rust should be readable");
    rust.push_str("\nfn main() { assert_eq!(answer(), 42); }\n");
    fs::write(&generated, rust).expect("test harness should be appended");
    let executable = temp
        .path()
        .join(format!("generated{}", std::env::consts::EXE_SUFFIX));
    let compile = Command::new("rustc")
        .args(["--edition=2021"])
        .arg(&generated)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("rustc should start");
    assert_success(&compile);
    let run = Command::new(executable)
        .output()
        .expect("generated program should start");
    assert_success(&run);
}

#[test]
fn check_extract_holes_and_emit_rust_report_invalid_inputs() {
    let temp = TestDirectory::new();
    let invalid = temp.write("invalid.ax", "fn broken( -> Int { 1 }");
    for command in ["check", "extract-holes", "emit-rust"] {
        let output = axiom(&[command, path_text(&invalid)]);
        assert!(!output.status.success(), "{command} unexpectedly succeeded");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("error: could not"),
            "unexpected {command} diagnostic: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let unresolved = temp.write("unresolved.ax", "fn pending() -> Int { ?implementation }");
    let output = axiom(&["emit-rust", path_text(&unresolved)]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unresolved hole `?implementation`"));
}

#[test]
fn cli_happy_paths_cover_check_stdout_json_emit_and_run() {
    let temp = TestDirectory::new();
    let with_hole = temp.write(
        "pending.ax",
        "fn pending(value: Int) -> Int { ?implementation }",
    );

    let check = axiom(&["check", path_text(&with_hole)]);
    assert_success(&check);
    assert!(String::from_utf8_lossy(&check.stdout).contains("1 unresolved hole(s)"));

    let extract = axiom(&["extract-holes", path_text(&with_hole)]);
    assert_success(&extract);
    let tasks: serde_json::Value =
        serde_json::from_slice(&extract.stdout).expect("stdout should be machine-readable JSON");
    assert_eq!(tasks[0]["hole_id"], "implementation");

    let runnable = temp.write("main.ax", "fn main() -> () { let answer = 40 + 2; () }");
    let emit = axiom(&["emit-rust", path_text(&runnable)]);
    assert_success(&emit);
    assert!(String::from_utf8_lossy(&emit.stdout).contains("pub fn main()"));

    let run = axiom(&["run", path_text(&runnable)]);
    assert_success(&run);
}

#[test]
fn db_put_and_export_round_trip_a_content_addressed_module() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "stored.ax",
        r#"
        type Answer = { value: Int }

        fn answer() -> Answer {
            Answer { value: 40 + 2 }
        }
        "#,
    );
    let database = temp.path().join("nested").join("astdb.json");

    let put = axiom(&[
        "db",
        "put",
        path_text(&source),
        "--db",
        path_text(&database),
    ]);
    assert_success(&put);
    let module_hash = String::from_utf8(put.stdout)
        .expect("module hash should be UTF-8")
        .trim()
        .to_owned();
    assert!(module_hash.starts_with("#mod_"));
    assert_eq!(module_hash.len(), "#mod_".len() + 64);

    let export_stdout = axiom(&["db", "export", &module_hash, "--db", path_text(&database)]);
    assert_success(&export_stdout);
    let projected = temp.write(
        "projected.ax",
        std::str::from_utf8(&export_stdout.stdout).expect("projected source should be UTF-8"),
    );

    let put_again = axiom(&[
        "db",
        "put",
        path_text(&projected),
        "--db",
        path_text(&database),
    ]);
    assert_success(&put_again);
    assert_eq!(
        String::from_utf8(put_again.stdout)
            .expect("module hash should be UTF-8")
            .trim(),
        module_hash
    );

    let output_path = temp.path().join("exported.ax");
    let export_file = axiom(&[
        "db",
        "export",
        &module_hash,
        "--db",
        path_text(&database),
        "--out",
        path_text(&output_path),
    ]);
    assert_success(&export_file);
    assert!(export_file.stdout.is_empty());
    assert_eq!(
        fs::read(&output_path).expect("exported source should be readable"),
        export_stdout.stdout
    );
}

fn axiom(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_axiom"))
        .args(arguments)
        .output()
        .expect("axiom CLI should start")
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("test path should be valid UTF-8")
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path =
            std::env::temp_dir().join(format!("axiom-cli-test-{}-{sequence}", std::process::id()));
        if path.exists() {
            fs::remove_dir_all(&path).expect("stale test directory should be removable");
        }
        fs::create_dir(&path).expect("test directory should be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn write(&self, name: &str, contents: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, contents).expect("test source should be written");
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
