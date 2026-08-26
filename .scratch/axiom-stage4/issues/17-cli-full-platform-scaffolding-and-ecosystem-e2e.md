# 17 — CLI 全平台脚手架、WIT/FFI 导出命令与生态互操作端到端全链路

**What to build:**
在 `axiom-cli` 中新增 `export-wit`, `export-ffi`, `scaffold` 命令，并编写端到端测试，验证从 Axiom 源码一键生成 WIT、TS 声明文件、C 头文件以及全平台脚手架项目模板。

**Blocked by:**
- 15 — WebAssembly Component Model (WIT) 契约描述生成器
- 16 — 声明式多语言 FFI 代码生成器 (TypeScript / C / Rust C-ABI)

**Status:**
ready-for-agent

**Seams:**
- `axiom_cli::Commands::ExportWit`
- `axiom_cli::Commands::ExportFfi`
- `axiom_cli::Commands::Scaffold`
- `tests/ecosystem_e2e_test.rs`

**Acceptance criteria:**
- [ ] CLI 成功导出 `.wit` 文件、`.d.ts` 文件与 `.h` 头文件
- [ ] CLI `scaffold` 成功生成 Web (WASM) / Native / FFI 目标工程模板
- [ ] 全量 `cargo test --workspace` 100% 通过，`cargo clippy` 0 警告
- [ ] 更新 `docs/ROADMAP.md` 阶段四状态
