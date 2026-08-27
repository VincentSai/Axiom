# Task: Implement Roguelike Core Engine in Rust

Implement the Roguelike Core Engine in Rust according to the specification below.

## Required Entities & Logic (from SPEC.md)
1. Structs:
   - `Point`: `pub struct Point { pub x: i64, pub y: i64 }`
   - `Stats`: `pub struct Stats { pub hp: i64, pub max_hp: i64, pub attack: i64, pub defense: i64 }`
   - `Item`: `pub struct Item { pub id: i64, pub name: String, pub value: i64 }`
   - `Player`: `pub struct Player { pub id: i64, pub pos: Point, pub stats: Stats, pub exp: i64, pub level: i64 }`
   - `Monster`: `pub struct Monster { pub id: i64, pub pos: Point, pub stats: Stats, pub is_alive: bool }`
2. Functions:
   - `pub fn try_move(pos: &Point, dx: i64, dy: i64, map_width: i64, map_height: i64) -> Point`
   - `pub fn calculate_damage(attacker_atk: i64, defender_def: i64) -> i64`
   - `pub fn apply_damage(current_hp: i64, damage: i64) -> i64`
   - `pub fn level_up(player: &Player, exp_gained: i64) -> Player`

## Execution Steps:
1. Create `Cargo.toml` with `[workspace]` table and `src/lib.rs`.
2. Write unit tests in `tests/engine_test.rs`.
3. Run `cargo test` to verify all tests pass.
