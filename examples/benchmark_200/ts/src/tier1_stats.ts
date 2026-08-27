import * as spatial from './tier0_spatial.js';

export function calc_max_hp_from_vit(
  base_hp: number,
  vitality: number,
  strength: number,
): number {
  return base_hp + vitality * 10 + strength * 2;
}

export function calc_max_mana_from_int(base_mana: number, intelligence: number): number {
  return base_mana + intelligence * 15;
}

export function calc_phys_atk_from_str(strength: number, agility: number): number {
  return strength * 2 + Math.trunc(agility / 2);
}

export function calc_armor_damage_reduction(damage: number, armor: number): number {
  return damage - Math.trunc((damage * armor) / (100 + armor));
}

export function calc_magic_res_reduction(magic_dmg: number, res: number): number {
  return magic_dmg - Math.trunc((magic_dmg * res) / (100 + res));
}

export function calc_elemental_multiplier(
  elem_type: number,
  target_type: number,
  base_dmg: number,
): number {
  const isStrong =
    (elem_type === 0 && target_type === 1) ||
    (elem_type === 1 && target_type === 2) ||
    (elem_type === 2 && target_type === 3) ||
    (elem_type === 3 && target_type === 4) ||
    (elem_type === 4 && target_type === 0);

  if (isStrong) {
    return Math.trunc((base_dmg * 150) / 100);
  }

  const isWeak =
    (elem_type === 0 && target_type === 4) ||
    (elem_type === 1 && target_type === 0) ||
    (elem_type === 2 && target_type === 1) ||
    (elem_type === 3 && target_type === 2) ||
    (elem_type === 4 && target_type === 3);

  if (isWeak) {
    return Math.trunc((base_dmg * 75) / 100);
  }

  return base_dmg;
}

export function calc_crit_chance_clamped(base_crit: number, agility: number): number {
  return spatial.clamp_to_bounds_x(base_crit + agility * 10, 10001);
}

export function calc_effective_penetration(
  raw_armor: number,
  pct_pen: number,
  flat_pen: number,
): number {
  return Math.max(Math.trunc((raw_armor * (100 - pct_pen)) / 100) - flat_pen, 0);
}

export function calc_overburden_penalty(
  weight: number,
  max_weight: number,
  base_speed: number,
): number {
  return weight > max_weight ? Math.trunc((base_speed * 50) / 100) : base_speed;
}

export function calc_lifesteal_conversion(
  damage_dealt: number,
  lifesteal_rate: number,
): number {
  return Math.trunc((damage_dealt * lifesteal_rate) / 100);
}

export function stat_subsys_051(stat_val: number, modifier: number): number {
  return spatial.calc_manhattan_dist(0, 0, stat_val, modifier);
}

export function stat_subsys_052(stat_val: number, modifier: number): number {
  return spatial.calc_chebyshev_dist(0, 0, stat_val, modifier);
}

export function stat_subsys_053(stat_val: number, modifier: number): number {
  return spatial.calc_euclidean_sq(0, 0, stat_val, modifier);
}

export function stat_subsys_054(stat_val: number, modifier: number): number {
  return Number(spatial.check_in_radius_sq(stat_val, modifier));
}

export function stat_subsys_055(stat_val: number, modifier: number): number {
  return stat_val + spatial.bresenham_step_x(stat_val, modifier) + 1;
}

export function stat_subsys_056(stat_val: number, modifier: number): number {
  return stat_val + spatial.bresenham_step_y(stat_val, modifier) + 1;
}

export function stat_subsys_057(stat_val: number, modifier: number): number {
  return spatial.pack_coords_to_hash(stat_val, modifier);
}

export function stat_subsys_058(stat_val: number, modifier: number): number {
  return modifier + spatial.unpack_hash_x(stat_val);
}

export function stat_subsys_059(stat_val: number, modifier: number): number {
  return modifier + spatial.unpack_hash_y(stat_val);
}

export function stat_subsys_060(stat_val: number, modifier: number): number {
  return spatial.wrap_map_coord_x(stat_val, modifier + 1);
}

export function stat_subsys_061(stat_val: number, modifier: number): number {
  return spatial.wrap_map_coord_y(stat_val, modifier + 1);
}

export function stat_subsys_062(stat_val: number, modifier: number): number {
  return spatial.clamp_to_bounds_x(stat_val, modifier + 1);
}

export function stat_subsys_063(stat_val: number, modifier: number): number {
  return spatial.clamp_to_bounds_y(stat_val, modifier + 1);
}

export function stat_subsys_064(stat_val: number, modifier: number): number {
  return Number(
    spatial.check_aabb_overlap(
      0,
      0,
      stat_val,
      modifier,
      modifier,
      stat_val,
      stat_val + modifier,
      stat_val + modifier,
    ),
  );
}

export function stat_subsys_065(stat_val: number, modifier: number): number {
  return spatial.calc_grid_index(stat_val, modifier, stat_val + 1);
}

export function stat_subsys_066(stat_val: number, modifier: number): number {
  return spatial.calc_line_of_sight_cost(stat_val, modifier);
}

export function stat_subsys_067(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_017(stat_val, modifier);
}

export function stat_subsys_068(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_018(stat_val, modifier);
}

export function stat_subsys_069(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_019(stat_val, modifier);
}

export function stat_subsys_070(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_020(stat_val, modifier);
}

export function stat_subsys_071(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_021(stat_val, modifier);
}

export function stat_subsys_072(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_022(stat_val, modifier);
}

export function stat_subsys_073(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_023(stat_val, modifier);
}

export function stat_subsys_074(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_024(stat_val, modifier);
}

export function stat_subsys_075(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_025(stat_val, modifier);
}

export function stat_subsys_076(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_026(stat_val, modifier);
}

export function stat_subsys_077(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_027(stat_val, modifier);
}

export function stat_subsys_078(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_028(stat_val, modifier);
}

export function stat_subsys_079(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_029(stat_val, modifier);
}

export function stat_subsys_080(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_030(stat_val, modifier);
}

export function stat_subsys_081(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_031(stat_val, modifier);
}

export function stat_subsys_082(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_032(stat_val, modifier);
}

export function stat_subsys_083(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_033(stat_val, modifier);
}

export function stat_subsys_084(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_034(stat_val, modifier + 1);
}

export function stat_subsys_085(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_035(stat_val, modifier + 1);
}

export function stat_subsys_086(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_036(stat_val, modifier);
}

export function stat_subsys_087(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_037(stat_val, modifier);
}

export function stat_subsys_088(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_038(stat_val, modifier);
}

export function stat_subsys_089(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_039(stat_val, modifier);
}

export function stat_subsys_090(stat_val: number, modifier: number): number {
  return stat_val + spatial.spatial_subsys_040(stat_val, modifier);
}
