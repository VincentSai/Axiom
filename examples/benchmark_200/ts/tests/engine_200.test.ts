import assert from 'node:assert/strict';
import test from 'node:test';

import * as engine from '../src/index.js';
import type {
  BaseAttributes,
  DerivedStats,
  Entity,
  Point,
  Rect,
  StatusEffect,
  Velocity,
} from '../src/index.js';

type BinaryEngineFunction = (a: number, b: number) => number;

const publicApi = engine as unknown as Record<string, unknown>;

function getBinaryFunction(name: string): BinaryEngineFunction {
  const candidate = publicApi[name];
  assert.equal(typeof candidate, 'function', `${name} must be exported`);
  return candidate as BinaryEngineFunction;
}

test('the seven domain models describe a complete entity value', () => {
  const point: Point = { x: 3, y: 4 };
  const velocity: Velocity = { dx: 1, dy: -1 };
  const rect: Rect = { min_x: 0, min_y: 1, max_x: 2, max_y: 3 };
  const baseAttributes: BaseAttributes = {
    strength: 10,
    agility: 11,
    intelligence: 12,
    vitality: 13,
  };
  const derivedStats: DerivedStats = {
    max_hp: 100,
    max_mana: 80,
    max_stamina: 60,
    phys_atk: 20,
    magic_atk: 21,
    armor: 22,
    magic_res: 23,
    crit_rate: 500,
    crit_dmg_pct: 150,
    dodge_rate: 300,
    move_speed: 10,
  };
  const status: StatusEffect = {
    bleed_stacks: 1,
    poison_duration: 2,
    burn_damage: 3,
    freeze_stacks: 4,
    shield_hp: 5,
  };
  const entity: Entity = {
    id: 7,
    pos: point,
    base_attr: baseAttributes,
    stats: derivedStats,
    status,
    curr_hp: 90,
    curr_mana: 70,
    curr_stamina: 50,
    gold: 40,
    level: 3,
    exp: 200,
    weight: 9,
    max_weight: 20,
    is_alive: true,
  };

  assert.deepEqual(entity.pos, { x: 3, y: 4 });
  assert.equal(velocity.dy, -1);
  assert.equal(rect.max_y, 3);
});

test('index exports exactly all 200 engine functions', () => {
  const expectedNames = [
    'calc_manhattan_dist',
    'calc_chebyshev_dist',
    'calc_euclidean_sq',
    'check_in_radius_sq',
    'bresenham_step_x',
    'bresenham_step_y',
    'pack_coords_to_hash',
    'unpack_hash_x',
    'unpack_hash_y',
    'wrap_map_coord_x',
    'wrap_map_coord_y',
    'clamp_to_bounds_x',
    'clamp_to_bounds_y',
    'check_aabb_overlap',
    'calc_grid_index',
    'calc_line_of_sight_cost',
    ...Array.from({ length: 24 }, (_, offset) =>
      `spatial_subsys_${String(offset + 17).padStart(3, '0')}`,
    ),
    'calc_max_hp_from_vit',
    'calc_max_mana_from_int',
    'calc_phys_atk_from_str',
    'calc_armor_damage_reduction',
    'calc_magic_res_reduction',
    'calc_elemental_multiplier',
    'calc_crit_chance_clamped',
    'calc_effective_penetration',
    'calc_overburden_penalty',
    'calc_lifesteal_conversion',
    ...Array.from({ length: 40 }, (_, offset) =>
      `stat_subsys_${String(offset + 51).padStart(3, '0')}`,
    ),
    'resolve_physical_hit',
    'resolve_crit_hit',
    'apply_dual_shield_absorption',
    'apply_bleed_stack_tick',
    'apply_poison_exponential_decay',
    'apply_burn_max_hp_tick',
    'check_freeze_threshold',
    'calc_stamina_skill_cost',
    'calc_mana_overload_burst',
    'calc_poise_break_stun',
    ...Array.from({ length: 40 }, (_, offset) =>
      `combat_subsys_${String(offset + 101).padStart(3, '0')}`,
    ),
    'calc_threat_decay_by_dist',
    'evaluate_ai_behavior_state',
    'calc_weighted_loot_tier',
    'scale_drop_by_luck',
    'calc_exp_gain_with_level_diff',
    'calc_exp_to_next_level',
    'calc_shop_inflation_price',
    ...Array.from({ length: 33 }, (_, offset) =>
      `ai_loot_subsys_${String(offset + 148).padStart(3, '0')}`,
    ),
    'determine_turn_initiative',
    'resolve_boss_phase_transition',
    'simulate_single_turn_exchange',
    'resolve_terrain_hazard_tick',
    ...Array.from({ length: 16 }, (_, offset) => `world_engine_${offset + 185}`),
  ];
  const exportedFunctionNames = Object.entries(publicApi)
    .filter(([, value]) => typeof value === 'function')
    .map(([name]) => name)
    .sort();

  assert.equal(expectedNames.length, 200);
  assert.deepEqual(exportedFunctionNames, [...expectedNames].sort());
});

