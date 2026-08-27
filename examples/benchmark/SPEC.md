# Universal Roguelike Engine Benchmark Specification

本文档定义用于跨语言（Axiom vs Rust vs TypeScript vs Python）同台基准测试的统一标准业务逻辑契约。

---

## 1. 核心领域数据模型 (Domain Entities)

- **Point**: `{ x: Int, y: Int }`
- **Stats**: `{ hp: Int, max_hp: Int, attack: Int, defense: Int }`
- **Item**: `{ id: Int, name: String, value: Int }`
- **Player**: `{ id: Int, pos: Point, stats: Stats, inventory: List<Item>, max_inventory: Int, exp: Int, level: Int }`
- **Monster**: `{ id: Int, pos: Point, stats: Stats, is_alive: Bool }`
- **ActionResult**:
  - `MoveSuccess(Point)`
  - `CombatResolved({ player_hp: Int, monster_hp: Int, is_monster_dead: Bool })`
  - `ItemPicked(Player)`
  - `InventoryFull`
  - `InvalidMove`

---

## 2. 核心代数效应 / 副作用隔离 (Effects & I/O)

- **RandomEffect**:
  - `roll_dice(sides: Int) -> Int`
- **AudioEffect**:
  - `play_sound(name: String) -> ()`
- **RenderEffect**:
  - `draw_tile(x: Int, y: Int, tile_id: Int) -> ()`

---

## 3. 核心算法与前置/后置形式化契约 (Contracts)

1. **移动与边界判定 (`try_move`)**:
   - `requires`: 目标地图宽度 `width > 0`，高度 `height > 0`，当前坐标在合法范围内。
   - `ensures`: 成功移动后新坐标在边界内且曼哈顿距离为 1；越界或撞墙返回 `InvalidMove`。
2. **战斗结算与伤害减免 (`resolve_attack`)**:
   - `requires`: 攻击力 `atk >= 0`，防御力 `def >= 0`，目标 `target_hp >= 0`。
   - `ensures`: 实际造成伤害 `damage = max(1, atk - def)`，扣血后目标生命值 `result_hp >= 0` 且不小于 0（单调不溢出）。
3. **经验结算与升级 (`gain_exp`)**:
   - `requires`: 增加经验 `exp_gain > 0`，当前等级 `level >= 1`。
   - `ensures`: 升级后 `result.level >= player.level` 且 `result.stats.max_hp >= player.stats.max_hp`。
4. **背包拾取与容量限制 (`pickup_item`)**:
   - `requires`: 当前背包数量 `>= 0`。
   - `ensures`: 若数量 `< max_inventory` 则成功加入并返回新 Player；否则返回 `InventoryFull`。
