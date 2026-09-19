//! Disjoint ordered/content oracle, physical sharing, storage schedule and failures.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::super::value::{FunctionIndex, NormalizedRecord, RecordLayoutIndex, ValueOrigin};
use super::*;
use crate::platform::execution::ExecutionControl;
use std::collections::BTreeSet;

const TEST_LIMIT: u64 = 1_000_000;

fn free(_: Charge) -> Result<(), ExecutionError> {
    Ok(())
}

fn key(value: i64) -> NormalizedMapKey {
    NormalizedMapKey::I64(value)
}

fn value(value: i64) -> NormalizedValue {
    NormalizedValue::I64(value)
}

fn shape<'a>(
    root: &'a Link,
    lower: Option<&'a NormalizedMapKey>,
    upper: Option<&'a NormalizedMapKey>,
) -> (usize, usize) {
    let Some(root) = root else {
        return (0, 0);
    };
    assert!(lower.is_none_or(|lower| lower < &root.entry.key));
    assert!(upper.is_none_or(|upper| &root.entry.key < upper));
    let (left_height, left_count) = shape(&root.left, lower, Some(&root.entry.key));
    let (right_height, right_count) = shape(&root.right, Some(&root.entry.key), upper);
    assert!(left_height.abs_diff(right_height) <= 1);
    assert_eq!(root.height, left_height.max(right_height) + 1);
    (root.height, left_count + right_count + 1)
}

fn agrees(map: &Map, expected: &BTreeMap<NormalizedMapKey, NormalizedValue>) {
    assert_eq!(map.len(), expected.len());
    assert_eq!(map.is_empty(), expected.is_empty());
    assert_eq!(shape(&map.root, None, None).1, expected.len());
    assert!(map.iter().eq(expected.iter()));
    assert!(map.iter().rev().eq(expected.iter().rev()));
    assert!(map.keys().eq(expected.keys()));
    assert!(map.values().eq(expected.values()));
    let mut actual = map.iter();
    let mut oracle = expected.iter();
    let mut forward = true;
    while oracle.len() != 0 {
        assert_eq!(actual.len(), oracle.len());
        if forward {
            assert_eq!(actual.next(), oracle.next());
        } else {
            assert_eq!(actual.next_back(), oracle.next_back());
        }
        forward = !forward;
    }
    assert_eq!(actual.next(), None);
    assert_eq!(actual.next_back(), None);
    for (key, value) in expected {
        assert_eq!(map.get(key), Some(value));
        assert_eq!(map.get_key_value(key), Some((key, value)));
        assert!(map.contains_key(key));
    }
}

fn next_random(random: &mut u64) -> u64 {
    *random ^= *random << 13;
    *random ^= *random >> 7;
    *random ^= *random << 17;
    *random
}

#[test]
fn persistent_map_order_snapshots_and_rebalancing_match_btree() {
    let mut map = Map::default();
    let mut oracle = BTreeMap::new();
    let mut retained = Vec::new();
    let mut random = 0x123456789abcdef0_u64;
    for step in 0..8192 {
        let random = next_random(&mut random);
        let selected = key((random % 1024) as i64);
        if random & 3 == 0 {
            map = map.remove(&selected, &mut free).unwrap();
            oracle.remove(&selected);
        } else {
            map = map
                .insert(selected.clone(), value(step), TEST_LIMIT, &mut free)
                .unwrap();
            oracle.insert(selected, value(step));
        }
        if step % 128 == 0 {
            agrees(&map, &oracle);
            retained.push((map.clone(), oracle.clone()));
        }
    }
    agrees(&map, &oracle);
    for (snapshot, oracle) in &retained {
        agrees(snapshot, oracle);
    }
    for key in oracle.keys().cloned().collect::<Vec<_>>() {
        map = map.remove(&key, &mut free).unwrap();
    }
    assert!(map.is_empty());
    for (snapshot, oracle) in retained {
        agrees(&snapshot, &oracle);
    }
}

