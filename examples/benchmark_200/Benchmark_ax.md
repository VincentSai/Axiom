# 📊 200-Functions 业务函数 Axiom 打洞流水线实验记录 (Benchmark_ax.md)

> **实验目标**：按照 [`SPEC_200.md`](./SPEC_200.md) RPG 游戏引擎需求，通过 **`/axiom-pipeline` 打洞流水线** 实现 200 个业务函数，并与传统多 Agent 模式进行横向对比。

---

## 📊 一、打洞流水线实测数据矩阵 (/axiom-pipeline)

| 流水线阶段 (Pipeline Stage) | 执行主体 / 工具 | 输入与工作内容 | 消耗 Tokens | 执行耗时 | 阶段产出与状态 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **Stage 1: 架构师骨架设计** | Codex (axiom-architect) | 读取需求，设计 7 个 ADT、200 函数签名与 `requires`/`ensures` 契约骨架 (`?hole`) | **148,035 Tokens** | ~1.5 分钟 | ✅ 产出 `engine_200_holes.ax` (200 洞) |
| **Stage 2: 机械化任务拆解** | `axiom extract-holes` CLI | 将 200 个打洞点提取为独立的微型任务 JSON (`hole-task.json`) | **0 Tokens** | **0.09 秒** | ✅ 提取 200 个微型任务文件 |
| **Stage 3: 微型 Worker 并发填洞** | Codex (axiom-worker) | Worker 接收 $O(1)$ 微型局部契约（~160 Tokens），输出纯表达式 | **35,627 Tokens** | 批处理并发 | ✅ 200/200 表达式完成填补并组装 |
| **Stage 4: 静态契约与采样验证** | `axiom verify` CLI | 对带契约的函数执行 Z3 定理证明或网格采样检验，不支持的语法 Fail-Closed 跳过 | **0 Tokens** | **< 1 秒** | ✅ 契约验证通过 (`Proved` / `Sampled` / `Skipped`) |
| **📊 全流程流水线总计 (Total)** | **端到端流水线** | **200 个业务函数从需求到静态验证交付** | **183,662 Tokens** | **~2 分钟** | **🔥 比 Rust 基准节省约 58.9% Tokens** |

---

## 🥊 二、跨语言多 Agent 协作对比

| 协作范式与语言体系 | 架构模式 | 任务粒度与上下文边界 | 消耗 Tokens | 验证手段 | 相对传统 Rust 表现 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **🦀 1. 传统 Rust 多 Agent 模式** | 5 个 Rust Agent (文件系统 `mod`) | 40~50 函数/大模块（单 Agent 上下文庞大） | **446,775 Tokens** | 6 组单元测试用例 | 基准参照 (1.0x) |
| **🌟 2. Axiom 打洞流水线** | **`axiom-pipeline` (Architect + 200 Micro-Workers)** | **单函数 `hole-task.json` 隔离 ($O(1)$ 超轻量上下文)** | **183,662 Tokens** | **SMT 契约求解 + 网格采样 (Fail-Closed)** | **🔥 节省 58.9% Tokens** |

---

## 🔬 三、设计要点分析

1. **Worker 上下文收敛于单个函数签名**：
   - 200 个 Worker 的实现阶段仅消耗约 **3.56 万 Tokens**（平均每个函数实现仅 ~178 Tokens）；
   - Worker 接收单一任务 JSON，不需要加载全量业务源码，降低了多函数堆叠带来的幻觉与阅读损耗。
2. **Fail-Closed 严谨验证反馈**：
   - 支持前置与后置条件的数学约束，外部安装 Z3 时提供形式化证明；未安装 Z3 时进行离散采样检验；未标注契约或遇到高级语法时显示 Skipped，避免将未证明误报为已验证。
