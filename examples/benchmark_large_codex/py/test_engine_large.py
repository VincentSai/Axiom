"""Unit tests for every combat and world subsystem in engine_large."""

import unittest
from dataclasses import FrozenInstanceError, is_dataclass

from engine_large import (
    LootItem,
    Monster,
    Player,
    Point,
    Stats,
    apply_health_potion,
    apply_lifesteal,
    apply_poison_tick,
    apply_shield_absorption,
    calc_combo_multiplier,
    calc_crit_damage,
    calc_dodge_chance,
    calc_elemental_resistance,
    calc_exp_reward,
    calc_gold_drop,
    calc_inventory_weight,
    calc_mana_cost,
    calc_pathfinding_step,
    calc_stamina_drain,
    evaluate_ai_aggro,
    roll_loot_rarity,
)


class DataModelTests(unittest.TestCase):
    def setUp(self) -> None:
        self.point = Point(3, 4)
        self.stats = Stats(80, 100, 12, 7, 20, 30, 15, 25)

    def test_models_are_dataclasses_with_expected_values(self) -> None:
        loot = LootItem(1, "Ruby", 2, 4)
        player = Player(2, self.point, self.stats, 150, 3, 40)
        monster = Monster(3, self.point, self.stats, 10, True)

        for model in (self.point, self.stats, loot, player, monster):
            self.assertTrue(is_dataclass(model))
        self.assertEqual((self.point.x, self.point.y), (3, 4))
        self.assertEqual(loot.name, "Ruby")
        self.assertEqual(player.stats.max_hp, 100)
        self.assertTrue(monster.is_alive)

    def test_models_are_immutable(self) -> None:
        models_and_fields = (
            (self.point, "x"),
            (self.stats, "hp"),
            (LootItem(1, "Ruby", 2, 4), "rarity"),
            (Player(2, self.point, self.stats, 150, 3, 40), "gold"),
            (Monster(3, self.point, self.stats, 10, True), "is_alive"),
        )
        for model, field in models_and_fields:
            with self.subTest(model=type(model).__name__):
                with self.assertRaises(FrozenInstanceError):
                    setattr(model, field, None)

    def test_frozen_models_support_value_equality_and_hashing(self) -> None:
        self.assertEqual(Point(3, 4), self.point)
        self.assertEqual(len({Point(3, 4), self.point}), 1)


