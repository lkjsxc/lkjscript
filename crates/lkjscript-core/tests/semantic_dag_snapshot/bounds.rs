use super::*;

#[test]
fn constructor_and_decoder_enforce_node_edge_depth_byte_and_work_bounds() {
    let fixture = product_list_product();
    for limits in [
        StructuralSnapshotLimits {
            max_nodes: 5,
            ..StructuralSnapshotLimits::DEFAULT
        },
        StructuralSnapshotLimits {
            max_fields: 6,
            ..StructuralSnapshotLimits::DEFAULT
        },
        StructuralSnapshotLimits {
            max_depth: 4,
            ..StructuralSnapshotLimits::DEFAULT
        },
        StructuralSnapshotLimits {
            max_encode_work: 12,
            ..StructuralSnapshotLimits::DEFAULT
        },
    ] {
        assert!(
            SemanticDagSnapshot::new(fixture.nodes().to_vec(), fixture.root(), limits,).is_err()
        );
    }

    let fixture_wire = encoded(product_list_product());
    for structural in [
        StructuralSnapshotLimits {
            max_nodes: 5,
            ..StructuralSnapshotLimits::DEFAULT
        },
        StructuralSnapshotLimits {
            max_fields: 6,
            ..StructuralSnapshotLimits::DEFAULT
        },
        StructuralSnapshotLimits {
            max_depth: 4,
            ..StructuralSnapshotLimits::DEFAULT
        },
        StructuralSnapshotLimits {
            max_decode_work: 12,
            ..StructuralSnapshotLimits::DEFAULT
        },
    ] {
        let limits = ExecutionOutcomeCodecLimits::new(2 * 1024 * 1024, structural);
        assert!(decode_execution_outcome(&fixture_wire, limits).is_err());
    }

    let bytes_snapshot = SemanticDagSnapshot::new(
        vec![node(
            8,
            SemanticDagKind::Bytes,
            SemanticDagPayload::Bytes(vec![1, 2, 3]),
        )],
        SemanticDagNodeId::new(0),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("bytes snapshot");
    let wire = encoded(bytes_snapshot);
    let limits = ExecutionOutcomeCodecLimits::new(
        2 * 1024 * 1024,
        StructuralSnapshotLimits {
            max_aggregate_bytes: 2,
            max_string_bytes: 2,
            max_path_bytes: 2,
            ..StructuralSnapshotLimits::DEFAULT
        },
    );
    assert!(decode_execution_outcome(&wire, limits).is_err());
}

#[test]
fn decoder_rejects_zero_identity_forward_cycle_and_trailing_bytes() {
    let single = SemanticDagSnapshot::new(
        vec![node(
            1,
            SemanticDagKind::I64,
            SemanticDagPayload::Inline(InlineStructuralValue::I64(7)),
        )],
        SemanticDagNodeId::new(0),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("single node");
    let mut zero_identity = encoded(single);
    zero_identity[10..18].fill(0);
    assert!(decode_execution_outcome(&zero_identity, 2 * 1024 * 1024).is_err());

    let product = SemanticDagSnapshot::new(
        vec![
            node(
                1,
                SemanticDagKind::I64,
                SemanticDagPayload::Inline(InlineStructuralValue::I64(7)),
            ),
            node(
                2,
                SemanticDagKind::Product,
                SemanticDagPayload::Product(vec![SemanticDagNodeId::new(0)]),
            ),
        ],
        SemanticDagNodeId::new(1),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("two nodes");
    let mut cycle = encoded(product);
    cycle[56..60].copy_from_slice(&1_u32.to_le_bytes());
    assert!(decode_execution_outcome(&cycle, 2 * 1024 * 1024).is_err());

    let mut trailing = encoded(product_list_product());
    trailing.push(0);
    assert!(decode_execution_outcome(&trailing, 2 * 1024 * 1024).is_err());
}

#[test]
fn failed_private_decode_does_not_publish_or_poison_later_success() {
    let valid = encoded(product_list_product());
    for length in 0..valid.len() {
        assert!(decode_execution_outcome(&valid[..length], 2 * 1024 * 1024).is_err());
    }
    let decoded = decode_execution_outcome(&valid, 2 * 1024 * 1024).expect("later valid decode");
    assert_eq!(
        decoded
            .returned()
            .and_then(OwnedValue::as_semantic_dag)
            .map(|snapshot| snapshot.nodes().len()),
        Some(6)
    );
}
