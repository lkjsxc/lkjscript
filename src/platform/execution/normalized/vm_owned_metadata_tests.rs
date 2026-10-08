#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
#[path = "vm_owned_metadata_fixture.rs"]
pub(super) mod fixture;
use fixture::{Fixture, token};

#[test]
fn borrowed_metadata_shares_large_immutable_payload_without_walking_it() {
    let f = Fixture::new(0);
    for n in [0, 8, 1024, 65536] {
        let owner = f.owner(n);
        let read = f.read(&owner);
        for _ in 0..32 {
            let before = super::super::super::list::Work::current();
            let selected = read
                .owned_metadata(
                    &f.program,
                    f.domain,
                    &Name::new("payload").unwrap(),
                    &ExecutionControl::uncancelled(),
                    &mut |_| panic!("shared list projection allocates no payload"),
                )
                .unwrap();
            assert_eq!(before.since().node_visits, 0);
            assert_eq!(selected.class, Class::Free);
            assert!(selected.borrow.is_none());
            let NormalizedValue::List(list) = selected.raw() else {
                panic!("list")
            };
            assert_eq!(list.len(), n);
        }
        assert!(token(&owner).validate(f.domain, true).is_err());
        drop(read);
        token(&owner).validate(f.domain, true).unwrap();
    }
}

#[test]
fn borrowed_metadata_survives_release_without_retaining_owner_or_loan() {
    let f = Fixture::new(0);
    let owners = super::super::super::owned_product::StorageObservation::start();
    let cells = super::super::super::owned_i64_cell::StorageObservation::start();
    let owner = f.owner(1024);
    let read = f.read(&owner);
    let selected = f.project(&read).unwrap();
    drop(read);
    drop(owner);
    assert_eq!(owners.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(selected.raw(), &f.raw(1024));
}

#[test]
fn borrowed_metadata_raw_storage_and_late_invalid_input_confer_no_certificate() {
    let f = Fixture::new(0);
    let malformed =
        NormalizedValue::list(vec![NormalizedValue::I64(1), NormalizedValue::Bool(false)]).unwrap();
    assert!(f.admit(malformed.clone()).is_err());
    for raw in [f.raw(2), malformed] {
        let token = f.storage(raw);
        assert!(token.validate_admission(f.program.value_origin).is_err());
        // Fault-injected stale slot cannot promote unadmitted physical storage.
        let read = Value {
            raw: NormalizedValue::OwnedProduct(token.borrow().unwrap()),
            origin: f.program.value_origin,
            class: Class::Memory,
            borrow: None,
        };
        assert!(f.project(&read).is_err());
        assert!(Value::memory(&f.program, NormalizedValue::OwnedProduct(token)).is_err());
    }
    assert_eq!(f.project(&f.read(&f.owner(2))).unwrap().raw(), &f.raw(2));
}

#[test]
fn borrowed_metadata_rejects_foreign_program_even_with_same_exact_type_shapes() {
    let f = Fixture::new(0);
    let foreign = Fixture::new(0);
    assert_eq!(f.product, foreign.product);
    let owner = f.owner(2);
    let mut read = f.read(&owner);
    assert!(foreign.project(&read).is_err());
    // Corrupt only the slot origin: the live allocation proof still disagrees.
    read.origin = foreign.program.value_origin;
    assert!(
        read.owned_metadata(
            &foreign.program,
            f.domain,
            &Name::new("payload").unwrap(),
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(())
        )
        .is_err()
    );
    read.origin = f.program.value_origin;
    assert!(f.project(&read).is_ok());
}

#[test]
fn borrowed_metadata_requires_live_read_domain_exact_selector_and_ordinary_field() {
    let f = Fixture::new(0);
    let owner = f.owner(2);
    assert!(f.project(&owner).is_err());
    let mut read = f.read(&owner);
    for name in ["missing", "rest"] {
        assert!(
            read.owned_metadata(
                &f.program,
                f.domain,
                &Name::new(name).unwrap(),
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(())
            )
            .is_err()
        );
    }
    assert!(
        read.owned_metadata(
            &f.program,
            ValueOrigin::fresh().unwrap(),
            &Name::new("payload").unwrap(),
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(())
        )
        .is_err()
    );
    read.class = Class::Free;
    assert!(f.project(&read).is_err());
    read.class = Class::Memory;
    assert!(f.project(&read).is_ok());
    drop(owner); // Faulted cleanup must revoke even a surviving read placement.
    assert!(f.project(&read).is_err());
}

#[test]
fn borrowed_metadata_restored_raw_shape_does_not_restore_admission() {
    let f = Fixture::new(0);
    let owner = f.owner(2);
    let control = ExecutionControl::uncancelled();
    let child = token(&owner).pop(f.domain, &control).unwrap().unwrap();
    token(&owner)
        .push(f.domain, child, &control, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(token(&owner).len(f.domain).unwrap(), 2);
    assert!(
        token(&owner)
            .validate_admission(f.program.value_origin)
            .is_err()
    );
    assert!(f.project(&f.read(&owner)).is_err());
}

#[test]
fn borrowed_metadata_failed_adoption_cannot_reuse_preexisting_slot_classification() {
    let f = Fixture::new(0);
    let mut owner = f.owner(2);
    let NormalizedValue::OwnedProduct(token) = &mut owner.raw else {
        panic!("product")
    };
    assert!(
        token
            .adopt_transfer(f.domain, ValueOrigin::fresh().unwrap(), |_| {
                Err(ExecutionError::resource("test_adoption", "interrupted"))
            })
            .is_err()
    );
    assert!(f.project(&f.read(&owner)).is_err());
}

#[test]
fn borrowed_metadata_scoped_domain_adoption_keeps_original_program_and_custody() {
    let f = Fixture::new(0);
    let owner = f.owner(8);
    let mut read = f.read(&owner);
    let child_domain = ValueOrigin::fresh().unwrap();
    let NormalizedValue::OwnedProduct(child_token) = &mut read.raw else {
        panic!("product")
    };
    child_token
        .adopt_scoped_read(f.domain, child_domain)
        .unwrap();
    let selected = read
        .owned_metadata(
            &f.program,
            child_domain,
            &Name::new("payload").unwrap(),
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap();
    assert_eq!(selected.raw(), &f.raw(8));
    assert!(f.project(&read).is_err());
    assert!(token(&owner).validate(f.domain, true).is_err());
    drop(read);
    token(&owner).validate(f.domain, true).unwrap();
}
