#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, RegionOwner, RegionStore, RootKey, SemanticTypeIdentity,
    StructuralRootOwnership, StructuralRootState, StructuralRootTable, StructuralRootTableError,
    StructuralRuntime, Value,
};

const LAYOUT: u64 = 91;
const SEMANTIC_TYPE: u64 = 92;

fn identity(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("nonzero identity")
}

fn new_runtime() -> StructuralRuntime {
    StructuralRuntime::new().expect("runtime")
}

fn new_store(runtime: &StructuralRuntime) -> RegionStore<u64, ()> {
    RegionStore::new(
        runtime.identity(),
        LayoutIdentity::new(identity(LAYOUT)),
        SemanticTypeIdentity::new(identity(SEMANTIC_TYPE)),
    )
    .expect("region store")
}

fn allocate_root(
    runtime: &mut StructuralRuntime,
    store: &mut RegionStore<u64, ()>,
    value: u64,
) -> (RegionOwner<u64, ()>, RootKey) {
    let owner = store.create(runtime).expect("region");
    let root = store.allocate(&owner, value).expect("root").root();
    (owner, root)
}

fn release(
    runtime: &mut StructuralRuntime,
    store: &mut RegionStore<u64, ()>,
    owner: &RegionOwner<u64, ()>,
) {
    store
        .release(runtime, owner, |_| Result::<_, ()>::Ok(()))
        .expect("release region");
}

#[test]
fn compact_roots_move_borrow_reuse_and_round_trip_through_value() {
    let mut runtime = new_runtime();
    let mut store = new_store(&runtime);
    let (owner, root) = allocate_root(&mut runtime, &mut store, 7);
    let mut table = StructuralRootTable::new(runtime.identity()).expect("root table");
    let key = table
        .publish(root, StructuralRootOwnership::Owned)
        .expect("publish");
    assert_eq!(
        Value::from_structural_root(key).as_structural_root(),
        Some(key)
    );
    assert_eq!(table.state(key), Ok(StructuralRootState::Owned));

    let first = table.borrow_shared(key).expect("first shared loan");
    let second = table.borrow_shared(key).expect("second shared loan");
    assert_eq!(table.state(key), Ok(StructuralRootState::BorrowedShared));
    assert_eq!(
        table.borrow_exclusive(key),
        Err(StructuralRootTableError::BorrowConflict)
    );
    assert_eq!(
        table.drop_owned(key),
        Err(StructuralRootTableError::LiveLoan)
    );
    table.end_borrow(second).expect("end second");
    table.end_borrow(first).expect("end first");
    let exclusive = table.borrow_exclusive(key).expect("exclusive loan");
    assert_eq!(
        table.borrow_shared(key),
        Err(StructuralRootTableError::BorrowConflict)
    );
    table.end_borrow(exclusive).expect("end exclusive");

    assert_eq!(table.take_owned(key), Ok(root));
    let replacement = table
        .publish(root, StructuralRootOwnership::Owned)
        .expect("republish");
    assert_eq!(replacement.slot(), key.slot());
    assert_ne!(replacement, key);
    assert_eq!(table.drop_owned(replacement), Ok(root));
    table.assert_no_live_roots().expect("empty table");
    release(&mut runtime, &mut store, &owner);
}

#[test]
fn roots_cross_former_root_and_loan_limits() {
    const COUNT: usize = 65_537;
    let mut runtime = new_runtime();
    let mut store = new_store(&runtime);
    let owner = store.create(&mut runtime).expect("region");
    let mut table = StructuralRootTable::new(runtime.identity()).expect("table");
    let mut keys = Vec::with_capacity(COUNT);
    for value in 0..COUNT {
        let root = store.allocate(&owner, value as u64).expect("root").root();
        keys.push(
            table
                .publish(root, StructuralRootOwnership::Owned)
                .expect("publish"),
        );
    }
    let loans = keys
        .iter()
        .copied()
        .map(|key| table.borrow_shared(key).expect("loan"))
        .collect::<Vec<_>>();
    for loan in loans {
        table.end_borrow(loan).expect("end loan");
    }
    for key in keys {
        table.drop_owned(key).expect("drop root");
    }
    table.assert_no_live_roots().expect("empty table");
    release(&mut runtime, &mut store, &owner);
}

#[test]
fn root_table_rejects_foreign_and_duplicate_owners_without_mutation() {
    let mut runtime = new_runtime();
    let mut other_runtime = new_runtime();
    let mut store = new_store(&runtime);
    let mut other_store = new_store(&other_runtime);
    let (owner, root) = allocate_root(&mut runtime, &mut store, 1);
    let (other_owner, other_root) = allocate_root(&mut other_runtime, &mut other_store, 2);
    let mut table = StructuralRootTable::new(runtime.identity()).expect("table");
    assert_eq!(
        table.publish(other_root, StructuralRootOwnership::Owned),
        Err(StructuralRootTableError::WrongRuntime)
    );
    let key = table
        .publish(root, StructuralRootOwnership::Owned)
        .expect("publish");
    let before = table.stats();
    assert_eq!(
        table.publish(root, StructuralRootOwnership::Owned),
        Err(StructuralRootTableError::DuplicateOwner)
    );
    assert_eq!(table.stats(), before);
    table.drop_owned(key).expect("drop owner");
    release(&mut runtime, &mut store, &owner);
    release(&mut other_runtime, &mut other_store, &other_owner);
}
