# 02 — 代数数据类型 (Record / Enum) 与模式匹配解析

**What to build:**
在 AST 与解析器中支持 Product Types（记录类型 `type Point = { x: Float, y: Float }`）与 Sum Types（和类型 `type Shape = | Circle(Float) | Point`），支持记录不可变更新表达式（`{ ...p, x: 3.0 }`），以及 `match` 模式匹配（包含分支守卫与变量解构绑定）。

**Blocked by:**
- 01 — Rust Workspace 初始化与基础纯函数解析器

**Status:**
ready-for-agent

**Seams:**
- `axiom_ast::types::TypeDef`
- `axiom_ast::expr::MatchExpr`
- `axiom_parser::parse_type_def(source: &str) -> Result<TypeDef, ParseError>`
- `axiom_parser::parse_expr(source: &str) -> Result<Expr, ParseError>`

**Acceptance criteria:**
- [ ] AST 完整建模 Record 字段定义与 Enum 变体结构
- [ ] 能够成功解析 `examples/game_loop.ax` 中的所有 ADT 定义
- [ ] 能够成功解析多分支 `match` 与记录不可变更新
- [ ] 编写全面的单元测试覆盖模式匹配与 ADT 边界
