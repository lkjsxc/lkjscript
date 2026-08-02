use super::*;
use crate::{SealedSemanticDagRuntime, StructuralLimits};

#[test]
fn string_path_and_bytes_rehydrate_with_canonical_chunk_export() {
    let nodes = vec![
        node(
            10,
            SemanticDagKind::String,
            SemanticDagPayload::String(vec![b's'; 70]),
        ),
        node(
            11,
            SemanticDagKind::Path,
            SemanticDagPayload::Path(b"/sealed/semantic/dag".to_vec()),
        ),
        node(
            12,
            SemanticDagKind::Bytes,
            SemanticDagPayload::Bytes((0_u8..65).collect()),
        ),
        node(
            13,
            SemanticDagKind::Product,
            SemanticDagPayload::Product(vec![
                SemanticDagNodeId::new(0),
                SemanticDagNodeId::new(1),
                SemanticDagNodeId::new(2),
            ]),
        ),
    ];
    let snapshot = SemanticDagSnapshot::new(
        nodes,
        SemanticDagNodeId::new(3),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("byte-bearing snapshot");
    let expected = snapshot.clone();
    let root_type = snapshot.root_node().value_type;
    let mut closure = snapshot
        .nodes()
        .iter()
        .map(|node| node.value_type)
        .collect::<Vec<_>>();
    closure.sort_unstable();
    closure.dedup();

    let mut runtime = SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("runtime");
    let owner = runtime
        .rehydrate(snapshot, root_type, &closure)
        .expect("rehydrate");
    let borrow = runtime.begin_borrow(&owner).expect("borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");
    let report = runtime.release(owner).expect("release");
    assert_eq!(report.regions_released, 1);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}
