#![allow(clippy::unwrap_used, clippy::panic)]
//! Exact checked projection derives its proof from a live admitted sequence.
use super::super::super::owned_sequence::OwnedSequence;
use super::metadata_tests::fixture::Fixture;
use super::*;
use crate::platform::kernel::{TypeObject, encode_type_object};

fn fixture(depth: usize) -> (Fixture, TypeObjectDigest) {
    let mut f = Fixture::new(depth);
    let mut current = f.payload;
    loop {
        f.program.comparable_types.insert(current);
        current = match &f.program.types[&current].form {
            TypeForm::Option { item } | TypeForm::List { item } => *item,
            TypeForm::I64 => break,
            _ => panic!("fixture ordinary data"),
        };
    }
    let object = TypeObject::new(TypeForm::OwnedSequence { item: f.payload }).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    f.program.types.insert(ty, object);
    (f, ty)
}

fn storage(f: &Fixture, ty: TypeObjectDigest, raw: NormalizedValue) -> OwnedSequence {
    let control = ExecutionControl::uncancelled();
    OwnedSequence::create(f.domain, ty, &control, &mut |_| Ok(()))
        .unwrap()
        .push(f.domain, raw, &control, &mut |_| Ok(()))
        .unwrap()
}

fn owner(f: &Fixture, ty: TypeObjectDigest, count: usize) -> Value {
    let child = f.admit(f.raw(count)).unwrap();
    let token = storage(f, ty, child.into_raw());
    token.establish_admission(f.program.value_origin).unwrap();
    Value::memory(&f.program, NormalizedValue::OwnedSequence(token)).unwrap()
}

fn token(value: &Value) -> &OwnedSequence {
    let NormalizedValue::OwnedSequence(token) = value.raw() else {
        panic!("sequence")
    };
    token
}

