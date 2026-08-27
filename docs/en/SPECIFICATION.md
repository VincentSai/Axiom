> **Languages / 语言 / 語言**: [English](../en/SPECIFICATION.md) | [简体中文](../zh-CN/SPECIFICATION.md) | [繁體中文](../zh-TW/SPECIFICATION.md)

---

# Axiom Language Syntax & Semantics Specification (Draft v0.1)

---

## 1. Lexical Conventions & Syntax Basics

- **Comments**: Single-line `//`, block `/* ... */`
- **Identifiers**: Lower camelCase or snake_case for variables (`variable_name`), UpperCamelCase for types and effects (`TypeName`).
- **Blocks**: Explicit curly braces `{ ... }`. Semicolons `;` separate statements; the trailing expression in a block serves as its return value.

---

## 2. Type System

Axiom features a strict static type system where all data structures are **immutable by default**.

### 2.1 Primitive Types
- `Int`: 64-bit signed integer
- `Float`: 64-bit floating point number
- `Decimal`: Exact fixed-point numerical type for financial logic
- `Bool`: `true` | `false`
- `String`: Immutable UTF-8 text
- `()`: Unit type

### 2.2 Algebraic Data Types (ADTs)

#### Record Types (Product Types)
```rust
type Point = {
    x: Float,
    y: Float,
}

// Immutable Record Update
let p1 = Point { x: 1.0, y: 2.0 };
let p2 = { ...p1, x: 3.0 }; // Returns a new instance without mutating p1
```

#### Variant Types (Sum Types / Enums)
```rust
type Shape =
    | Circle(radius: Float)
    | Rectangle(width: Float, height: Float)
    | Point
```

#### Standard Library Generic Types
```rust
type Option<T> =
    | Some(value: T)
    | None

type Result<T, E> =
    | Ok(data: T)
    | Err(error: E)
```

---

## 3. Algebraic Effect System

All interactions with external systems must occur via explicitly declared `effect` blocks.

### 3.1 Defining an Effect
```rust
effect FileSystem {
    fn read_file(path: String): Result<String, IoError>
    fn write_file(path: String, content: String): Result<(), IoError>
}
```

### 3.2 Declaring Effect Dependencies on Functions
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

## 4. Design by Contract

Functions can enforce formal contract clauses:
- `requires { ... }`: Preconditions enforced upon the caller.
- `ensures { ... }`: Postconditions guaranteed by the implementation, referencing the returned `result`.

```rust
fn divide(numerator: Float, denominator: Float) -> Float
    requires {
        denominator != 0.0,
    }
    ensures {
        result * denominator == numerator
    }
{
    numerator / denominator
}
```

---

## 5. Hole-Driven Syntax

Use `?hole_identifier` as an executable placeholder for unfinished logic:

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

### Compiler-Generated Hole Task Schema (JSON)
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

## 6. Pattern Matching

All conditional branching favors exhaustive pattern matching:

```rust
fn area(shape: Shape) -> Float {
    match shape {
        Shape.Circle(r) => 3.14159265 * r * r,
        Shape.Rectangle(w, h) => w * h,
        Shape.Point => 0.0,
    }
}
```
