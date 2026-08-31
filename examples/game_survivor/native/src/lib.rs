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
#[derive(Clone, Copy, Debug)]
pub struct VisualEventBatch {
    pub data: *const VisualEvent,
    pub length: usize,
}

struct SurvivorRuntime {
    entities: [EntitySnapshot; MAX_ENTITIES],
    entity_count: usize,
    events: [VisualEvent; MAX_VISUAL_EVENTS],
    event_count: usize,
    revision: u64,
    next_entity_id: i64,
    snapshot_batch: SnapshotBatch,
    event_batch: VisualEventBatch,
}

impl SurvivorRuntime {
    fn new() -> Self {
        let mut runtime = Self {
            entities: [EntitySnapshot::default(); MAX_ENTITIES],
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
        };
        runtime.entities[0] = EntitySnapshot {
            id: 1,
            kind: EntityKind::Player,
            hp: 100,
            max_hp: 100,
            level: 1,
            active: 1,
            ..EntitySnapshot::default()
        };
        for index in 1..runtime.entity_count {
            runtime.entities[index] = EntitySnapshot {
                id: (index + 1) as i64,
                kind: EntityKind::Monster,
                x: 4.0 + index as f64 * 2.0,
                y: -3.0 + index as f64,
                hp: 20,
                max_hp: 20,
                sprite_id: 1,
                active: 1,
                ..EntitySnapshot::default()
            };
        }
        runtime
    }

    fn tick(&mut self, input: PlayerInput, delta_time: f64) {
        self.event_count = 0;
        let delta_time = if delta_time.is_finite() {
            delta_time.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let move_x = input.move_x.clamp(-1.0, 1.0);
        let move_y = input.move_y.clamp(-1.0, 1.0);

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
            speed: 5.0,
        };
        let core_input = generated_core::PlayerInput {
            move_x,
            move_y,
            is_firing: input.is_firing != 0,
        };
        let moved = generated_core::step_player(core_player, core_input, delta_time);
        self.entities[0].x = moved.pos.x;
        self.entities[0].y = moved.pos.y;
        self.entities[0].score = self.revision as i64 + 1;

        for index in 1..self.entity_count {
            if self.entities[index].active == 0 || self.entities[index].kind != EntityKind::Monster
            {
                continue;
            }
            let dx = (self.entities[0].x - self.entities[index].x).signum();
            let dy = (self.entities[0].y - self.entities[index].y).signum();
            let next = generated_core::move_position(
                generated_core::Vec2 {
                    x: self.entities[index].x,
                    y: self.entities[index].y,
                },
                dx,
                dy,
                0.75,
                delta_time,
            );
            self.entities[index].x = next.x;
            self.entities[index].y = next.y;
        }

        self.revision += 1;
        if input.is_firing != 0 && self.revision % 15 == 0 {
            self.push_event(VisualEventKind::Shot, 1, 0, 0);
        }
        if self.revision % 30 == 0 {
            self.spawn_monster();
        }
        if self.revision % 50 == 0 {
            self.entities[0].hp = (self.entities[0].hp - 1).max(0);
            self.push_event(VisualEventKind::Hit, 1, 2, 1);
        }
    }

    fn spawn_monster(&mut self) {
        if self.entity_count >= MAX_ENTITIES {
            return;
        }
        let id = self.next_entity_id;
        self.next_entity_id += 1;
        let index = self.entity_count;
        self.entity_count += 1;
        self.entities[index] = EntitySnapshot {
            id,
            kind: EntityKind::Monster,
            x: 8.0 + (self.revision % 7) as f64,
            y: -6.0 + (self.revision % 5) as f64,
            hp: 20,
            max_hp: 20,
            sprite_id: 1,
            active: 1,
            ..EntitySnapshot::default()
        };
        self.push_event(VisualEventKind::Spawn, id, 0, 0);
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
