> **Languages / 语言 / 語言**: [English](../en/SPECIFICATION.md) | [简体中文](../zh-CN/SPECIFICATION.md) | [繁體中文](../zh-TW/SPECIFICATION.md)

---

# Axiom 语言语法与语义规范草案 (Language Specification v0.1)

---

## 1. 基础语法与词法规则

- **注释**：单行 `//`，多行 `/* ... */`
- **标识符**：小写驼峰或蛇形命名 `variable_name`，类型与 Effect 采用大驼峰 `TypeName`
- **代码块**：强制使用大括号 `{ ... }`，分号 `;` 为语句分隔符（代码块最后一个表达式自动作为返回值）。

---

## 2. 类型系统 (Type System)

Axiom 拥有严格的静态类型系统，所有数据均为**不可变（Immutable）**。

### 2.1 原生类型 (Primitive Types)
- `Int` (64位有符号整数)
- `Float` (64位浮点数)
- `Decimal` (高精度定点/货币数)
- `Bool` (`true` | `false`)
- `String` (UTF-8 不可变文本)
- `()` (Unit 类型)

### 2.2 代数数据类型 (ADT)

#### 记录类型 (Product Type / Record)
```rust
type Point = {
    x: Float,
    y: Float,
}

// 不可变更新 (Record Update)
let p1 = Point { x: 1.0, y: 2.0 };
let p2 = { ...p1, x: 3.0 }; // 生成新对象，不修改 p1
```

#### 和类型 (Sum Type / Enum)
```rust
type Shape =
    | Circle(radius: Float)
    | Rectangle(width: Float, height: Float)
    | Point
```

#### 泛型标准库类型
```rust
type Option<T> =
    | Some(value: T)
    | None

type Result<T, E> =
    | Ok(data: T)
    | Err(error: E)
```

---

## 3. 代数效应系统 (Algebraic Effect System)

Axiom 的所有外部交互必须通过显式定义的 `effect` 进行。

### 3.1 定义 Effect
```rust
effect FileSystem {
    fn read_file(path: String): Result<String, IoError>
    fn write_file(path: String, content: String): Result<(), IoError>
}
```

### 3.2 声明函数依赖的 Effect
```rust
fn load_config(path: String): Result<Config, ConfigError> 
    with [FileSystem] 
{
    let content = match FileSystem.read_file(path) {
        Ok(c) => c,
        Err(e) => return Err(ConfigError.IoFailure(e)),
    };
    parse_config(content)
}
```

---

## 4. 契约驱动设计 (Design by Contract)

函数可包含形式化断言：
- `requires { ... }`：前置条件，调用者必须保证；
- `ensures { ... }`：后置条件，函数实现必须保证，可访问特殊变量 `result`。

```rust
fn divide(numerator: Float, denominator: Float) -> Float
    requires {
        denominator != 0.0,
    }
    ensures {
        // 后置检查
        result * denominator == numerator
    }
{
    numerator / denominator
}
```

---

## 5. 打洞协同语法 (Hole-Driven Syntax)

在函数实现未完成时，使用 `?hole_identifier` 占位：

```rust
fn calculate_tax(income: Decimal, rate: Decimal) -> Decimal
    requires {
        income >= 0.0,
        rate >= 0.0 && rate <= 1.0,
    }
    ensures {
        result >= 0.0
    }
{
    ?tax_calculator_logic
}
```

### 编译器生成的打洞任务描述协议 (JSON Schema)
```json
{
  "$schema": "https://axiom-lang.org/schemas/hole-task.json",
  "hole_id": "tax_calculator_logic",
  "target_function": "calculate_tax",
  "parameters": [
    { "name": "income", "type": "Decimal" },
    { "name": "rate", "type": "Decimal" }
  ],
  "return_type": "Decimal",
  "allowed_effects": [],
  "contracts": {
    "requires": ["income >= 0.0", "rate >= 0.0 && rate <= 1.0"],
    "ensures": ["result >= 0.0"]
  }
}
```

---

## 6. 模式匹配与控制流 (Pattern Matching)

所有的分支判断优先采用穷尽模式匹配（Exhaustive Matching）：

```rust
fn area(shape: Shape) -> Float {
    match shape {
        Shape.Circle(r) => 3.14159265 * r * r,
        Shape.Rectangle(w, h) => w * h,
        Shape.Point => 0.0,
    }
}
```
