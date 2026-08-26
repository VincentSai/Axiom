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
    assert_eq!(result, SmtResult::Verified);
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

    assert_eq!(verify_function(&module.functions[0]), SmtResult::Verified);
}
