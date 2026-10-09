//! Refusal, cancellation, and stack-safe ownership at every bulk reservation.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::super::value::{NormalizedRecord, RecordLayoutIndex, ValueOrigin};
use super::bulk_tests::{items, model};
use super::*;
use crate::platform::execution::ExecutionControl;

fn payload(origin: ValueOrigin, sentinel: &Arc<Vec<NormalizedValue>>) -> NormalizedValue {
    NormalizedValue::Record(NormalizedRecord::Nominal {
        layout: RecordLayoutIndex(7, origin),
        fields: Arc::clone(sentinel),
    })
}

#[test]
fn bulk_every_reservation_failure_releases_attached_and_pending_raw_owners() {
    let origin = ValueOrigin::fresh().unwrap();
    let sentinel = Arc::new(vec![NormalizedValue::Unit]);
    for length in [0, 33, 65, 1057] {
        let (leaves, branches, _) = model(length);
        let calls = length as u64 + leaves + branches + 1;
        for refused in 1..=calls {
            let raw = (0..length).map(|_| payload(origin, &sentinel)).collect();
            let mut visited = 0;
            let result = List::from_items(raw, MAXIMUM_LENGTH as u64, &mut |_| {
                visited += 1;
                if visited == refused {
                    Err(failure())
                } else {
                    Ok(())
                }
            });
            assert!(result.is_err(), "length={length} refused={refused}");
            assert_eq!(visited, refused);
            assert_eq!(
                Arc::strong_count(&sentinel),
                1,
                "attached or pending payload leaked: length={length} refused={refused}"
            );
        }
        let list = List::from_items(vec![payload(origin, &sentinel)], 1, &mut |_| Ok(())).unwrap();
        assert_eq!(Arc::strong_count(&sentinel), 2);
        drop(list);
        assert_eq!(Arc::strong_count(&sentinel), 1);
    }
}

#[test]
fn bulk_cancel_at_every_reservation_including_empty_and_final_installation() {
    let origin = ValueOrigin::fresh().unwrap();
    let sentinel = Arc::new(vec![NormalizedValue::Unit]);
    for length in [0, 65, 1057] {
        let (leaves, branches, _) = model(length);
        let calls = length as u64 + leaves + branches + 1;
        for cancelled in 1..=calls {
            let control = ExecutionControl::uncancelled();
            let mut visited = 0;
            let result = List::from_items(
                (0..length).map(|_| payload(origin, &sentinel)).collect(),
                MAXIMUM_LENGTH as u64,
                &mut |_| {
                    visited += 1;
                    if visited == cancelled {
                        control.cancel();
                    }
                    control.check()
                },
            );
            assert!(result.is_err());
            assert_eq!(visited, cancelled);
            assert!(control.is_cancelled());
            assert_eq!(Arc::strong_count(&sentinel), 1);
        }
    }
}

#[test]
fn bulk_length_refusal_precedes_storage_and_releases_raw_payloads() {
    let origin = ValueOrigin::fresh().unwrap();
    let sentinel = Arc::new(vec![NormalizedValue::Unit]);
    for (length, maximum) in [(1, 0), (33, 32), (MAXIMUM_LENGTH + 1, u64::MAX)] {
        let raw = (0..length).map(|_| payload(origin, &sentinel)).collect();
        let before = Work::current();
        let result = List::from_items(raw, maximum, &mut |_| panic!("length admits no storage"));
        assert!(result.is_err());
        assert_eq!(before.since(), Work::ZERO);
        assert_eq!(Arc::strong_count(&sentinel), 1);
    }
}

#[test]
fn bulk_deep_pending_and_attached_payloads_drop_on_a_bounded_stack() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let origin = ValueOrigin::fresh().unwrap();
            let sentinel = Arc::new(vec![NormalizedValue::Unit]);
            let length = 1057;
            let (leaves, branches, _) = model(length);
            let calls = length as u64 + leaves + branches + 1;
            for position in [0, 31, 32, 63, 64, 1056] {
                for refused in [0, 1, 2, 3, 35, 36, 37, 70, calls] {
                    let mut raw = items(length);
                    let mut deep = payload(origin, &sentinel);
                    for _ in 0..20_000 {
                        deep = NormalizedValue::Option(Some(Box::new(deep)));
                    }
                    raw[position] = deep;
                    let mut visited = 0;
                    let result = List::from_items(raw, length as u64, &mut |_| {
                        visited += 1;
                        if visited == refused {
                            Err(failure())
                        } else {
                            Ok(())
                        }
                    });
                    assert_eq!(result.is_ok(), refused == 0);
                    drop(result);
                    assert_eq!(
                        Arc::strong_count(&sentinel),
                        1,
                        "position={position} refused={refused}"
                    );
                }
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

#[test]
fn bulk_refusal_classes_release_real_cells_without_touching_an_unrelated_owner() {
    use super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation};
    let origin = ValueOrigin::fresh().unwrap();
    for length in [1, 65, 1057] {
        let (leaves, branches, _) = model(length);
        let calls = length as u64 + leaves + branches + 1;
        let mut points = vec![1, 2, 3, calls / 2, calls];
        points.sort_unstable();
        points.dedup();
        for refused in points {
            for cancel in [false, true] {
                let observed = StorageObservation::start();
                let unrelated = OwnedI64Cell::new(origin, 101);
                let raw = (0..length)
                    .map(|i| NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, i as i64)))
                    .collect();
                assert_eq!(observed.live(), (length + 1, 0));
                let control = ExecutionControl::uncancelled();
                let mut visited = 0;
                let error = List::from_items(raw, length as u64, &mut |_| {
                    visited += 1;
                    if visited == refused {
                        if cancel {
                            control.cancel();
                        } else {
                            return Err(failure());
                        }
                    }
                    control.check()
                })
                .unwrap_err();
                assert_eq!(visited, refused);
                assert_eq!(
                    error.class,
                    if cancel {
                        ExecutionFailureClass::Cancelled
                    } else {
                        ExecutionFailureClass::Resource
                    }
                );
                assert_eq!(
                    error.code,
                    if cancel {
                        "execution_cancelled"
                    } else {
                        "normalized_list_storage"
                    }
                );
                assert!(!error.retryable && !error.possibly_visible);
                assert_eq!(observed.created(), length + 1);
                assert_eq!(observed.live(), (1, 0));
                assert_eq!(unrelated.read().unwrap(), 101);
                drop(unrelated);
                assert_eq!(observed.live(), (0, 0));
            }
        }
    }
}
