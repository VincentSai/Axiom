> **Languages / 语言 / 語言**: [English](../en/SPECIFICATION.md) | [简体中文](../zh-CN/SPECIFICATION.md) | [繁體中文](../zh-TW/SPECIFICATION.md)

---

# Axiom 語言語法與語意規範草案 (Language Specification v0.1)

---

## 1. 基礎語法與詞法規則

- **註解**：單行 `//`，多行 `/* ... */`
- **識別碼**：小寫駝峰或蛇形命名 `variable_name`，型別與 Effect 採用大駝峰 `TypeName`
- **程式碼區塊**：強制使用大括號 `{ ... }`，分號 `;` 為陳述式分隔符（區塊最後一個運算式自動作為回傳值）。

---

## 2. 型別系統 (Type System)

Axiom 擁有嚴格的靜態型別系統，所有資料均為**不可變（Immutable）**。

### 2.1 原生型別 (Primitive Types)
- `Int` (64位元有號整數)
- `Float` (64位元浮點數)
- `Decimal` (高精度定點/貨幣數)
- `Bool` (`true` | `false`)
- `String` (UTF-8 不可變文字)
- `()` (Unit 型別)

### 2.2 代數資料型別 (ADT)

#### 記錄型別 (Product Type / Record)
```rust
type Point = {
    x: Float,
    y: Float,
}

// 不可變更新 (Record Update)
let p1 = Point { x: 1.0, y: 2.0 };
let p2 = { ...p1, x: 3.0 }; // 生成新物件，不修改 p1
```

#### 和型別 (Sum Type / Enum)
```rust
type Shape =
    | Circle(radius: Float)
    | Rectangle(width: Float, height: Float)
    | Point
```

#### 泛型標準庫型別
```rust
type Option<T> =
    | Some(value: T)
    | None

type Result<T, E> =
    | Ok(data: T)
    | Err(error: E)
```

---

## 3. 代數效應系統 (Algebraic Effect System)

Axiom 的所有外部互動必須透過顯式定義的 `effect` 進行。

### 3.1 定義 Effect
```rust
effect FileSystem {
    fn read_file(path: String): Result<String, IoError>
    fn write_file(path: String, content: String): Result<(), IoError>
}
```

### 3.2 宣告函數依賴的 Effect
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

## 4. 契約驅動設計 (Design by Contract)

函數可包含形式化斷言：
- `requires { ... }`：前置條件，呼叫者必須保證；
- `ensures { ... }`：後置條件，函數實作必須保證，可存取特殊變數 `result`。

```rust
fn divide(numerator: Float, denominator: Float) -> Float
    requires {
        denominator != 0.0,
    }
    ensures {
        // 後置檢查
        result * denominator == numerator
    }
{
    numerator / denominator
}
```

---

## 5. 打洞協同語法 (Hole-Driven Syntax)

在函數實作未完成時，使用 `?hole_identifier` 佔位：

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

### 編譯器生成的打洞任務描述協定 (JSON Schema)
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

## 6. 模式匹配與控制流 (Pattern Matching)

所有的分支判斷優先採用窮盡模式匹配（Exhaustive Matching）：

```rust
fn area(shape: Shape) -> Float {
    match shape {
        Shape.Circle(r) => 3.14159265 * r * r,
        Shape.Rectangle(w, h) => w * h,
        Shape.Point => 0.0,
    }
}
```
