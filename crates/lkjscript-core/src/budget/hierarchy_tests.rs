#![allow(clippy::unwrap_used)]

use super::*;

const CATEGORY: ResourceCategory = ResourceCategory::StagedPublicationBytes;
const CAUSE: BudgetCause = BudgetCause::SemanticNode(7);

#[test]
fn reservation_exact_and_plus_one_are_prechecked() {
    let profile = ResourceProfile::default().lowered(CATEGORY, 5).unwrap();
    let mut ledger = BudgetLedger::new(profile);
    {
        let mut scope = ledger.scope(BudgetAuthority::CompileRequest);
        let reservation = scope.reserve(CATEGORY, 5, CAUSE).unwrap();
        assert_eq!(reservation.amount(), 5);
        reservation.commit();
    }
    assert_eq!(ledger.used(CATEGORY), 5);
    let error = ledger
        .scope(BudgetAuthority::CompileRequest)
        .reserve(CATEGORY, 1, CAUSE)
        .unwrap_err();
    assert_eq!(error.kind, BudgetErrorKind::LimitExceeded);
    assert_eq!(error.limit, 0);
    assert_eq!(error.attempted, 1);
    assert!(!error.allocated_before_rejection);
}

#[test]
fn child_grants_cannot_oversubscribe_parent() {
    let mut ledger = BudgetLedger::default();
    let mut parent = ledger.scope(BudgetAuthority::SemanticRequest);
    parent.lower_grant(CATEGORY, 5).unwrap();
    {
        let mut first = parent.child(BudgetAuthority::ProtocolDecode).unwrap();
        first.reserve(CATEGORY, 4, CAUSE).unwrap().commit();
    }
    let mut second = parent.child(BudgetAuthority::ProtocolEncode).unwrap();
    assert_eq!(second.grant(CATEGORY), 1);
    let error = second.reserve(CATEGORY, 2, CAUSE).unwrap_err();
    assert_eq!(error.limit, 1);
    assert_eq!(error.observed, 0);
    assert_eq!(error.reserved, 0);
}

#[test]
fn reservation_consume_return_and_drop_are_conservative() {
    let mut ledger = BudgetLedger::default();
    {
        let mut scope = ledger.scope(BudgetAuthority::ArtifactBuild);
        let mut returned = scope.reserve(CATEGORY, 5, CAUSE).unwrap();
        returned.consume(2).unwrap();
        assert_eq!(returned.remaining(), 3);
        returned.return_unused();
        assert_eq!(scope.observed(CATEGORY), 2);

        let dropped = scope.reserve(CATEGORY, 3, CAUSE).unwrap();
        assert_eq!(dropped.state(), ReservationState::Active);
        drop(dropped);
        assert_eq!(scope.observed(CATEGORY), 5);
        assert_eq!(scope.reserved(CATEGORY), 0);
    }
    assert_eq!(ledger.used(CATEGORY), 5);
}

fn exceed_depth(scope: &mut BudgetScope<'_>, children: usize) -> Option<BudgetError> {
    if children == 0 {
        return scope.child(BudgetAuthority::Parsing).err();
    }
    let mut child = scope.child(BudgetAuthority::Parsing).unwrap();
    exceed_depth(&mut child, children - 1)
}

#[test]
fn authority_path_is_fixed_and_depth_bounded() {
    let mut ledger = BudgetLedger::default();
    let mut root = ledger.scope(BudgetAuthority::CompileRequest);
    let error = exceed_depth(&mut root, MAX_BUDGET_PATH_DEPTH - 1).unwrap();
    assert_eq!(error.kind, BudgetErrorKind::PathTooDeep);
    assert_eq!(error.path.len(), MAX_BUDGET_PATH_DEPTH);
    assert_eq!(error.authority, Some(BudgetAuthority::Parsing));
}

#[test]
fn diagnostics_are_complete_and_deterministic() {
    let profile = ResourceProfile::new(crate::ResourceProfileName::Deterministic)
        .lowered(CATEGORY, 4)
        .unwrap();
    let run = || {
        let mut ledger = BudgetLedger::new(profile);
        let mut scope = ledger.scope(BudgetAuthority::SemanticRequest);
        let mut child = scope.child(BudgetAuthority::ProtocolEncode).unwrap();
        child.reserve(CATEGORY, 3, CAUSE).unwrap().commit();
        child.reserve(CATEGORY, 2, CAUSE).unwrap_err()
    };
    let first = run();
    let second = run();
    assert_eq!(first, second);
    assert_eq!(
        first.profile.contract,
        lkjscript_contracts::RESOURCE_PROFILES_DIGEST
    );
    assert_eq!(first.category, CATEGORY);
    assert_eq!(first.authority, Some(BudgetAuthority::ProtocolEncode));
    assert_eq!(first.path.to_string(), "semantic_request/protocol_encode");
    assert_eq!(first.limit, 4);
    assert_eq!(first.reserved, 0);
    assert_eq!(first.attempted, 2);
    assert_eq!(first.observed, 3);
    assert_eq!(first.to_string(), second.to_string());
}

#[test]
fn missing_authority_fails_closed_without_charge() {
    let mut ledger = BudgetLedger::default();
    let error = ledger
        .charge_with_authority(None, CATEGORY, 1, CAUSE)
        .unwrap_err();
    assert_eq!(error.kind, BudgetErrorKind::MissingAuthority);
    assert_eq!(error.authority, None);
    assert!(error.path.is_empty());
    assert_eq!(ledger.used(CATEGORY), 0);
}
