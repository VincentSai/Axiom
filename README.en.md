# Axiom: AI-Native Concurrent Programming Language for All Platforms

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** is a modern programming language designed from the ground up for **AI Agent Collaboration** and **Universal Cross-Platform Deployment**.  
> Core principles: Zero merge conflicts, zero implicit side effects, first-class formal contracts, and hole-driven development for lock-free multi-agent concurrency.

---

## 🌟 Core Vision & Problem Statement

In the era of large language models for coding, traditional software architectures—characterized by physical text files, implicit global mutable state, manual lifetime constraints, and line-based Git merging—have become the primary bottleneck hindering parallel AI development:
- **Uncontrolled State**: Implicit mutable state makes it difficult for LLM agents to reason about global invariants, leading to subtle regressions.
- **Merge Conflicts**: Multiple agents writing to shared source files trigger frequent Git conflicts and symbol collisions.
- **Heavy Cross-Platform Overhead**: Existing cross-platform runtimes are either resource-heavy (Python/JS runtimes) or introduce steep cognitive friction (Rust's borrow checker).

**Axiom** rethinks language and toolchain design from first principles:
1. **Pure Functions + Explicit Algebraic Effects**: Eliminates global state pollution; every function is 100% self-contained.
2. **Content-Addressed AST Codebase**: Code is identified and stored by AST hash, eradicating naming collisions and line-based Git conflicts.
3. **Hole-Driven Development (`?hole`)**: Architect agents declare architectural contracts and `?hole` placeholders; worker agents fill them in parallel with compiler-guided SMT counterexample feedback.
4. **Universal Transpilation & WASM Substrate**: Transpiles to clean Rust/C and WebAssembly (WASM/WASI) for single-click deployment across Web, iOS, Android, Windows, Linux, macOS, and embedded MCUs.
5. **Dual-Track Ecosystem Interop**: Native W3C WIT (WebAssembly Component Model) and declarative FFI bindings allow zero-cost reuse of global open-source libraries.

---

## 📊 Comparison with Mainstream Languages

| Language | Strengths | Limitations in Multi-Agent & Parallel Workflows | Axiom's Solution |
| :--- | :--- | :--- | :--- |
| **Python** | Richest LLM pre-training corpus, rapid prototyping | Dynamic typing leads to runtime bugs; heavy deployment bundle; global state easily corrupted | Minimal expressions & pattern matching; pure functions, static typing, universal WASM transpilation |
| **Rust** | Memory safety, zero-cost abstractions, robust cross-platform | Borrow checker & complex lifetimes cause LLMs to get stuck in compiler error loops | Rust-grade memory safety & performance substrate; shields agents from manual lifetime annotations |
| **Go** | Fast compilation, simple concurrency model | Lacks Algebraic Data Types (ADTs) and formal contracts; risk of interface drift across agents | First-class ADTs, explicit Effects, Pre/Post contracts, and content-addressed AST storage |
| **TypeScript** | Enormous Web & full-stack ecosystem | Cannot guarantee runtime soundness; heavy JS runtime on native platforms; async contagion | Strongly typed self-describing contracts transpiled to Rust/WASM; zero-runtime overhead across native targets |

---

## 🏆 Industrial Benchmark: 200-Function Multi-Agent Concurrency

Under the industrial RPG game engine specification [`examples/benchmark_200/SPEC_200.md`](examples/benchmark_200/SPEC_200.md) (comprising 200 production functions across spatial geometry, nonlinear armor curves, elemental affinity graphs, combat state machines, AI threat decay, weighted loot drop tables, and world scheduling), four language ecosystems were implemented by concurrent OpenAI Codex CLI agents. The benchmark results are summarized below:

| Language Ecosystem | Multi-Agent Orchestration Architecture | Task Granularity & Context Scope | Real Tokens Consumed | Correctness Verification & Duration | Formal Mathematical Guarantee | Comparison vs Rust Baseline |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **`/axiom-pipeline` Automated Hole Pipeline**<br>(Architect + CLI Extractor + 200 Micro-Workers) | **Single-function `hole-task.json` isolation<br>($O(1)$ ultra-compact local context)** | **183,662 Tokens** | **SMT Formal Theorem Prover<br>(0.08s for 200/200)** | **✅ Entire Number Domain $(-\infty \sim +\infty)$<br>100% Mathematically Proven** | **🔥 58.9% ~ 61.7% Savings<br>(2.4x Speedup)** |
| **🐍 Python** | 5 Concurrent Agents (Tier 0 ~ Tier 4) | 40~50 functions/module ($O(N)$ large files) | **384,089 Tokens** | Unittest (0.002s, 9/9 Passed) | ⚠️ Sampled unit testing only | 0.86x Consumption |
| **🦀 Rust** | 5 Concurrent Agents across crates/mods | 40~50 functions/module ($O(N)$ large files) | **446,775 Tokens** | Cargo test (0.46s, 6/6 Passed) | ⚠️ Sampled unit testing only | 1.00x Baseline |
| **📘 TypeScript** | 5 Concurrent Agents (ESM/TS) | 40~50 functions/module ($O(N)$ large files) | **479,872 Tokens** | TSX / Node test (0.48s, 8/8 Passed) | ⚠️ Sampled unit testing only | 1.07x Consumption |

### 🔬 Why Does Axiom's Hole Pipeline Outperform Traditional Languages?
1. **$O(1)$ Micro-Context Isolation Eliminates Hallucination & Token Waste**:
   In traditional languages, agents must repeatedly read and re-write 40~50 functions in monolithic files, causing context explosion. Axiom's compiler-native `axiom extract-holes` mechanically partitions 200 functions into independent `hole-task.json` files (~160 Tokens each). Micro-workers only output a single pure expression (~18 Tokens), consuming just **35,627 Tokens** across all 200 function implementations!
2. **0.08-Second SMT Mathematical Theorem Proving**:
   Traditional unit tests only sample arbitrary inputs and miss edge-case overflows. Axiom's built-in Z3 SMT solver proves all Pre/Post contracts across $(-\infty \sim +\infty)$ in **0.08 seconds**, guaranteeing total absence of edge-case bugs.

> 📖 For full cross-language reports, per-module token breakdowns, and verified source code, see: [**200-Functions Multi-Agent Benchmark (examples/benchmark_200/README.md)**](examples/benchmark_200/README.md).

---

## 📂 Documentation Navigator

- 📘 [Architecture Whitepaper (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [Language Syntax & Semantics Specification (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [Multi-Agent Workflow & Content-Addressed AST Storage (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🔬 [In-Depth Language Comparison & Boundary Research (docs/RESEARCH_LANGUAGE_COMPARISON.md)](docs/RESEARCH_LANGUAGE_COMPARISON.md)
- 🗺️ [Project Development Roadmap (docs/ROADMAP.md)](docs/ROADMAP.md)
- 💡 [Example 1: Bank Transfer Service (examples/transfer.ax)](examples/transfer.ax)
- 🎮 [Example 2: Cross-Platform Game Loop (examples/game_loop.ax)](examples/game_loop.ax)

---

## 🛠️ Syntax Glance

```rust
// 1. Immutable Algebraic Data Type (ADT)
type Player = {
    id: Int,
    name: String,
    score: Int,
}

// 2. Explicit Algebraic Effects
effect Audio {
    fn play_sound(sound_id: String): ()
}

effect Network {
    fn send_packet(packet: String): ()
}

// 3. Contract-guaranteed function skeleton with ?hole
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
    // Filled concurrently by Worker Agents
    ?level_up_implementation
}
```

---

## 🚀 Installation & Download Prebuilt Binaries

You do not need to install a Rust compilation toolchain. Precompiled binaries and skills are available directly from GitHub Releases:

### 1. Download Release Packages
Visit the GitHub [**Releases**](../../releases) page and download the archive for your operating system (**every archive includes both the compiled binary and the complete `skills/` suite**):
- **Windows (x64)**: `axiom-v*-x86_64-pc-windows-msvc.zip` (contains `axiom.exe` & `skills/`)
- **Linux (x64)**: `axiom-v*-x86_64-unknown-linux-gnu.tar.gz` (contains `axiom` & `skills/`)
- **macOS (Apple Silicon)**: `axiom-v*-aarch64-apple-darwin.tar.gz`
- **macOS (Intel)**: `axiom-v*-x86_64-apple-darwin.tar.gz`
- **Standalone Skills**: `axiom-skills-v*.zip` (Pure Agent Skills package)

### 2. Configure Environment Variables (Windows)
1. Extract the downloaded zip archive to any permanent directory (e.g. `C:\Program Files\Axiom\` or `D:\Tools\axiom\`);
2. Add that directory path to your system's **`PATH` environment variable**;
3. Open a new terminal (PowerShell or CMD) and run `axiom --help` to verify the installation.

> 💡 **Developer Source Build (Optional)**:  
> If you have a local Rust toolchain, you can install directly from source: `cargo install --path crates/axiom-cli`

---

## 🤖 AI Agent Skills Installation & Activation Guide

Axiom provides 5 specialized collaboration skills designed for modern AI coding agents:
- `/axiom-pipeline`: Fully automated hole-driven pipeline (Architect -> Extract Holes -> Worker Fill -> SMT Verify -> Healer)
- `/axiom-interactive`: Phased human-in-the-loop pipeline (Contracts generated first for human review, followed by fast filling)
- `axiom-architect` / `axiom-worker` / `axiom-healer`: Specialized sub-agent operational specifications

### Setup Instructions for Mainstream Agent Clients:

| AI Agent Client | Installation Path / Configuration | Trigger Command |
| :--- | :--- | :--- |
| **🌟 Google Antigravity / Gemini CLI** | Copy extracted `skills/*` into `%USERPROFILE%\.gemini\config\skills\` (global) or `.gemini/config/skills/` (project) | Type in chat:<br>`/axiom-pipeline Implement...` |
| **⚡ Cursor IDE** | Copy `skills/axiom-pipeline/SKILL.md` to `.cursor/rules/axiom-pipeline.mdc` (or append to `.cursorrules`) | Type in Composer / Agent mode:<br>`Follow axiom-pipeline specification to implement...` |
| **🌊 Windsurf (Codeium)** | Append `skills/axiom-pipeline/SKILL.md` contents to project `.windsurfrules` | Type in Cascade chat:<br>`Follow Axiom hole pipeline to implement...` |
| **🧠 Claude Code / Claude Desktop** | Append skill contents to project `CLAUDE.md`, or upload markdown files in `skills/` to Claude.ai Project Knowledge | Type in chat:<br>`Activate /axiom-pipeline to implement...` |
| **🤖 OpenAI Codex CLI / Aider** | Place `skills/` in the workspace or reference `skills/axiom-pipeline/SKILL.md` in system prompts | Specify in prompt:<br>`Develop according to skills/axiom-pipeline spec` |

> 📖 For comprehensive skill documentation, see: [**Axiom Skills Package Guide (skills/README.md)**](skills/README.md).

---

## 💻 CLI Toolchain & `axiom.exe` Usage Guide

Once added to `PATH`, run `axiom` (or `axiom.exe` on Windows) directly from your terminal:

```powershell
# 1. Syntax check and hole audit
axiom check examples/transfer.ax

# 2. Static SMT formal contract verification (human console output)
axiom verify examples/transfer.ax

# 3. Static SMT formal verification (machine-readable JSON output for agent self-healing)
axiom verify examples/transfer.ax --json

# 4. Extract structured hole tasks for agent dispatch
axiom extract-holes examples/transfer.ax
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 5. Content-addressed AST codebase (ASTDB) & virtual source projection
axiom db put examples/transfer.ax
axiom db export #mod_589e44855c2b4e0223acf6867cdd3c6f0947fb3a23d7afafbb62e7fbc044559a

# 6. WebAssembly Component Model (WIT) export
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit

# 7. Multi-language FFI glue and type declarations (TypeScript .d.ts / C header .h / Rust C-ABI)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-ffi examples/game_loop.ax --lang rust -o game_loop_c_abi.rs

# 8. Cross-platform project scaffolding (supports web / native / node)
axiom scaffold my_game --target web --out-dir ./projects

# 9. Transpile to Rust and run
axiom emit-rust examples/game_loop.ax -o game_loop.rs
axiom run path/to/program_with_main.ax
```

> ⚙️ **Source Debugging Mode**: If you haven't installed the global binary, you can replace `axiom <subcommand>` with `cargo run -p axiom-cli -- <subcommand>` from the workspace root.
