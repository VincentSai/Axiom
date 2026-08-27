from dataclasses import dataclass, field
from typing import List, Optional, Union

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
    inventory: List[Item] = field(default_factory=list)
    max_inventory: int = 5
    exp: int = 0
    level: int = 1

@dataclass(frozen=True)
class Monster:
    id: int
    pos: Point
    stats: Stats
    is_alive: bool = True

def try_move(pos: Point, dx: int, dy: int, map_width: int, map_height: int) -> Point:
    new_x = pos.x + dx
    new_y = pos.y + dy
    if 0 <= new_x < map_width and 0 <= new_y < map_height:
        return Point(new_x, new_y)
    return pos

def calculate_damage(attacker_atk: int, defender_def: int) -> int:
    raw = attacker_atk - defender_def
    return raw if raw > 1 else 1

def apply_damage(current_hp: int, damage: int) -> int:
    remaining = current_hp - damage
    return remaining if remaining > 0 else 0

def level_up(player: Player, exp_gained: int) -> Player:
    new_exp = player.exp + exp_gained
    if new_exp >= player.level * 100:
        new_stats = Stats(
            hp=player.stats.max_hp + 10,
            max_hp=player.stats.max_hp + 10,
            attack=player.stats.attack + 2,
            defense=player.stats.defense + 1
        )
        return Player(
            id=player.id,
            pos=player.pos,
            stats=new_stats,
            inventory=player.inventory,
            max_inventory=player.max_inventory,
            exp=new_exp - player.level * 100,
            level=player.level + 1
        )
    return Player(
        id=player.id,
        pos=player.pos,
        stats=player.stats,
        inventory=player.inventory,
        max_inventory=player.max_inventory,
        exp=new_exp,
        level=player.level
    )
