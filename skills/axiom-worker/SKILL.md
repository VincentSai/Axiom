---
name: axiom-worker
description: Axiom Worker 填洞 Agent 极速指南。用于根据 hole-task.json 编写无副作用的纯表达式实现。
---

# Axiom Worker 极简填洞指南 (Axiom Pure Expression Guide)

当你扮演 **Worker Agent** 时，你接收到一个 `HoleTask`。你的**唯一任务**是：**编写一个返回目标类型的纯表达式（Pure Expression），严禁破坏签名，必须满足契约。**

---

## 1. 核心语法卡片 (Expression CheatSheet)

Axiom 函数体是一个**单一表达式**（最后一个分号表达式即为返回值）：

```axiom
// 局部不可变绑定与块表达式
let fee = 2.5;
let total = amount + fee;
total

// Record 构造与不可变更新 (Functional Record Update)
Account { id: 1, balance: 100.0, is_frozen: false }
Account { ..acc, balance: acc.balance - amount }

// 穷尽模式匹配 (Pattern Matching)
match state {
    Status::Moving(v) => v.x + v.y,
    Status::Idle => 0.0,
    Status::Dead => -1.0,
}
```

---

## 2. 预训练常见幻觉反例表 (Hallucination Defense)

| 常见错误幻觉 (DO NOT DO) | Axiom 正确语法 (DO THIS) | 规则解释 |
| :--- | :--- | :--- |
| `let mut x = 10; x = 20;` | `let x = 10; let x_new = 20;` | **没有 `mut` 关键字**，所有变量 100% 不可变 |
| `return result;` | 直接写末尾表达式 `result` | 推荐纯表达式结尾 |
| `acc.balance = 200.0;` | `Account { ..acc, balance: 200.0 }` | 使用 Record Update 构造新对象 |
| `fn foo() { ... }` | 仅输出函数体内容表达式 | **不要重复声明函数头和契约** |

---

## 3. 黄金准则
1. **纯粹性**：只能读取参数与局部 `let`，禁止修改外部变量。
2. **契约对齐**：若函数有 `requires { b != 0.0 }` 与 `ensures { result * b == a }`，函数体直接写 `a / b` 即可通过 SMT 证明。
