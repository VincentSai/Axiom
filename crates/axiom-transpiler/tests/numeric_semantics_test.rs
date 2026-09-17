use axiom_parser::parse_module;
use axiom_smt::verify_module;
use axiom_transpiler::transpile_to_rust;
use std::{fs, process::Command};

fn run_probe_binary(name: &str, source: &str, call: &str) -> (String, Option<i32>) {
    let module = parse_module(source).expect("probe must parse");
    let rust = transpile_to_rust(&module).expect("probe must transpile");

    let temp_dir = std::env::temp_dir().join(format!("axiom_numeric_probe_{}", std::process::id()));
    fs::create_dir_all(&temp_dir).unwrap();

    let rust_path = temp_dir.join(format!("{name}.rs"));
    let exe_path = temp_dir.join(format!("{name}.exe"));
    fs::write(
        &rust_path,
        format!("{rust}\nfn main() {{ println!(\"{{}}\", {call}); }}\n"),
    )
    .unwrap();

    let compiled = Command::new("rustc")
        .args([
            "--edition=2021",
            "-A",
            "warnings",
            "-C",
            "debug-assertions=no",
        ])
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
    (stdout, ran.status.code())
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
