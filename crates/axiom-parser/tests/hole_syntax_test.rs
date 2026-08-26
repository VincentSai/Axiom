use axiom_ast::Expr;
use axiom_parser::{parse_expr, parse_module};

#[test]
fn parses_named_hole_expression() {
    let expression = parse_expr("?tax_calculator_logic").expect("the hole should parse");

    assert_eq!(expression, Expr::Hole("tax_calculator_logic".into()));
}

#[test]
fn parses_hole_as_function_body() {
    let module =
        parse_module("fn calculate_tax(income: Decimal) -> Decimal { ?tax_calculator_logic }")
            .expect("the function containing a hole should parse");

    assert_eq!(
        module.functions[0].body,
        Expr::Hole("tax_calculator_logic".into())
    );
}
