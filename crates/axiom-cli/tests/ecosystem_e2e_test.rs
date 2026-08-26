use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);

#[test]
fn export_wit_writes_a_component_interface() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "profiles.ax",
        r#"
        type Profile = { id: Int, display_name: String }
        fn lookup_profile(id: Int) -> Profile { Profile { id: id, display_name: "Axiom" } }
        "#,
    );
    let generated = temp.path().join("profiles.wit");

    let output = axiom(&[
        "export-wit",
        path_text(&source),
        "--package",
        "example:profiles",
        "--output",
        path_text(&generated),
    ]);
    assert_success(&output);

    let wit = fs::read_to_string(generated).expect("generated WIT should be readable");
    assert!(wit.starts_with("package example:profiles;\n"));
    assert!(wit.contains("record profile {"));
    assert!(wit.contains("export lookup-profile: func(id: s64) -> profile;"));
}

#[test]
fn export_ffi_selects_typescript_and_c_outputs() {
    let temp = TestDirectory::new();
    let source = temp.write(
        "profiles.ax",
        r#"
        type Profile = { id: Int, display_name: String }
        fn lookup_profile(id: Int) -> Profile { Profile { id: id, display_name: "Axiom" } }
        "#,
    );
    let declarations = temp.path().join("profiles.d.ts");
    let header = temp.path().join("profiles.h");

    let typescript = axiom(&[
        "export-ffi",
        path_text(&source),
        "--lang",
        "ts",
        "--output",
        path_text(&declarations),
    ]);
    assert_success(&typescript);
    let declarations =
        fs::read_to_string(declarations).expect("TypeScript declarations should be readable");
    assert!(declarations.contains("export interface Profile"));
    assert!(declarations.contains("export declare function lookup_profile(id: number): Profile;"));

    let c = axiom(&[
        "export-ffi",
        path_text(&source),
        "--lang",
        "c",
        "--guard",
        "AXIOM_PROFILES_H",
        "--output",
        path_text(&header),
    ]);
    assert_success(&c);
    let header = fs::read_to_string(header).expect("C header should be readable");
    assert!(header.contains("#ifndef AXIOM_PROFILES_H"));
    assert!(header.contains("Profile lookup_profile(int64_t id);"));
}

#[test]
fn scaffold_creates_a_valid_project_structure() {
    let temp = TestDirectory::new();
    let output = axiom(&[
        "scaffold",
        "test_game",
        "--target",
        "web",
        "--out-dir",
        path_text(temp.path()),
    ]);
    assert_success(&output);

    let main_ax = temp.path().join("test_game").join("src").join("main.ax");
    let config = temp.path().join("test_game").join("axiom.json");

    assert!(main_ax.exists());
    assert!(config.exists());

    let check_output = axiom(&["check", path_text(&main_ax)]);
    assert_success(&check_output);

    let verify_output = axiom(&["verify", path_text(&main_ax)]);
    assert_success(&verify_output);
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
            std::env::temp_dir().join(format!("axiom-ecosystem-test-{}-{sequence}", std::process::id()));
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
