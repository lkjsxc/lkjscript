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

fn rejects_decode(value: SemanticValue, structural: StructuralSnapshotLimits) {
    let owned = OwnedValue::from_structural(value, StructuralSnapshotLimits::DEFAULT)
        .expect("default structural snapshot");
    let bytes = encode_execution_outcome(&ExecutionOutcome::Returned(owned), 2 * 1024 * 1024)
        .expect("encode structural snapshot");
    let limits = ExecutionOutcomeCodecLimits::new(2 * 1024 * 1024, structural);
    assert!(decode_execution_outcome(&bytes, limits).is_err());
}

#[test]
fn structural_decoder_rejects_every_exact_bound_plus_one() {
    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_depth = 1;
    rejects_decode(
        value(
            1,
            StructuralKind::Product,
            SemanticPayload::Product(vec![leaf(2)]),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_nodes = 2;
    rejects_decode(
        value(
            3,
            StructuralKind::Product,
            SemanticPayload::Product(vec![leaf(4), leaf(5)]),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_fields = 1;
    rejects_decode(
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
    rejects_decode(
        value(9, StructuralKind::Bytes, SemanticPayload::Bytes(vec![0; 3])),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_string_bytes = 2;
    rejects_decode(
        value(
            10,
            StructuralKind::String,
            SemanticPayload::String(b"abc".to_vec()),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_path_bytes = 2;
    rejects_decode(
        value(
            11,
            StructuralKind::Path,
            SemanticPayload::Path(b"/ab".to_vec()),
        ),
        limits,
    );

    let mut limits = StructuralSnapshotLimits::DEFAULT;
    limits.max_decode_work = 2;
    rejects_decode(
        value(
            12,
            StructuralKind::Bytes,
            SemanticPayload::Bytes(vec![0; 2]),
        ),
        limits,
    );
}
