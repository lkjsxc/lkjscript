use super::*;

#[test]
fn dynamic_bytes_clones_publish_independent_exact_owners() {
    let mut store = store_with(60, 3, 16, 3, 3, 3);
    let source = store
        .allocate_bytes(bytes_with_capacity(6, &[1, 2, 3]))
        .expect("source bytes");
    let source_pointer = store.bytes(source).expect("source pointer").as_ptr();
    let clone = store.clone_bytes(source).expect("bytes clone");
    let vector = store
        .clone_bytes_to_byte_vector(source)
        .expect("byte-vector clone");

    assert_eq!(store.bytes(clone), Ok(&[1, 2, 3][..]));
    assert_eq!(store.byte_vector(vector), Ok(&[1, 2, 3][..]));
    assert_ne!(
        store.bytes(clone).expect("clone pointer").as_ptr(),
        source_pointer
    );
    assert_ne!(
        store.byte_vector(vector).expect("vector pointer").as_ptr(),
        source_pointer
    );
    assert_eq!(store.stats().allocations, 3);
    assert_eq!(store.stats().live_objects, 3);
    assert_eq!(store.stats().live_bytes, 12);
    assert_eq!(store.stats().allocated_bytes, 12);
    assert_eq!(store.stats().transfers, 0);

    store.free_bytes(source).expect("free source");
    assert_eq!(store.bytes(clone), Ok(&[1, 2, 3][..]));
    store.free_bytes(clone).expect("free clone");
    store.free_byte_vector(vector).expect("free vector");
    assert_eq!(store.stats().allocations, store.stats().frees);
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn path_structural_copy_has_independent_owner_exact_equality_and_return_transfer() {
    let mut store = store_with(65, 2, 32, 2, 2, 3);
    let source = store
        .clone_path_slice(b"/tmp/exact-path")
        .expect("source path");
    let source_pointer = store.path(source).expect("source path bytes").as_ptr();
    let clone = store.clone_path(source).expect("structural path copy");

    assert!(store.paths_equal(source, clone).expect("path equality"));
    assert_ne!(
        store.path(clone).expect("clone path bytes").as_ptr(),
        source_pointer
    );
    assert_eq!(store.stats().allocations, 2);
    assert_eq!(store.stats().live_objects, 2);
    assert_eq!(store.stats().live_bytes, 30);
    assert_eq!(store.stats().allocated_bytes, 30);

    store.free_path(source).expect("free source path");
    assert_eq!(store.path(clone), Ok(&b"/tmp/exact-path"[..]));
    let returned = store.take_path(clone).expect("return path backing");
    assert_eq!(returned.as_ref(), b"/tmp/exact-path");
    assert_eq!(store.stats().frees, 2);
    assert_eq!(store.assert_no_leaks(), Ok(()));
    assert_eq!(store.path(clone), Err(UniqueStoreError::StaleKey));
}

#[test]
fn path_copy_limit_failure_preserves_owner_metrics_and_layout() {
    let mut store = store_with(66, 2, 15, 2, 2, 3);
    let source = store.clone_path_slice(b"/tmp/exact").expect("source path");
    let before = store.stats();
    assert_eq!(store.clone_path(source), Err(UniqueStoreError::ByteLimit));
    assert_eq!(store.path(source), Ok(&b"/tmp/exact"[..]));
    assert_eq!(store.stats(), before);

    let bytes = store.allocate_bytes(Vec::new()).expect("wrong layout key");
    assert_eq!(
        store.import_path_key(bytes.packed_word()),
        Err(UniqueStoreError::WrongLayout {
            expected: UniqueLayout::Path,
            actual: UniqueLayout::Bytes,
        })
    );
    store.free_bytes(bytes).expect("free bytes");
    store.free_path(source).expect("free source path");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn static_thaw_is_one_accounted_copy_and_failures_publish_nothing() {
    let static_bytes = StaticBytes::new(b"static");
    let mut store = store_with(61, 1, 8, 1, 1, 2);
    let vector = store.thaw_static_bytes(static_bytes).expect("static thaw");
    assert_eq!(store.byte_vector(vector), Ok(&b"static"[..]));
    assert_ne!(
        store.byte_vector(vector).expect("dynamic pointer").as_ptr(),
        static_bytes.as_slice().as_ptr()
    );
    assert_eq!(
        store.stats(),
        UniqueStoreStats {
            allocations: 1,
            live_objects: 1,
            peak_live_objects: 1,
            live_bytes: 6,
            peak_live_bytes: 6,
            allocated_bytes: 6,
            ..UniqueStoreStats::default()
        }
    );
    store.free_byte_vector(vector).expect("free thaw");
    assert_eq!(store.assert_no_leaks(), Ok(()));

    let mut limited = store_with(62, 1, 5, 1, 1, 2);
    assert_eq!(
        limited.thaw_static_bytes(static_bytes),
        Err(UniqueStoreError::ByteLimit)
    );
    assert_eq!(limited.stats(), UniqueStoreStats::default());
    assert_eq!(limited.slot_count(), 0);

    let mut allocations = store_with(63, 1, 8, 1, 0, 2);
    assert_eq!(
        allocations.thaw_static_bytes(static_bytes),
        Err(UniqueStoreError::AllocationLimit)
    );
    assert_eq!(allocations.stats(), UniqueStoreStats::default());
    assert_eq!(allocations.slot_count(), 0);
}

#[test]
fn clone_limit_failure_preserves_source_and_exact_metrics() {
    let mut store = store_with(64, 2, 4, 2, 2, 2);
    let source = store.allocate_bytes(vec![1, 2, 3]).expect("source");
    let pointer = store.bytes(source).expect("source pointer").as_ptr();
    let before = store.stats();
    assert_eq!(store.clone_bytes(source), Err(UniqueStoreError::ByteLimit));
    assert_eq!(store.bytes(source), Ok(&[1, 2, 3][..]));
    assert_eq!(store.bytes(source).expect("same pointer").as_ptr(), pointer);
    assert_eq!(store.stats(), before);
    assert_eq!(store.slot_count(), 1);
    store.free_bytes(source).expect("free source");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}
