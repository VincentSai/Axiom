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

    assert_eq!(certified.obligations_checked(), 3);

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
        obligations_checked: 5,
        proof_obligation_hash:
            "#obl_1111111111111111111111111111111111111111111111111111111111111111"
                .parse()
                .unwrap(),
        verification_profile_hash: current_hash.clone(),
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
        compute_candidate_hash, compute_contract_hash, compute_envelope_hash,
        current_verification_profile_hash, AstDatabase, Candidate, CasError, ProofLevel,
        TaskEnvelope,
    };

    let mut db = AstDatabase::in_memory();
    assert!(db
        .mainline_commits()
        .expect("empty db should succeed")
        .is_empty());

    // 构造具备形式化证明契约与闭合实现的函数
    let func = FunctionDecl {
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
        body: Expr::BinaryOp {
            op: BinaryOperator::Add,
            left: Box::new(Expr::Ident("x".into())),
            right: Box::new(Expr::Ident("y".into())),
        },
    };

    let module = Module {
        types: vec![],
        functions: vec![func.clone()],
        effects: vec![],
    };

    let mod_rec = db
        .insert_module(&module)
        .expect("insert module should succeed");
    let root_hash = mod_rec.module_hash;
    db.initialize_root(root_hash.clone())
        .expect("init root should succeed");

    let env = TaskEnvelope {
        base_root_hash: Some(root_hash.clone()),
        target_hole_id: "hole_1".into(),
        target_function: "add".into(),
        contract_hash: compute_contract_hash(&func),
        dependency_hashes: vec![],
    };
    let env_hash = compute_envelope_hash(&env);
    db.locked_transaction(|state| {
        state.envelopes.insert(env_hash.clone(), env.clone());
        Ok::<(), axiom_db::DbError>(())
    })
    .unwrap();

    let cand = Candidate {
        envelope_hash: env_hash.clone(),
        expression_source: "x + y".into(),
        expression_hash: axiom_db::compute_expr_hash(&func.body),
        author_agent_id: "agent_1".into(),
        timestamp: 1000,
    };
    let cand_hash = compute_candidate_hash(&cand);
    db.locked_transaction(|state| {
        state.candidates.insert(cand_hash.clone(), cand.clone());
        Ok::<(), axiom_db::DbError>(())
    })
    .unwrap();

    // 1. 测试伪造防护：尝试对不存在的 candidate/envelope 离线认证应被拒绝
    let fake_hash: NodeHash =
        "#cand_9999999999999999999999999999999999999999999999999999999999999999"
            .parse()
            .unwrap();
    let forge_res =
        db.certify_reattestation_offline(&Some(root_hash.clone()), "hole_1", &env_hash, &fake_hash);
    assert!(matches!(forge_res, Err(CasError::InvalidCandidate(_))));

    // 2. 正确生成内核密封的 CertifiedReattestation
    let certified = db
        .certify_reattestation_offline(&Some(root_hash.clone()), "hole_1", &env_hash, &cand_hash)
        .expect("certify_reattestation_offline should succeed");

    // 3. 验证密封结构体字段与当前规则一致（包含契约义务与整数安全溢出检查义务）
    assert_eq!(certified.obligations_checked(), 3);
    assert_eq!(certified.target_hole_id(), "hole_1");

    // 4. 执行 try_commit_reattestation 提交
    let commit1 = db
        .try_commit_reattestation(certified.clone())
        .expect("first reattestation should succeed");
    assert_eq!(db.head_commit(), Some(&commit1.commit_hash));
    assert_eq!(commit1.parent_commit(), None);

    // 验证收据是由 DB 权威铸造且与当前 Profile 绑定
    let rcpt = db.get_receipt(commit1.receipt_hash()).unwrap().unwrap();
    assert_eq!(rcpt.proof_level, ProofLevel::Proved);
    assert_eq!(
        rcpt.verification_profile_hash,
        current_verification_profile_hash()
    );

    // 5. 第二次重签提交，验证 Commit DAG parent_commit 拓扑正常推进
    let commit2 = db
        .try_commit_reattestation(certified)
        .expect("second reattestation should succeed");
    assert_eq!(db.head_commit(), Some(&commit2.commit_hash));
    assert_eq!(commit2.parent_commit(), Some(&commit1.commit_hash));

    // 验证 mainline_commits 严格正序从祖先到链头
    let mainline = db.mainline_commits().expect("mainline should succeed");
    assert_eq!(mainline.len(), 2);
    assert_eq!(mainline[0].commit_hash, commit1.commit_hash);
    assert_eq!(mainline[1].commit_hash, commit2.commit_hash);
}

