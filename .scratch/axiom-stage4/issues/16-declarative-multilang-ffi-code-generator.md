# 16 — 声明式多语言 FFI 代码生成器 (TypeScript / C / Rust C-ABI)

**What to build:**
构建 `crates/axiom-ffi-gen`，将 Axiom 模块自动导出为 TypeScript 类型定义 (`.d.ts`)、C 语言头文件 (`.h`) 以及 Rust `extern "C"` C-ABI 胶水代码，实现 Axiom 代码在 Web 前端、嵌入式/原生 C 宿主环境的无缝互操作。

**Blocked by:**
None — can start immediately

**Status:**
ready-for-agent

**Seams:**
- `axiom_ffi_gen::generate_typescript_bindings(module: &Module) -> Result<String, FfiError>`
- `axiom_ffi_gen::generate_c_header(module: &Module) -> Result<String, FfiError>`
- `axiom_ffi_gen::generate_c_abi_rust_glue(module: &Module) -> Result<String, FfiError>`

**Acceptance criteria:**
- [ ] TypeScript 绑定生成器正确输出 interface, type, 函数签名
- [ ] C 语言头文件生成器正确输出 struct, typedef, `extern "C"` 声明与头文件守卫
- [ ] C-ABI Rust 胶水代码可成功通过 `rustc` 编译
- [ ] 单元与集成测试全面覆盖
