//! Native Unity runtime for the survivor example.
//!
//! `generated_core` is emitted from `ax/survivor_core.ax`; this module owns only
//! the fixed-capacity state and stable C ABI required by Unity.

#![allow(dead_code, unused_parens)]

mod generated_core;

use std::ffi::c_void;
use std::ptr;
use std::sync::atomic::{AtomicUsize, Ordering};

const MAX_ENTITIES: usize = 64;
const MAX_VISUAL_EVENTS: usize = 16;
static LIVE_INSTANCES: AtomicUsize = AtomicUsize::new(0);

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum EntityKind {
    #[default]
    Player = 0,
    Monster = 1,
    Projectile = 2,
    Gem = 3,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisualEventKind {
    #[default]
    Spawn = 0,
    Despawn = 1,
    Hit = 2,
    Death = 3,
    Shot = 4,
    Pickup = 5,
    LevelUp = 6,
    GameOver = 7,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct PlayerInput {
    pub move_x: f64,
    pub move_y: f64,
    pub is_firing: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
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
    pub active: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VisualEvent {
    pub kind: VisualEventKind,
    pub entity_id: i64,
    pub asset_id: i64,
    pub x: f64,
    pub y: f64,
    pub value: i64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SnapshotBatch {
    pub data: *const EntitySnapshot,
    pub length: usize,
    pub revision: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VisualEventBatch {
    pub data: *const VisualEvent,
    pub length: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct ProjectileMeta {
    vel_x: f64,
    vel_y: f64,
    lifetime: f64,
}

struct SurvivorRuntime {
    entities: [EntitySnapshot; MAX_ENTITIES],
    proj_meta: [ProjectileMeta; MAX_ENTITIES],
    entity_count: usize,
    events: [VisualEvent; MAX_VISUAL_EVENTS],
    event_count: usize,
    revision: u64,
    next_entity_id: i64,
    snapshot_batch: SnapshotBatch,
    event_batch: VisualEventBatch,
    facing_x: f64,
    facing_y: f64,
    fire_cooldown: f64,
    player_invincible_timer: f64,
    monster_spawn_timer: f64,
    is_game_over: bool,
}

impl SurvivorRuntime {
    fn new() -> Self {
        let mut runtime = Self {
            entities: [EntitySnapshot::default(); MAX_ENTITIES],
            proj_meta: [ProjectileMeta::default(); MAX_ENTITIES],
            entity_count: 4,
            events: [VisualEvent::default(); MAX_VISUAL_EVENTS],
            event_count: 0,
            revision: 0,
            next_entity_id: 5,
            snapshot_batch: SnapshotBatch {
                data: ptr::null(),
                length: 0,
                revision: 0,
            },
            event_batch: VisualEventBatch {
                data: ptr::null(),
                length: 0,
            },
            facing_x: 1.0,
            facing_y: 0.0,
            fire_cooldown: 0.0,
            player_invincible_timer: 0.0,
            monster_spawn_timer: 0.0,
            is_game_over: false,
        };

        // Player entity at origin
        runtime.entities[0] = EntitySnapshot {
            id: 1,
            kind: EntityKind::Player,
            x: 0.0,
            y: 0.0,
            hp: 100,
            max_hp: 100,
            level: 1,
            active: 1,
            ..EntitySnapshot::default()
        };

        // Initial monsters surrounding the player
        let initial_positions = [(5.0, 2.0), (-5.0, 3.0), (2.0, -5.0)];
        for (i, &(mx, my)) in initial_positions.iter().enumerate() {
            runtime.entities[i + 1] = EntitySnapshot {
                id: (i + 2) as i64,
                kind: EntityKind::Monster,
                x: mx,
                y: my,
                hp: 30,
                max_hp: 30,
                sprite_id: 1,
                active: 1,
                ..EntitySnapshot::default()
            };
        }

        runtime
    }

    fn reset(&mut self) {
        *self = Self::new();
    }

    fn tick(&mut self, input: PlayerInput, delta_time: f64) {
        self.event_count = 0;
        let dt = if delta_time.is_finite() {
            delta_time.clamp(0.0, 0.1)
        } else {
            0.0
        };

        // If Game Over, freeze gameplay and only wait for restart
        if self.is_game_over {
            return;
        }

        let move_x = input.move_x.clamp(-1.0, 1.0);
        let move_y = input.move_y.clamp(-1.0, 1.0);

        // 1. Update facing direction based on movement
        if move_x.abs() > 0.01 || move_y.abs() > 0.01 {
            let len = (move_x * move_x + move_y * move_y).sqrt();
            if len > 0.001 {
                self.facing_x = move_x / len;
                self.facing_y = move_y / len;
            }
        }

        // 2. Step Player Movement using SMT-verified step_player
        let player = self.entities[0];
        let core_player = generated_core::Player {
            pos: generated_core::Vec2 {
                x: player.x,
                y: player.y,
            },
            hp: player.hp,
            max_hp: player.max_hp,
            exp: player.exp,
            level: player.level,
            speed: 5.5,
        };
        let core_input = generated_core::PlayerInput {
            move_x,
            move_y,
            is_firing: input.is_firing != 0,
        };
        let moved = generated_core::step_player(core_player, core_input, dt);
        self.entities[0].x = moved.pos.x;
        self.entities[0].y = moved.pos.y;

        // 3. Handle Weapon Firing & Spawn Projectiles
        self.fire_cooldown -= dt;
        if input.is_firing != 0 && self.fire_cooldown <= 0.0 {
            self.fire_cooldown = 0.22; // ~4.5 shots per second
            self.spawn_projectile(
                self.entities[0].x,
                self.entities[0].y,
                self.facing_x,
                self.facing_y,
            );
        }

        // 4. Update Projectiles & Check Collisions with Monsters
        self.update_projectiles(dt);

        // 5. Update Monsters (Move towards player & Attack player)
        self.update_monsters(dt);

        // 6. Check Gem Pickups & Level Up
        self.update_gems();

        // 7. Periodic Monster Spawning (Every 2.0s)
        self.monster_spawn_timer += dt;
        if self.monster_spawn_timer >= 2.0 {
            self.monster_spawn_timer = 0.0;
            self.spawn_wave_monster();
        }

        self.revision += 1;
    }

    fn spawn_projectile(&mut self, x: f64, y: f64, dir_x: f64, dir_y: f64) {
        let slot = self.find_or_allocate_slot();
        let id = self.next_entity_id;
        self.next_entity_id += 1;

        self.entities[slot] = EntitySnapshot {
            id,
            kind: EntityKind::Projectile,
            x: x + dir_x * 0.4,
            y: y + dir_y * 0.4,
            hp: 1,
            max_hp: 1,
            active: 1,
            ..EntitySnapshot::default()
        };

        self.proj_meta[slot] = ProjectileMeta {
            vel_x: dir_x * 14.0,
            vel_y: dir_y * 14.0,
            lifetime: 1.2,
        };

        self.push_event(VisualEventKind::Shot, id, 0, 0);
    }

    fn update_projectiles(&mut self, dt: f64) {
        for i in 0..self.entity_count {
            if self.entities[i].active == 0 || self.entities[i].kind != EntityKind::Projectile {
                continue;
            }

            // Step Projectile using SMT-verified Axiom core function
            let core_proj = generated_core::Projectile {
                pos: generated_core::Vec2 {
                    x: self.entities[i].x,
                    y: self.entities[i].y,
                },
                vel: generated_core::Vec2 {
                    x: self.proj_meta[i].vel_x,
                    y: self.proj_meta[i].vel_y,
                },
                damage: 15,
                pierce: 1,
                lifetime: self.proj_meta[i].lifetime,
            };
            let stepped_proj = generated_core::step_projectile(core_proj, dt);
            self.entities[i].x = stepped_proj.pos.x;
            self.entities[i].y = stepped_proj.pos.y;
            self.proj_meta[i].lifetime = stepped_proj.lifetime;

            // Check if expired using SMT-verified Axiom predicate
            if !generated_core::is_projectile_alive(stepped_proj.clone()) {
                self.entities[i].active = 0;
                self.push_event(VisualEventKind::Despawn, self.entities[i].id, 0, 0);
                continue;
            }

            let proj_pos = stepped_proj.pos;

            // Check collision with all active monsters
            for m in 0..self.entity_count {
                if self.entities[m].active == 0 || self.entities[m].kind != EntityKind::Monster {
                    continue;
                }

                let monster_pos = generated_core::Vec2 {
                    x: self.entities[m].x,
                    y: self.entities[m].y,
                };

                if generated_core::check_collision(proj_pos, 0.4, monster_pos, 0.6) {
                    // Hit monster!
                    self.entities[i].active = 0;
                    self.push_event(VisualEventKind::Despawn, self.entities[i].id, 0, 0);

                    // Apply damage via SMT-verified Axiom function
                    let core_monster = generated_core::Monster {
                        id: self.entities[m].id,
                        pos: monster_pos,
                        hp: self.entities[m].hp,
                        speed: 1.8,
                        kind: 1,
                    };
                    let damaged_monster =
                        generated_core::apply_damage_to_monster(core_monster, 15, 0);
                    let dealt_damage = self.entities[m].hp - damaged_monster.hp;
                    self.entities[m].hp = damaged_monster.hp;
                    self.push_event(VisualEventKind::Hit, self.entities[m].id, 0, dealt_damage);

                    // Check if monster died
                    if self.entities[m].hp <= 0 {
                        self.entities[m].active = 0;
                        self.push_event(VisualEventKind::Death, self.entities[m].id, 0, 0);
                        self.entities[0].score += 100;

                        // Spawn Gem at monster location
                        let mx = self.entities[m].x;
                        let my = self.entities[m].y;
                        self.spawn_gem(mx, my);
                    }

                    break;
                }
            }
        }
    }

    fn update_monsters(&mut self, dt: f64) {
        let player_pos = generated_core::Vec2 {
            x: self.entities[0].x,
            y: self.entities[0].y,
        };
        self.player_invincible_timer -= dt;

        for i in 0..self.entity_count {
            if self.entities[i].active == 0 || self.entities[i].kind != EntityKind::Monster {
                continue;
            }

            let mx = self.entities[i].x;
            let my = self.entities[i].y;
            let dx = player_pos.x - mx;
            let dy = player_pos.y - my;
            let dist = (dx * dx + dy * dy).sqrt();

            if dist > 0.1 {
                let dir_x = dx / dist;
                let dir_y = dy / dist;
                let next = generated_core::move_position(
                    generated_core::Vec2 { x: mx, y: my },
                    dir_x,
                    dir_y,
                    1.8,
                    dt,
                );
                self.entities[i].x = next.x;
                self.entities[i].y = next.y;
            }

            // Check collision with player using SMT-verified function
            let current_monster_pos = generated_core::Vec2 {
                x: self.entities[i].x,
                y: self.entities[i].y,
            };

            if generated_core::check_collision(current_monster_pos, 0.5, player_pos, 0.5) {
                if self.player_invincible_timer <= 0.0 {
                    let core_player = generated_core::Player {
                        pos: player_pos,
                        hp: self.entities[0].hp,
                        max_hp: self.entities[0].max_hp,
                        exp: self.entities[0].exp,
                        level: self.entities[0].level,
                        speed: 5.5,
                    };
                    let damaged_player = generated_core::apply_damage_to_player(core_player, 10);
                    let dealt = self.entities[0].hp - damaged_player.hp;
                    self.entities[0].hp = damaged_player.hp;
                    self.player_invincible_timer = 0.6; // 0.6s i-frame
                    self.push_event(VisualEventKind::Hit, 1, 0, dealt);

                    if generated_core::is_game_over(damaged_player) {
                        self.is_game_over = true;
                        self.push_event(VisualEventKind::GameOver, 1, 0, 0);
                    }
                }
            }
        }
    }

    fn spawn_gem(&mut self, x: f64, y: f64) {
        let slot = self.find_or_allocate_slot();
        let id = self.next_entity_id;
        self.next_entity_id += 1;

        self.entities[slot] = EntitySnapshot {
            id,
            kind: EntityKind::Gem,
            x,
            y,
            hp: 50, // 50 EXP value
            max_hp: 50,
            active: 1,
            ..EntitySnapshot::default()
        };
        self.push_event(VisualEventKind::Spawn, id, 0, 0);
    }

    fn update_gems(&mut self) {
        let player_pos = generated_core::Vec2 {
            x: self.entities[0].x,
            y: self.entities[0].y,
        };

        for i in 0..self.entity_count {
            if self.entities[i].active == 0 || self.entities[i].kind != EntityKind::Gem {
                continue;
            }

            let gem_pos = generated_core::Vec2 {
                x: self.entities[i].x,
                y: self.entities[i].y,
            };

            if generated_core::check_collision(gem_pos, 0.4, player_pos, 0.9) {
                // Collect Gem
                self.entities[i].active = 0;
                self.push_event(VisualEventKind::Pickup, self.entities[i].id, 0, 50);

                let core_player = generated_core::Player {
                    pos: player_pos,
                    hp: self.entities[0].hp,
                    max_hp: self.entities[0].max_hp,
                    exp: self.entities[0].exp,
                    level: self.entities[0].level,
                    speed: 5.5,
                };
                let old_level = core_player.level;
                let updated_player = generated_core::apply_gem_pickup(core_player, 50);
                self.entities[0].hp = updated_player.hp;
                self.entities[0].max_hp = updated_player.max_hp;
                self.entities[0].exp = updated_player.exp;
                self.entities[0].level = updated_player.level;
                self.entities[0].score += 50;

                // Level up visual event
                if updated_player.level > old_level {
                    self.push_event(VisualEventKind::LevelUp, 1, 0, updated_player.level);
                }
            }
        }
    }

    fn spawn_wave_monster(&mut self) {
        let slot = self.find_or_allocate_slot();
        let id = self.next_entity_id;
        self.next_entity_id += 1;

        let angle = (self.revision as f64 * 0.7) % 6.28;
        let radius = 8.0 + (self.revision % 4) as f64;
        let px = self.entities[0].x + angle.cos() * radius;
        let py = self.entities[0].y + angle.sin() * radius;

        self.entities[slot] = EntitySnapshot {
            id,
            kind: EntityKind::Monster,
            x: px,
            y: py,
            hp: 30,
            max_hp: 30,
            sprite_id: 1,
            active: 1,
            ..EntitySnapshot::default()
        };
        self.push_event(VisualEventKind::Spawn, id, 0, 0);
    }

    fn find_or_allocate_slot(&mut self) -> usize {
        // 1. Allocate new slot first if under capacity (prevents same-frame slot collision)
        if self.entity_count < MAX_ENTITIES {
            let index = self.entity_count;
            self.entity_count += 1;
            return index;
        }

        // 2. Look for inactive slot
        for i in 1..self.entity_count {
            if self.entities[i].active == 0 {
                return i;
            }
        }

        // 3. Fallback: override oldest projectile or monster
        1 + (self.revision as usize % (MAX_ENTITIES - 1))
    }

    fn push_event(&mut self, kind: VisualEventKind, entity_id: i64, asset_id: i64, value: i64) {
        if self.event_count >= MAX_VISUAL_EVENTS {
            return;
        }
        let target = self
            .entities
            .iter()
            .take(self.entity_count)
            .find(|entity| entity.id == entity_id)
            .copied()
            .unwrap_or_default();
        self.events[self.event_count] = VisualEvent {
            kind,
            entity_id,
            asset_id,
            x: target.x,
            y: target.y,
            value,
        };
        self.event_count += 1;
    }

    fn snapshot_ptr(&mut self) -> *const SnapshotBatch {
        self.snapshot_batch = SnapshotBatch {
            data: self.entities.as_ptr(),
            length: self.entity_count,
            revision: self.revision,
        };
        &self.snapshot_batch
    }

    fn events_ptr(&mut self) -> *const VisualEventBatch {
        self.event_batch = VisualEventBatch {
            data: self.events.as_ptr(),
            length: self.event_count,
        };
        &self.event_batch
    }
}

#[no_mangle]
pub extern "C" fn axiom_init() -> *mut c_void {
    LIVE_INSTANCES.fetch_add(1, Ordering::Relaxed);
    Box::into_raw(Box::new(SurvivorRuntime::new())).cast()
}

#[no_mangle]
pub unsafe extern "C" fn axiom_tick(
    instance: *mut c_void,
    input: *const PlayerInput,
    delta_time: f64,
) {
    let Some(runtime) = instance.cast::<SurvivorRuntime>().as_mut() else {
        return;
    };
    let input = input.as_ref().copied().unwrap_or_default();
    runtime.tick(input, delta_time);
}

#[no_mangle]
pub unsafe extern "C" fn axiom_get_snapshot_ptr(instance: *mut c_void) -> *const SnapshotBatch {
    instance
        .cast::<SurvivorRuntime>()
        .as_mut()
        .map_or(ptr::null(), SurvivorRuntime::snapshot_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn axiom_get_events_ptr(instance: *mut c_void) -> *const VisualEventBatch {
    instance
        .cast::<SurvivorRuntime>()
        .as_mut()
        .map_or(ptr::null(), SurvivorRuntime::events_ptr)
}

#[no_mangle]
pub unsafe extern "C" fn axiom_destroy(instance: *mut c_void) {
    if instance.is_null() {
        return;
    }
    drop(Box::from_raw(instance.cast::<SurvivorRuntime>()));
    LIVE_INSTANCES.fetch_sub(1, Ordering::Relaxed);
}

#[no_mangle]
pub unsafe extern "C" fn axiom_reset(instance: *mut c_void) {
    if let Some(runtime) = instance.cast::<SurvivorRuntime>().as_mut() {
        runtime.reset();
    }
}

#[no_mangle]
pub extern "C" fn axiom_live_instances() -> usize {
    LIVE_INSTANCES.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_advances_without_reallocating_fixed_buffers() {
        let mut runtime = SurvivorRuntime::new();
        let entities = runtime.entities.as_ptr();
        let events = runtime.events.as_ptr();
        for _ in 0..100 {
            runtime.tick(
                PlayerInput {
                    move_x: 1.0,
                    move_y: 0.0,
                    is_firing: 1,
                },
                1.0 / 60.0,
            );
        }
        assert_eq!(runtime.revision, 100);
        assert_eq!(runtime.entities.as_ptr(), entities);
        assert_eq!(runtime.events.as_ptr(), events);
        assert!(runtime.entity_count > 4);
    }
}
