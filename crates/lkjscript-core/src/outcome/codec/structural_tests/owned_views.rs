#![allow(clippy::expect_used)]

use super::*;

#[test]
fn standalone_structural_string_and_path_have_key_free_owned_views() {
    let string_value = value(
        30,
        StructuralKind::String,
        SemanticPayload::String("deterministic".as_bytes().to_vec()),
    );
    let string =
        OwnedValue::from_structural(string_value.clone(), StructuralSnapshotLimits::DEFAULT)
            .expect("owned string");
    let same_string = OwnedValue::from_structural(string_value, StructuralSnapshotLimits::DEFAULT)
        .expect("same owned string");
    assert_eq!(string.as_str(), Some("deterministic"));
    assert_eq!(string.as_path_bytes(), None);
    assert_eq!(format!("{string:?}"), "\"deterministic\"");
    assert_eq!(string, same_string);
    assert_eq!(string.snapshot_object_count(), 1);
    let string_outcome = ExecutionOutcome::Returned(string.clone());
    let string_wire =
        encode_execution_outcome(&string_outcome, 2 * 1024 * 1024).expect("encode string");
    assert_eq!(
        decode_execution_outcome(&string_wire, 2 * 1024 * 1024).expect("decode string"),
        string_outcome
    );

    let path_value = value(
        31,
        StructuralKind::Path,
        SemanticPayload::Path(b"/tmp/deterministic".to_vec()),
    );
    let path = OwnedValue::from_structural(path_value.clone(), StructuralSnapshotLimits::DEFAULT)
        .expect("owned path");
    let same_path = OwnedValue::from_structural(path_value, StructuralSnapshotLimits::DEFAULT)
        .expect("same owned path");
    assert_eq!(path.as_str(), None);
    assert_eq!(path.as_path_bytes(), Some(b"/tmp/deterministic".as_slice()));
    assert_eq!(format!("{path:?}"), "#<owned-path:18>");
    assert_eq!(path, same_path);
    assert_eq!(path.snapshot_object_count(), 1);
    let path_outcome = ExecutionOutcome::Returned(path.clone());
    let path_wire = encode_execution_outcome(&path_outcome, 2 * 1024 * 1024).expect("encode path");
    assert_eq!(
        decode_execution_outcome(&path_wire, 2 * 1024 * 1024).expect("decode path"),
        path_outcome
    );
}
