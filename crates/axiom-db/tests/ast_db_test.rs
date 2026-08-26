use axiom_ast::{EffectDecl, EffectOp, Expr, FunctionDecl, FunctionParam, Module, Type, TypeDef};
use axiom_db::{
    compute_effect_hash, compute_function_hash, compute_module_hash, compute_type_hash, AstDatabase,
};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

fn sample_module() -> Module {
    Module {
        types: vec![TypeDef::Record {
            name: "Account".into(),
            fields: vec![axiom_ast::FieldDef {
                name: "balance".into(),
                ty: Type::Int,
            }],
        }],
        functions: vec![FunctionDecl {
            name: "balance".into(),
            params: vec![FunctionParam {
                name: "account".into(),
                ty: Type::Custom("Account".into()),
            }],
            return_type: Type::Int,
            effects: vec!["Audit".into()],
            requires: vec![],
            ensures: vec![],
            body: Expr::FieldAccess {
                target: Box::new(Expr::Ident("account".into())),
                field: "balance".into(),
            },
        }],
        effects: vec![EffectDecl {
            name: "Audit".into(),
            operations: vec![EffectOp {
                name: "record".into(),
                params: vec![FunctionParam {
                    name: "message".into(),
                    ty: Type::String,
                }],
                return_type: Type::Unit,
            }],
        }],
    }
}

fn temporary_database_path(test_name: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after Unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "axiom-db-{test_name}-{}-{nonce}.json",
        std::process::id()
    ))
}

#[test]
fn inserts_a_module_and_round_trips_every_ast_node() {
    let module = sample_module();
    let mut database = AstDatabase::in_memory();

    let record = database
        .insert_module(&module)
        .expect("insert should succeed");

    assert!(record.module_hash.as_str().starts_with("#mod_"));
    assert!(record.type_hashes[0].as_str().starts_with("#type_"));
    assert!(record.function_hashes[0].as_str().starts_with("#fn_"));
    assert!(record.effect_hashes[0].as_str().starts_with("#effect_"));
    assert_eq!(
        database.get_module(&record.module_hash).unwrap(),
        Some(module.clone())
    );
    assert_eq!(
        database.get_type(&record.type_hashes[0]).unwrap(),
        Some(module.types[0].clone())
    );
    assert_eq!(
        database.get_function(&record.function_hashes[0]).unwrap(),
        Some(module.functions[0].clone())
    );
    assert_eq!(
        database.get_effect(&record.effect_hashes[0]).unwrap(),
        Some(module.effects[0].clone())
    );
}

#[test]
fn hashes_are_stable_and_change_when_content_changes() {
    let module = sample_module();
    assert_eq!(
        compute_type_hash(&module.types[0]),
        compute_type_hash(&module.types[0])
    );
    assert_eq!(
        compute_function_hash(&module.functions[0]),
        compute_function_hash(&module.functions[0])
    );
    assert_eq!(
        compute_effect_hash(&module.effects[0]),
        compute_effect_hash(&module.effects[0])
    );
    assert_eq!(compute_module_hash(&module), compute_module_hash(&module));

    let mut changed = module.clone();
    changed.functions[0].name = "available_balance".into();
    assert_ne!(compute_module_hash(&module), compute_module_hash(&changed));
    assert_ne!(
        compute_function_hash(&module.functions[0]),
        compute_function_hash(&changed.functions[0])
    );
}

#[test]
fn textual_hashes_round_trip_and_reject_malformed_values() {
    let hash = compute_module_hash(&sample_module());
    assert_eq!(hash.as_str().parse::<axiom_db::NodeHash>().unwrap(), hash);

    for malformed in [
        "mod_deadbeef",
        "#unknown_deadbeef",
        "#mod_deadbeef",
        "#mod_ABCDEF0000000000000000000000000000000000000000000000000000",
    ] {
        assert!(malformed.parse::<axiom_db::NodeHash>().is_err());
    }
}

#[test]
fn persisted_database_can_reconstruct_the_original_module() {
    let path = temporary_database_path("persistence");
    let module = sample_module();
    let record = {
        let mut database = AstDatabase::open(&path).expect("database should open");
        database
            .insert_module(&module)
            .expect("insert should persist")
    };

    let reopened = AstDatabase::open(&path).expect("database should reopen");
    assert_eq!(
        reopened.get_module(&record.module_hash).unwrap(),
        Some(module)
    );

    std::fs::remove_file(path).expect("temporary database should be removable");
}
