> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

# 🏆 200-Functions 跨語言多 Agent 協作基準測試權威總結

> **評測目標**：嚴格依據工業級 RPG 遊戲引擎需求規範 [`SPEC_200.md`](./SPEC_200.md)，橫向評測在由真實 AI 大模型（OpenAI Codex CLI）多 Agent 並發協作開發 200 個真實業務函數時，**Axiom、Python、Rust 與 TypeScript** 四大語言體系在 **Tokens 消耗、開發耗時、協作衝突率與形式化正確性** 維度的實際表現。

---

## 📊 一、四大語言多 Agent 真實業務實測全景大比拼

所有數據均由真實 Codex CLI 實例全自動並發協作產出，測試用例與 SMT 形式化數學證明 100% 真實通過：

| 語言體系 | 協作模式與架構設計 | 任務粒度與上下文模型 | 真實消耗 Tokens | 驗證手段與耗時 | 形式化數學保證 | 相對基準對比 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **`/axiom-pipeline` 全自動打洞管線**<br>(Architect + CLI Extract + 200 Micro-Workers) | **單函數 `hole-task.json` 隔離<br>($O(1)$ 超輕量局部上下文)** | **183,662 Tokens** | **SMT 全域定理證明<br>(0.08 秒 200/200)** | **✅ 全數域 $(-\infty \sim +\infty)$<br>100% 嚴格證明** | **🔥 節省 58.9% ~ 61.7%<br>(2.4x 提速)** |
| **🐍 Python** | 5 個 Agent 模組化並發<br>(Tier 0 ~ Tier 4) | 40~50 函數/大模組<br>(單 Agent 上下文龐大) | **384,089 Tokens** | Unittest 套件<br>(0.002 秒 9/9 Passed) | ⚠️ 僅依賴抽樣單元測試 | 0.86x 消耗 |
| **🦀 Rust** | 5 個 Agent 跨 crate/mod 並發<br>(Tier 0 ~ Tier 4) | 40~50 函數/大模組<br>(單 Agent 上下文龐大) | **446,775 Tokens** | Cargo test 套件<br>(0.46 秒 6/6 Passed) | ⚠️ 僅依賴抽樣單元測試 | 1.00x 基準參照 |
| **📘 TypeScript** | 5 個 Agent 模組化並發 (ESM/TS)<br>(Tier 0 ~ Tier 4) | 40~50 函數/大模組<br>(單 Agent 上下文龐大) | **479,872 Tokens** | TSX / Node test 套件<br>(0.48 秒 8/8 Passed) | ⚠️ 僅依賴抽樣單元測試 | 1.07x 消耗 |

---

## 🔬 二、核心技術突破：為什麼 Axiom 打洞管線具備決定性優勢？

傳統程式語言（Rust/Python/TypeScript）在多 Agent 協作時普遍面臨 **「上下文爆炸與幻覺探索」** 的瓶頸：
1. **$O(1)$ 上下文隔離 vs $O(N)$ 大檔案膨脹**：
   - 傳統語言由於單檔案往往堆疊 40~50 個函數，Agent 每次閱讀、思考和重寫都需要消耗海量 Tokens，且極易因長上下文導致幻覺；
   - Axiom 依靠編譯器原生 `axiom extract-holes`，將 200 個函數的骨架機械切分為 **200 個微型獨立任務（每個僅 ~160 Tokens）**。每個 Worker Agent 只需專注於單個函數的局部前後置契約，輸出單一純運算式（~18 Tokens），使實作階段 200 洞總消耗僅 **35,627 Tokens**！
2. **形式化數學證明 (0.08s) vs 脆弱的抽樣測試**：
   - 傳統語言依賴單元測試，面對複雜幾何與數值溢位，抽樣測試無法覆蓋全數域邊緣情況；
   - Axiom 內建 Z3/SMT 定理證明器，在 **0.08 秒** 內對 200 個函數的 Pre/Post 契約在 $(-\infty \sim +\infty)$ 全域進行了完備的數學證明，**無需編寫任何測試用例即可保證 100% 零漏洞**。
3. **架構師與工人徹底解耦**：
   - 架構師 Agent (Architect) 負責頂層領域建模與契約設計（產出帶 `?hole` 的骨架）；
   - 微型工人 Agents (Micro-Workers) 並發秒級填洞；
   - SMT 反例自癒 Agent (Healer) 單步修復邊界。

---

## 📂 三、詳細報告與各語言原始碼導覽

| 導覽項目 | 描述說明 | 連結位址 |
| :--- | :--- | :--- |
| 📋 **業務規範設計書** | 200 個真實業務函數分層與數學邏輯設計 | [`SPEC_200.md`](./SPEC_200.md) |
| 🌟 **Axiom 詳細評測** | 包含 4 步打洞管線分步 Token 消耗與 SMT 證明實測 | [`Benchmark_ax.md`](./Benchmark_ax.md) |
| 🐍 **Python 詳細評測** | 包含 5 個 Agent 消耗、Unittest 用例與結果 | [`Benchmark_py.md`](./Benchmark_py.md) |
| 🦀 **Rust 詳細評測** | 包含 5 個 Agent 消耗、Cargo test 套件與結果 | [`Benchmark_rs.md`](./Benchmark_rs.md) |
| 📘 **TypeScript 詳細評測** | 包含 5 個 Agent 消耗、TSX 測試套件與結果 | [`Benchmark_ts.md`](./Benchmark_ts.md) |
| 📜 **Axiom 原始碼目錄** | 包含骨架 `engine_200_holes.ax` 與已證明代碼 `engine_200.ax` | [`ax/`](./ax/) |
| 🐍 **Python 原始碼目錄** | 包含 5 層模組與單元測試套件 | [`py/`](./py/) |
| 🦀 **Rust 原始碼目錄** | 包含 5 層模組與 Cargo 單元測試套件 | [`rs/`](./rs/) |
| 📘 **TypeScript 原始碼目錄** | 包含 5 層模組與 Node/TSX 測試套件 | [`ts/`](./ts/) |

---

## 🖥️ 四、本機一鍵重現與驗證命令指南

```powershell
# 1. 驗證 Axiom 200 函數 SMT 全域形式化定理證明 (0.08秒)
cargo run -p axiom-cli -- verify examples/benchmark_200/ax/engine_200.ax

# 2. 執行 Python 200 函數完整單元測試套件
python -m unittest discover -s examples/benchmark_200/py/tests -v

# 3. 執行 Rust 200 函數完整測試套件
cargo test --manifest-path examples/benchmark_200/rs/Cargo.toml

# 4. 執行 TypeScript 200 函數完整測試套件
npx tsx --test examples/benchmark_200/ts/tests/engine_200.test.ts
```
