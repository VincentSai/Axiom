from .tier0_spatial import *
from .tier1_stats import *
from .tier2_combat import *


def calc_threat_decay_by_dist(threat: int, dist: int) -> int:
    manhattan_dist = calc_manhattan_dist(0, 0, dist, 0)
    return threat * 100 // (100 + manhattan_dist * 10)


def evaluate_ai_behavior_state(hp_pct: int, aggro: int) -> int:
    if hp_pct < 20:
        return 1
    if aggro > 50:
        return 2
    return 3


def calc_weighted_loot_tier(roll_val: int) -> int:
    if roll_val < 6_000:
        return 1
    if roll_val < 8_500:
        return 2
    if roll_val < 9_500:
        return 3
    if roll_val < 9_900:
        return 4
    return 5


def scale_drop_by_luck(roll_val: int, luck: int) -> int:
    return calc_crit_chance_clamped(roll_val + luck * 2, 0)


def calc_exp_gain_with_level_diff(
    base_exp: int, player_lvl: int, monster_lvl: int
) -> int:
    level_diff = player_lvl - monster_lvl
    if level_diff > 10:
        return base_exp * 10 // 100
    if level_diff > 5:
        return base_exp * 50 // 100
    return base_exp


def calc_exp_to_next_level(level: int) -> int:
    return level * level * 100 + level * 500


def calc_shop_inflation_price(base_price: int, world_day: int) -> int:
    return base_price * (100 + world_day * 2) // 100


def ai_loot_subsys_148(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_101(delta, 0)


def ai_loot_subsys_149(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_102(delta, 0)


def ai_loot_subsys_150(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_103(delta, 0)


def ai_loot_subsys_151(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_104(delta, 0)


def ai_loot_subsys_152(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_105(delta, 0)


def ai_loot_subsys_153(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_106(delta, 0)


def ai_loot_subsys_154(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_107(delta, 0)


def ai_loot_subsys_155(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_108(delta, 0)


def ai_loot_subsys_156(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_109(delta, 0)


def ai_loot_subsys_157(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_110(delta, 0)


def ai_loot_subsys_158(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_111(delta, 0)


def ai_loot_subsys_159(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_112(delta, 0)


def ai_loot_subsys_160(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_113(delta, 0)


def ai_loot_subsys_161(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_114(delta, 0)


def ai_loot_subsys_162(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_115(delta, 0)


def ai_loot_subsys_163(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_116(delta, 0)


def ai_loot_subsys_164(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_117(delta, 0)


def ai_loot_subsys_165(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_118(delta, 0)


def ai_loot_subsys_166(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_119(delta, 0)


def ai_loot_subsys_167(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_120(delta, 0)


def ai_loot_subsys_168(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_121(delta, 0)


def ai_loot_subsys_169(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_122(delta, 0)


def ai_loot_subsys_170(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_123(delta, 0)


def ai_loot_subsys_171(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_124(delta, 0)


def ai_loot_subsys_172(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_125(delta, 0)


def ai_loot_subsys_173(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_126(delta, 0)


def ai_loot_subsys_174(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_127(delta, 0)


def ai_loot_subsys_175(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_128(delta, 0)


def ai_loot_subsys_176(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_129(delta, 0)


def ai_loot_subsys_177(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_130(delta, 0)


def ai_loot_subsys_178(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_131(delta, 0)


def ai_loot_subsys_179(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_132(delta, 0)


def ai_loot_subsys_180(curr_aggro: int, delta: int) -> int:
    return curr_aggro + combat_subsys_133(delta, 0)
