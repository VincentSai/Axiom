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
- 🗺️ [專案發展藍圖 (docs/ROADMAP.md)](docs/ROADMAP.md)
- 💡 [範例 1：銀行轉帳服務 (examples/transfer.ax)](examples/transfer.ax)
- 🎮 [範例 2：跨平臺遊戲循環 (examples/game_loop.ax)](examples/game_loop.ax)

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

## 💻 CLI 工具鏈完整指南

從 workspace 根目錄執行（或安裝 `axiom` 二進位制）：

```powershell
# 1. 語法檢查與打洞統計
cargo run -p axiom-cli -- check examples/transfer.ax

# 2. 靜態 SMT 契約形式化驗證（支援人類終端模式與 Agent 機器 JSON 自癒模式）
cargo run -p axiom-cli -- verify examples/transfer.ax
cargo run -p axiom-cli -- verify examples/transfer.ax --json

# 3. 提取打洞任務派發 JSON
cargo run -p axiom-cli -- extract-holes examples/transfer.ax
cargo run -p axiom-cli -- extract-holes examples/transfer.ax --out-dir .scratch/hole-tasks

# 4. 內容定址 AST 代碼庫 (ASTDB) 管理與虛擬原始碼投影
cargo run -p axiom-cli -- db put examples/transfer.ax
cargo run -p axiom-cli -- db export #mod_589e44855c2b4e0223acf6867cdd3c6f0947fb3a23d7afafbb62e7fbc044559a

# 5. WebAssembly Component Model (WIT) 契約匯出
cargo run -p axiom-cli -- export-wit examples/transfer.ax --package axiom:transfer -o transfer.wit

# 6. 多語言 FFI 膠水代碼與型別宣告匯出 (TypeScript .d.ts / C 標頭檔 .h / Rust C-ABI)
cargo run -p axiom-cli -- export-ffi examples/game_loop.ax --lang ts -o game_loop.d.ts
cargo run -p axiom-cli -- export-ffi examples/game_loop.ax --lang c -o game_loop.h
cargo run -p axiom-cli -- export-ffi examples/game_loop.ax --lang rust -o game_loop_c_abi.rs

# 7. 全平臺專案鷹架一鍵生成
cargo run -p axiom-cli -- scaffold my_game --target web --out-dir ./projects

# 8. 轉譯至 Rust 並編譯執行
cargo run -p axiom-cli -- emit-rust examples/game_loop.ax -o game_loop.rs
cargo run -p axiom-cli -- run path/to/program_with_main.ax
```

Axiom 工具鏈完整覆蓋了從語法解析、代數效應隔離、SMT 反例自癒、內容定址儲存、多 Agent 無衝突並發組裝到 Web/Native/FFI 全平臺發布的全鏈路。
