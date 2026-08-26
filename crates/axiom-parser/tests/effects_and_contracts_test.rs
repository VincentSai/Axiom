use axiom_ast::{BinaryOperator, Expr, Literal, Type};
use axiom_parser::parse_module;

#[test]
fn parses_effect_operations_with_both_return_syntaxes_and_separators() {
    let source = r#"
        effect FileSystem {
            fn read_file(path: String): Result<String, IoError>
            fn write_file(path: String, content: String) -> Result<(), IoError>;
        }
    "#;

    let module = parse_module(source).expect("the effect declaration should parse");

    assert_eq!(module.effects.len(), 1);
    let effect = &module.effects[0];
    assert_eq!(effect.name, "FileSystem");
    assert_eq!(effect.operations.len(), 2);
    assert_eq!(effect.operations[0].name, "read_file");
    assert_eq!(effect.operations[0].params.len(), 1);
    assert_eq!(effect.operations[0].params[0].name, "path");
    assert_eq!(effect.operations[0].params[0].ty, Type::String);
    assert_eq!(
        effect.operations[0].return_type,
        Type::Custom("Result<String, IoError>".into())
    );
    assert_eq!(effect.operations[1].name, "write_file");
    assert_eq!(effect.operations[1].params.len(), 2);
    assert_eq!(
        effect.operations[1].return_type,
        Type::Custom("Result<(), IoError>".into())
    );
}

#[test]
fn parses_function_effect_dependencies_and_contracts() {
    let source = r#"
        fn divide(numerator: Float, denominator: Float) -> Float
            with [FileSystem, Logger]
            requires {
                denominator != 0.0,
                numerator >= 0.0,
            }
            ensures {
                result >= 0.0,
                result * denominator == numerator,
            }
        {
            numerator / denominator
        }
    "#;

    let module = parse_module(source).expect("the annotated function should parse");
    let function = &module.functions[0];

    assert_eq!(function.effects, vec!["FileSystem", "Logger"]);
    assert_eq!(
        function.requires,
        vec![
            Expr::BinaryOp {
                left: Box::new(Expr::Ident("denominator".into())),
                op: BinaryOperator::NotEqual,
                right: Box::new(Expr::Literal(Literal::Float(0.0))),
            },
            Expr::BinaryOp {
                left: Box::new(Expr::Ident("numerator".into())),
                op: BinaryOperator::GreaterThanOrEqual,
                right: Box::new(Expr::Literal(Literal::Float(0.0))),
            },
        ]
    );
    assert_eq!(
        function.ensures,
        vec![
            Expr::BinaryOp {
                left: Box::new(Expr::Ident("result".into())),
                op: BinaryOperator::GreaterThanOrEqual,
                right: Box::new(Expr::Literal(Literal::Float(0.0))),
            },
            Expr::BinaryOp {
                left: Box::new(Expr::BinaryOp {
                    left: Box::new(Expr::Ident("result".into())),
                    op: BinaryOperator::Multiply,
                    right: Box::new(Expr::Ident("denominator".into())),
                }),
                op: BinaryOperator::Equal,
                right: Box::new(Expr::Ident("numerator".into())),
            },
        ]
    );
}

#[test]
fn parses_effects_and_contracts_from_transfer_example() {
    let source = include_str!("../../../examples/transfer.ax");

    let module = parse_module(source).expect("the transfer example should parse");

    assert_eq!(
        module
            .effects
            .iter()
            .map(|effect| effect.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Database", "Clock", "Log"]
    );
    assert_eq!(module.functions.len(), 1);
    let transfer = &module.functions[0];
    assert_eq!(transfer.name, "execute_transfer");
    assert_eq!(transfer.effects, vec!["Database", "Clock", "Log"]);
    assert_eq!(transfer.requires.len(), 2);
    assert_eq!(transfer.ensures.len(), 1);
}

#[test]
fn parses_top_level_effects_and_functions_in_any_order() {
    let source = r#"
        fn before() -> Int { 1 }
        effect Clock { fn now(): Int }
        fn after() -> Int { 2 }
        effect Logger { fn info(message: String): () }
    "#;

    let module = parse_module(source).expect("mixed declarations should parse");

    assert_eq!(
        module
            .functions
            .iter()
            .map(|function| function.name.as_str())
            .collect::<Vec<_>>(),
        vec!["before", "after"]
    );
    assert_eq!(
        module
            .effects
            .iter()
            .map(|effect| effect.name.as_str())
            .collect::<Vec<_>>(),
        vec!["Clock", "Logger"]
    );
    assert!(module.functions[0].effects.is_empty());
    assert!(module.functions[0].requires.is_empty());
    assert!(module.functions[0].ensures.is_empty());
}
