use super::*;

#[test]
fn constructor_rejects_forward_cycles_unreachable_nodes_and_nonfinal_root() {
    let forward = vec![
        node(
            1,
            SemanticDagKind::Product,
            SemanticDagPayload::Product(vec![SemanticDagNodeId::new(1)]),
        ),
        node(
            2,
            SemanticDagKind::Product,
            SemanticDagPayload::Product(vec![SemanticDagNodeId::new(0)]),
        ),
    ];
    assert!(SemanticDagSnapshot::new(forward, SemanticDagNodeId::new(1)).is_err());

    let self_cycle = vec![node(
        3,
        SemanticDagKind::Product,
        SemanticDagPayload::Product(vec![SemanticDagNodeId::new(0)]),
    )];
    assert!(SemanticDagSnapshot::new(self_cycle, SemanticDagNodeId::new(0)).is_err());

    let unreachable = vec![
        node(
            4,
            SemanticDagKind::I64,
            SemanticDagPayload::Inline(InlineStructuralValue::I64(1)),
        ),
        node(
            5,
            SemanticDagKind::I64,
            SemanticDagPayload::Inline(InlineStructuralValue::I64(2)),
        ),
    ];
    assert!(SemanticDagSnapshot::new(unreachable, SemanticDagNodeId::new(1)).is_err());

    assert!(SemanticDagSnapshot::new(
        product_list_product().nodes().to_vec(),
        SemanticDagNodeId::new(4),
    )
    .is_err());
}

#[test]
fn constructor_rejects_kind_and_list_type_layout_disagreement() {
    let wrong_kind = vec![node(
        1,
        SemanticDagKind::Bool,
        SemanticDagPayload::Inline(InlineStructuralValue::I64(1)),
    )];
    assert!(SemanticDagSnapshot::new(wrong_kind, SemanticDagNodeId::new(0)).is_err());

    let wrong_tail = vec![
        SemanticDagNode::new(
            list_ty(SemanticDagKind::EmptyList),
            SemanticDagPayload::EmptyList,
        ),
        SemanticDagNode::new(
            ty(91, SemanticDagKind::List),
            SemanticDagPayload::List {
                head: SemanticDagNodeId::new(0),
                tail: SemanticDagNodeId::new(0),
            },
        ),
    ];
    assert!(SemanticDagSnapshot::new(wrong_tail, SemanticDagNodeId::new(1)).is_err());
}
