# Axiom: 面向 AI 協作的可驗證邏輯中間層與執行時 (AI-Native Verified Logic Layer)

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** 是一套專門面向 **AI Agent 協作與形式化數學驗證** 的現代可驗證程式表示與執行系統。  
> **核心戰略定位**：**不替代 Rust / C# (Unity) / TypeScript**，而是與其深度共生——Axiom 專注於最適合 AI 生成、最需要確定性與數學契約防禦的**核心邏輯層**（遊戲機制、數值管線、狀態機、交易結算、Agent 工具權限），外圍無縫轉譯並嵌入 Rust、Unity C#、WASM 與 C。  
> **核心技術壁壘**：**打洞驅動開發 (Hole-Driven) + SMT 形式化契約定理證明 + 多 Agent 局部語義協作模型**。

---

## 🌟 核心設計理念與共生架構

```
┌─────────────────────────────────────────────────────────────────┐
│               Host Presentation / Engine Layer                  │
│       Unity / C#  •  Web / TypeScript  •  Native Rust / C       │
└───────────────────────────────┬─────────────────────────────────┘
                                │ FFI / Native Bridge / WASM
                                ▼
┌─────────────────────────────────────────────────────────────────┐
│               Axiom Verified Logic Layer (Axiom 內核)            │
│  • Pure Functions (不可變狀態轉移) • Algebraic Effects (顯式副作用隔離)  │
│  • SMT Formal Contracts (保證滿足數學契約) • Hole-Driven Micro-Workers  │
└─────────────────────────────────────────────────────────────────┘
```

在 AI 輔助核心業務與遊戲邏輯開發的場景中，傳統的通用程式語言存在若干痛點：
- **狀態失控與邊界隱患**：隱式全域狀態和隨機副作用導致 AI 生成的程式碼容易產生隱藏邊界 Bug 和連線不同步。
- **上下文膨脹與注意力發散**：讓 Agent 在龐大工程長文件中反覆通讀和修改，極易導致 Token 浪費與幻覺。
- **測試有限性 vs 形式化契約**：手寫單元測試只能抽樣驗證，無法從數學上窮舉邊界缺陷。

**Axiom** 圍繞這些問題構建了面向 AI 的自閉環工具鏈：
1. **純函數 + 顯式代數效應 (Algebraic Effects)**：業務邏輯表達為無副作用的純狀態轉移；副作用透過 Effect 顯式宣告，轉譯端自動生成原生 Rust Trait 與 Handler 相依性注入。
2. **SMT-LIB2 / Microsoft Z3 定理證明**：內建 `requires` / `ensures` 契約，可一鍵編譯為標準 SMT-LIB2 指令碼並透過 Z3 求解器證明或提取反例，**數學級保證程式嚴格滿足形式化契約**。
3. **微型打洞協作 (Hole-Driven Development)**：透過 `?hole` 宣告待實現點，編譯器機械化拆解出獨立的 `hole-task.json`，讓 Agent 在 $O(1)$ 極簡局部上下文中精準填空。
4. **多 Agent 協作任務池 (`axiom pool`)**：內建多 Agent 打洞協調系統，支援候選解提交、全自動 SMT 反例過濾攔截與無損程式碼組裝。
5. **Unity 零拷貝雙緩衝橋接 (`axiom bridge`)**：轉譯至 Rust 編譯為 C-ABI 原生庫，自動生成 Unity C# P/Invoke 與連續記憶體雙緩衝快照。

---

## 🔬 嚴謹科學實證：單一變數 A/B 對照評測體系

外部評測曾提出過一個核心問題：*「Axiom 的 Token 節省與可靠性，到底是因為語言本身，還是僅僅因為工作流切成了微任務？」*

為了徹底解耦變數，我們在 [`examples/benchmark_eval/`](examples/benchmark_eval/) 中建立了嚴格的 **單一變數 A/B 科學對照評測套件**：
- **黃金標準任務集 ([`golden_tasks.json`](examples/benchmark_eval/golden_tasks.json))**：精選覆蓋空間幾何、非線性抗性、戰鬥數值、掉落狀態機與世界仿真的 20 個黃金業務任務；
- **三組對照實驗協議 ([`prompt_templates.md`](examples/benchmark_eval/protocols/prompt_templates.md))**：
  1. **Group 1**：傳統語言大檔案工作流（Rust Monolith，長程式碼上下文）；
  2. **Group 2（核心解耦）**：傳統語言單函數微任務（Rust Micro-Worker，消除任務拆解顆粒度差異）；
  3. **Group 3**：Axiom 契約打洞流水線（Axiom Hole Pipeline，基於數學契約與 SMT 定理證明）。
