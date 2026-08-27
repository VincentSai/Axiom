"""Comprehensive unit tests for all 200 public engine functions."""

import unittest
from unittest.mock import patch

import engine_200 as engine


class Engine200Tests(unittest.TestCase):
    """One independently reported test for each public numbered function."""


def _make_math_test(number: int):
    function_name = f"math_func_{number:03d}"

    def test(self):
        function = getattr(engine, function_name)
        self.assertEqual(function(7, 5), 12)
        self.assertEqual(function(0, 0), 0)
        with self.assertRaises(ValueError):
            function(-1, 0)
        with self.assertRaises(ValueError):
            function(0, -1)

    return test


def _make_stat_test(number: int):
    function_name = f"stat_func_{number:03d}"
    dependency_number = ((number - 40) % 40) + 1
    dependency_name = f"math_func_{dependency_number:03d}"

    def test(self):
        function = getattr(engine, function_name)
        dependency = getattr(engine, dependency_name)
        with patch.object(engine, dependency_name, wraps=dependency) as delegated:
            self.assertEqual(function(10, 4), 14)
            delegated.assert_called_once_with(0, 4)
        self.assertEqual(function(0, 0), 0)
        with self.assertRaises(ValueError):
            function(-1, 0)
        with self.assertRaises(ValueError):
            function(0, -1)

    return test


def _make_combat_test(number: int):
    function_name = f"combat_func_{number:03d}"
    dependency_number = 41 + ((number - 90) % 50)
    dependency_name = f"stat_func_{dependency_number:03d}"

    def test(self):
        function = getattr(engine, function_name)
        dependency = getattr(engine, dependency_name)
        with patch.object(engine, dependency_name, wraps=dependency) as delegated:
            self.assertEqual(function(12, 5), 7)
            delegated.assert_called_once_with(5, 0)
        self.assertEqual(function(5, 5), 1)
        self.assertEqual(function(3, 8), 1)
        with self.assertRaises(ValueError):
            function(-1, 0)
        with self.assertRaises(ValueError):
            function(0, -1)

    return test


def _make_world_test(number: int):
    function_name = f"world_func_{number:03d}"
    dependency_number = 91 + ((number - 140) % 50)
    dependency_name = f"combat_func_{dependency_number:03d}"

    def test(self):
        function = getattr(engine, function_name)
        dependency = getattr(engine, dependency_name)
        with patch.object(engine, dependency_name, wraps=dependency) as delegated:
            self.assertEqual(function(10, 4), 14)
            delegated.assert_called_once_with(4, 0)
        self.assertEqual(function(10, 0), 11)
        with self.assertRaises(ValueError):
            function(-1, 0)
        with self.assertRaises(ValueError):
            function(0, -1)

    return test


def _make_engine_test(number: int):
    function_name = f"engine_func_{number:03d}"
    dependency_number = 141 + ((number - 180) % 40)
    dependency_name = f"world_func_{dependency_number:03d}"

    def test(self):
        function = getattr(engine, function_name)
        dependency = getattr(engine, dependency_name)
        with patch.object(engine, dependency_name, wraps=dependency) as delegated:
            self.assertEqual(function(17, 5), 2)
            delegated.assert_called_once_with(17, 0)
        self.assertEqual(function(0, 1), 0)
        with self.assertRaises(ValueError):
            function(-1, 5)
        with self.assertRaises(ValueError):
            function(5, 0)
        with self.assertRaises(ValueError):
            function(5, -1)

    return test


_TEST_FACTORIES = (
    (range(1, 41), _make_math_test),
    (range(41, 91), _make_stat_test),
    (range(91, 141), _make_combat_test),
    (range(141, 181), _make_world_test),
    (range(181, 201), _make_engine_test),
)

for _numbers, _factory in _TEST_FACTORIES:
    for _number in _numbers:
        _test = _factory(_number)
        _test.__name__ = f"test_{_number:03d}"
        _test.__qualname__ = f"Engine200Tests.test_{_number:03d}"
        setattr(Engine200Tests, _test.__name__, _test)

del _numbers, _factory, _number, _test


if __name__ == "__main__":
    suite = unittest.defaultTestLoader.loadTestsFromTestCase(Engine200Tests)
    result = unittest.TextTestRunner(verbosity=1).run(suite)
    raise SystemExit(not result.wasSuccessful())

