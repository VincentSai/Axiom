use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn unity_bridge_generates_native_double_buffer_asset_registry_and_runner() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "game.ax",
        r#"
        type EntitySnapshot = { id: Int, x: Float, y: Float, health: Float }
        type PlayerInput = { horizontal: Float, vertical: Float, fire: Bool }
        type AssetKind = | Player | Enemy | Projectile

        fn update(snapshot: EntitySnapshot, input: PlayerInput) -> EntitySnapshot {
            snapshot
        }
        "#,
    );
    let output_dir = temp.path().join("unity");

    let output = axiom(&[
        "bridge",
        path_text(&source),
        "--target",
        "unity",
        "--out",
        path_text(&output_dir),
    ]);
    assert_success(&output);

    let native = read_generated(&output_dir, "AxiomNative.cs");
    for symbol in [
        "axiom_init",
        "axiom_tick",
        "axiom_get_snapshot_ptr",
        "axiom_get_events_ptr",
        "axiom_destroy",
    ] {
        assert!(
            native.contains(symbol),
            "missing P/Invoke symbol `{symbol}`"
        );
    }
    assert!(native.contains("DllImport"));

    let double_buffer = read_generated(&output_dir, "AxiomDoubleBuffer.cs");
    assert!(double_buffer.contains("[StructLayout(LayoutKind.Sequential)]"));
    assert!(double_buffer.contains("struct EntitySnapshot"));
    assert!(double_buffer.contains("struct PlayerInput"));
    assert!(double_buffer.contains("class AxiomDoubleBuffer"));

    let registry = read_generated(&output_dir, "AxiomAssetRegistry.cs");
    assert!(registry.contains("ScriptableObject"));
    assert!(registry.contains("enum AssetKind"));
    assert!(registry.contains("GameObject Prefab"));
    assert!(registry.contains("AudioClip AudioClip"));

    let runner = read_generated(&output_dir, "AxiomRunner.cs");
    assert!(runner.contains("MonoBehaviour"));
    assert!(runner.contains("FixedUpdate"));
    assert!(runner.contains("AxiomNative.axiom_tick"));
    assert!(runner.contains("AxiomNative.axiom_get_snapshot_ptr"));
    assert!(runner.contains("AxiomNative.axiom_get_events_ptr"));
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

fn read_generated(directory: &Path, filename: &str) -> String {
    fs::read_to_string(directory.join(filename))
        .unwrap_or_else(|error| panic!("generated `{filename}` should be readable: {error}"))
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
            "axiom-cli-bridge-test-{}-{sequence}",
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
        fs::write(&path, contents).expect("test source should be written");
        path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}
