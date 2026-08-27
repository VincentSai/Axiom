"""Public API for the pure five-tier roguelike engine benchmark."""

from .models import *
from .tier0_spatial import *
from .tier1_stats import *
from .tier2_combat import *
from .tier3_ai_loot import *
from .tier4_world import *


_MODEL_NAMES = [
    "Point",
    "Velocity",
    "Rect",
    "BaseAttributes",
    "DerivedStats",
    "StatusEffect",
    "Entity",
]
_TIER0_NAMES = [
    "calc_manhattan_dist",
    "calc_chebyshev_dist",
    "calc_euclidean_sq",
    "check_in_radius_sq",
    "bresenham_step_x",
    "bresenham_step_y",
    "pack_coords_to_hash",
    "unpack_hash_x",
    "unpack_hash_y",
    "wrap_map_coord_x",
    "wrap_map_coord_y",
    "clamp_to_bounds_x",
    "clamp_to_bounds_y",
    "check_aabb_overlap",
    "calc_grid_index",
    "calc_line_of_sight_cost",
    *[f"spatial_subsys_{index:03d}" for index in range(17, 41)],
]
_TIER1_NAMES = [
    "calc_max_hp_from_vit",
    "calc_max_mana_from_int",
    "calc_phys_atk_from_str",
    "calc_armor_damage_reduction",
    "calc_magic_res_reduction",
    "calc_elemental_multiplier",
    "calc_crit_chance_clamped",
    "calc_effective_penetration",
    "calc_overburden_penalty",
    "calc_lifesteal_conversion",
    *[f"stat_subsys_{index:03d}" for index in range(51, 91)],
]
_TIER2_NAMES = [
    "resolve_physical_hit",
    "resolve_crit_hit",
    "apply_dual_shield_absorption",
    "apply_bleed_stack_tick",
    "apply_poison_exponential_decay",
    "apply_burn_max_hp_tick",
    "check_freeze_threshold",
    "calc_stamina_skill_cost",
    "calc_mana_overload_burst",
    "calc_poise_break_stun",
    *[f"combat_subsys_{index:03d}" for index in range(101, 141)],
]
_TIER3_NAMES = [
    "calc_threat_decay_by_dist",
    "evaluate_ai_behavior_state",
    "calc_weighted_loot_tier",
    "scale_drop_by_luck",
    "calc_exp_gain_with_level_diff",
    "calc_exp_to_next_level",
    "calc_shop_inflation_price",
    *[f"ai_loot_subsys_{index:03d}" for index in range(148, 181)],
]
_TIER4_NAMES = [
    "determine_turn_initiative",
    "resolve_boss_phase_transition",
    "simulate_single_turn_exchange",
    "resolve_terrain_hazard_tick",
    *[f"world_engine_{index}" for index in range(185, 201)],
]

__all__ = (
    _MODEL_NAMES
    + _TIER0_NAMES
    + _TIER1_NAMES
    + _TIER2_NAMES
    + _TIER3_NAMES
    + _TIER4_NAMES
)

del _MODEL_NAMES, _TIER0_NAMES, _TIER1_NAMES, _TIER2_NAMES, _TIER3_NAMES
del _TIER4_NAMES