#[test]
fn persistent_map_randomized_branches_keep_every_retained_history() {
    let mut histories = vec![(Map::default(), BTreeMap::new())];
    let mut random = 0x7d5230e18862d179_u64;
    for step in 0..2048 {
        let selection = next_random(&mut random);
        let parent = (selection as usize) % histories.len();
        let (map, oracle) = &histories[parent];
        let selected = match selection % 4 {
            0 => NormalizedMapKey::Bool(selection & 4 != 0),
            1 => key((selection % 37) as i64 - 18),
            2 => NormalizedMapKey::Bytes((selection % 41).to_be_bytes().to_vec()),
            _ => NormalizedMapKey::Text(format!("key-{}", selection % 43)),
        };
        let mut expected = oracle.clone();
        let changed = if selection & 8 == 0 {
            expected.remove(&selected);
            map.remove(&selected, &mut free).unwrap()
        } else {
            expected.insert(selected.clone(), value(step));
            map.insert(selected, value(step), TEST_LIMIT, &mut free)
                .unwrap()
        };
        agrees(map, oracle);
        agrees(&changed, &expected);
        histories.push((changed, expected));
    }
    for (map, expected) in histories {
        agrees(&map, &expected);
    }
}

#[test]
fn persistent_map_bulk_order_and_all_key_kinds_are_canonical() {
    // The literal order is independently specified, including cross-kind ordering.
    let ordered = vec![
        (NormalizedMapKey::Bool(false), value(1)),
        (NormalizedMapKey::Bool(true), value(2)),
        (key(i64::MIN), value(3)),
        (key(i64::MAX), value(4)),
        (NormalizedMapKey::Bytes(vec![]), value(5)),
        (NormalizedMapKey::Bytes(vec![0, 255]), value(6)),
        (NormalizedMapKey::Bytes(vec![255]), value(7)),
        (NormalizedMapKey::Text("".to_owned()), value(8)),
        (NormalizedMapKey::Text("a".to_owned()), value(9)),
        (NormalizedMapKey::Text("日本語".to_owned()), value(10)),
    ];
    let entries = ordered.iter().cloned().collect::<BTreeMap<_, _>>();
    let bulk = Map::from_items(entries.clone(), 10, &mut free).unwrap();
    let mut incremental = Map::default();
    for (key, value) in entries.iter().rev() {
        incremental = incremental
            .insert(key.clone(), value.clone(), 10, &mut free)
            .unwrap();
    }
    assert!(
        bulk.iter()
            .eq(ordered.iter().map(|(key, value)| (key, value)))
    );
    agrees(&bulk, &entries);
    agrees(&incremental, &entries);
    assert_eq!(bulk, incremental);
    assert_ne!(bulk, Map::default());
    assert_eq!(
        NormalizedMapKey::from_value(NormalizedValue::static_text("a")),
        Some(NormalizedMapKey::Text("a".to_owned()))
    );
    assert_eq!(
        Map::from_items(BTreeMap::new(), 0, &mut free).unwrap(),
        Map::default()
    );
    assert!(Map::from_items(entries, 9, &mut free).is_err());
}

// Independent inspection never reads the carrier's observations or charge counters.
fn physical(map: &Map) -> (BTreeSet<usize>, BTreeSet<usize>) {
    let mut nodes = BTreeSet::new();
    let mut entries = BTreeSet::new();
    let mut pending = Vec::from_iter(map.root.as_ref());
    while let Some(node) = pending.pop() {
        assert!(nodes.insert(Arc::as_ptr(node) as usize));
        assert!(entries.insert(Arc::as_ptr(&node.entry) as usize));
        pending.extend(node.left.as_ref());
        pending.extend(node.right.as_ref());
    }
    (nodes, entries)
}

fn retains_untouched_payloads(before: &Map, after: &Map, edited: &NormalizedMapKey) -> bool {
    before.iter().all(|(key, previous)| {
        key == edited
            || after
                .get(key)
                .is_some_and(|next| std::ptr::eq(previous, next))
    })
}

fn composite(n: i64, origin: ValueOrigin) -> NormalizedValue {
    let mut value = NormalizedValue::Record(NormalizedRecord::Nominal {
        layout: RecordLayoutIndex(7, origin),
        fields: Arc::new(vec![
            NormalizedValue::list(vec![
                NormalizedValue::bytes(vec![n as u8; 1024]),
                NormalizedValue::Result {
                    success: true,
                    value: Box::new(value(n)),
                },
            ])
            .unwrap(),
            NormalizedValue::Function {
                function: FunctionIndex(3, origin),
                type_arguments: Arc::from([]),
                effect_arguments: Arc::from([]),
                requirement_arguments: Arc::from([]),
                bound_arguments: Some(Arc::new(vec![value(n)])),
            },
        ]),
    });
    for _ in 0..24 {
        value = NormalizedValue::Option(Some(Box::new(value)));
    }
    value
}

