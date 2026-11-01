//! These controls reuse complete raw supplier/interface fixtures. Seeded
//! counters distinguish passive observations from exact finite permission.

use super::*;

#[test]
fn reference_capacity_dependency_correspondence_crosses_former_accounting_defaults()
-> Result<(), ExecutionError> {
    let (supplier, consumer) = fixture();
    let control = ExecutionControl::uncancelled();
    let mut work = Meter::new(None);
    assert!(work.charge(Observation::exact(10_000_000)).is_ok());
    let mut metadata = Meter::new(None);
    assert!(
        metadata
            .charge(Observation::exact(64 * 1024 * 1024))
            .is_ok()
    );
    let mut budget = Budget {
        control: &control,
        work: &mut work,
        bytes: &mut metadata,
    };
    validate_with_budget(&[&supplier, &consumer], &mut budget)?;
    assert!(budget.bytes.observation().consumed > 64 * 1024 * 1024);
    assert!(!budget.bytes.observation().saturated);
    assert!(work.observation().consumed > 10_000_000);
    assert!(!work.observation().saturated);

    // Some(MAX) is finite even when an ordinary observation can grow beyond it.
    let mut finite = Meter::new(Some(u64::MAX));
    assert!(finite.charge(Observation::exact(u64::MAX)).is_ok());
    let before = finite;
    let refusal = validate(&[&supplier, &consumer], &control, &mut finite);
    assert!(
        refusal
            .as_ref()
            .is_err_and(|error| error.class == ExecutionFailureClass::Resource
                && error.code == "normalized_reference_dependency_bound"),
        "{refusal:?}"
    );
    assert_eq!(finite, before);
    Ok(())
}

#[test]
fn reference_capacity_dependency_saturation_does_not_hide_missing_canonical_types()
-> Result<(), ExecutionError> {
    let (supplier, consumer) = fixture();
    let control = ExecutionControl::uncancelled();
    let mut work = Meter::new(None);
    let mut bytes = Meter::new(None);
    assert!(work.charge(Observation::exact(u64::MAX)).is_ok());
    assert!(bytes.charge(Observation::exact(u64::MAX)).is_ok());
    let mut budget = Budget {
        control: &control,
        work: &mut work,
        bytes: &mut bytes,
    };
    validate_with_budget(&[&supplier, &consumer], &mut budget)?;
    assert!(budget.bytes.observation().saturated);
    assert!(budget.work.observation().saturated);

    let mut missing = consumer.clone();
    let object = TypeObject::new(TypeForm::Bool)
        .map_err(|_| inventory_error("fixture Bool type construction failed"))?;
    missing
        .dependency_types
        .insert(TypeObjectDigest::from_bytes([213; 32]), object);
    let rejection = validate_with_budget(&[&supplier, &missing], &mut budget);
    assert!(
        rejection
            .as_ref()
            .is_err_and(|error| error.code == "normalized_reference_inventory"
                && error.message.contains("canonical source body")),
        "{rejection:?}"
    );
    assert!(budget.bytes.observation().saturated && budget.work.observation().saturated);
    Ok(())
}

#[test]
fn reference_capacity_associated_dependency_metadata_uses_the_selected_allowance()
-> Result<(), ExecutionError> {
    let (supplier, consumer) = fixture();
    let inputs = [&supplier, &consumer];
    let control = ExecutionControl::uncancelled();
    let mut work = Meter::new(None);
    let mut bytes = Meter::new(None);
    super::super::super::bind_associated_dependencies(&inputs, &control, &mut work, &mut bytes)?;
    assert!(!work.observation().saturated && !bytes.observation().saturated);
    let exact_work = work.observation().consumed;
    let exact_bytes = bytes.observation().consumed;
    assert!(exact_work > 0 && exact_bytes > 0);
    let mut finite_work = Meter::new(Some(exact_work));
    let mut finite_bytes = Meter::new(Some(exact_bytes));
    super::super::super::bind_associated_dependencies(
        &inputs,
        &control,
        &mut finite_work,
        &mut finite_bytes,
    )?;
    assert_eq!(finite_work.remaining(), Some(0));
    assert_eq!(finite_bytes.remaining(), Some(0));

    for limit in [0, exact_bytes - 1] {
        let result = super::super::super::bind_associated_dependencies(
            &inputs,
            &control,
            &mut Meter::new(None),
            &mut Meter::new(Some(limit)),
        );
        assert!(
            result
                .as_ref()
                .is_err_and(|error| error.class == ExecutionFailureClass::Resource
                    && error.code == "normalized_reference_dependency_bound"),
            "{result:?}"
        );
    }
    let mut shared = Meter::new(Some(exact_bytes));
    assert!(shared.charge(Observation::exact(1)).is_ok());
    let result = super::super::super::bind_associated_dependencies(
        &inputs,
        &control,
        &mut Meter::new(None),
        &mut shared,
    );
    assert!(
        result
            .as_ref()
            .is_err_and(|error| error.class == ExecutionFailureClass::Resource),
        "a fresh nested budget must not erase the caller's prefix: {result:?}"
    );
    Ok(())
}
