#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use super::*;
use crate::{
    LayoutIdentity, OwnedValue, SemanticPayload, SemanticTypeIdentity, SemanticValue,
    StructuralKind, StructuralSnapshotLimits, StructuralType,
};

fn value(id: u64, kind: StructuralKind, payload: SemanticPayload) -> SemanticValue {
    SemanticValue::new(
        StructuralType::new(
            LayoutIdentity::new(NonZeroU64::new(id).expect("layout")),
            SemanticTypeIdentity::new(NonZeroU64::new(id + 100).expect("semantic")),
            kind,
        ),
        payload,
    )
}

fn leaf(id: u64) -> SemanticValue {
    value(
        id,
        StructuralKind::Bytes,
        SemanticPayload::Bytes(Vec::new()),
    )
}

fn rejects(value: SemanticValue, limits: StructuralSnapshotLimits) {
    assert!(OwnedValue::from_structural(value, limits).is_err());
}

fn encoded(value: SemanticValue) -> Vec<u8> {
    let owned = OwnedValue::from_structural(value, StructuralSnapshotLimits::DEFAULT)
        .expect("structural snapshot");
    encode_execution_outcome(&ExecutionOutcome::Returned(owned), 2 * 1024 * 1024).expect("encode")
}

#[test]
fn structural_constructor_rejects_each_exact_bound_plus_one() {
    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_depth = 1;
    rejects(
        value(
            1,
            StructuralKind::Product,
            SemanticPayload::Product(vec![leaf(2)]),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_nodes = 2;
    rejects(
        value(
            3,
            StructuralKind::Product,
            SemanticPayload::Product(vec![leaf(4), leaf(5)]),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_fields = 1;
    rejects(
        value(
            6,
            StructuralKind::Product,
            SemanticPayload::Product(vec![leaf(7), leaf(8)]),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_aggregate_bytes = 2;
    limits.max_string_bytes = 2;
    limits.max_path_bytes = 2;
    rejects(
        value(9, StructuralKind::Bytes, SemanticPayload::Bytes(vec![0; 3])),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_string_bytes = 2;
    rejects(
        value(
            10,
            StructuralKind::String,
            SemanticPayload::String(b"abc".to_vec()),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_path_bytes = 2;
    rejects(
        value(
            11,
            StructuralKind::Path,
            SemanticPayload::Path(b"/ab".to_vec()),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_encode_work = 2;
    rejects(
        value(
            12,
            StructuralKind::Bytes,
            SemanticPayload::Bytes(vec![0; 2]),
        ),
        limits,
    );
}

#[test]
fn structural_constructor_rejects_wrong_payload_utf8_and_linux_paths() {
    rejects(
        value(
            20,
            StructuralKind::Path,
            SemanticPayload::String(b"wrong".to_vec()),
        ),
        StructuralSnapshotLimits::DEFAULT,
    );
    rejects(
        value(
            21,
            StructuralKind::String,
            SemanticPayload::String(vec![255]),
        ),
        StructuralSnapshotLimits::DEFAULT,
    );
    for path in [Vec::new(), b"relative".to_vec(), b"/nul\0path".to_vec()] {
        rejects(
            value(22, StructuralKind::Path, SemanticPayload::Path(path)),
            StructuralSnapshotLimits::DEFAULT,
        );
    }
    rejects(
        value(
            23,
            StructuralKind::Path,
            SemanticPayload::Path(vec![b'/'; crate::MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES + 1]),
        ),
        StructuralSnapshotLimits::DEFAULT,
    );
    let maximum = vec![b'/'; crate::MAX_STRUCTURAL_SNAPSHOT_PATH_BYTES];
    assert!(OwnedValue::from_structural(
        value(24, StructuralKind::Path, SemanticPayload::Path(maximum)),
        StructuralSnapshotLimits::DEFAULT,
    )
    .is_ok());
}

#[test]
fn structural_decoder_checks_work_and_field_bounds_before_publication() {
    let bytes = encoded(value(
        30,
        StructuralKind::Bytes,
        SemanticPayload::Bytes(vec![1, 2]),
    ));
    let mut structural = StructuralSnapshotLimits::DEFAULT;
    structural.max_decode_work = 2;
    let limits = ExecutionOutcomeCodecLimits::new(2 * 1024 * 1024, structural);
    assert!(decode_execution_outcome(&bytes, limits).is_err());

    let product = value(
        31,
        StructuralKind::Product,
        SemanticPayload::Product(Vec::new()),
    );
    let mut malformed = encoded(product);
    malformed[20..24].copy_from_slice(&2_u32.to_le_bytes());
    let mut structural = StructuralSnapshotLimits::DEFAULT;
    structural.max_fields = 1;
    let limits = ExecutionOutcomeCodecLimits::new(2 * 1024 * 1024, structural);
    assert!(decode_execution_outcome(&malformed, limits).is_err());
}

#[test]
fn structural_limit_descriptors_reject_zero_and_hard_plus_one() {
    let mut zero = StructuralSnapshotLimits::DEFAULT;
    zero.max_nodes = 0;
    assert!(zero.validate().is_err());
    let mut oversized = StructuralSnapshotLimits::DEFAULT;
    oversized.max_nodes = crate::MAX_STRUCTURAL_SNAPSHOT_NODES + 1;
    assert!(oversized.validate().is_err());
}
