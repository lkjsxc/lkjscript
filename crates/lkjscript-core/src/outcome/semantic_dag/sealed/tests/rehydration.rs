use super::*;
use crate::{
    SealedSemanticDagError, SealedSemanticDagRuntime, StructuralError, StructuralLimit,
    StructuralLimits,
};

fn type_closure(snapshot: &SemanticDagSnapshot) -> Vec<SemanticDagType> {
    let mut types = snapshot
        .nodes()
        .iter()
        .map(|node| node.value_type)
        .collect::<Vec<_>>();
    types.sort_unstable();
    types.dedup();
    types
}

#[test]
fn validated_dag_rehydrates_and_round_trips_through_one_coarse_region() {
    let snapshot = product_list_product();
    let expected = snapshot.clone();
    let closure = type_closure(&snapshot);
    let root_type = snapshot.root_node().value_type;
    let mut runtime = SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("runtime");
    let owner = runtime
        .rehydrate(snapshot, root_type, &closure)
        .expect("rehydrate");
    assert_eq!(owner.node_count(), 6);
    assert_eq!(owner.value_type(), root_type);
    assert_eq!(runtime.metrics().runtime.live_domains, 1);

    let retained = runtime.retain(&owner).expect("coarse retain");
    let borrow = runtime.begin_borrow(&owner).expect("region borrow");
    assert_eq!(runtime.export_snapshot(&borrow).expect("export"), expected);
    runtime.end_borrow(borrow).expect("end borrow");

    let first = runtime.release(retained).expect("release retained");
    assert_eq!(first.regions_released, 0);
    assert_eq!(first.cells_released, 0);
    let final_report = runtime.release(owner).expect("release final owner");
    assert_eq!(final_report.regions_released, 1);
    assert!(final_report.cells_released > expected.metrics().nodes as u64);
    assert_eq!(final_report.dependency_releases, 0);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
    assert_eq!(runtime.metrics().sealed.retains, 1);
    assert_eq!(runtime.metrics().sealed.releases, 2);
    runtime.validate().expect("valid empty runtime");
}

#[test]
fn unresolved_type_and_cell_limit_fail_before_builder_allocation() {
    let snapshot = product_list_product();
    let root_type = snapshot.root_node().value_type;
    let incomplete = vec![root_type];
    let expected_snapshot = snapshot.clone();
    let mut runtime = SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("runtime");
    let failure = runtime
        .rehydrate(snapshot, root_type, &incomplete)
        .expect_err("unresolved type rejected");
    assert!(matches!(
        failure.error,
        SealedSemanticDagError::UnresolvedType(_)
    ));
    assert_eq!(*failure.snapshot, expected_snapshot);
    assert_eq!(runtime.metrics().typed_stores, 0);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);

    let snapshot = product_list_product();
    let closure = type_closure(&snapshot);
    let limits = StructuralLimits {
        max_objects_per_domain: 4,
        ..StructuralLimits::default()
    };
    let mut runtime = SealedSemanticDagRuntime::new(limits).expect("bounded runtime");
    let failure = runtime
        .rehydrate(snapshot, root_type, &closure)
        .expect_err("cell count rejected");
    assert_eq!(
        failure.error,
        SealedSemanticDagError::Structural(StructuralError::LimitExceeded(
            StructuralLimit::Objects,
        ))
    );
    assert_eq!(runtime.metrics().typed_stores, 0);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);

    let snapshot = product_list_product();
    let closure = type_closure(&snapshot);
    let limits = StructuralLimits {
        max_chunks_per_domain: 1,
        chunk_objects: 1,
        ..StructuralLimits::default()
    };
    let mut runtime = SealedSemanticDagRuntime::new(limits).expect("bounded runtime");
    let failure = runtime
        .rehydrate(snapshot, root_type, &closure)
        .expect_err("mid-build chunk exhaustion rejected");
    assert_eq!(
        failure.error,
        SealedSemanticDagError::Structural(
            StructuralError::LimitExceeded(StructuralLimit::Chunks,)
        )
    );
    assert_eq!(runtime.metrics().typed_stores, 0);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
    runtime.validate().expect("rolled-back builder");
}

#[test]
fn final_release_rejects_a_live_region_borrow_without_losing_the_owner() {
    let snapshot = product_list_product();
    let closure = type_closure(&snapshot);
    let root_type = snapshot.root_node().value_type;
    let mut runtime = SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("runtime");
    let owner = runtime
        .rehydrate(snapshot, root_type, &closure)
        .expect("rehydrate");
    let borrow = runtime.begin_borrow(&owner).expect("borrow");
    let mut wrong_runtime =
        SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("other runtime");
    let borrow_failure = wrong_runtime
        .end_borrow(borrow)
        .expect_err("wrong runtime returns loan token");
    assert_eq!(borrow_failure.error, SealedSemanticDagError::CorruptRegion);
    let borrow = *borrow_failure.borrow;
    let failure = runtime
        .release(owner)
        .expect_err("live borrow blocks final release");
    assert_eq!(
        failure.error,
        SealedSemanticDagError::Structural(StructuralError::LiveLoan)
    );
    runtime.end_borrow(borrow).expect("end borrow");
    let report = runtime
        .release(failure.owner)
        .expect("release returned owner");
    assert_eq!(report.regions_released, 1);
    assert_eq!(runtime.metrics().runtime.live_domains, 0);
}

#[test]
fn owner_release_planning_is_independent_of_semantic_node_count() {
    let small = wide_product(8);
    let large = wide_product(2_048);
    let (small_cells, small_work) = rehydrate_and_release(small);
    let (large_cells, large_work) = rehydrate_and_release(large);
    assert!(large_cells > small_cells * 100);
    assert_eq!(small_work, 1);
    assert_eq!(large_work, 1);
}

fn rehydrate_and_release(snapshot: SemanticDagSnapshot) -> (u64, u64) {
    let closure = type_closure(&snapshot);
    let root_type = snapshot.root_node().value_type;
    let mut runtime = SealedSemanticDagRuntime::new(StructuralLimits::default()).expect("runtime");
    let owner = runtime
        .rehydrate(snapshot, root_type, &closure)
        .expect("rehydrate");
    let report = runtime.release(owner).expect("release");
    (report.cells_released, runtime.metrics().sealed.release_work)
}

fn wide_product(count: u32) -> SemanticDagSnapshot {
    assert!(count >= 2);
    let mut nodes = Vec::new();
    for value in 0..count - 1 {
        nodes.push(node(
            1,
            SemanticDagKind::I64,
            SemanticDagPayload::Inline(InlineStructuralValue::I64(i64::from(value))),
        ));
    }
    nodes.push(node(
        2,
        SemanticDagKind::Product,
        SemanticDagPayload::Product((0..count - 1).map(SemanticDagNodeId::new).collect()),
    ));
    SemanticDagSnapshot::new(
        nodes,
        SemanticDagNodeId::new(count - 1),
        StructuralSnapshotLimits::DEFAULT,
    )
    .expect("wide product")
}
