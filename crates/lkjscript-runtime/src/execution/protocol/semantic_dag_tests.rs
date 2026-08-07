#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use super::*;
use lkjscript_core::{
    InlineStructuralValue, LayoutIdentity, OwnedValue, SemanticDagKind, SemanticDagNode,
    SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot, SemanticDagType,
    SemanticTypeIdentity,
};

#[test]
fn process_protocol_round_trips_product_list_product_semantic_dag() {
    let dag_type = |id: u64, kind| {
        SemanticDagType::new(
            LayoutIdentity::new(NonZeroU64::new(id).expect("DAG layout")),
            SemanticTypeIdentity::new(NonZeroU64::new(id + 1_000).expect("DAG semantic")),
            kind,
        )
    };
    let list_type = |kind| {
        SemanticDagType::new(
            LayoutIdentity::new(NonZeroU64::new(80).expect("list layout")),
            SemanticTypeIdentity::new(NonZeroU64::new(1_080).expect("list semantic")),
            kind,
        )
    };
    let snapshot = SemanticDagSnapshot::new(
        vec![
            SemanticDagNode::new(
                dag_type(1, SemanticDagKind::I64),
                SemanticDagPayload::Inline(InlineStructuralValue::I64(7)),
            ),
            SemanticDagNode::new(
                dag_type(2, SemanticDagKind::Product),
                SemanticDagPayload::Product(vec![SemanticDagNodeId::new(0)]),
            ),
            SemanticDagNode::new(
                list_type(SemanticDagKind::EmptyList),
                SemanticDagPayload::EmptyList,
            ),
            SemanticDagNode::new(
                list_type(SemanticDagKind::List),
                SemanticDagPayload::List {
                    head: SemanticDagNodeId::new(1),
                    tail: SemanticDagNodeId::new(2),
                },
            ),
            SemanticDagNode::new(
                dag_type(3, SemanticDagKind::Product),
                SemanticDagPayload::Product(vec![SemanticDagNodeId::new(3)]),
            ),
        ],
        SemanticDagNodeId::new(4),
    )
    .expect("product-list-product DAG");
    let response = ProcessResponse::Outcome {
        provenance: super::tests::provenance(),
        cell: 78,
        outcome: ExecutionOutcome::Returned(OwnedValue::from_semantic_dag(snapshot)),
        output: b"semantic DAG output".to_vec(),
        flushes: 1,
    };
    let mut frame = Vec::new();
    write_response(&mut frame, &response).expect("semantic DAG process encode");
    assert_eq!(
        read_response(&mut frame.as_slice()).expect("semantic DAG process decode"),
        response
    );
}
