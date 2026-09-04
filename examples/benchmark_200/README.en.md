> **Languages**: [English](README.en.md) | [Simplified Chinese](README.zh-CN.md) | [Traditional Chinese](README.zh-TW.md)

---

# 📊 200-Functions Cross-Language Multi-Agent Collaboration Benchmark

> **Objective**: Compare the performance of **Axiom, Python, Rust, and TypeScript** in terms of **token consumption, context isolation, and verification feedback** during multi-agent concurrent development of 200 domain functions, following [`SPEC_200.md`](./SPEC_200.md).

---

## 📊 1. Multi-Agent Benchmark Comparison

Data was collected from concurrent OpenAI Codex CLI instances across different language paradigms:

| Language | Paradigm & Task Decomposition | Context Granularity | Token Usage | Verification Feedback | Relative Usage |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **Hole Pipeline (`axiom-pipeline`)**<br>(Architect Skeleton + 200 Sub-Tasks) | **Single-function `hole-task.json`<br>(Local context focused on signature)** | **183,662 Tokens** | **SMT/Z3 Prover + Grid Sampling**<br>(Pre/Post contracts, Fail-Closed) | **🔥 ~52% ~ 62% Savings<br>in Tokens** |
| **🐍 Python** | 5 Concurrent Agents<br>(Tier 0 ~ Tier 4) | 40~50 functions / large file<br>(Large per-agent context) | **384,089 Tokens** | Unittest Suite | 0.86x baseline |
| **🦀 Rust** | 5 Concurrent Agents across mods<br>(Tier 0 ~ Tier 4) | 40~50 functions / crate mod<br>(Large per-agent context) | **446,775 Tokens** | Cargo test Suite | 1.00x baseline |
| **📘 TypeScript** | 5 Concurrent Agents (ESM/TS)<br>(Tier 0 ~ Tier 4) | 40~50 functions / large file<br>(Large per-agent context) | **479,872 Tokens** | TSX / Node test Suite | 1.07x baseline |

---

## 🔬 2. Design Insights: Task Decomposition & Verification

1. **Local Context Isolation vs Monolithic Files**:
   - Monolithic files (40~50 functions) require large prompts for each agent pass, compounding token costs and hallucination risks.
   - Axiom extracts each `?hole` into an isolated `hole-task.json`. Micro-workers focus solely on the local signature and contracts, keeping the token footprint minimal.
2. **Verification Mechanism (SMT Theorem Proving & Sampling)**:
   - Traditional workflows rely on sampled unit test assertions.
   - Axiom supports `requires` and `ensures` contracts. If external Z3 is installed, contracts are formally proven (`Proved`). Without Z3, deterministic grid sampling provides heuristic coverage (`Sampled`), while unsupported constructs or unresolved holes are safely marked `Skipped` (Fail-Closed).
3. **Role Separation**:
   - Architect Agent focuses on top-level models, effect signatures, and contract skeletons.
   - Micro-Worker Agents complete side-effect-free pure expressions.
   - Diagnostics report actionable counterexamples for iterative self-healing.

---

## 📂 3. Navigation & Source Code

| Item | Description | Link |
| :--- | :--- | :--- |
| 📋 **Specification** | Domain requirements for 200 functions | [`SPEC_200.md`](./SPEC_200.md) |
| 🌟 **Axiom Log** | Token breakdowns and verification steps | [`Benchmark_ax.md`](./Benchmark_ax.md) |
| 🐍 **Python Log** | 5-agent execution logs and unittests | [`Benchmark_py.md`](./Benchmark_py.md) |
| 🦀 **Rust Log** | 5-agent execution logs and Cargo tests | [`Benchmark_rs.md`](./Benchmark_rs.md) |
| 📘 **TypeScript Log** | 5-agent execution logs and TSX tests | [`Benchmark_ts.md`](./Benchmark_ts.md) |

---

## 🖥️ 4. Reproduction Commands

```powershell
# 1. Run Axiom static & contract verification
cargo run -p axiom-cli -- verify examples/benchmark_200/ax/engine_200.ax

# 2. Run Python unittest suite
python -m unittest discover -s examples/benchmark_200/py/tests -v

# 3. Run Rust test suite
cargo test --manifest-path examples/benchmark_200/rs/Cargo.toml

# 4. Run TypeScript test suite
npx tsx --test examples/benchmark_200/ts/tests/engine_200.test.ts
```
