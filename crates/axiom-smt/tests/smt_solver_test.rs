use axiom_parser::parse_module;
use axiom_smt::{verify_function, verify_module, SmtResult};

#[test]
fn verifies_safe_division_with_precondition_is_fail_closed_skipped() {
    let module = parse_module(
        r#"
        fn safe_div(a: Float, b: Float) -> Float
            requires { b != 0.0, a >= -1000.0, a <= 1000.0, b >= -1000.0, b <= 1000.0 }
            ensures { result * b == a }
        {
            a / b
        }
        "#,
    )
    .expect("safe_div should parse");

    let result = verify_function(&module.functions[0]);
    match result {
        SmtResult::Skipped(reason) => {
            assert!(
                reason.contains("floating-point arithmetic is not verified soundly"),
                "Expected fail-closed reason, got: {reason}"
            );
        }
        other => panic!("expected Skipped due to float fail-closed, got {:?}", other),
    }
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
            assert!((*x - std::f64::consts::SQRT_2).abs() < 1e-6);
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
            requires { x > 0.0, x <= 1000.0, y > 0.0, y <= 1000.0 }
            ensures { result > 0.0 }
        {
            (x + 1.0) + (y * 2.0)
        }
        "#,
    )
    .expect("function should parse");

    let (smt_query, _) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("SMT query generation should succeed");

    // 实数计算不应被注入 i64 整数溢出检查
    assert!(!smt_query.contains("9223372036854775807"));

    // 验证入口对 Float 契约实施 Fail-Closed 拦截
    let res = verify_function(&module.functions[0]);
    assert!(
        matches!(res, SmtResult::Skipped(_)),
        "Float function must be fail-closed skipped! Got: {:?}",
        res
    );
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

#[test]
fn negative_division_truncates_towards_zero_and_rejects_euclidean() {
    // 1. 验证向零截断 (-5 / 2 == -2) 证明成功
    let trunc_module = parse_module(
        r#"
        fn test_trunc_div() -> Int
            ensures { result == -2 }
        {
            (-5) / 2
        }
        "#,
    )
    .expect("function should parse");

    let trunc_res = verify_function(&trunc_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(trunc_res, SmtResult::Proved { .. }),
            "Negative integer division must truncate towards zero (-2)! Got: {:?}",
            trunc_res
        );
    }

    // 2. 验证欧几里得除法 (-5 div 2 == -3) 被严格拒绝
    let euclidean_module = parse_module(
        r#"
        fn test_euclidean_div() -> Int
            ensures { result == -3 }
        {
            (-5) / 2
        }
        "#,
    )
    .expect("function should parse");

    let euclidean_res = verify_function(&euclidean_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(euclidean_res, SmtResult::CounterExample(_)),
            "Euclidean division result (-3) must be rejected with CounterExample! Got: {:?}",
            euclidean_res
        );
    }
}

#[test]
fn negative_modulo_preserves_sign_and_rejects_euclidean() {
    // 1. 验证向零截断取模 (-5 % 2 == -1) 证明成功
    let trunc_mod_module = parse_module(
        r#"
        fn test_trunc_mod() -> Int
            ensures { result == -1 }
        {
            (-5) % 2
        }
        "#,
    )
    .expect("function should parse");

    let (trunc_query, trunc_ob) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &trunc_mod_module.functions[0],
        Some(&trunc_mod_module),
    )
    .expect("trunc mod query generation must succeed");
    assert!(trunc_ob >= 1);
    assert!(trunc_query.contains("check-sat"));

    let trunc_res = verify_function(&trunc_mod_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(trunc_res, SmtResult::Proved { .. }),
            "Negative integer modulo must retain dividend sign (-1)! Got: {:?}",
            trunc_res
        );
    } else {
        assert!(matches!(trunc_res, SmtResult::Sampled { .. }));
    }

    // 2. 验证欧几里得非负余数 (-5 mod 2 == 1) 被严格拒绝
    let euclidean_mod_module = parse_module(
        r#"
        fn test_euclidean_mod() -> Int
            ensures { result == 1 }
        {
            (-5) % 2
        }
        "#,
    )
    .expect("function should parse");

    let (euclid_query, euclid_ob) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &euclidean_mod_module.functions[0],
        Some(&euclidean_mod_module),
    )
    .expect("euclid query generation must succeed");
    assert!(euclid_ob >= 1);
    assert!(euclid_query.contains("check-sat"));

    let euclidean_res = verify_function(&euclidean_mod_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(euclidean_res, SmtResult::CounterExample(_)),
            "Euclidean non-negative remainder (1) must be rejected with CounterExample! Got: {:?}",
            euclidean_res
        );
    } else {
        assert!(matches!(
            euclidean_res,
            SmtResult::CounterExample(_) | SmtResult::Sampled { .. }
        ));
    }
}

