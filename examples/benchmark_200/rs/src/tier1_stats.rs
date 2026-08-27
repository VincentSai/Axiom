use crate::tier0_spatial::*;

pub fn calc_max_hp_from_vit(base_hp: i64, vitality: i64, strength: i64) -> i64 {
    base_hp + vitality * 10 + strength * 2
}

pub fn calc_max_mana_from_int(base_mana: i64, intelligence: i64) -> i64 {
    base_mana + intelligence * 15
}

pub fn calc_phys_atk_from_str(strength: i64, agility: i64) -> i64 {
    strength * 2 + agility / 2
}

pub fn calc_armor_damage_reduction(damage: i64, armor: i64) -> i64 {
    damage - damage * armor / (100 + armor)
}

pub fn calc_magic_res_reduction(magic_dmg: i64, res: i64) -> i64 {
    magic_dmg - magic_dmg * res / (100 + res)
}

pub fn calc_elemental_multiplier(elem_type: i64, target_type: i64, base_dmg: i64) -> i64 {
    match (elem_type, target_type) {
        (0, 1) | (1, 2) | (2, 3) | (3, 4) | (4, 0) => base_dmg * 150 / 100,
        (0, 4) | (1, 0) | (2, 1) | (3, 2) | (4, 3) => base_dmg * 75 / 100,
        _ => base_dmg,
    }
}

pub fn calc_crit_chance_clamped(base_crit: i64, agility: i64) -> i64 {
    clamp_to_bounds_x(base_crit + agility * 10, 10001)
}

pub fn calc_effective_penetration(raw_armor: i64, pct_pen: i64, flat_pen: i64) -> i64 {
    (raw_armor * (100 - pct_pen) / 100 - flat_pen).max(0)
}

pub fn calc_overburden_penalty(weight: i64, max_weight: i64, base_speed: i64) -> i64 {
    if weight > max_weight {
        base_speed * 50 / 100
    } else {
        base_speed
    }
}

pub fn calc_lifesteal_conversion(damage_dealt: i64, lifesteal_rate: i64) -> i64 {
    damage_dealt * lifesteal_rate / 100
}

pub fn stat_subsys_051(stat_val: i64, modifier: i64) -> i64 {
    calc_manhattan_dist(0, 0, stat_val, modifier)
}

pub fn stat_subsys_052(stat_val: i64, modifier: i64) -> i64 {
    calc_chebyshev_dist(0, 0, stat_val, modifier)
}

pub fn stat_subsys_053(stat_val: i64, modifier: i64) -> i64 {
    calc_euclidean_sq(0, 0, stat_val, modifier)
}

pub fn stat_subsys_054(stat_val: i64, modifier: i64) -> i64 {
    i64::from(check_in_radius_sq(stat_val, modifier))
}

pub fn stat_subsys_055(stat_val: i64, modifier: i64) -> i64 {
    stat_val + bresenham_step_x(stat_val, modifier) + 1
}

pub fn stat_subsys_056(stat_val: i64, modifier: i64) -> i64 {
    stat_val + bresenham_step_y(stat_val, modifier) + 1
}

pub fn stat_subsys_057(stat_val: i64, modifier: i64) -> i64 {
    pack_coords_to_hash(stat_val, modifier)
}

pub fn stat_subsys_058(stat_val: i64, modifier: i64) -> i64 {
    modifier + unpack_hash_x(stat_val)
}

pub fn stat_subsys_059(stat_val: i64, modifier: i64) -> i64 {
    modifier + unpack_hash_y(stat_val)
}

pub fn stat_subsys_060(stat_val: i64, modifier: i64) -> i64 {
    wrap_map_coord_x(stat_val, modifier + 1)
}

pub fn stat_subsys_061(stat_val: i64, modifier: i64) -> i64 {
    wrap_map_coord_y(stat_val, modifier + 1)
}

pub fn stat_subsys_062(stat_val: i64, modifier: i64) -> i64 {
    clamp_to_bounds_x(stat_val, modifier + 1)
}

pub fn stat_subsys_063(stat_val: i64, modifier: i64) -> i64 {
    clamp_to_bounds_y(stat_val, modifier + 1)
}

pub fn stat_subsys_064(stat_val: i64, modifier: i64) -> i64 {
    i64::from(check_aabb_overlap(
        0,
        0,
        stat_val,
        modifier,
        modifier,
        stat_val,
        stat_val + modifier,
        stat_val + modifier,
    ))
}

pub fn stat_subsys_065(stat_val: i64, modifier: i64) -> i64 {
    calc_grid_index(stat_val, modifier, stat_val + 1)
}

pub fn stat_subsys_066(stat_val: i64, modifier: i64) -> i64 {
    calc_line_of_sight_cost(stat_val, modifier)
}

pub fn stat_subsys_067(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_017(stat_val, modifier)
}

pub fn stat_subsys_068(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_018(stat_val, modifier)
}

pub fn stat_subsys_069(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_019(stat_val, modifier)
}

pub fn stat_subsys_070(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_020(stat_val, modifier)
}

pub fn stat_subsys_071(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_021(stat_val, modifier)
}

pub fn stat_subsys_072(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_022(stat_val, modifier)
}

pub fn stat_subsys_073(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_023(stat_val, modifier)
}

pub fn stat_subsys_074(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_024(stat_val, modifier)
}

pub fn stat_subsys_075(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_025(stat_val, modifier)
}

pub fn stat_subsys_076(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_026(stat_val, modifier)
}

pub fn stat_subsys_077(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_027(stat_val, modifier)
}

pub fn stat_subsys_078(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_028(stat_val, modifier)
}

pub fn stat_subsys_079(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_029(stat_val, modifier)
}

pub fn stat_subsys_080(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_030(stat_val, modifier)
}

pub fn stat_subsys_081(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_031(stat_val, modifier)
}

pub fn stat_subsys_082(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_032(stat_val, modifier)
}

pub fn stat_subsys_083(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_033(stat_val, modifier)
}

pub fn stat_subsys_084(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_034(stat_val, modifier + 1)
}

pub fn stat_subsys_085(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_035(stat_val, modifier + 1)
}

pub fn stat_subsys_086(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_036(stat_val, modifier)
}

pub fn stat_subsys_087(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_037(stat_val, modifier)
}

pub fn stat_subsys_088(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_038(stat_val, modifier)
}

pub fn stat_subsys_089(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_039(stat_val, modifier)
}

pub fn stat_subsys_090(stat_val: i64, modifier: i64) -> i64 {
    stat_val + spatial_subsys_040(stat_val, modifier)
}
