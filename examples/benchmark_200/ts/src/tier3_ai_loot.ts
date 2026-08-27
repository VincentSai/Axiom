import * as spatial from './tier0_spatial.js';
import * as stats from './tier1_stats.js';
import * as combat from './tier2_combat.js';

export function calc_threat_decay_by_dist(threat: number, dist: number): number {
  const manhattanDist = spatial.calc_manhattan_dist(0, 0, dist, 0);
  return Math.trunc((threat * 100) / (100 + manhattanDist * 10));
}

export function evaluate_ai_behavior_state(hp_pct: number, aggro: number): number {
  if (hp_pct < 20) {
    return 1;
  }
  if (aggro > 50) {
    return 2;
  }
  return 3;
}

export function calc_weighted_loot_tier(roll_val: number): number {
  if (roll_val < 6_000) {
    return 1;
  }
  if (roll_val < 8_500) {
    return 2;
  }
  if (roll_val < 9_500) {
    return 3;
  }
  if (roll_val < 9_900) {
    return 4;
  }
  return 5;
}

export function scale_drop_by_luck(roll_val: number, luck: number): number {
  return stats.calc_crit_chance_clamped(roll_val + luck * 2, 0);
}

export function calc_exp_gain_with_level_diff(
  base_exp: number,
  player_lvl: number,
  monster_lvl: number,
): number {
  const levelDiff = player_lvl - monster_lvl;
  if (levelDiff > 10) {
    return Math.trunc((base_exp * 10) / 100);
  }
  if (levelDiff > 5) {
    return Math.trunc((base_exp * 50) / 100);
  }
  return base_exp;
}

export function calc_exp_to_next_level(level: number): number {
  return level * level * 100 + level * 500;
}

export function calc_shop_inflation_price(base_price: number, world_day: number): number {
  return Math.trunc((base_price * (100 + world_day * 2)) / 100);
}

export function ai_loot_subsys_148(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_101(delta, 0);
}

export function ai_loot_subsys_149(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_102(delta, 0);
}

export function ai_loot_subsys_150(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_103(delta, 0);
}

export function ai_loot_subsys_151(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_104(delta, 0);
}

export function ai_loot_subsys_152(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_105(delta, 0);
}

export function ai_loot_subsys_153(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_106(delta, 0);
}

export function ai_loot_subsys_154(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_107(delta, 0);
}

export function ai_loot_subsys_155(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_108(delta, 0);
}

export function ai_loot_subsys_156(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_109(delta, 0);
}

export function ai_loot_subsys_157(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_110(delta, 0);
}

export function ai_loot_subsys_158(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_111(delta, 0);
}

export function ai_loot_subsys_159(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_112(delta, 0);
}

export function ai_loot_subsys_160(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_113(delta, 0);
}

export function ai_loot_subsys_161(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_114(delta, 0);
}

export function ai_loot_subsys_162(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_115(delta, 0);
}

export function ai_loot_subsys_163(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_116(delta, 0);
}

export function ai_loot_subsys_164(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_117(delta, 0);
}

export function ai_loot_subsys_165(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_118(delta, 0);
}

export function ai_loot_subsys_166(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_119(delta, 0);
}

export function ai_loot_subsys_167(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_120(delta, 0);
}

export function ai_loot_subsys_168(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_121(delta, 0);
}

export function ai_loot_subsys_169(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_122(delta, 0);
}

export function ai_loot_subsys_170(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_123(delta, 0);
}

export function ai_loot_subsys_171(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_124(delta, 0);
}

export function ai_loot_subsys_172(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_125(delta, 0);
}

export function ai_loot_subsys_173(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_126(delta, 0);
}

export function ai_loot_subsys_174(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_127(delta, 0);
}

export function ai_loot_subsys_175(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_128(delta, 0);
}

export function ai_loot_subsys_176(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_129(delta, 0);
}

export function ai_loot_subsys_177(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_130(delta, 0);
}

export function ai_loot_subsys_178(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_131(delta, 0);
}

export function ai_loot_subsys_179(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_132(delta, 0);
}

export function ai_loot_subsys_180(curr_aggro: number, delta: number): number {
  return curr_aggro + combat.combat_subsys_133(delta, 0);
}