#[test]
fn match_wildcard_detects_dead_branch_division_by_zero_when_first_arm_is_false() {
    // [Spec 2, P0] 当首臂为 false 时，通配符 _ 覆盖 true (d == 0) 路径；
    // 内部的 100 / d 真实发生物理除零崩溃，必须被 SMT 精准拦截！
    let module = parse_module(
        r#"
        fn match_zero_div(d: Int) -> Int
            ensures { result >= 0 }
        {
            match d == 0 {
                false => 0,
                _ => 100 / d
            }
        }
        "#,
    )
    .expect("function should parse");

    let (query, obligations) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("match_zero_div query generation must succeed");
    assert!(obligations >= 2);
    assert!(query.contains("not (= d 0)"));

    let res = verify_function(&module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "Match wildcard with false first arm must detect division by zero at d == 0! Got: {:?}",
            res
        );
    } else {
        assert!(matches!(
            res,
            SmtResult::CounterExample(_) | SmtResult::Sampled { .. }
        ));
    }
}

#[test]
fn detects_division_overflow_on_i64_min_divided_by_negative_one() {
    let module = parse_module(
        r#"
        fn div_overflow(x: Int, y: Int) -> Int
            requires { y != 0 }
            ensures { result != 0 }
        {
            x / y
        }
        "#,
    )
    .expect("function should parse");

    let (query, obligations) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("div_overflow query generation must succeed");
    assert!(obligations >= 2);
    assert!(query.contains("9223372036854775808"));

    let res = verify_function(&module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "x / y must be rejected due to i64::MIN / -1 overflow! Got: {:?}",
            res
        );
    } else {
        assert!(matches!(
            res,
            SmtResult::CounterExample(_) | SmtResult::Sampled { .. }
        ));
    }
}

#[test]
fn match_first_match_semantics_with_wildcard_precedence() {
    // [Spec 1, P0] match b { _ => 0, true => 1 }
    // 必须遵从首匹配语义，首臂 _ => 0 胜出；若被错误编码为 (ite b 1 0)，则在 requires { b } 下会被误证为 1。
    let module = parse_module(
        r#"
        fn test_first_match(b: Bool) -> Int
            requires { b }
            ensures { result == 1 }
        {
            match b {
                _ => 0,
                true => 1
            }
        }
        "#,
    )
    .expect("function should parse");

    let (smt_query, obligations) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("SMT generation should succeed");
    // SMT 编码中首臂即为 0，绝不能生成 (ite b 1 0)
    assert!(!smt_query.contains("(ite b 1 0)"));
    // 包含 1 个 ensures 与 1 个 64位整型返回值范围安全义务
    assert_eq!(obligations, 2);

    let res = verify_function(&module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "First match arm _ => 0 must win, so result == 1 must be refuted! Got: {:?}",
            res
        );
    } else {
        assert!(matches!(
            res,
            SmtResult::Sampled { .. } | SmtResult::CounterExample(_)
        ));
    }
}

#[test]
fn match_dead_arm_safety_obligations_pruned() {
    // [Spec 1, P0] match x { _ => 0, 1 => 100 / 0 }
    // 后臂 1 => 100 / 0 为死代码，不能因为死分支中的除零导致整个函数被误拒！
    let module = parse_module(
        r#"
        fn test_dead_arm_pruning(x: Int) -> Int
            ensures { result == 0 }
        {
            match x {
                _ => 0,
                1 => 100 / 0
            }
        }
        "#,
    )
    .expect("function should parse");

    let (smt_query, obligations) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("SMT generation should succeed");
    // 收集到的义务只有 1 个 ensures 与 1 个 64位整型返回值范围安全义务，死分支内的 100 / 0 除零义务被剪枝（没有额外义务）
    assert_eq!(obligations, 2);
    assert!(!smt_query.contains("(not (= 0 0))"));

    let res = verify_function(&module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::Proved { .. }),
            "Function must be proved because dead branch division by zero is pruned! Got: {:?}",
            res
        );
    } else {
        assert!(matches!(
            res,
            SmtResult::Sampled { .. } | SmtResult::Proved { .. }
        ));
    }
}

