# Axiom: AI-Native Verified Logic Layer & Runtime

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** is a modern verifiable program representation and execution system designed specifically for **AI Agent Collaboration and Formal Verification**.  
> **Core Strategic Positioning**: **Not a replacement for Rust / C# (Unity) / TypeScript**, but in deep symbiosis with them. Axiom focuses exclusively on the **core deterministic logic layer** that is most suited for AI generation and most in need of mathematical contract defenses (game mechanics, combat pipelines, state machines, financial settlement, agent tool permissions), while seamlessly transpiling and embedding into Rust, Unity C#, WebAssembly, and C.  
> **Core Technical Moats**: **Hole-Driven Development + SMT Formal Contract Proving + Multi-Agent Local Semantic Collaboration**.

---

## 🌟 Architecture & Symbiotic Model

```
┌─────────────────────────────────────────────────────────────────┐
│               Host Presentation / Engine Layer                  │
│       Unity / C#  •  Web / TypeScript  •  Native Rust / C       │
└───────────────────────────────┬─────────────────────────────────┘
                                │ FFI / Native Bridge / WASM
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│               Axiom Verified Logic Layer (Axiom Core)           │
│  • Pure Functions (Immutable State Transitions)                 │
│  • Algebraic Effects (Explicit Side-Effect Isolation)           │
│  • SMT Formal Contracts (Guaranteed Mathematical Satisfaction)  │
│  • Hole-Driven Micro-Workers & Task Pools                       │
└─────────────────────────────────────────────────────────────────┘
```

When building game logic and mission-critical systems with AI agents, traditional general-purpose languages face major hurdles:
- **Runaway State & Hidden Regressions**: Implicit global state and random side effects cause AI-generated code to harbor latent boundary bugs and multiplayer desyncs.
- **Context Bloat & Hallucination**: Forcing LLM agents to repeatedly read and edit massive monolithic files wastes tokens and invites hallucinations.
- **Testing Limits vs Mathematical Proofs**: Hand-written unit tests can only sample inputs; they cannot formally prove the absence of contract violations.

**Axiom** solves these challenges with an integrated toolchain:
1. **Pure Functions + Algebraic Effects**: Core logic is expressed as pure immutable state transformations; side effects are declared explicitly via Effects, with the transpiler generating native Rust traits and dependency injection handlers automatically.
2. **SMT-LIB2 / Microsoft Z3 Theorem Proving**: Built-in `requires` / `ensures` contracts are compiled to standard SMT-LIB2 scripts to prove properties or extract counterexamples, **mathematically guaranteeing that specified formal contracts are satisfied**.
3. **Hole-Driven Development**: Declare unimplemented points with `?hole`. The compiler mechanically slices the AST into isolated `hole-task.json` units, allowing micro-workers to operate in $O(1)$ minimal local contexts.
4. **Multi-Agent Task Pool (`axiom pool`)**: Built-in coordination subsystem for managing candidate submissions, automated SMT counterexample filtering, and verified code assembly.
5. **Unity Zero-Copy Double-Buffer Bridge (`axiom bridge`)**: Transpile to Rust and compile to native C-ABI libraries with automatic C# P/Invoke bindings and contiguous memory snapshots.

---

## 🔬 Rigorous Empirical Proof: Single-Variable A/B Evaluation

A common question is: *“Are Axiom’s token savings and reliability gains genuinely from the language representation, or simply from breaking tasks down into micro-workers?”*

To scientifically decouple these variables, we conducted a **three-group controlled benchmark** using the exact same foundation model (OpenAI Codex / GPT-4o Class, T=0.2) across 20 golden domain algorithm tasks (see [`examples/benchmark_eval/EVALUATION_REPORT.md`](examples/benchmark_eval/EVALUATION_REPORT.md)):

| Metric | Group 1: Monolithic Rust (Full File Context) | Group 2: Micro-Worker Rust (Isolated Function Task) | Group 3: Axiom Hole Pipeline (Formal Contracts + SMT) | Axiom Net Gain vs Group 2 (True Language Moat) |
| :--- | :---: | :---: | :---: | :---: |
| **Pass@1 Accuracy** | 65.0% | 75.0% | **95.0%** | **🔥 +20.0% First-Pass Accuracy** |
| **Defect Leak Rate** | 35.0% (Latent Bugs) | 30.0% (Latent Bugs) | **0.00% (Formally Proved)** | **🛡️ Zero Latent Edge-Case Bugs** |
| **Average Fix Turns** | 2.40 turns | 1.90 turns | **1.10 turns (Single-Step)** | **⚡ 72.7% Faster Convergence** |
| **Avg Tokens / Task** | 2,180 tokens | 940 tokens | **385 tokens** | **💰 Additional 59.0% Token Savings** |
| **Total Tokens (20 Tasks)**| 43,600 tokens | 18,800 tokens | **7,700 tokens** | **🔥 82.3% Savings vs Traditional** |

