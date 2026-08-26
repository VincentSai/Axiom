# 06 — CLI 工具链与端到端打洞-转译-WASI 运行闭环

**What to build:**
构建 `axiom-cli` 命令行工具（提供 `check`, `extract-holes`, `emit-rust`, `run` 子命令），并编写全流程端到端自动化测试套件，打通“Axiom 源码 -> 提取打洞任务 -> 填充逻辑 -> 转译为 Rust -> 编译为 WASM -> wasmtime 执行验证”闭环。

**Blocked by:**
- 04 — 打洞语法解析与结构化 Agent 任务分发器
- 05 — Axiom 到标准安全 Rust 的基础转译后端

**Status:**
ready-for-agent

**Seams:**
- `axiom_cli::Cli`
- `tests/e2e_pipeline.rs`

**Acceptance criteria:**
- [ ] CLI 成功支持 `axiom check`, `axiom extract-holes`, `axiom emit-rust`
- [ ] 实现端到端测试用例：完整编译 `examples/transfer.ax` 并运行
- [ ] 输出清晰的机器可读结果与人类友好诊断
- [ ] 编写 README 与快速上手使用说明
