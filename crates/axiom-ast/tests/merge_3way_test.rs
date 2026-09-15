use axiom_ast::merge::{merge_modules_3way, AstConflict};
use axiom_ast::{
    BinaryOperator, Expr, FieldDef, FunctionDecl, FunctionParam, Literal, Module, Type, TypeDef,
};

fn dummy_module() -> Module {
    Module {
        types: Vec::new(),
        effects: Vec::new(),
        functions: Vec::new(),
    }
}

fn simple_func(name: &str, body: Expr) -> FunctionDecl {
    FunctionDecl {
        name: name.to_string(),
        params: vec![FunctionParam {
            name: "x".to_string(),
            ty: Type::Int,
        }],
        return_type: Type::Int,
        effects: Vec::new(),
        requires: Vec::new(),
        ensures: Vec::new(),
        body,
    }
}

#[test]
fn test_independent_function_additions_clean_merge() {
    let base = dummy_module();

    let mut ours = base.clone();
    ours.functions
        .push(simple_func("func_a", Expr::Literal(Literal::Int(1))));

    let mut theirs = base.clone();
    theirs
        .functions
        .push(simple_func("func_b", Expr::Literal(Literal::Int(2))));

    let merged = merge_modules_3way(&base, &ours, &theirs).expect("clean merge expected");
    assert_eq!(merged.functions.len(), 2);
    let names: Vec<_> = merged.functions.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"func_a"));
    assert!(names.contains(&"func_b"));
}

#[test]
fn test_independent_type_additions_clean_merge() {
    let base = dummy_module();

    let mut ours = base.clone();
    ours.types.push(TypeDef::Record {
        name: "User".to_string(),
        fields: vec![FieldDef {
            name: "id".to_string(),
            ty: Type::Int,
        }],
    });

    let mut theirs = base.clone();
    theirs.types.push(TypeDef::Record {
        name: "Order".to_string(),
        fields: vec![FieldDef {
            name: "order_id".to_string(),
            ty: Type::Int,
        }],
    });

    let merged = merge_modules_3way(&base, &ours, &theirs).expect("clean merge expected");
    assert_eq!(merged.types.len(), 2);
}

#[test]
fn test_concurrent_edits_on_different_functions() {
    let mut base = dummy_module();
    base.functions
        .push(simple_func("f1", Expr::Literal(Literal::Int(10))));
    base.functions
        .push(simple_func("f2", Expr::Literal(Literal::Int(20))));

    // Ours modifies f1
    let mut ours = base.clone();
    ours.functions[0].body = Expr::Literal(Literal::Int(11));

    // Theirs modifies f2
    let mut theirs = base.clone();
    theirs.functions[1].body = Expr::Literal(Literal::Int(22));

    let merged = merge_modules_3way(&base, &ours, &theirs).expect("clean merge expected");
    assert_eq!(merged.functions.len(), 2);
    assert_eq!(merged.functions[0].body, Expr::Literal(Literal::Int(11)));
    assert_eq!(merged.functions[1].body, Expr::Literal(Literal::Int(22)));
}

#[test]
fn test_contract_addition_and_hole_filling_orthogonal_fusion() {
    let mut base = dummy_module();
    base.functions
        .push(simple_func("f", Expr::Hole("hole_1".to_string())));

    // Ours adds a verification contract
    let mut ours = base.clone();
    ours.functions[0].requires.push(Expr::BinaryOp {
        op: BinaryOperator::GreaterThan,
        left: Box::new(Expr::Ident("x".to_string())),
        right: Box::new(Expr::Literal(Literal::Int(0))),
    });

    // Theirs fills the hole with a concrete expression
    let mut theirs = base.clone();
    theirs.functions[0].body = Expr::BinaryOp {
        op: BinaryOperator::Add,
        left: Box::new(Expr::Ident("x".to_string())),
        right: Box::new(Expr::Literal(Literal::Int(1))),
    };

    let merged = merge_modules_3way(&base, &ours, &theirs).expect("orthogonal merge expected");
    assert_eq!(merged.functions.len(), 1);
    let f = &merged.functions[0];
    assert_eq!(f.requires.len(), 1);
    assert!(matches!(
        f.body,
        Expr::BinaryOp {
            op: BinaryOperator::Add,
            ..
        }
    ));
}

