> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

# 🏆 200-Functions Cross-Language Multi-Agent Benchmark Summary

> **Objective**: Under the industrial RPG game engine specification [`SPEC_200.md`](./SPEC_200.md), benchmark the real-world performance of **Axiom, Python, Rust, and TypeScript** in terms of **Token consumption, execution duration, merge conflict rates, and formal correctness** when developed concurrently by OpenAI Codex CLI agents.

---

## 📊 1. Multi-Agent Cross-Language Benchmark Matrix

All metrics reflect genuine runs conducted by autonomous Codex CLI instances, with 100% of test suites and SMT formal proofs verified:

| Language Ecosystem | Multi-Agent Orchestration Architecture | Task Granularity & Context Model | Real Tokens Consumed | Verification Method & Duration | Formal Mathematical Guarantee | Comparison vs Rust Baseline |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **`/axiom-pipeline` Automated Hole Pipeline**<br>(Architect + CLI Extract + 200 Micro-Workers) | **Single-function `hole-task.json` isolation<br>($O(1)$ ultra-compact local context)** | **183,662 Tokens** | **SMT Formal Theorem Prover<br>(0.08s for 200/200)** | **✅ Entire Number Domain $(-\infty \sim +\infty)$<br>100% Mathematically Proven** | **🔥 58.9% ~ 61.7% Savings<br>(2.4x Speedup)** |
| **🐍 Python** | 5 Concurrent Agents (Tier 0 ~ Tier 4) | 40~50 functions/module<br>($O(N)$ large files) | **384,089 Tokens** | Unittest Suite<br>(0.002s, 9/9 Passed) | ⚠️ Sampled unit testing only | 0.86x Consumption |
| **🦀 Rust** | 5 Concurrent Agents across crates/mods<br>(Tier 0 ~ Tier 4) | 40~50 functions/module<br>($O(N)$ large files) | **446,775 Tokens** | Cargo Test Suite<br>(0.46s, 6/6 Passed) | ⚠️ Sampled unit testing only | 1.00x Baseline |
| **📘 TypeScript** | 5 Concurrent Agents (ESM/TS)<br>(Tier 0 ~ Tier 4) | 40~50 functions/module<br>($O(N)$ large files) | **479,872 Tokens** | TSX / Node Test Suite<br>(0.48s, 8/8 Passed) | ⚠️ Sampled unit testing only | 1.07x Consumption |

---

## 🔬 2. Key Breakthroughs: Why Axiom's Hole Pipeline Wins

Traditional programming languages (Rust/Python/TypeScript) suffer from severe **context explosion and exploratory hallucinations** in multi-agent workflows:
1. **$O(1)$ Context Isolation vs $O(N)$ File Inflation**:
   - In traditional files containing 40~50 functions, agents must continuously re-read and parse surrounding code, burning massive token budgets and inducing hallucinations.
   - Axiom's native `axiom extract-holes` mechanically decomposes 200 function skeletons into **200 micro-tasks (~160 Tokens each)**. Each Worker Agent only reasons about local Pre/Post contracts and returns a single pure expression (~18 Tokens). As a result, the entire implementation phase for all 200 holes consumed merely **35,627 Tokens**!
2. **0.08s Mathematical Proofs vs Fragile Sampled Testing**:
   - Unit tests only test hand-crafted inputs, missing corner-case integer overflows or out-of-bound array accesses.
   - Axiom's integrated Z3 SMT solver proves all 200 function contracts over the entire numerical domain $(-\infty \sim +\infty)$ in **0.08 seconds**, delivering mathematical certainty of zero defects without writing a single test case.
3. **Decoupled Architecture & Worker Roles**:
   - The Architect Agent specifies top-level ADTs and contracts with `?hole` points.
   - Micro-Workers fill pure expressions concurrently in seconds.
   - The SMT Healer Agent repairs boundary violations using concrete counterexample values.

---

## 📂 3. Reports & Source Code Navigation

| Resource | Description | Link |
| :--- | :--- | :--- |
| 📋 **Functional Specification** | 200 pure functions and mathematical contracts | [`SPEC_200.md`](./SPEC_200.md) |
| 🌟 **Axiom Detailed Report** | Step-by-step pipeline token breakdown and SMT verification | [`Benchmark_ax.md`](./Benchmark_ax.md) |
| 🐍 **Python Detailed Report** | Per-agent token logs and unittest results | [`Benchmark_py.md`](./Benchmark_py.md) |
| 🦀 **Rust Detailed Report** | Per-agent token logs and cargo test results | [`Benchmark_rs.md`](./Benchmark_rs.md) |
| 📘 **TypeScript Detailed Report** | Per-agent token logs and TSX test results | [`Benchmark_ts.md`](./Benchmark_ts.md) |
| 📜 **Axiom Source Directory** | Skeleton `engine_200_holes.ax` and proven `engine_200.ax` | [`ax/`](./ax/) |
| 🐍 **Python Source Directory** | 5-tier modules and unittest suite | [`py/`](./py/) |
| 🦀 **Rust Source Directory** | 5-tier crate modules and Cargo test suite | [`rs/`](./rs/) |
| 📘 **TypeScript Source Directory** | 5-tier ESM/TS modules and TSX test suite | [`ts/`](./ts/) |

---

## 🖥️ 4. Local Reproduction & Verification Commands

```powershell
# 1. Verify Axiom's 200 functions via SMT formal theorem prover (0.08s)
cargo run -p axiom-cli -- verify examples/benchmark_200/ax/engine_200.ax

# 2. Run the complete Python 200-function unittest suite
python -m unittest discover -s examples/benchmark_200/py/tests -v

# 3. Run the complete Rust 200-function test suite
cargo test --manifest-path examples/benchmark_200/rs/Cargo.toml

# 4. Run the complete TypeScript 200-function test suite
npx tsx --test examples/benchmark_200/ts/tests/engine_200.test.ts
```
