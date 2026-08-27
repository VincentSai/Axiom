/**
 * Tier 4 turn orchestration and world simulation.
 *
 * Every exported function is pure and composes lower engine tiers to preserve
 * the five-layer dependency topology required by the benchmark.
 */

import * as spatial from './tier0_spatial.js';
import * as stats from './tier1_stats.js';
import * as combat from './tier2_combat.js';
import * as aiLoot from './tier3_ai_loot.js';

const WORLD_CLOCK_MAX = 1_000_000;

function requireIntegerInRange(
  value: number,
  name: string,
  minimum: number,
  maximum: number,
): void {
  if (!Number.isInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(
      `${name} must be an integer in [${minimum}, ${maximum}]`,
    );
  }
}

function validateWorldClock(turn_num: number, cycle_len: number): void {
  requireIntegerInRange(turn_num, 'turn_num', 0, WORLD_CLOCK_MAX);
  requireIntegerInRange(cycle_len, 'cycle_len', 1, WORLD_CLOCK_MAX);
}

export function determine_turn_initiative(
  agility: number,
  roll_val: number,
): number {
  requireIntegerInRange(agility, 'agility', 0, WORLD_CLOCK_MAX);
  requireIntegerInRange(roll_val, 'roll_val', 0, 19);
  const baseInitiative = stats.calc_phys_atk_from_str(agility, 0);
  return baseInitiative + roll_val;
}

export function resolve_boss_phase_transition(hp_pct: number): number {
  requireIntegerInRange(hp_pct, 'hp_pct', 0, 100);

  const phase = hp_pct > 66 ? 1 : hp_pct >= 33 ? 2 : 3;
  return spatial.calc_grid_index(phase, 0, phase + 1);
}

export function simulate_single_turn_exchange(
  actor_atk: number,
  target_def: number,
  is_moving: boolean,
): number {
  requireIntegerInRange(actor_atk, 'actor_atk', 0, WORLD_CLOCK_MAX);
  requireIntegerInRange(target_def, 'target_def', 0, WORLD_CLOCK_MAX);

  const directDamage = combat.resolve_physical_hit(actor_atk, target_def);
  const movementDistance = Number(is_moving);
  const gridDistance = spatial.calc_manhattan_dist(
    0,
    0,
    movementDistance,
    0,
  );
  const bleedDamage = combat.apply_bleed_stack_tick(1, is_moving);
  const observedThreat = aiLoot.calc_threat_decay_by_dist(
    directDamage,
    gridDistance,
  );
  return directDamage + bleedDamage + observedThreat;
}

export function resolve_terrain_hazard_tick(
  hazard_type: number,
  max_hp: number,
): number {
  requireIntegerInRange(hazard_type, 'hazard_type', 0, 3);
  requireIntegerInRange(max_hp, 'max_hp', 0, WORLD_CLOCK_MAX);

  if (hazard_type === 0 || hazard_type === 3) {
    return 0;
  }
  if (hazard_type === 1) {
    return stats.calc_lifesteal_conversion(max_hp, 5) + 10;
  }
  return combat.apply_poison_exponential_decay(Math.trunc(max_hp / 100), 1);
}

export function world_engine_185(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const completedCycles = Math.trunc(turn_num / cycle_len);
  const checkpointWidth = Math.max(completedCycles + 1, cycle_len);
  return spatial.calc_grid_index(completedCycles, 0, checkpointWidth);
}

export function world_engine_186(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const completedCycles = Math.trunc(turn_num / cycle_len);
  return aiLoot.ai_loot_subsys_148(completedCycles, 0);
}

export function world_engine_187(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  return spatial.pack_coords_to_hash(turn_num, cycle_len);
}

export function world_engine_188(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const cyclePosition = spatial.wrap_map_coord_x(turn_num, cycle_len);
  return Math.trunc((cyclePosition * 4) / cycle_len);
}

