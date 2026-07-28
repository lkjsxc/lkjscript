use super::*;

#[test]
fn object_and_byte_limits_reject_without_publication() {
    let mut objects = store_with(10, 1, 16, 2, 4, 3);
    let first = objects.allocate_byte_vector(vec![1]).expect("first object");
    let before = objects.stats();
    assert_eq!(
        objects.allocate_bytes(vec![2]),
        Err(UniqueStoreError::ObjectLimit)
    );
    assert_eq!(objects.stats(), before);
    assert_eq!(objects.slot_count(), 1);
    objects.free_byte_vector(first).expect("object free");

    let mut bytes = store_with(11, 2, 3, 2, 4, 3);
    let first = bytes
        .allocate_byte_vector(bytes_with_capacity(3, &[1]))
        .expect("full byte budget");
    let before = bytes.stats();
    assert_eq!(
        bytes.allocate_path(vec![2].into_boxed_slice()),
        Err(UniqueStoreError::ByteLimit)
    );
    assert_eq!(bytes.stats(), before);
    assert_eq!(bytes.slot_count(), 1);
    bytes.free_byte_vector(first).expect("byte budget release");
}

#[test]
fn allocation_arithmetic_failure_keeps_free_slot_unpublished() {
    let mut store = store_with(12, 1, 8, 1, u64::MAX, 3);
    let first = store
        .allocate_byte_vector(vec![1])
        .expect("first allocation");
    store.free_byte_vector(first).expect("first release");
    store.stats.allocated_bytes = u64::MAX;
    let before = store.stats();
    let free_head = store.free_head;
    assert_eq!(
        store.allocate_byte_vector(vec![2]),
        Err(UniqueStoreError::ArithmeticOverflow)
    );
    assert_eq!(store.stats(), before);
    assert_eq!(store.free_head, free_head);
    assert_eq!(store.slot_count(), 1);
}

#[test]
fn freed_slots_are_reused_in_lifo_order_with_new_generations() {
    let mut store = store_with(13, 2, 8, 2, 6, 4);
    let first = store.allocate_byte_vector(vec![1]).expect("first");
    let second = store.allocate_byte_vector(vec![2]).expect("second");
    store.free_byte_vector(first).expect("free first");
    store.free_byte_vector(second).expect("free second");
    let third = store.allocate_byte_vector(vec![3]).expect("third");
    let fourth = store.allocate_byte_vector(vec![4]).expect("fourth");
    assert_eq!(third.raw().index, second.raw().index);
    assert_eq!(fourth.raw().index, first.raw().index);
    assert_ne!(third.raw().generation, second.raw().generation);
    assert_ne!(fourth.raw().generation, first.raw().generation);
    assert_eq!(store.stats().reused_slots, 2);
    assert_eq!(store.byte_vector(second), Err(UniqueStoreError::StaleKey));
    store.free_byte_vector(third).expect("free third");
    store.free_byte_vector(fourth).expect("free fourth");
}

#[test]
fn generation_exhaustion_retires_slot_without_wrap() {
    let mut store = store_with(14, 1, 8, 1, 4, 2);
    let first = store.allocate_byte_vector(vec![1]).expect("generation one");
    store.free_byte_vector(first).expect("free generation one");
    let second = store.allocate_byte_vector(vec![2]).expect("generation two");
    assert_ne!(first.raw().generation, second.raw().generation);
    store.free_byte_vector(second).expect("retiring free");
    assert_eq!(store.stats().retired_slots, 1);
    assert_eq!(store.slot_count(), 1);
    let before = store.stats();
    assert_eq!(
        store.allocate_byte_vector(vec![3]),
        Err(UniqueStoreError::SlotLimit)
    );
    assert_eq!(store.stats(), before);
}

#[test]
fn invalid_limit_relationships_are_rejected() {
    assert_eq!(
        UniqueStoreLimits::new(2, 8, 1, 2, 1),
        Err(InvalidUniqueStoreLimits::ObjectsExceedSlots)
    );
    assert_eq!(
        UniqueStoreLimits::new(1, 8, 1, 2, 0),
        Err(InvalidUniqueStoreLimits::ZeroGeneration)
    );
}
