#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{LayoutIdentity, RegionStore, SemanticTypeIdentity, StructuralRuntime};

fn runtime() -> StructuralRuntime {
    StructuralRuntime::new().expect("runtime")
}

fn region_store(runtime: &StructuralRuntime) -> RegionStore<u64, u32> {
    RegionStore::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(21).expect("nonzero")),
        SemanticTypeIdentity::new(NonZeroU64::new(31).expect("nonzero")),
    )
    .expect("region store")
}

#[test]
fn ordinary_region_crosses_former_edge_and_drop_limits() {
    const COUNT: u32 = 5_000;
    let mut runtime = runtime();
    let mut store = region_store(&runtime);
    let owner = store.create(&mut runtime).expect("region");
    let anchor = store.allocate(&owner, 0).expect("anchor");
    for value in 1..=COUNT {
        let object = store.allocate(&owner, u64::from(value)).expect("object");
        store
            .add_internal_edge(&owner, anchor, object)
            .expect("edge");
        store.register_drop(&owner, value).expect("drop");
    }
    store
        .add_internal_edge(&owner, anchor, anchor)
        .expect("ordinary cyclic edge");
    store.validate(&runtime).expect("valid before release");

    let mut drops = Vec::new();
    let report = store
        .release(&mut runtime, &owner, |drop| {
            drops.push(drop);
            Result::<_, ()>::Ok(())
        })
        .expect("release");
    assert_eq!(drops.len(), COUNT as usize);
    assert_eq!(drops.first(), Some(&COUNT));
    assert_eq!(drops.last(), Some(&1));
    assert_eq!(report.objects_released, u64::from(COUNT) + 1);
    assert_eq!(store.metrics().release_work, 1);
    assert_eq!(store.metrics().internal_edges, u64::from(COUNT) + 1);
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
fn live_loan_fails_without_mutation() {
    let mut runtime = runtime();
    let mut store = region_store(&runtime);
    let owner = store.create(&mut runtime).expect("owner");
    store.begin_loan(&owner).expect("loan");
    assert!(store
        .release(&mut runtime, &owner, |_| Result::<_, ()>::Ok(()))
        .is_err());
    let root = store.allocate(&owner, 1).expect("still reusable");
    assert_eq!(*store.get(root).expect("value"), 1);
    store.end_loan(&owner).expect("end loan");
    store
        .release(&mut runtime, &owner, |_| Result::<_, ()>::Ok(()))
        .expect("release");
}
