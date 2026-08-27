from .tier0_spatial import *


def calc_max_hp_from_vit(base_hp: int, vitality: int, strength: int) -> int:
    return base_hp + vitality * 10 + strength * 2


def calc_max_mana_from_int(base_mana: int, intelligence: int) -> int:
    return base_mana + intelligence * 15


def calc_phys_atk_from_str(strength: int, agility: int) -> int:
    return strength * 2 + agility // 2


def calc_armor_damage_reduction(damage: int, armor: int) -> int:
    return damage - damage * armor // (100 + armor)


def calc_magic_res_reduction(magic_dmg: int, res: int) -> int:
    return magic_dmg - magic_dmg * res // (100 + res)


def calc_elemental_multiplier(elem_type: int, target_type: int, base_dmg: int) -> int:
    strong_matchups = {(0, 1), (1, 2), (2, 3), (3, 4), (4, 0)}
    weak_matchups = {(0, 4), (1, 0), (2, 1), (3, 2), (4, 3)}
    matchup = (elem_type, target_type)
    if matchup in strong_matchups:
        return base_dmg * 150 // 100
    if matchup in weak_matchups:
        return base_dmg * 75 // 100
    return base_dmg


def calc_crit_chance_clamped(base_crit: int, agility: int) -> int:
    return clamp_to_bounds_x(base_crit + agility * 10, 10001)


def calc_effective_penetration(raw_armor: int, pct_pen: int, flat_pen: int) -> int:
    return max(raw_armor * (100 - pct_pen) // 100 - flat_pen, 0)


def calc_overburden_penalty(weight: int, max_weight: int, base_speed: int) -> int:
    if weight > max_weight:
        return base_speed * 50 // 100
    return base_speed


def calc_lifesteal_conversion(damage_dealt: int, lifesteal_rate: int) -> int:
    return damage_dealt * lifesteal_rate // 100


def stat_subsys_051(stat_val: int, modifier: int) -> int:
    return calc_manhattan_dist(0, 0, stat_val, modifier)


def stat_subsys_052(stat_val: int, modifier: int) -> int:
    return calc_chebyshev_dist(0, 0, stat_val, modifier)


def stat_subsys_053(stat_val: int, modifier: int) -> int:
    return calc_euclidean_sq(0, 0, stat_val, modifier)


def stat_subsys_054(stat_val: int, modifier: int) -> int:
    return int(check_in_radius_sq(stat_val, modifier))


def stat_subsys_055(stat_val: int, modifier: int) -> int:
    return stat_val + bresenham_step_x(stat_val, modifier) + 1


def stat_subsys_056(stat_val: int, modifier: int) -> int:
    return stat_val + bresenham_step_y(stat_val, modifier) + 1


def stat_subsys_057(stat_val: int, modifier: int) -> int:
    return pack_coords_to_hash(stat_val, modifier)


def stat_subsys_058(stat_val: int, modifier: int) -> int:
    return modifier + unpack_hash_x(stat_val)


def stat_subsys_059(stat_val: int, modifier: int) -> int:
    return modifier + unpack_hash_y(stat_val)


def stat_subsys_060(stat_val: int, modifier: int) -> int:
    return wrap_map_coord_x(stat_val, modifier + 1)


def stat_subsys_061(stat_val: int, modifier: int) -> int:
    return wrap_map_coord_y(stat_val, modifier + 1)


def stat_subsys_062(stat_val: int, modifier: int) -> int:
    return clamp_to_bounds_x(stat_val, modifier + 1)


def stat_subsys_063(stat_val: int, modifier: int) -> int:
    return clamp_to_bounds_y(stat_val, modifier + 1)


def stat_subsys_064(stat_val: int, modifier: int) -> int:
    return int(
        check_aabb_overlap(
            0,
            0,
            stat_val,
            modifier,
            modifier,
            stat_val,
            stat_val + modifier,
            stat_val + modifier,
        )
    )


def stat_subsys_065(stat_val: int, modifier: int) -> int:
    return calc_grid_index(stat_val, modifier, stat_val + 1)


def stat_subsys_066(stat_val: int, modifier: int) -> int:
    return calc_line_of_sight_cost(stat_val, modifier)


def stat_subsys_067(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_017(stat_val, modifier)


def stat_subsys_068(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_018(stat_val, modifier)


def stat_subsys_069(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_019(stat_val, modifier)


def stat_subsys_070(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_020(stat_val, modifier)


def stat_subsys_071(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_021(stat_val, modifier)


def stat_subsys_072(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_022(stat_val, modifier)


def stat_subsys_073(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_023(stat_val, modifier)


def stat_subsys_074(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_024(stat_val, modifier)


def stat_subsys_075(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_025(stat_val, modifier)


def stat_subsys_076(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_026(stat_val, modifier)


def stat_subsys_077(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_027(stat_val, modifier)


def stat_subsys_078(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_028(stat_val, modifier)


def stat_subsys_079(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_029(stat_val, modifier)


def stat_subsys_080(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_030(stat_val, modifier)


def stat_subsys_081(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_031(stat_val, modifier)


def stat_subsys_082(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_032(stat_val, modifier)


def stat_subsys_083(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_033(stat_val, modifier)


def stat_subsys_084(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_034(stat_val, modifier + 1)


def stat_subsys_085(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_035(stat_val, modifier + 1)


def stat_subsys_086(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_036(stat_val, modifier)


def stat_subsys_087(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_037(stat_val, modifier)


def stat_subsys_088(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_038(stat_val, modifier)


def stat_subsys_089(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_039(stat_val, modifier)


def stat_subsys_090(stat_val: int, modifier: int) -> int:
    return stat_val + spatial_subsys_040(stat_val, modifier)
