# 08 — 轻量级 SMT / 符号执行契约求解器与反例生成引擎

**What to build:**
构建 `crates/axiom-smt`，针对纯函数体与 `requires` / `ensures` 契约进行符号执行与约束求解。在编译期静态验证函数逻辑是否在所有满足前置条件的输入下均满足后置条件；若不满足，精准求解出具体的变量输入反例（Counter-Example）。

**Blocked by:**
None — can start immediately

**Status:**
ready-for-agent

**Seams:**
- `axiom_smt::verify_function(func: &FunctionDecl) -> SmtResult`
- `axiom_smt::SmtResult::Verified`
- `axiom_smt::SmtResult::CounterExample(CounterExample)`

**Acceptance criteria:**
- [ ] 支持对算术运算、比较运算、逻辑分支、`match` 进行符号约束推导
- [ ] 对有 Bug 的实现（如未处理除以零、金额透支越界）成功求解出具体数值反例
- [ ] 对正确满足契约的纯函数验证通过返回 `Verified`
- [ ] 纯 Rust 实现，零非托管外部二进制依赖
