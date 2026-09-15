use axiom_ast::analysis::CallGraph;
use axiom_ast::{Expr, FieldDef, FunctionDecl, FunctionParam, Literal, Module, Type, TypeDef};

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

fn call_func(name: &str, callee: &str) -> FunctionDecl {
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
        body: Expr::Call {
            callee: Box::new(Expr::Ident(callee.to_string())),
            args: vec![Expr::Ident("x".to_string())],
        },
    }
}

#[test]
fn test_linear_call_chain_transitive_closure() {
    // 依赖链: f (底层) <- g <- h (顶层调用方)
    // 即 g 调用 f, h 调用 g
    let module = Module {
        types: Vec::new(),
        effects: Vec::new(),
        functions: vec![
            simple_func("f", Expr::Literal(Literal::Int(1))),
            call_func("g", "f"),
            call_func("h", "g"),
        ],
    };

    let graph = CallGraph::from_module(&module);

    // 修改底层 f，应当传递影响直接与间接调用方: f, g, h
    let dependents_f = graph.transitive_dependents(&["f"]);
    assert_eq!(dependents_f.len(), 3);
    assert!(dependents_f.contains("f"));
    assert!(dependents_f.contains("g"));
    assert!(dependents_f.contains("h"));

    // 修改中间层 g，仅影响 g, h
    let dependents_g = graph.transitive_dependents(&["g"]);
    assert_eq!(dependents_g.len(), 2);
    assert!(dependents_g.contains("g"));
    assert!(dependents_g.contains("h"));

    // 修改顶层 h，仅影响自身 h
    let dependents_h = graph.transitive_dependents(&["h"]);
    assert_eq!(dependents_h.len(), 1);
    assert!(dependents_h.contains("h"));
}

#[test]
fn test_diamond_call_graph_and_cycles() {
    // 菱形调用图: a <- b, a <- c, b <- d, c <- d
    // 即 d 调用 b 和 c，b 和 c 都调用 a
    let mut d_func = simple_func("d", Expr::Literal(Literal::Int(0)));
    d_func.body = Expr::Block {
        statements: vec![
            Expr::Call {
                callee: Box::new(Expr::Ident("b".to_string())),
                args: vec![],
            },
            Expr::Call {
                callee: Box::new(Expr::Ident("c".to_string())),
                args: vec![],
            },
        ],
        tail: None,
    };

    let module = Module {
        types: Vec::new(),
        effects: Vec::new(),
        functions: vec![
            simple_func("a", Expr::Literal(Literal::Int(1))),
            call_func("b", "a"),
            call_func("c", "a"),
            d_func,
        ],
    };

    let graph = CallGraph::from_module(&module);
    let dependents_a = graph.transitive_dependents(&["a"]);
    assert_eq!(dependents_a.len(), 4);
    assert!(dependents_a.contains("a"));
    assert!(dependents_a.contains("b"));
    assert!(dependents_a.contains("c"));
    assert!(dependents_a.contains("d"));
}

#[test]
fn test_type_dependency_impact() {
    // 函数 u 使用了类型 User
    let mut func_u = simple_func("get_user_id", Expr::Literal(Literal::Int(0)));
    func_u.params = vec![FunctionParam {
        name: "user".to_string(),
        ty: Type::Custom("User".to_string()),
    }];

    let module = Module {
        types: vec![TypeDef::Record {
            name: "User".to_string(),
            fields: vec![FieldDef {
                name: "id".to_string(),
                ty: Type::Int,
            }],
        }],
        effects: Vec::new(),
        functions: vec![func_u],
    };

    let graph = CallGraph::from_module(&module);
    let dependents = graph.transitive_dependents(&["User"]);
    assert!(dependents.contains("User"));
    assert!(dependents.contains("get_user_id"));
}
