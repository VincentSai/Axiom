# Axiom: 面向 AI 协作的可验证逻辑中间层与运行时 (AI-Native Verified Logic Layer)

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** 是一套专门面向 **AI Agent 协作与形式化数学验证** 的现代可验证程序表示与执行系统。  
> **核心战略定位**：**不替代 Rust / C# (Unity) / TypeScript**，而是与其深度共生——Axiom 专注于最适合 AI 生成、最需要确定性与数学契约防御的**核心逻辑层**（游戏机制、数值管线、状态机、交易结算、Agent 工具权限），外围无缝转译并嵌入 Rust、Unity C#、WASM 与 C。  
> **核心技术壁垒**：**打洞驱动开发 (Hole-Driven) + SMT 形式化契约定理证明 + 多 Agent 局部语义协作模型**。

---

## 🌟 核心设计理念与共生架构

```
┌─────────────────────────────────────────────────────────────────┐
│               Host Presentation / Engine Layer                  │
│       Unity / C#  •  Web / TypeScript  •  Native Rust / C       │
└───────────────────────────────┬─────────────────────────────────┘
                                │ FFI / Native Bridge / WASM
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│               Axiom Verified Logic Layer (Axiom 内核)            │
│  • Pure Functions (不可变状态转移) • Algebraic Effects (显式副作用隔离)  │
│  • SMT Formal Contracts (保证满足数学契约) • Hole-Driven Micro-Workers  │
└─────────────────────────────────────────────────────────────────┘
```

在 AI 辅助核心业务与游戏逻辑开发的场景中，传统的通用编程语言存在若干痛点：
- **状态失控与边界隐患**：隐式全局状态和随机副作用导致 AI 生成的代码容易产生隐藏边界 Bug 和联机不同步。
- **上下文膨胀与注意力发散**：让 Agent 在庞大工程长文件中反复通读和修改，极易导致 Token 浪费与幻觉。
- **测试有限性 vs 形式化契约**：手写单元测试只能抽样验证，无法从数学上穷举边界缺陷。

**Axiom** 围绕这些问题构建了面向 AI 的自闭环工具链：
1. **纯函数 + 显式代数效应 (Algebraic Effects)**：业务逻辑表达为无副作用的纯状态转移；副作用通过 Effect 显式声明，转译端自动生成原生 Rust Trait 与 Handler 依赖注入。
2. **SMT-LIB2 / Microsoft Z3 定理证明**：内置 `requires` / `ensures` 契约，可一键编译为标准 SMT-LIB2 脚本并通过 Z3 求解器证明或提取反例，**数学级保证程序严格满足形式化契约**。
3. **微型打洞协作 (Hole-Driven Development)**：通过 `?hole` 声明待实现点，编译器机械化拆解出独立的 `hole-task.json`，让 Agent 在 $O(1)$ 极简局部上下文中精准填空。
4. **多 Agent 协作任务池 (`axiom pool`)**：内置多 Agent 打洞协调系统，支持候选解提交、全自动 SMT 反例过滤拦截与无损代码组装。
5. **Unity 零拷贝双缓冲桥接 (`axiom bridge`)**：转译至 Rust 编译为 C-ABI 原生库，自动生成 Unity C# P/Invoke 与连续内存双缓冲快照。

---

## 🔬 严谨科学实证：单一变量 A/B 对照评测体系

外部评测曾提出过一个核心问题：*“Axiom 的 Token 节省与可靠性，到底是因为语言本身，还是仅仅因为工作流切成了微任务？”*

为了彻底解耦变量，我们在 [`examples/benchmark_eval/`](examples/benchmark_eval/) 中建立了严格的 **单一变量 A/B 科学对照评测套件**：
- **黄金标准任务集 ([`golden_tasks.json`](examples/benchmark_eval/golden_tasks.json))**：精选覆盖空间几何、非线性抗性、战斗数值、掉落状态机与世界仿真的 20 个黄金业务任务；
- **三组对照实验协议 ([`prompt_templates.md`](examples/benchmark_eval/protocols/prompt_templates.md))**：
  1. **Group 1**：传统语言大文件工作流（Rust Monolith，长代码上下文）；
  2. **Group 2（核心解耦）**：传统语言单函数微任务（Rust Micro-Worker，消除任务拆解颗粒度差异）；
  3. **Group 3**：Axiom 契约打洞流水线（Axiom Hole Pipeline，基于数学契约与 SMT 定理证明）。
