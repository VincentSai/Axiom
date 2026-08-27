//! Tier 4 turn orchestration and world simulation.
//!
//! These pure calculations deliberately compose the lower four tiers so that
//! the crate preserves the dependency topology described by the benchmark.

use crate::tier0_spatial::*;
use crate::tier1_stats::*;
use crate::tier2_combat::*;
use crate::tier3_ai_loot::*;

/// Calculates initiative from agility and a pre-generated d20-style roll.
pub fn determine_turn_initiative(agility: i64, roll_val: i64) -> i64 {
    assert!(agility >= 0);
    assert!((0..20).contains(&roll_val));

    let base_initiative = calc_phys_atk_from_str(agility, 0);
    base_initiative + roll_val
}

/// Encodes the active boss phase: P1 above 66%, P2 at 33%-66%, P3 below 33%.
pub fn resolve_boss_phase_transition(hp_pct: i64) -> i64 {
    assert!((0..=100).contains(&hp_pct));

    let phase = if hp_pct > 66 {
        1
    } else if hp_pct >= 33 {
        2
    } else {
        3
    };
    calc_grid_index(phase, 0, 1)
}

/// Resolves one attack, its movement-sensitive bleed tick, and observed threat.
pub fn simulate_single_turn_exchange(actor_atk: i64, target_def: i64, is_moving: bool) -> i64 {
    assert!(actor_atk >= 0);
    assert!(target_def >= 0);

    let direct_damage = resolve_physical_hit(actor_atk, target_def);
    let movement_distance = i64::from(is_moving);
    let grid_distance = calc_manhattan_dist(0, 0, movement_distance, 0);
    let bleed_damage = apply_bleed_stack_tick(1, is_moving);
    let observed_threat = calc_threat_decay_by_dist(direct_damage, grid_distance);
    direct_damage + bleed_damage + observed_threat
}

/// Resolves one terrain tick (0=safe, 1=lava, 2=poison marsh, 3=ice).
pub fn resolve_terrain_hazard_tick(hazard_type: i64, max_hp: i64) -> i64 {
    assert!((0..=3).contains(&hazard_type));
    assert!(max_hp >= 0);

    match hazard_type {
        0 => 0,
        1 => calc_lifesteal_conversion(max_hp, 5) + 10,
        2 => apply_poison_exponential_decay(max_hp / 100, 1),
        3 => 0,
        _ => unreachable!("hazard type is validated above"),
    }
}

/// Counts completed world cycles and maps the count into checkpoint space.
pub fn world_engine_185(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let completed_cycles = turn_num / cycle_len;
    calc_grid_index(completed_cycles, 0, cycle_len)
}

/// Produces the monotonically increasing garbage-collection generation.
pub fn world_engine_186(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let completed_cycles = turn_num / cycle_len;
    ai_loot_subsys_148(completed_cycles, 0)
}

/// Computes a deterministic save-slot hash for a turn/cycle pair.
pub fn world_engine_187(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    pack_coords_to_hash(turn_num, cycle_len)
}

/// Resolves a four-phase day/night clock from the current cycle position.
pub fn world_engine_188(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let cycle_position = wrap_map_coord_x(turn_num, cycle_len);
    cycle_position * 4 / cycle_len
}

/// Returns turns remaining until the next cycle boundary (zero on a boundary).
pub fn world_engine_189(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let cycle_position = wrap_map_coord_y(turn_num, cycle_len);
    if cycle_position == 0 {
        0
    } else {
        cycle_len - cycle_position
    }
}

/// Applies world-day inflation to a cycle-sized baseline shop price.
pub fn world_engine_190(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let world_day = turn_num / cycle_len;
    calc_shop_inflation_price(cycle_len, world_day)
}

/// Derives a descending boss-health clock and resolves its active phase.
pub fn world_engine_191(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let cycle_position = wrap_map_coord_x(turn_num, cycle_len);
    let hp_pct = 100 - cycle_position * 100 / cycle_len;
    resolve_boss_phase_transition(hp_pct)
}

/// Rotates terrain hazards while scaling maximum HP by elapsed cycles.
pub fn world_engine_192(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let hazard_type = wrap_map_coord_x(turn_num, 4);
    let world_age = turn_num / cycle_len;
    let scaled_max_hp = calc_max_hp_from_vit(cycle_len, world_age, 0);
    resolve_terrain_hazard_tick(hazard_type, scaled_max_hp)
}

/// Schedules initiative using elapsed cycles as agility and turn modulo 20.
pub fn world_engine_193(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let agility = turn_num / cycle_len;
    let roll_val = wrap_map_coord_x(turn_num, 20);
    determine_turn_initiative(agility, roll_val)
}

/// Decays cycle-scaled encounter threat by distance into the active cycle.
pub fn world_engine_194(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let cycle_position = wrap_map_coord_x(turn_num, cycle_len);
    let distance = calc_manhattan_dist(0, 0, cycle_position, 0);
    let raw_threat = ai_loot_subsys_149(cycle_len, turn_num / cycle_len);
    calc_threat_decay_by_dist(raw_threat, distance)
}

/// Resolves a luck-scaled, safely bounded reward tier for the current turn.
pub fn world_engine_195(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let raw_roll = wrap_map_coord_x(turn_num * cycle_len, 10_000);
    let lucky_roll = scale_drop_by_luck(raw_roll, turn_num / cycle_len);
    let bounded_roll = clamp_to_bounds_x(lucky_roll, 10_000);
    calc_weighted_loot_tier(bounded_roll)
}

/// Calculates the next-level experience target with the supported level cap.
pub fn world_engine_196(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let raw_level = turn_num / cycle_len;
    let capped_level = clamp_to_bounds_x(raw_level, 300_000_000) + 1;
    calc_exp_to_next_level(capped_level)
}

/// Previews a combat exchange from world-clock-derived combatants.
pub fn world_engine_197(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let cycle_position = wrap_map_coord_x(turn_num, cycle_len);
    let actor_atk = calc_phys_atk_from_str(turn_num / cycle_len, cycle_position);
    let target_def = calc_armor_damage_reduction(cycle_len, cycle_position);
    simulate_single_turn_exchange(actor_atk, target_def, cycle_position > 0)
}

/// Reconstructs both components of a checkpoint hash for integrity checking.
pub fn world_engine_198(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let checkpoint_hash = pack_coords_to_hash(turn_num, cycle_len);
    unpack_hash_x(checkpoint_hash) + unpack_hash_y(checkpoint_hash)
}

/// Aggregates clock phase, reward quality, and encounter threat into a score.
pub fn world_engine_199(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let clock_phase = world_engine_188(turn_num, cycle_len);
    let reward_tier = world_engine_195(turn_num, cycle_len);
    let encounter_threat = world_engine_194(turn_num, cycle_len);
    clock_phase + reward_tier + encounter_threat
}

/// Runs a full world tick across scheduling, combat, hazards, and loot.
pub fn world_engine_200(turn_num: i64, cycle_len: i64) -> i64 {
    validate_world_clock(turn_num, cycle_len);
    let initiative = world_engine_193(turn_num, cycle_len);
    let combat_result = world_engine_197(turn_num, cycle_len);
    let hazard_result = world_engine_192(turn_num, cycle_len);
    let world_score = world_engine_199(turn_num, cycle_len);
    initiative + combat_result + hazard_result + world_score
}

fn validate_world_clock(turn_num: i64, cycle_len: i64) {
    assert!(turn_num >= 0);
    assert!(cycle_len > 0);
}
