#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! Synthetic checked packets exercise real retained tokens and the actual runtime allocator.
//! These controls do not claim native authoring or canonical source admission.
use super::super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation};
use super::super::super::shared_budget::SharedBudget;
use super::super::{NormalizedRunObservation, NormalizedRunPolicy, reserve_execution_allocation};
use super::*;
use crate::platform::execution::ExecutionFailureClass;

fn fresh() -> ValueOrigin {
    ValueOrigin::fresh().unwrap()
}
fn owner(program: ValueOrigin, domain: ValueOrigin, marker: i64) -> OwnedI64Cell {
    let cell = OwnedI64Cell::new(domain, marker);
    cell.establish_admission(program).unwrap();
    cell
}
fn packet(program: ValueOrigin, cell: &OwnedI64Cell, guards: Vec<Value>) -> Value {
    Value {
        raw: NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
        origin: program,
        class: Class::Memory,
        ordinary_type: None,
        borrow: Some(Box::new(BorrowMetadata {
            source: None,
            placement: None,
            packet: None,
            guards,
            view_borrowed: false,
            path: None,
            projected: None,
            next_cleanup: None,
        })),
    }
}
fn chain(program: ValueOrigin, cell: &OwnedI64Cell, depth: usize) -> Value {
    let mut value = packet(program, cell, vec![]);
    for _ in 0..depth {
        value = packet(program, cell, vec![value]);
    }
    value
}
fn marker(value: &Value) -> i64 {
    let NormalizedValue::OwnedI64Cell(cell) = value.raw() else {
        panic!("real cell read required")
    };
    cell.read().unwrap()
}
fn walk(
    value: &Value,
    control: &ExecutionControl,
    visits: &mut usize,
    reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
) -> Result<(), ExecutionError> {
    value.visit_ancestor_reads(control, reserve, &mut |guard, _reserve| {
        assert_eq!(marker(guard), 77);
        *visits += 1;
        Ok(())
    })
}

#[test]
fn checked_read_ancestors_traverse_and_drop_twenty_four_thousand_live_guards() {
    let observed = StorageObservation::start();
    let program = fresh();
    let domain = fresh();
    let cell = owner(program, domain, 77);
    let value = chain(program, &cell, 24_000);
    assert_eq!(observed.live(), (1, 24_001));
    assert!(cell.validate(domain, true).is_err());
    let control = ExecutionControl::uncancelled();
    let mut visits = 0;
    let mut allocation = NormalizedRunObservation::default();
    walk(&value, &control, &mut visits, &mut |bytes| {
        reserve_execution_allocation(
            NormalizedRunPolicy::foreground(),
            None,
            &mut allocation,
            bytes,
        )
    })
    .unwrap();
    assert_eq!(visits, 24_000);
    assert!(allocation.allocated_bytes > 0);
    value
        .validate_read_custody(&control, &mut |bytes| {
            reserve_execution_allocation(
                NormalizedRunPolicy::foreground(),
                None,
                &mut allocation,
                bytes,
            )
        })
        .unwrap();
    assert_eq!(observed.live(), (1, 24_001));
    drop(value);
    assert_eq!(observed.live(), (1, 0));
    assert_eq!(cell.extract().unwrap(), 77);
    assert_eq!(observed.live(), (0, 0));
}

