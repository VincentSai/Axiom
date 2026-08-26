# 04 — 打洞语法解析与结构化 Agent 任务分发器

**What to build:**
实现打洞语法 `?hole_id` 在表达式与函数体中的识别，构建 `axiom-hole-extractor` 模块，能静态遍历 AST，提取未实现洞位的上下文（参数列表、返回类型、契约条件、允许的 Effect），并输出符合规范的 `hole-task.json` 任务描述文件。

**Blocked by:**
- 02 — 代数数据类型 (Record / Enum) 与模式匹配解析
- 03 — 代数效应声明与契约 (Requires / Ensures) 语法解析

**Status:**
ready-for-agent

**Seams:**
- `axiom_ast::expr::Expr::Hole(String)`
- `axiom_hole_extractor::extract_holes(module: &Module) -> Vec<HoleTask>`
- `axiom_hole_extractor::HoleTask::to_json(&self) -> String`

**Acceptance criteria:**
- [ ] 解析器正确将 `?hole_id` 解析为 AST 独立节点
- [ ] Extractor 准确提取洞位的参数、返回值、契约与 Effect 约束
- [ ] 生成的 JSON 完全符合 `docs/SPECIFICATION.md` 第 5 节 Schema
- [ ] 针对 `examples/transfer.ax` 提取出完整的转账逻辑任务 JSON
