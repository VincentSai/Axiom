use axiom_agent_pool::auto_heal::{solve_and_heal_with_strategy, AutoHealOutcome};
use axiom_agent_pool::AgentTaskPool;

const ABS_HOLE_SKELETON: &str = r#"
    fn safe_abs(x: Int) -> Int
        requires { x > -1000000, x < 1000000 }
        ensures { result >= 0 }
    {
        ?abs_impl
    }
"#;

#[test]
fn test_auto_heal_recovers_from_counterexample_in_single_step() {
    let mut pool = AgentTaskPool::from_source(ABS_HOLE_SKELETON).expect("skeleton should parse");

    // 模拟一个双步 Worker 策略：
    // 第 0 步故意返回有漏洞的实现 `x` (当 x < 0 时触发 SMT 反例)
    // 第 1 步在接收到反例提示后，返回正确的分支实现 `match x < 0 { true => -x, false => x }`
    let strategy = |_step: usize, counter_example: Option<&str>| -> String {
        if let Some(ce) = counter_example {
            assert!(ce.contains("x"));
            // 第二步：根据反例调整为具有分支防护的正确解
            "match x < 0 { true => -x, false => x }".to_string()
        } else {
            // 第一步：初次尝试（幼稚实现）
            "x".to_string()
        }
    };

    let outcome: AutoHealOutcome = solve_and_heal_with_strategy(&mut pool, "abs_impl", 3, strategy)
        .expect("auto-heal workflow should execute cleanly");

    assert!(outcome.success);
    assert_eq!(outcome.total_steps, 2);
    assert_eq!(outcome.history.len(), 2);
    assert!(outcome.history[0].counter_example.is_some());
    assert!(outcome.history[1].counter_example.is_none());
    assert_eq!(
        outcome.final_solution.as_deref(),
        Some("match x < 0 { true => -x, false => x }")
    );

    // 验证 pool 状态已被签入并可以组装
    assert!(pool.is_complete());
    let verified_mod = pool
        .assemble_verified_module()
        .expect("verified module assembly");
    assert_eq!(verified_mod.functions.len(), 1);
}

#[test]
fn test_auto_heal_aborts_when_max_steps_exceeded() {
    let mut pool = AgentTaskPool::from_source(ABS_HOLE_SKELETON).expect("skeleton should parse");

    // 模拟持续产生反例的策略
    let stubborn_strategy = |_step: usize, _ce: Option<&str>| -> String { "x".to_string() };

    let outcome = solve_and_heal_with_strategy(&mut pool, "abs_impl", 2, stubborn_strategy)
        .expect("execution terminates with outcome");

    assert!(!outcome.success);
    assert_eq!(outcome.total_steps, 2);
    assert!(outcome.final_solution.is_none());
    assert!(!pool.is_complete());
}

#[test]
fn test_codex_cli_watchdog_timeout_kills_process_and_fails_closed() {
    use axiom_agent_pool::auto_heal::call_codex_cli_with_timeout;
    use std::path::Path;

    let fake_bin_path = env!("CARGO_BIN_EXE_fake_codex_hang");
    let hang_bin = Path::new(fake_bin_path);

    let start = std::time::Instant::now();
    let res = call_codex_cli_with_timeout("some prompt", hang_bin, 150);
    let elapsed = start.elapsed();

    assert!(
        res.is_err(),
        "Watchdog MUST kill hung process and return error!"
    );
    let err_msg = res.unwrap_err();
    assert!(
        err_msg.contains("timed out"),
        "Error message MUST strictly indicate timeout failure, but got: {err_msg}"
    );
    assert!(
        elapsed >= std::time::Duration::from_millis(100),
        "Watchdog MUST wait at least until the timeout deadline! Elapsed: {:?}",
        elapsed
    );
    assert!(
        elapsed < std::time::Duration::from_millis(3000),
        "Watchdog MUST terminate process promptly without hanging! Elapsed: {:?}",
        elapsed
    );
}
