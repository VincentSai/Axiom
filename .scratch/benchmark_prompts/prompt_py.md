# Task: Implement Roguelike Core Engine in Python

Implement the Roguelike Core Engine in Python according to the specification below.

## Required Entities & Logic (from SPEC.md)
1. Dataclasses:
   - `Point`: `x: int, y: int`
   - `Stats`: `hp: int, max_hp: int, attack: int, defense: int`
   - `Item`: `id: int, name: str, value: int`
   - `Player`: `id: int, pos: Point, stats: Stats, exp: int, level: int`
   - `Monster`: `id: int, pos: Point, stats: Stats, is_alive: bool`
2. Functions:
   - `try_move(pos: Point, dx: int, dy: int, map_width: int, map_height: int) -> Point`
   - `calculate_damage(attacker_atk: int, defender_def: int) -> int`
   - `apply_damage(current_hp: int, damage: int) -> int`
   - `level_up(player: Player, exp_gained: int) -> Player`

## Execution Steps:
1. Write the code into `engine.py`.
2. Write unit tests in `test_engine.py`.
3. Run `python test_engine.py` to verify all tests pass.
