use axiom_wit::generate_wit_from_source;

#[test]
fn generates_records_enums_and_variants() {
    let source = r#"
        type UserProfile = {
            user_id: Int,
            display_name: String,
            active: Bool,
        }

        type TrafficLight = | Red | Yellow | Green

        type LookupResult =
            | Found(profile: UserProfile)
            | Missing
            | Invalid(code: Int, message: String)

        fn lookup_user(user_id: Int) -> LookupResult { ?implementation }
    "#;

    let wit = generate_wit_from_source(source, "example:profiles").unwrap();

    assert!(wit.starts_with("package example:profiles;\n"));
    assert!(wit.contains("record user-profile {"));
    assert!(wit.contains("user-id: s64,"));
    assert!(wit.contains("display-name: string,"));
    assert!(wit.contains("active: bool,"));
    assert!(wit.contains("enum traffic-light {"));
    assert!(wit.contains("variant lookup-result {"));
    assert!(wit.contains("found(user-profile),"));
    assert!(wit.contains("invalid(tuple<s64, string>),"));
    assert!(wit.contains("export lookup-user: func(user-id: s64) -> lookup-result;"));
}

#[test]
fn generates_transfer_effect_interfaces_and_export() {
    let source = include_str!("../../../examples/transfer.ax");

    let wit = generate_wit_from_source(source, "axiom:transfer").unwrap();

    assert!(wit.contains("record account {"));
    assert!(wit.contains("record transfer-receipt {"));
    assert!(wit.contains("variant transfer-error {"));
    assert!(wit.contains("interface database {"));
    assert!(wit.contains("find-account: func(id: s64) -> option<account>;"));
    assert!(wit.contains("update-account: func(account: account) -> result<(), string>;"));
    assert!(wit.contains("interface clock {"));
    assert!(wit.contains("now-timestamp: func() -> s64;"));
    assert!(wit.contains("interface audit-log {"));
    assert!(wit.contains("info: func(msg: string) -> ();"));
    assert!(wit.contains("import database;"));
    assert!(wit.contains("import clock;"));
    assert!(wit.contains("import audit-log;"));
    assert!(wit.contains(
        "export execute-transfer: func(from-id: s64, to-id: s64, amount: s64) -> result<transfer-receipt, transfer-error>;"
    ));
}

#[test]
fn generates_game_loop_types_effects_and_exports() {
    let source = include_str!("../../../examples/game_loop.ax");

    let wit = generate_wit_from_source(source, "game-loop").unwrap();

    assert!(wit.starts_with("package axiom:game-loop;\n"));
    assert!(wit.contains("record vector2 {"));
    assert!(wit.contains("record entity {"));
    assert!(wit.contains("record world-state {"));
    assert!(wit.contains("enemies: list<entity>,"));
    assert!(wit.contains("interface render {"));
    assert!(wit.contains("draw-sprite: func(id: s64, pos: vector2) -> ();"));
    assert!(wit.contains("interface input {"));
    assert!(wit.contains("get-movement-axis: func() -> vector2;"));
    assert!(wit.contains("export update-physics: func(entity: entity, dt: f64) -> entity;"));
    assert!(wit.contains("export tick-frame: func(world: world-state, dt: f64) -> world-state;"));
}