#[test]
fn match_ident_lexical_shadowing_prevents_false_proof() {
    // [Spec 1, P0] match x { y => y }
    // 外层存在参数 y: Int 与 x: Int。match 内部模式 Ident(y) 必须建立词法绑定到 x。
    // 如果函数断言 ensures { result == y }，在运行时 result 实为 x，若误将模式 y 引用外层 y 会产生假证明 (Proved)！
    // 正确实现的 SMT 编码必须生成 (let ((y x)) y)，从而被 SMT 证明器精准证伪 (CounterExample)！
    let module = parse_module(
        r#"
        fn test_ident_shadowing(y: Int, x: Int) -> Int
            ensures { result == y }
        {
            match x {
                y => y
            }
        }
        "#,
    )
    .expect("function should parse");

    let (smt_query, obligations) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("SMT generation should succeed");

    // SMT 编码中必须包含词法作用域绑定 (let ((y x)) y)
    assert!(smt_query.contains("(let ((y x)) y)"));
    assert_eq!(obligations, 2);

    let res = verify_function(&module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "Lexical shadowing must bind to x, so result == y must be refuted by counterexample! Got: {:?}",
            res
        );
    } else {
        if std::env::var("AXIOM_REQUIRE_Z3").is_ok() {
            panic!("AXIOM_REQUIRE_Z3 is set but Z3 executable was not found");
        }
        assert!(matches!(
            res,
            SmtResult::Sampled { .. } | SmtResult::CounterExample(_)
        ));
    }
}

#[test]
fn let_bound_real_division_preserves_real_type() {
    // [Spec 6, P1] let a = x + 1.0; let b = y + 1.0; a / b
    // 局部变量 a 与 b 的类型必须被推导为 Real，除法必须生成实数除法 (/)，绝不能生成向零截断的整数除法
    let module = parse_module(
        r#"
        fn test_real_div(x: Float, y: Float) -> Float
            requires { y > 0.0 }
            ensures { result > 0.0 }
        {
            let a = x + 1.0;
            let b = y + 1.0;
            a / b
        }
        "#,
    )
    .expect("function should parse");

    let (smt_query, _) = axiom_smt::generate_smtlib2_with_module_and_obligations(
        &module.functions[0],
        Some(&module),
    )
    .expect("SMT generation should succeed");

    // 必须生成实数除法 (/ a b) 或其展开，绝不能生成整数 div
    assert!(smt_query.contains("(/ a b)"));
    assert!(!smt_query.contains("div a b"));
}

#[test]
fn smtlib2_rejects_function_containing_expr_raw() {
    use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Literal, Type};

    let func = FunctionDecl {
        name: "escape_hatch".into(),
        params: vec![FunctionParam {
            name: "x".into(),
            ty: Type::Int,
        }],
        return_type: Type::Int,
        effects: vec![],
        requires: vec![],
        ensures: vec![Expr::BinaryOp {
            op: BinaryOperator::GreaterThan,
            left: Box::new(Expr::Ident("result".into())),
            right: Box::new(Expr::Literal(Literal::Int(0))),
        }],
        body: Expr::Raw("x + 1".into()),
    };

    let res = axiom_smt::generate_smtlib2(&func);
    assert!(
        res.is_err(),
        "generate_smtlib2 must reject functions with Expr::Raw"
    );
    let err = res.unwrap_err();
    assert!(
        err.contains("contains Expr::Raw which cannot be verified with SMT"),
        "unexpected error message: {}",
        err
    );
}

