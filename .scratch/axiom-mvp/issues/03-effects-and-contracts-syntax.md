# 03 — 代数效应声明与契约 (Requires / Ensures) 语法解析

**What to build:**
在 AST 与解析器中实现 Algebraic Effects 接口定义（`effect FileSystem { fn ... }`）、函数签名上的 Effect 依赖声明（`with [FileSystem, Database]`），以及函数契约块（`requires { ... }`、`ensures { ... }`，支持特殊后置变量 `result`）。

**Blocked by:**
- 01 — Rust Workspace 初始化与基础纯函数解析器

**Status:**
ready-for-agent

**Seams:**
- `axiom_ast::effects::EffectDecl`
- `axiom_ast::contracts::ContractBlock`
- `axiom_ast::decl::FunctionDecl`
- `axiom_parser::parse_effect_decl(source: &str) -> Result<EffectDecl, ParseError>`

**Acceptance criteria:**
- [ ] AST 建模 Effect 操作声明与函数 Effects 标注
- [ ] AST 建模 `requires` 和 `ensures` 断言表达式列表
- [ ] 成功解析 `examples/transfer.ax` 中的全部 Effect 与契约
- [ ] 编写涵盖复杂契约表达式（逻辑与、比较、函数调用）的测试用例
