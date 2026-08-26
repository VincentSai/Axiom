# 13 — 多 Agent 无锁并发打洞派发与填充协作池

**What to build:**
构建 `crates/axiom-agent-pool`，实现面向 Multi-Agent 的并发打洞任务派发与无锁解决方案提交协作引擎。架构师派发 Hole 列表后，多 Worker 可在不同线程/进程中并发提交填充实现，自动触发 SMT 契约验证，并在验证通过后原子合入 ASTDB。

**Blocked by:**
- 11 — AST 节点规范化哈希计算与嵌入式内容寻址存储引擎 (ASTDB)

**Status:**
ready-for-agent

**Seams:**
- `axiom_agent_pool::AgentTaskPool`
- `AgentTaskPool::dispatch_holes(&mut self, module: &Module) -> Vec<HoleTask>`
- `AgentTaskPool::submit_solution(&mut self, hole_id: &str, solution_expr: Expr) -> Result<SmtResult, PoolError>`
- `AgentTaskPool::assemble_verified_module(&self) -> Result<Module, PoolError>`

**Acceptance criteria:**
- [ ] 支持并发派发与无冲突提交
- [ ] 提交实现时自动调用 `axiom-smt` 进行契约守卫，拒绝包含反例的实现
- [ ] 所有 Hole 填补并验证通过后，成功组装出完整的无洞 Module
