# 01 — Rust Workspace 初始化与基础纯函数解析器

**What to build:**
初始化 Axiom 编译器的 Cargo 多包工作区（`axiom-ast`, `axiom-parser`），定义不可变 AST 核心节点，并实现基础纯函数（基本类型 `Int`, `Float`, `Bool`, `String`, `()`, 变量绑定 `let`，二元/一元运算，函数定义与代码块返回值）的词法与语法解析。

**Blocked by:**
None — can start immediately

**Status:**
ready-for-agent

**Seams:**
- `axiom_ast::types::*`
- `axiom_ast::expr::*`
- `axiom_parser::parse_module(source: &str) -> Result<Module, ParseError>`

**Acceptance criteria:**
- [ ] 配置 Cargo Workspace 根工程，包含 `crates/axiom-ast` 和 `crates/axiom-parser`
- [ ] 严格定义不可变 AST 节点结构（支持 Serde 序列化）
- [ ] 成功解析基础纯函数（如 `fn add(a: Int, b: Int) -> Int { a + b }`）
- [ ] 语法错误时输出带有行号、列号与清晰描述的 `ParseError`
- [ ] 编写全面的单元测试与 TDD 红绿用例