#[test]
fn checked_read_ancestors_preserve_complete_branching_preorder() {
    let observed = StorageObservation::start();
    let program = fresh();
    let domain = fresh();
    let cells = (0..6)
        .map(|n| owner(program, domain, n))
        .collect::<Vec<_>>();
    let first = packet(
        program,
        &cells[1],
        vec![
            packet(program, &cells[3], vec![]),
            packet(program, &cells[4], vec![]),
        ],
    );
    let second = packet(program, &cells[2], vec![packet(program, &cells[5], vec![])]);
    let value = packet(program, &cells[0], vec![first, second]);
    let mut order = vec![];
    value
        .visit_ancestor_reads(
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
            &mut |guard, _reserve| {
                order.push(marker(guard));
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(order, [1, 3, 4, 2, 5]);
    assert_eq!(observed.live(), (6, 6));
    drop(value);
    assert_eq!(observed.live(), (6, 0));
    drop(cells);
    assert_eq!(observed.live(), (0, 0));
}

#[test]
fn checked_read_ancestor_cursor_exact_shared_quota_and_one_short_keep_custody() {
    let observed = StorageObservation::start();
    let program = fresh();
    let domain = fresh();
    let cell = owner(program, domain, 77);
    let value = chain(program, &cell, 300);
    let control = ExecutionControl::uncancelled();
    let mut measured = NormalizedRunObservation::default();
    let mut visits = 0;
    walk(&value, &control, &mut visits, &mut |bytes| {
        reserve_execution_allocation(
            NormalizedRunPolicy::foreground(),
            None,
            &mut measured,
            bytes,
        )
    })
    .unwrap();
    assert_eq!(visits, 300);
    let healthy = measured.allocated_bytes;
    assert!(healthy > 0);
    // Every geometric replacement is charged in full while its predecessor remains live.
    let width = std::mem::size_of::<std::slice::Iter<'_, Value>>() as u64;
    assert_eq!(
        healthy,
        (1 + 2 + 4 + 8 + 16 + 32 + 64 + 128 + 256 + 512) * width
    );
    let base = 7;
    let policy = NormalizedRunPolicy {
        maximum_allocated_bytes: Some(base + healthy),
        ..NormalizedRunPolicy::foreground()
    };
    let budget = SharedBudget::new(policy, None, base, 0);
    let mut exact = NormalizedRunObservation {
        allocated_bytes: base,
        ..Default::default()
    };
    visits = 0;
    walk(&value, &control, &mut visits, &mut |bytes| {
        reserve_execution_allocation(policy, Some(&budget), &mut exact, bytes)
    })
    .unwrap();
    assert_eq!(visits, 300);
    assert_eq!(exact.allocated_bytes, base + healthy);
    // A second traversal shares the exhausted ledger; dropping scratch never refunds quota.
    visits = 0;
    let error = walk(&value, &control, &mut visits, &mut |bytes| {
        reserve_execution_allocation(policy, Some(&budget), &mut exact, bytes)
    })
    .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "normalized_allocation");
    assert_eq!(visits, 0);
    for shared_one_short in [false, true] {
        let local_policy = NormalizedRunPolicy {
            maximum_allocated_bytes: Some(base + healthy - u64::from(!shared_one_short)),
            ..NormalizedRunPolicy::foreground()
        };
        let shared_policy = NormalizedRunPolicy {
            maximum_allocated_bytes: Some(base + healthy - u64::from(shared_one_short)),
            ..NormalizedRunPolicy::foreground()
        };
        let budget = SharedBudget::new(shared_policy, None, base, 0);
        let mut used = NormalizedRunObservation {
            allocated_bytes: base,
            ..Default::default()
        };
        visits = 0;
        let error = walk(&value, &control, &mut visits, &mut |bytes| {
            reserve_execution_allocation(local_policy, Some(&budget), &mut used, bytes)
        })
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        assert_eq!(error.code, "normalized_allocation");
        assert!(visits < 300);
        assert!(used.allocated_bytes < base + healthy);
        assert_eq!(observed.live(), (1, 301));
    }
    let zero = NormalizedRunPolicy {
        maximum_allocated_bytes: Some(base),
        ..NormalizedRunPolicy::foreground()
    };
    let mut denied = NormalizedRunObservation {
        allocated_bytes: base,
        ..Default::default()
    };
    visits = 0;
    assert!(
        walk(&value, &control, &mut visits, &mut |bytes| {
            reserve_execution_allocation(zero, None, &mut denied, bytes)
        })
        .is_err()
    );
    assert_eq!(visits, 0);
    assert_eq!(denied.allocated_bytes, base);
    visits = 0;
    walk(&value, &control, &mut visits, &mut |_| Ok(())).unwrap();
    assert_eq!(visits, 300);
    drop(value);
    assert_eq!(observed.live(), (1, 0));
    assert_eq!(cell.extract().unwrap(), 77);
    assert_eq!(observed.live(), (0, 0));
}

#[test]
fn checked_read_ancestor_cancel_at_every_walk_phase_preserves_packet_and_retry() {
    let observed = StorageObservation::start();
    let program = fresh();
    let domain = fresh();
    let cell = owner(program, domain, 77);
    let value = chain(program, &cell, 32);
    let mut completed = false;
    for cutoff in 0..1024 {
        let control = ExecutionControl::cancel_after_checks(cutoff);
        let mut visits = 0;
        let mut allocation = NormalizedRunObservation::default();
        match walk(&value, &control, &mut visits, &mut |bytes| {
            reserve_execution_allocation(
                NormalizedRunPolicy::foreground(),
                None,
                &mut allocation,
                bytes,
            )
        }) {
            Ok(()) => {
                assert_eq!(visits, 32);
                completed = true;
                break;
            }
            Err(error) => {
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                assert_eq!(error.code, "execution_cancelled");
                assert_eq!(observed.live(), (1, 33));
                assert!(cell.validate(domain, true).is_err());
                let mut retry = 0;
                walk(
                    &value,
                    &ExecutionControl::uncancelled(),
                    &mut retry,
                    &mut |_| Ok(()),
                )
                .unwrap();
                assert_eq!(retry, 32);
            }
        }
    }
    assert!(
        completed,
        "all actual cancellation cutoffs reached the first full completion"
    );
    drop(value);
    assert_eq!(observed.live(), (1, 0));
    assert_eq!(cell.extract().unwrap(), 77);
    assert_eq!(observed.live(), (0, 0));
}

#[test]
fn checked_read_ancestor_visitor_failure_never_detaches_actual_tokens() {
    let observed = StorageObservation::start();
    let program = fresh();
    let domain = fresh();
    let cell = owner(program, domain, 77);
    let value = chain(program, &cell, 600);
    let mut seen = 0;
    let error = value
        .visit_ancestor_reads(
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
            &mut |guard, _reserve| {
                assert_eq!(marker(guard), 77);
                seen += 1;
                if seen == 513 {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Trap,
                        "test_ancestor_failure",
                        "injected independent ancestor visitor refusal",
                    ));
                }
                Ok(())
            },
        )
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Trap);
    assert_eq!(error.code, "test_ancestor_failure");
    assert_eq!(seen, 513);
    assert_eq!(observed.live(), (1, 601));
    let mut retry = 0;
    walk(
        &value,
        &ExecutionControl::uncancelled(),
        &mut retry,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(retry, 600);
    drop(value);
    assert_eq!(observed.live(), (1, 0));
    assert_eq!(cell.extract().unwrap(), 77);
    assert_eq!(observed.live(), (0, 0));
}
