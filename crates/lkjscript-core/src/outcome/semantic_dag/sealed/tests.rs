#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use crate::{
    InlineStructuralValue, LayoutIdentity, SemanticDagKind, SemanticDagNode, SemanticDagNodeId,
    SemanticDagPayload, SemanticDagSnapshot, SemanticDagType, SemanticTypeIdentity,
    StructuralSnapshotLimits,
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
        SemanticTypeIdentity::new(NonZeroU64::new(1_090).expect("semantic identity")),
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

#[path = "tests/rehydration.rs"]
mod rehydration;
#[path = "tests/rehydration_bytes.rs"]
mod rehydration_bytes;
