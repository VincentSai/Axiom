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
pub struct Player {
    pub id: i64,
    pub pos: Point,
    pub stats: Stats,
    pub exp: i64,
    pub level: i64,
}

pub fn try_move(pos: &Point, dx: i64, dy: i64, map_width: i64, map_height: i64) -> Point {
    let new_x = pos.x + dx;
    let new_y = pos.y + dy;
    if new_x >= 0 && new_x < map_width && new_y >= 0 && new_y < map_height {
        Point { x: new_x, y: new_y }
    } else {
        pos.clone()
    }
}

pub fn calculate_damage(attacker_atk: i64, defender_def: i64) -> i64 {
    let raw = attacker_atk - defender_def;
    if raw > 1 { raw } else { 1 }
}

pub fn apply_damage(current_hp: i64, damage: i64) -> i64 {
    let remaining = current_hp - damage;
    if remaining > 0 { remaining } else { 0 }
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
