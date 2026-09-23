use axiom_parser::parse_module;
use axiom_smt::{verify_function, verify_module, SmtResult};

#[test]
fn test_bounded_float_addition_is_fail_closed_skipped() {
    // 审计反例：x == 0.1, result == 0.3 { x + 0.2 }
    // 在 Real 理论下数学上 unsat (0.1 + 0.2 = 0.3)，但在 IEEE-754 下 0.1 + 0.2 = 0.30000000000000004
    // 绝不能签发 Proved 假证明，必须 Fail-Closed 返回 Skipped！
    let src = r#"
        fn bounded(x: Float) -> Float
            requires { x == 0.1 }
            ensures { result == 0.3 }
        {
            x + 0.2
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let res = verify_function(&module.functions[0]);

    assert!(
        !res.is_proved(),
        "Bounded Float arithmetic must NOT be proved under Real theory! Got: {:?}",
        res
    );
    match res {
        SmtResult::Skipped(reason) => {
            assert!(
                reason.contains("floating-point arithmetic is not verified soundly"),
                "Unexpected skip reason: {reason}"
            );
        }
        other => panic!("expected SmtResult::Skipped, got: {:?}", other),
    }
}

#[test]
fn test_record_containing_float_is_fail_closed_skipped() {
    let src = r#"
        type Point = { x: Float, y: Float }

        fn translate(p: Point) -> Float
            requires { p.x >= 0.0, p.x <= 10.0 }
            ensures { result >= 0.0 }
        {
            p.x + 1.0
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let res = verify_function(&module.functions[0]);

    assert!(!res.is_proved());
    assert!(matches!(res, SmtResult::Skipped(_)));
}

#[test]
fn test_module_verification_skips_all_contracted_floats() {
    let src = r#"
        fn safe_math(a: Int) -> Int
            requires { a > 0, a < 1000 }
            ensures { result > a }
        {
            a + 1
        }

        fn bounded_float(x: Float) -> Float
            requires { x > 1.0, x < 2.0 }
            ensures { result > 1.0 }
        {
            x * 2.0
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let results = verify_module(&module);

    eprintln!("safe_math result: {:?}", results["safe_math"]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            results["safe_math"].is_proved(),
            "Pure integer contract should still be proved! Got: {:?}",
            results["safe_math"]
        );
    } else {
        assert!(
            matches!(results["safe_math"], SmtResult::Sampled { .. }),
            "Expected Sampled without Z3, got: {:?}",
            results["safe_math"]
        );
    }
    assert!(
        matches!(results["bounded_float"], SmtResult::Skipped(_)),
        "Float contract must be Skipped!"
    );
}

#[test]
fn test_hidden_float_indirect_call_returns_bool_is_fail_closed_skipped() {
    // 外部审查复现核心反例 (review-c391897)：
    // hidden_float 返回 Bool 且自身无契约，其函数体包含 0.1 + 0.2 == 0.3
    // public_check 带有契约并调用 hidden_float
    // 必须严密拦截 public_check 为 Skipped，绝不能在 Real 理论下错误 Proved！
    let src = r#"
        fn hidden_float() -> Bool {
            0.1 + 0.2 == 0.3
        }

        fn public_check() -> Bool
            ensures { result == true }
        {
            hidden_float()
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let results = verify_module(&module);

    let pub_res = &results["public_check"];
    assert!(
        !pub_res.is_proved(),
        "public_check calling hidden_float MUST NOT be proved! Got: {:?}",
        pub_res
    );
    assert!(
        matches!(pub_res, SmtResult::Skipped(_)),
        "public_check must be Skipped due to transitive float taint, got: {:?}",
        pub_res
    );
}

#[test]
fn test_hidden_float_indirect_call_with_float_param_is_fail_closed_skipped() {
    let src = r#"
        fn hidden_float(x: Float) -> Bool {
            x + 0.2 == 0.3
        }

        fn public_check() -> Bool
            ensures { result == true }
        {
            hidden_float(0.1)
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let results = verify_module(&module);

    let pub_res = &results["public_check"];
    assert!(
        !pub_res.is_proved(),
        "public_check calling hidden_float with float param MUST NOT be proved! Got: {:?}",
        pub_res
    );
    assert!(
        matches!(pub_res, SmtResult::Skipped(_)),
        "public_check must be Skipped due to transitive float taint, got: {:?}",
        pub_res
    );
}

#[test]
fn test_multi_layer_transitive_float_call_is_fail_closed_skipped() {
    // 三层传递依赖链: leaf_float (底层) -> mid_helper (中层) -> top_check (顶层契约函数)
    let src = r#"
        fn leaf_float() -> Bool {
            0.1 + 0.2 == 0.3
        }

        fn mid_helper() -> Bool {
            leaf_float()
        }

        fn top_check() -> Bool
            ensures { result == true }
        {
            mid_helper()
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let results = verify_module(&module);

    let top_res = &results["top_check"];
    assert!(
        !top_res.is_proved(),
        "top_check transitively calling leaf_float MUST NOT be proved! Got: {:?}",
        top_res
    );
    assert!(
        matches!(top_res, SmtResult::Skipped(_)),
        "top_check must be Skipped due to multi-layer transitive float taint, got: {:?}",
        top_res
    );
}

#[test]
fn test_unknown_external_call_in_contract_fails_closed() {
    // 审计要求：未建模未知外部调用默认 Fail-Closed 拦截，严禁签发假证明
    let src = r#"
        fn safe_looking(x: Int) -> Int
            requires { x > 0 }
            ensures { result > 0 }
        {
            unregistered_foreign_lib(x)
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let results = verify_module(&module);

    let res = &results["safe_looking"];
    assert!(
        !res.is_proved(),
        "Function calling unregistered foreign function MUST NOT be proved! Got: {:?}",
        res
    );
    match res {
        SmtResult::Skipped(reason) => {
            assert!(
                reason.contains("unknown external function calls (fail-closed)"),
                "Expected unknown external function fail-closed skip, got: {reason}"
            );
        }
        other => panic!("expected SmtResult::Skipped, got: {:?}", other),
    }
}

#[test]
fn test_type_and_function_name_collision_isolation() {
    // 测试类型与函数同名时，CallGraph 与 TypeDependencyGraph 严格隔离，不会错误发生污点扩散
    let src = r#"
        type Config = { val: Float }

        fn pure_calc(x: Int) -> Int
            requires { x > 10, x < 1000 }
            ensures { result > 20 }
        {
            x * 2
        }
    "#;
    let module = parse_module(src).expect("should parse");
    let results = verify_module(&module);

    let res = &results["pure_calc"];
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            res.is_proved(),
            "pure_calc must prove without false taint from unused Config type! Got: {:?}",
            res
        );
    } else {
        assert!(matches!(res, SmtResult::Sampled { .. }));
    }
}
