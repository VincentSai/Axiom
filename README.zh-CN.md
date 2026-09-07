# Axiom: 面向 AI 协作的确定性游戏逻辑 DSL 与形式化验证原型

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** 是一门专门为 **AI Agent 协作** 与 **跨引擎确定性逻辑** 探索的现代领域特定语言 (DSL) 原型系统。  
> 核心探索方向：通过纯函数消除隐式副作用、通过类型打洞（Hole-Driven）实现 Agent 局部上下文隔离、通过 SMT-LIB2 / Z3 进行数学契约定理证明，以及通过零拷贝双缓冲直通游戏引擎（如 Unity）。

---

## 🌟 核心设计理念

在 AI 辅助游戏逻辑开发的场景中，传统的通用编程语言存在若干痛点：
- **状态失控与难以追踪**：隐式全局状态和随机副作用导致 AI 生成的代码容易产生隐藏边界 Bug 和联机不同步。
- **上下文膨胀与幻觉**：让 Agent 在庞大工程代码中反复通读和修改，极易导致 Token 浪费与上下文污染。
- **跨引擎胶水繁琐**：不同游戏引擎和平台间传递状态时，缺乏零 GC、零拷贝的确定性内存同步机制。

**Axiom** 围绕这些问题构建了一套自闭环的探索型工具链：
1. **纯函数 + 显式代数效应 (Algebraic Effects)**：游戏逻辑表达为无副作用的纯状态转移，副作用显式隔离。
2. **SMT-LIB2 / Microsoft Z3 定理证明**：内置 `requires` / `ensures` 契约，可一键编译为标准 SMT-LIB2 脚本并通过 Z3 求解器证明或提取反例，内置轻量符号区间引擎。
3. **微型打洞协作 (Hole-Driven Development)**：通过 `?hole` 声明待实现点，编译器可机械化拆解出独立的 `hole-task.json`，让 Agent 在 $O(1)$ 极简局部上下文中精准填空。
4. **Unity 零拷贝双缓冲桥接 (`axiom bridge`)**：转译至 Rust 编译为 C-ABI 原生库，自动生成 Unity C# P/Invoke 与连续内存双缓冲快照。
5. **多语言绑定与内容寻址 AST 原型**：提供 TypeScript `.d.ts`、C `.h` 与 WebAssembly WIT 接口生成，探索基于 SHA-256 的 AST 模块存储。

---

## 🎮 实战演示：Axiom Survivors 割草游戏

在 [`examples/game_survivor/`](examples/game_survivor/) 中提供了一个完整的 2D 肉鸽割草游戏落地示范：
- **形式化逻辑内核 ([`survivor_core.ax`](examples/game_survivor/ax/survivor_core.ax))**：涵盖角色移动、弹道欧拉物理推进、欧几里得圆形碰撞、伤害减伤与经验升级突破等 15 个纯函数与数学契约；
- **Unity 表现层 ([`SurvivorGameController.cs`](examples/game_survivor/Assets/Scripts/Gameplay/SurvivorGameController.cs))**：零配置自举（打开 SampleScene 即可试玩）、过程化像素精灵生成、平滑相机跟随与死亡一键满血重开；
- **零 GC 双缓冲同步**：Rust Native 库与 Unity 之间通过定长连续内存进行 60FPS+ 双缓冲传递。

---

## 🔬 实验性评测：200-Function 微型任务拆解探索

在 [`examples/benchmark_200/SPEC_200.md`](examples/benchmark_200/SPEC_200.md) 领域规范下（涵盖空间几何、非线性减伤、状态机与数值曲线等 200 个函数），我们进行了微型打洞任务与传统大文件生成模式的对比实验：

- **微型打洞流水线 (`/axiom-pipeline`)**：利用 `axiom extract-holes` 将 200 个函数拆解为隔离的 `hole-task.json` 单任务，使 Worker Agent 仅需在局部上下文中输出单个表达式，大幅减少了单任务的 Prompt 上下文冗余；
- **SMT 契约约束**：通过 SMT-LIB2 与求解器在提交阶段快速过滤出包含边界缺陷的反例实现；
- **说明与局限性**：该实验主要验证局部上下文打洞模式在 Token 消耗与形式化验证上的可行性；具体性能与耗时受本地硬件、求解器算力与测试用例复杂度影响。

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

## 🎨 IDE 支持与工具链生态 (IDE Support)

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

# 7. 转译至 Rust 源码并由 rustc 编译运行
axiom emit-rust examples/game_loop.ax -o game_loop.rs
axiom run path/to/program_with_main.ax

# 8. 多语言接口绑定导出 (TypeScript / C / WIT)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit
```

---

## 📂 项目文档导航

- 📘 [架构设计白皮书 (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [语言语法与语义规范草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [多 Agent 协作工作流与 AST 存储 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🔬 [主流语言能力边界探索 (docs/RESEARCH_LANGUAGE_COMPARISON.md)](docs/RESEARCH_LANGUAGE_COMPARISON.md)
- 🗺️ [项目发展路线图 (docs/ROADMAP.md)](docs/ROADMAP.md)
- 👾 [Axiom Survivors Unity 示例工程 (examples/game_survivor/)](examples/game_survivor/README.md)
