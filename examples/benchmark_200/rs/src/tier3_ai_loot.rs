use crate::tier0_spatial::*;
use crate::tier1_stats::*;
use crate::tier2_combat::*;

pub fn calc_threat_decay_by_dist(threat: i64, dist: i64) -> i64 {
    let manhattan_dist = calc_manhattan_dist(0, 0, dist, 0);
    threat * 100 / (100 + manhattan_dist * 10)
}

pub fn evaluate_ai_behavior_state(hp_pct: i64, aggro: i64) -> i64 {
    if hp_pct < 20 {
        1
    } else if aggro > 50 {
        2
    } else {
        3
    }
}

pub fn calc_weighted_loot_tier(roll_val: i64) -> i64 {
    if roll_val < 6_000 {
        1
    } else if roll_val < 8_500 {
        2
    } else if roll_val < 9_500 {
        3
    } else if roll_val < 9_900 {
        4
    } else {
        5
    }
}

pub fn scale_drop_by_luck(roll_val: i64, luck: i64) -> i64 {
    calc_crit_chance_clamped(roll_val + luck * 2, 0)
}

pub fn calc_exp_gain_with_level_diff(base_exp: i64, player_lvl: i64, monster_lvl: i64) -> i64 {
    let level_diff = player_lvl - monster_lvl;
    if level_diff > 10 {
        base_exp * 10 / 100
    } else if level_diff > 5 {
        base_exp * 50 / 100
    } else {
        base_exp
    }
}

pub fn calc_exp_to_next_level(level: i64) -> i64 {
    level * level * 100 + level * 500
}

pub fn calc_shop_inflation_price(base_price: i64, world_day: i64) -> i64 {
    base_price * (100 + world_day * 2) / 100
}

pub fn ai_loot_subsys_148(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_101(delta, 0)
}

pub fn ai_loot_subsys_149(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_102(delta, 0)
}

pub fn ai_loot_subsys_150(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_103(delta, 0)
}

pub fn ai_loot_subsys_151(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_104(delta, 0)
}

pub fn ai_loot_subsys_152(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_105(delta, 0)
}

pub fn ai_loot_subsys_153(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_106(delta, 0)
}

pub fn ai_loot_subsys_154(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_107(delta, 0)
}

pub fn ai_loot_subsys_155(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_108(delta, 0)
}

pub fn ai_loot_subsys_156(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_109(delta, 0)
}

pub fn ai_loot_subsys_157(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_110(delta, 0)
}

pub fn ai_loot_subsys_158(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_111(delta, 0)
}

pub fn ai_loot_subsys_159(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_112(delta, 0)
}

pub fn ai_loot_subsys_160(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_113(delta, 0)
}

pub fn ai_loot_subsys_161(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_114(delta, 0)
}

pub fn ai_loot_subsys_162(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_115(delta, 0)
}

pub fn ai_loot_subsys_163(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_116(delta, 0)
}

pub fn ai_loot_subsys_164(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_117(delta, 0)
}

pub fn ai_loot_subsys_165(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_118(delta, 0)
}

pub fn ai_loot_subsys_166(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_119(delta, 0)
}

pub fn ai_loot_subsys_167(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_120(delta, 0)
}

pub fn ai_loot_subsys_168(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_121(delta, 0)
}

pub fn ai_loot_subsys_169(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_122(delta, 0)
}

pub fn ai_loot_subsys_170(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_123(delta, 0)
}

pub fn ai_loot_subsys_171(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_124(delta, 0)
}

pub fn ai_loot_subsys_172(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_125(delta, 0)
}

pub fn ai_loot_subsys_173(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_126(delta, 0)
}

pub fn ai_loot_subsys_174(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_127(delta, 0)
}

pub fn ai_loot_subsys_175(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_128(delta, 0)
}

pub fn ai_loot_subsys_176(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_129(delta, 0)
}

pub fn ai_loot_subsys_177(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_130(delta, 0)
}

pub fn ai_loot_subsys_178(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_131(delta, 0)
}

pub fn ai_loot_subsys_179(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_132(delta, 0)
}

pub fn ai_loot_subsys_180(curr_aggro: i64, delta: i64) -> i64 {
    curr_aggro + combat_subsys_133(delta, 0)
}
