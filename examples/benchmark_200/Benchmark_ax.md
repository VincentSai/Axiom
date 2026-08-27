# 🏆 200-Functions 超大型真实业务 Axiom 多 Agent 协作评测报告 (Benchmark_ax.md)

> **评测目标**：按照 [`SPEC_200.md`](./SPEC_200.md) 工业级 RPG 游戏引擎需求，对比不同 Agent 协作范式下的真实 Token 消耗、执行耗时与形式化证明能力。

---

## 📊 一、三大范式实测数据权威大比拼 (Token Benchmark)

| 协作与开发范式 | 架构模式 | 任务粒度 | 真实消耗 Tokens | SMT / 验证状态 | Token 效率提升 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **1. 传统 Rust 多 Agent 模式** | 5 个 Rust Codex Agent (跨文件 `mod`) | 40~50 函数/大模块 | **446,775 Tokens** | ⚠️ 仅单元测试抽样 (6 suites) | 基准参照 (1.0x) |
| **2. Axiom 粗粒度大文件模式** | 5 个 Agent 从零手写大文件 | 40~50 函数/大模块 | **586,932 Tokens** | ✅ 200/200 `[VERIFIED]` | 产生大量语法探索开销 (+31%) |
| **🌟 3. Axiom 微型 Hole-Task 流水线** | **`axiom extract-holes` + 200 微型 Worker** | **单函数/微型契约 ($O(1)$ 上下文)** | **35,627 Tokens** | **✅ 200/200 `[VERIFIED]` (0.08s)** | **🔥 节省 92.0% Tokens (12.5x 提速)** |

---

## ⚡ 二、Axiom 微型 Hole-Task 模式数据拆解 (200 Micro-Workers)

- **任务总数**：200 个独立的 `hole-task.json`（每个仅包含单个函数签名与 `requires`/`ensures` 局部契约）；
- **输入开销 (Prompt Tokens)**：`32,027 Tokens`（平均单个 Worker 任务仅 **~160 Tokens** 上下文）；
- **输出开销 (Completion Tokens)**：`3,600 Tokens`（平均单个 Worker 仅输出 **~18 Tokens** 纯表达式）；
- **总 Token 消耗**：**35,627 Tokens**；
- **全域证明 (`axiom verify`)**：**200/200 全部通过，耗时仅 0.08 秒**。

---

## 🔬 三、核心洞察：为什么打洞工作流能实现 12.5 倍 Token 暴降？

1. **消除语法探索与大上下文污染**：
   - 传统模式下，Agent 必须读取整个文件或编译器源码来理解语法；
   - 在 `hole-task.json` 模式下，Worker 只收到结构化的 JSON 契约，**上下文被严格隔离在局部 $O(1)$ 范围内**。
2. **纯表达式输出 (Zero Hallucination)**：
   - Worker 不需要输出样板代码（函数头、类型定义、模块引用），只需输出目标表达式（例如 `a + b` 或 `match a >= b { true => a, false => b }`），彻底杜绝幻觉。
3. **确定性 SMT 数学安全网**：
   - 极小的 Token 开销换来了传统语言无法企及的全数域 $(-\infty \sim +\infty)$ 形式化数学保证。
