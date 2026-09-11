//! Disjoint flat sequence and allocation models. Never used by production evaluation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;

fn append(list: &List, value: i64) -> List {
    list.append(
        NormalizedValue::I64(value),
        MAXIMUM_LENGTH as u64,
        &mut |_| Ok(()),
    )
    .unwrap()
}

fn verify(list: &List, expected: &[i64]) -> Result<(), String> {
    if list.len() != expected.len() {
        return Err("length".into());
    }
    for (index, value) in expected.iter().enumerate() {
        if list.get(index) != Some(&NormalizedValue::I64(*value)) {
            return Err(format!("index {index}"));
        }
    }
    if list.get(expected.len()).is_some() || list.get(usize::MAX).is_some() {
        return Err("out of bounds".into());
    }
    let forward = list
        .iter()
        .map(|v| match v {
            NormalizedValue::I64(n) => *n,
            _ => i64::MIN,
        })
        .collect::<Vec<_>>();
    let reverse = list
        .iter()
        .rev()
        .map(|v| match v {
            NormalizedValue::I64(n) => *n,
            _ => i64::MIN,
        })
        .collect::<Vec<_>>();
    if forward != expected || reverse != expected.iter().rev().copied().collect::<Vec<_>>() {
        return Err("ordered traversal".into());
    }
    let mut both = list.iter();
    let mut front = 0;
    let mut back = expected.len();
    while front < back {
        assert_eq!(both.next(), Some(&NormalizedValue::I64(expected[front])));
        front += 1;
        if front < back {
            back -= 1;
            assert_eq!(
                both.next_back(),
                Some(&NormalizedValue::I64(expected[back]))
            );
        }
    }
    if both.next().is_some() || both.next_back().is_some() {
        return Err("iterator overlap".into());
    }
    Ok(())
}

// Derived independently of the storage charge producer: fixed 32 pointer slots,
// one enum discriminant, two Arc counters; element payload plus two counters.
fn schedule(length: usize) -> (u64, u64) {
    let prefix_leaves = length.saturating_sub(1) / 32;
    let mut height = 0;
    let mut capacity = 1;
    while prefix_leaves > capacity {
        capacity *= 32;
        height += 1;
    }
    let branches = if length != 0 && length.is_multiple_of(32) && prefix_leaves != 0 {
        if prefix_leaves == capacity {
            height + 1
        } else {
            height
        }
    } else {
        0
    };
    let node_bytes = 32 * std::mem::size_of::<usize>() + 3 * std::mem::size_of::<usize>();
    assert_eq!(
        std::mem::size_of::<Node>() + 2 * std::mem::size_of::<usize>(),
        node_bytes
    );
    let bytes = (branches + 1) * node_bytes
        + std::mem::size_of::<NormalizedValue>()
        + 2 * std::mem::size_of::<usize>();
    (((branches + 1) * 32) as u64, bytes as u64)
}

#[test]
fn retained_roots_match_flat_oracle_across_tail_and_root_growth() {
    let mut list = List::default();
    let mut expected = Vec::new();
    let mut retained = Vec::new();
    for n in 0..=32_801 {
        if [
            0, 1, 31, 32, 33, 63, 64, 65, 1023, 1024, 1055, 1056, 1057, 32799, 32800, 32801,
        ]
        .contains(&n)
        {
            verify(&list, &expected).unwrap();
            retained.push((list.clone(), expected.clone()));
            let bulk = List::from_items(
                expected.iter().copied().map(NormalizedValue::I64).collect(),
                MAXIMUM_LENGTH as u64,
                &mut |_| Ok(()),
            )
            .unwrap();
            assert_eq!(list, bulk);
            verify(&bulk, &expected).unwrap();
        }
        list = append(&list, n as i64);
        expected.push(n as i64);
    }
    for (list, expected) in retained {
        let a = append(&list, -17);
        let b = append(&list, 91);
        verify(&list, &expected).unwrap();
        verify(&a, &[expected.as_slice(), &[-17]].concat()).unwrap();
        verify(&b, &[expected.as_slice(), &[91]].concat()).unwrap();
    }
}

