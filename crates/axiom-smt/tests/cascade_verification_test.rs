use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Literal, Module, Type};
use axiom_smt::cascade::verify_dependent_closure;

fn dummy_int_func(name: &str, body: Expr) -> FunctionDecl {
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
fn test_cascade_verification_proves_downstream_when_valid() {
    // inc(x: Int) -> Int requires x >= 0 && x < 1000 ensures result == x + 1 { x + 1 }
    let mut inc_func = dummy_int_func(
        "inc",
        Expr::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(1))),
        },
    );
    inc_func.requires = vec![
        Expr::BinaryOp {
            op: BinaryOperator::GreaterThanOrEqual,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(0))),
        },
        Expr::BinaryOp {
            op: BinaryOperator::LessThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(1000))),
        },
    ];
    inc_func.ensures = vec![Expr::BinaryOp {
        op: BinaryOperator::Equal,
        left: Box::new(Expr::Ident("result".to_string())),
        right: Box::new(Expr::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(1))),
        }),
    }];

    // caller(x: Int) -> Int requires x > 0 && x < 500 ensures result > 0 { inc(x) }
    let mut caller_func = dummy_int_func(
        "caller",
        Expr::Call {
            callee: Box::new(Expr::Ident("inc".to_string())),
            args: vec![Expr::Ident("x".to_string())],
        },
    );
    caller_func.requires = vec![
        Expr::BinaryOp {
            op: BinaryOperator::GreaterThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(0))),
        },
        Expr::BinaryOp {
            op: BinaryOperator::LessThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(500))),
        },
    ];
    caller_func.ensures = vec![Expr::BinaryOp {
        op: BinaryOperator::GreaterThan,
        left: Box::new(Expr::Ident("result".to_string())),
        right: Box::new(Expr::Literal(Literal::Int(0))),
    }];

    let module = Module {
        types: Vec::new(),
        effects: Vec::new(),
        functions: vec![inc_func, caller_func],
    };

    // 触发对 "inc" 的修改检验，级联闭包应当包含 "inc" 和 "caller"，且两者均证明通过
    let report = verify_dependent_closure(&module, &["inc"]);
    assert!(report.is_all_proved());
    assert!(report.verified_functions.contains(&"inc".to_string()));
    assert!(report.verified_functions.contains(&"caller".to_string()));
    assert!(report.failures.is_empty());
}

#[test]
fn test_cascade_verification_catches_upstream_breaking_downstream() {
    // helper(x: Int) -> Int requires x > 100 ensures result > 100 { x }
    let mut helper = dummy_int_func("helper", Expr::Ident("x".to_string()));
    helper.requires = vec![
        Expr::BinaryOp {
            op: BinaryOperator::GreaterThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(100))),
        },
        Expr::BinaryOp {
            op: BinaryOperator::LessThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(1000))),
        },
    ];
    helper.ensures = vec![Expr::BinaryOp {
        op: BinaryOperator::GreaterThan,
        left: Box::new(Expr::Ident("result".to_string())),
        right: Box::new(Expr::Literal(Literal::Int(100))),
    }];

    // caller(x: Int) -> Int requires x == 10 ensures result > 0 { helper(x) }
    // 违背 helper 的 requires x > 100，或如果 helper 返回的是未证明实现
    // 如果 caller 体内如果包含一个无法证明的后置条件，如 ensures result < 0 { helper(x) }
    let mut caller = dummy_int_func(
        "caller",
        Expr::Call {
            callee: Box::new(Expr::Ident("helper".to_string())),
            args: vec![Expr::Ident("x".to_string())],
        },
    );
    caller.requires = vec![
        Expr::BinaryOp {
            op: BinaryOperator::GreaterThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(100))),
        },
        Expr::BinaryOp {
            op: BinaryOperator::LessThan,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::Literal(Literal::Int(1000))),
        },
    ];
    // 故意写入与 helper(x) > 100 矛盾的断言：ensures result < 0
    caller.ensures = vec![Expr::BinaryOp {
        op: BinaryOperator::LessThan,
        left: Box::new(Expr::Ident("result".to_string())),
        right: Box::new(Expr::Literal(Literal::Int(0))),
    }];

    let module = Module {
        types: Vec::new(),
        effects: Vec::new(),
        functions: vec![helper, caller],
    };

    // 触发对 "helper" 的修改检验，级联闭包检查 "caller" 时必须检测出反例！
    let report = verify_dependent_closure(&module, &["helper"]);
    assert!(!report.is_all_proved());
    assert!(!report.failures.is_empty());
    assert!(report.failures.iter().any(|f| f.function_name == "caller"));
}