- **官方 API 驅動指令碼 ([`eval_runner.py`](examples/benchmark_eval/eval_runner.py))**：
  基於 Google Gemini 官方 API（`countTokens` 與 `generateContent`），**使用官方真實分詞器與官方 usageMetadata** 統計真實物理 Token 消耗與通過率，杜絕任何主觀偽造數據。使用者設定 `GEMINI_API_KEY` 後即可在本地一鍵執行複測。

---

## 🎮 實戰演示：Axiom Survivors 割草遊戲

在 [`examples/game_survivor/`](examples/game_survivor/) 中提供了一個完整的 2D 肉鴿割草遊戲落地示範：
- **形式化邏輯核心 ([`survivor_core.ax`](examples/game_survivor/ax/survivor_core.ax))**：涵蓋角色移動、彈道歐拉物理推進、歐幾里得圓形碰撞、傷害減傷與經驗升級突破等 15 個純函數與數學契約；
- **Unity 表現層 ([`SurvivorGameController.cs`](examples/game_survivor/Assets/Scripts/Gameplay/SurvivorGameController.cs))**：零配置自舉（打開 SampleScene 即可試玩）、過程化像素精靈生成、平滑相機跟隨與死亡一鍵滿血重開；
- **零 GC 雙緩衝同步**：Rust Native 庫與 Unity 之間透過定長連續記憶體進行 60FPS+ 雙緩衝傳遞。

---

## 🚀 安裝與建置 (Installation & Build)

透過 Cargo 從原始碼一鍵編譯並安裝 `axiom` CLI 工具鏈：

```bash
# 從原始碼安裝 axiom CLI 到本地 PATH
cargo install --path crates/axiom-cli

# 驗證安裝
axiom --help
```

---

## 🎨 IDE 支援與生態規劃 (IDE Support)

- **VS Code 擴充套件 (規劃中)**：語言伺服器協定 (LSP) 與 TextMate 語法突顯正在規劃和開發中（相關目錄位於 [`editors/vscode-axiom`](editors/vscode-axiom)）。
- **當前推薦**：直接使用 `axiom check` 與 `axiom verify` 命令列工具鏈配合編輯器終端機進行語法、型別與契約驗證。

---

## 💻 CLI 工具鏈指南

```powershell
# 1. 語法與靜態型別檢查、打洞統計
axiom check examples/transfer.ax

# 2. 形式化契約驗證 (自動探測 Z3 求解器，回退至內建符號引擎；支援 --strict 模式)
axiom verify examples/benchmark_200/ax/engine_200.ax
axiom verify examples/benchmark_200/ax/engine_200.ax --strict

# 3. 多智慧體打洞任務池排程與驗證 (Agent Task Pool)
axiom pool status examples/benchmark_200/ax/engine_200_holes.ax
axiom pool submit examples/math.ax --hole calc_h0 --expr "x + 1"
axiom pool assemble examples/math.ax --solutions solutions.json -o math_verified.ax

# 4. 匯出標準 SMT-LIB2 (.smt2) 檔案供第三方求解器 (Z3/CVC5) 獨立驗證
axiom export-smt2 examples/transfer.ax -o transfer.smt2

# 5. 機械化提取打洞任務 JSON (派發給 Worker Agents 並行填洞)
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 6. Unity 跨端程式碼生成與零拷貝雙緩衝橋接
axiom bridge examples/game_survivor/ax/survivor_core.ax --target unity --out examples/game_survivor/Assets/Scripts/AxiomBridge --library-name axiom_survivor

# 7. 轉譯至 Rust 原始碼並由 rustc 原生編譯（支援代數效應 Trait 生成與透傳）
axiom emit-rust examples/game_loop.ax -o game_loop.rs
rustc --crate-type lib game_loop.rs -o game_loop.rlib

# 8. 多語言介面繫結匯出 (TypeScript / C / WIT)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit
```

---

## 📂 專案文件導航

- 🔬 [單一變數 A/B 科學對照評測方法與套件 (examples/benchmark_eval/README.md)](examples/benchmark_eval/README.md)
- 📊 [200 函數高壓基準實測報告 (examples/benchmark_200/Benchmark_ax.md)](examples/benchmark_200/Benchmark_ax.md)
- 📘 [架構設計白皮書 (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [語言語法與語義規範草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [多 Agent 協作工作流與 AST 儲存 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🗺️ [專案發展路線圖 (docs/ROADMAP.md)](docs/ROADMAP.md)
- 👾 [Axiom Survivors Unity 範例工程 (examples/game_survivor/)](examples/game_survivor/README.md)
