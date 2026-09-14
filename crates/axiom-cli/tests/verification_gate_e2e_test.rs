use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let unique = format!(
            "axiom_gate_test_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time should be valid")
                .as_nanos()
        );
        let path = std::env::temp_dir().join(unique);
        fs::create_dir_all(&path).expect("temporary directory should be created");
        Self { path }
    }

    fn write(&self, name: &str, content: &str) -> PathBuf {
        let path = self.path.join(name);
        fs::write(&path, content).expect("file should be written");
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

fn axiom_bin() -> &'static str {
    env!("CARGO_BIN_EXE_axiom")
}

fn axiom(args: &[&str]) -> std::process::Output {
    Command::new(axiom_bin())
        .args(args)
        .output()
        .expect("CLI binary should run")
}

fn path_text(path: &Path) -> &str {
    path.to_str().expect("path should be valid UTF-8")
}

#[test]
fn emit_rust_rejects_unverified_contract_by_default() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "buggy.ax",
        r#"
        fn buggy_sub(a: Int, b: Int) -> Int
            requires { a > 0 }
            ensures { result > 0 }
        {
            a - b
        }
        "#,
    );

    let output = axiom(&["emit-rust", path_text(&source)]);
    assert!(
        !output.status.success(),
        "emit-rust must fail on unverified contract by default"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("formal verification gate failed"),
        "expected error to mention formal verification gate, got: {stderr}"
    );
    assert!(
        stderr.contains("--unsafe-unverified"),
        "expected error to suggest --unsafe-unverified flag, got: {stderr}"
    );
}

#[test]
fn emit_rust_with_unsafe_unverified_flag_succeeds_with_warning() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "buggy_override.ax",
        r#"
        fn buggy_sub(a: Int, b: Int) -> Int
            requires { a > 0 }
            ensures { result > 0 }
        {
            a - b
        }
        "#,
    );

    let output = axiom(&["emit-rust", path_text(&source), "--unsafe-unverified"]);
    assert!(
        output.status.success(),
        "emit-rust with --unsafe-unverified must succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "// WARNING: Generated with --unsafe-unverified, contracts are NOT formally proved."
        ),
        "expected stdout to contain unsafe-unverified warning, got: {stdout}"
    );
}

#[test]
fn emit_rust_with_proved_contracts_injects_receipt() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "proved.ax",
        r#"
        fn safe_inc(x: Int) -> Int
            requires { x > 0, x < 100 }
            ensures { result > x }
        {
            x + 1
        }
        "#,
    );

    let output = axiom(&["emit-rust", path_text(&source)]);
    assert!(
        output.status.success(),
        "emit-rust on strictly proved contract must succeed by default, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout
            .contains("// [Axiom Verification Receipt] All formal contracts proved by SMT solver."),
        "expected stdout to contain verification receipt, got: {stdout}"
    );
}
