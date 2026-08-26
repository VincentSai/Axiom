use axiom_transpiler::transpile_source_to_rust;

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

    assert!(rust.contains(
        "pub fn compute(count: i64, ratio: f64, amount: f64, enabled: bool, label: String) -> ()"
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

    assert!(rust.contains("#[derive(Debug, Clone, PartialEq)]\npub struct Point"));
    assert!(rust.contains("pub x: f64,"));
    assert!(rust.contains("pub y: f64,"));
    assert!(rust.contains("#[derive(Debug, Clone, PartialEq)]\npub enum Shape"));
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
