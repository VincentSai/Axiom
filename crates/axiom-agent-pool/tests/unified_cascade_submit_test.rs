use axiom_agent_pool::{AgentTaskPool, CasEvent, PoolVerificationPolicy, SmtResult};

#[test]
fn test_ordinary_submit_catches_downstream_contract_violation_via_unified_cascade() {
    let skeleton = r#"
    fn helper(x: Int) -> Int
        requires { x > 100, x < 1000 }
        ensures { result >= 0 }
    {
        ?helper_hole
    }

    fn caller(x: Int) -> Int
        requires { x > 100, x < 1000 }
        ensures { result > 10 }
    {
        helper(x)
    }
    "#;

    let mut pool = AgentTaskPool::from_source(skeleton)
        .expect("skeleton should parse")
        .with_policy(PoolVerificationPolicy::StrictProved);

    // 1. Worker 提交作弊解 "0"：
    // 对 helper 自身：result = 0 满足 result >= 0（自身契约通过）
    // 但对 downstream caller 而言：helper(x) 返回 0 破坏了 caller 的 ensures { result > 10 }！
    // 统一提交内核在普通非冲突提交路径上强制执行级联验证，坚决拦截该提交！
    let bad_outcome = pool
        .submit_solution_with_outcome("helper_hole", "0", "worker_cheat")
        .expect("submission should not crash");

    // 断言：由于破坏下游 caller 契约，普通提交被级联验证拦截，并返回具体反例
    assert!(
        !bad_outcome.smt_result.is_proved(),
        "Ordinary submit MUST NOT succeed when it breaks downstream caller contracts! Got: {:?}",
        bad_outcome.smt_result
    );
    assert!(
        matches!(bad_outcome.smt_result, SmtResult::CounterExample(_)),
        "Representative SmtResult on cascade failure must be the counterexample! Got: {:?}",
        bad_outcome.smt_result
    );
    assert!(
        bad_outcome
            .cas_events
            .iter()
            .any(|e| matches!(e, CasEvent::CascadeVerificationFailed { .. })),
        "cas_events MUST record CascadeVerificationFailed on ordinary submit! Events: {:?}",
        bad_outcome.cas_events
    );
    assert_ne!(
        pool.hole_status("helper_hole"),
        Some(axiom_agent_pool::HoleStatus::Verified),
        "helper_hole must NOT be marked Verified after breaking downstream contract"
    );

    // 2. Worker 提交诚实解 "x"：
    // x > 100，所以 helper(x) > 100 > 10，完全满足 caller 的 ensures { result > 10 }
    let good_outcome = pool
        .submit_solution_with_outcome("helper_hole", "x", "worker_honest")
        .expect("good candidate should submit cleanly");

    assert!(
        good_outcome.smt_result.is_proved(),
        "Honest solution satisfying both helper and caller must be proved! Got: {:?}",
        good_outcome.smt_result
    );
    assert_eq!(
        pool.hole_status("helper_hole"),
        Some(axiom_agent_pool::HoleStatus::Verified)
    );
    assert!(pool.is_complete());
}
