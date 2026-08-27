"""Core domain entities and rules for a small roguelike game."""

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


@dataclass(frozen=True)
class Item:
    id: int
    name: str
    value: int


@dataclass(frozen=True)
class Player:
    id: int
    pos: Point
    stats: Stats
    exp: int
    level: int


@dataclass(frozen=True)
class Monster:
    id: int
    pos: Point
    stats: Stats
    is_alive: bool


def try_move(
    pos: Point,
    dx: int,
    dy: int,
    map_width: int,
    map_height: int,
) -> Point:
    """Return the destination when it is in bounds, otherwise keep ``pos``."""
    destination = Point(pos.x + dx, pos.y + dy)
    if 0 <= destination.x < map_width and 0 <= destination.y < map_height:
        return destination
    return pos


def calculate_damage(attacker_atk: int, defender_def: int) -> int:
    """Calculate damage after defense, with a minimum hit of one."""
    return max(attacker_atk - defender_def, 1)


def apply_damage(current_hp: int, damage: int) -> int:
    """Reduce HP by ``damage`` without allowing it to fall below zero."""
    return max(current_hp - damage, 0)


def level_up(player: Player, exp_gained: int) -> Player:
    """Add experience and apply at most one level-up when the threshold is met."""
    new_exp = player.exp + exp_gained
    threshold = player.level * 100

    if new_exp < threshold:
        return Player(
            id=player.id,
            pos=player.pos,
            stats=player.stats,
            exp=new_exp,
            level=player.level,
        )

    new_max_hp = player.stats.max_hp + 10
    return Player(
        id=player.id,
        pos=player.pos,
        stats=Stats(
            hp=new_max_hp,
            max_hp=new_max_hp,
            attack=player.stats.attack + 2,
            defense=player.stats.defense + 1,
        ),
        exp=new_exp - threshold,
        level=player.level + 1,
    )
