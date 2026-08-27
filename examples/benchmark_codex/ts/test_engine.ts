import assert from "node:assert/strict";
import test from "node:test";

import {
  applyDamage,
  calculateDamage,
  type Item,
  levelUp,
  type Monster,
  type Player,
  tryMove,
} from "./engine";

const player: Player = {
  id: 1,
  pos: { x: 2, y: 3 },
  stats: { hp: 15, max_hp: 20, attack: 5, defense: 2 },
  exp: 40,
  level: 1,
};

test("domain entities expose the specified data", () => {
  const item: Item = { id: 7, name: "Health Potion", value: 25 };
  const monster: Monster = {
    id: 11,
    pos: { x: 3, y: 4 },
    stats: { hp: 12, max_hp: 12, attack: 4, defense: 1 },
    is_alive: true,
  };

  assert.equal(item.name, "Health Potion");
  assert.deepEqual(monster.pos, { x: 3, y: 4 });
  assert.equal(monster.is_alive, true);
  assert.equal(player.stats.max_hp, 20);
});

test("tryMove returns a valid destination", () => {
  assert.deepEqual(tryMove({ x: 2, y: 3 }, 1, -2, 5, 5), { x: 3, y: 1 });
});

test("tryMove rejects destinations outside the map", () => {
  assert.deepEqual(tryMove({ x: 0, y: 4 }, -1, 0, 5, 5), { x: 0, y: 4 });
  assert.deepEqual(tryMove({ x: 0, y: 4 }, 0, 1, 5, 5), { x: 0, y: 4 });
});

test("tryMove repairs an invalid original position when movement fails", () => {
  assert.deepEqual(tryMove({ x: -2, y: 3 }, -1, 0, 5, 5), { x: 0, y: 3 });
});

test("calculateDamage subtracts defense and has a minimum of one", () => {
  assert.equal(calculateDamage(8, 3), 5);
  assert.equal(calculateDamage(3, 3), 1);
  assert.equal(calculateDamage(2, 5), 1);
});

test("applyDamage subtracts damage and floors HP at zero", () => {
  assert.equal(applyDamage(10, 4), 6);
  assert.equal(applyDamage(10, 10), 0);
  assert.equal(applyDamage(10, 15), 0);
});

test("levelUp accumulates experience below the threshold", () => {
  assert.deepEqual(levelUp(player, 50), {
    ...player,
    exp: 90,
  });
});

test("levelUp consumes the threshold and increases stats", () => {
  assert.deepEqual(levelUp(player, 80), {
    id: 1,
    pos: { x: 2, y: 3 },
    stats: { hp: 30, max_hp: 30, attack: 7, defense: 3 },
    exp: 20,
    level: 2,
  });
});
