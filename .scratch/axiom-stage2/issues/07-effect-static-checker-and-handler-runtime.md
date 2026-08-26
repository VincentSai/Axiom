# 07 — 代数效应编译期隔离检查与 Handler 分发运行时

**What to build:**
构建 `crates/axiom-effects`，实现 Effect 调用的静态污染与传播分析（主调函数若调用了包含 `with [E]` 的操作或函数，必须显式标注 `with [E]`），并在转译器中支持将 Effect 声明映射为 Rust Trait / 依赖注入 Handler 接口，支持在运行时绑定具象或 Mock Handler。

**Blocked by:**
None — can start immediately

**Status:**
ready-for-agent

**Seams:**
- `axiom_effects::check_module_effects(module: &Module) -> Result<(), EffectCheckError>`
- `axiom_effects::EffectEnv`

**Acceptance criteria:**
- [ ] 静态检测未标注 `with` 的隐式 Effect 调用并报错
- [ ] 支持将 Effect 接口转译为 Rust Trait
- [ ] 支持为 `examples/transfer.ax` 中的 `Database`, `Clock`, `Log` 提供 mock 与真实 handler
- [ ] 编写全面的单元与集成测试
