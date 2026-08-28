# Axiom: AI-Native 全平台并发编程语言

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** 是一门专门为 **AI Agent 协作** 与 **全平台跨端发布** 量身设计的现代编程语言。  
> 核心理念：消除代码冲突、杜绝隐式副作用、内置形式化契约与类型打洞、实现数十个 Agent 无锁并行编写。

---

## 🌟 核心愿景与问题背景

在 AI 编码大模型时代，传统的文本文件存储、隐式全局状态、复杂生命周期以及基于行差分的 Git 协作方式成为了阻碍 AI 高效、无冲突并行开发的最大瓶颈：
- **状态失控**：隐式可变状态使得 Agent 难以追踪全局变化，导致生成代码频繁产生隐藏 Bug。
- **合并冲突**：多 Agent 并行编写传统源码文件时，极易产生 Git Merge 冲突与符号重名。
- **跨平台沉重**：现有跨端方案要么依赖庞大的解释器（Python/JS），要么心智负担极高（Rust 借用检查器）。

**Axiom** 从底层重新思考了语言与工具链的设计：
1. **纯函数 + 显式代数效应 (Algebraic Effects)**：杜绝全局状态污染，单函数 100% 自闭环。
2. **内容寻址 AST (Content-Addressed Codebase)**：代码以 AST Hash 存储，彻底消灭命名冲突与 Git Merge 冲突。
3. **类型打洞协同 (Hole-Driven Development)**：架构师 Agent 声明骨架与 `?hole`，Worker Agents 精准填空，编译器提供结构化反例闭环。
4. **全平台转译与 WASM 基底**：转译至 Rust/C 与 WebAssembly (WASM/WASI)，一键发布至 Web、iOS、Android、Windows、Linux、macOS 及嵌入式设备。
5. **双轨生态互操作**：支持 W3C WIT (WebAssembly Component Model) 与声明式 FFI 包装，零成本复用全球已有开源生态。

---

## 📊 对比热门编程语言

| 语言 | 优势 | 劣势 (在 AI / 跨平台 / 并发生成 视角下) | Axiom 的解法 |
| :--- | :--- | :--- | :--- |
| **Python** | LLM 训练语料最丰富，生成流畅 | 动态类型易产生隐式 Bug，跨平台打包极重，全局状态易被多 Agent 污染 | 保留极简表达式与模式匹配，引入纯函数、强类型与全平台 WASM 转译 |
| **Rust** | 内存安全、性能极致、跨平台好 | 借用检查器与复杂生命周期导致 AI 试错成本极高（易陷入编译器报错死循环） | 保留 Rust 级别的内存安全与性能底座，对上层屏蔽生命周期心智负担 |
| **Go** | 编译快速、跨平台友好、并发简单 | 缺乏代数数据类型 (ADT) 与形式化契约，多 Agent 仍有命名与接口破坏风险 | 引入 ADT、显式 Effect、内置 Pre/Post 契约与内容寻址 AST |
| **TypeScript** | Web/全栈生态丰富 | 运行时无法保证绝对类型安全，跨 Native 端依赖重型运行时，异步传染 | 强类型自描述 + 转译为 Rust/WASM，零运行时开销，统一跨端 |

---

## 🏆 权威基准评测：200-Function 真实业务多 Agent 协作大比拼

在工业级游戏引擎规范 [`examples/benchmark_200/SPEC_200.md`](examples/benchmark_200/SPEC_200.md) 下（涵盖空间几何、非线性护甲减伤、五行相克、战斗状态机、AI 衰减、加权掉落与世界调度等 200 个真实业务函数），由真实 AI 大模型（OpenAI Codex CLI）多 Agent 并发协作开发，**Axiom、Python、Rust 与 TypeScript** 四大语言体系的实测数据矩阵如下：

