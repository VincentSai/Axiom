use axiom_parser::parse_module;
use axiom_smt::verify_module;
use axiom_transpiler::transpile_to_rust;
use std::{fs, process::Command};

fn run_probe_binary_ext(
    name: &str,
    source: &str,
    call: &str,
    debug_assertions: bool,
) -> (String, bool, Option<i32>) {
    let module = parse_module(source).expect("probe must parse");
    let rust = transpile_to_rust(&module).expect("probe must transpile");

    let temp_dir = std::env::temp_dir().join(format!(
        "axiom_numeric_probe_{}_{}",
        std::process::id(),
        name
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join(format!("{name}.rs"));
    let exe_path = temp_dir.join(format!("{name}.exe"));
    fs::write(
        &rust_path,
        format!("{rust}\nfn main() {{ println!(\"{{}}\", {call}); }}\n"),
    )
    .unwrap();

    let debug_flag = if debug_assertions {
        "-Cdebug-assertions=yes"
    } else {
        "-Cdebug-assertions=no"
    };

    let compiled = Command::new("rustc")
        .args(["--edition=2021", "-A", "warnings", debug_flag])
        .arg(&rust_path)
        .arg("-o")
        .arg(&exe_path)
        .output()
        .expect("failed to execute rustc");

    assert!(
        compiled.status.success(),
        "rustc compilation failed: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );

    let ran = Command::new(&exe_path)
        .output()
        .expect("failed to run probe binary");

    let stdout = String::from_utf8_lossy(&ran.stdout).trim().to_string();
    (stdout, ran.status.success(), ran.status.code())
}

fn run_probe_binary(name: &str, source: &str, call: &str) -> (String, Option<i32>) {
    let (stdout, _success, code) = run_probe_binary_ext(name, source, call, false);
    (stdout, code)
}

#[test]
fn test_decimal_local_variable_scaling_preserves_micros() {
    let src = r#"
        fn multiply_local(a: Decimal, b: Decimal) -> Decimal
            requires { a == 1.5d, b == 2.0d }
            ensures { result == 3.0d }
        {
            let u = a;
            let v = b;
            u * v
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    assert!(
        proofs["multiply_local"].is_proved(),
        "multiply_local should be mathematically proved: {:?}",
        proofs
    );

    // 1.5d = 1_500_000, 2.0d = 2_000_000. 1.5d * 2.0d = 3.0d = 3_000_000 micros.
    let (out, code) =
        run_probe_binary("decimal_local", src, "multiply_local(1_500_000, 2_000_000)");
    assert_eq!(code, Some(0));
    assert_eq!(
        out, "3000000",
        "Decimal local variable scaling should evaluate to 3,000,000, got: {out}"
    );
}

#[test]
fn test_name_collision_does_not_pollute_integer_arithmetic() {
    let src = r#"
        fn decimal_identity(x: Decimal) -> Decimal { x }
        fn integer_double(x: Int) -> Int
            requires { x == 2 }
            ensures { result == 4 }
        {
            x * 2
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    assert!(
        proofs["integer_double"].is_proved(),
        "integer_double should be proved: {:?}",
        proofs
    );

    let (out, code) = run_probe_binary("name_collision", src, "integer_double(2)");
    assert_eq!(code, Some(0));
    assert_eq!(out, "4", "Integer arithmetic must not be divided by 1,000,000 due to cross-function decimal parameter name collision! Got: {out}");
}

#[test]
fn test_decimal_int_mixed_arithmetic_scaling() {
    let src = r#"
        fn decimal_double(a: Decimal, n: Int) -> Decimal
            requires { a == 1.5d, n == 2 }
            ensures { result == a * n }
        {
            a * n
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    assert!(
        proofs["decimal_double"].is_proved(),
        "decimal_double should be proved: {:?}",
        proofs
    );

    // a = 1.5d (1_500_000 micros), n = 2. a * n = 3_000_000 micros.
    let (out, code) = run_probe_binary("decimal_int", src, "decimal_double(1_500_000, 2)");
    assert_eq!(code, Some(0));
    assert_eq!(
        out, "3000000",
        "Decimal * Int should preserve micro-units without scaling division, got: {out}"
    );
}

#[test]
fn test_float_fail_closed_prevents_false_proof() {
    let src = r#"
        fn next_float(x: Float) -> Float
            requires { x >= 0.0 }
            ensures { result > x }
        {
            x + 1.0
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    let res = &proofs["next_float"];
    assert!(!res.is_proved(), "Contracted Float function must NOT be proved under Real theory (Fail-Closed prevented IEEE-754 divergence)!");
    assert!(
        matches!(res, axiom_smt::SmtResult::Skipped(_)),
        "Should be Skipped, got: {:?}",
        res
    );
}

#[test]
fn test_bounded_float_contract_fails_closed() {
    // 针对 630ed5e 审计反例：即使 x 在 requires 中被精确约束为 0.1 (满足历史有界条件)，
    // 依然由于 IEEE-754 硬件舍入分歧 (0.1 + 0.2 != 0.3) 必须被 Fail-Closed 拦截为 Skipped！
    let src = r#"
        fn bounded(x: Float) -> Float
            requires { x == 0.1 }
            ensures { result == 0.3 }
        {
            x + 0.2
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    let res = &proofs["bounded"];
    assert!(
        !res.is_proved(),
        "Bounded float contract must NOT be proved!"
    );
    match res {
        axiom_smt::SmtResult::Skipped(reason) => {
            assert!(
                reason.contains("floating-point arithmetic is not verified soundly"),
                "Unexpected skip reason: {reason}"
            );
        }
        other => panic!("Expected Skipped, got {:?}", other),
    }
}

#[test]
fn test_call_argument_type_mismatch_fails_smt_and_transpiler() {
    // 外部审查 934d05e P0 核心反例：
    // helper 期望 Decimal 形参，wrapper 将 Int 实参 n 传入。
    // 在旧逻辑中，SMT 内联跨类型代入误判为 Proved，而 Rust 转译裸传导致运行时契约失败。
    // 修复后：SMT 求解前门禁与内联门禁严格拦截为 Skipped，Transpiler 同步拦截拒绝生成代码！
    let src = r#"
        fn helper(x: Decimal) -> Decimal {
            x * 2.0d
        }

        fn wrapper(n: Int) -> Decimal
            requires { n == 1500000 }
            ensures { result == 3000000.0d }
        {
            helper(n)
        }
    "#;
    let module = parse_module(src).unwrap();

    // 1. SMT 必须拒绝证明 (Fail-Closed Skipped)
    let proofs = verify_module(&module);
    let res = &proofs["wrapper"];
    assert!(
        !res.is_proved(),
        "wrapper calling helper with mismatched argument type MUST NOT be proved! Got: {:?}",
        res
    );
    match res {
        axiom_smt::SmtResult::Skipped(reason) => {
            assert!(
                reason.contains("call argument type mismatch"),
                "Expected call argument type mismatch in skip reason, got: {reason}"
            );
        }
        other => panic!("Expected SmtResult::Skipped, got: {:?}", other),
    }

    // 2. 转译器必须拒绝生成代码
    let transpile_res = transpile_to_rust(&module);
    assert!(
        transpile_res.is_err(),
        "Transpiler must reject code with call argument type mismatch!"
    );
}

#[test]
fn test_decimal_return_type_int_literal_fail_closed_across_all_layers() {
    let src = r#"
        fn helper() -> Decimal {
            2
        }

        fn wrapper() -> Decimal
            ensures {
                result == 4.0d
            }
        {
            helper() * 2.0d
        }
    "#;

    // 1. SMT 门禁：直接 Fail-Closed Skipped，绝不误签发 Proved
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    let res = &proofs["wrapper"];
    assert!(
        !res.is_proved(),
        "wrapper calling helper with mismatched return type MUST NOT be proved! Got: {:?}",
        res
    );
    match res {
        axiom_smt::SmtResult::Skipped(reason) => {
            assert!(
                reason.contains("return type mismatch"),
                "Expected return type mismatch in skip reason, got: {reason}"
            );
        }
        other => panic!("Expected SmtResult::Skipped, got: {:?}", other),
    }

    // 2. 转译器门禁：直接拒绝生成代码
    let transpile_res = transpile_to_rust(&module);
    assert!(
        transpile_res.is_err(),
        "Transpiler must reject function declaring Decimal return but returning Int literal"
    );
}

#[test]
fn test_differential_hygienic_inlining_and_decimal_execution() {
    let src = r#"
        fn apply_markup(amount: Decimal) -> Decimal {
            let bonus = 5.0d;
            amount + bonus
        }

        fn compute_total(bonus: Decimal) -> Decimal
            requires { bonus == 10.0d }
            ensures { result == 15.0d }
        {
            apply_markup(bonus)
        }
    "#;
    let module = parse_module(src).unwrap();

    // 1. SMT 验证：在健全卫生内联下，apply_markup 内部的 bonus (5.0d) 不会捕获实参 bonus (10.0d)
    // SMT 必须证明 result == 10.0d + 5.0d == 15.0d (Proved)
    let proofs = verify_module(&module);
    assert!(
        proofs["compute_total"].is_proved(),
        "compute_total with hygienic inlining must be proved! Got: {:?}",
        proofs
    );

    // 2. Transpiler + rustc 编译并在操作系统中执行
    // 10.0d = 10_000_000 微元，返回值应为 15_000_000 微元
    let (out, code) = run_probe_binary("diff_hygiene", src, "compute_total(10_000_000)");
    assert_eq!(code, Some(0));
    assert_eq!(
        out, "15000000",
        "Differential execution: Rust compiled runtime result must match SMT proved result (15_000_000 micros), got: {out}"
    );
}

#[test]
fn test_differential_inlining_unused_param_division_by_zero_counterexample_and_panic() {
    let src = r#"
        fn helper(x: Int) -> Int ensures { result == 1 } {
            1
        }
        fn wrapper(n: Int) -> Int ensures { result == 1 } {
            helper(1 / n)
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    // 1. SMT 形式化验证：必须截获除零安全风险并输出 CounterExample，拒绝误判 Proved
    assert!(
        matches!(proofs["wrapper"], axiom_smt::SmtResult::CounterExample(_)),
        "SMT must produce CounterExample on wrapper(n) due to 1/n division-by-zero risk, got: {:?}",
        proofs["wrapper"]
    );

    // 2. Transpiled Rust 编译并在操作系统中真实执行：
    // 当传入 n = 0 时，Rust 执行 helper(1 / 0) 触发除零 panic (status.success() == false)
    let (_out_zero, success_zero, _code_zero) =
        run_probe_binary_ext("diff_div_zero", src, "wrapper(0)", false);
    assert!(
        !success_zero,
        "Running wrapper(0) in compiled Rust must panic on division by zero!"
    );

    // 当传入 n = 2 时，Rust 运行正常返回 1
    let (out_safe, success_safe, _code_safe) =
        run_probe_binary_ext("diff_div_safe", src, "wrapper(2)", false);
    assert!(success_safe, "Running wrapper(2) must succeed!");
    assert_eq!(out_safe, "1");
}

#[test]
fn test_differential_hygienic_renamer_avoids_user_shadowing() {
    let src = r#"
        fn helper(x: Int) -> Int ensures { result == x + 1 } {
            let y = 1;
            x + y
        }
        fn wrapper() -> Int ensures { result == 2 } {
            let y__axiom_in_1_1 = 2;
            helper(y__axiom_in_1_1)
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    // 1. SMT 验证：因为 2 + 1 = 3 != 2，绝不会与用户同名变量冲突，必须报 CounterExample
    assert!(
        matches!(proofs["wrapper"], axiom_smt::SmtResult::CounterExample(_)),
        "wrapper must produce CounterExample because 2 + 1 = 3 != 2, got: {:?}",
        proofs["wrapper"]
    );

    // 2. Transpiled Rust 编译运行：输出真实为 3，与 SMT 严格一致
    let (out, success, _) = run_probe_binary_ext("diff_hygiene_collision", src, "wrapper()", false);
    assert!(success);
    assert_eq!(
        out, "3",
        "Differential execution: Rust compiled runtime result must evaluate to 3, matching SMT mathematical semantics!"
    );
}

#[test]
fn test_differential_callee_precondition_counterexample_and_debug_assert() {
    let src = r#"
        fn helper(x: Int) -> Int requires { x > 0 } ensures { result == x } {
            x
        }
        fn wrapper() -> Int ensures { result == 0 } {
            helper(0)
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    // 1. SMT 验证：调用点传入 0 违背了 helper 的前置条件 x > 0，必须拒绝通过并返回 CounterExample
    assert!(
        matches!(proofs["wrapper"], axiom_smt::SmtResult::CounterExample(_)),
        "SMT must produce CounterExample because helper requires x > 0 but was called with 0, got: {:?}",
        proofs["wrapper"]
    );

    // 2. Transpiled Rust 在启用 debug_assertions 下运行：触发 helper 的 debug_assert!(x > 0) panic！
    let (_out_debug, success_debug, _) =
        run_probe_binary_ext("diff_callee_precond", src, "wrapper()", true);
    assert!(
        !success_debug,
        "Transpiled Rust under debug-assertions must panic when callee precondition is violated!"
    );
}

#[test]
fn test_differential_lexical_scoping_shadowing_counterexample_and_panic() {
    let src = r#"
        fn helper(x: Int) -> Int requires { x > 0 } ensures { result == x } {
            x
        }
        fn wrapper(n: Int) -> Int requires { n > 0 } ensures { result == -1 } {
            let n = -1;
            helper(n)
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);

    // 1. SMT 验证：局部 let n = -1 遮蔽了顶层参数 n，导致 helper(n) 实参为 -1 违背 x > 0，必须拒绝通过产出 CounterExample！
    assert!(
        matches!(proofs["wrapper"], axiom_smt::SmtResult::CounterExample(_)),
        "SMT must produce CounterExample because local let n = -1 violates helper's requires x > 0, got: {:?}",
        proofs["wrapper"]
    );

    // 2. Transpiled Rust：调用 wrapper(10)，运行时局部变量 n = -1 传入 helper(-1) 触发 debug_assert!(x > 0) panic！
    let (_out, success, _) = run_probe_binary_ext("diff_lexical_shadow", src, "wrapper(10)", true);
    assert!(
        !success,
        "Transpiled Rust must panic at runtime due to callee precondition violation from shadowed variable!"
    );
}

#[test]
fn test_differential_requires_nested_call_well_formedness_counterexample_and_panic() {
    let src = r#"
        fn helper(x: Int) -> Int requires { x > 0 } ensures { result == x } {
            x
        }
        fn wrapper() -> Int requires { helper(0) == 0 } ensures { result == 0 } {
            0
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);

    // 1. SMT 验证：前置条件自身调用 helper(0) 违背良好定义性，必须报 CounterExample！
    assert!(
        matches!(proofs["wrapper"], axiom_smt::SmtResult::CounterExample(_)),
        "SMT must produce CounterExample because requires clause calls helper(0) violating x > 0, got: {:?}",
        proofs["wrapper"]
    );

    // 2. Transpiled Rust：调用 wrapper()，在执行 requires 检查时先计算 helper(0)，触发 helper 的 debug_assert panic！
    let (_out, success, _) = run_probe_binary_ext("diff_req_wf", src, "wrapper()", true);
    assert!(
        !success,
        "Transpiled Rust must panic when evaluating ill-formed precondition calling helper(0)!"
    );
}

#[test]
fn test_differential_ensures_nested_call_counterexample_and_panic() {
    let src = r#"
        fn helper(x: Int) -> Int requires { x > 0 } ensures { result == x } {
            x
        }
        fn wrapper() -> Int requires { true } ensures { helper(result) > 0 } {
            0
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);

    // 1. SMT 验证：后置条件 helper(result) 在 result=0 时违背 helper 的 requires x > 0，必须报 CounterExample！
    assert!(
        matches!(proofs["wrapper"], axiom_smt::SmtResult::CounterExample(_)),
        "SMT must produce CounterExample because result=0 violates helper's requires x > 0 in ensures, got: {:?}",
        proofs["wrapper"]
    );

    // 2. Transpiled Rust：调用 wrapper() 返回 0，在校验 ensures 时调用 helper(0)，触发 panic！
    let (_out, success, _) = run_probe_binary_ext("diff_ens_nested", src, "wrapper()", true);
    assert!(
        !success,
        "Transpiled Rust must panic when evaluating ensures calling helper(0)!"
    );
}
