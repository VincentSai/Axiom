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
        !results["bounded_float"].is_proved(),
        "Float contract must NOT be proved!"
    );
    assert!(
        matches!(results["bounded_float"], SmtResult::Skipped(_)),
        "Float contract must be Skipped!"
    );
}
