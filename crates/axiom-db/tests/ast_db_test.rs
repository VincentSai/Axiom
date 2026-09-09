use axiom_ast::{EffectDecl, EffectOp, Expr, FunctionDecl, FunctionParam, Module, Type, TypeDef};
use axiom_db::{
    compute_effect_hash, compute_function_hash, compute_module_hash, compute_type_hash, AstDatabase,
    NodeHash,
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

fn sample_skeleton_module() -> Module {
    let mut module = sample_module();
    module.functions[0].body = Expr::Hole("balance_impl".into());
    module
}

#[test]
fn cas_commits_valid_candidate_and_mints_self_verifying_commit_record() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, compute_expr_hash, Candidate, ProofLevel,
        TaskEnvelope, VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_skeleton_module();
    let _mod_rec = db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "balance_impl".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "account.balance + 10";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();
    let expr_hash = compute_expr_hash(&parsed_expr);

    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: expr_src.into(),
        expression_hash: expr_hash,
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
    axiom_db::replace_hole_in_expr(&mut updated_mod.functions[0].body, "balance_impl", &parsed_expr);

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

    assert_eq!(commit.parent_root(), Some(&initial_root));
    assert_ne!(commit.new_root(), commit.parent_root().unwrap());
    assert_eq!(db.current_root().unwrap(), *commit.new_root());
    assert_eq!(db.get_candidate(&cand_hash).unwrap(), Some(candidate.clone()));
    assert_eq!(db.get_receipt(&commit.receipt_hash()).unwrap(), Some(receipt));
    assert_eq!(db.get_commit(&commit.commit_hash).unwrap(), Some(commit.clone()));

    // 2. 密码学自校验断言：CommitRecord 哈希必须能重新计算自验证
    assert!(commit.verify_self_hash());
    assert_eq!(axiom_db::compute_commit_hash(&commit), commit.commit_hash);
}

#[test]
fn cas_detects_and_rejects_forged_module() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, compute_expr_hash, Candidate, CasError,
        ProofLevel, TaskEnvelope, VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_skeleton_module();
    db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "balance_impl".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "account.balance + 10";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();

    let candidate = Candidate {
        envelope_hash: env_hash,
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "malicious_agent".into(),
        timestamp: 1700000000,
    };
    let cand_hash = compute_candidate_hash(&candidate);

    let receipt = VerificationReceipt {
        candidate_hash: cand_hash,
        proof_level: ProofLevel::Proved,
        solver: "Z3".into(),
        solver_version: "4.13.0".into(),
        obligations_checked: 2,
        proof_obligation_hash: NodeHash::default(),
        verified_at: 1700000001,
    };

    // 恶意调用者夹带私货：在 module 中篡改了函数名称或逻辑
    let mut forged_mod = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut forged_mod.functions[0].body, "balance_impl", &parsed_expr);
    forged_mod.functions[0].name = "unauthorized_name_change".into();

    let res = db.try_commit_candidate(
        &Some(initial_root),
        envelope,
        candidate,
        receipt,
        &forged_mod,
    );

    match res {
        Err(CasError::ForgedModule(msg)) => {
            assert!(msg.contains("does not match kernel-derived module"));
        }
        other => panic!("expected ForgedModule rejection, got: {:?}", other),
    }
}

#[test]
fn cas_stale_root_preserves_candidates_without_silent_overwrite() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, compute_expr_hash, Candidate, CasError,
        ProofLevel, TaskEnvelope, VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_skeleton_module();
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
        target_hole_id: "balance_impl".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "account.balance + 20";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();

    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
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

    // Candidate 绝不丢失！必须可在数据库中被检索，供后续重验或回溯
    assert_eq!(db.get_candidate(&cand_hash).unwrap(), Some(candidate));
}