#[test]
fn decimal_fixed_point_exact_addition_proves_without_float_error() {
    let module = parse_module(
        r#"
        fn add_exact(a: Decimal, b: Decimal) -> Decimal
            requires { a == 0.1d, b == 0.2d }
            ensures { result == 0.3d }
        {
            a + b
        }
        "#,
    )
    .expect("add_exact should parse");

    let result = verify_function(&module.functions[0]);
    assert!(
        matches!(
            result,
            SmtResult::Proved { .. } | SmtResult::Sampled { .. }
        ),
        "0.1d + 0.2d == 0.3d must be proved exact under fixed-point integer SMT without float error! Got: {:?}",
        result
    );
}

#[test]
fn calc_sell_refund_rejects_endpoint_cheat_and_proves_exact_formula() {
    // 1. 端点作弊实现：只在 durability_cur == durability_max 时返回 cost，否则返回 0
    let cheat_module = parse_module(
        r#"
        fn calc_sell_refund(cost: Int, durability_cur: Int, durability_max: Int) -> Int
            requires {
                cost >= 0,
                cost <= 100000000,
                durability_max > 0,
                durability_max <= 100000,
                durability_cur >= 0,
                durability_cur <= durability_max
            }
            ensures {
                result >= 0,
                result <= cost,
                result == (cost * durability_cur) / durability_max
            }
        {
            match durability_cur == durability_max {
                true => cost,
                false => 0
            }
        }
        "#,
    )
    .expect("cheat_module should parse");

    let cheat_res = verify_function(&cheat_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(cheat_res, SmtResult::CounterExample(_)),
            "Endpoint cheat implementation MUST be rejected by SMT counterexample under exact contract! Got: {:?}",
            cheat_res
        );
    }

    // 2. 真实数学公式实现：(cost * durability_cur) / durability_max
    let honest_module = parse_module(
        r#"
        fn calc_sell_refund(cost: Int, durability_cur: Int, durability_max: Int) -> Int
            requires {
                cost >= 0,
                cost <= 100000000,
                durability_max > 0,
                durability_max <= 100000,
                durability_cur >= 0,
                durability_cur <= durability_max
            }
            ensures {
                result >= 0,
                result <= cost,
                result == (cost * durability_cur) / durability_max
            }
        {
            (cost * durability_cur) / durability_max
        }
        "#,
    )
    .expect("honest_module should parse");

    let honest_res = verify_function(&honest_module.functions[0]);
    if axiom_smt::find_z3_executable().is_some() {
        assert!(
            matches!(honest_res, SmtResult::Proved { .. }),
            "Exact honest formula MUST be proved by SMT! Got: {:?}",
            honest_res
        );
    }
}

