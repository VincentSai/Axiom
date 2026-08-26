# Axiom: AI-Native 全平台并发编程语言

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

## 📂 项目文档导航

- 📘 [架构设计白皮书 (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [语言语法与语义规范草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [多 Agent 协作工作流与 AST 存储 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
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