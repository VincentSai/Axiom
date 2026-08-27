#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stats {
    pub hp: i64,
    pub max_hp: i64,
    pub attack: i64,
    pub defense: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub value: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Player {
    pub id: i64,
    pub pos: Point,
    pub stats: Stats,
    pub exp: i64,
    pub level: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Monster {
    pub id: i64,
    pub pos: Point,
    pub stats: Stats,
    pub is_alive: bool,
}

pub fn try_move(pos: &Point, dx: i64, dy: i64, map_width: i64, map_height: i64) -> Point {
    let destination = Point {
        x: pos.x + dx,
        y: pos.y + dy,
    };

    if destination.x >= 0
        && destination.x < map_width
        && destination.y >= 0
        && destination.y < map_height
    {
        destination
    } else {
        pos.clone()
    }
}

pub fn calculate_damage(attacker_atk: i64, defender_def: i64) -> i64 {
    (attacker_atk - defender_def).max(1)
}

pub fn apply_damage(current_hp: i64, damage: i64) -> i64 {
    (current_hp - damage).max(0)
}

pub fn level_up(player: &Player, exp_gained: i64) -> Player {
    let new_exp = player.exp + exp_gained;
    let threshold = player.level * 100;

    if new_exp >= threshold {
        Player {
            id: player.id,
            pos: player.pos.clone(),
            stats: Stats {
                hp: player.stats.max_hp + 10,
                max_hp: player.stats.max_hp + 10,
                attack: player.stats.attack + 2,
                defense: player.stats.defense + 1,
            },
            exp: new_exp - threshold,
            level: player.level + 1,
        }
    } else {
        Player {
            exp: new_exp,
            ..player.clone()
        }
    }
}
