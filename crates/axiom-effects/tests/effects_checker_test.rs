use axiom_effects::{
    check_module_effects, EffectError, EffectHandler, HandlerError, HandlerRegistry,
};
use axiom_parser::parse_module;

#[test]
fn accepts_calls_when_the_caller_declares_the_callee_effects() {
    let module = parse_module(
        r#"
            effect Database { fn read(): Int }
            fn load() -> Int with [Database] { Database.read() }
            fn entry() -> Int with [Database] { load() }
        "#,
    )
    .expect("module should parse");

    assert_eq!(check_module_effects(&module), Ok(()));
}

#[test]
fn rejects_a_call_when_the_caller_does_not_propagate_an_effect() {
    let module = parse_module(
        r#"
            effect Database { fn read(): Int }
            fn load() -> Int with [Database] { Database.read() }
            fn entry() -> Int { load() }
        "#,
    )
    .expect("module should parse");

    assert_eq!(
        check_module_effects(&module),
        Err(EffectError::UndeclaredEffect {
            function: "entry".into(),
            effect: "Database".into(),
            source: "load".into(),
        })
    );
}

#[test]
fn rejects_effect_names_without_a_declaration() {
    let module =
        parse_module("fn load() -> Int with [Database] { 1 }").expect("module should parse");

    assert!(matches!(
        check_module_effects(&module),
        Err(EffectError::UndefinedEffect { function, effect })
            if function == "load" && effect == "Database"
    ));
}

struct MockHandler(&'static str);

impl EffectHandler for MockHandler {
    fn effect_name(&self) -> &str {
        self.0
    }
}

#[test]
fn transfer_example_checks_and_resolves_mock_handlers() {
    let module = parse_module(include_str!("../../../examples/transfer.ax"))
        .expect("transfer example should parse");
    check_module_effects(&module).expect("transfer effects should be valid");

    let transfer = &module.functions[0];
    assert_eq!(transfer.effects, ["Database", "Clock", "AuditLog"]);

    let mut handlers = HandlerRegistry::new();
    handlers.register(MockHandler("Database"));
    handlers.register(MockHandler("Clock"));
    handlers.register(MockHandler("AuditLog"));

    let resolved = handlers
        .resolve_for_function(transfer)
        .expect("all required handlers should resolve");
    assert_eq!(
        resolved
            .iter()
            .map(|handler| handler.effect_name())
            .collect::<Vec<_>>(),
        ["Database", "Clock", "AuditLog"]
    );
}

#[test]
fn reports_a_missing_runtime_handler() {
    let registry = HandlerRegistry::new();

    assert!(matches!(
        registry.resolve(["Clock"]),
        Err(HandlerError::MissingHandler { effect }) if effect == "Clock"
    ));
}

#[test]
fn rejects_unauthorized_host_io_calls() {
    let source = r#"
        fn dangerous_io() -> () {
            if true {
                std::fs::write("hack.txt", "payload")
            } else {
                ()
            }
        }
    "#;
    let module = parse_module(source).expect("should parse");
    let result = check_module_effects(&module);
    assert!(matches!(
        result,
        Err(EffectError::DisallowedExternalCall { function, target })
            if function == "dangerous_io" && target == "std::fs::write"
    ));
}
