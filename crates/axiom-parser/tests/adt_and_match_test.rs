#![allow(clippy::approx_constant)]

use axiom_ast::{
    BinaryOperator, Expr, FieldDef, Literal, MatchArm, Pattern, Type, TypeDef, VariantDef,
};
use axiom_parser::{parse_expr, parse_module};

#[test]
fn parses_record_type_definition() {
    let module =
        parse_module("type Point = { x: Float, y: Float }").expect("the record type should parse");

    assert_eq!(
        module.types,
        vec![TypeDef::Record {
            name: "Point".into(),
            fields: vec![
                FieldDef {
                    name: "x".into(),
                    ty: Type::Float,
                },
                FieldDef {
                    name: "y".into(),
                    ty: Type::Float,
                },
            ],
        }]
    );
    assert!(module.functions.is_empty());
}

#[test]
fn parses_enum_type_with_payload_and_unit_variants() {
    let module = parse_module(
        "type Shape = | Circle(radius: Float) | Rectangle(width: Float, height: Float) | Point",
    )
    .expect("the enum type should parse");

    assert_eq!(
        module.types,
        vec![TypeDef::Enum {
            name: "Shape".into(),
            variants: vec![
                VariantDef {
                    name: "Circle".into(),
                    fields: vec![FieldDef {
                        name: "radius".into(),
                        ty: Type::Float,
                    }],
                },
                VariantDef {
                    name: "Rectangle".into(),
                    fields: vec![
                        FieldDef {
                            name: "width".into(),
                            ty: Type::Float,
                        },
                        FieldDef {
                            name: "height".into(),
                            ty: Type::Float,
                        },
                    ],
                },
                VariantDef {
                    name: "Point".into(),
                    fields: vec![],
                },
            ],
        }]
    );
}

#[test]
fn parses_generic_type_annotations() {
    let module = parse_module("fn load(value: Option<T>) -> Result<String, IoError> { \"ok\" }")
        .expect("generic type annotations should parse");

    let function = &module.functions[0];
    assert_eq!(function.params[0].ty, Type::Custom("Option<T>".into()));
    assert_eq!(
        function.return_type,
        Type::Custom("Result<String, IoError>".into())
    );
}

#[test]
fn parses_chained_field_access() {
    let expr = parse_expr("entity.pos.x").expect("field access should parse");

    assert_eq!(
        expr,
        Expr::FieldAccess {
            target: Box::new(Expr::FieldAccess {
                target: Box::new(Expr::Ident("entity".into())),
                field: "pos".into(),
            }),
            field: "x".into(),
        }
    );
}

#[test]
fn parses_named_record_construction() {
    let expr = parse_expr("Point { x: 1.0, y: 2.0 }").expect("record construction should parse");

    assert_eq!(
        expr,
        Expr::RecordInit {
            ty_name: Some("Point".into()),
            fields: vec![
                ("x".into(), Expr::Literal(axiom_ast::Literal::Float(1.0))),
                ("y".into(), Expr::Literal(axiom_ast::Literal::Float(2.0))),
            ],
        }
    );
}

#[test]
fn parses_immutable_record_update() {
    let expr = parse_expr("{ ...p1, x: 3.0 }").expect("immutable record update should parse");

    assert_eq!(
        expr,
        Expr::RecordUpdate {
            base: Box::new(Expr::Ident("p1".into())),
            updates: vec![("x".into(), Expr::Literal(axiom_ast::Literal::Float(3.0)),)],
        }
    );
}

#[test]
fn parses_exhaustive_match_with_destructuring() {
    let expr =
        parse_expr("match shape { Shape.Circle(r) => 3.14 * r * r, Shape.Point => 0.0, _ => 0.0 }")
            .expect("match expression should parse");

    assert_eq!(
        expr,
        Expr::Match {
            target: Box::new(Expr::Ident("shape".into())),
            arms: vec![
                MatchArm {
                    pattern: Pattern::Variant {
                        enum_name: Some("Shape".into()),
                        variant_name: "Circle".into(),
                        fields: vec![Pattern::Ident("r".into())],
                    },
                    body: Expr::BinaryOp {
                        left: Box::new(Expr::BinaryOp {
                            left: Box::new(Expr::Literal(Literal::Float(3.14))),
                            op: BinaryOperator::Multiply,
                            right: Box::new(Expr::Ident("r".into())),
                        }),
                        op: BinaryOperator::Multiply,
                        right: Box::new(Expr::Ident("r".into())),
                    },
                },
                MatchArm {
                    pattern: Pattern::Variant {
                        enum_name: Some("Shape".into()),
                        variant_name: "Point".into(),
                        fields: vec![],
                    },
                    body: Expr::Literal(Literal::Float(0.0)),
                },
                MatchArm {
                    pattern: Pattern::Wildcard,
                    body: Expr::Literal(Literal::Float(0.0)),
                },
            ],
        }
    );
}

#[test]
fn parses_literal_patterns() {
    let expr = parse_expr(r#"match value { 0 => 1, true => 2, "ready" => 3, () => 4 }"#)
        .expect("literal patterns should parse");

    let Expr::Match { arms, .. } = expr else {
        panic!("expected a match expression");
    };
    assert_eq!(
        arms.into_iter().map(|arm| arm.pattern).collect::<Vec<_>>(),
        vec![
            Pattern::Literal(Literal::Int(0)),
            Pattern::Literal(Literal::Bool(true)),
            Pattern::Literal(Literal::String("ready".into())),
            Pattern::Literal(Literal::Unit),
        ]
    );
}
