use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn faulty_contract_emits_machine_readable_counter_example() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "withdraw.ax",
        r#"
        fn withdraw(balance: Decimal, amount: Decimal) -> Decimal
            requires { balance >= 0.0, amount > 0.0 }
            ensures { result >= 0.0 }
        {
            balance - amount
        }
        "#,
    );

    let output = axiom(&["verify", path_text(&source), "--json"]);

    assert!(!output.status.success(), "faulty contract was accepted");
    assert!(output.stderr.is_empty(), "JSON mode polluted stderr");
    let reports: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("stdout should be valid JSON");
    assert_eq!(reports[0]["status"], "ContractViolation");
    assert_eq!(reports[0]["function_name"], "withdraw");
    assert_eq!(reports[0]["violated_clause"], "ensures: result >= 0.0");
    assert!(reports[0]["counter_example"].is_object());
}

fn axiom(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_axiom"))
        .args(arguments)
        .output()
        .expect("axiom CLI should start")
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
        let path = std::env::temp_dir().join(format!(
            "axiom-cli-self-healing-test-{}-{sequence}",
            std::process::id()
        ));
        if path.exists() {
            fs::remove_dir_all(&path).expect("stale test directory should be removable");
        }
        fs::create_dir(&path).expect("test directory should be created");
        Self { path }
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