| 语言体系 | 多 Agent 协作模式与架构 | 任务粒度与上下文边界 | 真实消耗 Tokens | 正确性验证手段与耗时 | 形式化数学保证 | 相对基准对比 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **`/axiom-pipeline` 全自动打洞流水线**<br>(Architect + 编译器拆解 + 200 Workers) | **单函数 `hole-task.json` 隔离<br>($O(1)$ 超轻量局部上下文)** | **183,662 Tokens** | **SMT 全域定理证明<br>(0.08 秒 200/200)** | **✅ 全数域 $(-\infty \sim +\infty)$<br>100% 绝对零漏洞** | **🔥 节省 58.9% ~ 61.7%<br>(2.4x 提速)** |
| **🐍 Python** | 5 个 Agent 模块化并发 (Tier 0~4) | 40~50 函数/大模块 ($O(N)$ 大文件) | **384,089 Tokens** | Unittest (0.002s, 9/9 Passed) | ⚠️ 仅依赖抽样单元测试 | 0.86x 消耗 |
| **🦀 Rust** | 5 个 Agent 跨 crate/mod 并发 | 40~50 函数/大模块 ($O(N)$ 大文件) | **446,775 Tokens** | Cargo test (0.46s, 6/6 Passed) | ⚠️ 仅依赖抽样单元测试 | 1.00x 基准参照 |
| **📘 TypeScript** | 5 个 Agent 模块化并发 (ESM/TS) | 40~50 函数/大模块 ($O(N)$ 大文件) | **479,872 Tokens** | TSX / Node test (0.48s, 8/8 Passed) | ⚠️ 仅依赖抽样单元测试 | 1.07x 消耗 |

### 🔬 为什么 Axiom 打洞流水线具备颠覆性优势？
1. **$O(1)$ 微型上下文隔离，消灭幻觉与 Token 浪费**：
   传统语言由于 Agent 必须在 40~50 函数的大文件中反复阅读与重写，上下文持续膨胀。Axiom 编译器原生工具 `axiom extract-holes` 机械化拆解出 200 个独立的 `hole-task.json`（每个仅 ~160 Tokens），Worker 仅需输出单个纯表达式（~18 Tokens），**200 个函数的实现阶段仅消耗 35,627 Tokens**！
2. **0.08 秒 SMT 全数域形式化定理证明**：
   传统语言面对边界溢出只能依靠有限用例抽样，而 Axiom 内置 Z3 定理证明器在 0.08 秒内对 200 个函数的契约完成了 $(-\infty \sim +\infty)$ 全数域数学证明，**从根本上杜绝了隐式 Bug 与数值溢出**。

> 📖 详细跨语言测试报告、各模块消耗明细与完整源码，请参阅：[**200-Functions 基准测试专题 (examples/benchmark_200/README.md)**](examples/benchmark_200/README.md)。

---

## 📂 项目文档导航

