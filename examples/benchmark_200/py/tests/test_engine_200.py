import sys
from pathlib import Path
sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
"""Unit tests for all 200 engine functions and seven domain models."""

import inspect
import unittest
from dataclasses import is_dataclass

import engine_200 as engine
from engine_200 import (
    BaseAttributes,
    DerivedStats,
    Entity,
    Point,
    Rect,
    StatusEffect,
    Velocity,
)


class DomainModelTests(unittest.TestCase):
    def test_models_are_public_dataclasses(self) -> None:
        model_types = (
            Point,
            Velocity,
            Rect,
            BaseAttributes,
            DerivedStats,
            StatusEffect,
            Entity,
        )
        self.assertTrue(all(is_dataclass(model_type) for model_type in model_types))

        entity = Entity(
            id=7,
            pos=Point(3, 4),
            base_attr=BaseAttributes(10, 11, 12, 13),
            stats=DerivedStats(
                max_hp=100,
                max_mana=80,
                max_stamina=60,
                phys_atk=20,
                magic_atk=21,
                armor=22,
                magic_res=23,
                crit_rate=500,
                crit_dmg_pct=150,
                dodge_rate=300,
                move_speed=10,
            ),
            status=StatusEffect(1, 2, 3, 4, 5),
            curr_hp=90,
            curr_mana=70,
            curr_stamina=50,
            gold=40,
            level=3,
            exp=200,
            weight=9,
            max_weight=20,
            is_alive=True,
        )

        self.assertEqual(entity.pos, Point(3, 4))
        self.assertEqual(Velocity(1, -1).dy, -1)
        self.assertEqual(Rect(0, 1, 2, 3).max_y, 3)

    def test_entity_defaults_do_not_share_nested_state(self) -> None:
        first = Entity()
        second = Entity()
        self.assertIsNot(first.pos, second.pos)
        self.assertIsNot(first.status, second.status)


class PublicApiTests(unittest.TestCase):
    def test_root_exports_exactly_200_functions_and_7_models(self) -> None:
        self.assertEqual(len(engine.__all__), 207)
        self.assertEqual(len(set(engine.__all__)), 207)
        self.assertTrue(all(hasattr(engine, name) for name in engine.__all__))

        exported_functions = [
            name for name in engine.__all__ if inspect.isfunction(getattr(engine, name))
        ]
        exported_models = [
            name for name in engine.__all__ if inspect.isclass(getattr(engine, name))
        ]
        self.assertEqual(len(exported_functions), 200)
        self.assertEqual(len(exported_models), 7)


