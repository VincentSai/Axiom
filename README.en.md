# Axiom: AI-Native Verified Logic Layer & Runtime

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** is a restricted pure-function DSL and formal verification pipeline prototype designed specifically for **AI Agent Collaboration**.  
> **Core Strategic Positioning**: **Not a replacement for general-purpose languages (Rust / C# / TypeScript)**, but in deep symbiosis with them. Axiom focuses exclusively on the **core deterministic logic layer** where AI generation benefits most from constrained search spaces and mathematical contract defenses (combat pipelines, state transitions, rule evaluations, permission checks), while seamlessly transpiling and embedding into Rust, Unity C#, WebAssembly, and C.  
> **Core Value Proposition**: Constraining LLM generation spaces via pure expressions and strong contracts; eliminating long-context token waste via hole-driven localization; and enabling closed-loop self-healing via SMT counterexample feedback.  
> **Objective Verification Boundary**: Formal theorem proving guarantees only that code satisfies specified formal contracts, not that contracts perfectly mirror human business intent. Axiom is an evolving engineering prototype that stands alongside mature verification frameworks (e.g. Dafny, F*, Kani), exploring lightweight, LLM-specialized verification pipelines.

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
│  • Pure Functions (Restricted Pure Expressions)                 │
│  • Algebraic Effects (Explicit Side-Effect Isolation)           │
│  • SMT Formal Contracts (Theorem Proving & Counterexamples)     │
│  • Hole-Driven Micro-Workers & Task Pools                       │
└─────────────────────────────────────────────────────────────────┘
```

When building game logic and mission-critical systems with AI agents, traditional general-purpose languages face major hurdles:
- **Runaway State & Hidden Regressions**: Implicit global state and unconstrained side effects cause AI-generated code to harbor latent boundary bugs and behavioral drift.
- **Context Bloat & Attention Degradation**: Forcing LLM agents to repeatedly read and edit massive monolithic files wastes tokens and invites hallucinations.
- **Testing Limits vs Mathematical Proofs**: Hand-written unit tests can only sample inputs, whereas constrained functions checked by SMT can be systematically searched for counterexamples.

**Axiom** addresses these challenges with a streamlined pipeline:
1. **Pure Functions + Algebraic Effects**: Core logic is expressed as pure immutable state transformations; host IO and side effects are declared explicitly via Effects, with the transpiler generating native Rust traits.
2. **SMT-LIB2 / Microsoft Z3 Theorem Proving**: Built-in `requires` / `ensures` contracts are compiled to standard SMT-LIB2 scripts to prove properties or extract counterexamples; a lightweight local interval engine also provides discrete grid validation.
3. **Hole-Driven Development**: Declare unimplemented points with `?hole`. The compiler mechanically slices the AST into isolated task units, allowing micro-workers to operate in minimal local contexts.
4. **Multi-Agent Task Pool Prototype (`axiom pool`)**: In-memory task coordination prototype for managing candidate submissions, automated SMT counterexample filtering, and verified code assembly.
5. **Unity Zero-Copy Double-Buffer Bridge (`axiom bridge`)**: Transpile to Rust and compile to native C-ABI libraries with automatic C# P/Invoke bindings and contiguous memory snapshots.

---

## 🔬 Empirical Scientific Evaluation: Single-Variable A/B Eval Suite

A common question is: *“Are Axiom’s token savings and reliability gains genuinely from the language representation, or simply from breaking tasks down into micro-workers?”*

To scientifically decouple these variables, we established a rigorous **Single-Variable A/B Benchmark Eval Suite** located in [`examples/benchmark_eval/`](examples/benchmark_eval/):
- **Golden Task Suite ([`golden_tasks.json`](examples/benchmark_eval/golden_tasks.json))**: 20 golden domain algorithm tasks spanning spatial geometry, non-linear resistance, combat mechanics, loot drop state machines, and world simulation.
- **Three-Group Controlled Protocol ([`prompt_templates.md`](examples/benchmark_eval/protocols/prompt_templates.md))**:
  1. **Group 1**: Traditional monolithic file workflow (Rust Monolith, large context);
  2. **Group 2 (Critical Decoupling)**: Traditional single-function micro-tasks (Rust Micro-Worker, eliminating task granularity differences);
  3. **Group 3**: Axiom contract hole pipeline (Axiom Hole Pipeline, powered by formal contracts and SMT verification).
- **Official API Runner ([`eval_runner.py`](examples/benchmark_eval/eval_runner.py))**:
  Powered directly by the official Google Gemini REST API (`countTokens` and `generateContent`), **measuring genuine physical tokens via the official tokenizer and usageMetadata**, strictly eliminating subjective or unverified claims. Anyone can run the reproduction suite with `GEMINI_API_KEY`.

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

- 🔬 [Single-Variable A/B Evaluation Suite (examples/benchmark_eval/README.md)](examples/benchmark_eval/README.md)
- 📊 [200-Function Stress Benchmark (examples/benchmark_200/Benchmark_ax.md)](examples/benchmark_200/Benchmark_ax.md)
- 📘 [Architecture Whitepaper (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [Language Specification Draft (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [Multi-Agent Workflow & AST Storage (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🗺️ [Project Roadmap (docs/ROADMAP.md)](docs/ROADMAP.md)
- 👾 [Axiom Survivors Unity Project (examples/game_survivor/)](examples/game_survivor/README.md)
