# Axiom 项目发展路线图 (Roadmap)

---

## 阶段一：原型与验证编译器 (MVP Prototype - Q1)
- [x] 完成语言架构设计与 /grill-me 压力验证
- [x] 编写基于 Rust 的语法解析器 (Parser) 与 AST 定义 (`axiom-ast`, `axiom-parser`)
- [x] 实现基础打洞机制 (`?hole`) 与 JSON 任务分发器 (`axiom-hole-extractor`)
- [x] 实现 Axiom 到 Rust 的基础转译器 (Transpiler) (`axiom-transpiler`)
- [x] 构建 CLI 工具链并完成端到端全流程验证 (`axiom-cli`, 35 个全量测试 100% 通过)

## 阶段二：Effect 系统与契约验证 (Q2)
- [ ] 实现代数效应 (Algebraic Effects) 的编译期隔离与分发
- [ ] 集成轻量 SMT 求解器（如 Z3 绑定），实现 `requires` / `ensures` 静态反例生成
- [ ] 实现结构化机器报错诊断协议 (JSON-based Diagnostics)

## 阶段三：内容寻址 AST 代码库与多 Agent 协同 (Q3)
- [ ] 实现基于 SQLite / LMDB 的 AST 节点哈希持久化存储引擎
- [ ] 开发虚拟文件系统视图 (Virtual FS)，支持人类在 VS Code 中像普通文件一样查看 AST
- [ ] 构建 Multi-Agent 调度演示：架构师 Agent 派发 20 个 Holes，多 Worker Agent 无锁并发实现

## 阶段四：全平台与生态互操作 (Q4)
- [ ] 完善 WebAssembly Component Model (WIT) 双向绑定
- [ ] 自动生成 C / TS / Rust 声明式 FFI 胶水代码
- [ ] 发布全平台脚手架（一键输出 Web、iOS/Android 动态库、Native CLI）