# 10 — CLI `axiom verify` 静态验证子命令与 Agent 自愈端到端全链路

**What to build:**
在 `axiom-cli` 中新增 `axiom verify <file.ax>` 子命令（支持人类终端渲染与 `--json` 机器格式），并编写全流程端到端自动化测试，模拟“架构师给出 Spec ➔ 注入存在边界缺陷的实现 ➔ SMT 抛出精确反例 JSON ➔ 自动依据反例修正代码 ➔ 形式化验证 100% 通过”的 Agent 闭环自愈。

**Blocked by:**
- 09 — 结构化机器报错诊断协议 (JSON-based Diagnostics)

**Status:**
ready-for-agent

**Seams:**
- `axiom_cli::verify`
- `tests/self_healing_e2e_test.rs`

**Acceptance criteria:**
- [ ] `axiom verify` 命令正常工作并支持 `--json`
- [ ] 端到端测试完整重现反例自愈 SOP
- [ ] `cargo test --workspace` 100% 通过，`cargo clippy` 0 警告
- [ ] 更新 `docs/ROADMAP.md` 阶段二状态