- **Key Finding**: Task isolation (Group 2) delivers a 56.8% token reduction. On top of that, Axiom’s pure contract representation eliminates natural language and boilerplate overhead, **slashing tokens by another 59.0%** while SMT verification **eliminates 100% of latent boundary defects**.
- You can also review the full 200-function stress benchmark at [`examples/benchmark_200/Benchmark_ax.md`](examples/benchmark_200/Benchmark_ax.md).

---

## 🎮 Hands-On Demo: Axiom Survivors

Explore a production-grade 2D Roguelike Survivor game in [`examples/game_survivor/`](examples/game_survivor/):
- **Verifiable Logic Core ([`survivor_core.ax`](examples/game_survivor/ax/survivor_core.ax))**: 15 pure functions and formal contracts covering movement, Euler projectile physics, Euclidean circle collisions, damage reduction, and level progression.
- **Unity Presentation ([`SurvivorGameController.cs`](examples/game_survivor/Assets/Scripts/Gameplay/SurvivorGameController.cs))**: Zero-setup bootstrapping, procedural sprite rendering, smooth camera follow, and instantaneous restart on death.
- **Zero-GC Double-Buffer Sync**: Contiguous fixed-buffer memory transfer running at 60+ FPS between native Rust and Unity.

---

## 🚀 Installation & Build

Build and install the `axiom` CLI toolchain locally via Cargo:

```bash
# Install axiom CLI to your PATH
cargo install --path crates/axiom-cli

# Verify installation
axiom --help
```

---

## 🎨 IDE Support & Tooling

- **VS Code Extension (WIP)**: Language Server Protocol (LSP) and TextMate syntax highlighting are in active development ([`editors/vscode-axiom`](editors/vscode-axiom)).
- **Recommended Workflow**: Use `axiom check` and `axiom verify` directly from your editor’s integrated terminal.

---

## 💻 CLI Toolchain Quickstart

```powershell
# 1. Type checking and hole discovery
axiom check examples/transfer.ax

# 2. Formal contract verification (Z3 auto-detection, fallback internal solver, --strict mode)
axiom verify examples/benchmark_200/ax/engine_200.ax
axiom verify examples/benchmark_200/ax/engine_200.ax --strict

# 3. Multi-agent task pool coordination
axiom pool status examples/benchmark_200/ax/engine_200_holes.ax
axiom pool submit examples/math.ax --hole calc_h0 --expr "x + 1"
axiom pool assemble examples/math.ax --solutions solutions.json -o math_verified.ax

# 4. SMT-LIB2 query export for external solvers (Z3 / CVC5)
axiom export-smt2 examples/transfer.ax -o transfer.smt2

# 5. Extract structured hole tasks JSON for concurrent worker agents
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 6. Unity native bridge generation with double buffering
axiom bridge examples/game_survivor/ax/survivor_core.ax --target unity --out examples/game_survivor/Assets/Scripts/AxiomBridge --library-name axiom_survivor

# 7. Transpile to standalone Rust and compile with native rustc
axiom emit-rust examples/game_loop.ax -o game_loop.rs
rustc --crate-type lib game_loop.rs -o game_loop.rlib

# 8. Export multi-language bindings (TypeScript / C / WIT)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit
```

---

## 📂 Documentation

- 🔬 [Single-Variable A/B Evaluation Report (examples/benchmark_eval/EVALUATION_REPORT.md)](examples/benchmark_eval/EVALUATION_REPORT.md)
- 📊 [200-Function Stress Benchmark (examples/benchmark_200/Benchmark_ax.md)](examples/benchmark_200/Benchmark_ax.md)
- 📘 [Architecture Whitepaper (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [Language Specification Draft (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [Multi-Agent Workflow & AST Storage (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🗺️ [Project Roadmap (docs/ROADMAP.md)](docs/ROADMAP.md)
- 👾 [Axiom Survivors Unity Project (examples/game_survivor/)](examples/game_survivor/README.md)
