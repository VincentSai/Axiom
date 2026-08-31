# Axiom: AI-Native 全平臺並發程式語言

> **Languages / 语言 / 語言**: [English](README.en.md) | [简体中文](README.zh-CN.md) | [繁體中文](README.zh-TW.md)

---

> **Axiom** 是一門專門為 **AI Agent 協作** 與 **全平臺跨端發布** 量身設計的現代程式語言。  
> 核心理念：消除代碼衝突、杜絕隱式副作用、內建形式化契約與型別打洞、實現數十個 Agent 無鎖並行編寫。

---

## 🌟 核心願景與問題背景

在 AI 編碼大模型時代，傳統的文字檔案儲存、隱式全域狀態、複雜生命週期以及基於行差異的 Git 協作方式成為了阻礙 AI 高效、無衝突並行開發的最大瓶頸：
- **狀態失控**：隱式可變狀態使得 Agent 難以追蹤全域變化，導致生成代碼頻繁產生隱藏 Bug。
- **合併衝突**：多 Agent 並行編寫傳統原始碼檔案時，極易產生 Git Merge 衝突與符號重名。
- **跨平臺沉重**：現有跨端方案要麼依賴龐大的直譯器（Python/JS），要麼心智負擔極高（Rust 借用檢查器）。

**Axiom** 從底層重新思考了語言與工具鏈的設計：
1. **純函數 + 顯式代數效應 (Algebraic Effects)**：杜絕全域狀態污染，單函數 100% 自閉環。
2. **內容定址 AST (Content-Addressed Codebase)**：代碼以 AST Hash 儲存，徹底消滅命名衝突與 Git Merge 衝突。
3. **型別打洞協同 (Hole-Driven Development)**：架構師 Agent 宣告骨架與 `?hole`，Worker Agents 精準填空，編譯器提供結構化反例閉環。
4. **全平臺轉譯與 WASM 基底**：轉譯至 Rust/C 與 WebAssembly (WASM/WASI)，一鍵發布至 Web、iOS、Android、Windows、Linux、macOS 及嵌入式裝置。
5. **雙軌生態互操作**：支援 W3C WIT (WebAssembly Component Model) 與宣告式 FFI 包裝，零成本復用全球已有開源生態。

---

## 📊 對比熱門程式語言

| 語言 | 優勢 | 劣勢 (在 AI / 跨平臺 / 並發生成 視角下) | Axiom 的解法 |
| :--- | :--- | :--- | :--- |
| **Python** | LLM 訓練語料最豐富，生成流暢 | 動態型別易產生隱式 Bug，跨平臺打包極重，全域狀態易被多 Agent 污染 | 保留極簡運算式與模式匹配，引入純函數、強型別與全平臺 WASM 轉譯 |
| **Rust** | 記憶體安全、效能極致、跨平臺好 | 借用檢查器與複雜生命週期導致 AI 試錯成本極高（易陷入編譯器報錯死循環） | 保留 Rust 級別的記憶體安全與效能底座，對上層屏蔽生命週期心智負擔 |
| **Go** | 編譯快速、跨平臺友好、並發簡單 | 缺乏代數資料型別 (ADT) 與形式化契約，多 Agent 仍有命名與介面破壞風險 | 引入 ADT、顯式 Effect、內建 Pre/Post 契約與內容定址 AST |
| **TypeScript** | Web/全端生態豐富 | 執行期無法保證絕對型別安全，跨 Native 端依賴重型執行期，非同步傳染 | 強型別自描述 + 轉譯為 Rust/WASM，零執行期開銷，統一跨端 |

---

## 🏆 權威基準評測：200-Function 真實業務多 Agent 協作大比拼

在工業級遊戲引擎規範 [`examples/benchmark_200/SPEC_200.md`](examples/benchmark_200/SPEC_200.md) 下（涵蓋空間幾何、非線性護甲減傷、五行相剋、戰鬥狀態機、AI 衰減、加權掉落與世界排程等 200 個真實業務函數），由真實 AI 大模型（OpenAI Codex CLI）多 Agent 並發協作開發，**Axiom、Python、Rust 與 TypeScript** 四大語言體系的實測數據矩陣如下：

| 語言體系 | 多 Agent 協作模式與架構 | 任務粒度與上下文邊界 | 真實消耗 Tokens | 正確性驗證手段與耗時 | 形式化數學保證 | 相對基準對比 |
| :--- | :--- | :--- | :--- | :--- | :--- | :--- |
| **🌟 Axiom** | **`/axiom-pipeline` 全自動打洞管線**<br>(Architect + 編譯器拆解 + 200 Workers) | **單函數 `hole-task.json` 隔離<br>($O(1)$ 超輕量局部上下文)** | **183,662 Tokens** | **SMT 全域定理證明<br>(0.08 秒 200/200)** | **✅ 全數域 $(-\infty \sim +\infty)$<br>100% 絕對零漏洞** | **🔥 節省 58.9% ~ 61.7%<br>(2.4x 提速)** |
| **🐍 Python** | 5 個 Agent 模組化並發 (Tier 0~4) | 40~50 函數/大模組 ($O(N)$ 大檔案) | **384,089 Tokens** | Unittest (0.002s, 9/9 Passed) | ⚠️ 僅依賴抽樣單元測試 | 0.86x 消耗 |
| **🦀 Rust** | 5 個 Agent 跨 crate/mod 並發 | 40~50 函數/大模組 ($O(N)$ 大檔案) | **446,775 Tokens** | Cargo test (0.46s, 6/6 Passed) | ⚠️ 僅依賴抽樣單元測試 | 1.00x 基準參照 |
| **📘 TypeScript** | 5 個 Agent 模組化並發 (ESM/TS) | 40~50 函數/大模組 ($O(N)$ 大檔案) | **479,872 Tokens** | TSX / Node test (0.48s, 8/8 Passed) | ⚠️ 僅依賴抽樣單元測試 | 1.07x 消耗 |

