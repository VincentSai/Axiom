# 15 — WebAssembly Component Model (WIT) 契约描述生成器

**What to build:**
构建 `crates/axiom-wit`，实现将 Axiom Module 的数据类型（Record, Enum, Primitive）、纯函数签名与代数效应（作为 import/export interface）精确转译为标准 WebAssembly Interface Type (`.wit`) 格式。

**Blocked by:**
None — can start immediately

**Status:**
ready-for-agent

**Seams:**
- `axiom_wit::generate_wit(module: &Module, package_name: &str) -> Result<String, WitError>`

**Acceptance criteria:**
- [ ] Record 与 Enum 正确映射为 WIT `record` 与 `variant`
- [ ] 代数效应映射为 WIT `interface` import
- [ ] 导出函数正确生成 WIT `export` 定义
- [ ] 单元与集成测试全面覆盖