export function world_engine_189(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const cyclePosition = spatial.wrap_map_coord_y(turn_num, cycle_len);
  return cyclePosition === 0 ? 0 : cycle_len - cyclePosition;
}

export function world_engine_190(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const worldDay = Math.trunc(turn_num / cycle_len);
  return aiLoot.calc_shop_inflation_price(cycle_len, worldDay);
}

export function world_engine_191(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const cyclePosition = spatial.wrap_map_coord_x(turn_num, cycle_len);
  const hpPct = 100 - Math.trunc((cyclePosition * 100) / cycle_len);
  return resolve_boss_phase_transition(hpPct);
}

export function world_engine_192(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const hazardType = spatial.wrap_map_coord_x(turn_num, 4);
  const worldAge = Math.trunc(turn_num / cycle_len);
  const scaledMaxHp = stats.calc_max_hp_from_vit(cycle_len, worldAge, 0);
  return resolve_terrain_hazard_tick(hazardType, scaledMaxHp);
}

export function world_engine_193(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const agility = Math.trunc(turn_num / cycle_len);
  const rollVal = spatial.wrap_map_coord_x(turn_num, 20);
  return determine_turn_initiative(agility, rollVal);
}

export function world_engine_194(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const cyclePosition = spatial.wrap_map_coord_x(turn_num, cycle_len);
  const distance = spatial.calc_manhattan_dist(0, 0, cyclePosition, 0);
  const rawThreat = aiLoot.ai_loot_subsys_149(
    cycle_len,
    Math.trunc(turn_num / cycle_len),
  );
  return aiLoot.calc_threat_decay_by_dist(rawThreat, distance);
}

export function world_engine_195(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const reducedProduct = ((turn_num % 10_000) * (cycle_len % 10_000)) % 10_000;
  const rawRoll = spatial.wrap_map_coord_x(reducedProduct, 10_000);
  const luckyRoll = aiLoot.scale_drop_by_luck(
    rawRoll,
    Math.trunc(turn_num / cycle_len),
  );
  const boundedRoll = spatial.clamp_to_bounds_x(luckyRoll, 10_000);
  return aiLoot.calc_weighted_loot_tier(boundedRoll);
}

export function world_engine_196(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const rawLevel = Math.trunc(turn_num / cycle_len);
  const cappedLevel = spatial.clamp_to_bounds_x(rawLevel, WORLD_CLOCK_MAX) + 1;
  return aiLoot.calc_exp_to_next_level(cappedLevel);
}

export function world_engine_197(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const cyclePosition = spatial.wrap_map_coord_x(turn_num, cycle_len);
  const actorAtk = stats.calc_phys_atk_from_str(
    Math.trunc(turn_num / cycle_len),
    cyclePosition,
  );
  const targetDef = stats.calc_armor_damage_reduction(
    cycle_len,
    cyclePosition,
  );
  return simulate_single_turn_exchange(actorAtk, targetDef, cyclePosition > 0);
}

export function world_engine_198(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const checkpointHash = spatial.pack_coords_to_hash(turn_num, cycle_len);
  return (
    spatial.unpack_hash_x(checkpointHash) +
    spatial.unpack_hash_y(checkpointHash)
  );
}

export function world_engine_199(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const clockPhase = world_engine_188(turn_num, cycle_len);
  const rewardTier = world_engine_195(turn_num, cycle_len);
  const encounterThreat = world_engine_194(turn_num, cycle_len);
  return clockPhase + rewardTier + encounterThreat;
}

export function world_engine_200(turn_num: number, cycle_len: number): number {
  validateWorldClock(turn_num, cycle_len);
  const initiative = world_engine_193(turn_num, cycle_len);
  const combatResult = world_engine_197(turn_num, cycle_len);
  const hazardResult = world_engine_192(turn_num, cycle_len);
  const worldScore = world_engine_199(turn_num, cycle_len);
  return initiative + combatResult + hazardResult + worldScore;
}
