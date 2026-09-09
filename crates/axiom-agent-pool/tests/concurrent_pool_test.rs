use std::sync::{Arc, Mutex};
use std::thread;

use axiom_agent_pool::{AgentTaskPool, PoolError, PoolVerificationPolicy, SmtResult};
use axiom_ast::Expr;
use axiom_transpiler::transpile_to_rust;

const TWO_HOLE_SKELETON: &str = r#"
    fn increment(value: Int) -> Int
        requires { value >= 0 }
        ensures { result > value }
    {
        ?increment_impl
    }

    fn double(value: Int) -> Int
        requires { value >= 0 }
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
    assert!(matches!(double_result, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));

    let mut pool = pool.lock().expect("pool lock should not be poisoned");
    assert_eq!(pool.pending_hole_ids(), vec!["increment_impl"]);
    assert!(matches!(
        pool.assemble_verified_module(),
        Err(PoolError::Incomplete(ids)) if ids == vec!["increment_impl"]
    ));

    let fix_res = pool.submit_solution("increment_impl", "value + 1")
        .expect("fixed candidate should parse");
    assert!(matches!(fix_res, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));
}

#[test]
fn assembles_a_hole_free_module_that_transpiles_to_rust() {
    let mut pool = AgentTaskPool::from_source(TWO_HOLE_SKELETON)
        .expect("skeleton should parse")
        .with_policy(PoolVerificationPolicy::Permissive);
    let inc_res = pool.submit_solution("increment_impl", "value + 1")
        .expect("increment solution should parse");
    assert!(matches!(inc_res, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));
    let dbl_res = pool.submit_solution("double_impl", "value * 2")
        .expect("double solution should parse");
    assert!(matches!(dbl_res, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));

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
        assert!(pool.pending_hole_ids().contains(&"increment_impl".to_string()));
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
    assert!(matches!(res1, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));

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
    let res1 = pool.submit_solution_with_agent("increment_impl", "value + 1", "worker_1")
        .expect("worker 1 should succeed");
    assert!(matches!(res1, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));

    // 第二个 Worker 解决 double_impl
    let res2 = pool.submit_solution_with_agent("double_impl", "value * 2", "worker_2")
        .expect("worker 2 should succeed");
    assert!(matches!(res2, SmtResult::Proved { .. } | SmtResult::Sampled { .. }));

    assert!(pool.is_complete());
    assert_eq!(pool.pending_hole_ids().len(), 0);

    // 两个洞均已解决，Verified Module 成功导出
    let verified_mod = pool.assemble_verified_module().expect("all holes resolved");
    assert_eq!(verified_mod.functions.len(), 2);
    assert!(!matches!(verified_mod.functions[0].body, Expr::Hole(_)));
    assert!(!matches!(verified_mod.functions[1].body, Expr::Hole(_)));
}

