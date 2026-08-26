# 05 — Axiom 到标准安全 Rust 的基础转译后端

**What to build:**
构建 `axiom-transpiler` 模块，将无洞的完整 Axiom AST 转译为符合 Rust 编程习惯的高质量 Rust 源码。纯函数转为纯 Rust 函数，ADT 转为带有必要 Derive 的 `struct`/`enum`，`requires`/`ensures` 契约转为输入输出防御性断言与运行时验证代码。

**Blocked by:**
- 02 — 代数数据类型 (Record / Enum) 与模式匹配解析
- 03 — 代数效应声明与契约 (Requires / Ensures) 语法解析

**Status:**
ready-for-agent

**Seams:**
- `axiom_transpiler::transpile_to_rust(module: &Module) -> Result<String, TranspileError>`

**Acceptance criteria:**
- [ ] 纯函数与基本表达式转译为有效 Rust 语法
- [ ] ADT 结构转译为 `#[derive(Clone, Debug, PartialEq)]` 的 Struct/Enum
- [ ] 模式匹配与不可变更新转译为对应 Rust 表达式
- [ ] `requires` 与 `ensures` 生成断言保护代码
- [ ] 转译输出的代码可被 `rustc` 直接编译运行
