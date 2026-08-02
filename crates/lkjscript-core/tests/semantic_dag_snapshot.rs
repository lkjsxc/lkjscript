#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    decode_execution_outcome, encode_execution_outcome, ExecutionOutcome,
    ExecutionOutcomeCodecLimits, InlineStructuralValue, LayoutIdentity, OwnedValue,
    SemanticDagKind, SemanticDagNode, SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot,
    SemanticDagType, SemanticTypeIdentity, StructuralSnapshotLimits,
};

fn ty(id: u64, kind: SemanticDagKind) -> SemanticDagType {
    SemanticDagType::new(
        LayoutIdentity::new(NonZeroU64::new(id).expect("layout identity")),
        SemanticTypeIdentity::new(NonZeroU64::new(id + 1_000).expect("semantic identity")),
        kind,
    )
}

fn list_ty(kind: SemanticDagKind) -> SemanticDagType {
    SemanticDagType::new(
        LayoutIdentity::new(NonZeroU64::new(90).expect("list layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(1_090).expect("list semantic type")),
        kind,
    )
}

fn node(id: u64, kind: SemanticDagKind, payload: SemanticDagPayload) -> SemanticDagNode {
    SemanticDagNode::new(ty(id, kind), payload)
}

fn product_list_product() -> SemanticDagSnapshot {
    let nodes = vec![
        node(
            1,
            SemanticDagKind::I64,
            SemanticDagPayload::Inline(InlineStructuralValue::I64(41)),
        ),
        node(
            2,
            SemanticDagKind::Product,
            SemanticDagPayload::Product(vec![SemanticDagNodeId::new(0)]),
        ),
        SemanticDagNode::new(
            list_ty(SemanticDagKind::EmptyList),
            SemanticDagPayload::EmptyList,
        ),
        SemanticDagNode::new(
            list_ty(SemanticDagKind::List),
            SemanticDagPayload::List {
                head: SemanticDagNodeId::new(1),
                tail: SemanticDagNodeId::new(2),
            },
        ),
        SemanticDagNode::new(
            list_ty(SemanticDagKind::List),
            SemanticDagPayload::List {
                head: SemanticDagNodeId::new(1),
                tail: SemanticDagNodeId::new(3),
            },
        ),
        node(
            3,
            SemanticDagKind::Product,
            SemanticDagPayload::Product(vec![SemanticDagNodeId::new(4), SemanticDagNodeId::new(1)]),
        ),
    ];
    SemanticDagSnapshot::new(
        nodes,
        SemanticDagNodeId::new(5),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("product-list-product semantic DAG")
}

fn encoded(snapshot: SemanticDagSnapshot) -> Vec<u8> {
    encode_execution_outcome(
        &ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(snapshot)),
        2 * 1024 * 1024,
    )
    .expect("semantic DAG outcome encode")
}

#[test]
fn product_list_product_dag_round_trips_with_sharing_and_exact_identity() {
    let snapshot = product_list_product();
    assert_eq!(snapshot.metrics().nodes, 6);
    assert_eq!(snapshot.metrics().fields, 7);
    assert_eq!(snapshot.metrics().aggregate_bytes, 0);
    assert_eq!(snapshot.metrics().encode_work, 13);
    assert_eq!(
        snapshot.root_node().value_type,
        ty(3, SemanticDagKind::Product)
    );
    snapshot
        .require_root_type(ty(3, SemanticDagKind::Product))
        .expect("exact root identity");
    assert!(snapshot
        .require_root_type(ty(4, SemanticDagKind::Product))
        .is_err());

    let outcome = ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(snapshot));
    let bytes = encode_execution_outcome(&outcome, 2 * 1024 * 1024).expect("encode");
    let decoded = decode_execution_outcome(&bytes, 2 * 1024 * 1024).expect("decode");
    assert_eq!(decoded, outcome);
    let returned = decoded.returned().expect("returned value");
    let decoded = returned.as_semantic_dag().expect("semantic DAG");
    assert_eq!(
        decoded.nodes()[3].payload,
        SemanticDagPayload::List {
            head: SemanticDagNodeId::new(1),
            tail: SemanticDagNodeId::new(2),
        }
    );
    assert_eq!(
        decoded.nodes()[4].payload,
        SemanticDagPayload::List {
            head: SemanticDagNodeId::new(1),
            tail: SemanticDagNodeId::new(3),
        }
    );
}

#[path = "semantic_dag_snapshot/bounds.rs"]
mod bounds;
#[path = "semantic_dag_snapshot/rejection.rs"]
mod rejection;
