#![allow(clippy::expect_used)]

use std::num::NonZeroU64;

use lkjscript_core::{
    LayoutIdentity, RegionOwner, RegionStore, RootKey, SemanticTypeIdentity, StructuralLimits,
    StructuralRootOwnership, StructuralRootState, StructuralRootTable, StructuralRootTableError,
    StructuralRootTableLimits, StructuralRuntime, Value,
};

const LAYOUT: u64 = 91;
const SEMANTIC_TYPE: u64 = 92;

fn identity(value: u64) -> NonZeroU64 {
    NonZeroU64::new(value).expect("nonzero identity")
}

fn new_runtime() -> StructuralRuntime {
    StructuralRuntime::new(StructuralLimits::default()).expect("runtime")
}

fn new_store(runtime: &StructuralRuntime) -> RegionStore<u64, ()> {
    RegionStore::new(
        runtime.identity(),
        LayoutIdentity::new(identity(LAYOUT)),
        SemanticTypeIdentity::new(identity(SEMANTIC_TYPE)),
        StructuralLimits::default(),
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
    let mut table =
        StructuralRootTable::new(runtime.identity(), StructuralRootTableLimits::default())
            .expect("root table");
    let key = table
        .publish(root, StructuralRootOwnership::Owned)
        .expect("publish");
    let value = Value::from_structural_root(key);
    assert_eq!(value.as_structural_root(), Some(key));
    assert_eq!(table.state(key), Ok(StructuralRootState::Owned));
    assert_eq!(
        table.root(
            key,
            LayoutIdentity::new(identity(LAYOUT)),
            SemanticTypeIdentity::new(identity(SEMANTIC_TYPE)),
        ),
        Ok(root)
    );

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
    assert_eq!(table.state(key), Ok(StructuralRootState::BorrowedExclusive));
    assert_eq!(
        table.borrow_shared(key),
        Err(StructuralRootTableError::BorrowConflict)
    );
    table.end_borrow(exclusive).expect("end exclusive");

    assert_eq!(table.take_owned(key), Ok(root));
    assert_eq!(table.state(key), Ok(StructuralRootState::Moved));
    let replacement = table
        .publish(root, StructuralRootOwnership::Owned)
        .expect("republish moved owner");
    assert_ne!(replacement, key);
    assert_eq!(replacement.slot(), key.slot());
    assert_eq!(table.drop_owned(replacement), Ok(root));
    assert_eq!(table.state(replacement), Ok(StructuralRootState::Dropped));
    table.assert_no_live_roots().expect("empty table");
    assert_eq!(table.stats().root_slots_reused, 1);
    release(&mut runtime, &mut store, &owner);
}

#[test]
fn root_table_rejects_runtime_layout_type_and_duplicate_owner() {
    let mut runtime = new_runtime();
    let mut other_runtime = new_runtime();
    let mut store = new_store(&runtime);
    let mut other_store = new_store(&other_runtime);
    let (owner, root) = allocate_root(&mut runtime, &mut store, 1);
    let (other_owner, other_root) = allocate_root(&mut other_runtime, &mut other_store, 2);
    let mut table =
        StructuralRootTable::new(runtime.identity(), StructuralRootTableLimits::default())
            .expect("table");
    assert_eq!(
        table.publish(other_root, StructuralRootOwnership::Owned),
        Err(StructuralRootTableError::WrongRuntime)
    );
    let key = table
        .publish(root, StructuralRootOwnership::Owned)
        .expect("publish");
    assert_eq!(
        table.publish(root, StructuralRootOwnership::Owned),
        Err(StructuralRootTableError::DuplicateOwner)
    );
    assert_eq!(
        table.root(
            key,
            LayoutIdentity::new(identity(LAYOUT + 1)),
            SemanticTypeIdentity::new(identity(SEMANTIC_TYPE)),
        ),
        Err(StructuralRootTableError::WrongLayout)
    );
    assert_eq!(
        table.root(
            key,
            LayoutIdentity::new(identity(LAYOUT)),
            SemanticTypeIdentity::new(identity(SEMANTIC_TYPE + 1)),
        ),
        Err(StructuralRootTableError::WrongSemanticType)
    );
    table.drop_owned(key).expect("drop root table owner");
    release(&mut runtime, &mut store, &owner);
    release(&mut other_runtime, &mut other_store, &other_owner);
}

#[test]
fn root_and_loan_limits_retire_without_partial_state() {
    let limits = StructuralRootTableLimits {
        max_roots: 1,
        max_loans: 1,
        max_generation: 2,
    };
    let mut runtime = new_runtime();
    let mut store = new_store(&runtime);
    let (first_owner, first_root) = allocate_root(&mut runtime, &mut store, 1);
    let (second_owner, second_root) = allocate_root(&mut runtime, &mut store, 2);
    let mut table = StructuralRootTable::new(runtime.identity(), limits).expect("table");
    let first = table
        .publish(first_root, StructuralRootOwnership::Owned)
        .expect("first root");
    assert_eq!(
        table.publish(second_root, StructuralRootOwnership::Owned),
        Err(StructuralRootTableError::LimitExceeded(
            lkjscript_core::StructuralRootTableLimit::Roots,
        ))
    );
    let loan = table.borrow_shared(first).expect("first loan");
    assert_eq!(
        table.borrow_shared(first),
        Err(StructuralRootTableError::LimitExceeded(
            lkjscript_core::StructuralRootTableLimit::Loans,
        ))
    );
    assert_eq!(table.state(first), Ok(StructuralRootState::BorrowedShared));
    table.end_borrow(loan).expect("end loan");
    let reused_loan = table.borrow_shared(first).expect("reuse loan slot");
    table.end_borrow(reused_loan).expect("retire loan slot");
    assert_eq!(table.stats().loan_slots_retired, 1);
    table.drop_owned(first).expect("drop first");
    let second = table
        .publish(second_root, StructuralRootOwnership::Owned)
        .expect("reuse root slot");
    table.drop_owned(second).expect("retire root slot");
    assert_eq!(table.state(second), Ok(StructuralRootState::Retired));
    assert_eq!(table.stats().root_slots_retired, 1);
    table.assert_no_live_roots().expect("empty table");
    release(&mut runtime, &mut store, &first_owner);
    release(&mut runtime, &mut store, &second_owner);
}
