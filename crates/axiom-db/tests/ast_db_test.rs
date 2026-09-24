use axiom_ast::{EffectDecl, EffectOp, Expr, FunctionDecl, FunctionParam, Module, Type, TypeDef};
use axiom_db::{
    compute_contract_hash, compute_effect_hash, compute_function_hash, compute_module_hash,
    compute_type_hash, AstDatabase, DraftCasExpectation, NodeHash,
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

fn sample_verifiable_skeleton() -> Module {
    axiom_parser::parse_module(
        r#"
        fn increment(x: Int) -> Int
            requires { x >= 0, x < 1000000 }
            ensures { result > x }
        {
            ?inc_impl
        }
    "#,
    )
    .expect("sample verifiable skeleton must parse")
}

#[test]
fn cas_commits_valid_candidate_and_kernel_mints_receipt() {
    use axiom_db::{
        compute_candidate_hash, compute_envelope_hash, compute_expr_hash,
        compute_proof_obligation_hash, Candidate, ProofLevel, TaskEnvelope,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_verifiable_skeleton();
    let _mod_rec = db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "x + 1";
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

    let mut updated_mod = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut updated_mod.functions[0].body, "inc_impl", &parsed_expr);

    // 1. CAS 提交：由内核独立进行 SMT 验证并亲自铸造收据！
    let commit = db
        .try_commit_candidate(
            &Some(initial_root.clone()),
            envelope.clone(),
            candidate.clone(),
            Some(&updated_mod),
        )
        .expect("CAS with formally verified candidate should succeed");

    assert_eq!(commit.parent_root(), Some(&initial_root));
    assert_ne!(commit.new_root(), commit.parent_root().unwrap());
    assert_eq!(db.current_root().unwrap(), *commit.new_root());
    assert_eq!(
        db.get_candidate(&cand_hash).unwrap(),
        Some(candidate.clone())
    );

    // 2. 验证收据是由内核亲自铸造，包含真实的证明义务绑定
    let minted_receipt = db
        .get_receipt(commit.receipt_hash())
        .unwrap()
        .expect("receipt must exist");
    assert_eq!(minted_receipt.candidate_hash, cand_hash);
    assert_eq!(minted_receipt.proof_level, ProofLevel::Proved);
    assert_eq!(minted_receipt.obligations_checked, 3);
    let expected_obligation_hash = compute_proof_obligation_hash(&updated_mod.functions[0]);
    assert_eq!(
        minted_receipt.proof_obligation_hash,
        expected_obligation_hash
    );

    // 3. 密码学自校验断言：CommitRecord 哈希自验证
    assert!(commit.verify_self_hash());
    assert_eq!(axiom_db::compute_commit_hash(&commit), commit.commit_hash);
}

#[test]
fn cas_detects_and_rejects_forged_module() {
    use axiom_db::{compute_envelope_hash, compute_expr_hash, Candidate, CasError, TaskEnvelope};

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_verifiable_skeleton();
    db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "x + 1";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();

    let candidate = Candidate {
        envelope_hash: env_hash,
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "malicious_agent".into(),
        timestamp: 1700000000,
    };

    // 恶意调用者夹带私货：在 module 中篡改了函数名称或逻辑
    let mut forged_mod = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut forged_mod.functions[0].body, "inc_impl", &parsed_expr);
    forged_mod.functions[0].name = "unauthorized_name_change".into();

    let res = db.try_commit_candidate(&Some(initial_root), envelope, candidate, Some(&forged_mod));

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
        TaskEnvelope,
    };

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_verifiable_skeleton();
    let mod_rec = db.insert_module(&initial_mod).unwrap();
    let stale_root = mod_rec.module_hash;

    // 另一 Worker 先行通过 CAS 提交推进了 root
    let adv_envelope = TaskEnvelope {
        base_root_hash: Some(stale_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let adv_cand = Candidate {
        envelope_hash: compute_envelope_hash(&adv_envelope),
        expression_source: "x + 1".into(),
        expression_hash: compute_expr_hash(&axiom_parser::parse_expr("x + 1").unwrap()),
        author_agent_id: "worker_other".into(),
        timestamp: 100,
    };
    db.try_commit_candidate(&Some(stale_root.clone()), adv_envelope, adv_cand, None)
        .unwrap();
    let current_root = db.current_root().unwrap();
    assert_ne!(current_root, stale_root);

    let envelope = TaskEnvelope {
        base_root_hash: Some(stale_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "x + 2";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();

    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "late_worker".into(),
        timestamp: 1700000050,
    };
    let cand_hash = compute_candidate_hash(&candidate);

    let mut updated_mod = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut updated_mod.functions[0].body, "inc_impl", &parsed_expr);

    // 试图在过期的 stale_root 上提交 -> 必须返回 StaleRoot 错误
    let res = db.try_commit_candidate(
        &Some(stale_root.clone()),
        envelope,
        candidate.clone(),
        Some(&updated_mod),
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
fn kernel_verification_rejects_counterexample_candidate() {
    use axiom_db::{compute_envelope_hash, compute_expr_hash, Candidate, CasError, TaskEnvelope};

    let mut db = AstDatabase::in_memory();
    let initial_mod = sample_verifiable_skeleton();
    db.insert_module(&initial_mod).unwrap();
    let root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    // 错误候选解：x - 1 违反 ensures { result > x }
    let expr_src = "x - 1";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();

    let candidate = Candidate {
        envelope_hash: env_hash,
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "buggy_worker".into(),
        timestamp: 1700000100,
    };

    let res = db.try_commit_candidate(&Some(root), envelope, candidate, None);

    match res {
        Err(CasError::VerificationRejected(msg)) => {
            assert!(msg.contains("counterexample") || msg.contains("increment"));
        }
        other => panic!("expected VerificationRejected error, got: {:?}", other),
    }
}

#[test]
fn cross_instance_cas_file_locking_and_stale_root_detection() {
    use axiom_db::{compute_envelope_hash, compute_expr_hash, Candidate, CasError, TaskEnvelope};

    let db_path = temporary_database_path("cross_instance_test");

    // 实例 1 初始化物理文件数据库
    let mut db1 = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let mod_rec = db1.insert_module(&initial_mod).unwrap();
    let initial_root = mod_rec.module_hash;

    // 实例 2 同时打开同一个物理文件数据库
    let mut db2 = AstDatabase::open(&db_path).unwrap();
    assert_eq!(db2.current_root(), Some(initial_root.clone()));

    // 实例 1 提交一个候选推进 Root
    let expr1_src = "x + 1";
    let parsed_expr1 = axiom_parser::parse_expr(expr1_src).unwrap();
    let envelope1 = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let candidate1 = Candidate {
        envelope_hash: compute_envelope_hash(&envelope1),
        expression_source: expr1_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr1),
        author_agent_id: "worker_1".into(),
        timestamp: 100,
    };
    let mut mod1 = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut mod1.functions[0].body, "inc_impl", &parsed_expr1);

    let commit1 = db1
        .try_commit_candidate(
            &Some(initial_root.clone()),
            envelope1,
            candidate1,
            Some(&mod1),
        )
        .expect("db1 commit should succeed");
    let new_root1 = commit1.new_root().clone();

    // 实例 2 持有旧的 initial_root 试图提交 -> 跨进程排他锁与磁盘重读检测到 StaleRoot！
    let expr2_src = "x + 2";
    let parsed_expr2 = axiom_parser::parse_expr(expr2_src).unwrap();
    let envelope2 = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: axiom_db::compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let candidate2 = Candidate {
        envelope_hash: compute_envelope_hash(&envelope2),
        expression_source: expr2_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr2),
        author_agent_id: "worker_2".into(),
        timestamp: 101,
    };
    let mut mod2 = initial_mod.clone();
    axiom_db::replace_hole_in_expr(&mut mod2.functions[0].body, "inc_impl", &parsed_expr2);

    let res2 = db2.try_commit_candidate(
        &Some(initial_root.clone()),
        envelope2,
        candidate2,
        Some(&mod2),
    );
    match res2 {
        Err(CasError::StaleRoot { current, expected }) => {
            assert_eq!(current, Some(new_root1));
            assert_eq!(expected, Some(initial_root));
        }
        other => panic!("expected cross-instance StaleRoot error, got: {:?}", other),
    }

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn os_file_lock_guard_provides_mutual_exclusion() {
    use axiom_db::FileLockGuard;

    let db_path = temporary_database_path("lock_exclusion_test");

    // 持有排他锁
    let guard1 = FileLockGuard::acquire(&db_path).expect("first lock acquire should succeed");

    // 第二个锁应该在超时后失败或者被阻塞
    let start = std::time::Instant::now();
    // 在子线程尝试获取锁
    let handle = std::thread::spawn({
        let path = db_path.clone();
        move || FileLockGuard::acquire(&path)
    });

    // 释放 guard1
    std::thread::sleep(std::time::Duration::from_millis(100));
    drop(guard1);

    // guard1 释放后，子线程应该能成功获取到锁
    let guard2_res = handle.join().expect("thread join failed");
    assert!(
        guard2_res.is_ok(),
        "second acquire after drop should succeed"
    );
    assert!(start.elapsed() >= std::time::Duration::from_millis(50));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn locked_transaction_serializes_concurrent_writes() {
    use axiom_db::TaskEnvelope;
    use std::sync::Arc;

    let db_path = temporary_database_path("concurrent_tx_test");
    let mut db = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    db.insert_module(&initial_mod).unwrap();

    let db_path = Arc::new(db_path);
    let mut handles = Vec::new();

    for i in 0..5 {
        let path = Arc::clone(&db_path);
        handles.push(std::thread::spawn(move || {
            let mut local_db = AstDatabase::open(&path).expect("open must succeed");
            let dummy_env = TaskEnvelope {
                base_root_hash: None,
                target_hole_id: format!("hole_{i}"),
                target_function: "dummy".into(),
                contract_hash: NodeHash::default(),
                dependency_hashes: vec![],
            };
            local_db.record_envelope(dummy_env)
        }));
    }

    for h in handles {
        h.join().unwrap().expect("transaction should succeed");
    }

    let final_db = AstDatabase::open(&db_path).unwrap();
    for i in 0..5 {
        let dummy_env = TaskEnvelope {
            base_root_hash: None,
            target_hole_id: format!("hole_{i}"),
            target_function: "dummy".into(),
            contract_hash: NodeHash::default(),
            dependency_hashes: vec![],
        };
        let hash = axiom_db::compute_envelope_hash(&dummy_env);
        assert!(final_db.get_envelope(&hash).unwrap().is_some());
    }

    let _ = std::fs::remove_file(&*db_path);
}

#[test]
fn two_phase_cas_offline_verification_and_short_lock_commit() {
    let db_path = temporary_database_path("two_phase_cas_test");
    let mut db = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let mod_rec = db.insert_module(&initial_mod).unwrap();
    db.initialize_root(mod_rec.module_hash.clone()).unwrap();

    let root = db.current_root();
    let func = &initial_mod.functions[0];
    let contract_hash = compute_contract_hash(func);

    let envelope = axiom_db::TaskEnvelope {
        base_root_hash: root.clone(),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash,
        dependency_hashes: vec![],
    };

    let candidate = axiom_db::Candidate {
        envelope_hash: axiom_db::compute_envelope_hash(&envelope),
        expression_source: "x + 1".into(),
        expression_hash: axiom_db::compute_expr_hash(&axiom_parser::parse_expr("x + 1").unwrap()),
        author_agent_id: "agent_two_phase".into(),
        timestamp: 123456,
    };

    // 1. 无锁离线验证：耗时的 SMT 证明完全在锁外完成
    let certified = db
        .verify_candidate_offline(&root, &envelope, &candidate, None)
        .expect("offline verification should succeed");

    assert_eq!(certified.evidence().obligations_checked(), 3);

    // 2. 微秒级短锁原子提交
    let commit = db
        .try_commit_verified(&root, envelope, candidate, certified)
        .expect("short-lock commit should succeed");

    assert_eq!(commit.parent_root, root);
    assert_ne!(Some(commit.new_root.clone()), root);
    assert_eq!(db.current_root(), Some(commit.new_root.clone()));

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn two_phase_cas_rejects_mismatched_base_root() {
    let db_path = temporary_database_path("two_phase_mismatch_test");
    let mut db = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let mod_rec = db.insert_module(&initial_mod).unwrap();
    db.initialize_root(mod_rec.module_hash.clone()).unwrap();

    let root = db.current_root();
    let func = &initial_mod.functions[0];
    let contract_hash = compute_contract_hash(func);

    let envelope = axiom_db::TaskEnvelope {
        base_root_hash: root.clone(),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash,
        dependency_hashes: vec![],
    };

    let candidate = axiom_db::Candidate {
        envelope_hash: axiom_db::compute_envelope_hash(&envelope),
        expression_source: "x + 1".into(),
        expression_hash: axiom_db::compute_expr_hash(&axiom_parser::parse_expr("x + 1").unwrap()),
        author_agent_id: "agent_mismatch".into(),
        timestamp: 123456,
    };

    let certified = db
        .verify_candidate_offline(&root, &envelope, &candidate, None)
        .expect("offline verification should succeed");

    // 试图将基于 root 的证书提交给一个伪造的/过期的 root
    let fake_root = Some(
        "#mod_0000000000000000000000000000000000000000000000000000000000000001"
            .parse::<NodeHash>()
            .unwrap(),
    );
    let err = db.try_commit_verified(&fake_root, envelope, candidate, certified);
    assert!(
        err.is_err(),
        "commit with mismatched base_root_hash must be rejected!"
    );

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn receipt_binds_verification_profile_hash_and_invalidates_on_profile_mismatch() {
    use axiom_db::{
        compute_receipt_hash, current_verification_profile_hash, ProofLevel, VerificationReceipt,
    };

    let current_hash = current_verification_profile_hash();
    assert!(current_hash.as_str().starts_with("#vprof_"));

    let rcpt1 = VerificationReceipt {
        candidate_hash: "#cand_1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap(),
        proof_level: ProofLevel::Proved,
        solver: "z3".into(),
        solver_version: "4.12.0".into(),
        solver_sha256: None,
        obligations_checked: 5,
        proof_obligation_hash:
            "#obl_1111111111111111111111111111111111111111111111111111111111111111"
                .parse()
                .unwrap(),
        verification_profile_hash: current_hash.clone(),
        attested_commit_hash: None,
        verified_at: 1000,
    };

    let mut rcpt_old = rcpt1.clone();
    rcpt_old.verification_profile_hash =
        "#vprof_0000000000000000000000000000000000000000000000000000000000000000"
            .parse()
            .unwrap();

    assert_ne!(
        compute_receipt_hash(&rcpt1),
        compute_receipt_hash(&rcpt_old)
    );
    assert_ne!(
        rcpt_old.verification_profile_hash,
        current_verification_profile_hash()
    );
}

#[test]
fn persisted_database_persists_and_restores_draft_records() {
    use axiom_db::DraftRecord;

    let db_path = temporary_database_path("draft-persistence");
    if db_path.exists() {
        let _ = std::fs::remove_file(&db_path);
    }

    {
        let mut db = AstDatabase::open(&db_path).expect("db open should succeed");
        assert!(db.drafts().is_empty());

        let draft = DraftRecord {
            hole_id: "hole_add".into(),
            agent_id: "worker_1".into(),
            code: "a + b".into(),
            timestamp: 1700000000,
            base_root: None,
            revision: 1,
        };
        db.insert_draft(draft.clone())
            .expect("insert draft should succeed");

        assert_eq!(db.get_draft("hole_add"), Some(&draft));
        assert_eq!(db.drafts().len(), 1);
    }

    // Re-open from disk in a fresh instance
    {
        let mut db = AstDatabase::open(&db_path).expect("reopen should succeed");
        assert_eq!(db.drafts().len(), 1);
        let restored = db
            .get_draft("hole_add")
            .expect("draft should exist after reopen");
        assert_eq!(restored.agent_id, "worker_1");
        assert_eq!(restored.code, "a + b");

        let removed = db
            .remove_draft("hole_add")
            .expect("remove draft should succeed");
        assert!(removed.is_some());
        assert!(db.drafts().is_empty());

        // 测试 insert_draft_cas 乐观并发控制
        let draft_v1 = DraftRecord {
            hole_id: "hole_cas".into(),
            agent_id: "agent_a".into(),
            code: "10".into(),
            timestamp: 100,
            base_root: None,
            revision: 0,
        };
        let committed_v1 = db
            .insert_draft_cas(draft_v1, &DraftCasExpectation::initial(None))
            .expect("initial cas draft should succeed");
        assert_eq!(committed_v1.revision, 1);

        // 使用过期的 expected_revision 尝试提交应被 CAS 拦截
        let draft_stale = DraftRecord {
            hole_id: "hole_cas".into(),
            agent_id: "agent_b".into(),
            code: "20".into(),
            timestamp: 101,
            base_root: None,
            revision: 0,
        };
        let cas_err = db.insert_draft_cas(draft_stale, &DraftCasExpectation::initial(None));
        assert!(matches!(
            cas_err,
            Err(axiom_db::CasError::StaleDraftRevision { .. })
        ));

        // 预期错误的 base_root 应被 StaleRootMismatch 拦截
        let bogus_root: NodeHash =
            "#mod_0000000000000000000000000000000000000000000000000000000000000001"
                .parse()
                .unwrap();
        let draft_bogus_root = DraftRecord {
            hole_id: "hole_cas".into(),
            agent_id: "agent_c".into(),
            code: "30".into(),
            timestamp: 101,
            base_root: None,
            revision: 0,
        };
        let root_err = db.insert_draft_cas(
            draft_bogus_root,
            &DraftCasExpectation::update(1, Some(bogus_root)),
        );
        assert!(matches!(
            root_err,
            Err(axiom_db::CasError::StaleRootMismatch { .. })
        ));

        // 使用正确的 revision 提交应成功递增
        let draft_v2 = DraftRecord {
            hole_id: "hole_cas".into(),
            agent_id: "agent_b".into(),
            code: "20".into(),
            timestamp: 102,
            base_root: None,
            revision: 0,
        };
        let committed_v2 = db
            .insert_draft_cas(draft_v2, &DraftCasExpectation::update(1, None))
            .expect("subsequent cas draft should succeed");
        assert_eq!(committed_v2.revision, 2);
    }

    // Verify disk state after removal
    {
        let db = AstDatabase::open(&db_path).expect("third open should succeed");
        assert_eq!(db.drafts().len(), 1);
        assert_eq!(db.get_draft("hole_cas").unwrap().revision, 2);
    }

    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn legacy_receipt_without_profile_deserializes_with_fallback_and_invalidates_current_profile() {
    use axiom_db::{
        current_verification_profile_hash, legacy_verification_profile_hash, ProofLevel,
        VerificationReceipt,
    };

    // 模拟无 verification_profile_hash 字段的历史旧版收据 JSON
    let legacy_json = r##"{
        "candidate_hash": "#cand_abc123",
        "proof_level": "Proved",
        "solver": "z3",
        "solver_version": "4.13.0",
        "obligations_checked": 2,
        "proof_obligation_hash": "#obl_xyz456",
        "verified_at": 1700000000
    }"##;

    let receipt: VerificationReceipt = serde_json::from_str(legacy_json)
        .expect("legacy receipt JSON should deserialize with default profile");

    assert_eq!(receipt.candidate_hash.as_str(), "#cand_abc123");
    assert_eq!(receipt.proof_level, ProofLevel::Proved);
    assert_eq!(
        receipt.verification_profile_hash,
        legacy_verification_profile_hash()
    );
    assert_ne!(
        receipt.verification_profile_hash,
        current_verification_profile_hash(),
        "Legacy receipt must never match current profile hash and will trigger downgrade!"
    );
}

#[test]
fn test_mainline_commits_and_reattestation() {
    use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Module, Type};
    use axiom_db::{
        compute_contract_hash, compute_envelope_hash, current_verification_profile_hash,
        AstDatabase, Candidate, CasError, ProofLevel, TaskEnvelope,
    };

    let mut db = AstDatabase::in_memory();
    assert!(db
        .mainline_commits()
        .expect("empty db should succeed")
        .is_empty());
    // 构造初始带有 Hole 的函数
    let func_with_hole = FunctionDecl {
        name: "add".into(),
        params: vec![
            FunctionParam {
                name: "x".into(),
                ty: Type::Int,
            },
            FunctionParam {
                name: "y".into(),
                ty: Type::Int,
            },
        ],
        return_type: Type::Int,
        effects: vec![],
        requires: vec![
            Expr::BinaryOp {
                op: BinaryOperator::GreaterThanOrEqual,
                left: Box::new(Expr::Ident("x".into())),
                right: Box::new(Expr::Literal(axiom_ast::Literal::Int(0))),
            },
            Expr::BinaryOp {
                op: BinaryOperator::LessThanOrEqual,
                left: Box::new(Expr::Ident("x".into())),
                right: Box::new(Expr::Literal(axiom_ast::Literal::Int(1000))),
            },
            Expr::BinaryOp {
                op: BinaryOperator::GreaterThanOrEqual,
                left: Box::new(Expr::Ident("y".into())),
                right: Box::new(Expr::Literal(axiom_ast::Literal::Int(0))),
            },
            Expr::BinaryOp {
                op: BinaryOperator::LessThanOrEqual,
                left: Box::new(Expr::Ident("y".into())),
                right: Box::new(Expr::Literal(axiom_ast::Literal::Int(1000))),
            },
        ],
        ensures: vec![Expr::BinaryOp {
            op: BinaryOperator::Equal,
            left: Box::new(Expr::Ident("result".into())),
            right: Box::new(Expr::BinaryOp {
                op: BinaryOperator::Add,
                left: Box::new(Expr::Ident("x".into())),
                right: Box::new(Expr::Ident("y".into())),
            }),
        }],
        body: Expr::Hole("hole_1".into()),
    };

    let module = Module {
        types: vec![],
        functions: vec![func_with_hole.clone()],
        effects: vec![],
    };

    let mod_rec = db
        .insert_module(&module)
        .expect("insert module should succeed");
    let root0 = mod_rec.module_hash;
    db.initialize_root(root0.clone())
        .expect("init root should succeed");

    let env = TaskEnvelope {
        base_root_hash: Some(root0.clone()),
        target_hole_id: "hole_1".into(),
        target_function: "add".into(),
        contract_hash: compute_contract_hash(&func_with_hole),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&env);

    let parsed_body = Expr::BinaryOp {
        op: BinaryOperator::Add,
        left: Box::new(Expr::Ident("x".into())),
        right: Box::new(Expr::Ident("y".into())),
    };
    let cand = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "x + y".into(),
        expression_hash: axiom_db::compute_expr_hash(&parsed_body),
        author_agent_id: "agent_1".into(),
        timestamp: 1000,
    };

    // 1. 通过公共标准 CAS 接口将合法 candidate 提交落地到主线
    let commit1 = db
        .try_commit_candidate(&Some(root0), env.clone(), cand.clone(), None)
        .expect("first candidate commit should succeed");
    let head1 = db.head_commit().cloned();
    assert_eq!(head1, Some(commit1.commit_hash.clone()));

    // 2. [Spec 1 核心] 测试主线落地见证核验：未在主线落地的虚假 hole 绝不可被签发重验认证
    let forge_res = db.reverify_and_reattest_hole("unlanded_hole");
    assert!(
        matches!(forge_res, Err(CasError::InvalidCandidate(_))),
        "未在主线落地的 candidate 必须被拦截"
    );

    // 3. 通过公共安全接口 reverify_and_reattest_hole 执行重新验证并原子提交
    let commit2 = db
        .reverify_and_reattest_hole("hole_1")
        .expect("reverify_and_reattest_hole should succeed");
    let head2 = db.head_commit().cloned();
    assert_eq!(head2, Some(commit2.commit_hash.clone()));
    assert_eq!(commit2.parent_commit(), Some(&commit1.commit_hash));
    // 强断言：no-op commit 的 attested_commit_hash 必须严格指向原始 landing commit！
    assert_eq!(commit2.attested_commit_hash(), Some(&commit1.commit_hash));

    // 验证收据是由 DB 权威铸造，与当前 Profile 绑定，且同样记录了 attested_commit_hash
    let rcpt = db.get_receipt(commit2.receipt_hash()).unwrap().unwrap();
    assert_eq!(rcpt.proof_level, ProofLevel::Proved);
    assert_eq!(
        rcpt.verification_profile_hash,
        current_verification_profile_hash()
    );
    assert_eq!(rcpt.attested_commit_hash, Some(commit1.commit_hash.clone()));

    // 5. 验证 mainline_commits 严格正序且各提交自身哈希自洽
    let mainline = db.mainline_commits().expect("mainline should succeed");
    assert_eq!(mainline.len(), 2);
    assert_eq!(mainline[0].commit_hash, commit1.commit_hash);
    assert_eq!(mainline[1].commit_hash, commit2.commit_hash);
    assert!(mainline[0].verify_self_hash());
    assert!(mainline[1].verify_self_hash());
}

#[test]
fn test_forged_noop_receipt_without_landing_commit_is_rejected() {
    use axiom_db::{
        compute_candidate_hash, compute_commit_payload_hash, compute_contract_hash,
        compute_envelope_hash, compute_expr_hash, compute_proof_obligation_hash,
        compute_receipt_hash, current_verification_profile_hash, AstDatabase, Candidate,
        CommitPayload, ProofLevel, TaskEnvelope, VerificationReceipt,
    };

    let temp_dir = std::env::temp_dir().join(format!("test_forged_noop_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_forged_noop.json");

    let mut db_file = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let _ = db_file.insert_module(&initial_mod).unwrap();
    let initial_root = db_file.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let parsed_cand = axiom_parser::parse_expr("x + 1").unwrap();
    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "x + 1".into(),
        expression_hash: compute_expr_hash(&parsed_cand),
        author_agent_id: "honest_worker".into(),
        timestamp: 100,
    };

    let commit1 = db_file
        .try_commit_candidate(
            &Some(initial_root),
            envelope.clone(),
            candidate.clone(),
            None,
        )
        .expect("landing commit must succeed");
    let current_root = db_file.current_root().unwrap();
    drop(db_file);

    // 攻击者试图伪造一个 no-op receipt-only commit:
    // parent_root == current_root, new_root == current_root
    // 但这个 no-op commit 故意不提供合法的 attested_commit_hash (例如缺失)
    let fake_cand = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "x + 999".into(),
        expression_hash: compute_expr_hash(&axiom_parser::parse_expr("x + 999").unwrap()),
        author_agent_id: "forger".into(),
        timestamp: 200,
    };
    let fake_cand_hash = compute_candidate_hash(&fake_cand);

    let fake_rcpt = VerificationReceipt {
        candidate_hash: fake_cand_hash.clone(),
        proof_level: ProofLevel::Proved,
        solver: "z3".into(),
        solver_version: "4.12.0".into(),
        solver_sha256: None,
        obligations_checked: 1,
        proof_obligation_hash: compute_proof_obligation_hash(&initial_mod.functions[0]),
        verification_profile_hash: current_verification_profile_hash(),
        attested_commit_hash: None, // 恶意缺失 attested_commit_hash
        verified_at: 200,
    };
    let fake_rcpt_hash = compute_receipt_hash(&fake_rcpt);

    let fake_commit_payload = CommitPayload {
        parent_commit: Some(commit1.commit_hash.clone()),
        parent_root: Some(current_root.clone()),
        new_root: current_root.clone(),
        envelope_hash: env_hash.clone(),
        candidate_hash: fake_cand_hash.clone(),
        receipt_hash: fake_rcpt_hash.clone(),
        attested_commit_hash: None,
        timestamp: 200,
    };
    let fake_commit_hash = compute_commit_payload_hash(&fake_commit_payload);

    // 将伪造提交写入文件，head 指向 fake_commit_hash
    let content = std::fs::read_to_string(&db_path).unwrap();
    let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();

    val.get_mut("candidates")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            fake_cand_hash.as_str().into(),
            serde_json::to_value(&fake_cand).unwrap(),
        );
    val.get_mut("receipts")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            fake_rcpt_hash.as_str().into(),
            serde_json::to_value(&fake_rcpt).unwrap(),
        );
    val.get_mut("commits")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            fake_commit_hash.as_str().into(),
            serde_json::json!({
                "commit_hash": fake_commit_hash.as_str(),
                "parent_commit": commit1.commit_hash.as_str(),
                "parent_root": current_root.as_str(),
                "new_root": current_root.as_str(),
                "envelope_hash": env_hash.as_str(),
                "candidate_hash": fake_cand_hash.as_str(),
                "receipt_hash": fake_rcpt_hash.as_str(),
                "timestamp": 200
            }),
        );
    val["head_commit"] = serde_json::Value::String(fake_commit_hash.as_str().into());
    std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

    // 重新打开数据库：必须被 Fail-Closed 拦截！
    let open_res = AstDatabase::open(&db_path);
    match open_res {
        Err(axiom_db::DbError::CorruptedCommitChain(msg)) => {
            assert!(
                msg.contains("missing required attested_commit_hash"),
                "error message should indicate missing attested_commit_hash: {msg}"
            );
        }
        other => panic!(
            "expected open to reject un-attested no-op commit, got {:?}",
            other
        ),
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_reattestation_deterministic_replay_rejects_mismatch() {
    use axiom_db::{
        compute_candidate_hash, compute_commit_payload_hash, compute_contract_hash,
        compute_envelope_hash, compute_expr_hash, compute_receipt_hash,
        current_verification_profile_hash, AstDatabase, Candidate, CommitPayload, ProofLevel,
        TaskEnvelope, VerificationReceipt,
    };

    let temp_dir = std::env::temp_dir().join(format!("test_replay_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_replay.json");

    let mut db_file = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let _ = db_file.insert_module(&initial_mod).unwrap();
    let initial_root = db_file.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let expr_src = "x + 1";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();
    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "agent_42".into(),
        timestamp: 1000,
    };

    let commit = db_file
        .try_commit_candidate(
            &Some(initial_root),
            envelope.clone(),
            candidate.clone(),
            None,
        )
        .expect("initial candidate commit should succeed");

    // [Spec 1] 构造一个自洽但表达式不匹配的 candidate_mismatch ("x + 2") 及其合法收据
    let expr_mismatch = "x + 2";
    let parsed_mismatch = axiom_parser::parse_expr(expr_mismatch).unwrap();
    let cand_mismatch = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: expr_mismatch.into(),
        expression_hash: compute_expr_hash(&parsed_mismatch),
        author_agent_id: "agent_adversary".into(),
        timestamp: 1000,
    };
    let cand_mismatch_hash = compute_candidate_hash(&cand_mismatch);

    let rcpt_mismatch = VerificationReceipt {
        candidate_hash: cand_mismatch_hash.clone(),
        proof_level: ProofLevel::Proved,
        solver: "z3".into(),
        solver_version: "4.12.0".into(),
        solver_sha256: None,
        obligations_checked: 3,
        proof_obligation_hash: commit.receipt_hash().clone(),
        verification_profile_hash: current_verification_profile_hash(),
        attested_commit_hash: None,
        verified_at: 1000,
    };
    let rcpt_mismatch_hash = compute_receipt_hash(&rcpt_mismatch);

    // 将 candidate_mismatch 与其 receipt 写入落盘文件，并将 commit 中的 candidate_hash 和 receipt_hash 指向它们
    // （此时工件自哈希与交叉绑定均完全自洽，但 landing commit 的 new_root 实际上是由 "x + 1" 生成的）
    let content = std::fs::read_to_string(&db_path).unwrap();
    let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();

    val.get_mut("candidates")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            cand_mismatch_hash.as_str().into(),
            serde_json::to_value(&cand_mismatch).unwrap(),
        );
    val.get_mut("receipts")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            rcpt_mismatch_hash.as_str().into(),
            serde_json::to_value(&rcpt_mismatch).unwrap(),
        );

    let commit_obj = val
        .get_mut("commits")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .get_mut(commit.commit_hash.as_str())
        .unwrap()
        .as_object_mut()
        .unwrap();
    commit_obj.insert(
        "candidate_hash".into(),
        serde_json::to_value(&cand_mismatch_hash).unwrap(),
    );
    commit_obj.insert(
        "receipt_hash".into(),
        serde_json::to_value(&rcpt_mismatch_hash).unwrap(),
    );

    // 重新计算并更新该 commit 的 commit_hash（以满足主链 self-hash 校验）
    let payload: CommitPayload =
        serde_json::from_value(serde_json::Value::Object(commit_obj.clone())).unwrap();
    let new_commit_hash = compute_commit_payload_hash(&payload);
    commit_obj.insert(
        "commit_hash".into(),
        serde_json::to_value(&new_commit_hash).unwrap(),
    );

    let commits_map = val.get_mut("commits").unwrap().as_object_mut().unwrap();
    let commit_val = commits_map.remove(commit.commit_hash.as_str()).unwrap();
    commits_map.insert(new_commit_hash.as_str().into(), commit_val);
    val.as_object_mut().unwrap().insert(
        "head_commit".into(),
        serde_json::to_value(&new_commit_hash).unwrap(),
    );

    std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

    // [Spec 3] 验证 AstDatabase::open 在加载时深度校验主链语义：发现 candidate 与 new_root 重放不匹配立即拒绝！
    let open_res = AstDatabase::open(&db_path);
    match open_res {
        Err(axiom_db::DbError::CorruptedCommitChain(msg)) => {
            assert!(
                msg.contains("candidate replay derivation failed")
                    || msg.contains("new_root mismatch"),
                "error message should indicate replay hash mismatch on open: {msg}"
            );
        }
        other => panic!(
            "expected open to reject corrupted replay mismatch, got {:?}",
            other
        ),
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_mainline_commits_validates_artifacts_and_cross_bindings() {
    use axiom_db::{
        compute_contract_hash, compute_envelope_hash, compute_expr_hash, AstDatabase, Candidate,
        DbError, TaskEnvelope,
    };

    let temp_dir = std::env::temp_dir().join(format!("test_bindings_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_bindings.json");

    let mut db = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let _ = db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };

    let expr_src = "x + 1";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();
    let candidate = Candidate {
        envelope_hash: compute_envelope_hash(&envelope),
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "agent_42".into(),
        timestamp: 1000,
    };

    let _commit = db
        .try_commit_candidate(&Some(initial_root), envelope, candidate, None)
        .expect("candidate commit should succeed");

    assert!(db.mainline_commits().is_ok());

    // [Spec 3] 篡改场景 A：删除 envelope 工件，断言 mainline_commits 深度校验拦截
    let content = std::fs::read_to_string(&db_path).unwrap();
    let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();
    val.as_object_mut().unwrap().remove("envelopes");
    std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

    let reopen_res1 = AstDatabase::open(&db_path);
    match reopen_res1 {
        Err(DbError::CorruptedCommitChain(msg)) => {
            assert!(msg.contains("references missing task envelope"));
        }
        other => panic!(
            "expected missing envelope CorruptedCommitChain error on open, got {:?}",
            other
        ),
    }

    // [Spec 3] 篡改场景 B：交叉绑定破坏，将 receipt 的 candidate 篡改成伪造值
    let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();
    let receipts = val.get_mut("receipts").unwrap().as_object_mut().unwrap();
    for (_rcpt_hash, rcpt_val) in receipts.iter_mut() {
        rcpt_val.as_object_mut().unwrap().insert(
            "candidate_hash".into(),
            serde_json::json!(
                "#cand_0000000000000000000000000000000000000000000000000000000000000000"
            ),
        );
    }
    std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

    let reopen_res2 = AstDatabase::open(&db_path);
    match reopen_res2 {
        Err(DbError::CorruptedCommitChain(msg)) => {
            assert!(
                msg.contains("receipt hash mismatch") || msg.contains("does not match"),
                "expected hash mismatch or cross-binding mismatch on open, got {msg}"
            );
        }
        other => panic!(
            "expected receipt cross-binding CorruptedCommitChain error on open, got {:?}",
            other
        ),
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_nested_hole_repeated_reattestation_success() {
    use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Module, Type};
    use axiom_db::{
        compute_contract_hash, compute_envelope_hash, compute_expr_hash, AstDatabase, Candidate,
        TaskEnvelope,
    };

    let temp_dir = std::env::temp_dir().join(format!("test_repeated_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_repeated.json");

    let mut db = AstDatabase::open(&db_path).unwrap();

    // 构造嵌套打洞函数：1 + ?inner_hole
    let initial_mod = Module {
        types: vec![],
        functions: vec![FunctionDecl {
            name: "compute".into(),
            params: vec![FunctionParam {
                name: "x".into(),
                ty: Type::Int,
            }],
            return_type: Type::Int,
            effects: vec![],
            requires: vec![
                Expr::BinaryOp {
                    left: Box::new(Expr::Ident("x".into())),
                    op: BinaryOperator::GreaterThanOrEqual,
                    right: Box::new(Expr::Literal(axiom_ast::Literal::Int(0))),
                },
                Expr::BinaryOp {
                    left: Box::new(Expr::Ident("x".into())),
                    op: BinaryOperator::LessThan,
                    right: Box::new(Expr::Literal(axiom_ast::Literal::Int(1000))),
                },
            ],
            ensures: vec![Expr::BinaryOp {
                left: Box::new(Expr::Ident("result".into())),
                op: BinaryOperator::GreaterThan,
                right: Box::new(Expr::Literal(axiom_ast::Literal::Int(0))),
            }],
            body: Expr::BinaryOp {
                left: Box::new(Expr::Literal(axiom_ast::Literal::Int(1))),
                op: BinaryOperator::Add,
                right: Box::new(Expr::Hole("inner_hole".into())),
            },
        }],
        effects: vec![],
    };

    let _ = db.insert_module(&initial_mod).unwrap();
    let root0 = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(root0.clone()),
        target_hole_id: "inner_hole".into(),
        target_function: "compute".into(),
        contract_hash: compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };

    let expr_src = "x + 2";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();
    let candidate = Candidate {
        envelope_hash: compute_envelope_hash(&envelope),
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "agent_42".into(),
        timestamp: 100,
    };

    // 1. 提交 candidate，落地为 Commit 1 (root-changing commit)
    let commit1 = db
        .try_commit_candidate(&Some(root0), envelope, candidate, None)
        .expect("initial candidate commit should succeed");
    assert_ne!(commit1.parent_root(), Some(commit1.new_root()));

    // 2. 第一次重签升级：生成 Commit 2 (receipt-only commit, parent_root == new_root)
    let commit2 = db
        .reverify_and_reattest_hole("inner_hole")
        .expect("first reverify_and_reattest_hole should succeed");
    assert_eq!(commit2.parent_root(), Some(commit2.new_root()));
    assert_eq!(commit2.parent_commit(), Some(&commit1.commit_hash));

    // 3. [Spec 1 核心] 第二次重签升级：必须跳过 Commit 2，准确定位 Commit 1 并成功在 parent 模块重放！
    // 若错误在 Commit 2 的 parent 模块重放，将因已经没有 inner_hole 而报错 "target hole not found"
    let commit3 = db
        .reverify_and_reattest_hole("inner_hole")
        .expect("second reverify_and_reattest_hole must succeed and skip receipt-only commits");
    assert_eq!(commit3.parent_root(), Some(commit3.new_root()));
    assert_eq!(commit3.parent_commit(), Some(&commit2.commit_hash));

    // 4. 验证主线包含 3 次有序提交，且全链完整性校验通过
    let mainline = db
        .mainline_commits()
        .expect("mainline validation must succeed");
    assert_eq!(mainline.len(), 3);
    assert_eq!(mainline[0].commit_hash, commit1.commit_hash);
    assert_eq!(mainline[1].commit_hash, commit2.commit_hash);
    assert_eq!(mainline[2].commit_hash, commit3.commit_hash);

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_noop_receipt_with_invalid_attested_commit_is_rejected() {
    use axiom_db::{
        compute_commit_payload_hash, compute_contract_hash, compute_envelope_hash,
        compute_expr_hash, compute_proof_obligation_hash, compute_receipt_hash,
        current_verification_profile_hash, AstDatabase, Candidate, DbError, NodeHash, ProofLevel,
        TaskEnvelope, VerificationReceipt,
    };

    let temp_dir =
        std::env::temp_dir().join(format!("test_invalid_attested_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_invalid_attested.json");

    let mut db_file = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let _ = db_file.insert_module(&initial_mod).unwrap();
    let initial_root = db_file.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&envelope);

    let parsed_cand = axiom_parser::parse_expr("x + 1").unwrap();
    let candidate = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "x + 1".into(),
        expression_hash: compute_expr_hash(&parsed_cand),
        author_agent_id: "honest_worker".into(),
        timestamp: 100,
    };

    let commit1 = db_file
        .try_commit_candidate(
            &Some(initial_root),
            envelope.clone(),
            candidate.clone(),
            None,
        )
        .expect("landing commit must succeed");
    let current_root = db_file.current_root().unwrap();
    drop(db_file);

    // 构造一个 no-op commit，其 attested_commit_hash 指向不存在的祖先提交
    let invalid_attested_hash: NodeHash =
        "#commit_0000000000000000000000000000000000000000000000000000000000000000"
            .parse()
            .unwrap();

    let fake_rcpt = VerificationReceipt {
        candidate_hash: commit1.candidate_hash().clone(),
        proof_level: ProofLevel::Proved,
        solver: "z3".into(),
        solver_version: "4.12.0".into(),
        solver_sha256: None,
        obligations_checked: 1,
        proof_obligation_hash: compute_proof_obligation_hash(&initial_mod.functions[0]),
        verification_profile_hash: current_verification_profile_hash(),
        attested_commit_hash: Some(invalid_attested_hash.clone()),
        verified_at: 200,
    };
    let fake_rcpt_hash = compute_receipt_hash(&fake_rcpt);

    let fake_commit_payload = axiom_db::CommitPayload {
        parent_commit: Some(commit1.commit_hash.clone()),
        parent_root: Some(current_root.clone()),
        new_root: current_root.clone(),
        envelope_hash: env_hash,
        candidate_hash: commit1.candidate_hash().clone(),
        receipt_hash: fake_rcpt_hash.clone(),
        attested_commit_hash: Some(invalid_attested_hash.clone()),
        timestamp: 200,
    };
    let fake_commit_hash = compute_commit_payload_hash(&fake_commit_payload);

    let content = std::fs::read_to_string(&db_path).unwrap();
    let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();

    val.get_mut("receipts")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            fake_rcpt_hash.as_str().into(),
            serde_json::to_value(&fake_rcpt).unwrap(),
        );
    val.get_mut("commits")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert(
            fake_commit_hash.as_str().into(),
            serde_json::json!({
                "commit_hash": fake_commit_hash.as_str(),
                "parent_commit": commit1.commit_hash.as_str(),
                "parent_root": current_root.as_str(),
                "new_root": current_root.as_str(),
                "envelope_hash": commit1.envelope_hash().as_str(),
                "candidate_hash": commit1.candidate_hash().as_str(),
                "receipt_hash": fake_rcpt_hash.as_str(),
                "attested_commit_hash": invalid_attested_hash.as_str(),
                "timestamp": 200
            }),
        );
    val["head_commit"] = serde_json::Value::String(fake_commit_hash.as_str().into());
    std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

    // 重新打开数据库：必须被 Fail-Closed 拦截！
    let open_res = AstDatabase::open(&db_path);
    match open_res {
        Err(DbError::CorruptedCommitChain(msg)) => {
            assert!(
                msg.contains("is not an ancestor in the active mainline history")
                    || msg.contains("attested landing commit"),
                "error message should indicate invalid attested commit: {msg}"
            );
        }
        other => panic!(
            "expected open to reject invalid attested commit, got {:?}",
            other
        ),
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_mainline_semantic_invariants_reject_tampered_expressions() {
    use axiom_db::{
        compute_contract_hash, compute_envelope_hash, compute_expr_hash, AstDatabase, Candidate,
        DbError, TaskEnvelope,
    };

    let temp_dir = std::env::temp_dir().join(format!("test_semantics_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_semantics.json");

    let mut db = AstDatabase::open(&db_path).unwrap();
    let initial_mod = sample_verifiable_skeleton();
    let _ = db.insert_module(&initial_mod).unwrap();
    let initial_root = db.current_root().unwrap();

    let envelope = TaskEnvelope {
        base_root_hash: Some(initial_root.clone()),
        target_hole_id: "inc_impl".into(),
        target_function: "increment".into(),
        contract_hash: compute_contract_hash(&initial_mod.functions[0]),
        dependency_hashes: vec![],
    };

    let expr_src = "x + 1";
    let parsed_expr = axiom_parser::parse_expr(expr_src).unwrap();
    let candidate = Candidate {
        envelope_hash: compute_envelope_hash(&envelope),
        expression_source: expr_src.into(),
        expression_hash: compute_expr_hash(&parsed_expr),
        author_agent_id: "agent_42".into(),
        timestamp: 1000,
    };

    let _ = db
        .try_commit_candidate(&Some(initial_root), envelope, candidate, None)
        .expect("candidate commit should succeed");

    // [Spec 3] 篡改候选表达源码：将源码改为 "x + 99"，使 expression_source 与 expression_hash 不匹配
    let content = std::fs::read_to_string(&db_path).unwrap();
    let mut val: serde_json::Value = serde_json::from_str(&content).unwrap();
    let candidates = val.get_mut("candidates").unwrap().as_object_mut().unwrap();
    for (_cand_hash, cand_val) in candidates.iter_mut() {
        cand_val
            .as_object_mut()
            .unwrap()
            .insert("expression_source".into(), serde_json::json!("x + 99"));
    }
    std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();

    // 断言 AstDatabase::open 在校验 mainline 时立即拦截 expression_hash 不匹配
    let open_res = AstDatabase::open(&db_path);
    match open_res {
        Err(DbError::CorruptedCommitChain(msg)) => {
            assert!(
                msg.contains("candidate hash mismatch")
                    || msg.contains("expression_hash mismatch")
                    || msg.contains("candidate replay derivation failed"),
                "expected expression integrity rejection, got {msg}"
            );
        }
        other => panic!("expected CorruptedCommitChain, got {:?}", other),
    }

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_receipt_hash_includes_solver_sha256_and_backward_compatible() {
    use axiom_db::{
        compute_receipt_hash, current_verification_profile_hash, ProofLevel, VerificationReceipt,
    };

    let rcpt_base = VerificationReceipt {
        candidate_hash: "#cand_1111111111111111111111111111111111111111111111111111111111111111"
            .parse()
            .unwrap(),
        proof_level: ProofLevel::Proved,
        solver: "Microsoft Z3 (SMT-LIB2)".into(),
        solver_version: "4.13.0".into(),
        solver_sha256: None,
        obligations_checked: 2,
        proof_obligation_hash:
            "#obl_1111111111111111111111111111111111111111111111111111111111111111"
                .parse()
                .unwrap(),
        verification_profile_hash: current_verification_profile_hash(),
        attested_commit_hash: None,
        verified_at: 5000,
    };

    let mut rcpt_with_sha = rcpt_base.clone();
    rcpt_with_sha.solver_sha256 =
        Some("d486350238477489eebcac6165cb56e83e7b2dd6c0b4347ada0be2ba9ae215e6".into());

    let h_base = compute_receipt_hash(&rcpt_base);
    let h_with_sha = compute_receipt_hash(&rcpt_with_sha);

    assert_ne!(
        h_base, h_with_sha,
        "solver_sha256 must be integrated into compute_receipt_hash cryptographic envelope"
    );

    // 验证向后兼容：反序列化不带 solver_sha256 的历史 JSON
    let legacy_json = r##"{
        "candidate_hash": "#cand_1111111111111111111111111111111111111111111111111111111111111111",
        "proof_level": "Proved",
        "solver": "z3",
        "solver_version": "4.12.0",
        "obligations_checked": 2,
        "proof_obligation_hash": "#obl_1111111111111111111111111111111111111111111111111111111111111111",
        "verification_profile_hash": "#vprof_0000000000000000000000000000000000000000000000000000000000000000",
        "attested_commit_hash": null,
        "verified_at": 5000
    }"##;

    let deserialized: VerificationReceipt = serde_json::from_str(legacy_json).unwrap();
    assert_eq!(deserialized.solver_sha256, None);
    assert_eq!(deserialized.solver, "z3");

    // 关键断言：当 solver_sha256 为 None 时，跳过序列化该字段，避免输出 solver_sha256: null 破坏历史哈希
    let serialized_again = serde_json::to_string(&deserialized).unwrap();
    assert!(
        !serialized_again.contains("solver_sha256"),
        "None 值的 solver_sha256 严禁序列化为 null，必须保持与历史 Schema V3 字节完全一致"
    );
}

#[test]
fn test_v3_database_golden_fixture_migration_to_v4() {
    use axiom_ast::{BinaryOperator, Expr, FunctionDecl, FunctionParam, Literal, Module, Type};
    use axiom_db::{
        compute_candidate_hash, compute_commit_payload_hash, compute_contract_hash,
        compute_envelope_hash, compute_expr_hash, compute_module_hash,
        compute_proof_obligation_hash, compute_receipt_hash, AstDatabase, Candidate, CommitPayload,
        CommitRecord, ProofLevel, TaskEnvelope, VerificationReceipt, CURRENT_SCHEMA_VERSION,
    };
    use std::collections::BTreeMap;

    // 构建一个真实的 Schema V3 数据库状态（收据中无 solver_sha256，schema_version 为 3）
    let initial_fn = FunctionDecl {
        name: "test_inc".into(),
        params: vec![FunctionParam {
            name: "x".into(),
            ty: Type::Int,
        }],
        return_type: Type::Int,
        effects: vec![],
        requires: vec![],
        ensures: vec![],
        body: Expr::Hole("h1".into()),
    };
    let initial_mod = Module {
        types: vec![],
        functions: vec![initial_fn.clone()],
        effects: vec![],
    };
    let root_hash = compute_module_hash(&initial_mod);

    let parsed_cand = Expr::BinaryOp {
        op: BinaryOperator::Add,
        left: Box::new(Expr::Ident("x".into())),
        right: Box::new(Expr::Literal(Literal::Int(1))),
    };
    let cand_expr_hash = compute_expr_hash(&parsed_cand);

    let env = TaskEnvelope {
        target_hole_id: "h1".into(),
        target_function: "test_inc".into(),
        contract_hash: compute_contract_hash(&initial_fn),
        base_root_hash: Some(root_hash.clone()),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&env);

    let cand = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "x + 1".into(),
        expression_hash: cand_expr_hash,
        author_agent_id: "agent_golden".into(),
        timestamp: 1000,
    };
    let cand_hash = compute_candidate_hash(&cand);

    let landed_fn = FunctionDecl {
        name: "test_inc".into(),
        params: vec![FunctionParam {
            name: "x".into(),
            ty: Type::Int,
        }],
        return_type: Type::Int,
        effects: vec![],
        requires: vec![],
        ensures: vec![],
        body: parsed_cand,
    };
    let landed_mod = Module {
        types: vec![],
        functions: vec![landed_fn.clone()],
        effects: vec![],
    };
    let new_root_hash = compute_module_hash(&landed_mod);
    let obl_hash = compute_proof_obligation_hash(&landed_fn);

    // V3 收据：无 solver_sha256 (None)
    let rcpt_v3 = VerificationReceipt {
        candidate_hash: cand_hash.clone(),
        proof_level: ProofLevel::Proved,
        solver: "z3".into(),
        solver_version: "Z3 4.12.2".into(),
        solver_sha256: None,
        obligations_checked: 1,
        proof_obligation_hash: obl_hash,
        verification_profile_hash: axiom_db::current_verification_profile_hash(),
        attested_commit_hash: None,
        verified_at: 1200,
    };
    let rcpt_hash = compute_receipt_hash(&rcpt_v3);

    let payload_v3 = CommitPayload {
        parent_commit: None,
        parent_root: Some(root_hash.clone()),
        new_root: new_root_hash.clone(),
        envelope_hash: env_hash.clone(),
        candidate_hash: cand_hash.clone(),
        receipt_hash: rcpt_hash.clone(),
        attested_commit_hash: None,
        timestamp: 1300,
    };
    let commit_hash = compute_commit_payload_hash(&payload_v3);
    let commit = CommitRecord {
        commit_hash: commit_hash.clone(),
        payload: payload_v3,
    };

    let mut modules = BTreeMap::new();
    modules.insert(root_hash.clone(), initial_mod);
    modules.insert(new_root_hash.clone(), landed_mod);

    let mut envelopes = BTreeMap::new();
    envelopes.insert(env_hash.clone(), env);

    let mut candidates = BTreeMap::new();
    candidates.insert(cand_hash.clone(), cand);

    let mut receipts = BTreeMap::new();
    receipts.insert(rcpt_hash.clone(), rcpt_v3);

    let mut commits = BTreeMap::new();
    commits.insert(commit_hash.clone(), commit);

    #[derive(serde::Serialize)]
    struct V3GoldenDatabase {
        root_ref: Option<axiom_db::NodeHash>,
        head_commit: Option<axiom_db::NodeHash>,
        schema_version: Option<u32>,
        modules: BTreeMap<axiom_db::NodeHash, Module>,
        envelopes: BTreeMap<axiom_db::NodeHash, TaskEnvelope>,
        candidates: BTreeMap<axiom_db::NodeHash, Candidate>,
        receipts: BTreeMap<axiom_db::NodeHash, VerificationReceipt>,
        commits: BTreeMap<axiom_db::NodeHash, CommitRecord>,
    }

    let golden_v3 = V3GoldenDatabase {
        root_ref: Some(new_root_hash.clone()),
        head_commit: Some(commit_hash.clone()),
        schema_version: Some(3), // 历史 Schema V3 标识
        modules,
        envelopes,
        candidates,
        receipts,
        commits,
    };

    let temp_dir = std::env::temp_dir().join(format!("test_v3_golden_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("v3_golden.json");

    let json_bytes = serde_json::to_vec_pretty(&golden_v3).expect("serialize v3 golden db");
    std::fs::write(&db_path, json_bytes).expect("write golden v3 db");

    // 打开历史 Schema V3 数据库：验证能够无缝加载、验真历史链并升级到 Schema V4
    let db = AstDatabase::open(&db_path).expect("open v3 golden db should succeed");
    assert_eq!(db.head_commit(), Some(&commit_hash));
    assert_eq!(db.current_root(), Some(new_root_hash));

    // 验证主链深层验真通过
    let mainline = db.mainline_commits().expect("mainline should validate");
    assert_eq!(mainline.len(), 1);
    assert_eq!(mainline[0].commit_hash, commit_hash);

    // [Round 20 P1 Spec 1]: 验证旧收据在当前 v4 规则下不属于当前有效证明（缺少 solver_sha256 且 profile 已升级）
    let rcpt_loaded = db
        .get_receipt(&rcpt_hash)
        .expect("get receipt")
        .expect("receipt must exist");
    let current_profile = axiom_db::current_verification_profile_hash();
    assert!(
        !rcpt_loaded.is_currently_valid_proved(&current_profile),
        "历史 V3 收据缺少合法 solver_sha256，在当前规则下必须被判定为非当前有效证明并触发重验"
    );

    // 重新从磁盘读取原始文件，断言已被原子升级保存为 Schema 4
    let upgraded_json = std::fs::read_to_string(&db_path).expect("read upgraded db");
    let upgraded_state: serde_json::Value =
        serde_json::from_str(&upgraded_json).expect("parse upgraded db");
    assert_eq!(
        upgraded_state["schema_version"],
        serde_json::json!(CURRENT_SCHEMA_VERSION),
        "数据库落盘后 schema_version 必须已升级为 CURRENT_SCHEMA_VERSION"
    );

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_verification_profile_v8_invalidates_v7_receipts() {
    let v7_profile_str = "axiom-smt-v7-anf-hygienic-caller-requires-micros-abi";
    let v7_profile_hash = axiom_db::compute_hash("vprof", &v7_profile_str);
    let current_v8_profile = axiom_db::current_verification_profile_hash();

    assert_ne!(
        v7_profile_hash, current_v8_profile,
        "v7 profile hash must not match current v8 profile hash"
    );

    let rcpt_v7 = axiom_db::VerificationReceipt {
        candidate_hash: axiom_db::compute_hash("cand", &"test_cand"),
        proof_level: axiom_db::ProofLevel::Proved,
        solver: "Microsoft Z3 (SMT-LIB2)".into(),
        solver_version: "4.12.2".into(),
        solver_sha256: Some(
            "abcdef0123456789abcdef0123456789abcdef0123456789abcdef0123456789".into(),
        ),
        obligations_checked: 5,
        proof_obligation_hash: axiom_db::compute_hash("obl", &"test_obl"),
        verification_profile_hash: v7_profile_hash,
        attested_commit_hash: None,
        verified_at: 1000,
    };

    assert!(
        !rcpt_v7.is_currently_valid_proved(&current_v8_profile),
        "v7 receipt must be rejected by is_currently_valid_proved under current v8 profile"
    );

    let mut rcpt_v8 = rcpt_v7.clone();
    rcpt_v8.verification_profile_hash = current_v8_profile.clone();
    assert!(
        rcpt_v8.is_currently_valid_proved(&current_v8_profile),
        "v8 receipt with valid sha256 and version must be accepted under current v8 profile"
    );
}
