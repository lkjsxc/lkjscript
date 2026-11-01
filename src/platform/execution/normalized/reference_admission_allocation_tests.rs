//! A passive preparation total cannot serve as an exact finite allocation debit.
use super::*;
use crate::platform::admission::Observation;

#[test]
fn reference_preparation_allocation_keeps_none_and_all_finite_policies_distinct() {
    for limit in [None, Some(0), Some(6), Some(7), Some(u64::MAX)] {
        let policy = NormalizedRunPolicy {
            maximum_allocated_bytes: limit,
            ..NormalizedRunPolicy::foreground()
        };
        let mut observed = NormalizedReferenceObservation::default();
        let result =
            charge_reference_preparation_allocation(policy, &mut observed, Observation::exact(7));
        if limit.is_some_and(|limit| limit < 7) {
            assert!(result.is_err());
            assert_eq!(observed.allocated_bytes, 0);
            assert_eq!(observed.allocation_charges, 0);
        } else {
            assert!(result.is_ok());
            assert_eq!(observed.allocated_bytes, 7);
            assert!(!observed.allocated_bytes_saturated);
            assert_eq!(observed.allocation_charges, 1);
        }
    }
    let policy = NormalizedRunPolicy {
        maximum_allocated_bytes: Some(u64::MAX),
        ..NormalizedRunPolicy::foreground()
    };
    let mut observed = NormalizedReferenceObservation::default();
    assert!(
        charge_reference_preparation_allocation(
            policy,
            &mut observed,
            Observation::exact(u64::MAX)
        )
        .is_ok()
    );
    assert!(!observed.allocated_bytes_saturated);
    assert!(
        charge_reference_preparation_allocation(policy, &mut observed, Observation::exact(1))
            .is_err()
    );
    assert_eq!(observed.allocated_bytes, u64::MAX);
    assert_eq!(observed.allocation_charges, 1);
}

#[test]
fn reference_preparation_inexact_storage_never_grants_finite_permission() {
    let amount = Observation {
        consumed: 3,
        saturated: true,
    };
    for limit in [None, Some(0), Some(3), Some(u64::MAX)] {
        let policy = NormalizedRunPolicy {
            maximum_allocated_bytes: limit,
            ..NormalizedRunPolicy::foreground()
        };
        let mut observed = NormalizedReferenceObservation {
            allocated_bytes: 2,
            ..NormalizedReferenceObservation::default()
        };
        let result = charge_reference_preparation_allocation(policy, &mut observed, amount);
        if limit.is_some() {
            assert!(result.is_err());
            assert_eq!(observed.allocated_bytes, 2);
            assert!(!observed.allocated_bytes_saturated);
            assert_eq!(observed.allocation_charges, 0);
        } else {
            assert!(result.is_ok());
            assert_eq!(observed.allocated_bytes, 5);
            assert!(observed.allocated_bytes_saturated);
            assert_eq!(observed.allocation_charges, 1);
        }
    }
}