- 📘 [架构设计白皮书 (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [语言语法与语义规范草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [多 Agent 协作工作流与 AST 存储 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🔬 [主流语言深度对比与能力边界分析 (docs/RESEARCH_LANGUAGE_COMPARISON.md)](docs/RESEARCH_LANGUAGE_COMPARISON.md)
- 🗺️ [项目发展路线图 (docs/ROADMAP.md)](docs/ROADMAP.md)
- 💡 [示例 1：银行转账服务 (examples/transfer.ax)](examples/transfer.ax)
- 🎮 [示例 2：跨平台游戏循环 (examples/game_loop.ax)](examples/game_loop.ax)

---

## 🛠️ 语法速览 (Syntax Glance)

```rust
// 1. 不可变代数数据类型 (ADT)
type Player = {
    id: Int,
    name: String,
    score: Int,
}

// 2. 声明显式副作用 (Algebraic Effect)
effect Audio {
    fn play_sound(sound_id: String): ()
}

effect Network {
    fn send_packet(packet: String): ()
}

// 3. 架构师 Agent 定义的带契约函数骨架 (含 ?hole 打洞)
fn level_up(player: Player, bonus: Int) -> Player 
    with [Audio, Network]
    requires {
        bonus > 0,
        player.score >= 0,
    }
    ensures {
        result.score == player.score + bonus
    }
{
    // 交由 Worker Agent 并发填洞实现
    ?level_up_implementation
}
```

---

## 🚀 快速安装与预编译二进制下载 (Installation & Download)

无需安装 Rust 编译环境，你可以直接从 GitHub 下载开箱即用的预编译二进制与技能包：

### 1. 下载 Release 预编译包
前往 GitHub 仓库的 [**Releases**](../../releases) 页面，下载对应系统的压缩包（**每个压缩包内均已内置二进制与完整的 `skills/` 技能套件**）：
- **Windows (x64)**: `axiom-v*-x86_64-pc-windows-msvc.zip`（内含 `axiom.exe` 与 `skills/`）
- **Linux (x64)**: `axiom-v*-x86_64-unknown-linux-gnu.tar.gz`（内含 `axiom` 与 `skills/`）
- **macOS (Apple Silicon)**: `axiom-v*-aarch64-apple-darwin.tar.gz`
- **macOS (Intel)**: `axiom-v*-x86_64-apple-darwin.tar.gz`
- **独立技能包 (通用)**: `axiom-skills-v*.zip`（纯 Agent Skills 规则包）

### 2. 配置环境变量 (Windows)
1. 将下载的压缩包解压到任意固定目录（例如 `C:\Program Files\Axiom\` 或 `D:\Tools\axiom\`）；
2. 将该目录路径添加到系统的 **环境变量 `PATH`** 中；
3. 打开新的终端（PowerShell 或 CMD），运行 `axiom --help` 验证安装成功。

> 💡 **开发者源码安装（可选）**：  
> 若本地拥有 Rust 工具链，可直接在仓库根目录执行：`cargo install --path crates/axiom-cli`

---

## 🤖 AI Agent 技能包 (Skills) 安装与激活指南

Axiom 随 Release 提供了专为现代 AI 编码 Agent 设计的 5 大协同技能包：
- `/axiom-pipeline`：全自动打洞流水线（架构师打洞 -> 提取任务 -> Worker 并发填洞 -> SMT 证明 -> 自愈）
- `/axiom-interactive`：分阶段人机协作打洞（先生成契约等待人类审查，确认后再极速填洞）
- `axiom-architect` / `axiom-worker` / `axiom-healer`：微专业 Agent 规范

### 各主流 Agent 客户端安装方法：

| AI Agent 客户端 | 安装路径 / 配置方法 | 激活与使用命令 |
| :--- | :--- | :--- |
| **🌟 Google Antigravity / Gemini CLI** | 将解压出的 `skills/*` 复制到全局 `%USERPROFILE%\.gemini\config\skills\` 或项目目录 `.gemini/config/skills/` | 在聊天框直接输入：<br>`/axiom-pipeline 实现...` |
| **⚡ Cursor IDE** | 将 `skills/axiom-pipeline/SKILL.md` 复制为项目根目录 `.cursor/rules/axiom-pipeline.mdc`（或追加至 `.cursorrules`） | 在 Composer / Agent 模式输入：<br>`使用 axiom-pipeline 规范实现...` |
| **🌊 Windsurf (Codeium)** | 将 `skills/axiom-pipeline/SKILL.md` 的内容追加到项目根目录的 `.windsurfrules` | 在 Cascade 聊天框中输入：<br>`遵循 Axiom 打洞流水线实现...` |
| **🧠 Claude Code / Claude Desktop** | 将技能内容追加至项目根目录 `CLAUDE.md`，或在 Claude.ai Project Knowledge 中上传 `skills/` 下的 markdown 文件 | 在对话中输入：<br>`启动 /axiom-pipeline 实现...` |
| **🤖 OpenAI Codex CLI / Aider** | 在当前工作区放入 `skills/` 或在系统提示词中引用 `skills/axiom-pipeline/SKILL.md` | 在提示词中指定：<br>`根据 skills/axiom-pipeline 规范开发` |

> 📖 更多详细技能说明，请参阅：[**Axiom 技能包使用手册 (skills/README.md)**](skills/README.md)。

---

## 💻 CLI 工具链与 `axiom.exe` 使用指南

配置好环境变量后，你可以直接使用 `axiom`（或 `axiom.exe`）命令调用完整的工具链功能：

```powershell
# 1. 语法检查与打洞统计
axiom check examples/transfer.ax

# 2. 静态 SMT 契约形式化验证（人类可读控制台模式）
axiom verify examples/transfer.ax

# 3. 静态 SMT 形式化验证（输出机器可读 JSON，供 Agent 闭环自愈）
axiom verify examples/transfer.ax --json

# 4. 机械化提取打洞任务 JSON（派发给多个 Worker Agent 并发填洞）
axiom extract-holes examples/transfer.ax
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 5. 内容寻址 AST 代码库 (ASTDB) 管理与虚拟源码投影
axiom db put examples/transfer.ax
axiom db export #mod_589e44855c2b4e0223acf6867cdd3c6f0947fb3a23d7afafbb62e7fbc044559a

# 6. WebAssembly Component Model (WIT) 契约导出
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit

# 7. 多语言 FFI 胶水代码与类型声明导出 (TypeScript .d.ts / C 头文件 .h / Rust C-ABI)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-ffi examples/game_loop.ax --lang rust -o game_loop_c_abi.rs

# 8. 全平台工程脚手架一键生成 (支持 web / native / node)
axiom scaffold my_game --target web --out-dir ./projects

# 9. 转译至 Rust 源码并直接编译运行
axiom emit-rust examples/game_loop.ax -o game_loop.rs
axiom run path/to/program_with_main.ax
```

> ⚙️ **源码调试模式**：若未安装全局二进制，在源码根目录下将 `axiom <subcommand>` 替换为 `cargo run -p axiom-cli -- <subcommand>` 即可达到完全相同的运行效果。
