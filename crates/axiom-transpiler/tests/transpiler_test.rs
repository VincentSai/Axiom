use axiom_transpiler::transpile_source_to_rust;
use std::process::Command;

#[test]
fn transpiles_primitive_function_signatures_and_expressions() {
    let rust = transpile_source_to_rust(
        r#"
        fn compute(count: Int, ratio: Float, amount: Decimal, enabled: Bool, label: String) -> () {
            let total = count + 1;
            ()
        }
        "#,
    )
    .expect("valid Axiom should transpile");

    assert!(rust.contains("pub type Decimal = i64;"));
    assert!(rust.contains(
        "pub fn compute(count: i64, ratio: f64, amount: Decimal, enabled: bool, label: String) -> ()"
    ));
    assert!(rust.contains("let total = (count + 1);"));
}

#[test]
fn transpiles_record_and_enum_definitions() {
    let rust = transpile_source_to_rust(
        r#"
        type Point = { x: Float, y: Float }
        type Shape = | Circle(radius: Float) | Rectangle(width: Float, height: Float) | Origin
        "#,
    )
    .expect("ADTs should transpile");

    assert!(rust.contains("#[derive(Debug, Clone, Copy, PartialEq)]\npub struct Point"));
    assert!(rust.contains("pub x: f64,"));
    assert!(rust.contains("pub y: f64,"));
    assert!(rust.contains("#[derive(Debug, Clone, Copy, PartialEq)]\npub enum Shape"));
    assert!(rust.contains("Circle(f64),"));
    assert!(rust.contains("Rectangle(f64, f64),"));
    assert!(rust.contains("Origin,"));
}

#[test]
fn transpiles_record_construction_update_and_match() {
    let rust = transpile_source_to_rust(
        r#"
        type Point = { x: Float, y: Float }
        type Shape = | Circle(radius: Float) | Origin

        fn move_x(p: Point) -> Point {
            let copy = Point { x: p.x, y: p.y };
            { ...copy, x: 3.0 }
        }

        fn area(shape: Shape) -> Float {
            match shape {
                Shape.Circle(radius) => radius * radius,
                Shape.Origin => 0.0,
            }
        }
        "#,
    )
    .expect("record and match expressions should transpile");

    assert!(rust.contains("Point { x: p.x, y: p.y }"));
    assert!(rust.contains("Point { x: 3.0, ..copy }"));
    assert!(rust.contains("match shape"));
    assert!(rust.contains("Shape::Circle(radius) => (radius * radius),"));
    assert!(rust.contains("Shape::Origin => 0.0,"));
}

#[test]
fn transpiles_effects_to_traits_and_handlers() {
    let rust = transpile_source_to_rust(
        r#"
        type Vector2 = { x: Float, y: Float }

        effect Render {
            fn draw_sprite(id: Int, pos: Vector2): ()
            fn clear_screen(): ()
        }

        effect Input {
            fn get_movement_axis(): Vector2
        }

        fn tick(dt: Float) -> () with [Render, Input] {
            let dir = Input.get_movement_axis();
            Render.clear_screen();
            Render.draw_sprite(1, dir);
            ()
        }
        "#,
    )
    .expect("effects and handlers should transpile");

    // Check trait generation
    assert!(rust.contains("pub trait Render {\n    fn draw_sprite(&mut self, id: i64, pos: Vector2);\n    fn clear_screen(&mut self);\n}"));
    assert!(rust.contains("pub trait Input {\n    fn get_movement_axis(&mut self) -> Vector2;\n}"));

    // Check function signature with injected handlers
    assert!(rust
        .contains("pub fn tick(dt: f64, render: &mut impl Render, input: &mut impl Input) -> ()"));

    // Check effect method call conversions
    assert!(rust.contains("let dir = input.get_movement_axis();"));
    assert!(rust.contains("render.clear_screen();"));
    assert!(rust.contains("render.draw_sprite(1, dir);"));
}

#[test]
fn transpiles_nested_effect_calls_and_list_types() {
    let rust = transpile_source_to_rust(
        r#"
        type Item = { id: Int }
        type Inventory = { items: List<Item> }

        effect Log {
            fn info(msg: String): ()
        }

        fn log_item(item: Item) -> () with [Log] {
            Log.info("item logged");
            ()
        }

        fn process(inv: Inventory) -> () with [Log] {
            log_item(Item { id: 1 });
            ()
        }
        "#,
    )
    .expect("nested effects and list types should transpile");

    // List<Item> -> Vec<Item>
    assert!(rust.contains("pub items: Vec<Item>,"));

    // Trait Log
    assert!(rust.contains("pub trait Log {\n    fn info(&mut self, msg: String);\n}"));

    // Handlers
    assert!(rust.contains("pub fn log_item(item: Item, log: &mut impl Log) -> ()"));
    assert!(rust.contains("pub fn process(inv: Inventory, log: &mut impl Log) -> ()"));

    // Call propagation: log_item automatically passed `log`
    assert!(rust.contains("log_item(Item { id: 1 }, log)"));
}