test('Tier 0 covers functions 001 through 040', () => {
  assert.equal(engine.calc_manhattan_dist(1, 2, 4, 6), 7);
  assert.equal(engine.calc_chebyshev_dist(1, 2, 4, 6), 4);
  assert.equal(engine.calc_euclidean_sq(1, 2, 4, 6), 25);
  assert.equal(engine.check_in_radius_sq(25, 5), true);
  assert.equal(engine.bresenham_step_x(1, 4), 1);
  assert.equal(engine.bresenham_step_y(4, 1), -1);
  assert.equal(engine.pack_coords_to_hash(12, 34), 786_466);
  assert.equal(engine.unpack_hash_x(786_466), 12);
  assert.equal(engine.unpack_hash_y(786_466), 34);
  assert.equal(engine.wrap_map_coord_x(-1, 10), 9);
  assert.equal(engine.wrap_map_coord_y(11, 10), 1);
  assert.equal(engine.clamp_to_bounds_x(-3, 10), 0);
  assert.equal(engine.clamp_to_bounds_y(12, 10), 9);
  assert.equal(engine.check_aabb_overlap(0, 0, 5, 5, 5, 5, 8, 8), true);
  assert.equal(engine.calc_grid_index(3, 2, 10), 23);
  assert.equal(engine.calc_line_of_sight_cost(10, 25), 2);

  const expected = [
    0, 6, 4, 18, 20, 8, 2, 2, 2, 3, 262_146, 6, 8, 20, 6, 2, 2, 2, 0,
    0, 5, 12, 2, 6,
  ];

  expected.forEach((expectedResult, offset) => {
    const index = offset + 17;
    const args: [number, number] = index === 26 ? [2, 4] : [4, 2];
    assert.equal(
      getBinaryFunction(`spatial_subsys_${String(index).padStart(3, '0')}`)(
        ...args,
      ),
      expectedResult,
    );
  });
});

test('Tier 1 covers functions 041 through 090', () => {
  assert.equal(engine.calc_max_hp_from_vit(100, 10, 5), 210);
  assert.equal(engine.calc_max_mana_from_int(100, 10), 250);
  assert.equal(engine.calc_phys_atk_from_str(10, 6), 23);
  assert.equal(engine.calc_armor_damage_reduction(100, 100), 50);
  assert.equal(engine.calc_magic_res_reduction(200, 100), 100);
  assert.equal(engine.calc_elemental_multiplier(0, 1, 100), 150);
  assert.equal(engine.calc_crit_chance_clamped(9_500, 100), 10_000);
  assert.equal(engine.calc_effective_penetration(100, 20, 10), 70);
  assert.equal(engine.calc_overburden_penalty(11, 10, 100), 50);
  assert.equal(engine.calc_lifesteal_conversion(200, 25), 50);

  const expected = [
    6, 4, 20, 1, 4, 4, 262_146, 2, 6, 1, 1, 2, 2, 0, 14, 0, 4, 10, 8,
    22, 24, 12, 6, 6, 6, 5, 262_150, 10, 12, 24, 10, 6, 6, 5, 5, 4, 9,
    16, 6, 10,
  ];

  expected.forEach((expectedResult, offset) => {
    const index = offset + 51;
    const args: [number, number] = index === 76 ? [2, 4] : [4, 2];
    assert.equal(
      getBinaryFunction(`stat_subsys_${String(index).padStart(3, '0')}`)(
        ...args,
      ),
      expectedResult,
    );
  });
});

