"""Pure combat and world calculations for the roguelike engine.

The functions in this module deliberately contain no random number generation or
mutable state.  Callers provide already-resolved rolls and receive deterministic
integer results.
"""

from dataclasses import dataclass


@dataclass(frozen=True)
class Point:
    x: int
    y: int


@dataclass(frozen=True)
class Stats:
    hp: int
    max_hp: int
    attack: int
    defense: int
    mana: int
    max_mana: int
    stamina: int
    max_stamina: int


@dataclass(frozen=True)
class LootItem:
    id: int
    name: str
    weight: int
    rarity: int


@dataclass(frozen=True)
class Player:
    id: int
    pos: Point
    stats: Stats
    exp: int
    level: int
    gold: int


@dataclass(frozen=True)
class Monster:
    id: int
    pos: Point
    stats: Stats
    aggro: int
    is_alive: bool


def _require(condition: bool, message: str) -> None:
    """Raise a consistent error when a subsystem contract is violated."""

    if not condition:
        raise ValueError(message)


def calc_crit_damage(base_dmg: int, is_crit: bool, crit_multiplier: int) -> int:
    """Return base damage, multiplied when the hit is critical."""

    _require(base_dmg >= 0, "base_dmg must be non-negative")
    _require(crit_multiplier >= 1, "crit_multiplier must be at least 1")
    return base_dmg * crit_multiplier if is_crit else base_dmg


def calc_elemental_resistance(dmg: int, res_percent: int) -> int:
    """Reduce damage by an integer percentage from 0 through 100."""

    _require(dmg >= 0, "dmg must be non-negative")
    _require(0 <= res_percent <= 100, "res_percent must be between 0 and 100")
    return dmg - (dmg * res_percent // 100)


def apply_lifesteal(
    current_hp: int, max_hp: int, dmg_dealt: int, lifesteal_rate: int
) -> int:
    """Heal from dealt damage without exceeding maximum health."""

    _require(current_hp >= 0, "current_hp must be non-negative")
    _require(max_hp > 0, "max_hp must be positive")
    _require(dmg_dealt >= 0, "dmg_dealt must be non-negative")
    _require(lifesteal_rate >= 0, "lifesteal_rate must be non-negative")
    healing = dmg_dealt * lifesteal_rate // 100
    return min(current_hp + healing, max_hp)


def calc_dodge_chance(accuracy: int, evasion: int) -> int:
    """Return evasion over accuracy as a chance capped at 100 percent."""

    _require(accuracy >= 0, "accuracy must be non-negative")
    _require(evasion >= 0, "evasion must be non-negative")
    return min(max(evasion - accuracy, 0), 100)


def apply_poison_tick(current_hp: int, poison_dmg: int) -> int:
    """Apply poison damage, flooring remaining health at zero."""

    _require(current_hp >= 0, "current_hp must be non-negative")
    _require(poison_dmg >= 0, "poison_dmg must be non-negative")
    return max(current_hp - poison_dmg, 0)


def apply_shield_absorption(shield_amount: int, incoming_dmg: int) -> int:
    """Return incoming damage left after the shield absorbs what it can."""

    _require(shield_amount >= 0, "shield_amount must be non-negative")
    _require(incoming_dmg >= 0, "incoming_dmg must be non-negative")
    return max(incoming_dmg - shield_amount, 0)


def calc_mana_cost(base_cost: int, cost_reduction: int) -> int:
    """Apply a flat mana-cost reduction, flooring the result at zero."""

    _require(base_cost >= 0, "base_cost must be non-negative")
    _require(cost_reduction >= 0, "cost_reduction must be non-negative")
    return max(base_cost - cost_reduction, 0)


def calc_stamina_drain(current_stamina: int, action_cost: int) -> int:
    """Return stamina remaining after an action, floored at zero."""

    _require(current_stamina >= 0, "current_stamina must be non-negative")
    _require(action_cost >= 0, "action_cost must be non-negative")
    return max(current_stamina - action_cost, 0)


def calc_gold_drop(base_gold: int, luck_bonus: int) -> int:
    """Increase a gold drop by an integer percentage luck bonus."""

    _require(base_gold >= 0, "base_gold must be non-negative")
    _require(luck_bonus >= 0, "luck_bonus must be non-negative")
    return base_gold + (base_gold * luck_bonus // 100)


def calc_exp_reward(base_exp: int, player_lvl: int, monster_lvl: int) -> int:
    """Adjust experience for the level difference between player and monster."""

    _require(base_exp >= 0, "base_exp must be non-negative")
    _require(player_lvl >= 1, "player_lvl must be at least 1")
    _require(monster_lvl >= 1, "monster_lvl must be at least 1")
    if monster_lvl >= player_lvl:
        return base_exp + (monster_lvl - player_lvl) * 10
    penalty = (player_lvl - monster_lvl) * 5
    return base_exp - penalty if base_exp > penalty else 1


def evaluate_ai_aggro(current_aggro: int, player_threat: int) -> int:
    """Accumulate a player's threat into a monster's aggro score."""

    _require(current_aggro >= 0, "current_aggro must be non-negative")
    _require(player_threat >= 0, "player_threat must be non-negative")
    return current_aggro + player_threat


def calc_pathfinding_step(cur_x: int, target_x: int) -> int:
    """Move one coordinate step toward the target, or stay when aligned."""

    _require(cur_x >= 0, "cur_x must be non-negative")
    _require(target_x >= 0, "target_x must be non-negative")
    if target_x > cur_x:
        return cur_x + 1
    if target_x < cur_x:
        return cur_x - 1
    return cur_x


def roll_loot_rarity(roll_value: int) -> int:
    """Map an inclusive 0..100 roll to rarity tier 1..5."""

    _require(0 <= roll_value <= 100, "roll_value must be between 0 and 100")
    if roll_value > 95:
        return 5
    if roll_value > 80:
        return 4
    if roll_value > 50:
        return 3
    if roll_value > 20:
        return 2
    return 1


def calc_inventory_weight(current_weight: int, item_weight: int) -> int:
    """Add an item's weight to the current inventory weight."""

    _require(current_weight >= 0, "current_weight must be non-negative")
    _require(item_weight >= 0, "item_weight must be non-negative")
    return current_weight + item_weight


def apply_health_potion(current_hp: int, max_hp: int, heal_amount: int) -> int:
    """Apply potion healing without exceeding maximum health."""

    _require(current_hp >= 0, "current_hp must be non-negative")
    _require(max_hp > 0, "max_hp must be positive")
    _require(heal_amount >= 0, "heal_amount must be non-negative")
    return min(current_hp + heal_amount, max_hp)


def calc_combo_multiplier(combo_count: int) -> int:
    """Return a percentage multiplier: 100% plus 10% per combo hit."""

    _require(combo_count >= 0, "combo_count must be non-negative")
    return 100 + combo_count * 10
