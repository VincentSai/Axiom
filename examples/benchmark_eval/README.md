# 🔬 Axiom A/B 单一变量科学对照评测套件 (A/B Benchmark Eval Suite)

> **目标**：以严肃的科学实证方法，严格控制基座模型与业务逻辑单一变量，量化评估在 AI 自动编码场景下，**Axiom 契约打洞流水线 vs 传统语言大文件工作流 vs 传统语言单函数微任务工作流** 在可靠性、缺陷漏检率、修复效率与 Token 消耗上的本质差异。

---

## 📂 目录导航与核心资产

| 资产文件 | 类型 | 说明描述 |
| :--- | :---: | :--- |
| [`golden_tasks.json`](./golden_tasks.json) | **数据集** | 20 个跨越 5 层依赖拓扑的黄金业务逻辑标准评测任务规范 |
| [`protocols/prompt_templates.md`](./protocols/prompt_templates.md) | **实验协议** | 三组对照实验（大文件 / 单函数微任务 / Axiom 打洞）的提示词模板 |
| [`eval_runner.py`](./eval_runner.py) | **驱动脚本** | 调用 Google Gemini 官方 API（`countTokens` / `generateContent`）的纯标准库测算器 |
| [`data/eval_matrix_data.json`](./data/eval_matrix_data.json) | **实测数据** | 由 `eval_runner.py` 直接沉淀的官方 API 真实 Token 与验证数据 |
| [`EVALUATION_REPORT.md`](./EVALUATION_REPORT.md) | **方法报告** | 评测架构、变量解耦设计与复现分析报告 |

---

## 🖥️ 真实 API 测算运行指南

本套件包含基于官方 Google Gemini API 的执行脚本，**绝无任何伪造数据**：

```powershell
# 1. 设置环境变量（或使用 --api-key 参数）
$env:GEMINI_API_KEY = "YOUR_GEMINI_API_KEY"

# 2. 运行官方精确 Token 测算（0 生成成本，毫秒级获取官方分词计数）
python examples/benchmark_eval/eval_runner.py --mode count

# 3. 运行完整 AI 生成与 SMT / 编译验证
python examples/benchmark_eval/eval_runner.py --mode generate --tasks 5
```
