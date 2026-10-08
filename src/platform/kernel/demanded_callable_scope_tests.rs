#![allow(clippy::unwrap_used, clippy::panic)]
use super::*;
#[path = "demanded_callable_scope_boundary_tests.rs"]
mod boundaries;
#[path = "demanded_callable_scope_cost_tests.rs"]
mod costs;
#[path = "demanded_callable_scope_fixture.rs"]
mod fixture;
use fixture::{Source, analysis, ids, owner};

#[test]
fn owner_local_projection_keeps_exact_packages_declarations_and_order() {
    let ids = ids(8);
    let mut source = Source::default();
    let owners = [owner(1, 1), owner(2, 1), owner(1, 2)];
    for (offset, owner) in owners.iter().enumerate() {
        let mut ordered = ids.clone();
        ordered.rotate_left(offset);
        source.add(*owner, &ordered);
    }
    let mut work = 0;
    let mut analysis = analysis(&source, &mut work);
    let mut scopes = Scopes::default();
    for _ in 0..3 {
        for owner in owners {
            let OwnerRecord::Declaration(record) = source
                .owners
                .get(&(owner.package, OwnerKey::Declaration(owner.declaration)))
                .unwrap()
            else {
                panic!("declaration fixture");
            };
            for id in ids.iter().rev() {
                let expected = analysis.parameter_ordinal(&record.payload, *id).unwrap();
                assert_eq!(scopes.ordinal(&mut analysis, owner, *id).unwrap(), expected);
            }
        }
    }
    assert_eq!(source.reads.get(), owners.len());
    assert_eq!(scopes.entries.len(), owners.len());
}

#[test]
fn missing_parameter_keeps_the_original_diagnostic_without_partial_publication() {
    let ids = ids(4);
    let missing = fixture::ids(5)[4];
    let owner = owner(1, 1);
    let mut source = Source::default();
    source.add(owner, &ids);
    for warm in [false, true] {
        let mut work = 0;
        let mut analysis = analysis(&source, &mut work);
        let record = analysis.declaration(owner).unwrap();
        let expected = analysis
            .parameter_ordinal(&record.payload, missing)
            .unwrap_err();
        let mut scopes = Scopes::default();
        if warm {
            scopes.ordinal(&mut analysis, owner, ids[0]).unwrap();
        }
        let error = scopes.ordinal(&mut analysis, owner, missing).unwrap_err();
        assert_eq!((error.class, error.code), (expected.class, expected.code));
        assert_eq!(scopes.entries.len(), usize::from(warm));
    }
}

#[test]
fn new_operation_reloads_the_same_owner_after_a_source_change() {
    let mut ids = ids(4);
    let selected = ids[0];
    let owner = owner(1, 1);
    let mut old_source = Source::default();
    old_source.add(owner, &ids);
    let mut work = 0;
    let mut old = analysis(&old_source, &mut work);
    let mut old_scopes = Scopes::default();
    assert_eq!(old_scopes.ordinal(&mut old, owner, selected).unwrap(), 0);
    ids.rotate_left(1);
    let mut new_source = Source::default();
    new_source.add(owner, &ids);
    let mut work = 0;
    let mut new = analysis(&new_source, &mut work);
    let mut new_scopes = Scopes::default();
    assert_eq!(new_scopes.ordinal(&mut new, owner, selected).unwrap(), 3);
    assert_eq!(old_source.reads.get(), 1);
    assert_eq!(new_source.reads.get(), 1);
}

#[test]
fn local_projection_does_not_invent_a_new_duplicate_identifier_policy() {
    // This raw fixture is not an admitted source program. Preserve the existing
    // local lookup behavior; complete source validation still owns uniqueness.
    let ids = ids(3);
    let ordered = [ids[0], ids[1], ids[0], ids[2]];
    let owner = owner(1, 1);
    let mut source = Source::default();
    source.add(owner, &ordered);
    let mut work = 0;
    let mut analysis = analysis(&source, &mut work);
    let record = analysis.declaration(owner).unwrap();
    let mut scopes = Scopes::default();
    for id in [ids[1], ids[0], ids[2]] {
        let expected = analysis.parameter_ordinal(&record.payload, id);
        let actual = scopes.ordinal(&mut analysis, owner, id);
        assert_eq!(
            actual.map_err(|e| (e.class, e.code)),
            expected.map_err(|e| (e.class, e.code))
        );
    }
}

#[test]
fn missing_declaration_does_not_publish_a_projection() {
    let source = Source::default();
    let owner = owner(1, 1);
    let mut work = 0;
    let mut analysis = analysis(&source, &mut work);
    let expected = analysis.declaration(owner).unwrap_err();
    let mut scopes = Scopes::default();
    let error = scopes.ordinal(&mut analysis, owner, ids(1)[0]).unwrap_err();
    assert_eq!((error.class, error.code), (expected.class, expected.code));
    assert!(scopes.entries.is_empty());
}