### 🔬 為什麼 Axiom 打洞管線具備顛覆性優勢？
1. **$O(1)$ 微型上下文隔離，消滅幻覺與 Token 浪費**：
   傳統語言由於 Agent 必須在 40~50 函數的大檔案中反覆閱讀與重寫，上下文持續膨脹。Axiom 編譯器原生工具 `axiom extract-holes` 機械化拆解出 200 個獨立的 `hole-task.json`（每個僅 ~160 Tokens），Worker 僅需輸出單個純運算式（~18 Tokens），**200 個函數的實作階段僅消耗 35,627 Tokens**！
2. **0.08 秒 SMT 全數域形式化定理證明**：
   傳統語言面對邊界溢位只能依賴有限用例抽樣，而 Axiom 內建 Z3 定理證明器在 0.08 秒內對 200 個函數的契約完成了 $(-\infty \sim +\infty)$ 全數域數學證明，**從根本上杜絕了隱式 Bug 與數值溢位**。

> 📖 詳細跨語言測試報告、各模組消耗明細與完整原始碼，請參閱：[**200-Functions 基準測試專題 (examples/benchmark_200/README.md)**](examples/benchmark_200/README.md)。

---

## 📂 專案文件導覽

- 📘 [架構設計白皮書 (docs/ARCHITECTURE.md)](docs/ARCHITECTURE.md)
- 📝 [語言語法與語意規範草案 (docs/SPECIFICATION.md)](docs/SPECIFICATION.md)
- 🤖 [多 Agent 協作工作流程與 AST 儲存 (docs/MULTI_AGENT_WORKFLOW.md)](docs/MULTI_AGENT_WORKFLOW.md)
- 🔬 [主流語言深度對比與能力邊界分析 (docs/RESEARCH_LANGUAGE_COMPARISON.md)](docs/RESEARCH_LANGUAGE_COMPARISON.md)
- 🗺️ [專案發展藍圖 (docs/ROADMAP.md)](docs/ROADMAP.md)
- 💡 [範例 1：銀行轉帳服務 (examples/transfer.ax)](examples/transfer.ax)
- 🎮 [範例 2：跨平臺遊戲循環 (examples/game_loop.ax)](examples/game_loop.ax)
- 👾 [範例 3：Axiom Survivors 完整版 Unity 遊戲 (examples/game_survivor/)](examples/game_survivor/README.md)

---

## 🛠️ 語法速覽 (Syntax Glance)

```rust
// 1. 不可變代數資料型別 (ADT)
type Player = {
    id: Int,
    name: String,
    score: Int,
}

// 2. 宣告顯式副作用 (Algebraic Effect)
effect Audio {
    fn play_sound(sound_id: String): ()
}

effect Network {
    fn send_packet(packet: String): ()
}

// 3. 架構師 Agent 定義的帶契約函數骨架 (含 ?hole 打洞)
fn level_up(player: Player, bonus: Int) -> Player 
    with [Audio, Network]
    requires {
        bonus > 0,
        player.score >= 0,
    }
    ensures {
        result.score == player.score + bonus
    }
{
    // 交由 Worker Agent 並發填洞實作
    ?level_up_implementation
}
```

---

## 🚀 快速安裝與預編譯二進位制下載 (Installation & Download)

無需安裝 Rust 編譯環境，您可以直接從 GitHub 下載開箱即用的預編譯二進位制與技能包：

### 1. 下載 Release 預編譯包
前往 GitHub 倉庫的 [**Releases**](../../releases) 頁面，下載對應作業系統的壓縮包（**每個壓縮包內均已內建二進位制與完整的 `skills/` 技能套件**）：
- **Windows (x64)**: `axiom-v*-x86_64-pc-windows-msvc.zip`（內含 `axiom.exe` 與 `skills/`）
- **Linux (x64)**: `axiom-v*-x86_64-unknown-linux-gnu.tar.gz`（內含 `axiom` 與 `skills/`）
- **macOS (Apple Silicon)**: `axiom-v*-aarch64-apple-darwin.tar.gz`
- **macOS (Intel)**: `axiom-v*-x86_64-apple-darwin.tar.gz`
- **獨立技能包 (通用)**: `axiom-skills-v*.zip`（純 Agent Skills 規則包）

