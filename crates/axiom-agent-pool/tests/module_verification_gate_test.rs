use axiom_agent_pool::{SubmitStatus, VerifiedWorkspace};
use axiom_db::{current_verification_profile_hash, ProofLevel};
use std::str::FromStr;

#[test]
fn test_zero_hole_faulty_module_is_rejected_by_workspace_and_cannot_publish() {
    let source = r#"
        fn bad() -> Int
            ensures { result == 1 }
        {
            0
        }
    "#;

    let ws = VerifiedWorkspace::from_source(source).expect("faulty syntax is valid Axiom");

    // 零洞错误模块：虽然没有任何未完成洞任务，但全模块契约未经验证或存在反例
    assert!(
        !ws.is_fully_verified(),
        "Zero-hole module containing contract violation MUST NOT be marked fully verified!"
    );

    let assemble_res = ws.assemble_verified_module();
    assert!(
        assemble_res.is_err(),
        "assemble_verified_module MUST fail-closed on module with contract violations!"
    );
}

#[test]
fn test_module_with_both_valid_hole_and_unrelated_faulty_function_cannot_publish() {
    let source = r#"
        fn good(x: Int) -> Int
            requires { x >= 0 }
            ensures { result == x }
        {
            ?hole_good
        }

        fn bad() -> Int
            ensures { result == 1 }
        {
            0
        }
    "#;

    let mut ws = VerifiedWorkspace::from_source(source).expect("valid syntax");
    assert!(!ws.is_fully_verified());

    // 填好正常函数
    let outcome = ws
        .submit_source("hole_good", "x")
        .expect("submission succeeds");
    assert_eq!(outcome.status, SubmitStatus::Accepted);
    assert!(outcome.proved);

    // 此时虽然所有洞已填完，但由于无洞函数 bad 违背契约，整个模块不能算完全验证
    assert!(
        ws.holes_complete(),
        "All holes are filled, holes_complete must be true"
    );
    assert!(
        !ws.is_fully_verified(),
        "Module with faulty non-hole function MUST NOT be marked fully verified even if all holes are solved!"
    );

    let assemble_res = ws.assemble_verified_module();
    assert!(
        assemble_res.is_err(),
        "assemble_verified_module MUST reject release due to contract violation in bad()!"
    );
}

#[test]
fn test_profile_v5_invalidates_v4_receipts() {
    use axiom_db::{compute_hash, NodeHash, VerificationReceipt};

    let v4_profile = "axiom-smt-v4-sha256sealed-truncdiv-residualmatch";
    let v4_hash = compute_hash("vprof", &v4_profile);
    let v5_hash = current_verification_profile_hash();

    assert_ne!(
        v4_hash, v5_hash,
        "Profile v5 hash must differ from v4 hash!"
    );

    let valid_sha256 =
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();
    let old_v4_receipt = VerificationReceipt {
        candidate_hash: NodeHash::from_str(
            "#cand_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap(),
        proof_level: ProofLevel::Proved,
        solver: "Microsoft Z3".to_string(),
        solver_version: "4.13.0".to_string(),
        solver_sha256: Some(valid_sha256),
        obligations_checked: 2,
        proof_obligation_hash: NodeHash::from_str(
            "#obl_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        )
        .unwrap(),
        verification_profile_hash: v4_hash,
        attested_commit_hash: None,
        verified_at: 1000,
    };

    // 在当前最新系统下，v4 旧回执在当前 v5 profile 下必须被判定为无效
    assert!(
        !old_v4_receipt.is_currently_valid_proved(&v5_hash),
        "Old v4 verification receipt must be invalidated under current v5 profile!"
    );
}
