#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Player {
    pub pos: Vec2,
    pub hp: i64,
    pub max_hp: i64,
    pub exp: i64,
    pub level: i64,
    pub speed: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Monster {
    pub id: i64,
    pub pos: Vec2,
    pub hp: i64,
    pub speed: f64,
    pub kind: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Projectile {
    pub pos: Vec2,
    pub vel: Vec2,
    pub damage: i64,
    pub pierce: i64,
    pub lifetime: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gem {
    pub pos: Vec2,
    pub exp_val: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    pub player: Player,
    pub score: i64,
    pub is_game_over: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlayerInput {
    pub move_x: f64,
    pub move_y: f64,
    pub is_firing: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum EntityKind {
    Player,
    Monster,
    Projectile,
    Gem,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VisualEventKind {
    Spawn,
    Despawn,
    Hit,
    Death,
    Shot,
    Pickup,
    LevelUp,
    GameOver,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntitySnapshot {
    pub id: i64,
    pub kind: EntityKind,
    pub x: f64,
    pub y: f64,
    pub hp: i64,
    pub max_hp: i64,
    pub exp: i64,
    pub level: i64,
    pub score: i64,
    pub sprite_id: i64,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisualEvent {
    pub kind: VisualEventKind,
    pub entity_id: i64,
    pub asset_id: i64,
    pub x: f64,
    pub y: f64,
    pub value: i64,
}

pub fn distance_sq(p1: Vec2, p2: Vec2) -> f64 {
    debug_assert!((p1.x >= (-1000000.0)), "requires clause failed: (p1.x >= (-1000000.0))");
    debug_assert!((p1.x <= 1000000.0), "requires clause failed: (p1.x <= 1000000.0)");
    debug_assert!((p1.y >= (-1000000.0)), "requires clause failed: (p1.y >= (-1000000.0))");
    debug_assert!((p1.y <= 1000000.0), "requires clause failed: (p1.y <= 1000000.0)");
    debug_assert!((p2.x >= (-1000000.0)), "requires clause failed: (p2.x >= (-1000000.0))");
    debug_assert!((p2.x <= 1000000.0), "requires clause failed: (p2.x <= 1000000.0)");
    debug_assert!((p2.y >= (-1000000.0)), "requires clause failed: (p2.y >= (-1000000.0))");
    debug_assert!((p2.y <= 1000000.0), "requires clause failed: (p2.y <= 1000000.0)");
    let result = {
        {
        let dx = (p1.x - p2.x);
        let dy = (p1.y - p2.y);
        ((dx * dx) + (dy * dy))
    }
    };
    debug_assert!((result >= 0.0), "ensures clause failed: (result >= 0.0)");
    debug_assert!((result == (((p1.x - p2.x) * (p1.x - p2.x)) + ((p1.y - p2.y) * (p1.y - p2.y)))), "ensures clause failed: (result == (((p1.x - p2.x) * (p1.x - p2.x)) + ((p1.y - p2.y) * (p1.y - p2.y))))");
    result
}

pub fn check_collision(p1: Vec2, r1: f64, p2: Vec2, r2: f64) -> bool {
    debug_assert!((r1 >= 0.0), "requires clause failed: (r1 >= 0.0)");
    debug_assert!((r1 <= 1000000.0), "requires clause failed: (r1 <= 1000000.0)");
    debug_assert!((r2 >= 0.0), "requires clause failed: (r2 >= 0.0)");
    debug_assert!((r2 <= 1000000.0), "requires clause failed: (r2 <= 1000000.0)");
    debug_assert!((p1.x >= (-1000000.0)), "requires clause failed: (p1.x >= (-1000000.0))");
    debug_assert!((p1.x <= 1000000.0), "requires clause failed: (p1.x <= 1000000.0)");
    debug_assert!((p1.y >= (-1000000.0)), "requires clause failed: (p1.y >= (-1000000.0))");
    debug_assert!((p1.y <= 1000000.0), "requires clause failed: (p1.y <= 1000000.0)");
    debug_assert!((p2.x >= (-1000000.0)), "requires clause failed: (p2.x >= (-1000000.0))");
    debug_assert!((p2.x <= 1000000.0), "requires clause failed: (p2.x <= 1000000.0)");
    debug_assert!((p2.y >= (-1000000.0)), "requires clause failed: (p2.y >= (-1000000.0))");
    debug_assert!((p2.y <= 1000000.0), "requires clause failed: (p2.y <= 1000000.0)");
    let result = {
        (distance_sq(p1, p2) <= ((r1 + r2) * (r1 + r2)))
    };
    debug_assert!((result == (distance_sq(p1, p2) <= ((r1 + r2) * (r1 + r2)))), "ensures clause failed: (result == (distance_sq(p1, p2) <= ((r1 + r2) * (r1 + r2))))");
    result
}

pub fn hp_after_delta_unclamped(current: i64, delta: i64) -> i64 {
    debug_assert!((current >= (-1000000000)), "requires clause failed: (current >= (-1000000000))");
    debug_assert!((current <= 1000000000), "requires clause failed: (current <= 1000000000)");
    debug_assert!((delta >= (-1000000000)), "requires clause failed: (delta >= (-1000000000))");
    debug_assert!((delta <= 1000000000), "requires clause failed: (delta <= 1000000000)");
    let result = {
        (current + delta)
    };
    debug_assert!((result == (current + delta)), "ensures clause failed: (result == (current + delta))");
    debug_assert!((result >= (-2000000000)), "ensures clause failed: (result >= (-2000000000))");
    debug_assert!((result <= 2000000000), "ensures clause failed: (result <= 2000000000)");
    result
}

pub fn clamp_hp(current: i64, delta: i64, max_hp: i64) -> i64 {
    debug_assert!((current >= (-1000000000)), "requires clause failed: (current >= (-1000000000))");
    debug_assert!((current <= 1000000000), "requires clause failed: (current <= 1000000000)");
    debug_assert!((delta >= (-1000000000)), "requires clause failed: (delta >= (-1000000000))");
    debug_assert!((delta <= 1000000000), "requires clause failed: (delta <= 1000000000)");
    debug_assert!((max_hp >= 0), "requires clause failed: (max_hp >= 0)");
    debug_assert!((max_hp <= 1000000000), "requires clause failed: (max_hp <= 1000000000)");
    let result = {
        {
        let candidate = hp_after_delta_unclamped(current, delta);
        match (candidate < 0) {
        true => 0,
        false => match (candidate > max_hp) {
        true => max_hp,
        false => candidate,
    },
    }
    }
    };
    debug_assert!((result >= 0), "ensures clause failed: (result >= 0)");
    debug_assert!((result <= max_hp), "ensures clause failed: (result <= max_hp)");
    debug_assert!(((((current + delta) < 0) || ((current + delta) > max_hp)) || (result == (current + delta))), "ensures clause failed: ((((current + delta) < 0) || ((current + delta) > max_hp)) || (result == (current + delta)))");
    debug_assert!((((current + delta) >= 0) || (result == 0)), "ensures clause failed: (((current + delta) >= 0) || (result == 0))");
    debug_assert!((((current + delta) <= max_hp) || (result == max_hp)), "ensures clause failed: (((current + delta) <= max_hp) || (result == max_hp))");
    result
}

pub fn calc_damage_reduction(armor: i64, raw_damage: i64) -> i64 {
    debug_assert!((armor >= 0), "requires clause failed: (armor >= 0)");
    debug_assert!((armor <= 1000000000), "requires clause failed: (armor <= 1000000000)");
    debug_assert!((raw_damage >= 0), "requires clause failed: (raw_damage >= 0)");
    debug_assert!((raw_damage <= 1000000000), "requires clause failed: (raw_damage <= 1000000000)");
    let result = {
        {
        let reduced = clamp_hp(raw_damage, (-armor), raw_damage);
        match (raw_damage == 0) {
        true => 0,
        false => match (reduced == 0) {
        true => 1,
        false => reduced,
    },
    }
    }
    };
    debug_assert!((result >= 0), "ensures clause failed: (result >= 0)");
    debug_assert!((result <= raw_damage), "ensures clause failed: (result <= raw_damage)");
    debug_assert!(((raw_damage != 0) || (result == 0)), "ensures clause failed: ((raw_damage != 0) || (result == 0))");
    debug_assert!(((raw_damage == 0) || (result >= 1)), "ensures clause failed: ((raw_damage == 0) || (result >= 1))");
    debug_assert!(((armor != 0) || (result == raw_damage)), "ensures clause failed: ((armor != 0) || (result == raw_damage))");
    result
}

pub fn move_position(pos: Vec2, dir_x: f64, dir_y: f64, speed: f64, dt: f64) -> Vec2 {
    debug_assert!((pos.x >= (-1000000.0)), "requires clause failed: (pos.x >= (-1000000.0))");
    debug_assert!((pos.x <= 1000000.0), "requires clause failed: (pos.x <= 1000000.0)");
    debug_assert!((pos.y >= (-1000000.0)), "requires clause failed: (pos.y >= (-1000000.0))");
    debug_assert!((pos.y <= 1000000.0), "requires clause failed: (pos.y <= 1000000.0)");
    debug_assert!((dir_x >= (-1.0)), "requires clause failed: (dir_x >= (-1.0))");
    debug_assert!((dir_x <= 1.0), "requires clause failed: (dir_x <= 1.0)");
    debug_assert!((dir_y >= (-1.0)), "requires clause failed: (dir_y >= (-1.0))");
    debug_assert!((dir_y <= 1.0), "requires clause failed: (dir_y <= 1.0)");
    debug_assert!((speed >= 0.0), "requires clause failed: (speed >= 0.0)");
    debug_assert!((speed <= 1000000.0), "requires clause failed: (speed <= 1000000.0)");
    debug_assert!((dt >= 0.0), "requires clause failed: (dt >= 0.0)");
    debug_assert!((dt <= 1000.0), "requires clause failed: (dt <= 1000.0)");
    let result = {
        Vec2 { x: (pos.x + ((dir_x * speed) * dt)), y: (pos.y + ((dir_y * speed) * dt)) }
    };
    debug_assert!((result.x == (pos.x + ((dir_x * speed) * dt))), "ensures clause failed: (result.x == (pos.x + ((dir_x * speed) * dt)))");
    debug_assert!((result.y == (pos.y + ((dir_y * speed) * dt))), "ensures clause failed: (result.y == (pos.y + ((dir_y * speed) * dt)))");
    result
}

pub fn exp_for_next_level(current_level: i64) -> i64 {
    debug_assert!((current_level >= 1), "requires clause failed: (current_level >= 1)");
    debug_assert!((current_level <= 10000000), "requires clause failed: (current_level <= 10000000)");
    let result = {
        (current_level * 100)
    };
    debug_assert!((result == (current_level * 100)), "ensures clause failed: (result == (current_level * 100))");
    debug_assert!((result >= 100), "ensures clause failed: (result >= 100)");
    debug_assert!((result <= 1000000000), "ensures clause failed: (result <= 1000000000)");
    result
}

pub fn can_level_up(current_exp: i64, level: i64) -> bool {
    debug_assert!((current_exp >= 0), "requires clause failed: (current_exp >= 0)");
    debug_assert!((current_exp <= 1000000000), "requires clause failed: (current_exp <= 1000000000)");
    debug_assert!((level >= 1), "requires clause failed: (level >= 1)");
    debug_assert!((level <= 10000000), "requires clause failed: (level <= 10000000)");
    let result = {
        (current_exp >= (level * 100))
    };
    debug_assert!((result == (current_exp >= (level * 100))), "ensures clause failed: (result == (current_exp >= (level * 100)))");
    result
}

pub fn step_player(player: Player, input: PlayerInput, dt: f64) -> Player {
    debug_assert!((player.pos.x >= (-1000000.0)), "requires clause failed: (player.pos.x >= (-1000000.0))");
    debug_assert!((player.pos.x <= 1000000.0), "requires clause failed: (player.pos.x <= 1000000.0)");
    debug_assert!((player.pos.y >= (-1000000.0)), "requires clause failed: (player.pos.y >= (-1000000.0))");
    debug_assert!((player.pos.y <= 1000000.0), "requires clause failed: (player.pos.y <= 1000000.0)");
    debug_assert!((player.hp >= 0), "requires clause failed: (player.hp >= 0)");
    debug_assert!((player.hp <= player.max_hp), "requires clause failed: (player.hp <= player.max_hp)");
    debug_assert!((player.max_hp >= 0), "requires clause failed: (player.max_hp >= 0)");
    debug_assert!((player.exp >= 0), "requires clause failed: (player.exp >= 0)");
    debug_assert!((player.level >= 1), "requires clause failed: (player.level >= 1)");
    debug_assert!((player.speed >= 0.0), "requires clause failed: (player.speed >= 0.0)");
    debug_assert!((player.speed <= 1000000.0), "requires clause failed: (player.speed <= 1000000.0)");
    debug_assert!((input.move_x >= (-1.0)), "requires clause failed: (input.move_x >= (-1.0))");
    debug_assert!((input.move_x <= 1.0), "requires clause failed: (input.move_x <= 1.0)");
    debug_assert!((input.move_y >= (-1.0)), "requires clause failed: (input.move_y >= (-1.0))");
    debug_assert!((input.move_y <= 1.0), "requires clause failed: (input.move_y <= 1.0)");
    debug_assert!((dt >= 0.0), "requires clause failed: (dt >= 0.0)");
    debug_assert!((dt <= 1000.0), "requires clause failed: (dt <= 1000.0)");
    let result = {
        {
        let new_pos = move_position(player.pos, input.move_x, input.move_y, player.speed, dt);
        Player { pos: new_pos, hp: player.hp, max_hp: player.max_hp, exp: player.exp, level: player.level, speed: player.speed }
    }
    };
    debug_assert!((result.pos.x == (player.pos.x + ((input.move_x * player.speed) * dt))), "ensures clause failed: (result.pos.x == (player.pos.x + ((input.move_x * player.speed) * dt)))");
    debug_assert!((result.pos.y == (player.pos.y + ((input.move_y * player.speed) * dt))), "ensures clause failed: (result.pos.y == (player.pos.y + ((input.move_y * player.speed) * dt)))");
    debug_assert!((result.hp == player.hp), "ensures clause failed: (result.hp == player.hp)");
    debug_assert!((result.max_hp == player.max_hp), "ensures clause failed: (result.max_hp == player.max_hp)");
    debug_assert!((result.exp == player.exp), "ensures clause failed: (result.exp == player.exp)");
    debug_assert!((result.level == player.level), "ensures clause failed: (result.level == player.level)");
    debug_assert!((result.speed == player.speed), "ensures clause failed: (result.speed == player.speed)");
    result
}

pub fn step_projectile(proj: Projectile, dt: f64) -> Projectile {
    debug_assert!((proj.pos.x >= (-1000000.0)), "requires clause failed: (proj.pos.x >= (-1000000.0))");
    debug_assert!((proj.pos.x <= 1000000.0), "requires clause failed: (proj.pos.x <= 1000000.0)");
    debug_assert!((proj.pos.y >= (-1000000.0)), "requires clause failed: (proj.pos.y >= (-1000000.0))");
    debug_assert!((proj.pos.y <= 1000000.0), "requires clause failed: (proj.pos.y <= 1000000.0)");
    debug_assert!((proj.vel.x >= (-1000000.0)), "requires clause failed: (proj.vel.x >= (-1000000.0))");
    debug_assert!((proj.vel.x <= 1000000.0), "requires clause failed: (proj.vel.x <= 1000000.0)");
    debug_assert!((proj.vel.y >= (-1000000.0)), "requires clause failed: (proj.vel.y >= (-1000000.0))");
    debug_assert!((proj.vel.y <= 1000000.0), "requires clause failed: (proj.vel.y <= 1000000.0)");
    debug_assert!((proj.damage >= 0), "requires clause failed: (proj.damage >= 0)");
    debug_assert!((proj.damage <= 1000000000), "requires clause failed: (proj.damage <= 1000000000)");
    debug_assert!((proj.pierce >= 0), "requires clause failed: (proj.pierce >= 0)");
    debug_assert!((proj.pierce <= 1000000000), "requires clause failed: (proj.pierce <= 1000000000)");
    debug_assert!((proj.lifetime >= (-1000.0)), "requires clause failed: (proj.lifetime >= (-1000.0))");
    debug_assert!((proj.lifetime <= 1000.0), "requires clause failed: (proj.lifetime <= 1000.0)");
    debug_assert!((dt >= 0.0), "requires clause failed: (dt >= 0.0)");
    debug_assert!((dt <= 1000.0), "requires clause failed: (dt <= 1000.0)");
    let result = {
        Projectile { pos: Vec2 { x: (proj.pos.x + (proj.vel.x * dt)), y: (proj.pos.y + (proj.vel.y * dt)) }, vel: proj.vel, damage: proj.damage, pierce: proj.pierce, lifetime: (proj.lifetime - dt) }
    };
    debug_assert!((result.pos.x == (proj.pos.x + (proj.vel.x * dt))), "ensures clause failed: (result.pos.x == (proj.pos.x + (proj.vel.x * dt)))");
    debug_assert!((result.pos.y == (proj.pos.y + (proj.vel.y * dt))), "ensures clause failed: (result.pos.y == (proj.pos.y + (proj.vel.y * dt)))");
    debug_assert!((result.damage == proj.damage), "ensures clause failed: (result.damage == proj.damage)");
    debug_assert!((result.pierce == proj.pierce), "ensures clause failed: (result.pierce == proj.pierce)");
    debug_assert!((result.lifetime == (proj.lifetime - dt)), "ensures clause failed: (result.lifetime == (proj.lifetime - dt))");
    result
}

pub fn is_projectile_alive(proj: Projectile) -> bool {
    debug_assert!((proj.lifetime >= (-1000000.0)), "requires clause failed: (proj.lifetime >= (-1000000.0))");
    debug_assert!((proj.lifetime <= 1000000.0), "requires clause failed: (proj.lifetime <= 1000000.0)");
    let result = {
        (proj.lifetime > 0.0)
    };
    debug_assert!((result == (proj.lifetime > 0.0)), "ensures clause failed: (result == (proj.lifetime > 0.0))");
    result
}

pub fn apply_damage_to_monster(monster: Monster, raw_damage: i64, armor: i64) -> Monster {
    debug_assert!((monster.hp >= 0), "requires clause failed: (monster.hp >= 0)");
    debug_assert!((monster.hp <= 1000000000), "requires clause failed: (monster.hp <= 1000000000)");
    debug_assert!((monster.pos.x >= (-1000000.0)), "requires clause failed: (monster.pos.x >= (-1000000.0))");
    debug_assert!((monster.pos.x <= 1000000.0), "requires clause failed: (monster.pos.x <= 1000000.0)");
    debug_assert!((monster.pos.y >= (-1000000.0)), "requires clause failed: (monster.pos.y >= (-1000000.0))");
    debug_assert!((monster.pos.y <= 1000000.0), "requires clause failed: (monster.pos.y <= 1000000.0)");
    debug_assert!((monster.speed >= 0.0), "requires clause failed: (monster.speed >= 0.0)");
    debug_assert!((monster.speed <= 1000000.0), "requires clause failed: (monster.speed <= 1000000.0)");
    debug_assert!((raw_damage >= 0), "requires clause failed: (raw_damage >= 0)");
    debug_assert!((raw_damage <= 1000000000), "requires clause failed: (raw_damage <= 1000000000)");
    debug_assert!((armor >= 0), "requires clause failed: (armor >= 0)");
    debug_assert!((armor <= 1000000000), "requires clause failed: (armor <= 1000000000)");
    let result = {
        {
        let eff_damage = calc_damage_reduction(armor, raw_damage);
        let new_hp = clamp_hp(monster.hp, (-eff_damage), monster.hp);
        Monster { id: monster.id, pos: monster.pos, hp: new_hp, speed: monster.speed, kind: monster.kind }
    }
    };
    debug_assert!((result.id == monster.id), "ensures clause failed: (result.id == monster.id)");
    debug_assert!((result.pos.x == monster.pos.x), "ensures clause failed: (result.pos.x == monster.pos.x)");
    debug_assert!((result.pos.y == monster.pos.y), "ensures clause failed: (result.pos.y == monster.pos.y)");
    debug_assert!((result.speed == monster.speed), "ensures clause failed: (result.speed == monster.speed)");
    debug_assert!((result.kind == monster.kind), "ensures clause failed: (result.kind == monster.kind)");
    debug_assert!((result.hp <= monster.hp), "ensures clause failed: (result.hp <= monster.hp)");
    debug_assert!((result.hp >= 0), "ensures clause failed: (result.hp >= 0)");
    result
}

pub fn apply_damage_to_player(player: Player, raw_damage: i64) -> Player {
    debug_assert!((player.pos.x >= (-1000000.0)), "requires clause failed: (player.pos.x >= (-1000000.0))");
    debug_assert!((player.pos.x <= 1000000.0), "requires clause failed: (player.pos.x <= 1000000.0)");
    debug_assert!((player.pos.y >= (-1000000.0)), "requires clause failed: (player.pos.y >= (-1000000.0))");
    debug_assert!((player.pos.y <= 1000000.0), "requires clause failed: (player.pos.y <= 1000000.0)");
    debug_assert!((player.hp >= 0), "requires clause failed: (player.hp >= 0)");
    debug_assert!((player.hp <= player.max_hp), "requires clause failed: (player.hp <= player.max_hp)");
    debug_assert!((player.max_hp >= 0), "requires clause failed: (player.max_hp >= 0)");
    debug_assert!((player.max_hp <= 1000000000), "requires clause failed: (player.max_hp <= 1000000000)");
    debug_assert!((player.exp >= 0), "requires clause failed: (player.exp >= 0)");
    debug_assert!((player.level >= 1), "requires clause failed: (player.level >= 1)");
    debug_assert!((player.speed >= 0.0), "requires clause failed: (player.speed >= 0.0)");
    debug_assert!((player.speed <= 1000000.0), "requires clause failed: (player.speed <= 1000000.0)");
    debug_assert!((raw_damage >= 0), "requires clause failed: (raw_damage >= 0)");
    debug_assert!((raw_damage <= 1000000000), "requires clause failed: (raw_damage <= 1000000000)");
    let result = {
        {
        let new_hp = clamp_hp(player.hp, (-raw_damage), player.max_hp);
        Player { pos: player.pos, hp: new_hp, max_hp: player.max_hp, exp: player.exp, level: player.level, speed: player.speed }
    }
    };
    debug_assert!((result.pos.x == player.pos.x), "ensures clause failed: (result.pos.x == player.pos.x)");
    debug_assert!((result.pos.y == player.pos.y), "ensures clause failed: (result.pos.y == player.pos.y)");
    debug_assert!((result.max_hp == player.max_hp), "ensures clause failed: (result.max_hp == player.max_hp)");
    debug_assert!((result.exp == player.exp), "ensures clause failed: (result.exp == player.exp)");
    debug_assert!((result.level == player.level), "ensures clause failed: (result.level == player.level)");
    debug_assert!((result.speed == player.speed), "ensures clause failed: (result.speed == player.speed)");
    debug_assert!((result.hp >= 0), "ensures clause failed: (result.hp >= 0)");
    debug_assert!((result.hp <= player.max_hp), "ensures clause failed: (result.hp <= player.max_hp)");
    result
}

pub fn apply_gem_pickup(player: Player, exp_gain: i64) -> Player {
    debug_assert!((player.pos.x >= (-1000000.0)), "requires clause failed: (player.pos.x >= (-1000000.0))");
    debug_assert!((player.pos.x <= 1000000.0), "requires clause failed: (player.pos.x <= 1000000.0)");
    debug_assert!((player.pos.y >= (-1000000.0)), "requires clause failed: (player.pos.y >= (-1000000.0))");
    debug_assert!((player.pos.y <= 1000000.0), "requires clause failed: (player.pos.y <= 1000000.0)");
    debug_assert!((player.hp >= 0), "requires clause failed: (player.hp >= 0)");
    debug_assert!((player.hp <= player.max_hp), "requires clause failed: (player.hp <= player.max_hp)");
    debug_assert!((player.max_hp >= 0), "requires clause failed: (player.max_hp >= 0)");
    debug_assert!((player.max_hp <= 1000000000), "requires clause failed: (player.max_hp <= 1000000000)");
    debug_assert!((player.exp >= 0), "requires clause failed: (player.exp >= 0)");
    debug_assert!((player.exp <= 1000000000), "requires clause failed: (player.exp <= 1000000000)");
    debug_assert!((player.level >= 1), "requires clause failed: (player.level >= 1)");
    debug_assert!((player.level <= 10000000), "requires clause failed: (player.level <= 10000000)");
    debug_assert!((player.speed >= 0.0), "requires clause failed: (player.speed >= 0.0)");
    debug_assert!((player.speed <= 1000000.0), "requires clause failed: (player.speed <= 1000000.0)");
    debug_assert!((exp_gain >= 0), "requires clause failed: (exp_gain >= 0)");
    debug_assert!((exp_gain <= 1000000000), "requires clause failed: (exp_gain <= 1000000000)");
    let result = {
        let total_exp = player.exp + exp_gain;
    let upgraded = can_level_up(total_exp, player.level);
    match upgraded {
        true => {
            let next_level = player.level + 1;
            let next_max_hp = player.max_hp + 20;
            Player {
                pos: player.pos,
                hp: next_max_hp,
                max_hp: next_max_hp,
                exp: total_exp,
                level: next_level,
                speed: player.speed,
            }
        },
        false => {
            Player {
                pos: player.pos,
                hp: player.hp,
                max_hp: player.max_hp,
                exp: total_exp,
                level: player.level,
                speed: player.speed,
            }
        },
    }
    };
    debug_assert!((result.pos.x == player.pos.x), "ensures clause failed: (result.pos.x == player.pos.x)");
    debug_assert!((result.pos.y == player.pos.y), "ensures clause failed: (result.pos.y == player.pos.y)");
    debug_assert!((result.speed == player.speed), "ensures clause failed: (result.speed == player.speed)");
    debug_assert!((result.exp == (player.exp + exp_gain)), "ensures clause failed: (result.exp == (player.exp + exp_gain))");
    debug_assert!((result.level >= player.level), "ensures clause failed: (result.level >= player.level)");
    debug_assert!((result.hp >= 0), "ensures clause failed: (result.hp >= 0)");
    debug_assert!((result.hp <= result.max_hp), "ensures clause failed: (result.hp <= result.max_hp)");
    result
}

pub fn is_game_over(player: Player) -> bool {
    debug_assert!((player.hp >= (-1000000000)), "requires clause failed: (player.hp >= (-1000000000))");
    debug_assert!((player.hp <= 1000000000), "requires clause failed: (player.hp <= 1000000000)");
    let result = {
        (player.hp <= 0)
    };
    debug_assert!((result == (player.hp <= 0)), "ensures clause failed: (result == (player.hp <= 0))");
    result
}

