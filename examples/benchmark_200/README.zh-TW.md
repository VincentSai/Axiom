> **Languages / 語言 / 语言**: [English](README.en.md) | [繁體中文](README.zh-TW.md) | [簡體中文](README.zh-CN.md)

---

# 📊 200-Functions 跨語言多 Agent 協作對比實驗

> **實驗目標**：依據 RPG 業務需求規範 [`SPEC_200.md`](./SPEC_200.md)，橫向對比在 AI 大模型（OpenAI Codex CLI）多 Agent 並發協作開發 200 個業務函數時，**Axiom、Python、Rust 與 TypeScript** 四大語言體系在 **Tokens 消耗、上下文隔離與驗證反饋** 維度的實際表現。

---

## 📊 一、四大語言多 Agent 協作實測對比

數據由 Codex CLI 實例並發協作產出，對比在不同程式碼組織結構下的上下文開銷與驗證機制：

| 語言體系 | 協作模式與任務拆解 | 上下文模型與粒度 | 消耗 Tokens | 驗證機制與狀態 | 相對消耗對比 |
| :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **打洞管線 (`axiom-pipeline`)**<br>(Architect 骨架 + 200 個填洞任務) | **單函數 `hole-task.json` 隔離<br>(局部上下文聚焦單個簽名)** | **183,662 Tokens** | **SMT/Z3 形式化證明 + 網格採樣檢驗**<br>(支援前置/後置契約求解，Fail-Closed) | **🔥 節省約 52% ~ 62%<br>Tokens 消耗** |
| **🐍 Python** | 5 個 Agent 模組化並發<br>(Tier 0 ~ Tier 4) | 40~50 函數/大檔案<br>(單 Agent 上下文較龐大) | **384,089 Tokens** | Unittest 單元測試套件 | 0.86x 消耗基準 |
| **🦀 Rust** | 5 個 Agent 跨 crate/mod 並發<br>(Tier 0 ~ Tier 4) | 40~50 函數/大模組<br>(單 Agent 上下文較龐大) | **446,775 Tokens** | Cargo test 單元測試套件 | 1.00x 參照基準 |
| **📘 TypeScript** | 5 個 Agent 模組化並發 (ESM/TS)<br>(Tier 0 ~ Tier 4) | 40~50 函數/大檔案<br>(單 Agent 上下文較龐大) | **479,872 Tokens** | TSX / Node test 單元測試套件 | 1.07x 消耗基準 |

---

## 🔬 二、核心設計探索：Axiom 任務拆解與驗證機制

在多 Agent 協作開發長邏輯時，傳統語言普遍面臨「上下文冗餘與跨函數幻覺」的挑戰：
1. **局部上下文隔離 vs 大檔案長上下文**：
   - 傳統語言由於單檔案通常包含 40~50 個函數，Agent 在閱讀和修改時需要加載大量非相關程式碼上下文，Token 開銷隨檔案尺寸線性膨脹；
   - Axiom 通過 `axiom extract-holes`，將包含洞（`?hole`）的骨架模組機械切分為 **單函數級別的獨立任務檔案（`hole-task.json`）**。每個 Worker Agent 僅需閱讀當前函數的入參型別與 Pre/Post 契約，輸出單個純表達式，有效降低了實現階段的 Token 消耗。
2. **契約驗證機制（SMT 證明與採樣檢驗）**：
   - 傳統語言依靠開發者手寫單元測試進行有限抽樣；
   - Axiom 支援在函數簽名中宣告 `requires` 與 `ensures` 契約。驗證層優先調用外部 SMT 求解器（如 Z3）生成形式化證明（`Proved`）；若無 Z3 環境，則回退為確定性離散網格採樣（`Sampled`），並在遇到複雜代數效應或不支援表達式時明確標記為 `Skipped`，確保診斷結果客觀真實（Fail-Closed）。
3. **架構師與填洞工人角色分離**：
   - 架構師 Agent (Architect) 專注於全域領域建模、型別定義與契約骨架；
   - 填洞工人 (Micro-Workers) 並發生成無副作用的純函數體；
   - 診斷系統捕獲反例並輔助單步自癒。

---

## 📂 三、詳細報告與各語言原始碼導航

| 導航項目 | 描述說明 | 連結地址 |
| :--- | :--- | :--- |
| 📋 **業務規範設計書** | 200 個業務函數分層與數學邏輯設計 | [`SPEC_200.md`](./SPEC_200.md) |
| 🌟 **Axiom 評測記錄** | 打洞管線分步 Token 消耗與驗證記錄 | [`Benchmark_ax.md`](./Benchmark_ax.md) |
| 🐍 **Python 評測記錄** | 5 個 Agent 消耗、Unittest 用例與結果 | [`Benchmark_py.md`](./Benchmark_py.md) |
| 🦀 **Rust 評測記錄** | 5 個 Agent 消耗、Cargo test 套件與結果 | [`Benchmark_rs.md`](./Benchmark_rs.md) |
| 📘 **TypeScript 評測記錄** | 5 個 Agent 消耗、TSX 測試套件與結果 | [`Benchmark_ts.md`](./Benchmark_ts.md) |
| 📜 **Axiom 原始碼目錄** | 骨架 `engine_200_holes.ax` 與填洞後程式碼 `engine_200.ax` | [`ax/`](./ax/) |
| 🐍 **Python 原始碼目錄** | 5 層模組與單元測試套件 | [`py/`](./py/) |
| 🦀 **Rust 原始碼目錄** | 5 層模組與 Cargo 單元測試套件 | [`rs/`](./rs/) |
| 📘 **TypeScript 原始碼目錄** | 5 層模組與 Node/TSX 測試套件 | [`ts/`](./ts/) |

---

## 🖥️ 四、本機驗證命令指南

```powershell
# 1. 執行 Axiom 契約與靜態驗證
cargo run -p axiom-cli -- verify examples/benchmark_200/ax/engine_200.ax

# 2. 執行 Python 完整單元測試套件
python -m unittest discover -s examples/benchmark_200/py/tests -v

# 3. 執行 Rust 完整測試套件
cargo test --manifest-path examples/benchmark_200/rs/Cargo.toml

# 4. 執行 TypeScript 完整測試套件
npx tsx --test examples/benchmark_200/ts/tests/engine_200.test.ts
```
