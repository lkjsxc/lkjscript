use super::*;

#[test]
fn allocate_read_mutate_free_and_exact_stats() {
    let mut store = store_with(1, 2, 16, 2, 4, 4);
    let key = store
        .allocate_byte_vector(bytes_with_capacity(4, &[1, 2]))
        .expect("byte-vector allocation");
    assert_eq!(store.byte_vector(key), Ok(&[1, 2][..]));
    store.byte_vector_mut(key).expect("mutable access")[1] = 9;
    assert_eq!(store.byte_vector(key), Ok(&[1, 9][..]));
    assert_eq!(
        store.stats(),
        UniqueStoreStats {
            allocations: 1,
            live_objects: 1,
            peak_live_objects: 1,
            live_bytes: 4,
            peak_live_bytes: 4,
            allocated_bytes: 4,
            ..UniqueStoreStats::default()
        }
    );
    store.free_byte_vector(key).expect("single free");
    assert_eq!(store.stats().frees, 1);
    assert_eq!(store.stats().live_objects, 0);
    assert_eq!(store.stats().live_bytes, 0);
    assert_eq!(store.stats().allocated_bytes, 4);
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn double_free_and_stale_access_fail_without_deallocation() {
    let mut store = store_with(2, 1, 8, 1, 2, 4);
    let key = store
        .allocate_byte_vector(vec![7])
        .expect("byte-vector allocation");
    store.free_byte_vector(key).expect("first free");
    assert_eq!(store.free_byte_vector(key), Err(UniqueStoreError::StaleKey));
    assert_eq!(store.byte_vector(key), Err(UniqueStoreError::StaleKey));
    assert_eq!(store.stats().frees, 1);
    assert_eq!(store.stats().stale_failures, 2);
}

#[test]
fn store_mismatch_is_rejected_before_slot_or_failure_access() {
    let mut owner = store_with(3, 1, 8, 1, 1, 2);
    let mut other = store_with(4, 1, 8, 1, 1, 2);
    let key = owner
        .allocate_byte_vector(vec![3])
        .expect("owner allocation");
    assert_eq!(other.byte_vector(key), Err(UniqueStoreError::StoreMismatch));
    assert_eq!(other.stats(), UniqueStoreStats::default());
    assert_eq!(owner.byte_vector(key), Ok(&[3][..]));
    owner.free_byte_vector(key).expect("owner free");
}

#[test]
fn paths_are_immutable_typed_payloads_and_static_bytes_use_no_slot() {
    let mut store = store_with(5, 1, 16, 1, 1, 2);
    let static_bytes = StaticBytes::new(b"static");
    assert_eq!(static_bytes.as_slice(), b"static");
    assert_eq!(static_bytes.len(), 6);
    assert_eq!(store.slot_count(), 0);
    let path = store
        .allocate_path(Vec::from(&b"/tmp"[..]).into_boxed_slice())
        .expect("path allocation");
    assert_eq!(store.path(path), Ok(&b"/tmp"[..]));
    assert_eq!(store.stats().live_bytes, 4);
    store.free_path(path).expect("path free");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}

#[test]
fn leak_assertion_reports_exact_live_obligation() {
    let mut store = store_with(6, 1, 8, 1, 1, 2);
    let key = store
        .allocate_bytes(bytes_with_capacity(3, &[1]))
        .expect("dynamic bytes allocation");
    assert_eq!(
        store.assert_no_leaks(),
        Err(UniqueStoreLeak {
            live_objects: 1,
            live_bytes: 3,
        })
    );
    store.free_bytes(key).expect("bytes free");
    assert_eq!(store.assert_no_leaks(), Ok(()));
}
