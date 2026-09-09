> **Languages / 语言 / 語言**: [English](../en/ROADMAP.md) | [简体中文](../zh-CN/ROADMAP.md) | [繁體中文](../zh-TW/ROADMAP.md)

---

# Axiom 專案發展藍圖 (Roadmap)

---

## 階段一：原型與驗證編譯器 (MVP Prototype - Q1)
- [x] 完成語言架構設計與 /grill-me 壓力驗證
- [x] 編寫基於 Rust 的語法解析器 (Parser) 與 AST 定義 (`axiom-ast`, `axiom-parser`)
- [x] 實現基礎打洞機制 (`?hole`) 與 JSON 任務分發器 (`axiom-hole-extractor`)
- [x] 實現 Axiom 到 Rust 的基礎轉譯器 (Transpiler) (`axiom-transpiler`)
- [x] 構建 CLI 工具鏈並完成端到端全流程驗證 (`axiom-cli`, 35 個全量測試 100% 通過)

## 階段二：Effect 系統與契約驗證 (Q2)
- [x] 實現代數效應 (Algebraic Effects) 的編譯期隔離與分發 (`axiom-effects`)
- [x] 輕量純 Rust SMT / 符號求解器，實現 `requires` / `ensures` 靜態反例生成 (`axiom-smt`)
- [x] 實現結構化機器報錯診斷協定 (JSON-based Diagnostics) (`axiom-diagnostics`)
- [x] CLI `axiom verify` 靜態形式化驗證與 Agent 自癒閉環 (`axiom-cli`, 52 個全量測試 100% 通過)

## 階段三：內容定址 AST 代碼庫與多 Agent 協同 (Q3)
- [x] 實現 AST 節點規範化雜湊與內容定址儲存引擎 (`axiom-db`)
- [x] 開發虛擬代碼投影視圖與雙向原始碼同步 (`axiom-vfs`)
- [x] 實現基於 AST 內容定址與 CAS (Compare-And-Swap) 的樂觀並發協作池 (`axiom-agent-pool`)
- [x] CLI `axiom db` 工具集與 20+ Holes 多 Agent 並發協同端到端演示 (`axiom-cli`, 65 個全量測試 100% 通過)

## 階段四：全平臺與生態互操作 (Q4)
- [x] 完善 WebAssembly Component Model (WIT) 雙向綁定 (`axiom-wit`)
- [x] 自動生成 C / TS / Rust 宣告式 FFI 膠水代碼 (`axiom-ffi-gen`)
- [x] 發布全平臺鷹架與生態匯出工具鏈 (`axiom-cli`, 76 個全量測試 100% 通過)
