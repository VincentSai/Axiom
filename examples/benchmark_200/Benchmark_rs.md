# 🏆 200-Functions 超大型真实业务 Rust 多 Agent 协作评测报告 (Benchmark_rs.md)

> **评测目标**：按照 [`SPEC_200.md`](./SPEC_200.md) 工业级 RPG 游戏引擎需求，由 **5 个真实的 Codex CLI Agent 实例并发流水线协作** 实现 200 个真实业务函数，并完成完整测试套件与 `cargo test` 验证。

---

## 📊 一、5 个 Rust Codex Agent 并发协作实测数据矩阵

| Agent 角色 (Agent Role) | 负责模块与源码文件 | 核心业务算法 / 逻辑 | 依赖层级 | 真实消耗 Tokens | 执行结果 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Agent 1 (空间几何专家)** | Tier 0: `src/tier0_spatial.rs` (40 函数) | 曼哈顿/切比雪夫距离、Bresenham步进、坐标哈希打包、AABB碰撞 | 纯叶子底座 | **74,240 Tokens** | ✅ 40/40 完成 |
| **Agent 2 (RPG 数值专家)** | Tier 1: `src/tier1_stats.rs` (50 函数) | 主属性换算、非线性护甲减伤公式、五行属性相克、分段负重惩罚 | 依赖 Tier 0 | **107,760 Tokens** | ✅ 50/50 完成 |
| **Agent 3 (战斗状态机专家)**| Tier 2: `src/tier2_combat.rs` (50 函数) | 物法混合破防、双护盾吸收、流血/中毒/燃烧/冰冻状态机、韧性削减 | 依赖 Tier 0, 1 | **76,108 Tokens** | ✅ 50/50 完成 |
| **Agent 4 (AI 与掉落专家)**  | Tier 3: `src/tier3_ai_loot.rs` (40 函数) | 仇恨距离衰减、AI三态机、轮盘赌加权掉落二分、经验阶梯 | 依赖 Tier 0, 1, 2 | **85,960 Tokens** | ✅ 40/40 完成 |
| **Agent 5 (主架构与测试)**  | Tier 4: `tier4_world.rs` + `lib.rs` + `tests/` | 先攻判定、Boss三阶段血线、世界Tick、全局结构体与200测试断言 | 依赖 Tier 0~3 | **102,707 Tokens** | ✅ 20/20 + 全局测试通过 |
| **🏆 全局总计 (Total)**     | **全量 5 层 200 个真实业务函数** | **200 个带完整 `assert!` 约束与真实算法的 Rust 工业级代码** | **5 层完整拓扑** | **446,775 Tokens** | **✅ 6/6 Suites 100% Passed** |

---

## ⚡ 二、编译与测试性能

- **Cargo 编译与测试耗时**：`cargo test --manifest-path examples/benchmark_200/rs/Cargo.toml`
  - 编译耗时：**0.46 秒**
  - 测试用例：**6 个完整测试套件（覆盖全部 200 个函数）**
  - 测试结果：`test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

---

## 🔬 三、Rust 与 Axiom 多 Agent 协作维度对比

1. **模块隔离与并行开发**：
   - Rust 依靠 `src/tierX_*.rs` 的文件系统与 `mod` 树体系，成功实现了 4 个 Agent 并行开发；
   - 依赖解析必须显式依靠 `use crate::tierX_*::*;` 进行跨模块导入。
2. **测试验证 vs 形式化证明**：
   - Rust 的正确性建立在 `assert!` 运行时断言与单元测试抽样上；
   - Axiom 则在编译期直接由 SMT 求解器完成全域数学证明，无需编写额外的手工测试用例。
