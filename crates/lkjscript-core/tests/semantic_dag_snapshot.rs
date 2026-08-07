#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    decode_execution_outcome, encode_execution_outcome, ExecutionOutcome, InlineStructuralValue,
    LayoutIdentity, OwnedValue, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, SemanticTypeIdentity,
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
    SemanticDagSnapshot::new(nodes, SemanticDagNodeId::new(5))
        .expect("product-list-product semantic DAG")
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

#[test]
fn semantic_dag_codec_crosses_former_node_and_field_limits() {
    const COUNT: u64 = 65_537;
    let mut nodes = Vec::with_capacity(usize::try_from(COUNT).expect("test count fits usize"));
    for value in 0..COUNT - 1 {
        nodes.push(node(
            10,
            SemanticDagKind::I64,
            SemanticDagPayload::Inline(InlineStructuralValue::I64(
                i64::try_from(value).expect("test value fits i64"),
            )),
        ));
    }
    nodes.push(node(
        11,
        SemanticDagKind::Product,
        SemanticDagPayload::Product((0..COUNT - 1).map(SemanticDagNodeId::new).collect()),
    ));
    let snapshot = SemanticDagSnapshot::new(nodes, SemanticDagNodeId::new(COUNT - 1))
        .expect("wide semantic DAG");
    let outcome = ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(snapshot));
    let bytes = encode_execution_outcome(&outcome, 16 * 1024 * 1024).expect("wide encode");
    let decoded = decode_execution_outcome(&bytes, 16 * 1024 * 1024).expect("wide decode");
    let metrics = decoded
        .returned()
        .and_then(OwnedValue::as_semantic_dag)
        .expect("wide returned DAG")
        .metrics();
    assert_eq!(metrics.nodes, COUNT);
    assert_eq!(metrics.fields, COUNT - 1);
}

#[test]
fn high_malformed_dag_references_fail_before_host_indexing() {
    let high = u64::from(u32::MAX) + 19;
    let nodes = vec![node(
        12,
        SemanticDagKind::Product,
        SemanticDagPayload::Product(vec![SemanticDagNodeId::new(high)]),
    )];
    assert!(SemanticDagSnapshot::new(nodes, SemanticDagNodeId::new(0)).is_err());

    let valid = ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(
        SemanticDagSnapshot::new(
            vec![node(
                13,
                SemanticDagKind::I64,
                SemanticDagPayload::Inline(InlineStructuralValue::I64(1)),
            )],
            SemanticDagNodeId::new(0),
        )
        .expect("valid DAG"),
    ));
    let mut wire = encode_execution_outcome(&valid, 4096).expect("encode DAG");
    let root = wire
        .len()
        .checked_sub(8)
        .expect("wire contains canonical u64 root ID");
    wire[root..].copy_from_slice(&high.to_le_bytes());
    assert!(decode_execution_outcome(&wire, 4096).is_err());
}

#[path = "semantic_dag_snapshot/rejection.rs"]
mod rejection;
