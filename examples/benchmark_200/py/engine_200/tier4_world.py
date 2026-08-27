"""Tier 4 turn orchestration and world simulation.

Every function is pure and composes calculations from lower engine tiers, which
preserves the five-layer dependency topology required by the benchmark.
"""

from .tier0_spatial import (
    calc_grid_index,
    calc_manhattan_dist,
    clamp_to_bounds_x,
    pack_coords_to_hash,
    unpack_hash_x,
    unpack_hash_y,
    wrap_map_coord_x,
    wrap_map_coord_y,
)
from .tier1_stats import (
    calc_armor_damage_reduction,
    calc_lifesteal_conversion,
    calc_max_hp_from_vit,
    calc_phys_atk_from_str,
)
from .tier2_combat import (
    apply_bleed_stack_tick,
    apply_poison_exponential_decay,
    resolve_physical_hit,
)
from .tier3_ai_loot import (
    ai_loot_subsys_148,
    ai_loot_subsys_149,
    calc_exp_to_next_level,
    calc_shop_inflation_price,
    calc_threat_decay_by_dist,
    calc_weighted_loot_tier,
    scale_drop_by_luck,
)


def determine_turn_initiative(agility: int, roll_val: int) -> int:
    """Calculate initiative from agility and a pre-generated d20-style roll."""
    assert agility >= 0, "agility must be nonnegative"
    assert 0 <= roll_val < 20, "roll_val must be in the range [0, 20)"

    base_initiative = calc_phys_atk_from_str(agility, 0)
    return base_initiative + roll_val


def resolve_boss_phase_transition(hp_pct: int) -> int:
    """Return P1 above 66%, P2 at 33%-66%, and P3 below 33%."""
    assert 0 <= hp_pct <= 100, "hp_pct must be in the range [0, 100]"

    if hp_pct > 66:
        phase = 1
    elif hp_pct >= 33:
        phase = 2
    else:
        phase = 3
    return calc_grid_index(phase, 0, 1)


def simulate_single_turn_exchange(
    actor_atk: int, target_def: int, is_moving: bool
) -> int:
    """Resolve one attack, its movement-sensitive bleed tick, and threat."""
    assert actor_atk >= 0, "actor_atk must be nonnegative"
    assert target_def >= 0, "target_def must be nonnegative"

    direct_damage = resolve_physical_hit(actor_atk, target_def)
    movement_distance = int(is_moving)
    grid_distance = calc_manhattan_dist(0, 0, movement_distance, 0)
    bleed_damage = apply_bleed_stack_tick(1, is_moving)
    observed_threat = calc_threat_decay_by_dist(direct_damage, grid_distance)
    return direct_damage + bleed_damage + observed_threat


