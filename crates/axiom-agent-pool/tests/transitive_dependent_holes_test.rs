use axiom_agent_pool::{AgentTaskPool, CasEvent};
use axiom_parser::parse_module;

const STALE_REBASE_SKELETON: &str = r#"
fn independent(x: Int) -> Int
    requires { x >= 0, x <= 100 }
    ensures { result == x + 1 }
{
    ?independent_impl
}

fn helper(x: Int) -> Int
    requires { x >= 0, x <= 100 }
    ensures { result == x * 2 }
{
    ?helper_impl
}

fn caller(x: Int) -> Int
    requires { x >= 0, x <= 100 }
    ensures { result == helper(x) + 1 }
{
    ?caller_impl
}
"#;

#[test]
fn test_concurrent_workers_stale_root_rebase_with_pending_downstream_holes() {
    let temp_dir = std::env::temp_dir().join(format!(
        "axiom_stale_rebase_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    let db_path = temp_dir.join("astdb.json");

    let skeleton = parse_module(STALE_REBASE_SKELETON).unwrap();
    let mut pool1 = AgentTaskPool::open_with_database(
        skeleton.clone(),
        axiom_db::AstDatabase::open(&db_path).unwrap(),
    )
    .unwrap();

    let mut pool2 =
        AgentTaskPool::open_with_database(skeleton, axiom_db::AstDatabase::open(&db_path).unwrap())
            .unwrap();

    // 1. Worker 1 提交 independent_impl: x + 1，写入数据库主干
    let outcome1 = pool1
        .submit_solution_with_outcome("independent_impl", "x + 1", "worker_1")
        .unwrap();
    assert!(outcome1.smt_result.is_proved());

    // 2. Worker 2 提交 helper_impl: x * 2，此时基线已经过期（StaleRoot）
    // 且下游 caller_impl 仍然有洞（in-flight pending hole）
    // 旧版中因 !cascade_report.is_all_proved() 导致误判级联失败被拒绝
    // 统一为 has_failures() 后，重基必须成功接受！
    let outcome2 = pool2
        .submit_solution_with_outcome("helper_impl", "x * 2", "worker_2")
        .unwrap();

    assert!(
        outcome2.smt_result.is_proved(),
        "Rebased candidate must be formally proved: {:?}",
        outcome2.smt_result
    );
    assert!(
        outcome2
            .cas_events
            .iter()
            .any(|e| matches!(e, CasEvent::StaleRootDetected { .. })),
        "StaleRootDetected must be recorded in cas_events: {:?}",
        outcome2.cas_events
    );
    assert!(
        outcome2
            .cas_events
            .iter()
            .any(|e| matches!(e, CasEvent::Ast3WayRebaseSucceeded { .. })),
        "Ast3WayRebaseSucceeded must be recorded in cas_events: {:?}",
        outcome2.cas_events
    );
    assert!(
        !outcome2
            .cas_events
            .iter()
            .any(|e| matches!(e, CasEvent::CascadeVerificationFailed { .. })),
        "Rebase must NOT fail on pending downstream holes: {:?}",
        outcome2.cas_events
    );

    let _ = std::fs::remove_dir_all(temp_dir);
}

const THREE_TIER_TRANSITIVE_SKELETON: &str = r#"
fn helper(x: Int) -> Int
    requires { x >= 0, x <= 100 }
    ensures { result == x * 2 }
{
    ?helper_impl
}

fn intermediate(x: Int) -> Int
    requires { x >= 0, x <= 100 }
    ensures { result == helper(x) }
{
    ?intermediate_impl
}

fn caller(x: Int) -> Int
    requires { x >= 0, x <= 100 }
    ensures { result == helper(x) + 1 }
{
    intermediate(x) + 1
}
"#;

#[test]
fn test_transitive_in_flight_holes_do_not_falsely_block_upstream_commit() {
    let mut pool =
        AgentTaskPool::from_source(THREE_TIER_TRANSITIVE_SKELETON).expect("skeleton should parse");

    // helper 填洞；intermediate 仍有洞；caller 本身无洞但依赖 intermediate。
    // 在旧版中，caller 因为自身无洞而强行被 SMT 证明，但 intermediate 展开有洞返回 Skipped，
    // 导致 caller 被误判为破坏失败（CascadeFailure），阻塞 helper 提交！
    // 在新版中，pending_closure 沿依赖链向上传播，caller 正确被识别为待决依赖（pending_dependents）！
    let outcome = pool
        .submit_solution_with_outcome("helper_impl", "x * 2", "worker_helper")
        .expect("submit helper should succeed");

    assert!(
        outcome.smt_result.is_proved(),
        "helper implementation must be proved: {:?}",
        outcome.smt_result
    );
    assert!(
        !outcome
            .cas_events
            .iter()
            .any(|e| matches!(e, CasEvent::CascadeVerificationFailed { .. })),
        "transitive caller must not cause cascade verification failure when intermediate has holes: {:?}",
        outcome.cas_events
    );
}
