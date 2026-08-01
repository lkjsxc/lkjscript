#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, SealedRegionMetrics, SealedRegionStore, SemanticTypeIdentity, StructuralLimits,
    StructuralRuntime,
};

#[test]
fn sealed_sharing_counts_regions_not_nodes() {
    let small = share_and_release(8, 128);
    let large = share_and_release(2_048, 128);
    assert_eq!(small.retains, 128);
    assert_eq!(small.releases, 129);
    assert_eq!(small.release_work, 129);
    assert_eq!(large.retains, small.retains);
    assert_eq!(large.releases, small.releases);
    assert_eq!(large.release_work, small.release_work);
}

fn share_and_release(nodes: usize, shares: usize) -> SealedRegionMetrics {
    let limits = StructuralLimits::default();
    let mut runtime = StructuralRuntime::new(limits).expect("runtime");
    let mut store = SealedRegionStore::<u64, ()>::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(71).expect("layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(72).expect("type")),
        limits,
    )
    .expect("sealed store");
    let builder = store.begin(&mut runtime).expect("builder");
    for value in 0..nodes {
        store.allocate(&builder, value as u64).expect("sealed node");
    }
    let mut owners = store
        .seal_batch(&mut runtime, vec![builder])
        .expect("seal region");
    let owner = owners.pop().expect("published owner");
    let retained: Vec<_> = (0..shares)
        .map(|_| store.retain(&owner).expect("coarse retain"))
        .collect();
    for retained in retained {
        let report = store
            .release(&mut runtime, retained, |_| Result::<_, ()>::Ok(()))
            .expect("coarse release");
        assert_eq!(report.objects_released, 0);
    }
    let report = store
        .release(&mut runtime, owner, |_| Result::<_, ()>::Ok(()))
        .expect("final release");
    assert_eq!(report.objects_released, nodes as u64);
    assert_eq!(runtime.metrics().live_domains, 0);
    store.metrics()
}