#[test]
fn persistent_map_retained_composites_share_entries_subtrees_and_owned_buffers() {
    let origin = ValueOrigin::fresh().unwrap();
    for count in [0, 1, 2, 8, 1024, 4096] {
        let expected = (0..count)
            .map(|n| (key(n), composite(n, origin)))
            .collect::<BTreeMap<_, _>>();
        let map = Map::from_items(expected.clone(), TEST_LIMIT, &mut free).unwrap();
        let before = physical(&map);
        let initial_height = shape(&map.root, None, None).0;
        let retained = map.clone();
        for (edited, replacement) in [
            (key(count), Some(composite(-1, origin))),
            (key(count / 2), Some(composite(-2, origin))),
            (key(count / 2), None),
            (key(-1), None),
        ] {
            let mut oracle = expected.clone();
            let is_insert = replacement.is_some();
            let changed = if let Some(replacement) = replacement {
                oracle.insert(edited.clone(), replacement.clone());
                map.insert(edited.clone(), replacement, TEST_LIMIT, &mut free)
                    .unwrap()
            } else {
                oracle.remove(&edited);
                map.remove(&edited, &mut free).unwrap()
            };
            agrees(&changed, &oracle);
            agrees(&retained, &expected);
            assert!(retains_untouched_payloads(&map, &changed, &edited));
            let after = physical(&changed);
            assert_eq!(
                after.1.difference(&before.1).count(),
                usize::from(is_insert)
            );
            let new_nodes = after.0.difference(&before.0).count();
            assert!(new_nodes <= 3 * initial_height + 3);
            if count > 128 {
                assert!(after.0.intersection(&before.0).count() > count as usize - 64);
            }
        }
        if count >= 8 {
            // Negative control: the historical whole-map clone has equal contents,
            // but replaces every entry handle and recursively clones Option boxes.
            let rebuilt = Map::from_items(
                map.iter()
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
                TEST_LIMIT,
                &mut free,
            )
            .unwrap();
            assert_eq!(rebuilt, map);
            assert!(!retains_untouched_payloads(&map, &rebuilt, &key(-1)));
            let rebuilt_physical = physical(&rebuilt);
            assert_eq!(
                rebuilt_physical.1.difference(&before.1).count(),
                count as usize
            );
            assert_eq!(rebuilt_physical.0.intersection(&before.0).count(), 0);
            let NormalizedValue::Option(Some(old_box)) = map.get(&key(0)).unwrap() else {
                unreachable!()
            };
            let NormalizedValue::Option(Some(new_box)) = rebuilt.get(&key(0)).unwrap() else {
                unreachable!()
            };
            assert!(!std::ptr::eq(old_box.as_ref(), new_box.as_ref()));
        }
    }
    let text = String::from("owned key buffer is moved intact");
    let key_pointer = text.as_ptr();
    let payload = Box::new(value(73));
    let payload_pointer = payload.as_ref() as *const NormalizedValue;
    let map = Map::from_items(
        BTreeMap::from([(
            NormalizedMapKey::Text(text),
            NormalizedValue::Option(Some(payload)),
        )]),
        1,
        &mut free,
    )
    .unwrap();
    let (NormalizedMapKey::Text(text), NormalizedValue::Option(Some(payload))) =
        map.iter().next().unwrap()
    else {
        unreachable!()
    };
    assert_eq!(text.as_ptr(), key_pointer);
    assert_eq!(payload.as_ref() as *const NormalizedValue, payload_pointer);
}

// Derived from the documented layout, independently of node_bytes/entry_bytes:
// a node owns three pointer handles, height and two reference counters; an entry
// owns a key/value header and two counters. Owned buffers have another owner.
fn independent_storage_schedule(nodes: u64, entries: u64, key_buffers: u64) -> Charge {
    let word = std::mem::size_of::<usize>() as u64;
    let node_bytes = 6 * word;
    let entry_bytes = (std::mem::size_of::<NormalizedMapKey>()
        + std::mem::size_of::<NormalizedValue>()) as u64
        + 2 * word;
    assert_eq!(std::mem::size_of::<Node>() as u64 + 2 * word, node_bytes);
    assert_eq!(std::mem::size_of::<Entry>() as u64 + 2 * word, entry_bytes);
    Charge {
        slots: nodes,
        bytes: nodes * node_bytes + entries * entry_bytes + key_buffers,
    }
}

