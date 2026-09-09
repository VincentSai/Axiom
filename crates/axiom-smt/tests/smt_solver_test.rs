use axiom_parser::parse_module;
use axiom_smt::{verify_function, verify_module, SmtResult};

#[test]
fn verifies_safe_division_with_precondition() {
    let module = parse_module(
        r#"
        fn safe_div(a: Decimal, b: Decimal) -> Decimal
            requires { b != 0.0 }
            ensures { result * b == a }
        {
            a / b
        }
        "#,
    )
    .expect("safe_div should parse");

    let result = verify_function(&module.functions[0]);
    assert!(matches!(
        result,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));
}

#[test]
fn detects_counter_example_on_division_by_zero() {
    let module = parse_module(
        r#"
        fn buggy_div(a: Decimal, b: Decimal) -> Decimal
            requires { a > 0.0 }
            ensures { result > 0.0 }
        {
            a / b
        }
        "#,
    )
    .expect("buggy_div should parse");

    let result = verify_function(&module.functions[0]);
    match result {
        SmtResult::CounterExample(ce) => {
            let a = *ce.get("a").unwrap();
            let b = *ce.get("b").unwrap();
            assert!(a > 0.0);
            assert!(b == 0.0 || a / b <= 0.0);
        }
        other => panic!("expected counter-example, got {:?}", other),
    }
}

#[test]
fn detects_counter_example_on_overdraft_or_contract_violation() {
    let module = parse_module(
        r#"
        fn withdraw(balance: Decimal, amount: Decimal) -> Decimal
            requires { balance >= 0.0, amount > 0.0 }
            ensures { result >= 0.0 }
        {
            balance - amount
        }
        "#,
    )
    .expect("withdraw should parse");

    let result = verify_function(&module.functions[0]);
    match result {
        SmtResult::CounterExample(ce) => {
            let balance = *ce.get("balance").unwrap();
            let amount = *ce.get("amount").unwrap();
            assert!(balance >= 0.0);
            assert!(amount > 0.0);
            assert!(balance - amount < 0.0);
        }
        other => panic!("expected counter-example, got {:?}", other),
    }
}

#[test]
fn skips_unsupported_or_hole_functions() {
    let module = parse_module(
        r#"
        fn plain(x: Int) -> Int { x + 1 }
        fn hole_func(x: Int) -> Int requires { x > 0 } ensures { result > 0 } { ?hole_body }
        "#,
    )
    .expect("module should parse");

    let results = verify_module(&module);
    assert!(matches!(results["plain"], SmtResult::Skipped(_)));
    assert!(matches!(results["hole_func"], SmtResult::Skipped(_)));
}