class CombatSubsystemTests(unittest.TestCase):
    def test_crit_damage(self) -> None:
        self.assertEqual(calc_crit_damage(20, False, 3), 20)
        self.assertEqual(calc_crit_damage(20, True, 3), 60)
        self.assertEqual(calc_crit_damage(0, True, 5), 0)
        self.assertEqual(calc_crit_damage(7, True, 1), 7)
        with self.assertRaises(ValueError):
            calc_crit_damage(-1, True, 2)
        with self.assertRaises(ValueError):
            calc_crit_damage(1, True, 0)

    def test_elemental_resistance(self) -> None:
        self.assertEqual(calc_elemental_resistance(101, 25), 76)
        self.assertEqual(calc_elemental_resistance(50, 0), 50)
        self.assertEqual(calc_elemental_resistance(50, 100), 0)
        self.assertEqual(calc_elemental_resistance(0, 75), 0)
        for invalid in (-1, 101):
            with self.subTest(res_percent=invalid), self.assertRaises(ValueError):
                calc_elemental_resistance(10, invalid)

    def test_lifesteal(self) -> None:
        self.assertEqual(apply_lifesteal(40, 100, 50, 20), 50)
        self.assertEqual(apply_lifesteal(95, 100, 50, 20), 100)
        self.assertEqual(apply_lifesteal(40, 100, 3, 20), 40)
        self.assertEqual(apply_lifesteal(100, 100, 999, 100), 100)
        self.assertEqual(apply_lifesteal(101, 100, 10, 10), 100)
        for args in ((-1, 100, 10, 10), (1, 0, 1, 1), (1, 2, -1, 1), (1, 2, 1, -1)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                apply_lifesteal(*args)

    def test_dodge_chance(self) -> None:
        self.assertEqual(calc_dodge_chance(80, 30), 0)
        self.assertEqual(calc_dodge_chance(50, 50), 0)
        self.assertEqual(calc_dodge_chance(50, 75), 25)
        self.assertEqual(calc_dodge_chance(1, 500), 100)
        with self.assertRaises(ValueError):
            calc_dodge_chance(-1, 0)

    def test_poison_tick(self) -> None:
        self.assertEqual(apply_poison_tick(20, 5), 15)
        self.assertEqual(apply_poison_tick(20, 20), 0)
        self.assertEqual(apply_poison_tick(20, 30), 0)
        self.assertEqual(apply_poison_tick(0, 0), 0)
        with self.assertRaises(ValueError):
            apply_poison_tick(10, -1)

    def test_shield_absorption(self) -> None:
        self.assertEqual(apply_shield_absorption(5, 20), 15)
        self.assertEqual(apply_shield_absorption(20, 20), 0)
        self.assertEqual(apply_shield_absorption(30, 20), 0)
        self.assertEqual(apply_shield_absorption(0, 20), 20)
        with self.assertRaises(ValueError):
            apply_shield_absorption(-1, 20)

    def test_mana_cost(self) -> None:
        self.assertEqual(calc_mana_cost(30, 5), 25)
        self.assertEqual(calc_mana_cost(30, 30), 0)
        self.assertEqual(calc_mana_cost(30, 99), 0)
        self.assertEqual(calc_mana_cost(0, 0), 0)
        with self.assertRaises(ValueError):
            calc_mana_cost(30, -1)

    def test_stamina_drain(self) -> None:
        self.assertEqual(calc_stamina_drain(30, 5), 25)
        self.assertEqual(calc_stamina_drain(30, 30), 0)
        self.assertEqual(calc_stamina_drain(30, 99), 0)
        self.assertEqual(calc_stamina_drain(0, 0), 0)
        with self.assertRaises(ValueError):
            calc_stamina_drain(-1, 0)

    def test_health_potion(self) -> None:
        self.assertEqual(apply_health_potion(30, 100, 20), 50)
        self.assertEqual(apply_health_potion(90, 100, 20), 100)
        self.assertEqual(apply_health_potion(100, 100, 20), 100)
        self.assertEqual(apply_health_potion(30, 100, 0), 30)
        self.assertEqual(apply_health_potion(101, 100, 10), 100)
        for args in ((-1, 100, 10), (0, 0, 10), (1, 10, -1)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                apply_health_potion(*args)

    def test_combo_multiplier(self) -> None:
        self.assertEqual(calc_combo_multiplier(0), 100)
        self.assertEqual(calc_combo_multiplier(1), 110)
        self.assertEqual(calc_combo_multiplier(25), 350)
        with self.assertRaises(ValueError):
            calc_combo_multiplier(-1)


class WorldSubsystemTests(unittest.TestCase):
    def test_gold_drop(self) -> None:
        self.assertEqual(calc_gold_drop(100, 25), 125)
        self.assertEqual(calc_gold_drop(101, 25), 126)
        self.assertEqual(calc_gold_drop(100, 0), 100)
        self.assertEqual(calc_gold_drop(0, 999), 0)
        with self.assertRaises(ValueError):
            calc_gold_drop(-1, 10)

    def test_exp_reward(self) -> None:
        self.assertEqual(calc_exp_reward(100, 5, 5), 100)
        self.assertEqual(calc_exp_reward(100, 5, 8), 130)
        self.assertEqual(calc_exp_reward(100, 8, 5), 85)
        self.assertEqual(calc_exp_reward(15, 8, 5), 1)
        self.assertEqual(calc_exp_reward(10, 8, 5), 1)
        self.assertEqual(calc_exp_reward(0, 1, 1), 0)
        for args in ((-1, 1, 1), (10, 0, 1), (10, 1, 0)):
            with self.subTest(args=args), self.assertRaises(ValueError):
                calc_exp_reward(*args)

    def test_ai_aggro(self) -> None:
        self.assertEqual(evaluate_ai_aggro(30, 20), 50)
        self.assertEqual(evaluate_ai_aggro(0, 0), 0)
        self.assertEqual(evaluate_ai_aggro(30, 0), 30)
        with self.assertRaises(ValueError):
            evaluate_ai_aggro(0, -1)

    def test_pathfinding_step(self) -> None:
        self.assertEqual(calc_pathfinding_step(5, 10), 6)
        self.assertEqual(calc_pathfinding_step(5, 1), 4)
        self.assertEqual(calc_pathfinding_step(5, 5), 5)
        self.assertEqual(calc_pathfinding_step(0, 0), 0)
        self.assertEqual(calc_pathfinding_step(0, 1), 1)
        with self.assertRaises(ValueError):
            calc_pathfinding_step(-1, 0)

    def test_loot_rarity_all_thresholds(self) -> None:
        cases = {
            0: 1,
            20: 1,
            21: 2,
            50: 2,
            51: 3,
            80: 3,
            81: 4,
            95: 4,
            96: 5,
            100: 5,
        }
        for roll, expected in cases.items():
            with self.subTest(roll=roll):
                self.assertEqual(roll_loot_rarity(roll), expected)
        for invalid in (-1, 101):
            with self.subTest(roll=invalid), self.assertRaises(ValueError):
                roll_loot_rarity(invalid)

    def test_inventory_weight(self) -> None:
        self.assertEqual(calc_inventory_weight(10, 5), 15)
        self.assertEqual(calc_inventory_weight(0, 0), 0)
        self.assertEqual(calc_inventory_weight(10, 0), 10)
        with self.assertRaises(ValueError):
            calc_inventory_weight(10, -1)


if __name__ == "__main__":
    unittest.main(verbosity=2)
