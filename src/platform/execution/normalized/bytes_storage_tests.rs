use super::*;
use crate::platform::execution::normalized::value::{NormalizedMapKey, NormalizedValue};

fn buffer(bytes: &[u8], capacity: usize) -> BytePayload {
    let mut value = Vec::with_capacity(capacity);
    value.extend_from_slice(bytes);
    BytePayload(Storage::Buffer(Arc::new(value)))
}

fn run(
    left: BytePayload,
    right: BytePayload,
    maximum: u64,
    budget: u64,
) -> (Result<BytePayload, ExecutionError>, Work, Vec<u64>) {
    let mut work = Work::default();
    let mut charges = Vec::new();
    let mut used = 0_u64;
    let result = left.concat(
        right,
        maximum,
        &ExecutionControl::uncancelled(),
        &mut |bytes| {
            let next = used
                .checked_add(bytes)
                .filter(|n| *n <= budget)
                .ok_or_else(|| {
                    ExecutionError::resource("fixture_budget", "test storage budget exhausted")
                })?;
            used = next;
            charges.push(bytes);
            Ok(())
        },
        &mut work,
    );
    (result, work, charges)
}

#[test]
fn byte_storage_representation_does_not_change_key_order_or_retained_values() {
    let prefix = buffer(&[0, 128, 255], 8);
    let key = NormalizedMapKey::from_value(NormalizedValue::Bytes(prefix.clone())).unwrap();
    let canonical = NormalizedMapKey::Bytes([0, 128, 255].into());
    assert_eq!(key, canonical);
    assert_eq!(key.cmp(&canonical), Ordering::Equal);
    let mut map = std::collections::BTreeMap::new();
    map.insert(key.clone(), 17);
    assert_eq!(map.insert(canonical, 23), Some(17));
    let result = run(prefix, [17].into(), 100, 100).0.unwrap();
    assert_eq!(result.as_ref(), &[0, 128, 255, 17]);
    assert_eq!(key.to_value(), NormalizedValue::bytes([0, 128, 255]));
    assert_eq!(
        map.get(&NormalizedMapKey::Bytes([0, 128, 255].into())),
        Some(&23)
    );
    assert!(!map.contains_key(&NormalizedMapKey::Bytes([0, 128, 255, 17].into())));
    assert_eq!(format!("{result:?}"), "[0, 128, 255, 17]");
}

#[test]
fn byte_storage_admits_each_allocation_before_copy_and_never_charges_reuse() {
    let total = BytePayload::DESCRIPTOR_BYTES + 3;
    let (result, work, charges) = run([0, 128].into(), [255].into(), 3, total);
    assert_eq!(result.unwrap().as_ref(), &[0, 128, 255]);
    assert_eq!(charges, [BytePayload::DESCRIPTOR_BYTES, 3]);
    assert_eq!(
        (
            work.fresh_buffers,
            work.payload_bytes_copied,
            work.requested_capacity_bytes
        ),
        (1, 3, 3)
    );
    for budget in [0, BytePayload::DESCRIPTOR_BYTES - 1, total - 1] {
        let (result, work, _) = run([0, 128].into(), [255].into(), 3, budget);
        assert_eq!(result.unwrap_err().code, "fixture_budget");
        assert_eq!(work.payload_bytes_copied, 0);
        assert_eq!(work.requested_capacity_bytes, 0);
    }
    let (result, work, charges) = run([0, 128].into(), [255].into(), 2, total);
    assert_eq!(result.unwrap_err().code, "normalized_allocation");
    assert_eq!(work, Work::default());
    assert!(charges.is_empty());
    let unique = buffer(&[0, 128, 255], 4);
    let original = unique.as_ptr();
    let (result, work, charges) = run(unique, [17].into(), 4, 0);
    let result = result.unwrap();
    assert_eq!(result.as_ref(), &[0, 128, 255, 17]);
    assert_eq!(result.as_ptr(), original);
    assert_eq!((work.in_place_appends, work.payload_bytes_copied), (1, 1));
    assert!(charges.is_empty());
    let original = result.as_ptr();
    let (same, work, charges) = run(result, [].into(), 4, 0);
    assert_eq!(same.unwrap().as_ptr(), original);
    assert_eq!(work.empty_operand_reuses, 1);
    assert!(charges.is_empty());
}

