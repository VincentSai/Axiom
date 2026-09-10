use std::sync::{Arc, Mutex};
use std::thread;

use axiom_agent_pool::{AgentTaskPool, PoolError, PoolVerificationPolicy, SmtResult};
use axiom_ast::Expr;
use axiom_transpiler::transpile_to_rust;

const TWO_HOLE_SKELETON: &str = r#"
    fn increment(value: Int) -> Int
        requires { value >= 0, value < 1000000 }
        ensures { result > value }
    {
        ?increment_impl
    }

    fn double(value: Int) -> Int
        requires { value >= 0, value < 1000000 }
        ensures { result >= value }
    {
        ?double_impl
    }
"#;

#[test]
fn dispatches_a_task_for_each_named_hole() {
    let pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON).expect("skeleton should parse");

    let tasks = pool.dispatch_tasks();
    assert_eq!(tasks.len(), 2);
    assert_eq!(
        tasks
            .iter()
            .map(|task| task.hole_id.as_str())
            .collect::<Vec<_>>(),
        vec!["increment_impl", "double_impl"]
    );
    assert_eq!(
        pool.pending_hole_ids(),
        vec!["increment_impl", "double_impl"]
    );
    assert!(!pool.is_complete());
}

#[test]
fn concurrent_workers_get_counter_examples_without_contaminating_the_pool() {
    let pool = Arc::new(Mutex::new(
        AgentTaskPool::from_source(TWO_HOLE_SKELETON)
            .expect("skeleton should parse")
            .with_policy(PoolVerificationPolicy::Permissive),
    ));

    let bad_increment_worker = {
        let pool = Arc::clone(&pool);
        thread::spawn(move || {
            pool.lock()
                .expect("pool lock should not be poisoned")
                .submit_solution("increment_impl", "value - 1")
                .expect("candidate should parse")
        })
    };
    let valid_double_worker = {
        let pool = Arc::clone(&pool);
        thread::spawn(move || {
            pool.lock()
                .expect("pool lock should not be poisoned")
                .submit_solution("double_impl", "value * 2")
                .expect("candidate should parse")
        })
    };

    let bad_result = bad_increment_worker
        .join()
        .expect("increment worker should finish");
    let double_result = valid_double_worker
        .join()
        .expect("double worker should finish");
    assert!(matches!(bad_result, SmtResult::CounterExample(_)));
    assert!(matches!(
        double_result,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));

    let mut pool = pool.lock().expect("pool lock should not be poisoned");
    assert_eq!(pool.pending_hole_ids(), vec!["increment_impl"]);
    assert!(matches!(
        pool.assemble_verified_module(),
        Err(PoolError::Incomplete(ids)) if ids == vec!["increment_impl"]
    ));

    let fix_res = pool
        .submit_solution("increment_impl", "value + 1")
        .expect("fixed candidate should parse");
    assert!(matches!(
        fix_res,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));
}

#[test]
fn assembles_a_hole_free_module_that_transpiles_to_rust() {
    let mut pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON)
        .expect("skeleton should parse")
        .with_policy(PoolVerificationPolicy::Permissive);
    let inc_res = pool
        .submit_solution("increment_impl", "value + 1")
        .expect("increment solution should parse");
    assert!(matches!(
        inc_res,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));
    let dbl_res = pool
        .submit_solution("double_impl", "value * 2")
        .expect("double solution should parse");
    assert!(matches!(
        dbl_res,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));

    assert!(pool.is_complete());
    assert!(pool.pending_hole_ids().is_empty());
    assert!(pool.dispatch_tasks().is_empty());

    let module = pool
        .assemble_verified_module()
        .expect("complete pool should assemble");
    assert!(module
        .functions
        .iter()
        .all(|function| !matches!(function.body, Expr::Hole(_))));

    let rust = transpile_to_rust(&module).expect("verified module should transpile");
    assert!(rust.contains("pub fn increment(value: i64) -> i64"));
    assert!(rust.contains("pub fn double(value: i64) -> i64"));
    assert!(!rust.contains('?'));
}

