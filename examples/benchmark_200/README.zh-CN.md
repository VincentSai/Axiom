> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

# 📊 200-Functions 跨语言多 Agent 协作对比实验

> **实验目标**：依据 RPG 业务需求规范 [`SPEC_200.md`](./SPEC_200.md)，横向对比在 AI 大模型（OpenAI Codex CLI）多 Agent 并发协作开发 200 个业务函数时，**Axiom、Python、Rust 与 TypeScript** 四大语言体系在 **Tokens 消耗、上下文隔离与验证反馈** 维度的实际表现。

---

## 📊 一、四大语言多 Agent 协作实测对比

数据由 Codex CLI 实例并发协作产出，对比在不同代码组织结构下的上下文开销与验证机制：

| 语言体系 | 协作模式与任务拆解 | 上下文模型与粒度 | 消耗 Tokens | 验证机制与状态 | 相对消耗对比 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **打洞流水线 (`axiom-pipeline`)**<br>(Architect 骨架 + 200 个填洞任务) | **单函数 `hole-task.json` 隔离<br>(局部上下文聚焦单个签名)** | **183,662 Tokens** | **SMT/Z3 形式化证明 + 网格采样检验**<br>(支持前置/后置契约求解，Fail-Closed) | **🔥 节省约 52% ~ 62%<br>Tokens 消耗** |
| **🐍 Python** | 5 个 Agent 模块化并发<br>(Tier 0 ~ Tier 4) | 40~50 函数/大文件<br>(单 Agent 上下文较庞大) | **384,089 Tokens** | Unittest 单元测试套件 | 0.86x 消耗基准 |
| **🦀 Rust** | 5 个 Agent 跨 crate/mod 并发<br>(Tier 0 ~ Tier 4) | 40~50 函数/大模块<br>(单 Agent 上下文较庞大) | **446,775 Tokens** | Cargo test 单元测试套件 | 1.00x 参照基准 |
| **📘 TypeScript** | 5 个 Agent 模块化并发 (ESM/TS)<br>(Tier 0 ~ Tier 4) | 40~50 函数/大文件<br>(单 Agent 上下文较庞大) | **479,872 Tokens** | TSX / Node test 单元测试套件 | 1.07x 消耗基准 |

---

## 🔬 二、核心设计探索：Axiom 任务拆解与验证机制

在多 Agent 协作开发长逻辑时，传统语言普遍面临“上下文冗余与跨函数幻觉”的挑战：
1. **局部上下文隔离 vs 大文件长上下文**：
   - 传统语言由于单文件通常包含 40~50 个函数，Agent 在阅读和修改时需要加载大量非相关代码上下文，Token 开销随文件尺寸线性膨胀；
   - Axiom 通过 `axiom extract-holes`，将包含洞（`?hole`）的骨架模块机械切分为 **单函数级别的独立任务文件（`hole-task.json`）**。每个 Worker Agent 仅需阅读当前函数的入参类型与 Pre/Post 契约，输出单个纯表达式，有效降低了实现阶段的 Token 消耗。
2. **契约验证机制（SMT 证明与采样检验）**：
   - 传统语言依靠开发者手写单元测试进行有限抽样；
   - Axiom 支持在函数签名中声明 `requires` 与 `ensures` 契约。验证层优先调用外部 SMT 求解器（如 Z3）生成形式化证明（`Proved`）；若无 Z3 环境，则回退为确定性离散网格采样（`Sampled`），并在遇到复杂代数效应或不支持表达式时明确标记为 `Skipped`，确保诊断结果客观真实（Fail-Closed）。
3. **架构师与填洞工人角色分离**：
   - 架构师 Agent (Architect) 专注于全局领域建模、类型定义与契约骨架；
   - 填洞工人 (Micro-Workers) 并发生成无副作用的纯函数体；
   - 诊断系统捕获反例并辅助单步自愈。

---

## 📂 三、详细报告与各语言源码导航

| 导航项目 | 描述说明 | 链接地址 |
| :--- | :--- | :--- |
| 📋 **业务规范设计书** | 200 个业务函数分层与数学逻辑设计 | [`SPEC_200.md`](./SPEC_200.md) |
| 🌟 **Axiom 评测记录** | 打洞流水线分步 Token 消耗与验证记录 | [`Benchmark_ax.md`](./Benchmark_ax.md) |
| 🐍 **Python 评测记录** | 5 个 Agent 消耗、Unittest 用例与结果 | [`Benchmark_py.md`](./Benchmark_py.md) |
| 🦀 **Rust 评测记录** | 5 个 Agent 消耗、Cargo test 套件与结果 | [`Benchmark_rs.md`](./Benchmark_rs.md) |
| 📘 **TypeScript 评测记录** | 5 个 Agent 消耗、TSX 测试套件与结果 | [`Benchmark_ts.md`](./Benchmark_ts.md) |
| 📜 **Axiom 源码目录** | 骨架 `engine_200_holes.ax` 与填洞后代码 `engine_200.ax` | [`ax/`](./ax/) |
| 🐍 **Python 源码目录** | 5 层模块与单元测试套件 | [`py/`](./py/) |
| 🦀 **Rust 源码目录** | 5 层模块与 Cargo 单元测试套件 | [`rs/`](./rs/) |
| 📘 **TypeScript 源码目录** | 5 层模块与 Node/TSX 测试套件 | [`ts/`](./ts/) |

---

## 🖥️ 四、本地验证命令指南

```powershell
# 1. 运行 Axiom 契约与静态验证
cargo run -p axiom-cli -- verify examples/benchmark_200/ax/engine_200.ax

# 2. 运行 Python 完整单元测试套件
python -m unittest discover -s examples/benchmark_200/py/tests -v

# 3. 运行 Rust 完整测试套件
cargo test --manifest-path examples/benchmark_200/rs/Cargo.toml

# 4. 运行 TypeScript 完整测试套件
npx tsx --test examples/benchmark_200/ts/tests/engine_200.test.ts
```