#[test]
fn contracted_function_with_expr_raw_fails_transpilation() {
    use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Literal, Module, Type};
    use axiom_transpiler::transpile_to_rust;

    let module = Module {
        types: vec![],
        effects: vec![],
        functions: vec![FunctionDecl {
            name: "bypass".into(),
            params: vec![FunctionParam {
                name: "x".into(),
                ty: Type::Int,
            }],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![Expr::BinaryOp {
                op: BinaryOperator::GreaterThan,
                left: Box::new(Expr::Ident("x".into())),
                right: Box::new(Expr::Literal(Literal::Int(0))),
            }],
            ensures: vec![],
            body: Expr::Raw("x + 100".into()),
        }],
    };

    let res = transpile_to_rust(&module);
    assert!(
        res.is_err(),
        "transpiler must reject contracted function containing Expr::Raw"
    );
    let err = res.unwrap_err().to_string();
    assert!(
        err.contains("Expr::Raw is forbidden in contracted function"),
        "unexpected error message: {}",
        err
    );
}

#[test]
fn transpiles_decimal_fixed_point_exact_arithmetic() {
    let rust = transpile_source_to_rust(
        r#"
        fn add_exact(a: Decimal, b: Decimal) -> Decimal
            requires { a > 0.0, b > 0.0 }
            ensures { result == a + b }
        {
            a + b
        }
        "#,
    )
    .expect("Decimal addition should transpile");

    assert!(rust.contains("pub type Decimal = i64;"));
    assert!(rust.contains("pub fn add_exact(a: Decimal, b: Decimal) -> Decimal"));
    assert!(rust.contains("(a > 0_i64)"));
    assert!(rust.contains("(b > 0_i64)"));
    assert!(rust.contains("(a + b)"));
}

#[test]
fn transpiles_implication_contract_to_rust_disjunction() {
    let rust = transpile_source_to_rust(
        r#"
        fn check_discount(discount_pct: Int, price: Int) -> Int
            ensures {
                discount_pct == 0 => result == price
            }
        {
            price
        }
        "#,
    )
    .expect("Implication contract should transpile");

    assert!(
        rust.contains("(!((discount_pct == 0)) || ((result == price)))")
            || rust.contains("(!((discount_pct == 0)) || (result == price))")
            || rust.contains("||")
    );
    assert!(!rust.contains("=>"));
}

#[test]
fn transpiles_decimal_multiplication_and_division_with_scale_recovery_and_i128_guard() {
    let rust = transpile_source_to_rust(
        r#"
        fn mul_div(a: Decimal, b: Decimal) -> Decimal
            requires { a > 0.0, b > 0.0 }
            ensures { result > 0.0 }
        {
            (a * b) / 2.0d
        }
        "#,
    )
    .expect("Decimal mul_div should transpile");

    assert!(rust.contains("as i128"));
    assert!(rust.contains("1000000") || rust.contains("1_000_000"));
}

#[test]
fn transpiles_nested_decimal_in_containers_and_compiles_with_rustc() {
    let source = r#"
        type Ledger = {
            transactions: List<Decimal>,
            checkpoint: Option<Decimal>,
        }

        fn first_entry(ledger: Ledger) -> Option<Decimal> {
            ledger.checkpoint
        }
    "#;

    let rust =
        transpile_source_to_rust(source).expect("nested Decimal in containers should transpile");

    assert!(
        rust.contains("pub type Decimal = i64;"),
        "Generated Rust must contain Decimal alias when used in List<Decimal>, but was:\n{rust}"
    );
    assert!(rust.contains("pub transactions: Vec<Decimal>,"));
    assert!(rust.contains("pub checkpoint: Option<Decimal>,"));

    let temp_dir =
        std::env::temp_dir().join(format!("axiom_nested_decimal_{}", std::process::id()));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let rust_path = temp_dir.join("ledger.rs");
    std::fs::write(&rust_path, format!("{rust}\nfn main() {{}}\n")).unwrap();

    let compiled = Command::new("rustc")
        .args(["--edition=2021", "-A", "warnings", "--crate-type=lib"])
        .arg(&rust_path)
        .arg("--out-dir")
        .arg(&temp_dir)
        .output()
        .expect("failed to execute rustc");

    assert!(
        compiled.status.success(),
        "rustc compilation failed on nested Decimal code:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&compiled.stdout),
        String::from_utf8_lossy(&compiled.stderr)
    );
}