### 2. 設定環境變數 (Windows)
1. 將下載的壓縮包解壓縮至任意固定目錄（例如 `C:\Program Files\Axiom\` 或 `D:\Tools\axiom\`）；
2. 將該目錄路徑添加至系統的 **環境變數 `PATH`** 中；
3. 開啟新的終端（PowerShell 或 CMD），執行 `axiom --help` 驗證安裝成功。

> 💡 **開發者原始碼安裝（可選）**：  
> 若本機具備 Rust 工具鏈，可直接在倉庫根目錄執行：`cargo install --path crates/axiom-cli`

---

## 🤖 AI Agent 技能包 (Skills) 安裝與啟用指南

Axiom 隨 Release 提供了專為現代 AI 編碼 Agent 設計的 5 大協同技能包：
- `/axiom-pipeline`：全自動打洞管線（架構師打洞 -> 提取任務 -> Worker 並發填洞 -> SMT 證明 -> 自癒）
- `/axiom-interactive`：分階段人機協作打洞（先生成契約等待人類審查，確認後再極速填洞）
- `axiom-architect` / `axiom-worker` / `axiom-healer`：微專業 Agent 規範

### 各主流 Agent 客戶端安裝方法：

| AI Agent 客戶端 | 安裝路徑 / 設定方法 | 啟用與使用指令 |
| :--- | :--- | :--- |
| **🌟 Google Antigravity / Gemini CLI** | 將解壓縮後的 `skills/*` 複製至全域 `%USERPROFILE%\.gemini\config\skills\` 或專案目錄 `.gemini/config/skills/` | 在聊天框直接輸入：<br>`/axiom-pipeline 實作...` |
| **⚡ Cursor IDE** | 將 `skills/axiom-pipeline/SKILL.md` 複製為專案根目錄 `.cursor/rules/axiom-pipeline.mdc`（或追加至 `.cursorrules`） | 在 Composer / Agent 模式輸入：<br>`使用 axiom-pipeline 規範實作...` |
| **🌊 Windsurf (Codeium)** | 將 `skills/axiom-pipeline/SKILL.md` 的內容追加至專案根目錄的 `.windsurfrules` | 在 Cascade 聊天框中輸入：<br>`遵循 Axiom 打洞管線實作...` |
| **🧠 Claude Code / Claude Desktop** | 將技能內容追加至專案根目錄 `CLAUDE.md`，或在 Claude.ai Project Knowledge 中上傳 `skills/` 下的 markdown 檔案 | 在對話中輸入：<br>`啟動 /axiom-pipeline 實作...` |
| **🤖 OpenAI Codex CLI / Aider** | 在當前工作區放入 `skills/` 或在系統提示詞中引用 `skills/axiom-pipeline/SKILL.md` | 在提示詞中指定：<br>`依據 skills/axiom-pipeline 規範開發` |

> 📖 更多詳細技能說明，請參閱：[**Axiom 技能包使用手冊 (skills/README.md)**](skills/README.md)。

---

## 💻 CLI 工具鏈與 `axiom.exe` 使用指南

設定好環境變數後，您可以直接使用 `axiom`（或 `axiom.exe`）指令呼叫完整的工具鏈功能：

```powershell
# 1. 語法檢查與打洞統計
axiom check examples/transfer.ax

# 2. 靜態 SMT 契約形式化驗證（人類終端可讀模式）
axiom verify examples/transfer.ax

# 3. 靜態 SMT 形式化驗證（輸出機器可讀 JSON，供 Agent 閉環自癒）
axiom verify examples/transfer.ax --json

# 4. 機械化提取打洞任務 JSON（派發給多個 Worker Agent 並行填洞）
axiom extract-holes examples/transfer.ax
axiom extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 5. 內容定址 AST 代碼庫 (ASTDB) 管理與虛擬原始碼投影
axiom db put examples/transfer.ax
axiom db export #mod_589e44855c2b4e0223acf6867cdd3c6f0947fb3a23d7afafbb62e7fbc044559a

# 6. WebAssembly Component Model (WIT) 契約匯出
axiom export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit

# 7. 多語言 FFI 膠水代碼與型別宣告匯出 (TypeScript .d.ts / C 標頭檔 .h / Rust C-ABI)
axiom export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
axiom export-ffi examples/game_loop.ax --lang c -o game_loop.h
axiom export-ffi examples/game_loop.ax --lang rust -o game_loop_c_abi.rs

# 8. 全平臺專案鷹架一鍵生成 (支援 web / native / node)
axiom scaffold my_game --target web --out-dir ./projects

# 9. 轉譯至 Rust 原始碼並直接編譯執行
axiom emit-rust examples/game_loop.ax -o game_loop.rs
axiom run path/to/program_with_main.ax
```

> ⚙️ **原始碼除錯模式**：若未安裝全域二進位制，在原始碼根目錄下將 `axiom <subcommand>` 替換為 `cargo run -p axiom-cli -- <subcommand>` 即可達到完全相同的執行效果。