#[test]
fn default_strict_proved_policy_rejects_unproved_sampled_solutions() {
    let mut pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON).expect("skeleton should parse");
    assert_eq!(pool.policy(), PoolVerificationPolicy::StrictProved);

    // If external Z3 is not installed, submitting contracted hole returns Sampled, which MUST be rejected under StrictProved
    if axiom_smt::find_z3_executable().is_none() {
        let res = pool
            .submit_solution("increment_impl", "value + 1")
            .expect("candidate parses");
        assert!(matches!(res, SmtResult::Sampled { .. }));
        // Must NOT be resolved, pending holes remain
        assert!(pool
            .pending_hole_ids()
            .contains(&"increment_impl".to_string()));
    }
}

#[test]
fn verifies_cas_optimistic_commit_and_audit_receipts() {
    let mut pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON)
        .expect("skeleton should parse")
        .with_policy(PoolVerificationPolicy::Permissive);

    let initial_root = pool.current_root().expect("initial root should be set");

    let res1 = pool
        .submit_solution_with_agent("increment_impl", "value + 1", "agent-alpha")
        .expect("submit should succeed");
    assert!(matches!(
        res1,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));

    // Verify commit record was minted
    if axiom_smt::find_z3_executable().is_some() {
        assert_eq!(pool.commits().len(), 1);
        let commit1 = pool.commits()[0].clone();
        assert_eq!(commit1.parent_root, Some(initial_root));
        assert_ne!(commit1.new_root, commit1.parent_root.clone().unwrap());

        // Second submission by beta agent on next hole
        let res2 = pool
            .submit_solution_with_agent("double_impl", "value * 2", "agent-beta")
            .expect("submit should succeed");
        assert!(matches!(res2, SmtResult::Proved { .. }));
        assert_eq!(pool.commits().len(), 2);
        let commit2 = &pool.commits()[1];
        assert_eq!(commit2.parent_root, Some(commit1.new_root.clone()));
    }
}

#[test]
fn pool_rejects_vacuous_contracts_without_polluting_state() {
    const VACUOUS_SKELETON: &str = r#"
        fn impossible(value: Int) -> Int
            requires { value > 10 }
            requires { value < 5 }
            ensures { result == 0 }
        {
            ?impossible_impl
        }
    "#;

    let mut pool = AgentTaskPool::from_source(VACUOUS_SKELETON).expect("skeleton parses");
    let res = pool
        .submit_solution("impossible_impl", "0")
        .expect("submit completes");

    if axiom_smt::find_z3_executable().is_some() {
        assert!(matches!(res, SmtResult::VacuousContract { .. }));
        // State remains incomplete and hole is not resolved
        assert!(!pool.is_complete());
        assert_eq!(pool.pending_hole_ids(), vec!["impossible_impl"]);
    }
}

#[test]
fn assemble_verified_module_fails_closed_on_unresolved_holes_while_provisional_module_succeeds() {
    let pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON).expect("skeleton parses");
    // 1. 严格模式下，有洞时 assemble_verified_module 必须强报错拒绝导出
    assert!(matches!(
        pool.assemble_verified_module(),
        Err(PoolError::Incomplete(_))
    ));

    // 2. 草稿导出允许用于审查与预览
    let provisional = pool.assemble_provisional_module();
    assert_eq!(provisional.functions.len(), 2);
    assert!(matches!(provisional.functions[0].body, Expr::Hole(_)));
}

#[test]
fn fast_forward_reverification_mints_clean_receipt_and_commits_successfully() {
    let mut pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON)
        .expect("skeleton parses")
        .with_policy(PoolVerificationPolicy::Permissive);

    // 第一个 Worker 解决 increment_impl
    let res1 = pool
        .submit_solution_with_agent("increment_impl", "value + 1", "worker_1")
        .expect("worker 1 should succeed");
    assert!(matches!(
        res1,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));

    // 第二个 Worker 解决 double_impl
    let res2 = pool
        .submit_solution_with_agent("double_impl", "value * 2", "worker_2")
        .expect("worker 2 should succeed");
    assert!(matches!(
        res2,
        SmtResult::Proved { .. } | SmtResult::Sampled { .. }
    ));

    assert!(pool.is_complete());
    assert_eq!(pool.pending_hole_ids().len(), 0);

    // 两个洞均已解决，Verified Module 成功导出
    let verified_mod = pool.assemble_verified_module().expect("all holes resolved");
    assert_eq!(verified_mod.functions.len(), 2);
    assert!(!matches!(verified_mod.functions[0].body, Expr::Hole(_)));
    assert!(!matches!(verified_mod.functions[1].body, Expr::Hole(_)));
}

