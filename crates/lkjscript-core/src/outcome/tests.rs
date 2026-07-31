use super::*;
use crate::{ResourceKind, Value};

#[test]
fn owned_list_snapshots_reject_noncanonical_or_runtime_local_graphs() {
    let forward = vec![
        OwnedListNode {
            head: Value::UNIT,
            tail: Value::from_owned_list(1),
        },
        OwnedListNode {
            head: Value::UNIT,
            tail: Value::EMPTY_LIST,
        },
    ];
    assert!(OwnedValue::from_materialized_snapshot(Value::from_owned_list(0), forward).is_err());

    let unreachable = vec![
        OwnedListNode {
            head: Value::UNIT,
            tail: Value::EMPTY_LIST,
        },
        OwnedListNode {
            head: Value::UNIT,
            tail: Value::from_owned_list(0),
        },
    ];
    assert!(
        OwnedValue::from_materialized_snapshot(Value::from_owned_list(0), unreachable,).is_err()
    );

    let runtime_local = vec![OwnedListNode {
        head: Value::from_segmented_list(1),
        tail: Value::EMPTY_LIST,
    }];
    assert!(
        OwnedValue::from_materialized_snapshot(Value::from_owned_list(0), runtime_local,).is_err()
    );
}

#[test]
fn owned_snapshots_reject_list_cycles() {
    let self_cycle = vec![OwnedListNode {
        head: Value::from_owned_list(0),
        tail: Value::EMPTY_LIST,
    }];
    assert!(
        OwnedValue::from_materialized_snapshot(Value::from_owned_list(0), self_cycle,).is_err()
    );
}

#[test]
fn cleanup_failures_bound_records_bytes_and_utf8() {
    let limits = CleanupFailureLimits::new(2, 4).unwrap_or_else(|| std::process::abort());
    let mut failures = CleanupFailures::new(limits);
    failures.push(
        CleanupPhase::Emergency,
        CleanupSubject::Resource(ResourceKind::FileReader),
        "aébc",
    );
    failures.push(
        CleanupPhase::RuntimeTeardown,
        CleanupSubject::Terminal,
        "second",
    );
    failures.push(
        CleanupPhase::RuntimeTeardown,
        CleanupSubject::StandardOutput,
        "third",
    );

    assert_eq!(failures.retained().len(), 2);
    assert_eq!(failures.retained()[0].message(), "aéb");
    assert_eq!(failures.retained()[0].omitted_message_bytes(), 1);
    assert_eq!(failures.retained()[1].message(), "");
    assert_eq!(failures.retained()[1].omitted_message_bytes(), 6);
    assert_eq!(failures.retained_message_bytes(), 4);
    assert_eq!(failures.omitted_message_bytes(), 12);
    assert_eq!(failures.omitted_failures(), 1);
}

#[test]
fn cleanup_attachment_retains_primary_outcome() {
    let mut failures = CleanupFailures::new(CleanupFailureLimits::default());
    failures.push(
        CleanupPhase::Emergency,
        CleanupSubject::UniqueStorage,
        "release failed",
    );
    let outcome = ExecutionOutcome::Exited(7).with_cleanup_failures(failures.clone());

    assert_eq!(outcome.primary(), &ExecutionOutcome::Exited(7));
    assert_eq!(outcome.cleanup_failures(), Some(&failures));
    assert_eq!(outcome.returned(), None);
    assert!(outcome
        .summary()
        .starts_with("CleanupFailed(primary=Exited(7)"));
}

#[test]
fn zero_retention_still_records_omitted_failure() {
    let limits = CleanupFailureLimits::new(0, 0).unwrap_or_else(|| std::process::abort());
    let mut failures = CleanupFailures::new(limits);
    failures.push(
        CleanupPhase::Ordinary,
        CleanupSubject::EvaluatorProvider,
        "failure",
    );
    assert!(!failures.is_empty());
    assert_eq!(failures.retained(), []);
    assert_eq!(failures.omitted_failures(), 1);
    assert_eq!(failures.omitted_message_bytes(), 7);
}
