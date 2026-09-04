use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use axiom_agent_pool::{AgentTaskPool, SmtResult};
use axiom_db::AstDatabase;
use axiom_hole_extractor::extract_holes;
use axiom_parser::parse_module;
use axiom_transpiler::transpile_to_rust;
use axiom_vfs::export_database_module;

const HOLE_COUNT: usize = 20;
static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn twenty_workers_verify_and_publish_a_hole_free_release() {
    let skeleton = twenty_hole_skeleton();
    let pool = Arc::new(Mutex::new(
        AgentTaskPool::from_source(&skeleton).expect("20-hole skeleton should parse"),
    ));

    let tasks = pool
        .lock()
        .expect("task pool lock should not be poisoned")
        .dispatch_tasks();
    assert_eq!(tasks.len(), HOLE_COUNT);
    assert_eq!(
        tasks
            .iter()
            .map(|task| task.hole_id.clone())
            .collect::<Vec<_>>(),
        (1..=HOLE_COUNT)
            .map(|index| format!("hole_{index:02}"))
            .collect::<Vec<_>>()
    );

    let workers = tasks
        .into_iter()
        .enumerate()
        .map(|(index, task)| {
            let pool = Arc::clone(&pool);
            thread::spawn(move || {
                let rejected = if index == 0 {
                    Some(
                        pool.lock()
                            .expect("task pool lock should not be poisoned")
                            .submit_solution(&task.hole_id, "value - 1")
                            .expect("invalid candidate should still parse"),
                    )
                } else {
                    None
                };
                let accepted = pool
                    .lock()
                    .expect("task pool lock should not be poisoned")
                    .submit_solution(&task.hole_id, "value + 1")
                    .expect("worker candidate should parse");
                (rejected, accepted)
            })
        })
        .collect::<Vec<_>>();

    for (index, worker) in workers.into_iter().enumerate() {
        let (rejected, accepted) = worker.join().expect("worker should finish");
        if index == 0 {
            assert!(
                matches!(rejected, Some(SmtResult::CounterExample(_))),
                "SMT must reject the deliberately invalid worker solution"
            );
        } else {
            assert!(rejected.is_none());
        }
        assert!(
            matches!(accepted, SmtResult::Proved { .. } | SmtResult::Sampled { .. }),
            "accepted result must be Proved or Sampled"
        );
    }

    let release = {
        let pool = pool.lock().expect("task pool lock should not be poisoned");
        assert!(pool.is_complete());
        assert!(pool.pending_hole_ids().is_empty());
        pool.assemble_verified_module()
            .expect("all verified solutions should assemble")
    };
    assert_eq!(release.functions.len(), HOLE_COUNT);
    assert!(extract_holes(&release).is_empty());

    let temp = TestDirectory::new();
    let database_path = temp.path().join("release-astdb.json");
    let release_hash = {
        let mut database = AstDatabase::open(&database_path).expect("release database should open");
        database
            .insert_module(&release)
            .expect("release AST should be stored")
            .module_hash
    };
    let database = AstDatabase::open(&database_path).expect("release database should reopen");
    let exported = export_database_module(&database, &release_hash)
        .expect("release AST should export as source");
    assert_eq!(
        parse_module(&exported).expect("exported release should parse"),
        release
    );

    let mut rust = transpile_to_rust(&release).expect("release should transpile to Rust");
    rust.push_str("\nfn main() {\n");
    for index in 1..=HOLE_COUNT {
        writeln!(rust, "    assert_eq!(worker_{index:02}(41), 42);")
            .expect("writing Rust harness to a String cannot fail");
    }
    rust.push_str("}\n");
    let rust_path = temp.write("release.rs", &rust);
    let executable = temp
        .path()
        .join(format!("release{}", std::env::consts::EXE_SUFFIX));
    let compile = Command::new("rustc")
        .args(["--edition=2021"])
        .arg(&rust_path)
        .arg("-o")
        .arg(&executable)
        .output()
        .expect("rustc should start");
    assert_success(&compile);
    let run = Command::new(executable)
        .output()
        .expect("compiled release should start");
    assert_success(&run);
}

fn twenty_hole_skeleton() -> String {
    let mut source = String::new();
    for index in 1..=HOLE_COUNT {
        writeln!(
            source,
            r#"
fn worker_{index:02}(value: Int) -> Int
    requires {{ value >= 0 }}
    ensures {{ result > value }}
{{
    ?hole_{index:02}
}}
"#
        )
        .expect("writing Axiom skeleton to a String cannot fail");
    }
    source
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "axiom-multi-agent-e2e-{}-{sequence}",
            std::process::id()
        ));
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
        fs::write(&path, contents).expect("test file should be written");
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
