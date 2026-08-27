"""Domain data models for the roguelike world simulation engine."""

from dataclasses import dataclass, field


@dataclass(slots=True)
class Point:
    """A two-dimensional grid position."""

    x: int = 0
    y: int = 0


@dataclass(slots=True)
class Velocity:
    """A two-dimensional velocity vector."""

    dx: int = 0
    dy: int = 0


@dataclass(slots=True)
class Rect:
    """An inclusive axis-aligned rectangle."""

    min_x: int = 0
    min_y: int = 0
    max_x: int = 0
    max_y: int = 0


@dataclass(slots=True)
class BaseAttributes:
    """An entity's unmodified attributes."""

    strength: int = 0
    agility: int = 0
    intelligence: int = 0
    vitality: int = 0


@dataclass(slots=True)
class DerivedStats:
    """Attributes derived from an entity's base values and equipment."""

    max_hp: int = 0
    max_mana: int = 0
    max_stamina: int = 0
    phys_atk: int = 0
    magic_atk: int = 0
    armor: int = 0
    magic_res: int = 0
    crit_rate: int = 0
    crit_dmg_pct: int = 0
    dodge_rate: int = 0
    move_speed: int = 0


@dataclass(slots=True)
class StatusEffect:
    """Stack and duration state for combat status effects."""

    bleed_stacks: int = 0
    poison_duration: int = 0
    burn_damage: int = 0
    freeze_stacks: int = 0
    shield_hp: int = 0


@dataclass(slots=True)
class Entity:
    """Complete pure-data representation of a world entity."""

    id: int = 0
    pos: Point = field(default_factory=Point)
    base_attr: BaseAttributes = field(default_factory=BaseAttributes)
    stats: DerivedStats = field(default_factory=DerivedStats)
    status: StatusEffect = field(default_factory=StatusEffect)
    curr_hp: int = 0
    curr_mana: int = 0
    curr_stamina: int = 0
    gold: int = 0
    level: int = 0
    exp: int = 0
    weight: int = 0
    max_weight: int = 0
    is_alive: bool = False


__all__ = [
    "Point",
    "Velocity",
    "Rect",
    "BaseAttributes",
    "DerivedStats",
    "StatusEffect",
    "Entity",
]
