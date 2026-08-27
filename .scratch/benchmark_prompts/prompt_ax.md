# Task: Implement Roguelike Core Engine in Axiom Language

You are tasked with implementing the Roguelike Core Engine in Axiom according to the specification below.

## Rules & Skill Instructions (Axiom Language Guide)

### 1. Language Rules
- ADT Types: `type Name = { field: Type, ... }`
- Primitives: `Int`, `Float`, `Decimal`, `Bool`, `String`, `()`
- Pure Functions: `fn name(param: Type) -> RetType requires { ... } ensures { ... } { expression }`
- Pure Expressions: Variables are 100% immutable (NO `mut` keyword). Tail expression without semicolon is the return value.
- Record Update: `Player { ..p, exp: new_exp }`
- Effects: `effect Name { fn op(args): RetType; }`

### 2. Required Entities & Logic (from SPEC.md)
1. Types:
   - `Point`: `{ x: Int, y: Int }`
   - `Stats`: `{ hp: Int, max_hp: Int, attack: Int, defense: Int }`
   - `Item`: `{ id: Int, name: String, value: Int }`
   - `Player`: `{ id: Int, pos: Point, stats: Stats, exp: Int, level: Int }`
   - `Monster`: `{ id: Int, pos: Point, stats: Stats, is_alive: Bool }`
2. Effects:
   - `AudioEffect`: `fn play_sound(name: String) -> ();`
   - `RenderEffect`: `fn draw_tile(x: Int, y: Int, tile_id: Int) -> ();`
   - `RandomEffect`: `fn roll_dice(sides: Int) -> Int;`
3. Functions with Contracts:
   - `try_move(pos: Point, dx: Int, dy: Int, map_width: Int, map_height: Int) -> Point`
     - requires `map_width > 0, map_height > 0`
     - ensures `result.x >= 0 && result.x < map_width && result.y >= 0 && result.y < map_height`
   - `calculate_damage(attacker_atk: Int, defender_def: Int) -> Int`
     - requires `attacker_atk >= 0, defender_def >= 0`
     - ensures `result >= 1`
   - `apply_damage(current_hp: Int, damage: Int) -> Int`
     - requires `current_hp >= 0, damage >= 0`
     - ensures `result >= 0 && result <= current_hp`
   - `level_up(player: Player, exp_gained: Int) -> Player`
     - requires `exp_gained >= 0, player.level >= 1, player.stats.max_hp > 0`
     - ensures `result.level >= player.level && result.stats.max_hp >= player.stats.max_hp`

## Execution Steps:
1. Write the implementation into `roguelike.ax` in the current working directory.
2. Run `cargo run -p axiom-cli -- check roguelike.ax` to verify syntax.
3. Run `cargo run -p axiom-cli -- verify roguelike.ax` to verify SMT formal contracts.
