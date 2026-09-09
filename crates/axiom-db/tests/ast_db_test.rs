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

#[test]
fn verifiable_change_protocol_cas_flow() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, Candidate, ProofLevel, TaskEnvelope,
        VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_module();
    let mod_rec = db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();
    assert_eq!(initial_root, mod_rec.module_hash);

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "?hole_1".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "account.balance + 10".into(),
        expression_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        author_agent_id: "agent_worker_42".into(),
        timestamp: 1700000000,
    };
    let cand_hash = compute_candidate_hash(&candidate);

    let receipt = VerificationReceipt {
        candidate_hash: cand_hash.clone(),
        proof_level: ProofLevel::Proved,
        solver: "Z3".into(),
        solver_version: "4.13.0".into(),
        obligations_checked: 2,
        proof_obligation_hash: cand_hash.clone(),
        verified_at: 1700000001,
    };

    let mut updated_mod = initial_mod.clone();
    updated_mod.functions[0].name = "new_balance".into();

    // 1. CAS 成功提交
    let commit = db
        .try_commit_candidate(
            &Some(initial_root.clone()),
            envelope.clone(),
            candidate.clone(),
            receipt.clone(),
            &updated_mod,
        )
        .expect("CAS should succeed");

    assert_eq!(commit.parent_root, Some(initial_root));
    assert_ne!(commit.new_root, commit.parent_root.clone().unwrap());
    assert_eq!(db.current_root().unwrap(), commit.new_root);
    assert_eq!(db.get_candidate(&cand_hash).unwrap(), Some(candidate.clone()));
    assert_eq!(db.get_receipt(&commit.receipt_hash).unwrap(), Some(receipt));
    assert_eq!(db.get_commit(&commit.commit_hash).unwrap(), Some(commit.clone()));
}

#[test]
fn cas_stale_root_preserves_candidates_without_silent_overwrite() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, Candidate, CasError, ProofLevel, TaskEnvelope,
        VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_module();
    let mod_rec = db.insert_module(&initial_mod).unwrap();
    let stale_root = mod_rec.module_hash;

    // 另一 Worker 先行推进了 root
    let mut intermediate_mod = initial_mod.clone();
    intermediate_mod.functions[0].name = "concurrent_worker_advanced".into();
    let inter_rec = db.insert_module(&intermediate_mod).unwrap();
    db.set_root(inter_rec.module_hash.clone()).unwrap();
    let current_root = db.current_root().unwrap();
    assert_ne!(current_root, stale_root);

    let envelope = TaskEnvelope {
        base_root_hash: Some(stale_root.clone()),
        target_hole_id: "?hole_1".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "stale_attempt()".into(),
        expression_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        author_agent_id: "late_worker".into(),
        timestamp: 1700000050,
    };
    let cand_hash = compute_candidate_hash(&candidate);

    let receipt = VerificationReceipt {
        candidate_hash: cand_hash.clone(),
        proof_level: ProofLevel::Proved,
        solver: "Z3".into(),
        solver_version: "4.13.0".into(),
        obligations_checked: 1,
        proof_obligation_hash: cand_hash.clone(),
        verified_at: 1700000051,
    };

    // 试图在过期的 stale_root 上提交 -> 必须返回 StaleRoot 错误
    let res = db.try_commit_candidate(
        &Some(stale_root.clone()),
        envelope,
        candidate.clone(),
        receipt,
        &initial_mod,
    );

    match res {
        Err(CasError::StaleRoot { current, expected }) => {
            assert_eq!(current, Some(current_root));
            assert_eq!(expected, Some(stale_root));
        }
        other => panic!("expected StaleRoot error, got: {:?}", other),
    }

    // 关键安全验收标准：Candidate 绝不丢失！必须可在数据库中被检索，供后续重验或回溯
    assert_eq!(db.get_candidate(&cand_hash).unwrap(), Some(candidate));
}

#[test]
fn evidence_gate_rejects_sampled_proof_from_release() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, Candidate, CasError, ProofLevel, TaskEnvelope,
        VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_module();
    db.insert_module(&initial_mod).unwrap();
    let root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(root.clone()),
        target_hole_id: "?hole_1".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let candidate = Candidate {
        envelope_hash: env_hash,
        expression_source: "sampled_expr()".into(),
        expression_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        author_agent_id: "heuristic_worker".into(),
        timestamp: 1700000100,
    };
    let cand_hash = compute_candidate_hash(&candidate);

    // Sampled 级别
    let receipt = VerificationReceipt {
        candidate_hash: cand_hash,
        proof_level: ProofLevel::Sampled,
        solver: "InternalSymbolic".into(),
        solver_version: "0.1.0".into(),
        obligations_checked: 10,
        proof_obligation_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        verified_at: 1700000101,
    };

    let res = db.try_commit_candidate(
        &Some(root),
        envelope,
        candidate,
        receipt,
        &initial_mod,
    );

    match res {
        Err(CasError::UnverifiedCandidate(msg)) => {
            assert!(msg.contains("Sampled"));
        }
        other => panic!("expected UnverifiedCandidate error for Sampled proof, got: {:?}", other),
    }
}