- **官方 API 驱动脚本 ([`eval_runner.py`](examples/benchmark_eval/eval_runner.py))**：
  基于 Google Gemini 官方 API（`countTokens` 与 `generateContent`），**使用官方真实分词器与官方 usageMetadata** 统计真实物理 Token 消耗与通过率，杜绝任何主观伪造数据。用户配置 `GEMINI_API_KEY` 后即可在本地一键运行复测。

---

## 🎮 实战演示：Axiom Survivors 割草游戏

在 [`examples/game_survivor/`](examples/game_survivor/) 中提供了一个完整的 2D 肉鸽割草游戏落地示范：
- **形式化逻辑内核 ([`survivor_core.ax`](examples/game_survivor/ax/survivor_core.ax))**：涵盖角色移动、弹道欧拉物理推进、欧几里得圆形碰撞、伤害减伤与经验升级突破等 15 个纯函数与数学契约；
- **Unity 表现层 ([`SurvivorGameController.cs`](examples/game_survivor/Assets/Scripts/Gameplay/SurvivorGameController.cs))**：零配置自举（打开 SampleScene 即可试玩）、过程化像素精灵生成、平滑相机跟随与死亡一键满血重开；
- **零 GC 双缓冲同步**：Rust Native 库与 Unity 之间通过定长连续内存进行 60FPS+ 双缓冲传递。

---

## 🚀 安装与构建 (Installation & Build)

通过 Cargo 从源码一键编译并安装 `axiom` CLI 工具链：

```bash
# 从源码安装 axiom CLI 到本地 PATH
cargo install --path crates/axiom-cli

# 验证安装
axiom --help
```

---

## 🎨 IDE 支持与生态规划 (IDE Support)

- **VS Code 扩展 (规划中)**：语言服务器协议 (LSP) 与 TextMate 语法高亮正在规划和开发中（相关目录位于 [`editors/vscode-axiom`](editors/vscode-axiom)）。
- **当前推荐**：直接使用 `axiom check` 与 `axiom verify` 命令行工具链配合编辑器终端进行语法、类型与契约验证。

---

## 💻 CLI 工具链指南

```powershell
# 1. 语法与静态类型检查、打洞统计
axiom check examples/transfer.ax

# 2. 形式化契约验证 (自动探测 Z3 求解器，回退至内置符号引擎；支持 --strict 模式)
axiom verify examples/benchmark_200/ax/engine_200.ax
axiom verify examples/benchmark_200/ax/engine_200.ax --strict

# 3. 多智能体打洞任务池调度与验证 (Agent Task Pool)
axiom pool status examples/benchmark_200/ax/engine_200_holes.ax
axiom pool submit examples/math.ax --hole calc_h0 --expr "x + 1"
axiom pool assemble examples/math.ax --solutions solutions.json -o math_verified.ax

# 4. 导出标准 SMT-LIB2 (.smt2) 文件供第三方求解器 (Z3/CVC5) 独立验证
axiom export-smt2 examples/transfer.ax -o transfer.smt2

# 5. 机械化提取打洞任务 JSON (派发给 Worker Agents 并发填洞)
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 6. Unity 跨端代码生成与零拷贝双缓冲桥接
axiom bridge examples/game_survivor/ax/survivor_core.ax --target unity --out examples/game_survivor/Assets/Scripts/AxiomBridge --library-name axiom_survivor

# 7. 转译至 Rust 源码并由 rustc 原生编译（支持代数效应 Trait 生成与透传）
axiom emit-rust examples/game_loop.ax -o game_loop.rs
rustc --crate-type lib game_loop.rs -o game_loop.rlib

# 8. 多语言接口绑定导出 (TypeScript / C / WIT)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit
```

---

## 📂 项目文档导航

- 🔬 [单一变量 A/B 科学对照评测方法与套件 (examples/benchmark_eval/README.md)](examples/benchmark_eval/README.md)
- 📊 [200 函数高压基准实测报告 (examples/benchmark_200/Benchmark_ax.md)](examples/benchmark_200/Benchmark_ax.md)
- 📘 [架构设计白皮书 (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [语言语法与语义规范草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [多 Agent 协作工作流与 AST 存储 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🗺️ [项目发展路线图 (docs/ROADMAP.md)](docs/ROADMAP.md)
- 👾 [Axiom Survivors Unity 示例工程 (examples/game_survivor/)](examples/game_survivor/README.md)