test('Tier 2 covers functions 091 through 140', () => {
  assert.equal(engine.resolve_physical_hit(100, 100), 50);
  assert.equal(engine.resolve_crit_hit(100, true, 150), 150);
  assert.equal(engine.apply_dual_shield_absorption(100, 30), 70);
  assert.equal(engine.apply_bleed_stack_tick(3, false), 15);
  assert.equal(engine.apply_poison_exponential_decay(10, 3), 20);
  assert.equal(engine.apply_burn_max_hp_tick(1_000, 5), 35);
  assert.equal(engine.check_freeze_threshold(5), true);
  assert.equal(engine.calc_stamina_skill_cost(100, 25), 75);
  assert.equal(engine.calc_mana_overload_burst(99), 9);
  assert.equal(engine.calc_poise_break_stun(10, 10), true);

  for (let index = 101; index <= 140; index += 1) {
    assert.ok(
      getBinaryFunction(`combat_subsys_${String(index).padStart(3, '0')}`)(
        10,
        0,
      ) >= 1,
    );
  }
});

test('Tier 3 covers functions 141 through 180', () => {
  assert.equal(engine.calc_threat_decay_by_dist(100, 5), 66);
  assert.equal(engine.evaluate_ai_behavior_state(19, 0), 1);
  assert.equal(engine.calc_weighted_loot_tier(9_900), 5);
  assert.equal(engine.scale_drop_by_luck(9_000, 100), 9_200);
  assert.equal(engine.calc_exp_gain_with_level_diff(100, 20, 9), 10);
  assert.equal(engine.calc_exp_to_next_level(3), 2_400);
  assert.equal(engine.calc_shop_inflation_price(100, 5), 110);

  for (let index = 148; index <= 180; index += 1) {
    assert.ok(
      getBinaryFunction(`ai_loot_subsys_${String(index).padStart(3, '0')}`)(
        10,
        5,
      ) >= 10,
    );
  }
});

test('Tier 4 covers functions 181 through 200', () => {
  assert.equal(engine.determine_turn_initiative(10, 5), 25);
  assert.equal(engine.resolve_boss_phase_transition(67), 1);
  assert.equal(engine.resolve_boss_phase_transition(66), 2);
  assert.equal(engine.resolve_boss_phase_transition(33), 2);
  assert.equal(engine.resolve_boss_phase_transition(32), 3);
  assert.equal(engine.simulate_single_turn_exchange(100, 100, true), 105);
  assert.equal(engine.resolve_terrain_hazard_tick(0, 1_000), 0);
  assert.equal(engine.resolve_terrain_hazard_tick(1, 1_000), 60);
  assert.equal(engine.resolve_terrain_hazard_tick(2, 1_000), 0);
  assert.equal(engine.resolve_terrain_hazard_tick(3, 1_000), 0);

  const expected = [
    2, 3, 1_638_410, 2, 5, 10, 2, 11, 9, 8, 1, 2_400, 21, 35, 11, 52,
  ];
  expected.forEach((expectedResult, offset) => {
    assert.equal(
      getBinaryFunction(`world_engine_${offset + 185}`)(25, 10),
      expectedResult,
    );
  });
});

test('Tier 4 enforces world-simulation contract boundaries', () => {
  assert.throws(() => engine.determine_turn_initiative(-1, 0), RangeError);
  assert.throws(() => engine.determine_turn_initiative(0, 20), RangeError);
  assert.throws(() => engine.resolve_boss_phase_transition(101), RangeError);
  assert.throws(() => engine.resolve_terrain_hazard_tick(4, 100), RangeError);
  assert.throws(() => engine.world_engine_200(-1, 10), RangeError);
  assert.throws(() => engine.world_engine_200(1, 0), RangeError);
});
