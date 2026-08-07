#![allow(clippy::expect_used, clippy::panic)]

use std::num::NonZeroU64;

use lkjscript_core::{LayoutIdentity, SealedRegionStore, SemanticTypeIdentity, StructuralRuntime};

fn setup() -> (StructuralRuntime, SealedRegionStore<u64, u32>) {
    let runtime = StructuralRuntime::new().expect("runtime");
    let store = SealedRegionStore::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(42).expect("nonzero")),
        SemanticTypeIdentity::new(NonZeroU64::new(43).expect("nonzero")),
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
    let weak = store.downgrade(root);
    let retained = store.retain(&owner).expect("retain");
    assert_eq!(
        store
            .release(&mut runtime, retained, |_| Result::<_, ()>::Ok(()))
            .expect("release retained")
            .regions_released,
        0
    );
    let final_report = store
        .release(&mut runtime, owner, |_| Result::<_, ()>::Ok(()))
        .expect("final release");
    assert_eq!(final_report.regions_released, 1);
    assert!(store.upgrade(weak).expect("stale upgrade").is_none());
}

#[test]
fn sealed_dependencies_cross_former_dependency_limit() {
    const DEPENDENCIES: u32 = 4_200;
    let (mut runtime, mut store) = setup();
    let parent = store.begin(&mut runtime).expect("parent");
    store.allocate(&parent, 0).expect("parent value");
    let capacity = usize::try_from(DEPENDENCIES)
        .expect("test dependency count fits usize")
        .checked_add(1)
        .expect("test builder capacity");
    let mut builders = Vec::with_capacity(capacity);
    builders.push(parent);
    for value in 1..=DEPENDENCIES {
        let child = store.begin(&mut runtime).expect("child");
        store
            .allocate(&child, u64::from(value))
            .expect("child value");
        store
            .add_dependency(&builders[0], child.domain())
            .expect("dependency");
        builders.push(child);
    }
    let mut owners = store.seal_batch(&mut runtime, builders).expect("seal DAG");
    let parent_owner = owners.remove(0);
    for child_owner in owners {
        store
            .release(&mut runtime, child_owner, |_| Result::<_, ()>::Ok(()))
            .expect("release explicit child owner");
    }
    let report = store
        .release(&mut runtime, parent_owner, |_| Result::<_, ()>::Ok(()))
        .expect("release parent");
    assert_eq!(report.regions_released, u64::from(DEPENDENCIES) + 1);
    assert_eq!(report.dependency_releases, u64::from(DEPENDENCIES));
    assert_eq!(runtime.metrics().live_domains, 0);
}

#[test]
fn strong_cycle_is_rejected_without_publishing_and_builders_remain_valid() {
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