#[test]
fn test_conflicting_expressions_on_same_node_yields_conflict_diagnostic() {
    let mut base = dummy_module();
    base.functions
        .push(simple_func("f", Expr::Hole("hole_1".to_string())));

    let mut ours = base.clone();
    ours.functions[0].body = Expr::Literal(Literal::Int(100));

    let mut theirs = base.clone();
    theirs.functions[0].body = Expr::Literal(Literal::Int(200));

    let err = merge_modules_3way(&base, &ours, &theirs).unwrap_err();
    assert!(!err.conflicts.is_empty());
    match &err.conflicts[0] {
        AstConflict::NodeConflict { path, .. } => {
            assert!(path.contains("functions[f].body"));
        }
        _ => panic!("expected NodeConflict, got {:?}", err.conflicts[0]),
    }
}

#[test]
fn test_delete_modify_conflict_detected() {
    let mut base = dummy_module();
    base.functions
        .push(simple_func("f", Expr::Literal(Literal::Int(1))));

    // Ours deletes f
    let ours = dummy_module();

    // Theirs modifies f
    let mut theirs = base.clone();
    theirs.functions[0].body = Expr::Literal(Literal::Int(999));

    let err = merge_modules_3way(&base, &ours, &theirs).unwrap_err();
    assert!(!err.conflicts.is_empty());
    assert!(matches!(
        &err.conflicts[0],
        AstConflict::DeleteModifyConflict { symbol, .. } if symbol == "f"
    ));
}

#[test]
fn test_signature_conflict_detected() {
    let mut base = dummy_module();
    base.functions
        .push(simple_func("f", Expr::Literal(Literal::Int(1))));

    // Ours changes return type to Float
    let mut ours = base.clone();
    ours.functions[0].return_type = Type::Float;

    // Theirs changes return type to Bool
    let mut theirs = base.clone();
    theirs.functions[0].return_type = Type::Bool;

    let err = merge_modules_3way(&base, &ours, &theirs).unwrap_err();
    assert!(!err.conflicts.is_empty());
    assert!(matches!(
        &err.conflicts[0],
        AstConflict::SignatureConflict { symbol, .. } if symbol == "f"
    ));
}

#[test]
fn test_nested_block_statements_clean_merge() {
    let mut base = dummy_module();
    let body = Expr::Block {
        statements: vec![
            Expr::Let {
                name: "a".to_string(),
                value: Box::new(Expr::Literal(Literal::Int(1))),
            },
            Expr::Let {
                name: "b".to_string(),
                value: Box::new(Expr::Literal(Literal::Int(2))),
            },
        ],
        tail: Some(Box::new(Expr::Ident("a".to_string()))),
    };
    base.functions.push(simple_func("calc", body));

    // Ours modifies statement 0 (let a = 10)
    let mut ours = base.clone();
    if let Expr::Block { statements, .. } = &mut ours.functions[0].body {
        statements[0] = Expr::Let {
            name: "a".to_string(),
            value: Box::new(Expr::Literal(Literal::Int(10))),
        };
    }

    // Theirs modifies statement 1 (let b = 20) and tail (b)
    let mut theirs = base.clone();
    if let Expr::Block { statements, tail } = &mut theirs.functions[0].body {
        statements[1] = Expr::Let {
            name: "b".to_string(),
            value: Box::new(Expr::Literal(Literal::Int(20))),
        };
        *tail = Some(Box::new(Expr::Ident("b".to_string())));
    }

    let merged = merge_modules_3way(&base, &ours, &theirs).expect("clean merge expected");
    if let Expr::Block { statements, tail } = &merged.functions[0].body {
        assert_eq!(
            statements[0],
            Expr::Let {
                name: "a".to_string(),
                value: Box::new(Expr::Literal(Literal::Int(10))),
            }
        );
        assert_eq!(
            statements[1],
            Expr::Let {
                name: "b".to_string(),
                value: Box::new(Expr::Literal(Literal::Int(20))),
            }
        );
        assert_eq!(*tail, Some(Box::new(Expr::Ident("b".to_string()))));
    } else {
        panic!("expected Block body");
    }
}
