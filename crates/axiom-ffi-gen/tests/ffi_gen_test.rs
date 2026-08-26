use std::fs;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use axiom_ffi_gen::{
    generate_c_abi_rust_glue, generate_c_header_from_source, generate_typescript_from_source,
};
use axiom_parser::parse_module;

const ADT_SOURCE: &str = r#"
    type Player = { id: Int, score: Float, active: Bool }
    type Event = | Joined(player: Player) | Scored(points: Int) | Left
    fn update(player: Player, event: Event) -> Player { player }
"#;

#[test]
fn generates_typescript_records_discriminated_unions_and_functions() {
    let bindings =
        generate_typescript_from_source(ADT_SOURCE).expect("TypeScript bindings should generate");

    assert!(bindings.contains("export interface Player"));
    assert!(bindings.contains("readonly id: number;"));
    assert!(bindings.contains("readonly active: boolean;"));
    assert!(bindings.contains("export type Event ="));
    assert!(bindings.contains("readonly kind: \"Joined\"; readonly player: Player"));
    assert!(bindings.contains("readonly kind: \"Left\""));
    assert!(
        bindings.contains("export declare function update(player: Player, event: Event): Player;")
    );
}

#[test]
fn generates_guarded_c_header_with_primitives_and_cpp_linkage() {
    let header = generate_c_header_from_source(ADT_SOURCE, "AXIOM_GAME_BINDINGS_H")
        .expect("C header should generate");

    assert!(header.contains("#ifndef AXIOM_GAME_BINDINGS_H"));
    assert!(header.contains("#define AXIOM_GAME_BINDINGS_H"));
    assert!(header.contains("#include <stdint.h>"));
    assert!(header.contains("#include <stdbool.h>"));
    assert!(header.contains("int64_t id;"));
    assert!(header.contains("double score;"));
    assert!(header.contains("bool active;"));
    assert!(header.contains("extern \"C\""));
    assert!(header.contains("Player update(Player player, Event event);"));
    assert!(header.contains("#endif // AXIOM_GAME_BINDINGS_H"));
}

#[test]
fn game_loop_c_abi_rust_glue_compiles() {
    let source = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/game_loop.ax"),
    )
    .expect("game loop example should be readable");
    let module = parse_module(&source).expect("game loop example should parse");
    let glue = generate_c_abi_rust_glue(&module).expect("Rust glue should generate");

    assert!(glue.contains("#[repr(C)]"));
    assert!(glue.contains("pub extern \"C\" fn axiom_tick_frame"));
    assert!(glue.contains("AxiomSlice<Entity>"));

    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should follow the Unix epoch")
        .as_nanos();
    let temp_dir =
        std::env::temp_dir().join(format!("axiom-ffi-gen-{}-{nonce}", std::process::id()));
    fs::create_dir_all(&temp_dir).expect("temporary directory should be created");
    let source_path = temp_dir.join("game_loop_ffi.rs");
    let output_path = temp_dir.join("libgame_loop_ffi.rlib");
    fs::write(&source_path, glue).expect("generated glue should be written");

    let result = Command::new("rustc")
        .arg("--edition=2021")
        .arg("--crate-type=lib")
        .arg("--deny=warnings")
        .arg(&source_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .expect("rustc should run");

    let _ = fs::remove_dir_all(&temp_dir);
    assert!(
        result.status.success(),
        "generated Rust glue did not compile:\n{}",
        String::from_utf8_lossy(&result.stderr)
    );
}
