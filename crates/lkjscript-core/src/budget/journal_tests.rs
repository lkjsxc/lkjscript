#![allow(clippy::unwrap_used)]

use super::*;

const CATEGORY: ResourceCategory = ResourceCategory::StagedPublicationNodes;
const CAUSE: BudgetCause = BudgetCause::SemanticNode(41);

#[test]
fn journal_prefix_is_equal_across_identical_runs() {
    let run = || {
        let mut ledger = BudgetLedger::default();
        let mut scope = ledger.scope(BudgetAuthority::CompileRequest);
        scope.reserve(CATEGORY, 2, CAUSE).unwrap().commit();
        scope.reserve(CATEGORY, u64::MAX, CAUSE).unwrap_err()
    };
    let first = run();
    let second = run();
    assert_eq!(first, second);
    assert_eq!(first.prefix(), second.prefix());
    assert_eq!(first.prefix().profile(), first.profile);
    assert_eq!(first.prefix().rejected().unwrap().attempted, u64::MAX);
}

#[test]
fn nested_prefix_contains_ancestors_siblings_and_current_work_once() {
    let mut ledger = BudgetLedger::default();
    let error = {
        let mut root = ledger.scope(BudgetAuthority::SemanticRequest);
        root.reserve(CATEGORY, 1, BudgetCause::Request)
            .unwrap()
            .commit();
        {
            let mut sibling = root.child(BudgetAuthority::ProtocolDecode).unwrap();
            let mut reservation = sibling.reserve(CATEGORY, 2, CAUSE).unwrap();
            reservation.consume(1).unwrap();
            reservation.return_unused();
        }
        let mut current = root.child(BudgetAuthority::ProtocolEncode).unwrap();
        drop(current.reserve(CATEGORY, 3, CAUSE).unwrap());
        current.reserve(CATEGORY, u64::MAX, CAUSE).unwrap_err()
    };
    let prefix = error.prefix();
    assert_eq!(prefix.reservation_count(), 3);
    assert_eq!(prefix.committed(CATEGORY), 5);
    let records = prefix.reservations();
    assert_eq!(records[0].id.get(), 1);
    assert_eq!(records[0].owner.len(), 1);
    assert_eq!((records[0].amount, records[0].consumed), (1, 1));
    assert_eq!(
        records[1].owner.entries()[1],
        BudgetAuthority::ProtocolDecode
    );
    assert_eq!((records[1].consumed, records[1].returned), (1, 1));
    assert!(!records[1].conservative_drop);
    assert_eq!(
        records[2].owner.entries()[1],
        BudgetAuthority::ProtocolEncode
    );
    assert_eq!((records[2].amount, records[2].consumed), (3, 3));
    assert!(records[2].conservative_drop);
    let rejected = prefix.rejected().unwrap();
    assert_eq!(rejected.kind, BudgetErrorKind::LimitExceeded);
    assert_eq!(rejected.path, records[2].owner);
    assert_eq!(rejected.cause, CAUSE);
    assert!(!rejected.allocated_before_rejection);
}

#[test]
fn exact_journal_capacity_succeeds_and_plus_one_is_prechecked() {
    let mut ledger = BudgetLedger::default();
    {
        let mut scope = ledger.scope(BudgetAuthority::CompileRequest);
        for expected in 1..=MAX_BUDGET_JOURNAL_ENTRIES {
            let reservation = scope.reserve(CATEGORY, 0, CAUSE).unwrap();
            assert_eq!(reservation.id().get(), expected as u64);
            reservation.commit();
        }
        let before = scope.prefix();
        let next_before = *scope.next_reservation;
        let error = scope.reserve(CATEGORY, 0, CAUSE).unwrap_err();
        assert_eq!(error.kind, BudgetErrorKind::JournalExhausted);
        assert_eq!(error.prefix().reservations(), before.reservations());
        assert_eq!(*scope.next_reservation, next_before);
        assert_eq!(scope.reserved(CATEGORY), 0);
        assert!(!error.allocated_before_rejection);
    }
    assert_eq!(
        ledger.prefix().reservation_count(),
        MAX_BUDGET_JOURNAL_ENTRIES
    );
}

#[test]
fn missing_authority_owns_existing_prefix_without_mutation() {
    let mut ledger = BudgetLedger::default();
    ledger
        .charge_with_authority(
            Some(BudgetAuthority::Hir),
            CATEGORY,
            2,
            BudgetCause::Request,
        )
        .unwrap();
    let before = ledger.clone();
    let error = ledger
        .charge_with_authority(None, CATEGORY, 1, CAUSE)
        .unwrap_err();
    assert_eq!(ledger, before);
    assert_eq!(error.kind, BudgetErrorKind::MissingAuthority);
    assert_eq!(error.prefix().reservation_count(), 1);
    assert_eq!(error.prefix().committed(CATEGORY), 2);
    assert!(error.prefix().rejected().unwrap().path.is_empty());
}

#[test]
fn reservation_id_and_consume_overrun_reject_without_mutation() {
    let mut exhausted = BudgetLedger::default();
    exhausted.next_reservation = u64::MAX;
    {
        let mut scope = exhausted.scope(BudgetAuthority::ArtifactBuild);
        let before = scope.prefix();
        let error = scope.reserve(CATEGORY, 1, CAUSE).unwrap_err();
        assert_eq!(error.kind, BudgetErrorKind::ReservationIdExhausted);
        assert_eq!(scope.prefix(), before);
        assert_eq!(scope.reserved(CATEGORY), 0);
    }

    let mut ledger = BudgetLedger::default();
    let mut scope = ledger.scope(BudgetAuthority::ArtifactBuild);
    let mut reservation = scope.reserve(CATEGORY, 3, CAUSE).unwrap();
    reservation.consume(1).unwrap();
    let before = reservation.prefix();
    let remaining = reservation.remaining();
    let error = reservation.consume(3).unwrap_err();
    assert_eq!(error.kind, BudgetErrorKind::ReservationExceeded);
    assert_eq!(error.prefix().reservations(), before.reservations());
    assert_eq!(reservation.prefix(), before);
    assert_eq!(reservation.remaining(), remaining);
    reservation.return_unused();
}

#[test]
fn forgotten_reservation_is_conservatively_recorded_by_scope() {
    let mut ledger = BudgetLedger::default();
    {
        let mut scope = ledger.scope(BudgetAuthority::ArtifactBuild);
        let reservation = scope.reserve(CATEGORY, 4, CAUSE).unwrap();
        std::mem::forget(reservation);
    }
    let prefix = ledger.prefix();
    assert_eq!(prefix.committed(CATEGORY), 4);
    assert_eq!(prefix.reservations()[0].consumed, 4);
    assert!(prefix.reservations()[0].conservative_drop);
}

#[test]
fn invalid_grant_rejects_without_mutating_scope_or_journal() {
    let mut ledger = BudgetLedger::default();
    let mut scope = ledger.scope(BudgetAuthority::CompileRequest);
    let grant = scope.grant(CATEGORY);
    let prefix = scope.prefix();
    let error = scope.lower_grant(CATEGORY, grant + 1).unwrap_err();
    assert_eq!(error.kind, BudgetErrorKind::GrantExceedsParent);
    assert_eq!(scope.grant(CATEGORY), grant);
    assert_eq!(scope.prefix(), prefix);
    assert!(!error.allocated_before_rejection);
}
