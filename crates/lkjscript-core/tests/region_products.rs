#![allow(clippy::expect_used)]

use std::num::NonZeroU32;

use lkjscript_core::{
    RegionProductArena, RegionProductError, RegionProductKey, RegionProductLimits, RuntimeLayoutId,
};

fn arena() -> RegionProductArena<u64> {
    RegionProductArena::new(RegionProductLimits {
        max_records: NonZeroU32::new(3).expect("record limit"),
        max_fields: NonZeroU32::new(8).expect("field limit"),
    })
    .expect("region product arena")
}

#[test]
fn region_products_are_typed_immutable_and_bulk_owned() {
    let mut arena = arena();
    let identity = RuntimeLayoutId::new([1; 32]);
    let key = arena.publish(identity, vec![10, 20]).expect("publish");
    assert_eq!(arena.field(key, identity, 0), Ok(&10));
    let updated = arena.update(key, identity, 1, 99).expect("update");
    assert_eq!(arena.fields(key, identity), Ok([10, 20].as_slice()));
    assert_eq!(arena.fields(updated, identity), Ok([10, 99].as_slice()));
    assert_eq!(arena.metrics().records, 2);
    assert_eq!(arena.metrics().fields, 4);
    assert_eq!(
        RegionProductKey::from_word(arena.id(), key.to_word()),
        Some(key)
    );
    assert_eq!(
        arena.fields(key, RuntimeLayoutId::new([2; 32])),
        Err(RegionProductError::WrongType)
    );
}

#[test]
fn region_product_keys_are_process_unique_and_arena_scoped() {
    let mut first = arena();
    let second = arena();
    let identity = RuntimeLayoutId::new([3; 32]);
    let key = first.publish(identity, vec![1]).expect("first key");
    assert_eq!(
        RegionProductKey::from_word(second.id(), key.to_word()),
        None
    );
    assert_eq!(
        second.fields(key, identity),
        Err(RegionProductError::InvalidKey)
    );
}

#[test]
fn region_product_limits_fail_without_publishing_a_record() {
    let mut arena = arena();
    let identity = RuntimeLayoutId::new([1; 32]);
    arena.publish(identity, vec![1, 2, 3]).expect("first");
    arena.publish(identity, vec![4, 5, 6]).expect("second");
    assert_eq!(
        arena.publish(identity, vec![7, 8, 9]),
        Err(RegionProductError::Fields)
    );
    assert_eq!(arena.metrics().records, 2);
    assert_eq!(arena.metrics().fields, 6);

    let mut oversized = Vec::with_capacity(9);
    oversized.push(1);
    assert_eq!(
        arena.publish(identity, oversized),
        Err(RegionProductError::Fields)
    );
    assert_eq!(arena.metrics().records, 2);
}
