# 14 — CLI ASTDB 命令集与 20+ Holes 多 Agent 并发无锁协同演示

**What to build:**
在 `axiom-cli` 中新增 ASTDB 管理命令（`db put`, `db get`, `db export`），并编写端到端多 Agent 协同流水线测试，演示 20 个打洞函数在多 Worker Agent 并发填充下 0 冲突合并，全自动通过 SMT 验证并成功转译为 Rust 运行。

**Blocked by:**
- 12 — 虚拟代码投影视图 (Virtual Code Projection) 与双向源码同步
- 13 — 多 Agent 无锁并发打洞派发与填充协作池

**Status:**
ready-for-agent

**Seams:**
- `axiom_cli::Commands::Db`
- `tests/multi_agent_e2e_test.rs`

**Acceptance criteria:**
- [ ] `axiom db` 命令行工具集正常工作
- [ ] 端到端集成测试成功模拟 20 个 Holes 的多 Agent 并发无锁填充与形式化验证
- [ ] `cargo test --workspace` 100% 通过，`cargo clippy` 0 警告
- [ ] 更新 `docs/ROADMAP.md` 阶段三状态
