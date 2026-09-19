//! Implementation-disjoint ordered oracle, retained versions and allocation failures.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;

fn free(_: Charge) -> Result<(), ExecutionError> {
    Ok(())
}

fn key(value: i64) -> NormalizedMapKey {
    NormalizedMapKey::I64(value)
}

fn value(value: i64) -> NormalizedValue {
    NormalizedValue::I64(value)
}

fn shape(root: &Link) -> (usize, usize) {
    let Some(root) = root else {
        return (0, 0);
    };
    let (left_height, left_count) = shape(&root.left);
    let (right_height, right_count) = shape(&root.right);
    assert!(left_height.abs_diff(right_height) <= 1);
    assert_eq!(root.height, left_height.max(right_height) + 1);
    if let Some(left) = &root.left {
        assert!(left.entry.key < root.entry.key);
    }
    if let Some(right) = &root.right {
        assert!(root.entry.key < right.entry.key);
    }
    (root.height, left_count + right_count + 1)
}

fn agrees(map: &Map, expected: &BTreeMap<NormalizedMapKey, NormalizedValue>) {
    assert_eq!(map.len(), expected.len());
    assert_eq!(map.is_empty(), expected.is_empty());
    assert_eq!(shape(&map.root).1, expected.len());
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
        assert!(map.contains_key(key));
    }
}

#[test]
fn persistent_map_order_snapshots_and_rebalancing_match_btree() {
    let mut map = Map::default();
    let mut oracle = BTreeMap::new();
    let mut retained = Vec::new();
    let mut random = 0x123456789abcdef0_u64;
    for step in 0..8192 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        let selected = key((random % 1024) as i64);
        if random & 3 == 0 {
            map = map.remove(&selected, &mut free).unwrap();
            oracle.remove(&selected);
        } else {
            map = map
                .insert(selected.clone(), value(step), 1_000_000, &mut free)
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
fn persistent_map_bulk_order_and_all_key_kinds_are_canonical() {
    let entries = BTreeMap::from([
        (NormalizedMapKey::Bool(false), value(1)),
        (NormalizedMapKey::Bool(true), value(2)),
        (key(i64::MIN), value(3)),
        (key(i64::MAX), value(4)),
        (NormalizedMapKey::Bytes(vec![0, 255]), value(5)),
        (NormalizedMapKey::Bytes(vec![255]), value(6)),
        (NormalizedMapKey::Text("".to_owned()), value(7)),
        (NormalizedMapKey::Text("日本語".to_owned()), value(8)),
    ]);
    let bulk = Map::from_items(entries.clone(), &mut free).unwrap();
    let mut incremental = Map::default();
    for (key, value) in entries.iter().rev() {
        incremental = incremental
            .insert(key.clone(), value.clone(), 8, &mut free)
            .unwrap();
    }
    agrees(&bulk, &entries);
    agrees(&incremental, &entries);
    assert_eq!(bulk, incremental);
    assert_ne!(bulk, Map::default());
}

#[test]
fn persistent_map_failed_reservations_preserve_all_versions() {
    let expected = (0..256)
        .map(|n| (key(n), value(n)))
        .collect::<BTreeMap<_, _>>();
    let map = Map::from_items(expected.clone(), &mut free).unwrap();
    let retained = map.clone();
    for removing in [false, true] {
        let mut reservations = 0;
        let mut measure = |_: Charge| {
            reservations += 1;
            Ok(())
        };
        let result = if removing {
            map.remove(&key(128), &mut measure)
        } else {
            map.insert(key(300), value(300), 1_000_000, &mut measure)
        };
        result.unwrap();
        for fail_at in 0..reservations {
            let mut calls = 0;
            let mut reject = |_: Charge| {
                let current = calls;
                calls += 1;
                if current == fail_at {
                    Err(storage_error())
                } else {
                    Ok(())
                }
            };
            let result = if removing {
                map.remove(&key(128), &mut reject)
            } else {
                map.insert(key(300), value(300), 1_000_000, &mut reject)
            };
            assert!(result.is_err());
            agrees(&map, &expected);
            agrees(&retained, &expected);
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
fn persistent_map_updates_have_logarithmic_storage_work() {
    use std::time::Instant;
    for count in [1000_i64, 4000, 16000] {
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
                .insert(key(n), value(n), 1_000_000, &mut |charge| {
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
