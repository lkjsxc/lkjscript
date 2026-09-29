use super::*;

fn range(value: BytePayload, start: i64, end: i64) -> (BytePayload, Vec<u64>) {
    let mut charges = Vec::new();
    let result = value
        .slice(start, end, &ExecutionControl::uncancelled(), &mut |bytes| {
            charges.push(bytes);
            Ok(())
        })
        .unwrap();
    (result, charges)
}

fn buffer() -> BytePayload {
    let mut bytes = Vec::with_capacity(64);
    bytes.extend(0..8);
    BytePayload(Storage::Buffer(Arc::new(bytes)))
}

#[test]
fn byte_slice_shares_both_backings_and_preserves_full_and_empty_ranges() {
    for original in [BytePayload::from((0..8).collect::<Vec<_>>()), buffer()] {
        let pointer = original.as_ptr();
        let (full, charges) = range(original.clone(), 0, 8);
        assert!(charges.is_empty());
        assert_eq!(full.as_ptr(), pointer);
        let (window, charges) = range(original.clone(), 2, 6);
        assert_eq!(charges, [BytePayload::SLICE_DESCRIPTOR_BYTES]);
        assert_eq!(window.as_ref(), [2, 3, 4, 5]);
        assert_eq!(window.as_ptr(), original[2..].as_ptr());
        let (empty, charges) = range(window, 2, 2);
        assert!(charges.is_empty());
        assert!(empty.is_empty());
        assert!(!matches!(empty.0, Storage::Window(_)));
        assert_eq!(original.as_ref(), [0, 1, 2, 3, 4, 5, 6, 7]);
    }
}

#[test]
fn byte_slice_unique_nested_views_are_flat_and_reuse_one_descriptor() {
    let root = Arc::<[u8]>::from(vec![127; 20_002]);
    let (mut window, charges) = range(BytePayload::from(root.clone()), 0, 20_001);
    assert_eq!(charges.len(), 1);
    let Storage::Window(descriptor) = &window.0 else {
        panic!("expected view")
    };
    let pointer = Arc::as_ptr(descriptor);
    for offset in 1..=10_000 {
        let length = window.len() as i64;
        let (next, charges) = range(window, 1, length);
        assert!(charges.is_empty());
        let Storage::Window(descriptor) = &next.0 else {
            panic!("expected view")
        };
        assert_eq!(Arc::as_ptr(descriptor), pointer);
        assert_eq!(descriptor.start, offset);
        assert_eq!(descriptor.end, 20_001);
        assert_eq!(Arc::strong_count(&root), 2);
        window = next;
    }
    drop(window);
    assert_eq!(Arc::strong_count(&root), 1);
}

#[test]
fn byte_slice_shared_and_weak_views_never_observe_metadata_mutation() {
    let original = buffer();
    let (view, _) = range(original.clone(), 1, 7);
    let retained = view.clone();
    let (nested, charges) = range(view, 1, 3);
    assert_eq!(charges, [BytePayload::SLICE_DESCRIPTOR_BYTES]);
    assert_eq!(retained.as_ref(), [1, 2, 3, 4, 5, 6]);
    assert_eq!(nested.as_ref(), [2, 3]);
    assert_eq!(nested.as_ptr(), original[2..].as_ptr());
    let Storage::Window(descriptor) = &nested.0 else {
        panic!("expected view")
    };
    let weak = Arc::downgrade(descriptor);
    let (smaller, charges) = range(nested, 0, 1);
    assert_eq!(charges, [BytePayload::SLICE_DESCRIPTOR_BYTES]);
    assert!(weak.upgrade().is_none());
    assert_eq!(smaller.as_ref(), [2]);
    assert_eq!(retained.as_ref(), [1, 2, 3, 4, 5, 6]);
}

