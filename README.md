# Axiom: 面向 AI 协作的可验证逻辑中间层与运行时 (AI-Native Verified Logic Layer)

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)  
> 📄 **核心文档**: [技术成果白皮书与审查终卷 (docs/WHITEPAPER.md)](docs/WHITEPAPER.md) | [单一变量实证报告 (examples/benchmark_eval/EVALUATION_REPORT.md)](examples/benchmark_eval/EVALUATION_REPORT.md)

---

> **Axiom** 是一个专门面向 **AI 协作的受限纯函数 DSL、形式化验证与多 Agent 协同流水线**。  
> **核心战略定位**：**不替代通用语言 (Rust / C# / TypeScript)**，而是与其深度共生——Axiom 专注于最适合 AI 生成、最需要确定性防御的核心逻辑层（数值管线、状态机转移、规则判定、权限校验），外围无缝转译并嵌入 Rust、Unity C#、WASM 与 C。  
> **不可替代性护城河**：
> 1. **形式化语义最小完备集**：受限纯表达式、显式代数效应（Algebraic Effects）与定点数微元制，将代码生成空间完全收敛在可判定的 SMT 理论域；
> 2. **AST 级 3-Way 结构化合并**：彻底杜绝多 Agent 协同中的文本级代码冲突，配合跨符号级联定理证明消除隐式语义破坏；
> 3. **反例单步自愈闭环**：SMT 求解器精准提取数值反例，驱动 Agent 在 1 轮内通过分支守卫自动修复收敛并原子签入 CAS 任务池；
> 4. **不可伪造的真实实证数据**：在真实 OpenAI Codex 宏观并发流水线实测中，Axiom 相比传统 Rust 微任务**全链路净节省 21.0% 的真实 API Token**，且达成 **Z3 SMT 全域数学证明（All UNSAT）**。

---

## 🌟 核心架构与共生体系

```
┌─────────────────────────────────────────────────────────────────┐
│               Host Presentation / Engine Layer                  │
│       Unity / C#  •  Web / TypeScript  •  Native Rust / C       │
└───────────────────────────────┬─────────────────────────────────┘
                                │ FFI / Native Bridge / WASM
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│               Axiom Verified Logic Layer (Axiom 内核)            │
│  • Pure Functions (受限纯表达式)   • Algebraic Effects (显式副作用隔离) │
│  • Decimal Fixed-Point (微元制)   • SMT Contracts (requires/ensures)│
│  • AST 3-Way CAS Database        • Cascade Verification Closure    │
└───────────────────────────────┬─────────────────────────────────┘
                                │ Hole Extraction & Task Envelope
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│              Multi-Agent Orchestration & Self-Healing           │
│   • OpenAI Codex CLI • Local SMT Solver • Single-Step Healing   │
└─────────────────────────────────────────────────────────────────┘
```

---

## 🏆 真实 OpenAI Codex 宏观并发实证 (Live Empirical Evidence)

在背包交易与库存结算子系统（5 个核心算法函数）的严格单一变量 A/B 对照实测中（基于本地已鉴权的 **OpenAI Codex CLI `v0.147.0`, `gpt-5.6-sol`** 与 **Microsoft Z3 4.13.0**）：

| 对照维度 | Group 2: 传统工程微任务 (Rust Git Flow) | Group 3: Axiom 形式化打洞流水线 | 科学对比结论 |
| :--- | :--- | :--- | :--- |
| **协作机制** | 5 分支独立生成 + 串行 Git 合并 | 5 Worker 纯代数填洞 + AST CAS 组合器 | 物理杜绝文本合并冲突 |
| **合并冲突率** | 0.0% (无二次文本冲突修复) | **0.0% (零文本级冲突)** | 保持同等高质量干净交付 |
| **Worker并发耗时** | 25.07 秒 | **22.97 秒** | 代数打洞 Worker 推理更纯粹、速度更快 |
| **全链路总真实账单** | **95,263** Tokens | **75,238** Tokens | **Axiom 全链路净节省 21.0% Token** |
| **组合器/合并消耗** | 0 Token (仅 Git 引擎本地合并) | **0 Token (原生 AST CAS 零模型调用)** | 保持 0 文本解决税 |
| **双Worker CAS争用压测** | - | **2/2 签入成功 (耗时 17.86s)** | 跨进程排他锁与 AST 重基 100% 稳健 |
| **终态质量保证** | 基于离散测试向量采样通过 | **Z3 SMT 全域定理证明通过 (All UNSAT)** | **无限输入全域无Bug，彻底消灭未测边界幻觉** |

> 完整实证审计数据链已归档于 [`examples/benchmark_eval/data/concurrent_eval_data.json`](examples/benchmark_eval/data/concurrent_eval_data.json)。

---

## 🎮 实战演示：Axiom Survivors 割草游戏

在 [`examples/game_survivor/`](examples/game_survivor/) 中提供了一个完整的 2D 肉鸽割草游戏落地示范：
- **形式化逻辑内核 ([`survivor_core.ax`](examples/game_survivor/ax/survivor_core.ax))**：涵盖角色移动、弹道欧拉推进、欧几里得圆形碰撞、伤害减伤与突破升级等 15 个纯函数与数学契约；
- **Unity 表现层 ([`SurvivorGameController.cs`](examples/game_survivor/Assets/Scripts/Gameplay/SurvivorGameController.cs))**：零配置自举（打开 SampleScene 即可试玩）、过程化像素精灵生成、平滑相机跟随与死亡一键重开；
- **零 GC 双缓冲同步**：Rust Native 动态库与 Unity 之间通过定长连续内存进行 60FPS+ 双缓冲传递。

---

## 💻 CLI 工具链核心命令

```powershell
# 1. 语法、类型与形式化契约全域验证 (自动检测 Z3，支持 --strict 模式)
axiom verify examples/benchmark_200/ax/engine_200.ax --strict

# 2. 机械化提取打洞任务 JSON (派发给 Worker Agents 并发填洞)
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 3. 原生 Agent 求解器与 SMT 数值反例单步自愈闭环
# (支持内置 mock 引擎或真实本地 codex 引擎)
axiom pool solve examples/math.ax --hole abs_hole --engine mock --max-steps 3 --json
axiom pool solve examples/math.ax --hole abs_hole --engine codex --max-steps 3

# 4. AST 级 CAS 任务池状态查看、候选提交与确定性模块装配
axiom pool status examples/benchmark_200/ax/engine_200_holes.ax
axiom pool submit examples/math.ax --hole calc_h0 --expr "x + 1"
axiom pool assemble examples/math.ax --solutions solutions.json -o math_verified.ax

# 5. 转译至 Rust 源码并由 rustc 原生编译（物理验证门闭锁，注入防伪证明回执）
axiom emit-rust examples/game_loop.ax -o game_loop.rs
rustc --crate-type lib game_loop.rs -o game_loop.rlib

# 6. Unity 跨端代码生成与零拷贝双缓冲桥接
axiom bridge examples/game_survivor/ax/survivor_core.ax --target unity --out examples/game_survivor/Assets/Scripts/AxiomBridge --library-name axiom_survivor

# 7. 多语言接口绑定导出 (TypeScript / C / WIT)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit
```

---

## 📂 项目文档导航

- 📘 [Axiom 技术成果白皮书与审查终卷 (docs/WHITEPAPER.md)](docs/WHITEPAPER.md)
- 🔬 [单一变量 A/B 科学对照评测方法与实证报告 (examples/benchmark_eval/EVALUATION_REPORT.md)](examples/benchmark_eval/EVALUATION_REPORT.md)
- 📊 [200 函数高压基准实测报告 (examples/benchmark_200/Benchmark_ax.md)](examples/benchmark_200/Benchmark_ax.md)
- 🤖 [多 Agent 协作工作流与 AST 存储规范 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 📝 [语言语法与语义规范草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 👾 [Axiom Survivors Unity 示例工程 (examples/game_survivor/)](examples/game_survivor/README.md)