fn project(f: &Fixture, value: &Value) -> Result<Value, ExecutionError> {
    value.sequence_item(
        &f.program,
        f.domain,
        0,
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
}

#[test]
fn checked_sequence_get_retains_large_data_without_descendant_walks_or_owner_loans() {
    let (f, ty) = fixture(0);
    let observations = super::super::super::owned_product::StorageObservation::start();
    for count in [0, 8, 1024, 65536] {
        let owner = owner(&f, ty, count);
        let read = owner.duplicate(ParameterUse::Borrow).unwrap();
        for _ in 0..32 {
            let before = super::super::super::list::Work::current();
            let selected = read
                .sequence_item(
                    &f.program,
                    f.domain,
                    0,
                    &ExecutionControl::uncancelled(),
                    &mut |_| panic!("immutable list projection allocates no backing"),
                )
                .unwrap();
            assert_eq!(before.since().node_visits, 0);
            assert_eq!(selected.class, Class::Free);
            assert!(selected.borrow.is_none());
            assert_eq!(selected.raw(), &f.raw(count));
        }
        let retained = project(&f, &read).unwrap();
        drop(read);
        token(&owner).validate(f.domain, true).unwrap();
        drop(owner);
        assert_eq!(observations.live(), (0, 0));
        assert_eq!(retained.raw(), &f.raw(count));
    }
}

#[test]
fn checked_sequence_get_rejects_raw_prefix_and_restored_shape_without_admission() {
    let (f, ty) = fixture(0);
    let malformed =
        NormalizedValue::list(vec![NormalizedValue::I64(1), NormalizedValue::Bool(false)]).unwrap();
    assert!(f.admit(malformed.clone()).is_err());
    for raw in [f.raw(2), malformed] {
        let sequence = storage(&f, ty, raw);
        let forged_read = Value {
            raw: NormalizedValue::OwnedSequence(sequence.borrow().unwrap()),
            origin: f.program.value_origin,
            class: Class::Memory,
            borrow: None,
        };
        assert!(project(&f, &forged_read).is_err());
        drop(forged_read);
        assert!(Value::memory(&f.program, NormalizedValue::OwnedSequence(sequence)).is_err());
    }
    let owner = owner(&f, ty, 2);
    let control = ExecutionControl::uncancelled();
    let displaced = token(&owner)
        .storage
        .replace(f.domain, 0, f.raw(3), &control)
        .unwrap();
    token(&owner)
        .storage
        .replace(f.domain, 0, displaced, &control)
        .unwrap();
    assert!(
        token(&owner)
            .validate_admission(f.program.value_origin)
            .is_err()
    );
    assert!(project(&f, &owner.duplicate(ParameterUse::Borrow).unwrap()).is_err());
}

#[test]
fn checked_sequence_get_checks_exact_program_type_eligibility_and_read_placement() {
    let (mut f, ty) = fixture(0);
    let (foreign, foreign_ty) = fixture(0);
    assert_eq!(ty, foreign_ty);
    let owner = owner(&f, ty, 2);
    assert!(project(&f, &owner).is_err());
    let mut read = owner.duplicate(ParameterUse::Borrow).unwrap();
    assert!(project(&foreign, &read).is_err());
    read.origin = foreign.program.value_origin;
    assert!(
        read.sequence_item(
            &foreign.program,
            f.domain,
            0,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .is_err()
    );
    read.origin = f.program.value_origin;
    let exact = f.program.types.remove(&ty).unwrap();
    assert!(project(&f, &read).is_err());
    f.program.types.insert(ty, exact);
    f.program.comparable_types.remove(&f.payload);
    assert!(project(&f, &read).is_err());
    f.program.comparable_types.insert(f.payload);
    assert!(
        read.sequence_item(
            &f.program,
            ValueOrigin::fresh().unwrap(),
            0,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .is_err()
    );
    assert!(project(&f, &read).is_ok());
    drop(owner);
    assert!(project(&f, &read).is_err());
}

#[test]
fn checked_sequence_get_snapshot_survives_owning_and_scoped_domain_adoption() {
    let (f, ty) = fixture(0);
    let mut owner = owner(&f, ty, 8);
    let mut read = owner.duplicate(ParameterUse::Borrow).unwrap();
    let child_domain = ValueOrigin::fresh().unwrap();
    let NormalizedValue::OwnedSequence(read_token) = &mut read.raw else {
        panic!("sequence read")
    };
    read_token
        .adopt_scoped_read(f.domain, child_domain)
        .unwrap();
    let retained = read
        .sequence_item(
            &f.program,
            child_domain,
            0,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap();
    assert!(project(&f, &read).is_err());
    assert!(token(&owner).validate(f.domain, true).is_err());
    drop(read);
    let NormalizedValue::OwnedSequence(owner_token) = &mut owner.raw else {
        panic!("sequence owner")
    };
    owner_token
        .adopt_transfer(f.domain, child_domain, |_| Ok(()))
        .unwrap();
    let read = owner.duplicate(ParameterUse::Borrow).unwrap();
    assert_eq!(
        read.sequence_item(
            &f.program,
            child_domain,
            0,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap()
        .raw(),
        retained.raw()
    );
    drop(read);
    drop(owner);
    assert_eq!(retained.raw(), &f.raw(8));
}

#[test]
fn checked_sequence_get_reserves_clone_spine_and_cancellation_leaves_healthy_owner() {
    let (f, ty) = fixture(8);
    let owner = owner(&f, ty, 16);
    let read = owner.duplicate(ParameterUse::Borrow).unwrap();
    let mut charges = Vec::new();
    let error = read
        .sequence_item(
            &f.program,
            f.domain,
            0,
            &ExecutionControl::uncancelled(),
            &mut |bytes| {
                charges.push(bytes);
                Err(ExecutionError::resource(
                    "sequence_read_quota",
                    "refused before clone",
                ))
            },
        )
        .unwrap_err();
    assert_eq!(error.code, "sequence_read_quota");
    assert_eq!(
        charges,
        vec![8 * std::mem::size_of::<NormalizedValue>() as u64]
    );
    let mut failures = 0;
    let mut successes = 0;
    for checks in 0..40 {
        match read.sequence_item(
            &f.program,
            f.domain,
            0,
            &ExecutionControl::cancel_after_checks(checks),
            &mut |_| Ok(()),
        ) {
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
        assert_eq!(project(&f, &read).unwrap().raw(), &f.raw(16));
    }
    assert!(failures >= 10 && successes > 0);
    drop(read);
    token(&owner).validate(f.domain, true).unwrap();
}

#[test]
fn checked_sequence_get_failed_adoption_invalidates_existing_checked_slot() {
    let (f, ty) = fixture(0);
    let mut owner = owner(&f, ty, 2);
    let NormalizedValue::OwnedSequence(sequence) = &mut owner.raw else {
        panic!("sequence")
    };
    assert!(
        sequence
            .adopt_transfer(f.domain, ValueOrigin::fresh().unwrap(), |_| {
                Err(ExecutionError::resource("sequence_adoption", "interrupted"))
            })
            .is_err()
    );
    assert!(project(&f, &owner.duplicate(ParameterUse::Borrow).unwrap()).is_err());
}
