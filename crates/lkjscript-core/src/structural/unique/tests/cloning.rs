use super::*;

#[test]
fn dynamic_bytes_clones_publish_independent_exact_owners() {
    let mut store = store(60);
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

    store.free_bytes(source).expect("free source");
    assert_eq!(store.bytes(clone), Ok(&[1, 2, 3][..]));
    store.free_bytes(clone).expect("free clone");
    store.free_byte_vector(vector).expect("free vector");
    assert_eq!(store.stats().allocations, store.stats().frees);
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn path_structural_copy_has_independent_owner_exact_equality_and_return_transfer() {
    let mut store = store(65);
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

    store.free_path(source).expect("free source path");
    assert_eq!(store.path(clone), Ok(&b"/tmp/exact-path"[..]));
    let returned = store.take_path(clone).expect("return path backing");
    assert_eq!(returned.as_ref(), b"/tmp/exact-path");
    assert_eq!(store.stats().frees, 2);
    assert_eq!(store.assert_no_leaks(), Ok(()));
    assert_eq!(store.path(clone), Err(UniqueStoreError::StaleKey));
}

#[test]
fn clone_rejects_wrong_layout_without_mutation() {
    let mut store = store(66);
    let source = store.clone_path_slice(b"/tmp/exact").expect("source path");
    let bytes = store.allocate_bytes(Vec::new()).expect("wrong layout key");
    let before = store.stats();
    assert_eq!(
        store.import_path_key(bytes.opaque_word()),
        Err(UniqueStoreError::WrongLayout {
            expected: UniqueLayout::Path,
            actual: UniqueLayout::Bytes,
        })
    );
    let after = store.stats();
    assert_eq!(
        after.wrong_layout_failures,
        before.wrong_layout_failures + 1
    );
    assert_eq!(after.live_objects, before.live_objects);
    assert_eq!(after.live_bytes, before.live_bytes);
    assert_eq!(after.allocations, before.allocations);
    store.free_bytes(bytes).expect("free bytes");
    store.free_path(source).expect("free source path");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn static_thaw_is_one_accounted_copy() {
    let static_bytes = StaticBytes::new(b"static");
    let mut store = store(61);
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
}
