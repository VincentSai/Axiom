> **Languages / 语言 / 語言**: [English](../en/ROADMAP.md) | [简体中文](../zh-CN/ROADMAP.md) | [繁體中文](../zh-TW/ROADMAP.md)

---

# Axiom Project Roadmap

---

## Phase 1: MVP Prototype & Core Compiler (Q1)
- [x] Complete language architecture specification and `/grill-me` stress testing
- [x] Implement Rust-based lexical scanner, parser, and AST data structures (`axiom-ast`, `axiom-parser`)
- [x] Implement hole mechanism (`?hole`) and structured JSON task dispatcher (`axiom-hole-extractor`)
- [x] Implement Axiom-to-Rust transpiler (`axiom-transpiler`)
- [x] Build CLI toolchain with end-to-end integration tests (`axiom-cli`, 35/35 tests passing)

## Phase 2: Effect System & Formal Contract Verification (Q2)
- [x] Implement compile-time algebraic effect boundaries and dispatch (`axiom-effects`)
- [x] Integrate lightweight pure-Rust SMT/symbolic solver for static counterexample generation (`axiom-smt`)
- [x] Implement machine-readable diagnostic reporting protocol (`axiom-diagnostics`)
- [x] Deliver `axiom verify` CLI with automated agent healing loop (`axiom-cli`, 52/52 tests passing)

## Phase 3: Content-Addressed AST Codebase & Multi-Agent Concurrency (Q3)
- [x] Implement normalized AST hashing and content-addressed storage engine (`axiom-db`)
- [x] Develop virtual code projection and bidirectional sync (`axiom-vfs`)
- [x] Implement lock-free concurrent hole dispatch and agent worker pool (`axiom-agent-pool`)
- [x] Deliver `axiom db` CLI with 20+ hole concurrent multi-agent verification (`axiom-cli`, 65/65 tests passing)

## Phase 4: Cross-Platform Runtimes & Ecosystem Interop (Q4)
- [x] Integrate WebAssembly Component Model (WIT) bidirectional export (`axiom-wit`)
- [x] Generate declarative C / TypeScript / Rust FFI bindings (`axiom-ffi-gen`)
- [x] Deliver full cross-platform project scaffolding (`axiom-cli`, 76/76 tests passing)