#[test]
fn decimal_fixed_point_exact_multiplication_and_division_prove_without_scale_error() {
    let module = parse_module(
        r#"
        fn mul_exact(a: Decimal, b: Decimal) -> Decimal
            requires { a == 2.5d, b == 4.0d }
            ensures { result == 10.0d }
        {
            a * b
        }

        fn div_exact(a: Decimal, b: Decimal) -> Decimal
            requires { a == 10.0d, b == 4.0d }
            ensures { result == 2.5d }
        {
            a / b
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let mul_res = verify_function(&module.functions[0]);
        assert!(
            matches!(mul_res, SmtResult::Proved { .. }),
            "Decimal multiplication (2.5d * 4.0d == 10.0d) MUST be proved under fixed-point scaling! Got: {:?}",
            mul_res
        );

        let div_res = verify_function(&module.functions[1]);
        assert!(
            matches!(div_res, SmtResult::Proved { .. }),
            "Decimal division (10.0d / 4.0d == 2.5d) MUST be proved under fixed-point scaling! Got: {:?}",
            div_res
        );
    }
}

#[test]
fn decimal_large_multiplication_with_i128_guard_proves_successfully() {
    let module = parse_module(
        r#"
        fn mul_large(a: Decimal, b: Decimal) -> Decimal
            requires { a == 50000.0d, b == 50000.0d }
            ensures { result == 2500000000.0d }
        {
            a * b
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let res = verify_function(&module.functions[0]);
        assert!(
            matches!(res, SmtResult::Proved { .. }),
            "Large Decimal multiplication (intermediate exceeds i64 but within i128) MUST be proved! Got: {:?}",
            res
        );
    }
}

#[test]
fn decimal_multiplication_overflowing_i64_result_is_caught() {
    let module = parse_module(
        r#"
        fn mul_overflow(a: Decimal, b: Decimal) -> Decimal
            requires { a >= 100000000000.0d, b >= 100000000000.0d }
            ensures { result >= 0.0d }
        {
            a * b
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let res = verify_function(&module.functions[0]);
        assert!(
            matches!(res, SmtResult::CounterExample { .. }),
            "Decimal multiplication overflowing final i64 MUST trigger CounterExample! Got: {:?}",
            res
        );
    }
}

#[test]
fn test_smt_inlining_variable_capture_soundness() {
    let module = parse_module(
        r#"
        fn helper(x: Int) -> Int {
            let y = 1;
            x + y
        }

        fn wrapper(y: Int) -> Int
            ensures {
                result == 2
            }
        {
            helper(y)
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let wrapper_fn = module
            .functions
            .iter()
            .find(|f| f.name == "wrapper")
            .unwrap();
        let res = axiom_smt::verify_function_in_module(wrapper_fn, &module);
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "wrapper must produce CounterExample because result is y + 1 != 2 for general y, got: {:?}",
            res
        );
    }
}

#[test]
fn test_smt_inlining_nested_and_shadowing_variable_capture_soundness() {
    let module = parse_module(
        r#"
        fn inner(x: Int) -> Int {
            let y = 10;
            let y = y + 5;
            x + y
        }

        fn outer(x: Int) -> Int {
            let y = 100;
            inner(x) + y
        }

        fn test_runner(y: Int) -> Int
            ensures {
                result == 115
            }
        {
            outer(y)
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let runner_fn = module
            .functions
            .iter()
            .find(|f| f.name == "test_runner")
            .unwrap();
        let res = axiom_smt::verify_function_in_module(runner_fn, &module);
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "test_runner must produce CounterExample because result is y + 115 != 115 for general y, got: {:?}",
            res
        );
    }
}

#[test]
fn test_smt_inlining_unused_param_preserves_safety_obligations_and_catches_div_zero() {
    let module = axiom_parser::parse_module(
        r#"
        fn helper(x: Int) -> Int ensures { result == 1 } {
            1
        }
        fn wrapper(n: Int) -> Int ensures { result == 1 } {
            helper(1 / n)
        }
        fn safe_wrapper(n: Int) -> Int requires { n != 0 } ensures { result == 1 } {
            helper(1 / n)
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let wrapper_fn = module
            .functions
            .iter()
            .find(|f| f.name == "wrapper")
            .unwrap();
        let res = axiom_smt::verify_function_in_module(wrapper_fn, &module);
        assert!(
            matches!(res, SmtResult::CounterExample(_)),
            "wrapper must produce CounterExample because 1/n can divide by zero when n=0, got: {:?}",
            res
        );

        let safe_wrapper_fn = module
            .functions
            .iter()
            .find(|f| f.name == "safe_wrapper")
            .unwrap();
        let safe_res = axiom_smt::verify_function_in_module(safe_wrapper_fn, &module);
        assert!(
            matches!(safe_res, SmtResult::Proved { .. }),
            "safe_wrapper with requires n != 0 must be Proved, got: {:?}",
            safe_res
        );
    }
}

#[test]
fn test_smt_hygienic_renaming_avoids_user_identifiers_collision() {
    let module = axiom_parser::parse_module(
        r#"
        fn helper(x: Int) -> Int ensures { result == x + 1 } {
            let y = 1;
            x + y
        }
        fn wrapper_false() -> Int ensures { result == 2 } {
            let y__axiom_in_1_1 = 2;
            helper(y__axiom_in_1_1)
        }
        fn wrapper_true() -> Int ensures { result == 3 } {
            let y__axiom_in_1_1 = 2;
            helper(y__axiom_in_1_1)
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        let wrapper_false_fn = module
            .functions
            .iter()
            .find(|f| f.name == "wrapper_false")
            .unwrap();
        let res_false = axiom_smt::verify_function_in_module(wrapper_false_fn, &module);
        assert!(
            matches!(res_false, SmtResult::CounterExample(_)),
            "wrapper_false must produce CounterExample because result is 3 != 2 (no variable capture), got: {:?}",
            res_false
        );

        let wrapper_true_fn = module
            .functions
            .iter()
            .find(|f| f.name == "wrapper_true")
            .unwrap();
        let res_true = axiom_smt::verify_function_in_module(wrapper_true_fn, &module);
        assert!(
            matches!(res_true, SmtResult::Proved { .. }),
            "wrapper_true must be Proved because 2 + 1 = 3, got: {:?}",
            res_true
        );
    }
}

#[test]
fn test_smt_hygienic_renaming_is_deterministic() {
    let module = axiom_parser::parse_module(
        r#"
        fn helper(x: Int) -> Int ensures { result == x + 10 } {
            let a = 4;
            let b = 6;
            x + a + b
        }
        fn caller(z: Int) -> Int ensures { result == z + 20 } {
            let first = helper(z);
            let second = helper(first);
            second
        }
        "#,
    )
    .expect("module should parse");

    let caller_fn = module
        .functions
        .iter()
        .find(|f| f.name == "caller")
        .unwrap();

    let (smt1, obs1) =
        axiom_smt::generate_smtlib2_with_module_and_obligations(caller_fn, Some(&module))
            .expect("SMT 1 should generate");
    let (smt2, obs2) =
        axiom_smt::generate_smtlib2_with_module_and_obligations(caller_fn, Some(&module))
            .expect("SMT 2 should generate");

    assert_eq!(obs1, obs2, "obligations count must be identical");
    assert_eq!(
        smt1, smt2,
        "generated SMT-LIB2 queries must be 100% byte-for-byte identical across runs"
    );
}

#[test]
fn test_smt_caller_must_satisfy_callee_preconditions() {
    let module = axiom_parser::parse_module(
        r#"
        fn helper(x: Int) -> Int requires { x > 0 } ensures { result == x } {
            x
        }
        fn wrapper() -> Int ensures { result == 0 } {
            helper(0)
        }
        fn safe_caller() -> Int ensures { result == 10 } {
            helper(10)
        }
        fn caller_with_var(n: Int) -> Int ensures { result == n } {
            helper(n)
        }
        fn caller_with_requires(n: Int) -> Int requires { n > 0 } ensures { result == n } {
            helper(n)
        }
        "#,
    )
    .expect("module should parse");

    if axiom_smt::find_z3_executable().is_some() {
        // 1. wrapper() 传入 0 违背 helper 的 requires { x > 0 } -> 报 CounterExample
        let wrapper_fn = module
            .functions
            .iter()
            .find(|f| f.name == "wrapper")
            .unwrap();
        let res_wrapper = axiom_smt::verify_function_in_module(wrapper_fn, &module);
        assert!(
            matches!(res_wrapper, SmtResult::CounterExample(_)),
            "wrapper must produce CounterExample because helper requires x > 0, but was called with 0, got: {:?}",
            res_wrapper
        );

        // 2. safe_caller() 传入 10 满足 helper 的 requires { x > 0 } -> 证明为 Proved
        let safe_caller_fn = module
            .functions
            .iter()
            .find(|f| f.name == "safe_caller")
            .unwrap();
        let res_safe = axiom_smt::verify_function_in_module(safe_caller_fn, &module);
        assert!(
            matches!(res_safe, SmtResult::Proved { .. }),
            "safe_caller passing 10 > 0 must be Proved, got: {:?}",
            res_safe
        );

        // 3. caller_with_var(n) 没有 requires { n > 0 }，无法保证 n > 0 -> 报 CounterExample
        let caller_var_fn = module
            .functions
            .iter()
            .find(|f| f.name == "caller_with_var")
            .unwrap();
        let res_var = axiom_smt::verify_function_in_module(caller_var_fn, &module);
        assert!(
            matches!(res_var, SmtResult::CounterExample(_)),
            "caller_with_var must produce CounterExample because parameter n is not guaranteed to be > 0, got: {:?}",
            res_var
        );

        // 4. caller_with_requires(n) 自身声明 requires { n > 0 } 承担契约 -> 证明为 Proved
        let caller_req_fn = module
            .functions
            .iter()
            .find(|f| f.name == "caller_with_requires")
            .unwrap();
        let res_req = axiom_smt::verify_function_in_module(caller_req_fn, &module);
        assert!(
            matches!(res_req, SmtResult::Proved { .. }),
            "caller_with_requires must be Proved, got: {:?}",
            res_req
        );
    }
}