#[test]
fn evidence_gate_rejects_sampled_proof_from_release() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, compute_expr_hash, Candidate, CasError,
        ProofLevel, TaskEnvelope, VerificationReceipt,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_skeleton_module();
    db.insert_module(&initial_mod).unwrap();
    let root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(root.clone()),
        target_hole_id: "balance_impl".into(),
        target_function: "balance".into(),
        contract_hash: axiom_db::compute_function_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "account.balance + 10";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();

    let candidate = Candidate {
        envelope_hash: env_hash,
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "heuristic_worker".into(),
        timestamp: 1700000100,
    };
    let cand_hash = compute_candidate_hash(&candidate);

    let receipt = VerificationReceipt {
        candidate_hash: cand_hash,
        proof_level: ProofLevel::Sampled,
        solver: "InternalSymbolic".into(),
        solver_version: "0.1.0".into(),
        obligations_checked: 10,
        proof_obligation_hash: NodeHash::default(),
        verified_at: 1700000101,
    };

    let mut derived_mod = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut derived_mod.functions[0].body, "balance_impl", &parsed_expr);

    let res = db.try_commit_candidate(
        &Some(root),
        envelope,
        candidate,
        receipt,
        &derived_mod,
    );

    match res {
        Err(CasError::UnverifiedCandidate(msg)) => {
            assert!(msg.contains("Sampled"));
        }
        other => panic!("expected UnverifiedCandidate error for Sampled proof, got: {:?}", other),
    }
}

#[test]
fn cross_instance_cas_file_locking_and_stale_root_detection() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, compute_expr_hash, Candidate, CasError,
        ProofLevel, TaskEnvelope, VerificationReceipt,
    };

    let db_path = temporary_database_path("cross_instance_test");

    // 实例 1 初始化物理文件数据库
    let mut db1 = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_skeleton_module();
    let mod_rec = db1.insert_module(&initial_mod).unwrap();
    let initial_root = mod_rec.module_hash;

    // 实例 2 同时打开同一个物理文件数据库
    let mut db2 = AstDatabase::open(&db_path).unwrap();
    assert_eq!(db2.current_root(), Some(initial_root.clone()));

    // 实例 1 提交一个候选推进 Root
    let expr1_src = "account.balance + 10";
    let parsed_expr1 = axiom_parser::parse_expr(expr1_src).unwrap();
    let envelope1 = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "balance_impl".into(),
        target_function: "balance".into(),
        contract_hash: NodeHash::default(),
        dependency_hashes: vec![],
    };
    let candidate1 = Candidate {
        envelope_hash: compute_envelope_hash(&envelope1),
        expression_source: expr1_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr1),
        author_agent_id: "worker_1".into(),
        timestamp: 100,
    };
    let receipt1 = VerificationReceipt {
        candidate_hash: compute_candidate_hash(&candidate1),
        proof_level: ProofLevel::Proved,
        solver: "Z3".into(),
        solver_version: "4.13.0".into(),
        obligations_checked: 1,
        proof_obligation_hash: NodeHash::default(),
        verified_at: 100,
    };
    let mut mod1 = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut mod1.functions[0].body, "balance_impl", &parsed_expr1);

    let commit1 = db1
        .try_commit_candidate(&Some(initial_root.clone()), envelope1, candidate1, receipt1, &mod1)
        .expect("db1 commit should succeed");
    let new_root1 = commit1.new_root().clone();

    // 实例 2 持有旧的 initial_root 试图提交 -> 跨进程排他锁与磁盘重读检测到 StaleRoot！
    let expr2_src = "account.balance + 20";
    let parsed_expr2 = axiom_parser::parse_expr(expr2_src).unwrap();
    let envelope2 = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "balance_impl".into(),
        target_function: "balance".into(),
        contract_hash: NodeHash::default(),
        dependency_hashes: vec![],
    };
    let candidate2 = Candidate {
        envelope_hash: compute_envelope_hash(&envelope2),
        expression_source: expr2_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr2),
        author_agent_id: "worker_2".into(),
        timestamp: 101,
    };
    let receipt2 = VerificationReceipt {
        candidate_hash: compute_candidate_hash(&candidate2),
        proof_level: ProofLevel::Proved,
        solver: "Z3".into(),
        solver_version: "4.13.0".into(),
        obligations_checked: 1,
        proof_obligation_hash: NodeHash::default(),
        verified_at: 101,
    };
    let mut mod2 = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut mod2.functions[0].body, "balance_impl", &parsed_expr2);

    let res2 = db2.try_commit_candidate(&Some(initial_root.clone()), envelope2, candidate2, receipt2, &mod2);
    match res2 {
        Err(CasError::StaleRoot { current, expected }) => {
            assert_eq!(current, Some(new_root1));
            assert_eq!(expected, Some(initial_root));
        }
        other => panic!("expected cross-instance StaleRoot error, got: {:?}", other),
    }

    let _ = std::fs::remove_file(&db_path);
}

