use lkjscript_core::{BudgetLedger, ResourceCategory, ResourceProfile};

use super::*;
use crate::semantic::schema::ResponseResult;

#[test]
fn one_shot_typed_budget_prefix_is_deterministic_and_keeps_prior_events() {
    let directory = case_dir("ledger-prefix");
    let root = directory.join("main.lkjscript");
    std::fs::write(
        &root,
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n",
    )
    .expect("write source");
    let input = request(&root, "{\"kind\":\"snapshot\"}");

    let mut measuring = BudgetLedger::default();
    let measured =
        crate::semantic::execute_with_ledger(&input, &mut measuring).expect("measure snapshot");
    let decoded = response(measured.response());
    assert!(matches!(decoded.result, ResponseResult::Snapshot { .. }));
    let nodes = decoded.charges.source_nodes;

    let exact = ResourceProfile::default()
        .lowered(ResourceCategory::SchemaNodes, nodes)
        .expect("lower exact node ceiling");
    let mut exact_ledger = BudgetLedger::new(exact);
    let exact_result =
        crate::semantic::execute_with_ledger(&input, &mut exact_ledger).expect("exact node budget");
    assert!(matches!(
        response(exact_result.response()).result,
        ResponseResult::Snapshot { .. }
    ));

    let rejected = || {
        let profile = ResourceProfile::default()
            .lowered(ResourceCategory::SchemaNodes, nodes - 1)
            .expect("lower rejecting node ceiling");
        let mut ledger = BudgetLedger::new(profile);
        let execution = crate::semantic::execute_with_ledger(&input, &mut ledger)
            .expect("encode bounded budget response");
        let failure = *execution.budget_error().expect("typed budget error");
        (failure, ledger.prefix(), execution.into_response())
    };
    let first = rejected();
    let second = rejected();
    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.2, second.2);
    assert_eq!(
        first.0.prefix().reservations(),
        &first.1.reservations()[..first.0.prefix().reservation_count()]
    );
    assert!(first.1.committed(ResourceCategory::ProtocolRequestBytes) > 0);
    assert!(first.1.reservation_count() > 1);
}
