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
