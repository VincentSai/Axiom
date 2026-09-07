# Axiom: A Deterministic Game Logic DSL & Formal Verification Research Prototype

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** is a modern domain-specific language (DSL) research prototype exploring **AI Agent collaboration** and **cross-engine deterministic game logic**.  
> Key research directions: Eliminating side effects via pure functions, isolating Agent context via hole-driven development, verifying mathematical contracts via SMT-LIB2 / Microsoft Z3, and connecting to engines like Unity via zero-copy double buffering.

---

## 🌟 Core Concepts

In AI-assisted game logic development, conventional general-purpose languages introduce several challenges:
- **Implicit State & Non-Determinism**: Hidden mutable globals and side effects cause AI-generated code to suffer from subtle boundary bugs and multiplayer desyncs.
- **Context Bloat & Hallucination**: Requiring Agents to inspect and rewrite large codebases wastes prompt tokens and contaminates context.
- **Engine Glue Overhead**: Passing state between game engines often lacks zero-GC, zero-copy deterministic memory synchronization.

**Axiom** explores an integrated toolchain addressing these issues:
1. **Pure Functions + Explicit Algebraic Effects**: Game logic is expressed as side-effect-free state transitions, with effects explicitly declared and isolated.
2. **SMT-LIB2 / Microsoft Z3 Theorem Proving**: Built-in `requires` / `ensures` contracts can be translated to standard SMT-LIB2 scripts for first-order theorem proving and counterexample generation via Z3, backed by a built-in symbolic interval engine.
3. **Micro Hole-Driven Collaboration**: Architect agents define skeletons with `?hole`, and the compiler extracts standalone `hole-task.json` tasks for worker agents to fill in $O(1)$ isolated local contexts.
4. **Zero-Copy Double-Buffered Unity Bridge (`axiom bridge`)**: Transpiles to Rust to build native C-ABI libraries, generating Unity C# P/Invoke and continuous memory snapshot buffers.
5. **Multi-Target Bindings & Content-Addressed AST Prototype**: Generates TypeScript `.d.ts`, C `.h`, and WebAssembly WIT declarations, and explores SHA-256 AST module storage.

---

## 🎮 Playable Demo: Axiom Survivors

A complete 2D roguelite survivor game is provided in [`examples/game_survivor/`](examples/game_survivor/):
- **Formal Core ([`survivor_core.ax`](examples/game_survivor/ax/survivor_core.ax))**: 15 pure functions and mathematical contracts covering player movement, projectile physics, Euclidean circle collisions, armor damage reduction, and exp leveling.
- **Unity Presentation ([`SurvivorGameController.cs`](examples/game_survivor/Assets/Scripts/Gameplay/SurvivorGameController.cs))**: Zero-config auto-bootstrap (open SampleScene and click Play), procedural pixel sprites, smooth camera follow, and one-key full restart.
- **Zero-GC Double Buffering**: Continuous memory batching between native Rust runtime and Unity at 60FPS+.

---

## 🚀 Installation & Build

Compile and install the `axiom` CLI toolchain directly from source using Cargo:

```bash
# Install axiom CLI to local PATH from source
cargo install --path crates/axiom-cli

# Verify installation
axiom --help
```

---

## 🎨 IDE Support & Ecosystem (Planned)

- **VS Code Extension (In Progress)**: Language Server Protocol (LSP) and TextMate syntax highlighting are currently in development under [`editors/vscode-axiom`](editors/vscode-axiom).
- **Current Recommendation**: Use the `axiom check` and `axiom verify` CLI commands integrated into your editor's terminal workflow for instant static type and formal contract feedback.

---

## 💻 CLI Usage Guide

```powershell
# 1. Syntax, static type checking, and hole count
axiom check examples/transfer.ax

# 2. Formal contract verification (auto-probes Z3, falls back to symbolic engine; supports --strict)
axiom verify examples/benchmark_200/ax/engine_200.ax
axiom verify examples/benchmark_200/ax/engine_200.ax --strict

# 3. Multi-agent hole task pool coordination and verification (Agent Task Pool)
axiom pool status examples/benchmark_200/ax/engine_200_holes.ax
axiom pool submit examples/math.ax --hole calc_h0 --expr "x + 1"
axiom pool assemble examples/math.ax --solutions solutions.json -o math_verified.ax

# 4. Export standard SMT-LIB2 (.smt2) for third-party solvers (Z3/CVC5)
axiom export-smt2 examples/transfer.ax -o transfer.smt2

# 5. Extract hole tasks as JSON
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 6. Unity cross-target codegen & zero-copy double-buffer bridge
axiom bridge examples/game_survivor/ax/survivor_core.ax --target unity --out examples/game_survivor/Assets/Scripts/AxiomBridge --library-name axiom_survivor

# 7. Transpile to Rust and execute
axiom emit-rust examples/game_loop.ax -o game_loop.rs
axiom run path/to/program_with_main.ax

# 8. Export multi-language bindings (TypeScript / C / WIT)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit
```

---

## 📂 Documentation

- 📘 [Architecture Whitepaper (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [Language Specification Draft (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [Multi-Agent Workflow (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 👾 [Axiom Survivors Unity Showcase (examples/game_survivor/)](examples/game_survivor/README.md)
