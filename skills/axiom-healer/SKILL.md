---
name: axiom-healer
description: Axiom SMT 反例自愈 Agent 指南。用于解析 DiagnosticReport JSON 并依据具体反例数值单步修复代码。
---

# Axiom SMT 反例自愈指南 (Counter-Example Auto-Healing Guide)

当你扮演 **自愈 Agent (Healer)** 时，你接收到 `axiom verify --json` 产生的 `DiagnosticReport`。你的职责是**根据具体反例数值，对函数体注入防御性分支或修复计算，单步达成 `[VERIFIED]`**。

---

## 1. 结构化诊断报告解读 (DiagnosticReport Schema)

```json
{
  "file": "transfer.ax",
  "function_name": "safe_withdraw",
  "status": "CounterExampleFound",
  "diagnosis": "Contract violation in safe_withdraw",
  "counter_example": {
    "balance": 100.0,
    "amount": 150.0
  },
  "violated_clause": "ensures: result >= 0.0",
  "suggested_action": "Add guard conditions or constrain arithmetic"
}
```

---

## 2. 四步自愈思维链 (4-Step CoT SOP)

1. **反例代入演算 (Trace)**：
   - 显式将反例值（`balance = 100.0, amount = 150.0`）代入现有函数体；
   - 发现直接计算 `balance - amount` 得到 `-50.0`，违背了后置契约 `result >= 0.0`。
2. **契约锁定铁律 (Contract Lock)**：
   - **绝对禁止修改函数签名与 `requires` / `ensures` 契约**。你的修改范围仅限函数体内部。
3. **守卫注入 / 分支修补 (Guard Injection)**：
   - 针对反例触发的边界，增加条件分支或模式匹配。
4. **单步提交验证 (Re-Verify)**：
   - 将修复后的函数体交由 SMT 求解器再次验证。

---

## 3. 典型反例与修复范例

### 范例 1：透支与下溢 (Underflow / Overdraft)
- **反例**：`balance = 10.0, amount = 20.0` 导致结果负数。
- **原错误代码**：
  ```axiom
  balance - amount
  ```
- **自愈修复代码**：
  ```axiom
  if amount <= balance {
      balance - amount
  } else {
      balance
  }
  ```

### 范例 2：除以零 (Division by Zero)
- **反例**：`b = 0.0`
- **自愈修复代码**：
  ```axiom
  if b != 0.0 {
      a / b
  } else {
      0.0
  }
  ```
