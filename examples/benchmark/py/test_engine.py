import unittest
from engine import Point, Stats, Player, try_move, calculate_damage, apply_damage, level_up

class TestRoguelike(unittest.TestCase):
    def test_movement(self):
        p = Point(0, 0)
        self.assertEqual(try_move(p, 1, 0, 10, 10), Point(1, 0))
        self.assertEqual(try_move(p, -1, 0, 10, 10), Point(0, 0))

    def test_combat(self):
        self.assertEqual(calculate_damage(10, 3), 7)
        self.assertEqual(calculate_damage(2, 5), 1)
        self.assertEqual(apply_damage(20, 7), 13)
        self.assertEqual(apply_damage(5, 10), 0)

    def test_level_up(self):
        player = Player(1, Point(0, 0), Stats(20, 20, 5, 2))
        next_p = level_up(player, 120)
        self.assertEqual(next_p.level, 2)
        self.assertEqual(next_p.stats.max_hp, 30)

if __name__ == "__main__":
    unittest.main()
