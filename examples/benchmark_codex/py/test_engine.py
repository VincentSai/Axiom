import unittest

from engine import (
    Item,
    Monster,
    Player,
    Point,
    Stats,
    apply_damage,
    calculate_damage,
    level_up,
    try_move,
)


class TestRoguelikeEngine(unittest.TestCase):
    def test_entities_expose_the_required_data(self) -> None:
        stats = Stats(hp=20, max_hp=20, attack=5, defense=2)
        item = Item(id=7, name="Health Potion", value=25)
        player = Player(id=1, pos=Point(0, 0), stats=stats, exp=0, level=1)
        monster = Monster(
            id=11,
            pos=Point(3, 4),
            stats=Stats(hp=12, max_hp=12, attack=4, defense=1),
            is_alive=True,
        )

        self.assertEqual(item.name, "Health Potion")
        self.assertEqual(player.stats, stats)
        self.assertEqual(monster.pos, Point(3, 4))
        self.assertTrue(monster.is_alive)

    def test_try_move_returns_an_in_bounds_destination(self) -> None:
        self.assertEqual(try_move(Point(2, 2), 1, -1, 5, 5), Point(3, 1))

    def test_try_move_keeps_position_for_each_out_of_bounds_edge(self) -> None:
        self.assertEqual(try_move(Point(0, 0), -1, 0, 5, 5), Point(0, 0))
        self.assertEqual(try_move(Point(0, 0), 0, -1, 5, 5), Point(0, 0))
        self.assertEqual(try_move(Point(4, 4), 1, 0, 5, 5), Point(4, 4))
        self.assertEqual(try_move(Point(4, 4), 0, 1, 5, 5), Point(4, 4))

    def test_calculate_damage_applies_defense_and_minimum_damage(self) -> None:
        self.assertEqual(calculate_damage(10, 3), 7)
        self.assertEqual(calculate_damage(2, 5), 1)
        self.assertEqual(calculate_damage(5, 5), 1)

    def test_apply_damage_never_goes_below_zero(self) -> None:
        self.assertEqual(apply_damage(20, 7), 13)
        self.assertEqual(apply_damage(5, 10), 0)
        self.assertEqual(apply_damage(5, 5), 0)

    def test_level_up_accumulates_insufficient_experience(self) -> None:
        player = Player(
            id=1,
            pos=Point(0, 0),
            stats=Stats(hp=20, max_hp=20, attack=5, defense=2),
            exp=30,
            level=1,
        )

        self.assertEqual(
            level_up(player, 40),
            Player(
                id=1,
                pos=Point(0, 0),
                stats=Stats(hp=20, max_hp=20, attack=5, defense=2),
                exp=70,
                level=1,
            ),
        )

    def test_level_up_spends_experience_and_improves_stats(self) -> None:
        player = Player(
            id=1,
            pos=Point(0, 0),
            stats=Stats(hp=12, max_hp=20, attack=5, defense=2),
            exp=0,
            level=1,
        )

        self.assertEqual(
            level_up(player, 120),
            Player(
                id=1,
                pos=Point(0, 0),
                stats=Stats(hp=30, max_hp=30, attack=7, defense=3),
                exp=20,
                level=2,
            ),
        )


if __name__ == "__main__":
    unittest.main()
