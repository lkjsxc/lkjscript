use super::*;

#[test]
fn resize_growth_and_shrink_update_exact_retained_metrics() {
    let mut store = store_with(50, 1, 32, 1, 1, 3);
    let key = store
        .allocate_byte_vector(bytes_with_capacity(2, &[1, 2]))
        .expect("vector");
    assert_eq!(store.stats().live_bytes, 2);
    assert_eq!(store.stats().allocated_bytes, 2);

    store
        .resize_byte_vector(key, 5, 9)
        .expect("transactional growth");
    assert_eq!(store.byte_vector(key), Ok(&[1, 2, 9, 9, 9][..]));
    let retained_after_growth = store.stats().live_bytes;
    assert!(retained_after_growth >= 5);
    assert_eq!(store.stats().allocated_bytes, retained_after_growth);
    assert_eq!(store.stats().peak_live_bytes, retained_after_growth);
    assert_eq!(store.stats().allocations, 1);

    let before_shrink = store.stats();
    store.resize_byte_vector(key, 1, 0).expect("length shrink");
    assert_eq!(store.byte_vector(key), Ok(&[1][..]));
    assert_eq!(store.stats(), before_shrink);

    let retained_len = usize::try_from(retained_after_growth).expect("retained length fits");
    store
        .resize_byte_vector(key, retained_len, 7)
        .expect("growth within retained capacity");
    assert_eq!(store.stats(), before_shrink);
    store.free_byte_vector(key).expect("free vector");
    assert_eq!(store.stats().live_bytes, 0);
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn resize_limit_and_capacity_failures_preserve_payload_and_metrics() {
    let mut limited = store_with(51, 1, 4, 1, 1, 2);
    let key = limited
        .allocate_byte_vector(bytes_with_capacity(2, &[1, 2]))
        .expect("vector");
    let pointer = limited.byte_vector(key).expect("pointer before").as_ptr();
    let before = limited.stats();
    assert_eq!(
        limited.resize_byte_vector(key, 5, 0),
        Err(UniqueStoreError::ByteLimit)
    );
    assert_eq!(limited.byte_vector(key), Ok(&[1, 2][..]));
    assert_eq!(
        limited.byte_vector(key).expect("pointer after").as_ptr(),
        pointer
    );
    assert_eq!(limited.stats(), before);
    limited.free_byte_vector(key).expect("free limited");

    let mut capacity = store_with(52, 1, u64::MAX, 1, 1, 2);
    let key = capacity.allocate_byte_vector(vec![3]).expect("vector");
    let pointer = capacity.byte_vector(key).expect("pointer before").as_ptr();
    let before = capacity.stats();
    assert_eq!(
        capacity.resize_byte_vector(key, usize::MAX, 0),
        Err(UniqueStoreError::StorageCapacity)
    );
    assert_eq!(capacity.byte_vector(key), Ok(&[3][..]));
    assert_eq!(
        capacity.byte_vector(key).expect("pointer after").as_ptr(),
        pointer
    );
    assert_eq!(capacity.stats(), before);
    capacity.free_byte_vector(key).expect("free capacity");
    assert_eq!(capacity.assert_no_leaks(), Ok(()));
}

#[test]
fn little_endian_u32_access_is_exact_and_failure_atomic() {
    let mut store = store_with(54, 1, 16, 1, 1, 2);
    let key = store.allocate_byte_vector(vec![9; 8]).expect("vector");
    let metrics = store.stats();

    store
        .write_byte_vector_u32_little_endian(key, 2, 0x7856_3412)
        .expect("checked word write");
    assert_eq!(
        store.byte_vector(key),
        Ok(&[9, 9, 0x12, 0x34, 0x56, 0x78, 9, 9][..])
    );
    assert_eq!(
        store.read_byte_vector_u32_little_endian(key, 2),
        Ok(0x7856_3412)
    );
    let before = store.byte_vector(key).expect("before rejection").to_vec();
    assert_eq!(
        store.write_byte_vector_u32_little_endian(key, 5, u32::MAX),
        Err(UniqueStoreError::RangeOutOfBounds {
            start: 5,
            len: 4,
            available: 8,
        })
    );
    assert_eq!(store.byte_vector(key), Ok(before.as_slice()));
    assert_eq!(store.stats(), metrics);
    store.free_byte_vector(key).expect("free vector");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn fill_and_overlapping_copy_are_memmove_like_and_atomic() {
    let mut store = store_with(53, 1, 16, 1, 1, 2);
    let key = store
        .allocate_byte_vector(vec![0, 1, 2, 3, 4, 5])
        .expect("vector");
    let metrics = store.stats();

    store
        .copy_byte_vector_range(key, 0, 2, 4)
        .expect("forward overlapping copy");
    assert_eq!(store.byte_vector(key), Ok(&[0, 1, 0, 1, 2, 3][..]));
    store
        .copy_byte_vector_range(key, 2, 0, 4)
        .expect("reverse overlapping copy");
    assert_eq!(store.byte_vector(key), Ok(&[0, 1, 2, 3, 2, 3][..]));
    store
        .fill_byte_vector_range(key, 1, 3, 8)
        .expect("range fill");
    assert_eq!(store.byte_vector(key), Ok(&[0, 8, 8, 8, 2, 3][..]));

    let before_payload = store.byte_vector(key).expect("before failure").to_vec();
    assert_eq!(
        store.copy_byte_vector_range(key, 0, usize::MAX, 2),
        Err(UniqueStoreError::RangeOverflow {
            start: usize::MAX,
            len: 2,
        })
    );
    assert_eq!(
        store.fill_byte_vector_range(key, 5, 2, 0),
        Err(UniqueStoreError::RangeOutOfBounds {
            start: 5,
            len: 2,
            available: 6,
        })
    );
    assert_eq!(store.byte_vector(key), Ok(before_payload.as_slice()));
    assert_eq!(store.stats(), metrics);
    store.fill_byte_vector(key, 4).expect("whole fill");
    assert_eq!(store.byte_vector(key), Ok(&[4; 6][..]));
    store.free_byte_vector(key).expect("free vector");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}
