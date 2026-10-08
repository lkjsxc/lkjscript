#![allow(clippy::unwrap_used, clippy::panic)]
use super::metadata_tests::fixture::{Fixture, token};
use super::*;

#[test]
fn borrowed_metadata_sum_spine_reserves_before_cloning_and_failure_keeps_custody() {
    let f = Fixture::new(8);
    let owner = f.owner(64);
    let read = f.read(&owner);
    let mut requests = Vec::new();
    let expected = 8 * std::mem::size_of::<NormalizedValue>() as u64;
    let error = read
        .owned_metadata(
            &f.program,
            f.domain,
            &Name::new("payload").unwrap(),
            &ExecutionControl::uncancelled(),
            &mut |bytes| {
                requests.push(bytes);
                Err(ExecutionError::resource(
                    "metadata_reservation_test",
                    "refused before clone",
                ))
            },
        )
        .unwrap_err();
    assert_eq!(error.code, "metadata_reservation_test");
    assert_eq!(requests, vec![expected]);
    token(&owner)
        .validate_admission(f.program.value_origin)
        .unwrap();
    assert!(token(&owner).validate(f.domain, true).is_err());
    requests.clear();
    let selected = read
        .owned_metadata(
            &f.program,
            f.domain,
            &Name::new("payload").unwrap(),
            &ExecutionControl::uncancelled(),
            &mut |bytes| {
                requests.push(bytes);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(requests, vec![expected]);
    assert_eq!(selected.raw(), &f.raw(64));
    drop(read);
    token(&owner).validate(f.domain, true).unwrap();
}

#[test]
fn borrowed_metadata_cancellation_at_each_check_leaves_no_new_loan_or_partial_value() {
    let f = Fixture::new(8);
    let owner = f.owner(16);
    let read = f.read(&owner);
    let owners = super::super::super::owned_product::StorageObservation::start();
    let mut failures = 0;
    let mut successes = 0;
    for checks in 0..32 {
        let result = read.owned_metadata(
            &f.program,
            f.domain,
            &Name::new("payload").unwrap(),
            &ExecutionControl::cancel_after_checks(checks),
            &mut |_| Ok(()),
        );
        match result {
            Ok(selected) => {
                successes += 1;
                assert_eq!(selected.raw(), &f.raw(16));
            }
            Err(error) => {
                failures += 1;
                assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Cancelled
                );
            }
        }
        token(&owner)
            .validate_admission(f.program.value_origin)
            .unwrap();
        assert_eq!(f.project(&read).unwrap().raw(), &f.raw(16));
    }
    assert!(failures >= 10 && successes > 0);
    assert_eq!(owners.created(), 0);
    drop(read);
    token(&owner).validate(f.domain, true).unwrap();
}

#[test]
fn borrowed_metadata_missing_type_or_lost_ordinary_eligibility_rejects() {
    let mut f = Fixture::new(0);
    let owner = f.owner(2);
    let read = f.read(&owner);
    let exact = f.program.types.remove(&f.product).unwrap();
    assert!(f.project(&read).is_err());
    f.program.types.insert(f.product, exact);
    f.program.ordinary_types.remove(&f.payload);
    assert!(f.project(&read).is_err());
    f.program.ordinary_types.insert(f.payload);
    assert!(f.project(&read).is_ok());
}