#[test]
fn deterministic_branching_histories_and_wrong_tail_fault_sensitivity() {
    for seed in [0x18a9_19bf_0721_u64, 0x8a17_ff09_4201, 0x329a_eda0_971f] {
        let mut random = seed;
        let mut roots = vec![(List::default(), Vec::new())];
        for step in 0..600 {
            random = random
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            let index = (random as usize) % roots.len();
            let (list, model) = &roots[index];
            let value = (random >> 32) as i64 - step;
            let successor = append(list, value);
            let mut expected = model.clone();
            expected.push(value);
            roots.push((successor, expected));
            for (list, model) in &roots {
                verify(list, model).unwrap();
            }
        }
        let (list, model) = roots.last().unwrap();
        let mut wrong = list.clone();
        let mut tail = leaf(wrong.tail.as_deref(), &mut |_| Ok(())).unwrap();
        tail[0] = Some(Arc::new(Element(NormalizedValue::I64(i64::MIN))));
        wrong.tail = Some(node(Node::Leaf(tail)));
        assert!(
            verify(&wrong, model).is_err(),
            "the shared-carrier wrong-tail fault must be detected"
        );
        verify(list, model).unwrap();
    }
}

#[test]
fn aliased_append_obeys_independent_cost_schedule_and_payload_sharing() {
    let mut list = List::default();
    let mut total_slots = 0;
    for length in 0..8192 {
        let alias = list.clone();
        let before = Work::current();
        let mut charged = Charge::default();
        let next = list
            .append(
                NormalizedValue::I64(length as i64),
                MAXIMUM_LENGTH as u64,
                &mut |charge| {
                    charged.slots += charge.slots;
                    charged.bytes += charge.bytes;
                    Ok(())
                },
            )
            .unwrap();
        let work = before.since();
        assert_eq!(
            (charged.slots, charged.bytes),
            schedule(length),
            "length={length}"
        );
        assert_eq!(work.element_handle_allocations, 1);
        assert!(work.element_handle_copies <= 31);
        assert_eq!(work.element_slots_reserved, 32);
        assert!(work.branch_slots_reserved <= 32 * 4);
        assert!(work.node_visits <= 4);
        assert_eq!(work.full_materializations, 0);
        assert_eq!(work.materialized_elements, 0);
        assert_eq!(alias.len(), length);
        if length > 0 {
            assert!(std::ptr::eq(alias.get(0).unwrap(), next.get(0).unwrap()));
        }
        total_slots += charged.slots;
        list = next;
    }
    assert!(total_slots < 300_000);
    let before = Work::current();
    assert_eq!(list.iter().count(), 8192);
    let work = before.since();
    assert_eq!(work.node_visits, 265); // 256 leaves, 8 bottom branches, one root.
    let before = Work::current();
    for index in 0..8192 {
        assert_eq!(list.get(index), Some(&NormalizedValue::I64(index as i64)));
    }
    assert_eq!(before.since().node_visits, 8160 * 3 + 32);
}

#[test]
fn physical_slot_byte_length_failure_and_partial_path_preserve_aliases() {
    let mut list = List::default();
    for index in 0..1056 {
        list = append(&list, index);
    }
    let expected = (0..1056).collect::<Vec<_>>();
    let (slots, bytes) = schedule(list.len());
    for (slot_limit, byte_limit, success) in [
        (slots, bytes, true),
        (slots - 1, bytes, false),
        (slots, bytes - 1, false),
    ] {
        let mut used = Charge::default();
        let result = list.append(
            NormalizedValue::I64(1056),
            MAXIMUM_LENGTH as u64,
            &mut |charge| {
                let next = Charge {
                    slots: used.slots + charge.slots,
                    bytes: used.bytes + charge.bytes,
                };
                if next.slots > slot_limit || next.bytes > byte_limit {
                    return Err(failure());
                }
                used = next;
                Ok(())
            },
        );
        assert_eq!(result.is_ok(), success);
        verify(&list, &expected).unwrap();
    }
    let mut progressed = 0;
    assert!(
        list.append(NormalizedValue::I64(-1), MAXIMUM_LENGTH as u64, &mut |_| {
            progressed += 1;
            if progressed == 3 {
                Err(failure())
            } else {
                Ok(())
            }
        })
        .is_err()
    );
    assert_eq!(progressed, 3);
    assert!(
        list.append(NormalizedValue::Unit, 1056, &mut |_| Ok(()))
            .is_err()
    );
    assert!(
        list.append(NormalizedValue::Unit, 1057, &mut |_| Ok(()))
            .is_ok()
    );
    verify(&list, &expected).unwrap();
    verify(&append(&list, 1056), &(0..1057).collect::<Vec<_>>()).unwrap();
}