#[test]
fn byte_slice_empty_and_detached_values_release_the_original_backing() {
    for copy in [false, true] {
        let root = Arc::new(vec![42; 1_000_000]);
        let weak = Arc::downgrade(&root);
        let value = BytePayload(Storage::Buffer(root));
        let (view, _) = range(value, 123, 127);
        let result = if copy {
            let mut charges = Vec::new();
            let detached = view
                .detached_copy(&ExecutionControl::uncancelled(), &mut |bytes| {
                    charges.push(bytes);
                    Ok(())
                })
                .unwrap();
            assert_eq!(charges, [BytePayload::DESCRIPTOR_BYTES, 4]);
            assert_ne!(detached.as_ptr(), view.as_ptr());
            assert_eq!(detached.as_ref(), [42; 4]);
            drop(view);
            detached
        } else {
            range(view, 2, 2).0
        };
        assert!(weak.upgrade().is_none());
        assert_eq!(result.len(), if copy { 4 } else { 0 });
    }
}

#[test]
fn byte_slice_concatenation_never_exposes_outside_bytes_or_mutates_aliases() {
    let value = buffer();
    let (view, _) = range(value.clone(), 2, 5);
    let mut work = Work::default();
    let combined = value
        .concat(
            view.clone(),
            1024,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
            &mut work,
        )
        .unwrap();
    assert_eq!(combined.as_ref(), [0, 1, 2, 3, 4, 5, 6, 7, 2, 3, 4]);
    assert_eq!(work.in_place_appends, 0);
    let joined = view
        .clone()
        .concat(
            BytePayload::from([255]),
            1024,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
            &mut work,
        )
        .unwrap();
    assert_eq!(joined.as_ref(), [2, 3, 4, 255]);
    assert_eq!(view.as_ref(), [2, 3, 4]);
    let mut keys = std::collections::BTreeMap::new();
    keys.insert(view, 7);
    assert_eq!(keys.get(&BytePayload::from([2, 3, 4])), Some(&7));
    assert!(!keys.contains_key(&BytePayload::from([2, 3, 4, 5])));
}

#[test]
fn byte_slice_reserves_before_growth_and_preserves_retained_values_on_failure() {
    let original = buffer();
    let mut calls = 0;
    let error = original
        .clone()
        .slice(2, 4, &ExecutionControl::uncancelled(), &mut |bytes| {
            calls += 1;
            assert_eq!(bytes, BytePayload::SLICE_DESCRIPTOR_BYTES);
            Err(ExecutionError::resource("slice_budget", "refused"))
        })
        .unwrap_err();
    assert_eq!(error.code, "slice_budget");
    assert_eq!(calls, 1);
    for refusal in [1, 2] {
        calls = 0;
        let error = original
            .detached_copy(&ExecutionControl::uncancelled(), &mut |_| {
                calls += 1;
                if calls == refusal {
                    Err(ExecutionError::resource("copy_budget", "refused"))
                } else {
                    Ok(())
                }
            })
            .unwrap_err();
        assert_eq!(error.code, "copy_budget");
        assert_eq!(calls, refusal);
    }
    assert_eq!(original.as_ref(), [0, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn byte_slice_cancellation_is_checked_before_allocating_after_reservation() {
    for copy in [false, true] {
        let original = buffer();
        let control = ExecutionControl::uncancelled();
        let mut charges = 0;
        let mut reserve = |_| {
            charges += 1;
            control.cancel();
            Ok(())
        };
        let result = if copy {
            original.detached_copy(&control, &mut reserve)
        } else {
            original.clone().slice(1, 3, &control, &mut reserve)
        };
        assert!(result.is_err());
        assert_eq!(charges, 1);
        assert_eq!(original.as_ref(), [0, 1, 2, 3, 4, 5, 6, 7]);
        assert!(
            original
                .clone()
                .slice(0, 8, &control, &mut |_| panic!("cancelled before reserve"))
                .is_err()
        );
        assert!(
            original
                .detached_copy(&control, &mut |_| panic!("cancelled before reserve"))
                .is_err()
        );
    }
}
