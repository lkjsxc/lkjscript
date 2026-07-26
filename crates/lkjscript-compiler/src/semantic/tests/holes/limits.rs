use super::*;
use lkjscript_core::{
    BudgetAuthority, BudgetCause, BudgetLedger, ResourceCategory, ResourceProfile,
};

#[test]
fn hole_candidate_and_legal_action_bounds_reject_limit_plus_one_before_allocation() {
    for (category, authority) in [
        (ResourceCategory::HoleCandidates, BudgetAuthority::Holes),
        (ResourceCategory::HoleSearchWork, BudgetAuthority::Holes),
        (ResourceCategory::LegalActions, BudgetAuthority::Holes),
        (
            ResourceCategory::TransactionOperations,
            BudgetAuthority::Transaction,
        ),
    ] {
        let profile = ResourceProfile::default()
            .lowered(category, 3)
            .expect("lower profile");
        let mut ledger = BudgetLedger::new(profile);
        ledger
            .scope(authority)
            .reserve(category, 3, BudgetCause::Request)
            .expect("exact reservation")
            .commit();
        let failure = ledger
            .scope(authority)
            .reserve(category, 1, BudgetCause::Request)
            .expect_err("limit plus one");
        assert!(!failure.allocated_before_rejection);
        assert_eq!(failure.category, category);
    }
}

#[test]
fn strict_contract_rejects_stale_digest_unknown_fields_and_duplicate_identities() {
    let directory = case_dir("hole-closed");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, function_source(&hole("body", None))).expect("write source");
    let valid =
        String::from_utf8(request(&root, "{\"kind\":\"snapshot\"}")).expect("request UTF-8");
    assert!(crate::semantic::execute(
        valid
            .replace(&crate::semantic::CONTRACT.to_hex(), &"0".repeat(64))
            .as_bytes()
    )
    .is_err());
    let context = context(&root);
    let operation = format!(
        concat!(
            "{{\"kind\":\"hole_context\",\"revision\":{:?},",
            "\"node\":{},\"invented\":true}}",
        ),
        context.hole.source_revision, context.hole.node
    );
    assert!(crate::semantic::execute(&request(&root, &operation)).is_err());

    let duplicate = directory.join("duplicate.lkjscript");
    let body = format!("do/\n{}{}0\n/do\n", hole("same", None), hole("same", None));
    std::fs::write(&duplicate, function_source(&body)).expect("write duplicate holes");
    let encoded = crate::semantic::execute(&request(&duplicate, "{\"kind\":\"snapshot\"}"))
        .expect("typed duplicate response");
    assert!(
        matches!(response(&encoded).result, ResponseResult::Error { error, .. }
        if error.code == crate::semantic::schema::ProtocolErrorCode::ValidationFailed)
    );
}

#[test]
fn executable_compilation_rejects_every_reachable_hole() {
    let source = function_source(&hole("body", None)).to_string();
    let failure = crate::compile_source(
        &source,
        "hole-release.lkjscript",
        &lkjscript_core::Limits::default(),
    )
    .expect_err("release compilation must reject typed holes");
    assert!(failure.to_string().contains("unknown call hole"));
}
