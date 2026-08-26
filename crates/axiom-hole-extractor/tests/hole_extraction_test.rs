use axiom_hole_extractor::{extract_holes_from_source, Contracts, HoleParameter, HoleTask};

#[test]
fn extracts_single_hole_as_structured_task_and_json() {
    let tasks = extract_holes_from_source(
        "fn calculate_tax(income: Decimal) -> Decimal { ?tax_calculator_logic }",
    )
    .expect("the source should parse");

    let expected = HoleTask {
        schema: "https://axiom-lang.org/schemas/hole-task.json".into(),
        hole_id: "tax_calculator_logic".into(),
        target_function: "calculate_tax".into(),
        parameters: vec![HoleParameter {
            name: "income".into(),
            r#type: "Decimal".into(),
        }],
        return_type: "Decimal".into(),
        allowed_effects: vec![],
        contracts: Contracts {
            requires: vec![],
            ensures: vec![],
        },
    };

    assert_eq!(tasks, vec![expected.clone()]);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&expected.to_json())
            .expect("compact JSON should be valid"),
        serde_json::json!({
            "$schema": "https://axiom-lang.org/schemas/hole-task.json",
            "hole_id": "tax_calculator_logic",
            "target_function": "calculate_tax",
            "parameters": [{ "name": "income", "type": "Decimal" }],
            "return_type": "Decimal",
            "allowed_effects": [],
            "contracts": { "requires": [], "ensures": [] }
        })
    );
    assert!(expected.to_json_pretty().contains("\n  \"$schema\""));
}

#[test]
fn extracts_multiple_holes_nested_in_a_function_body() {
    let tasks = extract_holes_from_source(
        r#"
            fn process(input: String) -> String {
                let normalized = ?normalize_input;
                persist(normalized, ?persistence_logic)
            }
        "#,
    )
    .expect("the source should parse");

    assert_eq!(
        tasks
            .iter()
            .map(|task| (task.hole_id.as_str(), task.target_function.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("normalize_input", "process"),
            ("persistence_logic", "process"),
        ]
    );
}

#[test]
fn extracts_parameters_contracts_and_allowed_effects() {
    let tasks = extract_holes_from_source(
        r#"
            fn calculate_tax(income: Decimal, rate: Decimal) -> Decimal
                with [TaxTable, AuditLog]
                requires {
                    income >= 0.0,
                    rate >= 0.0 && rate <= 1.0,
                }
                ensures {
                    result >= 0.0,
                }
            {
                ?tax_calculator_logic
            }
        "#,
    )
    .expect("the annotated function should parse");

    assert_eq!(tasks.len(), 1);
    let task = &tasks[0];
    assert_eq!(
        task.parameters,
        vec![
            HoleParameter {
                name: "income".into(),
                r#type: "Decimal".into(),
            },
            HoleParameter {
                name: "rate".into(),
                r#type: "Decimal".into(),
            },
        ]
    );
    assert_eq!(task.return_type, "Decimal");
    assert_eq!(task.allowed_effects, vec!["TaxTable", "AuditLog"]);
    assert_eq!(
        task.contracts,
        Contracts {
            requires: vec!["income >= 0.0".into(), "rate >= 0.0 && rate <= 1.0".into(),],
            ensures: vec!["result >= 0.0".into()],
        }
    );
}
