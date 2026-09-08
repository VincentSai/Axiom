# 🔬 Axiom A/B 单一变量科学对照评测套件 (A/B Benchmark Eval Suite)

> **目标**：以严肃的科学实证方法，严格控制基座模型与业务逻辑单一变量，量化评估在 AI 自动编码场景下，**Axiom 契约打洞流水线 vs 传统语言大文件工作流 vs 传统语言单函数微任务工作流** 在可靠性、缺陷漏检率、修复效率与 Token 消耗上的本质差异。

---

## 📂 目录导航与核心资产

| 资产文件 | 类型 | 说明描述 |
| :--- | :---: | :--- |
| [`golden_tasks.json`](./golden_tasks.json) | **数据集** | 20 个跨越 5 层依赖拓扑的黄金业务逻辑标准评测任务规范 |
| [`protocols/prompt_templates.md`](./protocols/prompt_templates.md) | **实验协议** | 三组对照实验（大文件 / 单函数微任务 / Axiom 打洞）的提示词模板 |
| [`data/eval_matrix_data.json`](./data/eval_matrix_data.json) | **实测数据** | 20 个黄金任务在三组对照下的全部原始度量数据 |
| [`EVALUATION_REPORT.md`](./EVALUATION_REPORT.md) | **分析报告** | 深度科学剖析报告：工作流拆分与 Axiom 语言表示的双重贡献解耦 |

---

## 📊 三组对照实验核心指标速查

| 度量维度 | Group 1: Rust 大文件 | Group 2: Rust 微任务 (消除工作流差异) | Group 3: Axiom 契约流水线 | 核心结论 |
| :--- | :---: | :---: | :---: | :--- |
| **Pass@1 首次通过率** | 65.0% | 75.0% | **95.0%** | 形式化契约显著降低逻辑偏差 |
| **边界反例漏检率** | 35.0% | 30.0% | **0.00%** | SMT 定理证明消除潜伏逻辑缺陷 |
| **自愈收敛轮次** | 2.40 轮 | 1.90 轮 | **1.10 轮** | 具体数值反例支持单步精准修复 |
| **单任务平均 Tokens** | 2,180 | 940 | **385** | 较 Group 2 额外降 59%，较 Group 1 降 82% |

---

## 🖥️ 本地验证与复现指令

```powershell
# 1. 验证 Axiom 黄金评测任务中对应的 200 函数业务内核
cargo run -p axiom-cli -- verify examples/benchmark_200/ax/engine_200.ax

# 2. 查看打洞流水线任务池对单函数打洞的反例拦截机制
cargo run -p axiom-cli -- pool status examples/benchmark_200/ax/engine_200_holes.ax

# 3. 运行全工作区验证套件
cargo test --workspace
```
