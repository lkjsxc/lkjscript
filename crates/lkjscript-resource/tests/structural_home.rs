#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, RegionStore, SemanticTypeIdentity, StructuralLimits, StructuralRuntime,
};
use lkjscript_resource::{
    DataOwnerId, GenerationTable, OwnerHomeTable, StructuralOwnerHomeTable, TaskId, WorkerId,
};

#[test]
fn generation_exhaustion_retires_before_identifier_reuse() {
    let mut ids = GenerationTable::<DataOwnerId>::with_max_generation(4, 1).expect("table");
    let first = ids.allocate().expect("first");
    ids.release(first).expect("retire first");
    let second = ids.allocate().expect("second");
    assert_ne!(first.slot, second.slot);
    assert!(!ids.contains(first));
    assert!(ids.contains(second));
}

#[test]
fn owner_epoch_exhaustion_changes_no_home_or_loan_state() {
    let owner = DataOwnerId::new(0, 1);
    let first_home = WorkerId::new(0, 1);
    let second_home = WorkerId::new(1, 1);
    let mut move_table = OwnerHomeTable::with_max_epoch(4, 4, 1).expect("move table");
    move_table.insert(owner, first_home).expect("owner");
    let proof = move_table.prove_no_live_loan(owner).expect("proof");
    assert!(move_table.move_owner(owner, second_home, proof).is_err());
    assert_eq!(move_table.home(owner).expect("unchanged home"), first_home);

    let mut loan_table = OwnerHomeTable::with_max_epoch(4, 4, 2).expect("loan table");
    loan_table.insert(owner, first_home).expect("owner");
    loan_table.begin_loan(owner).expect("loan");
    assert!(loan_table.end_loan(owner).is_err());
    assert!(loan_table.prove_no_live_loan(owner).is_err());
}

#[test]
fn structural_domains_use_exact_owner_homes_and_bounded_release() {
    let mut runtime = StructuralRuntime::new(StructuralLimits::default()).expect("runtime");
    let mut store = RegionStore::<u8, u8>::new(
        runtime.identity(),
        LayoutIdentity::new(NonZeroU64::new(92).expect("layout")),
        SemanticTypeIdentity::new(NonZeroU64::new(93).expect("type")),
        StructuralLimits::default(),
    )
    .expect("store");
    let region = store.create(&mut runtime).expect("region");
    let domain = region.domain();
    let first_home = WorkerId::new(0, 1);
    let second_home = WorkerId::new(1, 1);
    let mut homes = StructuralOwnerHomeTable::new(8, 2);
    homes.register(domain, first_home).expect("register");
    assert_eq!(homes.home(domain).expect("home"), first_home);
    homes.begin_loan(domain).expect("loan");
    assert!(homes.prove_no_live_loan(domain).is_err());
    homes.end_loan(domain).expect("end loan");
    let proof = homes.prove_no_live_loan(domain).expect("proof");
    homes.move_home(domain, second_home, proof).expect("move");
    homes
        .remote_release(domain, TaskId::new(3, 1))
        .expect("remote release");
    let mut releases = homes.drain_releases(second_home, 1).expect("drain");
    assert_eq!(releases.len(), 1);
    assert_eq!(releases[0].domain, domain);
    assert!(homes.prove_no_live_loan(domain).is_err());
    homes
        .complete_release(releases.remove(0), || {
            store
                .release(&mut runtime, &region, |_| Result::<_, ()>::Ok(()))
                .map(|_| ())
                .map_err(|error| {
                    lkjscript_resource::ResourceError::new("region", error.to_string())
                })
        })
        .expect("complete release");
    homes.validate_empty().expect("empty");
    runtime.validate().expect("runtime empty");
}
