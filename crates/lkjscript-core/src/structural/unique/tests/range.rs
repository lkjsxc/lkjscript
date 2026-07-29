use super::*;

#[test]
fn ranged_reads_and_mutation_cover_all_dynamic_layouts() {
    let mut store = store_with(40, 3, 32, 3, 3, 3);
    let vector = store
        .allocate_byte_vector(vec![0, 1, 2, 3, 4])
        .expect("vector");
    let bytes = store.allocate_bytes(vec![5, 6, 7, 8]).expect("bytes");
    let path = store
        .allocate_path(Vec::from(&b"/abc"[..]).into_boxed_slice())
        .expect("path");

    assert_eq!(store.byte_vector_range(vector, 1, 3), Ok(&[1, 2, 3][..]));
    assert_eq!(store.bytes_range(bytes, 2, 2), Ok(&[7, 8][..]));
    assert_eq!(store.path_range(path, 1, 3), Ok(&b"abc"[..]));
    assert_eq!(store.byte_vector_range(vector, 5, 0), Ok(&[][..]));
    store
        .byte_vector_range_mut(vector, 2, 2)
        .expect("mutable range")
        .copy_from_slice(&[9, 8]);
    assert_eq!(store.byte_vector(vector), Ok(&[0, 1, 9, 8, 4][..]));

    store.free_byte_vector(vector).expect("free vector");
    store.free_bytes(bytes).expect("free bytes");
    store.free_path(path).expect("free path");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn range_overflow_and_out_of_bounds_are_structured_and_atomic() {
    let mut store = store_with(41, 1, 8, 1, 1, 2);
    let key = store
        .allocate_byte_vector(vec![1, 2, 3, 4])
        .expect("vector");
    let before_stats = store.stats();
    let before_payload = store.byte_vector(key).expect("read before").to_vec();

    assert_eq!(
        store.byte_vector_range_mut(key, usize::MAX, 1),
        Err(UniqueStoreError::RangeOverflow {
            start: usize::MAX,
            len: 1,
        })
    );
    assert_eq!(
        store.byte_vector_range(key, 2, 3),
        Err(UniqueStoreError::RangeOutOfBounds {
            start: 2,
            len: 3,
            available: 4,
        })
    );
    assert_eq!(
        store.byte_vector_range(key, 5, 0),
        Err(UniqueStoreError::RangeOutOfBounds {
            start: 5,
            len: 0,
            available: 4,
        })
    );
    assert_eq!(store.byte_vector(key), Ok(before_payload.as_slice()));
    assert_eq!(store.stats(), before_stats);
    store.free_byte_vector(key).expect("free vector");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}
