//! Optional associated admission policies and passive observations are distinct.
//! Synthetic counter controls do not claim execution of a larger source graph.

use super::super::super::reference_schema::NormalizedReferenceSchema;
use super::*;

type TestResult<T = ()> = Result<T, String>;

fn checked<T, E: std::fmt::Debug>(value: Result<T, E>) -> TestResult<T> {
    value.map_err(|error| format!("{error:?}"))
}

fn rejected<T>(value: Result<T, ExecutionError>) -> TestResult<ExecutionError> {
    match value {
        Err(error) => Ok(error),
        Ok(_) => Err("associated admission unexpectedly succeeded".to_owned()),
    }
}

#[test]
fn associated_admission_default_budget_crosses_former_work_and_metadata_ceilings() -> TestResult {
    let limits = Limits::default();
    assert_eq!((limits.steps, limits.bytes), (None, None));
    let control = ExecutionControl::uncancelled();
    let mut budget = Budget {
        steps: Meter::new(limits.steps),
        bytes: Meter::new(limits.bytes),
        control: &control,
    };
    let old_work =
        Observation::from_usize(crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK);
    checked(budget.steps.charge(old_work))?;
    checked(budget.tick())?;
    assert_eq!(budget.steps.observation().consumed, old_work.consumed + 1);
    // reserve models metadata; it does not allocate a 256 MiB fixture.
    checked(budget.reserve::<u8>(256 * 1024 * 1024))?;
    checked(budget.reserve::<u8>(1))?;
    assert_eq!(
        budget.bytes.observation(),
        Observation::exact(256 * 1024 * 1024 + 1)
    );

    let mut saturated = Budget {
        steps: Meter::new(None),
        bytes: Meter::new(None),
        control: &control,
    };
    checked(saturated.steps.charge(Observation::exact(u64::MAX)))?;
    checked(saturated.bytes.charge(Observation::exact(u64::MAX)))?;
    checked(saturated.tick())?;
    checked(saturated.reserve::<u8>(1))?;
    assert!(saturated.steps.observation().saturated);
    assert!(saturated.bytes.observation().saturated);
    assert_eq!(
        (saturated.steps.remaining(), saturated.bytes.remaining()),
        (None, None)
    );

    let cancelled = ExecutionControl::cancel_after_checks(0);
    let mut cancelled_budget = Budget {
        steps: saturated.steps,
        bytes: saturated.bytes,
        control: &cancelled,
    };
    let before = (saturated.steps, saturated.bytes);
    assert_eq!(
        rejected(cancelled_budget.tick())?.class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!((cancelled_budget.steps, cancelled_budget.bytes), before);
    Ok(())
}

#[test]
fn associated_admission_complete_inventory_keeps_none_zero_and_finite_max_distinct() -> TestResult {
    let fixture = test_support::make_fixture();
    let control = ExecutionControl::uncancelled();
    let ordinary = checked(test_support::prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &control,
    ))?;
    assert!(!ordinary.steps_saturated && !ordinary.bytes_saturated);
    let maximum = checked(test_support::prove(
        &fixture,
        &[0, 1, 2],
        Limits {
            steps: Some(u64::MAX),
            bytes: Some(u64::MAX),
        },
        &control,
    ))?;
    assert_eq!(
        (maximum.steps, maximum.bytes),
        (ordinary.steps, ordinary.bytes)
    );
    assert!(!maximum.steps_saturated && !maximum.bytes_saturated);
    assert_eq!(maximum.types, ordinary.types);
    assert!(maximum.applications.keys().eq(ordinary.applications.keys()));
    assert!(maximum.symbolic.keys().eq(ordinary.symbolic.keys()));
    for limits in [
        Limits {
            steps: Some(0),
            bytes: None,
        },
        Limits {
            steps: None,
            bytes: Some(0),
        },
    ] {
        let error = rejected(test_support::prove(&fixture, &[0, 1, 2], limits, &control))?;
        assert_eq!(error.class, ExecutionFailureClass::Resource);
    }
    Ok(())
}

#[test]
fn associated_admission_dependency_metadata_uses_remaining_caller_allowance() -> TestResult {
    let fixture = test_support::make_fixture();
    let control = ExecutionControl::uncancelled();
    let snapshots = [&fixture.source];
    let prefix = checked(Derivation::new(&snapshots, Limits::default(), &control))?;
    let prefix_bytes = prefix.budget.bytes.observation();
    assert!(!prefix_bytes.saturated && prefix_bytes.consumed > 0);
    drop(prefix);

    // Initialization consumes this exact allowance. Dependency correspondence
    // must refuse its first modeled entry before insertion, using the same meter.
    let error = rejected(test_support::prove(
        &fixture,
        &[0, 1, 2],
        Limits {
            steps: None,
            bytes: Some(prefix_bytes.consumed),
        },
        &control,
    ))?;
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "normalized_reference_dependency_bound");
    Ok(())
}

#[test]
fn associated_admission_schema_keeps_phase_observation_after_saturated_source() -> TestResult {
    let fixture = test_support::make_fixture();
    let control = ExecutionControl::uncancelled();
    let proof = checked(test_support::prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &control,
    ))?;
    let ordinary = checked(super::super::associated_schema_fixture(
        &[&fixture.source],
        proof,
        &control,
    ))?;
    let proof = checked(test_support::prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &control,
    ))?;
    let mut saturated = NormalizedReferenceSchema {
        source_admission_steps: u64::MAX,
        source_admission_saturated: true,
        ..NormalizedReferenceSchema::default()
    };
    checked(super::super::associated_schema::core_with_limits(
        &mut saturated,
        &[&fixture.source],
        proof,
        Limits::default(),
        &control,
    ))?;
    assert_eq!(
        saturated.type_derivation_steps,
        ordinary.type_derivation_steps
    );
    assert_eq!(saturated.type_metadata_bytes, ordinary.type_metadata_bytes);
    assert!(!saturated.type_derivation_saturated && !saturated.type_metadata_saturated);
    assert!(saturated.source_admission_saturated);
    assert_eq!(saturated.types, ordinary.types);
    assert_eq!(saturated.functions, ordinary.functions);

    let proof = checked(test_support::prove(
        &fixture,
        &[0, 1, 2],
        Limits::default(),
        &control,
    ))?;
    let mut finite = NormalizedReferenceSchema {
        source_admission_steps: u64::MAX,
        source_admission_saturated: true,
        ..NormalizedReferenceSchema::default()
    };
    let error = rejected(super::super::associated_schema::core_with_limits(
        &mut finite,
        &[&fixture.source],
        proof,
        Limits {
            steps: Some(u64::MAX),
            bytes: None,
        },
        &control,
    ))?;
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert!(finite.associated_proof.is_none());
    assert!(finite.types.is_empty());
    Ok(())
}