#[test]
fn real_cross_instance_stale_root_fast_forward_rebasing() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skeleton_path = std::env::temp_dir().join(format!("axiom-test-skel-{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom-test-db-{nonce}.json"));

    std::fs::write(&skeleton_path, TWO_HOLE_SKELETON).unwrap();

    // 实例 1 打开共享数据库与骨架
    let mut pool1 = AgentTaskPool::open_persisted(&skeleton_path, &db_path).unwrap();
    let initial_root = pool1.current_root().unwrap();

    // 实例 2 同时打开同一个共享数据库与骨架
    let mut pool2 = AgentTaskPool::open_persisted(&skeleton_path, &db_path).unwrap();
    assert_eq!(pool2.current_root(), Some(initial_root.clone()));

    // 实例 1 解决 increment_impl 并推进 Root 落盘
    let res1 = pool1
        .submit_solution_with_agent("increment_impl", "value + 1", "worker_1")
        .expect("worker 1 submit should succeed");
    assert!(res1.is_proved());
    let root_after_pool1 = pool1.current_root().unwrap();
    assert_ne!(root_after_pool1, initial_root);

    // 实例 2 仍然持有旧的 initial_root 信封，尝试提交 double_impl
    // 此时由于底层文件数据库中 root 已经变为 root_after_pool1，CAS 检测出 StaleRoot，
    // pool2 自动触发 Fast-Forward 重基：从数据库中物理加载 root_after_pool1 的真实 AST
    // （该 AST 中已经包含了 pool1 填入的 increment_impl 实现！），在其上填入 double_impl 并重验成功！
    let res2 = pool2
        .submit_solution_with_agent("double_impl", "value * 2", "worker_2")
        .expect("worker 2 submit with fast-forward rebasing should succeed");
    assert!(res2.is_proved());
    let final_root = pool2.current_root().unwrap();
    assert_ne!(final_root, root_after_pool1);

    // 实例 3 从磁盘重新打开，验证两项提交均成功合并入库！
    let pool3 = AgentTaskPool::open_persisted(&skeleton_path, &db_path).unwrap();
    let final_mod = pool3.database().get_module(&final_root).unwrap().unwrap();
    assert!(!axiom_agent_pool::ast_contains_any_hole(&final_mod));

    let _ = std::fs::remove_file(&skeleton_path);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn permissive_sampled_solution_rejected_by_assemble_verified_module() {
    use axiom_db::ProofLevel;

    // 骨架包含一个没有 ensures 的纯函数（在当前引擎下无法获得 Proved，只能返回 Skipped 或 Sampled）
    const HEURISTIC_SKELETON: &str = r#"
        fn compute(value: Int) -> Int {
            ?compute_impl
        }
    "#;

    let mut pool = AgentTaskPool::from_source(HEURISTIC_SKELETON)
        .expect("skeleton parses")
        .with_policy(PoolVerificationPolicy::Permissive);

    // 在 Permissive 模式下提交解
    let res = pool
        .submit_solution("compute_impl", "value + 42")
        .expect("permissive mode accepts candidate");

    // 证实证据级别不是 Proved（例如 Sampled 或 Skipped）
    assert!(!res.is_proved());

    // 草稿可以正常导出
    let provisional = pool.assemble_provisional_module();
    assert_eq!(provisional.functions.len(), 1);

    // 严格门禁：assemble_verified_module 必须拒绝，绝不放行至生产！
    let verified_err = pool.assemble_verified_module();
    match verified_err {
        Err(PoolError::InsufficientEvidence { hole_id, level }) => {
            assert_eq!(hole_id, "compute_impl");
            assert_ne!(level, ProofLevel::Proved);
        }
        other => panic!("expected InsufficientEvidence error, got: {:?}", other),
    }
}

