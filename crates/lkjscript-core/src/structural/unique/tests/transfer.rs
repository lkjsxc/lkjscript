use super::*;

#[test]
fn freeze_and_unique_dynamic_thaw_transfer_backing_without_copy() {
    let mut store = store(20);
    let source = bytes_with_capacity(8, &[1, 2, 3]);
    let source_pointer = source.as_ptr();
    let vector = store
        .allocate_byte_vector(source)
        .expect("vector allocation");
    let opaque_word = vector.opaque_word();
    assert_eq!(
        store.byte_vector(vector).expect("vector read").as_ptr(),
        source_pointer
    );
    let before = store.stats();

    let bytes = store.freeze_byte_vector(vector).expect("zero-copy freeze");
    assert_eq!(bytes.opaque_word(), opaque_word);
    assert_eq!(store.bytes(bytes), Ok(&[1, 2, 3][..]));
    assert_eq!(
        store.bytes(bytes).expect("bytes read").as_ptr(),
        source_pointer
    );
    assert_eq!(
        store.byte_vector(vector),
        Err(UniqueStoreError::WrongLayout {
            expected: UniqueLayout::ByteVector,
            actual: UniqueLayout::Bytes,
        })
    );
    assert_eq!(store.stats().wrong_layout_failures, 1);
    assert_eq!(store.stats().live_bytes, before.live_bytes);
    assert_eq!(store.stats().allocated_bytes, before.allocated_bytes);

    let vector = store
        .thaw_dynamic_bytes(bytes)
        .expect("uniquely owned dynamic thaw");
    assert_eq!(vector.opaque_word(), opaque_word);
    assert_eq!(store.byte_vector(vector), Ok(&[1, 2, 3][..]));
    assert_eq!(
        store.byte_vector(vector).expect("thawed read").as_ptr(),
        source_pointer
    );
    assert_eq!(
        store.bytes(bytes),
        Err(UniqueStoreError::WrongLayout {
            expected: UniqueLayout::Bytes,
            actual: UniqueLayout::ByteVector,
        })
    );
    assert_eq!(store.stats().transfers, 2);
    assert_eq!(store.stats().wrong_layout_failures, 2);
    assert_eq!(store.stats().allocations, 1);
    assert_eq!(store.stats().frees, 0);
    store
        .free_byte_vector(vector)
        .expect("transferred owner free");
    assert_eq!(store.stats().frees, 1);
    assert_eq!(store.assert_no_leaks(), Ok(()));
}
