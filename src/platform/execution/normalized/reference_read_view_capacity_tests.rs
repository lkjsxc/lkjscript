//! Small proof fixtures and explicit meter prefixes isolate admission policy.
use super::*;

type TestResult<T = ()> = std::result::Result<T, String>;

fn checked<T, E: std::fmt::Debug>(result: std::result::Result<T, E>) -> TestResult<T> {
    result.map_err(|error| format!("{error:?}"))
}

fn failed<T>(result: Result<T>) -> TestResult<ExecutionError> {
    match result {
        Err(error) => Ok(error),
        Ok(_) => Err("proof operation unexpectedly succeeded".to_owned()),
    }
}

#[test]
fn reference_view_unbounded_reconstruction_accepts_a_prefix_beyond_former_policy() -> TestResult {
    let fixture = constructor(true);
    let control = ExecutionControl::uncancelled();
    let mut budget = Budget::new(&control);
    // This is an exact synthetic meter prefix, not a claim that the small fixture
    // performs ten million operations. Real reconstruction then uses that budget.
    let former = Observation::from_usize(kernel::contract::MAXIMUM_VALIDATION_WORK);
    checked(budget.work.charge(former))?;
    checked(budget.reserve(kernel::contract::MAXIMUM_VALIDATION_WORK))?;
    let (mut proof, budget) =
        checked(Proof::reconstruct_raw(&[&fixture.source], &control, budget))?;
    assert_eq!(budget.work.remaining(), None);
    assert_eq!(budget.entries.remaining(), None);
    assert!(budget.work.observation().consumed > former.consumed);
    assert!(budget.entries.observation().consumed > former.consumed);
    assert!(!budget.work.observation().saturated);
    assert!(!budget.entries.observation().saturated);
    checked(proof.prepare_complete_closure(budget))?;
    assert!(proof.prepared_layout(fixture.ty).is_ok());

    let mut finite = Budget::with_limits(&control, Some(former.consumed), None);
    checked(finite.work.charge(former))?;
    let error = failed(Proof::reconstruct_raw(&[&fixture.source], &control, finite))?;
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "reference_read_view_proof_work");
    assert!(proof.prepared_layout(fixture.ty).is_ok());
    Ok(())
}

#[test]
fn reference_view_work_and_entry_allowances_are_exact_before_consumption() -> TestResult {
    let control = ExecutionControl::uncancelled();
    for limit in [None, Some(0), Some(2), Some(3), Some(u64::MAX)] {
        let mut budget = Budget::with_limits(&control, limit, limit);
        let before_entries = budget.entries;
        let result = budget.reserve(3);
        if limit.is_some_and(|limit| limit < 3) {
            assert_eq!(failed(result)?.code, "reference_read_view_proof_storage");
            assert_eq!(budget.entries, before_entries);
        } else {
            checked(result)?;
            assert_eq!(budget.entries.remaining(), limit.map(|limit| limit - 3));
            assert_eq!(budget.entries.observation(), Observation::exact(3));
        }
        for completed in 0..3_u64 {
            let before = budget.work;
            let result = budget.step_iterative();
            if limit.is_some_and(|limit| limit <= completed) {
                assert_eq!(failed(result)?.code, "reference_read_view_proof_work");
                assert_eq!(budget.work, before);
                break;
            }
            checked(result)?;
            assert_eq!(budget.work.observation(), Observation::exact(completed + 1));
        }
    }
    Ok(())
}

#[test]
fn reference_view_saturation_is_passive_and_depth_guards_remain() -> TestResult {
    let control = ExecutionControl::uncancelled();
    let mut budget = Budget::new(&control);
    checked(budget.work.charge(Observation::exact(u64::MAX)))?;
    checked(budget.reserve_observed(Observation::exact(u64::MAX)))?;
    checked(budget.step_iterative())?;
    checked(budget.reserve(1))?;
    assert!(budget.work.observation().saturated);
    assert!(budget.entries.observation().saturated);
    let before = budget.work;
    let error = failed(budget.step(kernel::contract::MAXIMUM_TYPE_DEPTH + 1))?;
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(budget.work, before);
    checked(budget.step(0))?;
    let mut finite = Budget::with_limits(&control, Some(u64::MAX), Some(u64::MAX));
    checked(finite.work.charge(Observation::exact(u64::MAX)))?;
    checked(finite.reserve_observed(Observation::exact(u64::MAX)))?;
    assert_eq!(
        failed(finite.step(0))?.code,
        "reference_read_view_proof_work"
    );
    assert_eq!(
        failed(finite.reserve(1))?.code,
        "reference_read_view_proof_storage"
    );
    assert!(!finite.work.observation().saturated);
    assert!(!finite.entries.observation().saturated);
    Ok(())
}

#[test]
fn reference_view_cancelled_reservation_leaves_both_meters_unchanged() -> TestResult {
    let control = ExecutionControl::uncancelled();
    let mut budget = Budget::with_limits(&control, Some(0), Some(0));
    let work = budget.work;
    let entries = budget.entries;
    control.cancel();
    assert_eq!(
        failed(budget.step_iterative())?.class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(
        failed(budget.reserve(usize::MAX))?.class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(budget.work, work);
    assert_eq!(budget.entries, entries);
    Ok(())
}

#[test]
fn reference_view_modeled_sums_and_products_are_not_buffer_extents() -> TestResult {
    let control = ExecutionControl::uncancelled();
    let mut budget = Budget::new(&control);
    checked(budget.reserve_counts(&[usize::MAX, 1]))?;
    assert_eq!(
        budget.entries.observation(),
        Observation::sum_counts(&[usize::MAX, 1])
    );
    checked(budget.reserve_product(usize::MAX, 2))?;
    assert_eq!(budget.entries.remaining(), None);
    let amount = Observation::sum_counts(&[usize::MAX, 1]);
    let mut finite = Budget::with_limits(&control, None, Some(u64::MAX));
    let before = finite.entries;
    let result = finite.reserve_counts(&[usize::MAX, 1]);
    if amount.saturated {
        assert_eq!(failed(result)?.code, "reference_read_view_proof_storage");
        assert_eq!(finite.entries, before);
    } else {
        checked(result)?;
        assert_eq!(finite.entries.remaining(), Some(u64::MAX - amount.consumed));
    }
    Ok(())
}
