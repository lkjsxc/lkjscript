#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, SealedRegionStore, SemanticTypeIdentity, StructuralLimits,
    StructuralRootOwnership, StructuralRootState, StructuralRootTable, StructuralRootTableError,
    StructuralRootTableLimits, StructuralRuntime,
};

#[test]
fn sealed_root_leases_share_and_release_only_through_domain_owners() {
    let limits = StructuralLimits::default();
    let mut runtime = StructuralRuntime::new(limits).expect("runtime");
    let mut store = SealedRegionStore::<u64, ()>::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(51).expect("layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(52).expect("type")),
        limits,
    )
    .expect("sealed store");
    let builder = store.begin(&mut runtime).expect("builder");
    store.allocate(&builder, 9).expect("value");
    let mut owners = store.seal_batch(&mut runtime, vec![builder]).expect("seal");
    let owner = owners.pop().expect("owner");
    let retained = store.retain(&owner).expect("retain");
    let root = store.root(&owner, 0).expect("root").root();
    let mut table =
        StructuralRootTable::new(runtime.identity(), StructuralRootTableLimits::default())
            .expect("root table");
    let first = table
        .publish(root, StructuralRootOwnership::SealedShared)
        .expect("first lease");
    let second = table
        .publish(root, StructuralRootOwnership::SealedShared)
        .expect("second lease");
    let loan = table.borrow_shared(first).expect("shared borrow");
    assert_eq!(table.state(first), Ok(StructuralRootState::BorrowedShared));
    assert_eq!(
        table.borrow_exclusive(first),
        Err(StructuralRootTableError::BorrowConflict)
    );
    table.end_borrow(loan).expect("end borrow");
    assert_eq!(table.release_sealed(first), Ok(root));
    let first_report = store
        .release(&mut runtime, retained, |_| Result::<_, ()>::Ok(()))
        .expect("release retained owner");
    assert_eq!(first_report.regions_released, 0);
    assert_eq!(table.release_sealed(second), Ok(root));
    let final_report = store
        .release(&mut runtime, owner, |_| Result::<_, ()>::Ok(()))
        .expect("release final owner");
    assert_eq!(final_report.regions_released, 1);
    assert_eq!(table.stats().roots_released, 2);
    table.assert_no_live_roots().expect("empty root table");
    assert_eq!(runtime.metrics().live_domains, 0);
}
