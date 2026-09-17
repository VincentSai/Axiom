use axiom_parser::parse_module;
use axiom_smt::verify_module;
use axiom_transpiler::transpile_to_rust;
use std::{fs, process::Command};

fn run_probe_binary(name: &str, source: &str, call: &str) -> (String, Option<i32>) {
    let module = parse_module(source).expect("probe must parse");
    let rust = transpile_to_rust(&module).expect("probe must transpile");

    let temp_dir =
        std::env::temp_dir().join(format!("axiom_typed_ir_probe_{}", std::process::id()));
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
fn test_multiply_literal_context_refinement_rejects_unsound_postcondition() {
    // 1. 错误的后置条件必须被 SMT 拒绝证明
    let unsound_src = r#"
        fn multiply_literal_unsound(x: Decimal) -> Decimal
            requires { x >= 0.0, x <= 100.0 }
            ensures { result == x * 1.5 + 1.0 }
        {
            x * 1.5
        }
    "#;
    let module_unsound = parse_module(unsound_src).unwrap();
    let proofs_unsound = verify_module(&module_unsound);
    assert!(
        !proofs_unsound["multiply_literal_unsound"].is_proved(),
        "SMT must reject false postcondition for literal multiplication: {:?}",
        proofs_unsound
    );

    // 2. 正确的后置条件必须被证明通过，并且 Rust 执行结果为 3,000,000 微元
    let sound_src = r#"
        fn multiply_literal_sound(x: Decimal) -> Decimal
            requires { x >= 0.0, x <= 100.0 }
            ensures { result == x * 1.5 }
        {
            x * 1.5
        }
    "#;
    let module_sound = parse_module(sound_src).unwrap();
    let proofs_sound = verify_module(&module_sound);
    assert!(
        proofs_sound["multiply_literal_sound"].is_proved(),
        "SMT must prove sound literal multiplication: {:?}",
        proofs_sound
    );

    let (out, code) = run_probe_binary(
        "test_multiply_literal",
        sound_src,
        "multiply_literal_sound(2_000_000)",
    );
    assert_eq!(code, Some(0));
    assert_eq!(
        out, "3000000",
        "2.0d * 1.5 should evaluate to 3.0d (3_000_000 micros), got: {out}"
    );
}

#[test]
fn test_large_local_let_binding_proves_and_runs_without_spurious_overflow() {
    let src = r#"
        fn large_local(x: Decimal) -> Decimal
            requires { x >= 0.0, x <= 10000.0 }
            ensures { result == x * 100000.0 }
        {
            let y = x * 100000.0;
            y
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    assert!(
        proofs["large_local"].is_proved(),
        "large_local with intermediate let binding must be proved without spurious overflow: {:?}",
        proofs
    );

    let (out, code) = run_probe_binary("test_large_local", src, "large_local(2_000_000)");
    assert_eq!(code, Some(0));
    // 2.0d * 100000.0 = 200,000.0d = 200_000_000_000 micros
    assert_eq!(
        out, "200000000000",
        "2.0d * 100000.0 must evaluate to 200_000_000_000 micros, got: {out}"
    );
}

#[test]
fn test_decimal_match_pattern_variable_correctly_scaled() {
    let src = r#"
        fn decimal_match(x: Decimal) -> Decimal
            requires { x >= 0.0, x <= 100.0 }
            ensures { result == x + 2.0 }
        {
            match x {
                n => n + 2.0
            }
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    assert!(
        proofs["decimal_match"].is_proved(),
        "decimal_match must be proved: {:?}",
        proofs
    );

    let (out, code) = run_probe_binary("test_decimal_match", src, "decimal_match(1_000_000)");
    assert_eq!(code, Some(0));
    // 1.0d + 2.0 = 3.0d = 3_000_000 micros (must not be 1000002)
    assert_eq!(
        out, "3000000",
        "decimal match pattern variable must have Decimal type and yield 3,000,000 micros, got: {out}"
    );
}

#[test]
fn test_int_decimal_mixed_divide_proves_and_runs_correctly() {
    let src = r#"
        fn int_dec_div(n: Int, d: Decimal) -> Decimal
            requires { n == 1, d == 0.5 }
            ensures { result == 2.0 }
        {
            n / d
        }
    "#;
    let module = parse_module(src).unwrap();
    let proofs = verify_module(&module);
    assert!(
        proofs["int_dec_div"].is_proved(),
        "int_dec_div must be proved by SMT: {:?}",
        proofs
    );

    let (out, code) = run_probe_binary("test_int_dec_div", src, "int_dec_div(1, 500_000)");
    assert_eq!(code, Some(0));
    // 1 / 0.5 = 2.0d = 2_000_000 micros (must not truncate to 0)
    assert_eq!(
        out, "2000000",
        "1 / 0.5d must evaluate to 2.0d (2_000_000 micros), got: {out}"
    );
}
