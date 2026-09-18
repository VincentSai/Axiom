# Axiom: 面向 AI 协作的可验证逻辑中间层与运行时 (AI-Native Verified Logic Layer)

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)  
> 📄 **核心文档**: [技术成果白皮书与审查终卷 (docs/WHITEPAPER.md)](docs/WHITEPAPER.md) | [单一变量实证报告 (examples/benchmark_eval/EVALUATION_REPORT.md)](examples/benchmark_eval/EVALUATION_REPORT.md)

---

> **Axiom** 是一个专门面向 **AI 协作的受限纯函数 DSL、形式化验证与多 Agent 协同流水线**。  
> **核心战略定位**：**不替代通用语言 (Rust / C# / TypeScript)**，而是与其深度共生——Axiom 专注于最适合 AI 生成、最需要确定性防御的核心逻辑层（数值管线、状态机转移、规则判定、权限校验），外围无缝转译并嵌入 Rust、Unity C#、WASM 与 C。  
> **生产级能力链与核心价值**：
> 1. **受限纯表达式与充要契约**：语言内核仅保留纯表达式、代数效应边界与定点数微元制，消除隐式副作用与精度漂移，收敛在可判定的 SMT 理论域；
> 2. **深模块与统一级联提交内核**：通过 `VerifiedWorkspace` 封装 CAS 内容寻址与 AST 3-Way 合并，所有提交路径强制执行符号依赖闭包级联重验，杜绝隐式语义破坏；
> 3. **SMT 反例驱动单步自愈**：精确提取机器反例引导 Agent 单步增加守卫分支，自愈收敛耗时低于通用重试；
> 4. **定理证明与业务 Oracle 双重闭环**：不仅在全域取值上通过 Z3 形式化证明杜绝溢出/除零/越界，更通过转译为原生 Rust 执行业务测试向量验收，杜绝契约不充要带来的业务偏差；
> 5. **可审计的验证门禁与基准体系**：构建涵盖全要素离线协议仿真与模型实测的双轨评测，以不可变 `run_id` 快照持久化每一次实验过程；在历史探索基准中曾测得 21.0% Token 节省记录（定性为待新一轮对决复核的产品假说，详见正文说明）。

---

## 🌟 核心架构与端到端能力链

```
┌─────────────────────────────────────────────────────────────────┐
│               Host Presentation / Engine Layer                  │
│       Unity / C#  •  Web / TypeScript  •  Native Rust / C       │
└───────────────────────────────┬─────────────────────────────────┘
                                │ FFI / Native Bridge / WASM
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│           Axiom Verified Logic Layer (VerifiedWorkspace)        │
│  • Pure Functions (受限纯表达式)   • Algebraic Effects (显式副作用隔离) │
│  • Decimal Fixed-Point (微元制)   • SMT Contracts (requires/ensures)│
│  • AST 3-Way CAS Database        • Unified Cascade Submit Kernel   │
│  • ProcessSupervisor (看门狗防挂死) • Rust Business Oracle Testbed  │
└───────────────────────────────┬─────────────────────────────────┘
                                │ Task Envelope & Holes
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│              Multi-Agent Orchestration & Self-Healing           │
│   • OpenAI Codex CLI • Z3 SMT Counter-Example • 1-Step Healing  │
└─────────────────────────────────────────────────────────────────┘
```

**端到端真实能力链**：
`局部任务信封 (Task Envelope) -> Agent 受限表达式生成 -> SMT 全域定理证明 -> 依赖闭包级联验证 -> CAS 审计回执生成 -> AST 3-Way 重基提交 -> 无洞纯模块发布与业务 Oracle 验收`

---

## 🏆 基准评测与宏观并发实证 (Benchmark & Empirical Evidence)

Axiom 构建了涵盖**离线全要素协议仿真 (Mock)**与**真实模型宏观对决 (Live)**的双轨评测体系（评测套件位于 [`examples/benchmark_eval/`](examples/benchmark_eval/)）：

### 1. 离线全要素物理仿真基准 (Mock Benchmark)
- 完整流转 5 Worker 纯代数填洞、AST CAS 组合器零文本冲突装配、Z3 SMT 全域定理证明以及 Rust 业务测试向量双重验收；
- 证明了打洞协议、词法作用域类型系统、进程看门狗与级联提交闭环的端到端结构自洽性；
- 详细离线协议审计数据存档于 [`examples/benchmark_eval/data/mock/concurrent_eval_data.json`](examples/benchmark_eval/data/mock/concurrent_eval_data.json)（严格标注为 `synthetic_non_evidentiary` 协议自洽性基准），并支持通过 `runs/{run_id}.json` 追溯不可变运行档案。

### 2. 真实模型并发评测假说与历史基准记录 (Live Empirical Observations)
> [!NOTE]
> 下表数据为历史原型版本（commit `a22ce40` 前期探索版本，基于 OpenAI Codex CLI `v0.147.0`, `gpt-5.6-sol` 与 Z3 4.13.0）在 inventory_settlement 微任务下的单次基线观测记录。在当前版本引入更严密的 Fail-Closed 浮点拦截与严格双重门禁后，**“代数打洞是否必然带来全链路 Token 净节省”被作为核心产品假说**，待后续具备信息等价对照组与不可变实时快照的新一轮 Live 实测进一步复核检验，不作为当前版本的固有必然收益外推。

| 对照维度 | Group 2: 传统工程微任务 (Rust Git Flow) | Group 3: Axiom 形式化打洞流水线 | 历史实测观察 (commit a22ce40) |
| :--- | :--- | :--- | :--- |
| **协作机制** | 5 分支独立生成 + 串行 Git 合并 | 5 Worker 纯代数填洞 + AST CAS 组合器 | 物理杜绝文本合并冲突 |
| **合并冲突率** | 0.0% (无二次文本冲突修复) | **0.0% (零文本级冲突)** | 保持同等高质量干净交付 |
| **Worker并发耗时** | 25.07 秒 | **22.97 秒** | 代数打洞 Worker 推理更聚焦 |
| **全链路总账单** | **95,263** Tokens | **75,238** Tokens | **历史单次记录节省 21.0% Token (待新版复核)** |
| **组合器/合并消耗** | 0 Token (仅 Git 引擎本地合并) | **0 Token (原生 AST CAS 零模型调用)** | 保持 0 文本解决税 |
| **双Worker CAS争用压测** | - | **2/2 签入成功 (耗时 17.86s)** | 跨进程排他锁与 AST 重基 100% 稳健 |
| **终态质量保证** | 基于离散测试向量采样通过 | **Z3 SMT 全域定理证明 + 业务 Oracle 双重闭环** | **兼顾数学反例消灭与业务意图对齐** |

### 3. 适用子集与方法学边界说明
- **适用逻辑子集**：当前形式化证明与转译同构严格收敛于**无环纯表达式、定点微元制 Decimal (i64/i128)、显式代数效应边界**；
- **Fail-Closed 拦截**：由于实数理论 (Real) 无法健全建模 IEEE-754 有限字长浮点舍入与精度发散，凡带契约且涉及 `Type::Float` 的函数一律采取严格的 **Fail-Closed 拦截并返回 `Skipped`**，绝不签发虚假证明；
- **业务 Oracle 双重闭环**：形式化定理证明解决的是“程序不违背已写明的数学契约”；必须结合 Transpiler 转译为物理机器码后执行业务测试向量（Business Oracle），消除契约不充要导致的业务脱节；
- **运行档案与可复现性**：评测运行器全面支持唯一不可变 `run_id` 快照持久化（`data/mock/runs/` 及 `data/live/runs/`），每次运行均记录 Git Commit、Tree Hash、Dirty Status 及 Solver Identity，实现全程可溯源审计。

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
