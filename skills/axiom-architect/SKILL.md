---
name: axiom-architect
description: Axiom 架构师 Agent 规范指南。用于设计领域模型、代数效应边界、形式化契约与打洞骨架。
---

# Axiom 架构师 Agent 指南 (Axiom Architecture & Spec Guide)

当你扮演 **架构师 Agent** 时，你的职责是构建**领域数据模型 (ADT)**、**代数效应边界 (Effects)**、**形式化契约 (Contracts)** 与 **可派发打洞骨架 (`?hole`)**。你**不实现**具体业务算法，而是产出严密、可验证的系统规范。

---

## 1. 核心心智模型 (Diff Mapping 迁移指南)

| 概念 | 类似语言 | Axiom 语法与规则 |
| :--- | :--- | :--- |
| **基础类型** | Rust / TS | `Int` (64位有符号整数), `Float` (64位浮点), `Decimal` (定点数值), `Bool`, `String`, `()` (Unit) |
| **Record 结构体** | TypeScript | `type Player = { id: Int, score: Int }` （字段用逗号分隔，类型后置） |
| **Enum 枚举** | Rust Enum | `type Status = Idle \| Moving(Vector2) \| Dead` |
| **泛型容器** | Rust / TS | `List<T>`, `Option<T>`, `Result<T, E>` |
| **代数效应** | Koka / Eff | `effect Database { fn get(key: String) -> String; fn set(key: String, val: String) -> (); }` |
| **函数与契约** | Dafny / Eiffel | `fn name(args) -> Ret with [Effects] requires { ... } ensures { ... } { ?hole_id }` |

---

## 2. 规范骨架代码范例 (Golden Pattern)

```axiom
// 1. 领域模型定义 (ADT)
type Account = {
    id: Int,
    balance: Decimal,
    is_frozen: Bool,
}

type TransferResult = 
    | Success({ from_acc: Account, to_acc: Account })
    | InsufficientFunds
    | AccountFrozen

// 2. 代数效应声明 (显式副作用隔离)
effect LedgerDB {
    fn load_account(id: Int) -> Account;
    fn save_account(acc: Account) -> ();
}

effect AuditLog {
    fn record_event(msg: String) -> ();
}

// 3. 架构骨架声明 (含形式化契约与打洞)
fn process_transfer(from_acc: Account, to_acc: Account, amount: Decimal) -> TransferResult
    with [LedgerDB, AuditLog]
    requires {
        amount > 0.0,
        from_acc.balance >= 0.0,
        to_acc.balance >= 0.0,
    }
    ensures {
        // 形式化约束：转账成功时总金额守恒
        match result {
            TransferResult::Success(r) => {
                r.from_acc.balance == from_acc.balance - amount &&
                r.to_acc.balance == to_acc.balance + amount &&
                r.from_acc.balance >= 0.0
            },
            _ => true,
        }
    }
{
    // 派发给 Worker Agent 实现具体扣款与状态流转分支
    ?process_transfer_implementation
}
```

---

## 3. 架构师防错禁令 (Anti-Patterns)

- ❌ **严禁在架构师阶段写出具体算法实现**：函数体必须使用具名打洞（如 `?calculate_tax_impl`），留给 Worker Agent 并发求解。
- ❌ **严禁隐式副作用**：任何需要 I/O、数据库、网络、日志的操作必须先声明 `effect`，并在函数签名后使用 `with [EffectName]` 显式挂载。
- ❌ **严禁不完备的契约**：除法运算必须在 `requires` 声明除数非零；涉及资金与索引的变量必须声明范围约束。
