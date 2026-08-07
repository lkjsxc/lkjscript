#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use super::*;
use crate::{
    LayoutIdentity, OwnedValue, ResourceKind, SemanticPayload, SemanticTypeIdentity, SemanticValue,
    StructuralKind, StructuralSnapshotLimits, StructuralType, Value,
};

fn round_trip(outcome: ExecutionOutcome) {
    let bytes = encode_execution_outcome(&outcome, 64 * 1024).expect("encode outcome");
    let decoded = decode_execution_outcome(&bytes, 64 * 1024).expect("decode outcome");
    assert_eq!(decoded, outcome);
}

fn structural_value(kind: StructuralKind, payload: SemanticPayload) -> SemanticValue {
    SemanticValue::new(
        StructuralType::new(
            LayoutIdentity::new(NonZeroU64::new(1).expect("layout")),
            SemanticTypeIdentity::new(NonZeroU64::new(2).expect("semantic type")),
            kind,
        ),
        payload,
    )
}

#[test]
fn process_outcome_codec_preserves_closed_outcomes() {
    round_trip(ExecutionOutcome::Returned(
        OwnedValue::from_structural(
            structural_value(
                StructuralKind::String,
                SemanticPayload::String(b"cell-result".to_vec()),
            ),
            StructuralSnapshotLimits::DEFAULT,
        )
        .expect("owned structural string"),
    ));
    round_trip(ExecutionOutcome::Returned(
        OwnedValue::from_unique_byte_vector(vec![0, 1, 255]).expect("owned byte-vector"),
    ));
    round_trip(ExecutionOutcome::Returned(
        OwnedValue::from_unique_bytes(vec![255, 0, 1]).expect("owned immutable bytes"),
    ));
    round_trip(ExecutionOutcome::Exited(-7));
    round_trip(ExecutionOutcome::Trapped(Trap::new("cell trap")));
    round_trip(ExecutionOutcome::DeadlineExceeded);
    round_trip(ExecutionOutcome::ResourceLimitExceeded(
        ResourceLimitKind::Allocations,
    ));
    round_trip(ExecutionOutcome::HostFailure(HostError::new(
        "provider failed",
    )));
}

#[test]
fn process_outcome_codec_rejects_removed_value_tags_and_runtime_values() {
    for tag in [6, 7, 8, 10] {
        let error = decode_execution_outcome(&[0, 0, tag], 64).expect_err("removed value tag");
        assert_eq!(error.as_str(), "unknown value tag");
    }
    for value in [
        Value::from_capability(crate::CapabilityKind::Stdio),
        Value::from_resource(1),
        Value::from_function(1),
        Value::from_bytes_key(1),
    ] {
        let error = OwnedValue::from_value(value).expect_err("runtime value must reject");
        assert_eq!(
            error.as_str(),
            "owned snapshot retained a nontransportable runtime value"
        );
    }
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
