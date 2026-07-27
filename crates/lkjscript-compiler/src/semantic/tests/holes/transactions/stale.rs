use super::super::{case_dir, function_source, hole, snapshot};
use super::{apply, target};
use crate::semantic::schema::{
    Expression, ProtocolErrorCode, ResponseResult, TransactionOperation,
};

#[test]
fn stale_revision_and_expected_type_reject_without_mutation() {
    let directory = case_dir("hole-stale");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, function_source(&hole("body", None))).expect("write source");
    let before = std::fs::read(&root).expect("bytes before stale transaction");
    let (revision, snapshot) = snapshot(&root);
    let (declaration, node) = target(&snapshot);
    let fill = TransactionOperation::FillHole {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        node: node.index,
        node_fingerprint: node.fingerprint.clone(),
        hole_identity: "body".into(),
        expected_type: "bool".into(),
        expression: Expression::Bool { value: true },
    };
    assert!(
        matches!(apply(&root, revision, &snapshot.source_units, vec![fill]),
        ResponseResult::Error { error, .. }
            if error.code == ProtocolErrorCode::PreconditionFailed)
    );
    let stale_fill = TransactionOperation::FillHole {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        node: node.index,
        node_fingerprint: node.fingerprint.clone(),
        hole_identity: "body".into(),
        expected_type: "i64".into(),
        expression: Expression::I64 { value: 0 },
    };
    assert!(
        matches!(apply(&root, "0".repeat(64), &snapshot.source_units, vec![stale_fill]),
        ResponseResult::Error { error, .. } if error.code == ProtocolErrorCode::StaleRevision)
    );
    assert_eq!(
        std::fs::read(&root).expect("bytes after stale transaction"),
        before
    );
}
