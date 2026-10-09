//! Independent flat values and final-topology allocation oracle for bulk ingress.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

pub(super) fn model(length: usize) -> (u64, u64, Charge) {
    // Count final 32-way tree levels without calling the production builder,
    // metadata calculation, capacity helper, or charge producers.
    let leaves = length.div_ceil(32) as u64;
    let mut level = length.saturating_sub(1) / 32;
    let mut branches = 0;
    while level > 1 {
        level = level.div_ceil(32);
        branches += level as u64;
    }
    let node_bytes = (35 * std::mem::size_of::<usize>()) as u64;
    let element_bytes =
        (std::mem::size_of::<NormalizedValue>() + 2 * std::mem::size_of::<usize>()) as u64;
    (
        leaves,
        branches,
        Charge {
            slots: (leaves + branches) * 32,
            bytes: (leaves + branches) * node_bytes + length as u64 * element_bytes,
        },
    )
}

pub(super) fn items(length: usize) -> Vec<NormalizedValue> {
    (0..length)
        .map(|i| NormalizedValue::I64(i as i64 * 17 - 9))
        .collect()
}

fn verify(list: &List, length: usize) {
    assert_eq!(list.len(), length);
    for (index, value) in list.iter().enumerate() {
        assert_eq!(*value, NormalizedValue::I64(index as i64 * 17 - 9));
        assert_eq!(list.get(index), Some(value));
    }
    for (index, value) in list.iter().rev().enumerate() {
        assert_eq!(
            *value,
            NormalizedValue::I64((length - index - 1) as i64 * 17 - 9)
        );
    }
    assert!(list.get(length).is_none());
    assert!(list.get(usize::MAX).is_none());
}

#[test]
fn bulk_allocates_only_final_nodes_without_copying_existing_handles() {
    let _scope = WorkScope::enter();
    for length in [
        0, 1, 31, 32, 33, 63, 64, 65, 1023, 1024, 1025, 1055, 1056, 1057, 32767, 32768, 32769,
        32799, 32800, 32801, 262145, 1_000_000,
    ] {
        let (leaves, branches, expected) = model(length);
        let mut charged = Charge::default();
        let before = Work::current();
        let list = List::from_items(items(length), MAXIMUM_LENGTH as u64, &mut |charge| {
            charged.slots += charge.slots;
            charged.bytes += charge.bytes;
            Ok(())
        })
        .unwrap();
        let work = before.since();
        assert_eq!(charged, expected, "length={length}");
        assert_eq!(
            work,
            Work {
                element_handle_allocations: length as u64,
                element_slots_reserved: leaves * 32,
                branch_slots_reserved: branches * 32,
                nodes_allocated: leaves + branches,
                ..Work::ZERO
            },
            "length={length}"
        );
        assert_eq!(
            list.metadata_bytes().unwrap(),
            charged.bytes - length as u64 * std::mem::size_of::<NormalizedValue>() as u64
        );
        let traversal = Work::current();
        assert_eq!(list.iter().count(), length);
        assert_eq!(traversal.since().node_visits, leaves + branches);
        verify(&list, length);
    }
}

#[test]
fn bulk_work_report_checks_complete_values_without_assuming_implementation_cost() {
    let _scope = WorkScope::enter();
    for length in [0, 32, 1024, 4096, 32768, 262144, 1_000_000] {
        let mut charge = Charge::default();
        let before = Work::current();
        let list = List::from_items(items(length), MAXIMUM_LENGTH as u64, &mut |added| {
            charge.slots += added.slots;
            charge.bytes += added.bytes;
            Ok(())
        })
        .unwrap();
        let work = before.since();
        verify(&list, length);
        println!(
            "bulk length={length} nodes={} branch_copies={} visits={} slots={} bytes={}",
            work.nodes_allocated,
            work.branch_slot_copies,
            work.node_visits,
            charge.slots,
            charge.bytes
        );
    }
}

#[test]
fn bulk_roots_remain_append_compatible_and_share_retained_payloads() {
    for length in [
        0, 31, 32, 33, 63, 64, 65, 1023, 1024, 1055, 1056, 1057, 32767, 32768, 32799, 32800, 32801,
    ] {
        let root = List::from_items(items(length), MAXIMUM_LENGTH as u64, &mut |_| Ok(())).unwrap();
        let mut successor = root.clone();
        for next_length in length..length + 67 {
            successor = successor
                .append(
                    NormalizedValue::I64(next_length as i64 * 17 - 9),
                    MAXIMUM_LENGTH as u64,
                    &mut |_| Ok(()),
                )
                .unwrap();
        }
        let sibling = root
            .append(
                NormalizedValue::Bool(true),
                MAXIMUM_LENGTH as u64,
                &mut |_| Ok(()),
            )
            .unwrap();
        verify(&root, length);
        verify(&successor, length + 67);
        assert_eq!(sibling.len(), length + 1);
        assert_eq!(sibling.get(length), Some(&NormalizedValue::Bool(true)));
        for index in 0..length {
            assert!(std::ptr::eq(
                root.get(index).unwrap(),
                successor.get(index).unwrap()
            ));
            assert!(std::ptr::eq(
                root.get(index).unwrap(),
                sibling.get(index).unwrap()
            ));
        }
        drop(root);
        verify(&successor, length + 67);
    }
}

#[test]
fn bulk_exact_storage_limits_accept_and_one_less_refuses() {
    for length in [1, 33, 65, 1057, 32801] {
        let (_, _, required) = model(length);
        for (slots, bytes, success) in [
            (required.slots, required.bytes, true),
            (required.slots - 1, required.bytes, false),
            (required.slots, required.bytes - 1, false),
        ] {
            let mut used = Charge::default();
            let result = List::from_items(items(length), length as u64, &mut |charge| {
                let next = Charge {
                    slots: used.slots + charge.slots,
                    bytes: used.bytes + charge.bytes,
                };
                if next.slots > slots || next.bytes > bytes {
                    return Err(failure());
                }
                used = next;
                Ok(())
            });
            assert_eq!(
                result.is_ok(),
                success,
                "length={length} slots={slots} bytes={bytes}"
            );
            if let Ok(list) = result {
                assert_eq!(used, required);
                verify(&list, length);
            }
        }
    }
}