fn total(charges: &[Charge]) -> Charge {
    charges
        .iter()
        .fold(Charge::default(), |sum, charge| Charge {
            slots: sum.slots.checked_add(charge.slots).unwrap(),
            bytes: sum.bytes.checked_add(charge.bytes).unwrap(),
        })
}

#[test]
fn persistent_map_accounting_matches_independent_exact_fit_schedule() {
    let mut map = Map::default();
    // Sorted insertion 0, 1, 2 produces 1, 2, then 4 new nodes: the rotation's
    // superseded middle path node remains charged. Replacing root 1 needs one.
    for (key_value, expected_nodes) in [(0, 1), (1, 2), (2, 4), (1, 1)] {
        let expected = independent_storage_schedule(expected_nodes, 1, 0);
        let mut charged = Charge::default();
        map = map
            .insert(
                key(key_value),
                value(key_value),
                TEST_LIMIT,
                &mut |charge| {
                    let next = Charge {
                        slots: charged.slots + charge.slots,
                        bytes: charged.bytes + charge.bytes,
                    };
                    if next.slots > expected.slots || next.bytes > expected.bytes {
                        return Err(storage_error());
                    }
                    charged = next;
                    Ok(())
                },
            )
            .unwrap();
        assert_eq!(charged, expected);
    }
    let before = map.clone();
    let expected = independent_storage_schedule(1, 1, 0);
    for (slot_limit, byte_limit) in [
        (expected.slots - 1, expected.bytes),
        (expected.slots, expected.bytes - 1),
    ] {
        let mut charged = Charge::default();
        assert!(
            map.insert(key(1), value(77), TEST_LIMIT, &mut |charge| {
                let next = Charge {
                    slots: charged.slots + charge.slots,
                    bytes: charged.bytes + charge.bytes,
                };
                if next.slots > slot_limit || next.bytes > byte_limit {
                    return Err(storage_error());
                }
                charged = next;
                Ok(())
            })
            .is_err()
        );
        // The accepted entry allocation remains cumulative even though the next
        // node reservation refused and no successful map was exposed.
        assert_eq!(charged, independent_storage_schedule(0, 1, 0));
        assert_eq!(map, before);
    }
    let mut charges = Vec::new();
    let removed = map
        .remove(&key(1), &mut |charge| {
            charges.push(charge);
            Ok(())
        })
        .unwrap();
    assert_eq!(total(&charges), independent_storage_schedule(1, 0, 0));
    assert_eq!(removed.len(), 2);
    for count in [0, 1, 2, 31, 256] {
        let mut charges = Vec::new();
        let bulk = Map::from_items(
            (0..count).map(|n| (key(n), value(n))).collect(),
            TEST_LIMIT,
            &mut |charge| {
                charges.push(charge);
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(
            total(&charges),
            independent_storage_schedule(count as u64, count as u64, 0)
        );
        let word = std::mem::size_of::<usize>() as u64;
        assert_eq!(bulk.metadata_bytes().unwrap(), count as u64 * 8 * word);
    }
    let borrowed = NormalizedMapKey::Text("日本語".repeat(73));
    let bytes = match &borrowed {
        NormalizedMapKey::Text(text) => text.len() as u64,
        _ => unreachable!(),
    };
    let before = Work::current();
    let mut charges = Vec::new();
    let borrowed_map = Map::default()
        .insert_borrowed(&borrowed, value(1), 1, &mut |charge| {
            if charge.bytes != 0 && charge.slots == 0 {
                assert_eq!(before.since().key_bytes_copied, 0);
                assert_eq!(before.since().entry_handles_allocated, 0);
            }
            charges.push(charge);
            Ok(())
        })
        .unwrap();
    assert_eq!(total(&charges), independent_storage_schedule(1, 1, bytes));
    assert_eq!(before.since().key_bytes_copied, bytes);
    assert_eq!(borrowed_map.get(&borrowed), Some(&value(1)));
}

#[test]
fn persistent_map_failed_reservations_preserve_all_versions() {
    let expected = (0..256)
        .map(|n| (key(n), value(n)))
        .collect::<BTreeMap<_, _>>();
    let map = Map::from_items(expected.clone(), TEST_LIMIT, &mut free).unwrap();
    let retained = map.clone();
    for (selected, replacement) in [
        (key(300), Some(300)),
        (key(128), Some(-1)),
        (key(128), None),
        (key(0), None),
        (key(-1), None),
    ] {
        let operation = |reserve: &mut dyn FnMut(Charge) -> Result<(), ExecutionError>| {
            let mut reserve = |charge| reserve(charge);
            if let Some(replacement) = replacement {
                map.insert(
                    selected.clone(),
                    value(replacement),
                    TEST_LIMIT,
                    &mut reserve,
                )
            } else {
                map.remove(&selected, &mut reserve)
            }
        };
        let mut charges = Vec::new();
        operation(&mut |charge| {
            charges.push(charge);
            Ok(())
        })
        .unwrap();
        for fail_at in 0..charges.len() {
            let mut accepted = Vec::new();
            let result = operation(&mut |charge| {
                if accepted.len() == fail_at {
                    Err(storage_error())
                } else {
                    accepted.push(charge);
                    Ok(())
                }
            });
            assert!(result.is_err());
            assert_eq!(accepted, charges[..fail_at]);
            let control = ExecutionControl::cancel_after_checks(fail_at as u64);
            assert_eq!(
                operation(&mut |_| control.check()).unwrap_err().class,
                ExecutionFailureClass::Cancelled
            );
            agrees(&map, &expected);
            agrees(&retained, &expected);
            assert!(
                map.insert(key(777), value(777), TEST_LIMIT, &mut free)
                    .is_ok()
            );
        }
    }
    assert!(map.insert(key(300), value(300), 256, &mut free).is_err());
    assert!(map.insert(key(128), value(-1), 256, &mut free).is_ok());
    let mut allocated = Charge::default();
    let missing = map
        .remove(&key(-1), &mut |charge| {
            allocated.slots += charge.slots;
            allocated.bytes += charge.bytes;
            Ok(())
        })
        .unwrap();
    assert_eq!(allocated, Charge::default());
    assert!(Arc::ptr_eq(
        map.root.as_ref().unwrap(),
        missing.root.as_ref().unwrap()
    ));
    agrees(&map, &expected);
}

#[test]
fn persistent_map_bulk_failure_and_cancellation_release_every_owned_payload() {
    let payload: Arc<[u8]> = Arc::from([17_u8; 64]);
    let make_entries = || {
        (0..31)
            .map(|n| (key(n), NormalizedValue::Bytes(Arc::clone(&payload))))
            .collect()
    };
    let mut charges = Vec::new();
    drop(
        Map::from_items(make_entries(), TEST_LIMIT, &mut |charge| {
            charges.push(charge);
            Ok(())
        })
        .unwrap(),
    );
    assert_eq!(Arc::strong_count(&payload), 1);
    for fail_at in 0..charges.len() {
        let mut accepted = Vec::new();
        assert!(
            Map::from_items(make_entries(), TEST_LIMIT, &mut |charge| {
                if accepted.len() == fail_at {
                    Err(storage_error())
                } else {
                    accepted.push(charge);
                    Ok(())
                }
            })
            .is_err()
        );
        assert_eq!(accepted, charges[..fail_at]);
        assert_eq!(Arc::strong_count(&payload), 1);
        let control = ExecutionControl::cancel_after_checks(fail_at as u64);
        assert_eq!(
            Map::from_items(make_entries(), TEST_LIMIT, &mut |_| control.check())
                .unwrap_err()
                .class,
            ExecutionFailureClass::Cancelled
        );
        assert_eq!(Arc::strong_count(&payload), 1);
    }
    assert!(Map::from_items(make_entries(), 30, &mut free).is_err());
    assert_eq!(Arc::strong_count(&payload), 1);
    let healthy = Map::from_items(make_entries(), TEST_LIMIT, &mut free).unwrap();
    assert_eq!(healthy.len(), 31);
    drop(healthy);
    assert_eq!(Arc::strong_count(&payload), 1);
}

fn deep_raw(depth: usize, payload: &Arc<[u8]>) -> NormalizedValue {
    let mut value = NormalizedValue::Bytes(Arc::clone(payload));
    for n in 0..depth {
        value = match n % 5 {
            0 => NormalizedValue::Option(Some(Box::new(value))),
            1 => NormalizedValue::List(
                super::super::list::List::from_items(vec![value], TEST_LIMIT, &mut |_| Ok(()))
                    .unwrap(),
            ),
            2 => NormalizedValue::Map(
                Map::from_items(
                    BTreeMap::from([(key(n as i64), value)]),
                    TEST_LIMIT,
                    &mut free,
                )
                .unwrap(),
            ),
            3 => NormalizedValue::Result {
                success: false,
                value: Box::new(value),
            },
            _ => NormalizedValue::Record(NormalizedRecord::Nominal {
                layout: RecordLayoutIndex(3, ValueOrigin::default()),
                fields: Arc::new(vec![value]),
            }),
        };
    }
    value
}

#[test]
fn persistent_map_deep_rejected_raw_values_and_partial_trees_dispose_iteratively() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let payload: Arc<[u8]> = Arc::from([23_u8; 17]);
            let map = Map::from_items(
                BTreeMap::from([(key(0), deep_raw(20_000, &payload)), (key(1), value(1))]),
                TEST_LIMIT,
                &mut free,
            )
            .unwrap();
            let retained = map.clone();
            let changed = map.insert(key(2), value(2), TEST_LIMIT, &mut free).unwrap();
            assert!(std::ptr::eq(
                map.get(&key(0)).unwrap(),
                changed.get(&key(0)).unwrap()
            ));
            drop(map);
            drop(changed);
            assert_eq!(retained.get(&key(1)), Some(&value(1)));
            drop(retained);
            assert_eq!(Arc::strong_count(&payload), 1);
            // Every construction boundary, including final cancellation after a full
            // tree, destroys both already-built subtrees and still-unread entries.
            for fail_at in 0..16 {
                let entries = (0..7).map(|n| (key(n), deep_raw(2048, &payload))).collect();
                let control = ExecutionControl::cancel_after_checks(fail_at);
                assert!(Map::from_items(entries, TEST_LIMIT, &mut |_| control.check()).is_err());
                assert_eq!(Arc::strong_count(&payload), 1);
            }
            // Refusal before entry allocation and after a rotated path was partly
            // built both own deep values safely, preserving the independent alias.
            let base = Map::from_items(
                BTreeMap::from([(key(0), value(0)), (key(1), value(1))]),
                TEST_LIMIT,
                &mut free,
            )
            .unwrap();
            let mut checks = 0;
            drop(
                base.insert(key(2), value(2), TEST_LIMIT, &mut |_| {
                    checks += 1;
                    Ok(())
                })
                .unwrap(),
            );
            for fail_at in 0..checks {
                let control = ExecutionControl::cancel_after_checks(fail_at);
                assert!(
                    base.insert(key(2), deep_raw(4096, &payload), TEST_LIMIT, &mut |_| {
                        control.check()
                    })
                    .is_err()
                );
                assert_eq!(Arc::strong_count(&payload), 1);
                assert_eq!(base.get(&key(0)), Some(&value(0)));
                assert_eq!(base.get(&key(1)), Some(&value(1)));
            }
            assert!(
                Map::default()
                    .insert(key(0), value(17), TEST_LIMIT, &mut free)
                    .is_ok()
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn persistent_map_updates_have_logarithmic_storage_work() {
    use std::time::Instant;
    for count in [8_i64, 128, 1000, 4000, 16000] {
        let mut baseline = BTreeMap::new();
        let mut copied_entries = 0_u64;
        let started = Instant::now();
        for n in 0..count {
            copied_entries += baseline.len() as u64;
            let mut next = baseline.clone();
            next.insert(key(n), value(n));
            baseline = next;
        }
        let baseline_ns = started.elapsed().as_nanos();
        let mut candidate = Map::default();
        let mut allocated = Charge::default();
        let started = Instant::now();
        for n in 0..count {
            let previous = candidate.clone();
            candidate = candidate
                .insert(key(n), value(n), TEST_LIMIT, &mut |charge| {
                    allocated.slots += charge.slots;
                    allocated.bytes += charge.bytes;
                    Ok(())
                })
                .unwrap();
            assert_eq!(previous.get(&key(n)), None);
        }
        let candidate_ns = started.elapsed().as_nanos();
        agrees(&candidate, &baseline);
        assert_eq!(copied_entries, (count * (count - 1) / 2) as u64);
        assert!(allocated.slots < count as u64 * 32);
        eprintln!(
            "map-storage count={count} baseline_ns={baseline_ns} candidate_ns={candidate_ns} baseline_copied_entries={copied_entries} candidate_nodes={} candidate_bytes={}",
            allocated.slots, allocated.bytes
        );
    }
}

#[test]
fn persistent_map_observation_saturation_neither_authorizes_nor_exhausts_storage() {
    struct Restore(Work);
    impl Drop for Restore {
        fn drop(&mut self) {
            WORK.set(self.0);
        }
    }
    let saturated = Work {
        node_visits: u64::MAX,
        nodes_allocated: u64::MAX,
        entry_handles_allocated: u64::MAX,
        entry_handle_copies: u64::MAX,
        key_bytes_copied: u64::MAX,
    };
    let _restore = Restore(WORK.replace(saturated));
    let selected = NormalizedMapKey::Text("probe".to_owned());
    let expected = independent_storage_schedule(1, 1, 5);
    let mut accepted = Charge::default();
    let map = Map::default()
        .insert_borrowed(&selected, value(17), 1, &mut |charge| {
            let next = Charge {
                slots: accepted.slots.checked_add(charge.slots).unwrap(),
                bytes: accepted.bytes.checked_add(charge.bytes).unwrap(),
            };
            if next.slots > expected.slots || next.bytes > expected.bytes {
                return Err(storage_error());
            }
            accepted = next;
            Ok(())
        })
        .unwrap();
    assert_eq!(accepted, expected);
    assert_eq!(Work::current(), saturated);
    let retained = map.clone();
    assert!(
        map.insert(selected.clone(), value(99), 1, &mut |charge| {
            if charge.bytes != 0 || charge.slots != 0 {
                Err(storage_error())
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    assert_eq!(map.get(&selected), Some(&value(17)));
    assert_eq!(retained, map);
    assert_eq!(Work::current(), saturated);
}

#[test]
fn persistent_map_equality_keeps_admitted_nested_payloads_on_a_bounded_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let nested = |leaf| {
                let mut payload = value(leaf);
                for _ in 0..255 {
                    payload = NormalizedValue::Map(
                        Map::default()
                            .insert(key(0), payload, TEST_LIMIT, &mut free)
                            .unwrap(),
                    );
                }
                payload
            };
            let original_child = nested(7);
            let original = Map::default()
                .insert(key(-1), original_child.clone(), TEST_LIMIT, &mut free)
                .unwrap()
                .insert(key(1), original_child, TEST_LIMIT, &mut free)
                .unwrap();
            let alias = original.clone();
            let independent_child = nested(7);
            let independently_built = Map::default()
                .insert(key(1), independent_child.clone(), TEST_LIMIT, &mut free)
                .unwrap()
                .insert(key(-1), independent_child, TEST_LIMIT, &mut free)
                .unwrap();
            // Equal logical depth-256 values have different root shapes and separately
            // constructed descendants. Each root retains two aliases of its child.
            assert!(original == independently_built);
            let changed = original
                .insert(key(1), nested(8), TEST_LIMIT, &mut free)
                .unwrap();
            assert!(original != changed);
            assert!(changed != independently_built);
            assert!(alias == independently_built);
            assert!(alias == original);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn persistent_map_retirement_uses_inline_or_preexisting_owned_storage() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut scalars = Map::from_items(
                (0..4096).map(|n| (key(n), value(n))).collect(),
                TEST_LIMIT,
                &mut free,
            )
            .unwrap();
            let mut work = RawValueWork::default();
            scalars.drain_unique(&mut work);
            assert_eq!(work.scratch_storage().1, 0);
            work.release();
            assert_eq!(work.scratch_storage().1, 0);

            // This crosses the list's maximum branch height, independently checking
            // its fixed traversal stack when a map owns a large terminal payload.
            let list = NormalizedValue::list((0..65_536).map(value).collect()).unwrap();
            let mut owner = Map::default()
                .insert(key(0), list, TEST_LIMIT, &mut free)
                .unwrap();
            owner.drain_unique(&mut work);
            work.release();
            assert_eq!(work.scratch_storage().1, 0);

            let payload: Arc<[u8]> = Arc::from([19_u8; 17]);
            let mut unary = NormalizedValue::Bytes(Arc::clone(&payload));
            for _ in 0..20_000 {
                unary = NormalizedValue::Option(Some(Box::new(unary)));
            }
            let mut owner = Map::default()
                .insert(key(0), unary, TEST_LIMIT, &mut free)
                .unwrap();
            owner.drain_unique(&mut work);
            work.release();
            assert_eq!(work.scratch_storage().1, 0);
            assert_eq!(Arc::strong_count(&payload), 1);

            let origin = ValueOrigin::default();
            for shared in [
                NormalizedValue::list(vec![value(7)]).unwrap(),
                NormalizedValue::map(BTreeMap::from([(key(0), value(7))])).unwrap(),
                NormalizedValue::Record(NormalizedRecord::Nominal {
                    layout: RecordLayoutIndex(0, origin),
                    fields: Arc::new(vec![value(7)]),
                }),
                NormalizedValue::Function {
                    function: FunctionIndex(0, origin),
                    type_arguments: Arc::from([]),
                    effect_arguments: Arc::from([]),
                    requirement_arguments: Arc::from([]),
                    bound_arguments: Some(Arc::new(vec![value(7)])),
                },
            ] {
                let retained = shared.clone();
                let mut owner = Map::default()
                    .insert(key(0), shared, TEST_LIMIT, &mut free)
                    .unwrap();
                owner.drain_unique(&mut work);
                assert_eq!(work.scratch_storage().1, 0);
                work.release();
                assert_eq!(work.scratch_storage().1, 0);
                let retained_child = match &retained {
                    NormalizedValue::List(values) => values.get(0),
                    NormalizedValue::Map(values) => values.get(&key(0)),
                    NormalizedValue::Record(NormalizedRecord::Nominal { fields, .. }) => {
                        fields.first()
                    }
                    NormalizedValue::Function {
                        bound_arguments: Some(values),
                        ..
                    } => values.first(),
                    _ => unreachable!(),
                };
                assert_eq!(retained_child, Some(&value(7)));
            }

            // A unique nominal record supplies its own already allocated child Vec.
            // The disposal owner adopts that exact buffer, including its capacity.
            let fields = (0..1024).map(value).collect::<Vec<_>>();
            let original_storage = (fields.as_ptr(), fields.capacity());
            let mut owner = Map::default()
                .insert(
                    key(0),
                    NormalizedValue::Record(NormalizedRecord::Nominal {
                        layout: RecordLayoutIndex(0, origin),
                        fields: Arc::new(fields),
                    }),
                    TEST_LIMIT,
                    &mut free,
                )
                .unwrap();
            owner.drain_unique(&mut work);
            work.release();
            assert_eq!(work.scratch_storage(), original_storage);
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn persistent_map_branching_raw_cleanup_bounds_scratch_by_released_children() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let payload: Arc<[u8]> = Arc::from([31_u8; 17]);
            let build = || {
                let mut raw = NormalizedValue::Bytes(Arc::clone(&payload));
                for _ in 0..2048 {
                    let sibling =
                        NormalizedValue::list(vec![NormalizedValue::Bytes(Arc::clone(&payload))])
                            .unwrap();
                    raw = NormalizedValue::Map(
                        Map::from_items(
                            BTreeMap::from([(key(0), raw), (key(1), sibling)]),
                            TEST_LIMIT,
                            &mut free,
                        )
                        .unwrap(),
                    );
                }
                raw
            };
            let mut work = RawValueWork::default();
            work.push(build());
            work.release();
            // One still-unreleased sibling per level forms the frontier. Inspect
            // the actual Vec capacity, independently of all carrier observations.
            let capacity = work.scratch_storage().1;
            assert!(capacity >= 2048);
            assert!(capacity < 2 * 2048 + 4);
            assert_eq!(Arc::strong_count(&payload), 1);

            let base = Map::from_items(BTreeMap::from([(key(0), value(7))]), TEST_LIMIT, &mut free)
                .unwrap();
            let retained = base.clone();
            assert!(
                base.insert(key(1), build(), TEST_LIMIT, &mut |_| Err(storage_error()))
                    .is_err()
            );
            assert_eq!(Arc::strong_count(&payload), 1);
            assert_eq!(retained.get(&key(0)), Some(&value(7)));
            assert!(retained == base);
        })
        .unwrap()
        .join()
        .unwrap();
}