#[test]
fn nested_shared_payloads_are_not_cloned_and_deep_rejection_disposes_iteratively() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let mut value = NormalizedValue::I64(17);
            for _ in 0..20_000 {
                value = NormalizedValue::Option(Some(Box::new(value)));
                value = NormalizedValue::List(
                    List::from_items(vec![value], MAXIMUM_LENGTH as u64, &mut |_| Ok(())).unwrap(),
                );
            }
            let list =
                List::from_items(vec![value], MAXIMUM_LENGTH as u64, &mut |_| Ok(())).unwrap();
            let other = list
                .append(
                    NormalizedValue::Bool(true),
                    MAXIMUM_LENGTH as u64,
                    &mut |_| Ok(()),
                )
                .unwrap();
            assert!(std::ptr::eq(list.get(0).unwrap(), other.get(0).unwrap()));
            drop(list);
            drop(other);
            assert_eq!(
                append(&List::default(), 42).get(0),
                Some(&NormalizedValue::I64(42))
            );
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn flat_oracle_preserves_nested_nominal_and_transient_callable_elements() {
    use super::super::value::{FunctionIndex, NormalizedRecord, RecordLayoutIndex, ValueOrigin};
    let origin = ValueOrigin::fresh().unwrap();
    let model = vec![
        NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: RecordLayoutIndex(7, origin),
            fields: Arc::new(vec![NormalizedValue::Option(Some(Box::new(
                NormalizedValue::list(vec![NormalizedValue::I64(17)]).unwrap(),
            )))]),
        }),
        NormalizedValue::Function {
            effect_arguments: Arc::from([]),
            function: FunctionIndex(3, origin),
            type_arguments: Arc::from([]),
            bound_arguments: Some(Arc::new(vec![NormalizedValue::I64(5)])),
        },
    ];
    let bulk = List::from_items(model.clone(), MAXIMUM_LENGTH as u64, &mut |_| Ok(())).unwrap();
    let mut appended = List::default();
    for value in &model {
        appended = appended
            .append(value.clone(), MAXIMUM_LENGTH as u64, &mut |_| Ok(()))
            .unwrap();
    }
    // The carrier grants no equality/capture/durable permission: this compares raw test
    // structures only. Independent evaluator admission tests own those decisions.
    for root in [bulk, appended] {
        let left = root
            .append(
                NormalizedValue::Bool(true),
                MAXIMUM_LENGTH as u64,
                &mut |_| Ok(()),
            )
            .unwrap();
        let right = root
            .append(NormalizedValue::I64(-1), MAXIMUM_LENGTH as u64, &mut |_| {
                Ok(())
            })
            .unwrap();
        for (i, expected) in model.iter().enumerate() {
            assert_eq!(root.get(i), Some(expected));
            assert_eq!(left.get(i), Some(expected));
            assert_eq!(right.get(i), Some(expected));
            assert!(std::ptr::eq(root.get(i).unwrap(), left.get(i).unwrap()));
        }
        assert_eq!(root.len(), 2);
        assert_eq!(left.get(2), Some(&NormalizedValue::Bool(true)));
        assert_eq!(right.get(2), Some(&NormalizedValue::I64(-1)));
    }
}

#[test]
fn maximum_logical_length_rejects_before_new_storage() {
    let list = List::from_items(
        vec![NormalizedValue::Unit; 1_000_000],
        MAXIMUM_LENGTH as u64,
        &mut |_| Ok(()),
    )
    .unwrap();
    let before = Work::current();
    assert!(
        list.append(NormalizedValue::Unit, u64::MAX, &mut |_| panic!(
            "length must reject before storage reservation"
        ))
        .is_err()
    );
    assert_eq!(before.since(), Work::default());
    assert_eq!(list.len(), 1_000_000);
    assert_eq!(list.get(999_999), Some(&NormalizedValue::Unit));
    assert_eq!(list.get(1_000_000), None);
}