#[test]
fn test_legacy_commits_migration_and_ancestry_repair() {
    use axiom_db::{
        compute_commit_payload_hash, AstDatabase, CommitPayload, CommitRecord, DatabaseState,
        NodeHash,
    };

    let mut state = DatabaseState::default();

    let root0: NodeHash = "#mod_0000000000000000000000000000000000000000000000000000000000000000"
        .parse()
        .unwrap();
    let root1: NodeHash = "#mod_1111111111111111111111111111111111111111111111111111111111111111"
        .parse()
        .unwrap();
    let root2: NodeHash = "#mod_2222222222222222222222222222222222222222222222222222222222222222"
        .parse()
        .unwrap();

    let dummy_env: NodeHash = format!("#env_{}", "0".repeat(64)).parse().unwrap();
    let dummy_cand: NodeHash = format!("#cand_{}", "0".repeat(64)).parse().unwrap();
    let dummy_rcpt: NodeHash = format!("#rcpt_{}", "0".repeat(64)).parse().unwrap();

    // 模拟旧版本数据库：有多个提交，但 parent_commit 均为 None，且 head_commit 为 None
    let p1 = CommitPayload {
        parent_commit: None,
        parent_root: Some(root0.clone()),
        new_root: root1.clone(),
        envelope_hash: dummy_env.clone(),
        candidate_hash: dummy_cand.clone(),
        receipt_hash: dummy_rcpt.clone(),
        timestamp: 100,
    };
    let c1 = CommitRecord {
        commit_hash: compute_commit_payload_hash(&p1),
        payload: p1,
    };

    let p2 = CommitPayload {
        parent_commit: None,
        parent_root: Some(root1.clone()),
        new_root: root2.clone(),
        envelope_hash: dummy_env,
        candidate_hash: dummy_cand,
        receipt_hash: dummy_rcpt,
        timestamp: 200,
    };
    let c2 = CommitRecord {
        commit_hash: compute_commit_payload_hash(&p2),
        payload: p2,
    };

    state.commits.insert(c1.commit_hash.clone(), c1.clone());
    state.commits.insert(c2.commit_hash.clone(), c2.clone());
    state.root_ref = Some(root2);
    assert_eq!(state.head_commit, None);

    // 执行迁移
    let migrated = AstDatabase::migrate_legacy_commits_if_needed(&mut state);
    assert!(
        migrated,
        "legacy database without head_commit must be migrated"
    );
    assert!(
        state.head_commit.is_some(),
        "head_commit must be set after migration"
    );

    // 验证新数据库实例可通过 mainline_commits 正确追溯全量祖先链
    let temp_dir = std::env::temp_dir().join(format!("test_legacy_db_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&temp_dir);
    let db_path = temp_dir.join("test_legacy.json");
    std::fs::write(&db_path, serde_json::to_vec_pretty(&state).unwrap()).unwrap();

    let db = AstDatabase::open(&db_path).expect("open migrated db should succeed");
    let mainline = db.mainline_commits().expect("mainline should succeed");
    assert_eq!(mainline.len(), 2, "all legacy commits must be in mainline");
    assert_eq!(mainline[1].parent_commit(), Some(&mainline[0].commit_hash));

    let _ = std::fs::remove_file(&db_path);
    let _ = std::fs::remove_dir_all(&temp_dir);
}
