use axiom_diagnostics::{diagnose_source, DiagnosticStatus};

#[test]
fn invalid_source_returns_a_json_parse_error() {
    let reports = diagnose_source("fn broken( -> Int { 1 }");

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, DiagnosticStatus::ParseError);
    let json: serde_json::Value =
        serde_json::from_str(&reports[0].to_json()).expect("diagnostic should be valid JSON");
    assert_eq!(json["status"], "ParseError");
    assert!(json["diagnosis"]
        .as_str()
        .is_some_and(|text| !text.is_empty()));
}

#[test]
fn undeclared_effect_returns_an_effect_violation_json_report() {
    let reports = diagnose_source(
        r#"
        effect Database { fn read(): Int }
        fn load() -> Int { Database.read() }
        "#,
    );

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, DiagnosticStatus::EffectViolation);
    let json: serde_json::Value = serde_json::from_str(&reports[0].to_json_pretty())
        .expect("diagnostic should be valid JSON");
    assert_eq!(json["status"], "EffectViolation");
    assert_eq!(json["function_name"], "load");
}

#[test]
fn faulty_contract_returns_a_counter_example_json_report() {
    let reports = diagnose_source(
        r#"
        fn withdraw(balance: Decimal, amount: Decimal) -> Decimal
            requires { balance >= 0.0, amount > 0.0 }
            ensures { result >= 0.0 }
        {
            balance - amount
        }
        "#,
    );

    assert_eq!(reports.len(), 1);
    let report = &reports[0];
    assert_eq!(report.status, DiagnosticStatus::ContractViolation);
    assert_eq!(report.function_name.as_deref(), Some("withdraw"));
    assert_eq!(
        report.violated_clause.as_deref(),
        Some("ensures: result >= 0.0")
    );
    assert!(report
        .counter_example
        .as_ref()
        .is_some_and(|values| { values.contains_key("balance") && values.contains_key("amount") }));

    let json: serde_json::Value =
        serde_json::from_str(&report.to_json()).expect("diagnostic should be valid JSON");
    assert_eq!(json["status"], "ContractViolation");
    assert!(json["counter_example"].is_object());
}

#[test]
fn valid_contract_returns_a_verified_report() {
    let reports = diagnose_source(
        r#"
        fn identity(value: Int) -> Int
            requires { value >= 0 }
            ensures { result == value }
        {
            value
        }
        "#,
    );

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, DiagnosticStatus::Verified);
    assert_eq!(reports[0].function_name.as_deref(), Some("identity"));
    let json: serde_json::Value =
        serde_json::from_str(&reports[0].to_json()).expect("diagnostic should be valid JSON");
    assert_eq!(json["status"], "Verified");
    assert!(json["counter_example"].is_null());
}

#[test]
fn unresolved_implementation_returns_a_hole_report_before_smt_verification() {
    let reports = diagnose_source(
        "fn pending(value: Int) -> Int ensures { result > value } { ?implementation }",
    );

    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0].status, DiagnosticStatus::UnresolvedHole);
    assert_eq!(reports[0].function_name.as_deref(), Some("pending"));
    assert_eq!(reports[0].hole_id.as_deref(), Some("implementation"));
}
