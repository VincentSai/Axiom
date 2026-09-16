use axiom_agent_pool::{SubmitStatus, VerifiedWorkspace};

const SKELETON: &str = r#"
fn safe_add(a: Int, b: Int) -> Int
    requires { a >= 0, b >= 0, a <= 10000, b <= 10000 }
    ensures { result >= 0 }
{
    ?hole_add
}

fn safe_inc(x: Int) -> Int
    requires { x >= 0, x <= 10000 }
    ensures { result >= 1 }
{
    safe_add(x, 1)
}
"#;

#[test]
fn verified_workspace_lifecycle_and_deep_encapsulation() {
    let mut ws = VerifiedWorkspace::from_source(SKELETON).expect("skeleton should parse");

    // 1. Initial state: not fully verified, holes remain
    assert!(!ws.is_fully_verified());
    assert_eq!(ws.pending_holes(), vec!["hole_add".to_string()]);
    assert!(ws.task_for_hole("hole_add").is_some());
    assert!(ws.envelope_for_hole("hole_add").is_some());

    // 2. Dispatch tasks: only isolated envelopes exposed
    let tasks = ws.dispatch_tasks();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].target_hole_id, "hole_add");

    // 3. Assemble fails before verification completion
    let assemble_err = ws.assemble_verified_module();
    assert!(
        assemble_err.is_err(),
        "assemble_verified_module MUST fail when pending holes remain!"
    );

    // 4. Submit honest solution via high-level API
    let outcome = ws
        .submit_source("hole_add", "a + b")
        .expect("submission should succeed");
    assert_eq!(outcome.status, SubmitStatus::Accepted);
    assert!(outcome.proved);

    // 5. After verification: fully verified, assemble yields zero-hole verified module
    assert!(ws.is_fully_verified());
    assert!(ws.pending_holes().is_empty());

    let final_module = ws
        .assemble_verified_module()
        .expect("assembly must succeed after proof");
    assert_eq!(final_module.functions.len(), 2);
    assert!(!final_module
        .functions
        .iter()
        .any(|f| axiom_ast::expr_contains_hole(&f.body)));
}
