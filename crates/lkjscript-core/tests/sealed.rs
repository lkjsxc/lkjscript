#![allow(clippy::expect_used, clippy::panic)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, SealedRegionStore, SemanticTypeIdentity, StructuralLimits, StructuralRuntime,
};

fn setup() -> (StructuralRuntime, SealedRegionStore<u64, u8>) {
    setup_with_limits(StructuralLimits::default())
}

fn setup_with_limits(limits: StructuralLimits) -> (StructuralRuntime, SealedRegionStore<u64, u8>) {
    let runtime = StructuralRuntime::new(limits).expect("runtime");
    let store = SealedRegionStore::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(42).expect("nonzero")),
        SemanticTypeIdentity::new(NonZeroU64::new(43).expect("nonzero")),
        limits,
    )
    .expect("sealed store");
    (runtime, store)
}

#[test]
fn immutable_internal_cycle_shares_at_region_granularity() {
    let (mut runtime, mut store) = setup();
    let builder = store.begin(&mut runtime).expect("builder");
    let first = store.allocate(&builder, 10).expect("first");
    let second = store.allocate(&builder, 20).expect("second");
    store
        .add_internal_edge(&builder, first, second)
        .expect("edge");
    store
        .add_internal_edge(&builder, second, first)
        .expect("cycle");
    let mut owners = store.seal_batch(&mut runtime, vec![builder]).expect("seal");
    let owner = owners.pop().expect("owner");
    let root = store.root(&owner, 0).expect("root");
    assert_eq!(*store.get(root).expect("value"), 10);
    let weak = store.downgrade(root);

    let retained = store.retain(&owner).expect("retain");
    let first_release = store
        .release(&mut runtime, retained, |_| Result::<_, ()>::Ok(()))
        .expect("release retained");
    assert_eq!(first_release.regions_released, 0);
    let (upgraded, upgraded_root) = store.upgrade(weak).expect("upgrade").expect("live weak");
    let loan = store.begin_borrow(upgraded_root).expect("borrow");
    assert_eq!(*store.borrowed(&loan).expect("borrowed value"), 10);
    store.end_borrow(loan).expect("end borrow");
    store
        .release(&mut runtime, upgraded, |_| Result::<_, ()>::Ok(()))
        .expect("release upgraded");
    let final_report = store
        .release(&mut runtime, owner, |_| Result::<_, ()>::Ok(()))
        .expect("final release");
    assert_eq!(final_report.regions_released, 1);
    assert!(store.upgrade(weak).expect("stale upgrade").is_none());
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn sealed_dependencies_release_as_a_dag_without_payload_tracing() {
    let (mut runtime, mut store) = setup();
    let parent = store.begin(&mut runtime).expect("parent");
    let child = store.begin(&mut runtime).expect("child");
    store.allocate(&parent, 1).expect("parent value");
    store.allocate(&child, 2).expect("child value");
    store
        .add_dependency(&parent, child.domain())
        .expect("dependency");
    let mut owners = store
        .seal_batch(&mut runtime, vec![parent, child])
        .expect("seal DAG");
    let child_owner = owners.pop().expect("child owner");
    let parent_owner = owners.pop().expect("parent owner");
    store
        .release(&mut runtime, child_owner, |_| Result::<_, ()>::Ok(()))
        .expect("release explicit child owner");
    let report = store
        .release(&mut runtime, parent_owner, |_| Result::<_, ()>::Ok(()))
        .expect("release parent");
    assert_eq!(report.regions_released, 2);
    assert_eq!(report.dependency_releases, 1);
    assert_eq!(report.objects_released, 2);
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn seal_failure_changes_no_builder_or_runtime_state() {
    let limits = StructuralLimits {
        max_region_owners: 1,
        ..StructuralLimits::default()
    };
    let (mut runtime, mut store) = setup_with_limits(limits);
    let parent = store.begin(&mut runtime).expect("parent");
    let child = store.begin(&mut runtime).expect("child");
    store
        .add_dependency(&parent, child.domain())
        .expect("dependency");
    let failure = store
        .seal_batch(&mut runtime, vec![parent, child])
        .expect_err("owner count rejected");
    assert_eq!(
        failure.error,
        lkjscript_core::StructuralError::OwnerOverflow
    );
    store
        .discard_batch(&mut runtime, &failure.builders, |_| Result::<_, ()>::Ok(()))
        .expect("builders remain valid");
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn sealed_release_work_is_rejected_before_publication() {
    let limits = StructuralLimits {
        max_release_work: 2,
        ..StructuralLimits::default()
    };
    let (mut runtime, mut store) = setup_with_limits(limits);
    let first = store.begin(&mut runtime).expect("first");
    let second = store.begin(&mut runtime).expect("second");
    let third = store.begin(&mut runtime).expect("third");
    store
        .add_dependency(&first, second.domain())
        .expect("first edge");
    store
        .add_dependency(&second, third.domain())
        .expect("second edge");
    let failure = store
        .seal_batch(&mut runtime, vec![first, second, third])
        .expect_err("release work rejected");
    assert!(matches!(
        failure.error,
        lkjscript_core::StructuralError::LimitExceeded(_)
    ));
    store
        .discard_batch(&mut runtime, &failure.builders[..2], |_| {
            Result::<_, ()>::Ok(())
        })
        .expect("first bounded discard");
    store
        .discard_batch(&mut runtime, &failure.builders[2..], |_| {
            Result::<_, ()>::Ok(())
        })
        .expect("second bounded discard");
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn strong_cycle_is_rejected_with_a_closed_witness_and_can_be_discarded() {
    let (mut runtime, mut store) = setup();
    let first = store.begin(&mut runtime).expect("first");
    let second = store.begin(&mut runtime).expect("second");
    store
        .add_dependency(&first, second.domain())
        .expect("first edge");
    store
        .add_dependency(&second, first.domain())
        .expect("second edge");
    let failure = store
        .seal_batch(&mut runtime, vec![first, second])
        .expect_err("cycle rejected");
    let witness = match &failure.error {
        lkjscript_core::StructuralError::DependencyCycle(witness) => witness,
        other => panic!("unexpected error: {other:?}"),
    };
    assert_eq!(witness.first(), witness.last());
    store
        .discard_batch(&mut runtime, &failure.builders, |_| Result::<_, ()>::Ok(()))
        .expect("discard private builders");
    assert_eq!(runtime.metrics().live_domains, 0);
}