#[test]
fn verifies_logical_operators_and_let_bindings() {
    let module = parse_module(
        r#"
        fn clamp_non_negative(value: Int) -> Int
            requires { value >= 0 && value <= 100 }
            ensures { result >= 0 => result == value }
        {
            let copy = value;
            copy
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);
    assert!(matches!(
        res,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));
}

#[test]
fn generates_valid_smtlib2_query_script() {
    let module = parse_module(
        r#"
        fn safe_sub(balance: Int, amount: Int) -> Int
            requires { balance >= 0, amount > 0, balance >= amount }
            ensures { result >= 0, result == balance - amount }
        {
            balance - amount
        }
        "#,
    )
    .expect("safe_sub should parse");

    let smt2 =
        axiom_smt::generate_smtlib2(&module.functions[0]).expect("should generate SMT-LIB2 query");

    assert!(smt2.contains("(set-logic ALL)"));
    assert!(smt2.contains("(declare-const balance Int)"));
    assert!(smt2.contains("(declare-const amount Int)"));
    assert!(smt2.contains("(declare-const result Int)"));
    assert!(smt2.contains("(assert (>= balance 0))"));
    assert!(smt2.contains("(assert (= result (- balance amount)))"));
    assert!(smt2.contains("(>= result 0)"));
    assert!(smt2.contains("(= result (- balance amount))"));
    assert!(
        smt2.contains("(and (>= result (- 9223372036854775808)) (<= result 9223372036854775807))")
    );
    assert!(smt2.contains("(and (>= (- balance amount) (- 9223372036854775808)) (<= (- balance amount) 9223372036854775807))"));
    assert!(smt2.contains("(check-sat)"));
    assert!(smt2.contains("(get-model)"));
}

#[test]
fn generates_nonlinear_real_query_for_sqrt2_counterexample() {
    let module = parse_module(
        r#"
        fn sqrt2_contract(x: Float) -> Bool
            requires { x >= 0.0, x <= 2.0 }
            ensures { result == true }
        {
            x * x != 2.0
        }
        "#,
    )
    .expect("sqrt2_contract should parse");

    let smt2 = axiom_smt::generate_smtlib2(&module.functions[0])
        .expect("should generate SMT-LIB2 for non-linear real query");

    assert!(smt2.contains("(declare-const x Real)"));
    assert!(smt2.contains("(declare-const result Bool)"));
    assert!(smt2.contains("(assert (>= x 0.0))"));
    assert!(smt2.contains("(assert (<= x 2.0))"));
    assert!(smt2.contains("(assert (= result (distinct (* x x) 2.0)))"));
    assert!(smt2.contains("(assert (not (= result true)))"));
    assert!(smt2.contains("(check-sat)"));
    assert!(smt2.contains("(get-model)"));
}

#[test]
fn parses_z3_sat_model_with_real_numbers() {
    let z3_output = r#"sat
(model
  (define-fun x () Real
    1.4142135623)
  (define-fun result () Bool
    false)
)
"#;
    let res = axiom_smt::z3_solver::parse_z3_output(z3_output).expect("should parse sat output");
    match res {
        axiom_smt::Z3Result::Sat(model) => {
            let x = model.get("x").expect("x should be present in model");
            assert!((*x - 1.4142135623).abs() < 1e-6);
        }
        other => panic!("expected Sat, got {:?}", other),
    }
}

#[test]
fn detects_and_rejects_vacuous_contract_with_unsatisfiable_preconditions() {
    // 恶意/错误前置：x > 10 且 x < 5（在数学上自相矛盾，不可满足）
    // 在传统一阶逻辑中，False => 任何事都是 True，如果不做 Vacuity 检查，任何荒谬函数体都会被误判为 Proved！
    let module = parse_module(
        r#"
        fn impossible_precondition(x: Int) -> Int
            requires { x > 10, x < 5 }
            ensures { result == 99999 }
        {
            0
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            res.is_vacuous(),
            "Z3 SMT must detect unsatisfiable precondition and reject vacuous contract! Got: {:?}",
            res
        );
        match res {
            SmtResult::VacuousContract { reason } => {
                assert!(reason.contains("unsatisfiable"));
            }
            other => panic!("expected VacuousContract, got: {:?}", other),
        }
    }
}

#[test]
fn detects_and_rejects_integer_overflow_beyond_i64_bounds() {
    let module = parse_module(
        r#"
        fn potential_overflow(x: Int) -> Int
            requires { x >= 0 }
            ensures { result > x }
        {
            x + 1
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "Z3 SMT must find counterexample when x + 1 can exceed i64::MAX! Got: {:?}",
            res
        );
    }
}

#[test]
fn detects_and_rejects_intermediate_arithmetic_overflow_even_if_result_cancels_out() {
    let module = parse_module(
        r#"
        fn intermediate_overflow(x: Int) -> Int
            requires { x >= 0 }
            ensures { result == x }
        {
            (x + 1) - 1
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "Z3 SMT must reject intermediate overflow in (x + 1) - 1 even if final result is in bounds! Got: {:?}",
            res
        );
    }
}

#[test]
fn detects_overflow_in_unused_let_binding() {
    let module = parse_module(
        r#"
        fn unused_overflow(x: Int) -> Int
            requires { x >= 0 }
            ensures { result == 0 }
        {
            let overflowed = x + 1;
            0
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "Z3 SMT must reject overflow in unused let binding `x + 1`! Got: {:?}",
            res
        );
    }
}

#[test]
fn does_not_false_alarm_on_dead_branch_division_by_zero() {
    let module = parse_module(
        r#"
        fn safe_branch_division(d: Int) -> Int
            requires { d >= 0 }
            ensures { result >= 0 }
        {
            match d == 0 {
                true => 0,
                false => 100 / d,
            }
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::Proved { .. }),
            "Dead-branch division 100 / d when d == 0 must not trigger false positive! Got: {:?}",
            res
        );
    }
}

#[test]
fn composite_real_expression_skips_integer_safety_bounds() {
    let module = parse_module(
        r#"
        fn composite_real_calc(x: Float, y: Float) -> Float
            requires { x > 0.0 && y > 0.0 }
            ensures { result > 0.0 }
        {
            (x + 1.0) + (y * 2.0)
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::Proved { .. }),
            "Composite Real operations must not have 64-bit integer overflow bounds injected! Got: {:?}",
            res
        );
    }
}

#[test]
fn modulo_operation_generates_safety_obligations_and_verifies() {
    let module = parse_module(
        r#"
        fn safe_modulo(a: Int, b: Int) -> Int
            requires { a >= 0 && b > 0 }
            ensures { result >= 0 }
        {
            a % b
        }
        "#,
    )
    .expect("function should parse");

    let res = verify_function(&module.functions[0]);

    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::Proved { .. }),
            "Modulo operation should be verified with zero-divisor check! Got: {:?}",
            res
        );
    }

    let zero_mod_module = parse_module(
        r#"
        fn dangerous_modulo(a: Int) -> Int
            ensures { result >= 0 }
        {
            a % 0
        }
        "#,
    )
    .expect("function should parse");

    let zero_res = verify_function(&zero_mod_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(zero_res, SmtResult::CounterExample(_)),
            "Modulo by zero must be rejected with counterexample! Got: {:?}",
            zero_res
        );
    }
}
