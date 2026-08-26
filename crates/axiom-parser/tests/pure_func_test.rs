use axiom_ast::{BinaryOperator, Expr, Literal, Type, UnaryOperator};
use axiom_parser::{parse_expr, parse_module};

#[test]
fn parses_simple_arithmetic_function() {
    let module =
        parse_module("fn add(a: Int, b: Int) -> Int { a + b }").expect("the function should parse");

    assert_eq!(module.functions.len(), 1);
    let function = &module.functions[0];
    assert_eq!(function.name, "add");
    assert_eq!(function.params.len(), 2);
    assert_eq!(function.params[0].name, "a");
    assert_eq!(function.params[0].ty, Type::Int);
    assert_eq!(function.params[1].name, "b");
    assert_eq!(function.params[1].ty, Type::Int);
    assert_eq!(function.return_type, Type::Int);
    assert_eq!(
        function.body,
        Expr::BinaryOp {
            left: Box::new(Expr::Ident("a".into())),
            op: BinaryOperator::Add,
            right: Box::new(Expr::Ident("b".into())),
        }
    );
}

#[test]
fn parses_nested_arithmetic_with_precedence() {
    let module = parse_module("fn calc(x: Float, y: Float) -> Float { (x + 2.0) * y - 5.0 }")
        .expect("the function should parse");

    let function = &module.functions[0];
    assert_eq!(function.name, "calc");
    assert_eq!(function.return_type, Type::Float);
    assert_eq!(
        function.body,
        Expr::BinaryOp {
            left: Box::new(Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Ident("x".into())),
                    op: BinaryOperator::Add,
                    right: Box::new(Expr::Literal(Literal::Float(2.0))),
                }),
                op: BinaryOperator::Multiply,
                right: Box::new(Expr::Ident("y".into())),
            }),
            op: BinaryOperator::Subtract,
            right: Box::new(Expr::Literal(Literal::Float(5.0))),
        }
    );
}

#[test]
fn parses_let_bindings_and_block_result() {
    let module = parse_module("fn test_let() -> Int { let a = 10; let b = 20; a + b }")
        .expect("the function should parse");

    assert_eq!(
        module.functions[0].body,
        Expr::Block {
            statements: vec![
                Expr::Let {
                    name: "a".into(),
                    value: Box::new(Expr::Literal(Literal::Int(10))),
                },
                Expr::Let {
                    name: "b".into(),
                    value: Box::new(Expr::Literal(Literal::Int(20))),
                },
            ],
            tail: Some(Box::new(Expr::BinaryOp {
                left: Box::new(Expr::Ident("a".into())),
                op: BinaryOperator::Add,
                right: Box::new(Expr::Ident("b".into())),
            })),
        }
    );
}

#[test]
fn parses_boolean_comparison_function() {
    let module = parse_module("fn is_positive(n: Int) -> Bool { n > 0 }")
        .expect("the function should parse");

    assert_eq!(module.functions[0].return_type, Type::Bool);
    assert_eq!(
        module.functions[0].body,
        Expr::BinaryOp {
            left: Box::new(Expr::Ident("n".into())),
            op: BinaryOperator::GreaterThan,
            right: Box::new(Expr::Literal(Literal::Int(0))),
        }
    );
}

#[test]
fn reports_illegal_token_location() {
    let error = parse_module("fn bad() -> Int { @ }").expect_err("`@` must be rejected");

    assert_eq!(error.line, 1);
    assert_eq!(error.column, 19);
    assert_eq!(error.message, "unexpected token `@`");
}

#[test]
fn reports_missing_closing_parenthesis() {
    let error = parse_expr("(1 + 2").expect_err("the closing parenthesis is required");

    assert_eq!(error.line, 1);
    assert_eq!(error.column, 7);
    assert_eq!(error.message, "expected `)`");
}

#[test]
fn ignores_comments_and_parses_multiple_functions() {
    let source = r#"
        // The first declaration uses a line comment.
        fn first() -> Int { 1 }

        /* The second declaration uses
           a block comment. */
        fn second() -> Int { 2 }
    "#;

    let module = parse_module(source).expect("comments should be ignored");

    assert_eq!(module.functions.len(), 2);
    assert_eq!(module.functions[0].name, "first");
    assert_eq!(module.functions[1].name, "second");
}

#[test]
fn parses_calls_unary_operators_and_core_literals() {
    let expr = parse_expr(r#"notify(-amount / 2, !false, "ok", ())"#)
        .expect("the expression should parse");

    assert_eq!(
        expr,
        Expr::Call {
            callee: Box::new(Expr::Ident("notify".into())),
            args: vec![
                Expr::BinaryOp {
                    left: Box::new(Expr::UnaryOp {
                        op: UnaryOperator::Negate,
                        operand: Box::new(Expr::Ident("amount".into())),
                    }),
                    op: BinaryOperator::Divide,
                    right: Box::new(Expr::Literal(Literal::Int(2))),
                },
                Expr::UnaryOp {
                    op: UnaryOperator::Not,
                    operand: Box::new(Expr::Literal(Literal::Bool(false))),
                },
                Expr::Literal(Literal::String("ok".into())),
                Expr::Literal(Literal::Unit),
            ],
        }
    );
}

#[test]
fn parses_decimal_string_unit_and_custom_types() {
    let module = parse_module(
        r#"
            fn quote(amount: Decimal, currency: Currency) -> String { "ok" }
            fn noop() -> () { () }
        "#,
    )
    .expect("all core and custom types should parse");

    assert_eq!(module.functions[0].params[0].ty, Type::Decimal);
    assert_eq!(
        module.functions[0].params[1].ty,
        Type::Custom("Currency".into())
    );
    assert_eq!(module.functions[0].return_type, Type::String);
    assert_eq!(module.functions[1].return_type, Type::Unit);
}

#[test]
fn separates_block_statements_and_preserves_tail_expression() {
    let module = parse_module("fn sequence() -> Int { ping(); log(); 42 }")
        .expect("semicolon-separated statements should parse");

    assert_eq!(
        module.functions[0].body,
        Expr::Block {
            statements: vec![
                Expr::Call {
                    callee: Box::new(Expr::Ident("ping".into())),
                    args: vec![],
                },
                Expr::Call {
                    callee: Box::new(Expr::Ident("log".into())),
                    args: vec![],
                },
            ],
            tail: Some(Box::new(Expr::Literal(Literal::Int(42)))),
        }
    );
}

#[test]
fn treats_explicit_return_as_block_result() {
    let module = parse_module("fn answer() -> Int { return 42; }")
        .expect("an explicit return value should parse");

    assert_eq!(module.functions[0].body, Expr::Literal(Literal::Int(42)));
}
