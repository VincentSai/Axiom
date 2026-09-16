use axiom_agent_pool::{SubmitStatus, VerifiedWorkspace};

const DEPENDENT_SOURCE: &str = r#"
fn helper(x: Int) -> Int
    requires { x >= 0 }
    ensures { result >= 0 }
{
    ?hole_helper
}

fn caller(value: Int) -> Int
    requires { value >= 0 }
    ensures { result >= 0 }
{
    let h = helper(value);
    ?hole_caller
}
"#;

#[test]
fn test_dependent_holes_progressive_submission_unblocks_deadlock() {
    let mut ws = VerifiedWorkspace::from_source(DEPENDENT_SOURCE).expect("syntax is valid");

    assert_eq!(ws.pending_holes().len(), 2);
    assert!(!ws.is_fully_verified());

    // 1. 先提交 helper：正确解为 x
    // 此时 caller 中仍包含 ?hole_caller，下游尚未完工。
    // 级联验证机制必须识别出 caller 为在途依赖，绝不能产生假级联失败而死锁拒绝！
    let outcome1 = ws
        .submit_source("hole_helper", "x")
        .expect("helper submission must be accepted progressively without deadlock!");
    assert_eq!(
        outcome1.status,
        SubmitStatus::Accepted,
        "helper must be accepted even when caller still has a hole"
    );

    // 2. 此时 helper 解决，仅剩 hole_caller
    assert_eq!(ws.pending_holes(), vec!["hole_caller".to_string()]);
    assert!(!ws.is_fully_verified());

    // 3. 再提交 caller：正确解为 h
    let outcome2 = ws
        .submit_source("hole_caller", "h")
        .expect("caller submission must succeed");
    assert_eq!(outcome2.status, SubmitStatus::Accepted);
    assert!(outcome2.proved);

    // 4. 所有洞填完，模块完全验证通过并可成功发布
    assert!(ws.holes_complete());
    assert!(ws.is_fully_verified());

    let verified_mod = ws
        .assemble_verified_module()
        .expect("assembly must succeed when all dependent holes are proven");
    assert_eq!(verified_mod.functions.len(), 2);
    assert!(!verified_mod
        .functions
        .iter()
        .any(|f| axiom_ast::expr_contains_hole(&f.body)));
}

#[test]
fn test_upstream_change_breaking_downstream_must_be_rejected() {
    let source = r#"
        fn helper(x: Int) -> Int
            requires { x >= 0 }
            ensures { result >= -100 }
        {
            ?hole_helper
        }

        fn caller(value: Int) -> Int
            requires { value >= 0 }
            ensures { result >= 0 }
        {
            helper(value)
        }
    "#;

    let mut ws = VerifiedWorkspace::from_source(source).expect("valid syntax");
    // caller 无洞，其要求 result >= 0；但 helper 只保证 result >= -100。
    // 若提交 helper 返回 -50，caller 的后置条件即被破坏。
    let outcome = ws.submit_source("hole_helper", "-50");

    // 级联验证必须拦截此破坏行为，或者 assemble 必须坚决拒绝发布
    if let Ok(out) = outcome {
        if out.status == SubmitStatus::Accepted {
            assert!(
                ws.assemble_verified_module().is_err(),
                "assemble_verified_module MUST reject release when helper breaks caller contract!"
            );
        } else {
            assert_eq!(out.status, SubmitStatus::Rejected);
        }
    }
}
