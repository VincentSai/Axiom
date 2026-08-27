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

## 💻 CLI Toolchain Guide

Run from workspace root (or install `axiom` binary):

```powershell
# 1. Syntax check and hole audit
cargo run -p axiom-cli -- check examples/transfer.ax

# 2. Static SMT formal contract verification (human console & agent machine JSON modes)
cargo run -p axiom-cli -- verify examples/transfer.ax
cargo run -p axiom-cli -- verify examples/transfer.ax --json

# 3. Extract structured hole tasks for agent dispatch
cargo run -p axiom-cli -- extract-holes examples/transfer.ax
cargo run -p axiom-cli -- extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 4. Content-addressed AST codebase (ASTDB) & virtual source projection
cargo run -p axiom-cli -- db put examples/transfer.ax
cargo run -p axiom-cli -- db export #mod_589e44855c2b4e0223acf6867cdd3c6f0947fb3a23d7afafbb62e7fbc044559a

# 5. WebAssembly Component Model (WIT) export
cargo run -p axiom-cli -- export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit

# 6. Multi-language FFI glue and type declarations (TypeScript .d.ts / C header .h / Rust C-ABI)
cargo run -p axiom-cli -- export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
cargo run -p axiom-cli -- export-ffi examples/game_loop.ax --lang c -o game_loop.h
cargo run -p axiom-cli -- export-ffi examples/game_loop.ax --lang rust -o game_loop_c_abi.rs

# 7. Cross-platform project scaffolding
cargo run -p axiom-cli -- scaffold my_game --target web --out-dir ./projects

# 8. Transpile to Rust and run
cargo run -p axiom-cli -- emit-rust examples/game_loop.ax -o game_loop.rs
cargo run -p axiom-cli -- run path/to/program_with_main.ax
```

Axiom's toolchain comprehensively spans parsing, algebraic effect isolation, SMT counterexample self-healing, content-addressed storage, lock-free multi-agent concurrency, and multi-platform compilation.
