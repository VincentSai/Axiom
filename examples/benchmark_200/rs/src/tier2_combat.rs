use crate::tier1_stats::*;

/// Applies armor mitigation while preserving the combat-wide one-damage floor.
pub fn resolve_physical_hit(atk: i64, def_val: i64) -> i64 {
    calc_armor_damage_reduction(atk, def_val).max(1)
}

/// Applies the critical-hit percentage when the hit is critical.
pub fn resolve_crit_hit(dmg: i64, is_crit: bool, crit_pct: i64) -> i64 {
    let normalized_dmg = calc_armor_damage_reduction(dmg, 0);
    if is_crit {
        normalized_dmg * crit_pct / 100
    } else {
        normalized_dmg
    }
}

/// Returns the portion of incoming damage that penetrates the shield.
pub fn apply_dual_shield_absorption(incoming_dmg: i64, shield_hp: i64) -> i64 {
    calc_effective_penetration(incoming_dmg, 0, shield_hp)
}

/// Deals five damage per bleed stack, doubled while the target is moving.
pub fn apply_bleed_stack_tick(stacks: i64, is_moving: bool) -> i64 {
    let base_tick = calc_lifesteal_conversion(stacks * 5, 100);
    if is_moving {
        base_tick * 2
    } else {
        base_tick
    }
}

/// Applies the duration-weighted eighty-percent poison decay.
pub fn apply_poison_exponential_decay(base_poison: i64, duration: i64) -> i64 {
    base_poison * (duration * 80 / 100)
}

/// Combines three percent of maximum HP with flat burn damage.
pub fn apply_burn_max_hp_tick(target_max_hp: i64, flat_burn: i64) -> i64 {
    target_max_hp * 3 / 100 + flat_burn
}

/// Reports whether the five-stack freeze threshold has been reached.
pub fn check_freeze_threshold(freeze_stacks: i64) -> bool {
    freeze_stacks >= 5
}

/// Reduces a stamina cost by the supplied efficiency percentage.
pub fn calc_stamina_skill_cost(base_cost: i64, stamina_eff: i64) -> i64 {
    base_cost * (100 - stamina_eff) / 100
}

/// Converts ten percent of missing mana into overload damage.
pub fn calc_mana_overload_burst(missing_mana: i64) -> i64 {
    missing_mana * 10 / 100
}

/// Reports whether an impact depletes the target's remaining poise.
pub fn calc_poise_break_stun(poise: i64, impact: i64) -> bool {
    impact >= poise
}

pub fn combat_subsys_101(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_051(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_102(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_052(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_103(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_053(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_104(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_054(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_105(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_055(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_106(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_056(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_107(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_057(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_108(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_058(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_109(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_059(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_110(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_060(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_111(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_061(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_112(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_062(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_113(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_063(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_114(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_064(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_115(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_065(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_116(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_066(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_117(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_067(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_118(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_068(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_119(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_069(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_120(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_070(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_121(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_071(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_122(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_072(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_123(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_073(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_124(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_074(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_125(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_075(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_126(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_076(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_127(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_077(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_128(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_078(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_129(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_079(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_130(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_080(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_131(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_081(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_132(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_082(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_133(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_083(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_134(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_084(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_135(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_085(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_136(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_086(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_137(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_087(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_138(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_088(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_139(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_089(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}

pub fn combat_subsys_140(raw_dmg: i64, target_def: i64) -> i64 {
    let effective_def = stat_subsys_090(target_def, 0);
    if raw_dmg > effective_def {
        raw_dmg - effective_def
    } else {
        1
    }
}
