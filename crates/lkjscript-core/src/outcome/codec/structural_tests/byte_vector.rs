#![allow(clippy::expect_used)]

use super::*;

#[test]
fn semantic_byte_vector_keeps_public_owned_snapshot_semantics() {
    let bytes = vec![0, 1, 255];
    let value = value(
        30,
        StructuralKind::ByteVector,
        SemanticPayload::ByteVector(bytes.clone()),
    );
    let owned = OwnedValue::from_structural(value, StructuralSnapshotLimits::DEFAULT)
        .expect("owned byte-vector");
    assert_eq!(owned.as_byte_vector(), Some(bytes.as_slice()));
    assert_eq!(owned.as_bytes(), None);
}
