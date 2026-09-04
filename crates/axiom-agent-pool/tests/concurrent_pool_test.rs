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

