> **Languages / 语言 / 語言**: [English](../en/ROADMAP.md) | [简体中文](../zh-CN/ROADMAP.md) | [繁體中文](../zh-TW/ROADMAP.md)

---

# Axiom 项目发展路线图 (Roadmap)

---

## 阶段一：原型与验证编译器 (MVP Prototype - Q1)
- [x] 完成语言架构设计与 /grill-me 压力验证
- [x] 编写基于 Rust 的语法解析器 (Parser) 与 AST 定义 (`axiom-ast`, `axiom-parser`)
- [x] 实现基础打洞机制 (`?hole`) 与 JSON 任务分发器 (`axiom-hole-extractor`)
- [x] 实现 Axiom 到 Rust 的基础转译器 (Transpiler) (`axiom-transpiler`)
- [x] 构建 CLI 工具链并完成端到端全流程验证 (`axiom-cli`, 35 个全量测试 100% 通过)

## 阶段二：Effect 系统与契约验证 (Q2)
- [x] 实现代数效应 (Algebraic Effects) 的编译期隔离与分发 (`axiom-effects`)
- [x] 轻量纯 Rust SMT / 符号求解器，实现 `requires` / `ensures` 静态反例生成 (`axiom-smt`)
- [x] 实现结构化机器报错诊断协议 (JSON-based Diagnostics) (`axiom-diagnostics`)
- [x] CLI `axiom verify` 静态形式化验证与 Agent 自愈闭环 (`axiom-cli`, 52 个全量测试 100% 通过)

## 阶段三：内容寻址 AST 代码库与多 Agent 协同 (Q3)
- [x] 实现 AST 节点规范化哈希与内容寻址存储引擎 (`axiom-db`)
- [x] 开发虚拟代码投影视图与双向源码同步 (`axiom-vfs`)
- [x] 实现基于 AST 内容寻址与 CAS (Compare-And-Swap) 的乐观并发协作池 (`axiom-agent-pool`)
- [x] CLI `axiom db` 工具集与 20+ Holes 多 Agent 并发协同端到端演示 (`axiom-cli`, 65 个全量测试 100% 通过)

## 阶段四：全平台与生态互操作 (Q4)
- [x] 完善 WebAssembly Component Model (WIT) 双向绑定 (`axiom-wit`)
- [x] 自动生成 C / TS / Rust 声明式 FFI 胶水代码 (`axiom-ffi-gen`)
- [x] 发布全平台脚手架与生态导出工具链 (`axiom-cli`, 76 个全量测试 100% 通过)
