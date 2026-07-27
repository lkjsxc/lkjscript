mod stale;

use super::*;
use crate::semantic::schema::*;

pub(super) fn apply(
    root: &std::path::Path,
    base_revision: String,
    source_units: &[SourceUnitRecord],
    operations: Vec<TransactionOperation>,
) -> ResponseResult {
    let file_preconditions = source_units
        .iter()
        .map(|unit| FilePrecondition {
            path: unit.path.clone(),
            bytes: unit.bytes,
            sha256: unit.sha256.clone(),
        })
        .collect();
    let request = Request {
        schema: crate::semantic::SCHEMA.into(),
        contract: crate::semantic::CONTRACT.to_hex(),
        profile: ResourceProfile::Sandbox,
        root: root.to_string_lossy().into_owned(),
        operation: OperationRequest::ApplyTransaction {
            mode: ApplyMode::Preview,
            base_revision,
            file_preconditions,
            operations,
        },
    };
    let encoded = serde_json::to_vec(&request).expect("encode transaction");
    response(&crate::semantic::execute(&encoded).expect("execute transaction")).result
}

pub(super) fn target(snapshot: &SnapshotResult) -> (&DeclarationRecord, &NodeRecord) {
    let declaration = snapshot
        .declarations
        .iter()
        .find(|item| item.name == "f")
        .expect("function declaration");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| {
            node.declaration.as_deref() == Some(&declaration.key)
                && node.kind == SemanticNodeKind::TypedHole
        })
        .expect("typed hole");
    (declaration, node)
}

#[test]
fn all_closed_hole_transactions_validate_atomically() {
    let directory = case_dir("hole-transactions");
    let root = directory.join("main.lkjscript");
    std::fs::write(&root, function_source(&hole("body", None))).expect("write hole source");
    let original = std::fs::read(&root).expect("original bytes");
    let (revision, snapshot) = super::snapshot(&root);
    let (declaration, node) = target(&snapshot);
    let common = || {
        (
            declaration.key.clone(),
            declaration.fingerprint.clone(),
            node.index,
            node.fingerprint.clone(),
            "body".to_string(),
            "i64".to_string(),
        )
    };
    let (declaration_key, entity_fingerprint, node, node_fingerprint, hole_identity, expected_type) =
        common();
    let refine = TransactionOperation::RefineHole {
        declaration_key,
        entity_fingerprint,
        node,
        node_fingerprint,
        hole_identity,
        expected_type,
        goal: Some("refined".into()),
    };
    let result = apply(
        &root,
        revision.clone(),
        &snapshot.source_units,
        vec![refine],
    );
    let ResponseResult::ApplyTransaction { transaction } = result else {
        panic!("refine_hole failed")
    };
    assert_eq!(
        transaction.semantic_diff[0].relation,
        IdentityRelationKind::RefinedHole
    );

    let (declaration_key, entity_fingerprint, node, node_fingerprint, hole_identity, expected_type) =
        common();
    let fill = TransactionOperation::FillHole {
        declaration_key,
        entity_fingerprint,
        node,
        node_fingerprint,
        hole_identity,
        expected_type,
        expression: Expression::I64 { value: 7 },
    };
    let result = apply(&root, revision, &snapshot.source_units, vec![fill]);
    let ResponseResult::ApplyTransaction { transaction } = result else {
        panic!("fill_hole failed")
    };
    assert_eq!(
        transaction.semantic_diff[0].relation,
        IdentityRelationKind::FilledHole
    );
    assert_eq!(
        std::fs::read(&root).expect("bytes after previews"),
        original
    );

    let complete = directory.join("complete.lkjscript");
    std::fs::write(&complete, function_source("0\n")).expect("write complete source");
    let (revision, snapshot) = super::snapshot(&complete);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|item| item.name == "f")
        .expect("complete function");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| {
            node.declaration.as_deref() == Some(&declaration.key)
                && node.kind == SemanticNodeKind::I64Literal
        })
        .expect("complete expression");
    let insert = TransactionOperation::InsertHole {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        node: node.index,
        node_fingerprint: node.fingerprint.clone(),
        hole_identity: "inserted".into(),
        goal: None,
        expected_type: "i64".into(),
    };
    let result = apply(&complete, revision, &snapshot.source_units, vec![insert]);
    let ResponseResult::ApplyTransaction { transaction } = result else {
        panic!("insert_hole failed")
    };
    assert_eq!(
        transaction.semantic_diff[0].relation,
        IdentityRelationKind::InsertedHole
    );

    let deletable = directory.join("delete.lkjscript");
    let delete_source =
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n0\nhole/\nname/\nlast\n/name\n/hole\n/do\n/main\n";
    std::fs::write(&deletable, delete_source).expect("write deletable source");
    let (revision, snapshot) = super::snapshot(&deletable);
    let declaration = snapshot
        .declarations
        .iter()
        .find(|item| item.kind == SemanticDeclarationKind::Main)
        .expect("main declaration");
    let node = snapshot
        .nodes
        .iter()
        .find(|node| node.kind == SemanticNodeKind::TypedHole)
        .expect("deletable hole");
    let delete = TransactionOperation::DeleteHole {
        declaration_key: declaration.key.clone(),
        entity_fingerprint: declaration.fingerprint.clone(),
        node: node.index,
        node_fingerprint: node.fingerprint.clone(),
        hole_identity: "last".into(),
        expected_type: "i64".into(),
    };
    let result = apply(&deletable, revision, &snapshot.source_units, vec![delete]);
    let ResponseResult::ApplyTransaction { transaction } = result else {
        panic!("delete_hole failed")
    };
    assert_eq!(
        transaction.semantic_diff[0].relation,
        IdentityRelationKind::DeletedHole
    );
}