#[test]
fn persisted_pool_resumes_across_process_exits_seamlessly() {
    use axiom_agent_pool::HoleStatus;

    const MULTI_STEP_SKELETON: &str = r#"
        fn step_one(x: Int) -> Int
            requires { x >= 0, x < 1000000 }
            ensures { result > x }
        {
            ?step1_hole
        }

        fn step_two(y: Int) -> Int
            requires { y >= 0, y < 1000000 }
            ensures { result >= y }
        {
            ?step2_hole
        }
    "#;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skel_path = std::env::temp_dir().join(format!("axiom_resume_skel_{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom_resume_db_{nonce}.json"));

    std::fs::write(&skel_path, MULTI_STEP_SKELETON).unwrap();

    // 进程 1：初始化并提交 step1_hole，随后“退出”
    {
        let mut pool_p1 = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(pool_p1.pending_hole_ids().len(), 2);
        assert_eq!(
            pool_p1.hole_status("step1_hole"),
            Some(HoleStatus::Unresolved)
        );

        let outcome1 = pool_p1
            .submit_solution_with_outcome("step1_hole", "x + 1", "agent_worker_1")
            .unwrap();
        assert!(outcome1.smt_result.is_proved());
        assert!(
            !outcome1.upgraded_to_verified,
            "First-time submission must not be reported as upgrade from provisional"
        );
        assert_eq!(
            pool_p1.hole_status("step1_hole"),
            Some(HoleStatus::Verified)
        );
    } // 模拟进程 1 退出，内存状态全部销毁

    // 进程 2：从磁盘重新打开同一个 DB 和原始骨架，验证单源权威水合！
    {
        let mut pool_p2 = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        // 关键断言：step1_hole 必须已经被恢复为 Verified，不应出现在 pending 中！
        assert_eq!(
            pool_p2.hole_status("step1_hole"),
            Some(HoleStatus::Verified)
        );
        assert_eq!(
            pool_p2.hole_status("step2_hole"),
            Some(HoleStatus::Unresolved)
        );
        assert_eq!(pool_p2.pending_hole_ids(), vec!["step2_hole".to_string()]);
        assert!(!pool_p2.is_complete());

        // 提交 step2_hole
        let outcome2 = pool_p2
            .submit_solution_with_outcome("step2_hole", "y + 5", "agent_worker_2")
            .unwrap();
        assert!(outcome2.smt_result.is_proved());
        assert!(
            !outcome2.upgraded_to_verified,
            "Direct submission from Unresolved must not be marked upgraded"
        );
        assert_eq!(
            pool_p2.hole_status("step2_hole"),
            Some(HoleStatus::Verified)
        );
        assert!(pool_p2.is_complete());
    } // 进程 2 退出

    // 进程 3：全新进程重新打开，验证全量完成且成功组装发布模块！
    {
        let pool_p3 = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert!(pool_p3.is_complete());
        assert_eq!(pool_p3.pending_hole_ids().len(), 0);

        let verified_module = pool_p3
            .assemble_verified_module()
            .expect("assembled module must succeed");
        assert!(!axiom_agent_pool::ast_contains_any_hole(&verified_module));
    }

    let _ = std::fs::remove_file(&skel_path);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn provisional_draft_can_be_upgraded_to_verified_status() {
    use axiom_agent_pool::HoleStatus;

    const UPGRADE_SKELETON: &str = r#"
        fn safe_inc(x: Int) -> Int
            requires { x >= 0, x < 1000000 }
            ensures { result > x }
        {
            ?safe_inc_hole
        }
    "#;

    let mut pool = AgentTaskPool::from_source(UPGRADE_SKELETON)
        .unwrap()
        .with_policy(PoolVerificationPolicy::Permissive);

    assert_eq!(
        pool.hole_status("safe_inc_hole"),
        Some(HoleStatus::Unresolved)
    );

    // 1. 先提交一个草稿解，真正制造 Provisional 状态
    pool.submit_draft_solution("safe_inc_hole", "x + 5")
        .expect("draft submission should succeed");
    assert_eq!(
        pool.hole_status("safe_inc_hole"),
        Some(HoleStatus::Provisional),
        "hole must transition from Unresolved to Provisional"
    );
    // 草稿模块已可用，但权威已验证模块仍不可发布
    assert!(!axiom_agent_pool::ast_contains_any_hole(
        pool.provisional_module()
    ));
    assert!(pool.assemble_verified_module().is_err());

    // 2. 提交形式化严格证明解，草稿成功无缝升级为 Verified
    let outcome = pool
        .submit_solution_with_outcome("safe_inc_hole", "x + 10", "expert_agent")
        .expect("upgrading submission should succeed without MissingHole error");
    assert!(outcome.smt_result.is_proved());
    assert!(outcome.upgraded_to_verified);
    assert_eq!(
        pool.hole_status("safe_inc_hole"),
        Some(HoleStatus::Verified),
        "hole must transition from Provisional to Verified"
    );

    // 3. 已是 Verified 后，再次提交将被安全拒绝 (AlreadyResolved)
    let re_submit = pool.submit_solution("safe_inc_hole", "x + 20");
    match re_submit {
        Err(PoolError::AlreadyResolved(h)) => assert_eq!(h, "safe_inc_hole"),
        other => panic!("expected AlreadyResolved error, got: {:?}", other),
    }

    // 最终成功交付正式模块
    let final_mod = pool.assemble_verified_module().unwrap();
    assert!(!axiom_agent_pool::ast_contains_any_hole(&final_mod));
}

#[test]
fn multiple_holes_in_same_function_can_be_submitted_sequentially_without_contract_hash_drift() {
    const MULTI_SKELETON: &str = r#"
        fn add_pair(a: Int, b: Int) -> Int {
            ?hole_a + ?hole_b
        }
    "#;

    let mut pool = AgentTaskPool::from_source(MULTI_SKELETON)
        .unwrap()
        .with_policy(PoolVerificationPolicy::Permissive);

    // 1. 提交第一个洞 hole_a
    let res_a = pool
        .submit_solution("hole_a", "a")
        .expect("submitting first hole in function should succeed");
    assert!(matches!(
        res_a,
        SmtResult::Skipped(_) | SmtResult::Sampled { .. }
    ));

    // 2. 提交同一函数中的第二个洞 hole_b：因为使用规范级 compute_contract_hash，契约哈希绝不漂移，绝不报 InvalidEnvelope
    let res_b = pool
        .submit_solution("hole_b", "b")
        .expect("submitting second hole in same function must not fail with InvalidEnvelope");
    assert!(matches!(
        res_b,
        SmtResult::Skipped(_) | SmtResult::Sampled { .. }
    ));

    assert!(
        !pool.is_complete(),
        "unverified provisional holes must not make pool complete"
    );
    assert_eq!(
        pool.hole_status("hole_a"),
        Some(axiom_agent_pool::HoleStatus::Provisional)
    );
    assert_eq!(
        pool.hole_status("hole_b"),
        Some(axiom_agent_pool::HoleStatus::Provisional)
    );
    let draft = pool.assemble_provisional_module();
    assert!(!axiom_agent_pool::ast_contains_any_hole(&draft));
}

#[test]
fn rejects_candidate_containing_unresolved_hole() {
    const HOLE_SKELETON: &str = r#"
        fn compute(x: Int) -> Int
            requires { x >= 0, x < 1000000 }
            ensures { result > x }
        {
            ?target_hole
        }
    "#;

    let mut pool = AgentTaskPool::from_source(HOLE_SKELETON).unwrap();

    // 1. 提交包含自引用或任意 hole 的候选表达式，应立即被拒绝
    let err_submit = pool.submit_solution("target_hole", "?nested_hole + 1");
    match err_submit {
        Err(PoolError::CandidateContainsHole(h)) => assert_eq!(h, "target_hole"),
        other => panic!("expected CandidateContainsHole, got {:?}", other),
    }

    // 2. 提交包含 hole 的草稿解，也应被拒绝
    let err_draft = pool.submit_draft_solution("target_hole", "?target_hole");
    match err_draft {
        Err(PoolError::CandidateContainsHole(h)) => assert_eq!(h, "target_hole"),
        other => panic!("expected CandidateContainsHole, got {:?}", other),
    }
}

#[test]
fn provisional_draft_stays_in_dispatch_tasks_until_verified() {
    use axiom_agent_pool::HoleStatus;

    const SKELETON: &str = r#"
        fn inc(x: Int) -> Int
            requires { x >= 0, x < 1000000 }
            ensures { result > x }
        {
            ?inc_hole
        }
    "#;

    let mut pool = AgentTaskPool::from_source(SKELETON).unwrap();

    assert_eq!(pool.dispatch_tasks().len(), 1);
    assert_eq!(pool.hole_status("inc_hole"), Some(HoleStatus::Unresolved));

    // 1. 提交草稿后，洞变为 Provisional，但依然留在 dispatch_tasks 中，便于调度升级
    pool.submit_draft_solution("inc_hole", "x + 2").unwrap();
    assert_eq!(pool.hole_status("inc_hole"), Some(HoleStatus::Provisional));
    assert_eq!(pool.dispatch_tasks().len(), 1);
    assert!(!pool.is_complete());

    // 2. 提交正式证明后，洞变为 Verified，彻底移出 dispatch_tasks
    let outcome = pool
        .submit_solution_with_outcome("inc_hole", "x + 1", "verifier_agent")
        .unwrap();
    assert!(outcome.smt_result.is_proved());
    assert_eq!(pool.hole_status("inc_hole"), Some(HoleStatus::Verified));
    assert_eq!(pool.dispatch_tasks().len(), 0);
    assert!(pool.is_complete());
}

#[test]
fn persisted_pool_restores_provisional_draft_across_processes_and_upgrades() {
    use axiom_agent_pool::HoleStatus;

    const SKELETON: &str = r#"
        fn compute(x: Int) -> Int
            requires { x >= 0, x < 1000000 }
            ensures { result > x }
        {
            ?comp_hole
        }
    "#;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skel_path = std::env::temp_dir().join(format!("axiom_draft_skel_{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom_draft_db_{nonce}.json"));

    std::fs::write(&skel_path, SKELETON).unwrap();

    // 进程 1：提交草稿后退出
    {
        let mut pool_p1 = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        pool_p1
            .submit_draft_solution("comp_hole", "x + 99")
            .unwrap();
        assert_eq!(
            pool_p1.hole_status("comp_hole"),
            Some(HoleStatus::Provisional)
        );
    }

    // 进程 2：重新打开，验证草稿从数据库成功水合为 Provisional，并成功升级为 Verified
    {
        let mut pool_p2 = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(
            pool_p2.hole_status("comp_hole"),
            Some(HoleStatus::Provisional),
            "Provisional draft must survive process restart via DB persistence"
        );
        // 验证 provisional_module 中草稿洞已被替换，不再含 hole
        assert!(!axiom_agent_pool::ast_contains_any_hole(
            pool_p2.provisional_module()
        ));

        // 提交正式证明，升级为 Verified
        let outcome = pool_p2
            .submit_solution_with_outcome("comp_hole", "x + 1", "worker_upgrader")
            .unwrap();
        assert!(outcome.smt_result.is_proved());
        assert!(
            outcome.upgraded_to_verified,
            "Upgrading from provisional must report true"
        );
        assert_eq!(pool_p2.hole_status("comp_hole"), Some(HoleStatus::Verified));
    }

    // 进程 3：重新打开，验证已完全持久化为 Verified，并且 DB 草稿已被清理
    {
        let pool_p3 = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(pool_p3.hole_status("comp_hole"), Some(HoleStatus::Verified));
        assert!(pool_p3.database().drafts().is_empty());
    }

    let _ = std::fs::remove_file(&skel_path);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn permissive_skipped_and_sampled_persist_drafts_to_db() {
    use axiom_agent_pool::{HoleStatus, PoolVerificationPolicy};

    // 无 ensures 的函数在 Permissive 模式下返回 Skipped，并作为 Provisional 草稿持久化
    const SKIPPED_SKELETON: &str = r#"
        fn effect_worker(x: Int) -> Int
            requires { x >= 0 }
        {
            ?worker_hole
        }
    "#;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skel_path = std::env::temp_dir().join(format!("axiom_permissive_skel_{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom_permissive_db_{nonce}.json"));

    std::fs::write(&skel_path, SKIPPED_SKELETON).unwrap();

    // 进程 1：Permissive 模式下提交草稿
    {
        let mut pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        pool.set_policy(PoolVerificationPolicy::Permissive);

        let outcome = pool
            .submit_solution_with_outcome("worker_hole", "x + 42", "agent_worker_x")
            .unwrap();
        assert!(matches!(
            outcome.smt_result,
            axiom_smt::SmtResult::Skipped(_)
        ));
        assert_eq!(
            pool.hole_status("worker_hole"),
            Some(HoleStatus::Provisional)
        );
        assert_eq!(pool.database().drafts().len(), 1);
    }

    // 进程 2：重启进程打开，草稿必须完好恢复
    {
        let pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(
            pool.hole_status("worker_hole"),
            Some(HoleStatus::Provisional)
        );
        assert_eq!(pool.database().drafts().len(), 1);
    }

    let _ = std::fs::remove_file(&skel_path);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn profile_invalidation_and_reverify_unproved_holes_closed_loop() {
    use axiom_agent_pool::HoleStatus;
    use axiom_db::legacy_verification_profile_hash;

    const REVERIFY_SKELETON: &str = r#"
        fn add_safe(a: Int, b: Int) -> Int
            requires { a >= 0 && a < 100, b >= 0 && b < 100 }
            ensures { result == a + b }
        {
            ?add_hole
        }
    "#;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skel_path = std::env::temp_dir().join(format!("axiom_reverify_skel_{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom_reverify_db_{nonce}.json"));

    std::fs::write(&skel_path, REVERIFY_SKELETON).unwrap();

    // 1. 提交并通过验证
    {
        let mut pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        let outcome = pool
            .submit_solution_with_outcome("add_hole", "a + b", "agent_1")
            .unwrap();
        assert!(outcome.smt_result.is_proved());
        assert_eq!(pool.hole_status("add_hole"), Some(HoleStatus::Verified));
    }

    // 2. 模拟底层收据来自旧规则（将收据的 profile hash 更新为 legacy 并同步自哈希与 commit 引用）
    {
        let db_content = std::fs::read_to_string(&db_path).unwrap();
        let mut val: serde_json::Value = serde_json::from_str(&db_content).unwrap();
        let leg_hash = legacy_verification_profile_hash();

        let receipts = val.get_mut("receipts").unwrap().as_object_mut().unwrap();
        let old_rcpt_hash = receipts.keys().next().unwrap().clone();
        let mut rcpt: axiom_db::VerificationReceipt =
            serde_json::from_value(receipts.remove(&old_rcpt_hash).unwrap()).unwrap();
        rcpt.verification_profile_hash = leg_hash;
        let new_rcpt_hash = axiom_db::compute_receipt_hash(&rcpt);
        receipts.insert(
            new_rcpt_hash.as_str().into(),
            serde_json::to_value(&rcpt).unwrap(),
        );

        let commits = val.get_mut("commits").unwrap().as_object_mut().unwrap();
        let old_commit_hash = commits.keys().next().unwrap().clone();
        let mut commit_rec: axiom_db::CommitRecord =
            serde_json::from_value(commits.remove(&old_commit_hash).unwrap()).unwrap();
        commit_rec.payload.receipt_hash = new_rcpt_hash;
        commit_rec.commit_hash = axiom_db::compute_commit_payload_hash(&commit_rec.payload);
        let new_commit_hash = commit_rec.commit_hash.clone();
        commits.insert(
            new_commit_hash.as_str().into(),
            serde_json::to_value(&commit_rec).unwrap(),
        );

        val.as_object_mut().unwrap().insert(
            "head_commit".into(),
            serde_json::to_value(&new_commit_hash).unwrap(),
        );

        std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();
    }

    // 3. 重新打开池：断言已填洞被降级为 Unresolved，并且被重新收录进 pending_holes
    {
        let mut pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(pool.hole_status("add_hole"), Some(HoleStatus::Unresolved));
        assert!(pool.pending_hole_ids().contains(&"add_hole".to_string()));

        // 4. 调用 reverify_unproved_holes 自动重新定理证明，闭环晋升为 Verified
        let rev_results = pool.reverify_unproved_holes().unwrap();
        assert_eq!(rev_results.len(), 1);
        assert!(matches!(
            &rev_results[0],
            axiom_agent_pool::ReverifyOutcome::Committed { hole_id, .. } if hole_id == "add_hole"
        ));
        assert_eq!(pool.hole_status("add_hole"), Some(HoleStatus::Verified));
        assert!(!pool.pending_hole_ids().contains(&"add_hole".to_string()));
    }

    let _ = std::fs::remove_file(&skel_path);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn nested_hole_reattestation_reverify_closed_loop() {
    use axiom_db::legacy_verification_profile_hash;

    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skel_path = std::env::temp_dir().join(format!("axiom_nested_skel_{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom_nested_db_{nonce}.json"));

    // [Spec 5, P1] 函数体为嵌套打洞：1 + ?inner_hole
    let skel = r#"
    fn test_nested(x: Int) -> Int
        requires { x >= 0 }
        ensures { result > 0 }
    {
        1 + ?inner_hole
    }
    "#;
    std::fs::write(&skel_path, skel).unwrap();

    // 1. 提交单洞候选表达式 "2"，函数被填充为 "1 + 2" 并正式提交
    {
        let mut pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        pool.submit_solution("inner_hole", "2").unwrap();
        assert_eq!(
            pool.hole_status("inner_hole"),
            Some(axiom_agent_pool::HoleStatus::Verified)
        );
    }

    // 2. 模拟升级规则集，将旧收据的 profile hash 更新为 legacy 并同步自哈希与 commit 引用
    {
        let db_content = std::fs::read_to_string(&db_path).unwrap();
        let mut val: serde_json::Value = serde_json::from_str(&db_content).unwrap();
        let leg_hash = legacy_verification_profile_hash();

        let receipts = val.get_mut("receipts").unwrap().as_object_mut().unwrap();
        let old_rcpt_hash = receipts.keys().next().unwrap().clone();
        let mut rcpt: axiom_db::VerificationReceipt =
            serde_json::from_value(receipts.remove(&old_rcpt_hash).unwrap()).unwrap();
        rcpt.verification_profile_hash = leg_hash;
        let new_rcpt_hash = axiom_db::compute_receipt_hash(&rcpt);
        receipts.insert(
            new_rcpt_hash.as_str().into(),
            serde_json::to_value(&rcpt).unwrap(),
        );

        let commits = val.get_mut("commits").unwrap().as_object_mut().unwrap();
        let old_commit_hash = commits.keys().next().unwrap().clone();
        let mut commit_rec: axiom_db::CommitRecord =
            serde_json::from_value(commits.remove(&old_commit_hash).unwrap()).unwrap();
        commit_rec.payload.receipt_hash = new_rcpt_hash;
        commit_rec.commit_hash = axiom_db::compute_commit_payload_hash(&commit_rec.payload);
        let new_commit_hash = commit_rec.commit_hash.clone();
        commits.insert(
            new_commit_hash.as_str().into(),
            serde_json::to_value(&commit_rec).unwrap(),
        );

        val.as_object_mut().unwrap().insert(
            "head_commit".into(),
            serde_json::to_value(&new_commit_hash).unwrap(),
        );

        std::fs::write(&db_path, serde_json::to_string_pretty(&val).unwrap()).unwrap();
    }

    // 3. 重新打开池：当前函数体已为 "1 + 2"（无洞），断言降级并重新收录进 pending_holes
    {
        let mut pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(
            pool.hole_status("inner_hole"),
            Some(axiom_agent_pool::HoleStatus::Unresolved)
        );
        assert!(pool.pending_hole_ids().contains(&"inner_hole".to_string()));

        // 4. 执行重验：即使当前函数已为 "1 + 2"，原生 Re-attestation 机制也能成功换发新收据！
        let rev_results = pool.reverify_unproved_holes().unwrap();
        assert_eq!(rev_results.len(), 1);
        assert!(matches!(
            &rev_results[0],
            axiom_agent_pool::ReverifyOutcome::Committed { hole_id, .. } if hole_id == "inner_hole"
        ));
        assert_eq!(
            pool.hole_status("inner_hole"),
            Some(axiom_agent_pool::HoleStatus::Verified)
        );
        assert!(pool.is_complete());
    }

    let _ = std::fs::remove_file(&skel_path);
    let _ = std::fs::remove_file(&db_path);
}

#[test]
fn reopen_pool_hydrates_commits_in_causal_order_preventing_hash_order_inversion() {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let skel_path = std::env::temp_dir().join(format!("axiom_causal_skel_{nonce}.ax"));
    let db_path = std::env::temp_dir().join(format!("axiom_causal_db_{nonce}.json"));

    let skel = r#"
    fn compute(x: Int) -> Int
        requires { x >= 0, x < 1000 }
        ensures { result > 0 }
    {
        ?calc_hole
    }
    "#;
    std::fs::write(&skel_path, skel).unwrap();

    {
        let mut pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        pool.submit_solution("calc_hole", "x + 1").unwrap();
        assert_eq!(
            pool.hole_status("calc_hole"),
            Some(axiom_agent_pool::HoleStatus::Verified)
        );
    }

    // 重启池，验证因果拓扑正序水合后证据账本完备
    {
        let pool = AgentTaskPool::open_persisted(&skel_path, &db_path).unwrap();
        assert_eq!(
            pool.hole_status("calc_hole"),
            Some(axiom_agent_pool::HoleStatus::Verified)
        );
        assert!(pool.is_complete());
    }

    let _ = std::fs::remove_file(&skel_path);
    let _ = std::fs::remove_file(&db_path);
}
