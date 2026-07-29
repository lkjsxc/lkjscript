#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, RegionStore, SemanticTypeIdentity, StructuralLimits, StructuralRuntime,
};

fn runtime() -> StructuralRuntime {
    StructuralRuntime::new(StructuralLimits::default()).expect("runtime")
}

fn region_store(runtime: &StructuralRuntime) -> RegionStore<u64, u8> {
    RegionStore::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(21).expect("nonzero")),
        SemanticTypeIdentity::new(NonZeroU64::new(31).expect("nonzero")),
        StructuralLimits::default(),
    )
    .expect("region store")
}

#[test]
fn ordinary_region_cycles_release_without_internal_graph_walk() {
    let mut runtime = runtime();
    let mut store = region_store(&runtime);
    let owner = store.create(&mut runtime).expect("region");
    let first = store.allocate(&owner, 10).expect("first");
    let second = store.allocate(&owner, 20).expect("second");
    store
        .add_internal_edge(&owner, first, second)
        .expect("first edge");
    store
        .add_internal_edge(&owner, second, first)
        .expect("cycle edge");
    store.register_drop(&owner, 1).expect("drop one");
    store.register_drop(&owner, 2).expect("drop two");
    store.validate(&runtime).expect("valid before release");

    let mut drops = Vec::new();
    let report = store
        .release(&mut runtime, &owner, |drop| {
            drops.push(drop);
            Result::<_, ()>::Ok(())
        })
        .expect("release");
    assert_eq!(drops, vec![2, 1]);
    assert_eq!(report.domains_released, 1);
    assert_eq!(report.objects_released, 2);
    assert_eq!(store.metrics().release_work, 1);
    assert_eq!(store.metrics().chunks_created, 1);
    assert_eq!(store.metrics().internal_edges, 2);
    assert!(store.get(first).is_err());
    runtime.validate().expect("runtime valid");
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn child_regions_reset_and_stale_roots_are_exact() {
    let mut runtime = runtime();
    let mut store = region_store(&runtime);
    let parent = store.create(&mut runtime).expect("parent");
    let child = store.create(&mut runtime).expect("child");
    let old = store.allocate(&parent, 7).expect("old root");
    let child_root = store.allocate(&child, 8).expect("child root");
    store.attach(&parent, child).expect("attach child");

    let report = store
        .reset(&mut runtime, &parent, |_| Result::<_, ()>::Ok(()))
        .expect("reset");
    assert_eq!(report.domains_released, 1);
    assert_eq!(report.objects_released, 2);
    assert!(store.get(old).is_err());
    assert!(store.get(child_root).is_err());
    let new = store.allocate(&parent, 9).expect("new root");
    assert_eq!(*store.get(new).expect("new value"), 9);
    assert_ne!(old.root().generation(), new.root().generation());

    store
        .release(&mut runtime, &parent, |_| Result::<_, ()>::Ok(()))
        .expect("release parent");
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn aggregate_drop_failures_and_release_work_are_preflighted() {
    let limits = StructuralLimits {
        max_release_work: 2,
        max_drop_entries: 2,
        ..StructuralLimits::default()
    };
    let mut runtime = StructuralRuntime::new(limits).expect("runtime");
    let mut store = RegionStore::<u64, u8>::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(71).expect("layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(72).expect("type")),
        limits,
    )
    .expect("store");
    let parent = store.create(&mut runtime).expect("parent");
    let child = store.create(&mut runtime).expect("child");
    store.register_drop(&parent, 1).expect("parent one");
    store.register_drop(&parent, 2).expect("parent two");
    store.register_drop(&child, 3).expect("child one");
    store.register_drop(&child, 4).expect("child two");
    store.attach(&parent, child).expect("attach");
    let report = store
        .release(&mut runtime, &parent, Err::<(), _>)
        .expect("bounded release");
    assert_eq!(report.domains_released, 2);
    assert_eq!(report.drop_failures, vec![4, 3, 2, 1]);

    let root = store.create(&mut runtime).expect("root");
    let branch = store.create(&mut runtime).expect("branch");
    let leaf = store.create(&mut runtime).expect("leaf");
    store.attach(&branch, leaf).expect("branch leaf");
    let (error, branch) = store.attach(&root, branch).expect_err("work bound");
    assert!(matches!(
        error,
        lkjscript_core::StructuralError::LimitExceeded(_)
    ));
    store
        .release(&mut runtime, &root, |_| Result::<_, ()>::Ok(()))
        .expect("root release");
    store
        .release(&mut runtime, &branch, |_| Result::<_, ()>::Ok(()))
        .expect("branch release");
}

#[test]
fn live_loan_and_dependency_cycle_fail_before_mutation() {
    let mut runtime = runtime();
    let mut store = region_store(&runtime);
    let first = store.create(&mut runtime).expect("first");
    let second = store.create(&mut runtime).expect("second");
    store.begin_loan(&first).expect("loan");
    assert!(store
        .release(&mut runtime, &first, |_| Result::<_, ()>::Ok(()))
        .is_err());
    let root = store.allocate(&first, 1).expect("root");
    assert_eq!(*store.get(root).expect("value"), 1);
    store.end_loan(&first).expect("end loan");
    store.attach(&first, second).expect("attach");
    store
        .release(&mut runtime, &first, |_| Result::<_, ()>::Ok(()))
        .expect("release graph");
    assert_eq!(runtime.metrics().live_domains, 0);
}
