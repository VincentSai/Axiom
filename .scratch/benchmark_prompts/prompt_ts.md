# Task: Implement Roguelike Core Engine in TypeScript

Implement the Roguelike Core Engine in TypeScript according to the specification below.

## Required Entities & Logic (from SPEC.md)
1. Interfaces:
   - `Point`: `readonly x: number; readonly y: number;`
   - `Stats`: `readonly hp: number; readonly max_hp: number; readonly attack: number; readonly defense: number;`
   - `Item`: `readonly id: number; readonly name: string; readonly value: number;`
   - `Player`: `readonly id: number; readonly pos: Point; readonly stats: Stats; readonly exp: number; readonly level: number;`
   - `Monster`: `readonly id: number; readonly pos: Point; readonly stats: Stats; readonly is_alive: boolean;`
2. Functions:
   - `export function tryMove(pos: Point, dx: number, dy: number, mapWidth: number, mapHeight: number): Point`
   - `export function calculateDamage(attackerAtk: number, defenderDef: number): number`
   - `export function applyDamage(currentHp: number, damage: number): number`
   - `export function levelUp(player: Player, expGained: number): Player`

## Execution Steps:
1. Write the code to `engine.ts`.
2. Write tests in `test_engine.ts`.
