# 🤖 Axiom AI Agent 技能包 (Axiom Skills Package)

> 本目录包含了为现代 AI Agent（如 Antigravity, Cursor, Windsurf, Claude Code, OpenAI Codex, Aider 等）专门设计的 5 大 Axiom 核心协同技能。

---

## 📦 技能套件清单 (Skills Inventory)

| 技能目录 | 技能定位 | 触发指令 / 适用场景 | 核心职责 |
| :--- | :--- | :--- | :--- |
| **`axiom-pipeline`** | 全自动打洞流水线 | `/axiom-pipeline` | 自动串联 Architect 骨架设计 -> 任务机械化拆解 -> Worker 并发填洞 -> SMT 形式化验证 -> Healer 自愈全流程。 |
| **`axiom-interactive`** | 分阶段人机协作打洞 | `/axiom-interactive` | **Stage 1** 生成形式化数学契约与打洞骨架并暂停等待人类确认；**Stage 2** 确认后极速填洞与证明交付。 |
| **`axiom-architect`** | 架构师 Agent 规范 | 架构设计阶段 | 设计领域不可变模型 (ADT)、代数效应 (Effect)、形式化契约与 `?hole` 骨架。 |
| **`axiom-worker`** | Worker 填洞极速指南 | 填洞实现阶段 | 根据 `hole-task.json` 编写 $O(1)$ 无副作用的纯表达式实现。 |
| **`axiom-healer`** | SMT 反例自愈指南 | 契约违背阶段 | 解析 Diagnostic JSON 中的具体输入反例（Counter-Example），单步修复边界条件漏洞。 |

---

## 🛠️ 各主流 AI Agent 安装与配置指南

### 1. 🌟 Google Antigravity / Gemini CLI
将 `skills/` 目录下的所有子文件夹复制到全局或项目配置目录：
- **全局安装**：
  ```bash
  # Windows
  xcopy /E /I skills\* "%USERPROFILE%\.gemini\config\skills\"
  
  # macOS / Linux
  cp -r skills/* ~/.gemini/config/skills/
  ```
- **项目级安装**：直接放在项目根目录 `.gemini/config/skills/` 下。
- **使用**：在聊天窗口直接输入 `/axiom-pipeline 实现一个银行转账系统`。

---

### 2. ⚡ Cursor IDE
Cursor 支持通过 `.cursor/rules/` 或 `.cursorrules` 加载系统规则：
- **方式一（推荐 Rules 目录）**：
  ```bash
  # 在项目根目录下创建 .cursor/rules 并软链接或复制
  mkdir -p .cursor/rules
  cp skills/axiom-pipeline/SKILL.md .cursor/rules/axiom-pipeline.mdc
  cp skills/axiom-interactive/SKILL.md .cursor/rules/axiom-interactive.mdc
  ```
- **方式二（`.cursorrules` 文件）**：将 `skills/axiom-pipeline/SKILL.md` 的内容直接追加到项目根目录的 `.cursorrules` 文件中。
- **使用**：在 Cursor Chat (Composer / Agent 模式) 中输入：`请遵循 axiom-pipeline 规范，为我设计并实现业务功能`。

---

### 3. 🌊 Windsurf (Codeium)
- 将 `skills/` 目录的内容复制到项目根目录的 `.windsurfrules` 中：
  ```bash
  cat skills/axiom-pipeline/SKILL.md >> .windsurfrules
  ```
- **使用**：在 Cascade 聊天框中输入：`使用 Axiom 全自动打洞流水线实现 ...`。

---

### 4. 🧠 Claude Code / Claude Desktop
- **Claude Code CLI**：
  将各技能的规则添加至项目根目录的 `CLAUDE.md`：
  ```bash
  cat skills/axiom-pipeline/SKILL.md >> CLAUDE.md
  ```
- **Claude Desktop (Projects)**：
  在 Claude.ai Project 的 **Project Knowledge** 中上传 `skills/` 目录下的所有 `SKILL.md` 文件。
- **使用**：直接提示 Claude：`请扮演 Axiom 架构师，启动 /axiom-pipeline`。

---

### 5. 🤖 OpenAI Codex CLI / Aider / 通用 LLM 客户端
- 在任何 Agent 的系统提示词（System Prompt）或上下文引导中，引入 `skills/axiom-pipeline/SKILL.md` 与 `skills/axiom-worker/SKILL.md` 的内容即可。
