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

## 🚀 One-Liner Quick Install

Install prebuilt `axiom` CLI binaries directly:

- **Windows (PowerShell)**:
  ```powershell
  irm https://raw.githubusercontent.com/axiom-lang/axiom/main/scripts/install.ps1 | iex
  ```
- **Linux & macOS (Bash)**:
  ```bash
  curl -fsSL https://raw.githubusercontent.com/axiom-lang/axiom/main/scripts/install.sh | bash
  ```

---

## 🎨 Official VS Code Extension (IDE Support)

Located in [`editors/vscode-axiom`](editors/vscode-axiom):
- **Syntax Highlighting**: TextMate grammar for Axiom keywords, types, effects, and patterns.
- **Verify on Save**: Automatically runs `axiom check` / `verify` on save, surfacing parse errors, type mismatches, or SMT counterexamples in the Problems panel.
- **Snippets**: Scaffolds for `fn-contract`, `type-record`, `type-enum`, and more.

---

## 💻 CLI Usage Guide

```powershell
# 1. Syntax, static type checking, and hole count
axiom check examples/transfer.ax

# 2. Formal contract verification (auto-probes Z3, falls back to symbolic engine)
axiom verify examples/game_survivor/ax/survivor_core.ax

# 3. Export standard SMT-LIB2 (.smt2) for third-party solvers (Z3/CVC5)
axiom export-smt2 examples/transfer.ax -o transfer.smt2

# 4. Extract hole tasks as JSON
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 5. Unity bridge code generation
axiom bridge examples/game_survivor/ax/survivor_core.ax --target unity --out examples/game_survivor/Assets/Scripts/AxiomBridge --library-name axiom_survivor

# 6. Transpile and execute via Rust/rustc
axiom emit-rust examples/game_loop.ax -o game_loop.rs
axiom run path/to/program_with_main.ax
```

---

## 📂 Documentation

- 📘 [Architecture Whitepaper (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [Language Specification Draft (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [Multi-Agent Workflow (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 👾 [Axiom Survivors Unity Showcase (examples/game_survivor/)](examples/game_survivor/README.md)
