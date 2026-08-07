#![allow(clippy::expect_used)]

use lkjscript_core::{RegionProductArena, RegionProductError, RegionProductKey, RuntimeLayoutId};

fn arena() -> RegionProductArena<u64> {
    RegionProductArena::new().expect("region product arena")
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
fn region_products_cross_former_record_and_field_limits() {
    const RECORDS: u64 = 17_000;
    const FIELDS_PER_RECORD: usize = 16;
    let mut arena = arena();
    let identity = RuntimeLayoutId::new([4; 32]);
    let mut last = None;
    for value in 0..RECORDS {
        last = Some(
            arena
                .publish(identity, vec![value; FIELDS_PER_RECORD])
                .expect("publish beyond former limits"),
        );
    }
    assert_eq!(arena.metrics().records, RECORDS);
    assert_eq!(arena.metrics().fields, RECORDS * FIELDS_PER_RECORD as u64);
    assert_eq!(
        arena.field(last.expect("last record"), identity, FIELDS_PER_RECORD - 1),
        Ok(&(RECORDS - 1))
    );
}

#[test]
fn invalid_update_fails_without_publishing_a_record() {
    let mut first = arena();
    let second = arena();
    let identity = RuntimeLayoutId::new([3; 32]);
    let key = first.publish(identity, vec![1]).expect("first key");
    assert_eq!(
        RegionProductKey::from_word(second.id(), key.to_word()),
        None
    );
    let before = first.metrics();
    assert_eq!(
        first.update(key, identity, 3, 9),
        Err(RegionProductError::FieldOutOfRange)
    );
    assert_eq!(first.metrics(), before);
}