class EngineFunctionTests(unittest.TestCase):
    def test_tier0_covers_functions_001_through_040(self) -> None:
        self.assertEqual(engine.calc_manhattan_dist(1, 2, 4, 6), 7)
        self.assertEqual(engine.calc_chebyshev_dist(1, 2, 4, 6), 4)
        self.assertEqual(engine.calc_euclidean_sq(1, 2, 4, 6), 25)
        self.assertTrue(engine.check_in_radius_sq(25, 5))
        self.assertEqual(engine.bresenham_step_x(1, 4), 1)
        self.assertEqual(engine.bresenham_step_y(4, 1), -1)
        self.assertEqual(engine.pack_coords_to_hash(12, 34), 786_466)
        self.assertEqual(engine.unpack_hash_x(786_466), 12)
        self.assertEqual(engine.unpack_hash_y(786_466), 34)
        self.assertEqual(engine.wrap_map_coord_x(-1, 10), 9)
        self.assertEqual(engine.wrap_map_coord_y(11, 10), 1)
        self.assertEqual(engine.clamp_to_bounds_x(-3, 10), 0)
        self.assertEqual(engine.clamp_to_bounds_y(12, 10), 9)
        self.assertTrue(engine.check_aabb_overlap(0, 0, 5, 5, 5, 5, 8, 8))
        self.assertEqual(engine.calc_grid_index(3, 2, 10), 23)
        self.assertEqual(engine.calc_line_of_sight_cost(10, 25), 2)

        expected = [
            0,
            6,
            4,
            18,
            20,
            8,
            2,
            2,
            2,
            3,
            262_146,
            6,
            8,
            20,
            6,
            2,
            2,
            2,
            0,
            0,
            5,
            12,
            2,
            6,
        ]
        inputs = [(4, 2)] * 24
        inputs[9] = (2, 4)

        for index, expected_result, args in zip(range(17, 41), expected, inputs):
            with self.subTest(function=index):
                function = getattr(engine, f"spatial_subsys_{index:03d}")
                self.assertEqual(function(*args), expected_result)

    def test_tier1_covers_functions_041_through_090(self) -> None:
        self.assertEqual(engine.calc_max_hp_from_vit(100, 10, 5), 210)
        self.assertEqual(engine.calc_max_mana_from_int(100, 10), 250)
        self.assertEqual(engine.calc_phys_atk_from_str(10, 6), 23)
        self.assertEqual(engine.calc_armor_damage_reduction(100, 100), 50)
        self.assertEqual(engine.calc_magic_res_reduction(200, 100), 100)
        self.assertEqual(engine.calc_elemental_multiplier(0, 1, 100), 150)
        self.assertEqual(engine.calc_crit_chance_clamped(9_500, 100), 10_000)
        self.assertEqual(engine.calc_effective_penetration(100, 20, 10), 70)
        self.assertEqual(engine.calc_overburden_penalty(11, 10, 100), 50)
        self.assertEqual(engine.calc_lifesteal_conversion(200, 25), 50)

        expected = [
            6,
            4,
            20,
            1,
            4,
            4,
            262_146,
            2,
            6,
            1,
            1,
            2,
            2,
            0,
            14,
            0,
            4,
            10,
            8,
            22,
            24,
            12,
            6,
            6,
            6,
            5,
            262_150,
            10,
            12,
            24,
            10,
            6,
            6,
            5,
            5,
            4,
            9,
            16,
            6,
            10,
        ]

        for index, expected_result in zip(range(51, 91), expected):
            with self.subTest(function=index):
                function = getattr(engine, f"stat_subsys_{index:03d}")
                args = (2, 4) if index == 76 else (4, 2)
                self.assertEqual(function(*args), expected_result)

    def test_tier2_covers_functions_091_through_140(self) -> None:
        self.assertEqual(engine.resolve_physical_hit(100, 100), 50)
        self.assertEqual(engine.resolve_crit_hit(100, True, 150), 150)
        self.assertEqual(engine.apply_dual_shield_absorption(100, 30), 70)
        self.assertEqual(engine.apply_bleed_stack_tick(3, False), 15)
        self.assertEqual(engine.apply_poison_exponential_decay(10, 3), 20)
        self.assertEqual(engine.apply_burn_max_hp_tick(1_000, 5), 35)
        self.assertTrue(engine.check_freeze_threshold(5))
        self.assertEqual(engine.calc_stamina_skill_cost(100, 25), 75)
        self.assertEqual(engine.calc_mana_overload_burst(99), 9)
        self.assertTrue(engine.calc_poise_break_stun(10, 10))

        for index in range(101, 141):
            with self.subTest(function=index):
                function = getattr(engine, f"combat_subsys_{index:03d}")
                self.assertGreaterEqual(function(10, 0), 1)

    def test_tier3_covers_functions_141_through_180(self) -> None:
        self.assertEqual(engine.calc_threat_decay_by_dist(100, 5), 66)
        self.assertEqual(engine.evaluate_ai_behavior_state(19, 0), 1)
        self.assertEqual(engine.calc_weighted_loot_tier(9_900), 5)
        self.assertEqual(engine.scale_drop_by_luck(9_000, 100), 9_200)
        self.assertEqual(engine.calc_exp_gain_with_level_diff(100, 20, 9), 10)
        self.assertEqual(engine.calc_exp_to_next_level(3), 2_400)
        self.assertEqual(engine.calc_shop_inflation_price(100, 5), 110)

        for index in range(148, 181):
            with self.subTest(function=index):
                function = getattr(engine, f"ai_loot_subsys_{index:03d}")
                self.assertGreaterEqual(function(10, 5), 10)

    def test_tier4_covers_functions_181_through_200(self) -> None:
        self.assertEqual(engine.determine_turn_initiative(10, 5), 25)
        self.assertEqual(engine.resolve_boss_phase_transition(67), 1)
        self.assertEqual(engine.resolve_boss_phase_transition(66), 2)
        self.assertEqual(engine.resolve_boss_phase_transition(33), 2)
        self.assertEqual(engine.resolve_boss_phase_transition(32), 3)
        self.assertEqual(engine.simulate_single_turn_exchange(100, 100, True), 105)
        self.assertEqual(engine.resolve_terrain_hazard_tick(0, 1_000), 0)
        self.assertEqual(engine.resolve_terrain_hazard_tick(1, 1_000), 60)
        self.assertEqual(engine.resolve_terrain_hazard_tick(2, 1_000), 0)
        self.assertEqual(engine.resolve_terrain_hazard_tick(3, 1_000), 0)

        expected = [
            2,
            3,
            1_638_410,
            2,
            5,
            10,
            2,
            11,
            9,
            8,
            1,
            2_400,
            21,
            35,
            11,
            52,
        ]
        for index, expected_result in zip(range(185, 201), expected):
            with self.subTest(function=index):
                function = getattr(engine, f"world_engine_{index}")
                self.assertEqual(function(25, 10), expected_result)

    def test_tier4_contract_boundaries(self) -> None:
        with self.assertRaises(AssertionError):
            engine.determine_turn_initiative(-1, 0)
        with self.assertRaises(AssertionError):
            engine.determine_turn_initiative(0, 20)
        with self.assertRaises(AssertionError):
            engine.resolve_boss_phase_transition(101)
        with self.assertRaises(AssertionError):
            engine.resolve_terrain_hazard_tick(4, 100)
        with self.assertRaises(AssertionError):
            engine.world_engine_200(-1, 10)
        with self.assertRaises(AssertionError):
            engine.world_engine_200(1, 0)


if __name__ == "__main__":
    unittest.main()