def resolve_terrain_hazard_tick(hazard_type: int, max_hp: int) -> int:
    """Resolve one terrain tick: safe, lava, poison marsh, or ice."""
    assert 0 <= hazard_type <= 3, "hazard_type must be in the range [0, 3]"
    assert max_hp >= 0, "max_hp must be nonnegative"

    if hazard_type == 0:
        return 0
    if hazard_type == 1:
        return calc_lifesteal_conversion(max_hp, 5) + 10
    if hazard_type == 2:
        return apply_poison_exponential_decay(max_hp // 100, 1)
    return 0


def world_engine_185(turn_num: int, cycle_len: int) -> int:
    """Count completed world cycles and map them into checkpoint space."""
    _validate_world_clock(turn_num, cycle_len)
    completed_cycles = turn_num // cycle_len
    return calc_grid_index(completed_cycles, 0, cycle_len)


def world_engine_186(turn_num: int, cycle_len: int) -> int:
    """Produce the monotonically increasing garbage-collection generation."""
    _validate_world_clock(turn_num, cycle_len)
    completed_cycles = turn_num // cycle_len
    return ai_loot_subsys_148(completed_cycles, 0)


def world_engine_187(turn_num: int, cycle_len: int) -> int:
    """Compute a deterministic save-slot hash for a turn/cycle pair."""
    _validate_world_clock(turn_num, cycle_len)
    return pack_coords_to_hash(turn_num, cycle_len)


def world_engine_188(turn_num: int, cycle_len: int) -> int:
    """Resolve a four-phase day/night clock from the active cycle position."""
    _validate_world_clock(turn_num, cycle_len)
    cycle_position = wrap_map_coord_x(turn_num, cycle_len)
    return cycle_position * 4 // cycle_len


def world_engine_189(turn_num: int, cycle_len: int) -> int:
    """Return turns remaining until the next cycle boundary."""
    _validate_world_clock(turn_num, cycle_len)
    cycle_position = wrap_map_coord_y(turn_num, cycle_len)
    return 0 if cycle_position == 0 else cycle_len - cycle_position


def world_engine_190(turn_num: int, cycle_len: int) -> int:
    """Apply world-day inflation to a cycle-sized baseline shop price."""
    _validate_world_clock(turn_num, cycle_len)
    world_day = turn_num // cycle_len
    return calc_shop_inflation_price(cycle_len, world_day)


def world_engine_191(turn_num: int, cycle_len: int) -> int:
    """Derive a descending boss-health clock and resolve its active phase."""
    _validate_world_clock(turn_num, cycle_len)
    cycle_position = wrap_map_coord_x(turn_num, cycle_len)
    hp_pct = 100 - cycle_position * 100 // cycle_len
    return resolve_boss_phase_transition(hp_pct)


def world_engine_192(turn_num: int, cycle_len: int) -> int:
    """Rotate terrain hazards while scaling maximum HP by elapsed cycles."""
    _validate_world_clock(turn_num, cycle_len)
    hazard_type = wrap_map_coord_x(turn_num, 4)
    world_age = turn_num // cycle_len
    scaled_max_hp = calc_max_hp_from_vit(cycle_len, world_age, 0)
    return resolve_terrain_hazard_tick(hazard_type, scaled_max_hp)


def world_engine_193(turn_num: int, cycle_len: int) -> int:
    """Schedule initiative from elapsed cycles and turn modulo twenty."""
    _validate_world_clock(turn_num, cycle_len)
    agility = turn_num // cycle_len
    roll_val = wrap_map_coord_x(turn_num, 20)
    return determine_turn_initiative(agility, roll_val)


def world_engine_194(turn_num: int, cycle_len: int) -> int:
    """Decay cycle-scaled encounter threat by distance into the cycle."""
    _validate_world_clock(turn_num, cycle_len)
    cycle_position = wrap_map_coord_x(turn_num, cycle_len)
    distance = calc_manhattan_dist(0, 0, cycle_position, 0)
    raw_threat = ai_loot_subsys_149(cycle_len, turn_num // cycle_len)
    return calc_threat_decay_by_dist(raw_threat, distance)


def world_engine_195(turn_num: int, cycle_len: int) -> int:
    """Resolve a luck-scaled, safely bounded reward tier for this turn."""
    _validate_world_clock(turn_num, cycle_len)
    raw_roll = wrap_map_coord_x(turn_num * cycle_len, 10_000)
    lucky_roll = scale_drop_by_luck(raw_roll, turn_num // cycle_len)
    bounded_roll = clamp_to_bounds_x(lucky_roll, 10_000)
    return calc_weighted_loot_tier(bounded_roll)


def world_engine_196(turn_num: int, cycle_len: int) -> int:
    """Calculate the next-level experience target under the supported cap."""
    _validate_world_clock(turn_num, cycle_len)
    raw_level = turn_num // cycle_len
    capped_level = clamp_to_bounds_x(raw_level, 300_000_000) + 1
    return calc_exp_to_next_level(capped_level)


def world_engine_197(turn_num: int, cycle_len: int) -> int:
    """Preview a combat exchange from world-clock-derived combatants."""
    _validate_world_clock(turn_num, cycle_len)
    cycle_position = wrap_map_coord_x(turn_num, cycle_len)
    actor_atk = calc_phys_atk_from_str(turn_num // cycle_len, cycle_position)
    target_def = calc_armor_damage_reduction(cycle_len, cycle_position)
    return simulate_single_turn_exchange(actor_atk, target_def, cycle_position > 0)


def world_engine_198(turn_num: int, cycle_len: int) -> int:
    """Reconstruct both checkpoint-hash components for integrity checking."""
    _validate_world_clock(turn_num, cycle_len)
    checkpoint_hash = pack_coords_to_hash(turn_num, cycle_len)
    return unpack_hash_x(checkpoint_hash) + unpack_hash_y(checkpoint_hash)


def world_engine_199(turn_num: int, cycle_len: int) -> int:
    """Aggregate clock phase, reward quality, and encounter threat."""
    _validate_world_clock(turn_num, cycle_len)
    clock_phase = world_engine_188(turn_num, cycle_len)
    reward_tier = world_engine_195(turn_num, cycle_len)
    encounter_threat = world_engine_194(turn_num, cycle_len)
    return clock_phase + reward_tier + encounter_threat


def world_engine_200(turn_num: int, cycle_len: int) -> int:
    """Run a complete world tick across scheduling, combat, hazards, and loot."""
    _validate_world_clock(turn_num, cycle_len)
    initiative = world_engine_193(turn_num, cycle_len)
    combat_result = world_engine_197(turn_num, cycle_len)
    hazard_result = world_engine_192(turn_num, cycle_len)
    world_score = world_engine_199(turn_num, cycle_len)
    return initiative + combat_result + hazard_result + world_score


def _validate_world_clock(turn_num: int, cycle_len: int) -> None:
    assert turn_num >= 0, "turn_num must be nonnegative"
    assert cycle_len > 0, "cycle_len must be positive"


__all__ = [
    "determine_turn_initiative",
    "resolve_boss_phase_transition",
    "simulate_single_turn_exchange",
    "resolve_terrain_hazard_tick",
    *[f"world_engine_{index}" for index in range(185, 201)],
]
