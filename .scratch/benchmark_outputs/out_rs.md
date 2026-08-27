Implemented the Roguelike Core Engine.

- Added [Cargo.toml](G:/Workspace/Axiom/examples/benchmark_codex/rs/Cargo.toml)
- Implemented all entities and functions in [src/lib.rs](G:/Workspace/Axiom/examples/benchmark_codex/rs/src/lib.rs)
- Added 7 integration tests in [tests/engine_test.rs](G:/Workspace/Axiom/examples/benchmark_codex/rs/tests/engine_test.rs)

Verification:

- `cargo test`: 7 tests passed
- `cargo fmt --check`: passed
- `cargo clippy --all-targets --all-features -- -D warnings`: passed