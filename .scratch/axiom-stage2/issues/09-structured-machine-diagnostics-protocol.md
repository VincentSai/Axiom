# 09 — 结构化机器报错诊断协议 (JSON-based Diagnostics)

**What to build:**
构建 `crates/axiom-diagnostics`，统一抽象解析错误、类型错误、Effect 污染报错与 SMT 契约反例，输出符合 `docs/MULTI_AGENT_WORKFLOW.md` 第 4 节规范的机器自愈 JSON 协议（包含 status, function, clause, counter_example, diagnosis, suggested_action）。

**Blocked by:**
- 07 — 代数效应编译期隔离检查与 Handler 分发运行时
- 08 — 轻量级 SMT / 符号执行契约求解器与反例生成引擎

**Status:**
ready-for-agent

**Seams:**
- `axiom_diagnostics::DiagnosticReport`
- `axiom_diagnostics::DiagnosticReport::to_json(&self) -> String`

**Acceptance criteria:**
- [ ] 统一格式化所有编译期与形式化验证异常
- [ ] 输出结构化 JSON 完全对齐协议规范
- [ ] 提供清晰的人类可读终端格式化展示
- [ ] 单元测试覆盖各种违背场景的 JSON 序列化