#[test]
fn byte_storage_growth_reserves_before_replacing_the_unique_buffer() {
    for one_short in [false, true] {
        let prefix = buffer(&[1, 2, 3], 3);
        let Storage::Buffer(stored) = &prefix.0 else {
            panic!("buffer");
        };
        // Observe the allocator's capacity, not a promise of minimal reservation.
        let capacity = stored.capacity();
        let suffix = vec![4; capacity + 1 - prefix.len()];
        let mut expected = prefix.to_vec();
        expected.extend_from_slice(&suffix);
        let growth = (2 * capacity) as u64;
        let (result, work, charges) =
            run(prefix, suffix.into(), growth, growth - u64::from(one_short));
        if one_short {
            assert_eq!(result.unwrap_err().code, "fixture_budget");
            assert_eq!(work.payload_bytes_copied, 0);
            assert_eq!(work.requested_capacity_bytes, 0);
            assert!(charges.is_empty());
        } else {
            assert_eq!(result.unwrap().as_ref(), expected);
            assert_eq!(charges, [growth]);
            assert_eq!(work.buffer_growths, 1);
            assert_eq!(work.payload_bytes_copied, expected.len() as u64);
        }
    }
}

#[test]
fn byte_storage_self_concatenation_preserves_both_aliased_arguments() {
    let prefix = buffer(&[0, 128, 255], 8);
    let (result, work, _) = run(prefix.clone(), prefix, 100, 100);
    assert_eq!(result.unwrap().as_ref(), &[0, 128, 255, 0, 128, 255]);
    assert_eq!(work.fresh_buffers, 1);
    assert_eq!(work.in_place_appends, 0);
    assert_eq!(work.payload_bytes_copied, 6);
}

#[test]
fn byte_storage_weak_alias_cannot_observe_mutation() {
    let payload = buffer(&[1, 2, 3], 8);
    let Storage::Buffer(shared) = &payload.0 else {
        panic!("buffer");
    };
    let weak = Arc::downgrade(shared);
    let mut observer = None;
    let mut work = Work::default();
    let result = payload
        .concat(
            [4].into(),
            100,
            &ExecutionControl::uncancelled(),
            &mut |_| {
                // A weak observer may promote after the ownership decision, before allocation.
                observer = weak.upgrade();
                Ok(())
            },
            &mut work,
        )
        .unwrap();
    assert_eq!(observer.unwrap().as_slice(), &[1, 2, 3]);
    assert_eq!(result.as_ref(), &[1, 2, 3, 4]);
    assert_eq!(work.fresh_buffers, 1);
    assert_eq!(work.in_place_appends, 0);
}

#[test]
fn byte_storage_unique_growth_has_linear_explicit_copy_work() {
    let mut value = BytePayload::from([]);
    let mut work = Work::default();
    let mut requested = 0;
    for n in 0..4096 {
        value = value
            .concat(
                [(n % 256) as u8].into(),
                8192,
                &ExecutionControl::uncancelled(),
                &mut |bytes| {
                    requested += bytes;
                    Ok(())
                },
                &mut work,
            )
            .unwrap();
    }
    assert_eq!(
        value.as_ref(),
        (0..4096).map(|n| (n % 256) as u8).collect::<Vec<_>>()
    );
    assert!(work.payload_bytes_copied < 3 * 4096, "{work:?}");
    assert!(work.requested_capacity_bytes < 4 * 4096, "{work:?}");
    assert_eq!(
        requested,
        work.requested_capacity_bytes + BytePayload::DESCRIPTOR_BYTES
    );
    assert_eq!(work.fresh_buffers, 1);
    assert!(work.in_place_appends > 4000);
    assert!(work.buffer_growths <= 12);
}

#[test]
fn byte_storage_cancelled_copy_never_mutates_retained_payload_or_keys() {
    let prefix = buffer(&vec![17; 131_072], 262_144);
    let right = BytePayload::from(vec![255; 65_537]);
    let original = prefix.as_ptr();
    let mut cancelled = 0;
    let mut completed = 0;
    for checks in 0..=12 {
        let mut work = Work::default();
        let result = prefix.clone().concat(
            right.clone(),
            262_144,
            &ExecutionControl::cancel_after_checks(checks),
            &mut |_| Ok(()),
            &mut work,
        );
        match result {
            Ok(value) => {
                assert_eq!(&value[..131_072], prefix.as_ref());
                assert_eq!(&value[131_072..], right.as_ref());
                completed += 1;
            }
            Err(error) => {
                assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Cancelled
                );
                assert!(work.payload_bytes_copied <= 196_609);
                cancelled += 1;
            }
        }
        assert_eq!(prefix.as_ptr(), original);
        assert!(prefix.iter().all(|byte| *byte == 17));
        assert!(right.iter().all(|byte| *byte == 255));
    }
    assert!(cancelled > 1 && completed > 0);
}
