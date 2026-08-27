from .tier0_spatial import *
from .tier1_stats import *


def resolve_physical_hit(atk: int, def_val: int) -> int:
    return max(calc_armor_damage_reduction(atk, def_val), 1)


def resolve_crit_hit(dmg: int, is_crit: bool, crit_pct: int) -> int:
    normalized_dmg = calc_armor_damage_reduction(dmg, 0)
    return normalized_dmg * crit_pct // 100 if is_crit else normalized_dmg


def apply_dual_shield_absorption(incoming_dmg: int, shield_hp: int) -> int:
    return calc_effective_penetration(incoming_dmg, 0, shield_hp)


def apply_bleed_stack_tick(stacks: int, is_moving: bool) -> int:
    base_tick = calc_lifesteal_conversion(stacks * 5, 100)
    return base_tick * 2 if is_moving else base_tick


def apply_poison_exponential_decay(base_poison: int, duration: int) -> int:
    return base_poison * (duration * 80 // 100)


def apply_burn_max_hp_tick(target_max_hp: int, flat_burn: int) -> int:
    return target_max_hp * 3 // 100 + flat_burn


def check_freeze_threshold(freeze_stacks: int) -> bool:
    return freeze_stacks >= 5


def calc_stamina_skill_cost(base_cost: int, stamina_eff: int) -> int:
    return base_cost * (100 - stamina_eff) // 100


def calc_mana_overload_burst(missing_mana: int) -> int:
    return missing_mana * 10 // 100


def calc_poise_break_stun(poise: int, impact: int) -> bool:
    return impact >= poise


def combat_subsys_101(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_051(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_102(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_052(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_103(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_053(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_104(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_054(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_105(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_055(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_106(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_056(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_107(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_057(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_108(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_058(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_109(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_059(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_110(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_060(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_111(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_061(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_112(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_062(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_113(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_063(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_114(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_064(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_115(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_065(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_116(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_066(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_117(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_067(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_118(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_068(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_119(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_069(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_120(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_070(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_121(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_071(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_122(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_072(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_123(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_073(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_124(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_074(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_125(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_075(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_126(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_076(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_127(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_077(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_128(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_078(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_129(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_079(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_130(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_080(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_131(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_081(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_132(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_082(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_133(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_083(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_134(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_084(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_135(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_085(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_136(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_086(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_137(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_087(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_138(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_088(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_139(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_089(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1


def combat_subsys_140(raw_dmg: int, target_def: int) -> int:
    effective_def = stat_subsys_090(target_def, 0)
    return raw_dmg - effective_def if raw_dmg > effective_def else 1
