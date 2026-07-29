#![allow(clippy::expect_used)]

use super::*;
use crate::{HeapObj, OwnedValue, ResourceKind, Value};

fn round_trip(outcome: ExecutionOutcome) {
    let bytes = encode_execution_outcome(&outcome, 64 * 1024).expect("encode outcome");
    let decoded = decode_execution_outcome(&bytes, 64 * 1024).expect("decode outcome");
    assert_eq!(decoded, outcome);
}

#[test]
fn process_outcome_codec_preserves_closed_outcomes() {
    round_trip(ExecutionOutcome::Returned(
        OwnedValue::from_vm_snapshot(
            Value::from_legacy_traced(0),
            vec![Some(HeapObj::Str("cell-result".into()))],
        )
        .expect("owned string"),
    ));
    round_trip(ExecutionOutcome::Returned(
        OwnedValue::from_unique_byte_vector(vec![0, 1, 255]).expect("owned bytes"),
    ));
    round_trip(ExecutionOutcome::Exited(-7));
    round_trip(ExecutionOutcome::Trapped(Trap::new("cell trap")));
    round_trip(ExecutionOutcome::DeadlineExceeded);
    round_trip(ExecutionOutcome::ResourceLimitExceeded(
        ResourceLimitKind::LogicalAggregateConstructions,
    ));
    round_trip(ExecutionOutcome::HostFailure(HostError::new(
        "provider failed",
    )));
}

#[test]
fn process_outcome_codec_preserves_cleanup_accounting() {
    let limits = CleanupFailureLimits::new(3, 8).expect("limits");
    let mut failures = CleanupFailures::new(limits);
    failures.push(
        CleanupPhase::Emergency,
        CleanupSubject::Resource(ResourceKind::FileWriter),
        "long failure text",
    );
    failures.push(
        CleanupPhase::RuntimeTeardown,
        CleanupSubject::StandardOutput,
        "more",
    );
    failures.push(
        CleanupPhase::Ordinary,
        CleanupSubject::EvaluatorProvider,
        "third",
    );
    failures.push(
        CleanupPhase::Ordinary,
        CleanupSubject::UniqueStorage,
        "omitted",
    );
    round_trip(ExecutionOutcome::CleanupFailed {
        primary: Box::new(ExecutionOutcome::HostFailure(HostError::new("primary"))),
        failures,
    });
}

#[test]
fn process_outcome_codec_rejects_malformed_or_oversized_values() {
    let bytes = encode_execution_outcome(&ExecutionOutcome::Exited(0), 64).expect("encode");
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(decode_execution_outcome(&trailing, 64).is_err());
    assert!(decode_execution_outcome(&bytes, 1).is_err());
    assert!(decode_execution_outcome(&[99], 64).is_err());
}
