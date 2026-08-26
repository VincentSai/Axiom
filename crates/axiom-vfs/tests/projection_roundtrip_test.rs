use axiom_ast::{BinaryOperator, Expr, Pattern};
use axiom_db::{compute_module_hash, AstDatabase};
use axiom_parser::parse_module;
use axiom_vfs::{
    export_database_module, export_module_to_source, sync_source_to_database, VfsError,
};

#[test]
fn examples_round_trip_through_virtual_source() {
    for source in [
        include_str!("../../../examples/transfer.ax"),
        include_str!("../../../examples/game_loop.ax"),
    ] {
        let original = parse_module(source).expect("example should parse");
        let projected = export_module_to_source(&original);
        let reparsed = parse_module(&projected).expect("projected source should parse");

        assert_eq!(reparsed, original);
        assert_eq!(
            compute_module_hash(&reparsed),
            compute_module_hash(&original)
        );
    }
}

#[test]
fn structured_language_features_round_trip_without_hash_drift() {
    let source = r#"
        type Shape = | Circle(radius: Float) | Point
        type Drawing = { shape: Shape, visible: Bool }

        effect Render {
            fn draw(drawing: Drawing): ()
        }

        fn update(drawing: Drawing, shape: Shape) -> Drawing
            with [Render]
            requires { drawing.visible || shape is Point }
            ensures { result.visible && drawing.visible => result.shape is shape }
        {
            let next = { ...drawing, shape: shape };
            match shape {
                Shape.Circle(radius) => { ...next, visible: radius > 0.0 },
                Shape.Point => next,
            }
        }
    "#;
    let original = parse_module(source).expect("feature sample should parse");
    let projected = export_module_to_source(&original);
    let reparsed = parse_module(&projected).expect("projected feature sample should parse");

    assert_eq!(reparsed, original);
    assert_eq!(
        compute_module_hash(&reparsed),
        compute_module_hash(&original)
    );

    let Expr::Block {
        tail: Some(tail), ..
    } = &reparsed.functions[0].body
    else {
        panic!("expected a block with a match tail");
    };
    let Expr::Match { arms, .. } = tail.as_ref() else {
        panic!("expected pattern matching to survive projection");
    };
    assert!(matches!(
        arms[0].pattern,
        Pattern::Variant {
            enum_name: Some(_),
            ..
        }
    ));
    assert!(matches!(
        reparsed.functions[0].requires[0],
        Expr::BinaryOp {
            op: BinaryOperator::Or,
            ..
        }
    ));
}

#[test]
fn database_sync_export_and_resync_preserve_the_module_hash() {
    let source = include_str!("../../../examples/game_loop.ax");
    let mut database = AstDatabase::in_memory();

    let first = sync_source_to_database(&mut database, source).expect("source should synchronize");
    assert_eq!(first.type_count, 3);
    assert_eq!(first.function_count, 2);
    assert_eq!(first.effect_count, 2);
    assert_eq!(first.type_hashes.len(), first.type_count);
    assert_eq!(first.function_hashes.len(), first.function_count);
    assert_eq!(first.effect_hashes.len(), first.effect_count);

    let projected =
        export_database_module(&database, &first.module_hash).expect("stored module should export");
    let second = sync_source_to_database(&mut database, &projected)
        .expect("projected source should synchronize");

    assert_eq!(second.module_hash, first.module_hash);
    assert_eq!(second, first);
}

#[test]
fn exporting_an_unknown_hash_reports_the_missing_module() {
    let database = AstDatabase::in_memory();
    let source = parse_module("fn sample() -> Int { 1 }").unwrap();
    let unknown = compute_module_hash(&source);

    let error = export_database_module(&database, &unknown).unwrap_err();
    assert!(matches!(error, VfsError::ModuleNotFound(hash) if hash == unknown));
}
