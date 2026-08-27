//! Pure, five-tier roguelike engine benchmark.

/// A two-dimensional grid position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

/// A two-dimensional velocity vector.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Velocity {
    pub dx: i64,
    pub dy: i64,
}

/// An inclusive axis-aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub min_x: i64,
    pub min_y: i64,
    pub max_x: i64,
    pub max_y: i64,
}

/// An entity's unmodified attributes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BaseAttributes {
    pub strength: i64,
    pub agility: i64,
    pub intelligence: i64,
    pub vitality: i64,
}

/// Attributes derived from an entity's base values and equipment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DerivedStats {
    pub max_hp: i64,
    pub max_mana: i64,
    pub max_stamina: i64,
    pub phys_atk: i64,
    pub magic_atk: i64,
    pub armor: i64,
    pub magic_res: i64,
    pub crit_rate: i64,
    pub crit_dmg_pct: i64,
    pub dodge_rate: i64,
    pub move_speed: i64,
}

/// Stack and duration state for combat status effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct StatusEffect {
    pub bleed_stacks: i64,
    pub poison_duration: i64,
    pub burn_damage: i64,
    pub freeze_stacks: i64,
    pub shield_hp: i64,
}

/// Complete pure-data representation of a world entity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Entity {
    pub id: i64,
    pub pos: Point,
    pub base_attr: BaseAttributes,
    pub stats: DerivedStats,
    pub status: StatusEffect,
    pub curr_hp: i64,
    pub curr_mana: i64,
    pub curr_stamina: i64,
    pub gold: i64,
    pub level: i64,
    pub exp: i64,
    pub weight: i64,
    pub max_weight: i64,
    pub is_alive: bool,
}

pub mod tier0_spatial;
pub mod tier1_stats;
pub mod tier2_combat;
pub mod tier3_ai_loot;
pub mod tier4_world;

pub use tier0_spatial::*;
pub use tier1_stats::*;
pub use tier2_combat::*;
pub use tier3_ai_loot::*;
pub use tier4_world::*;
