#![allow(clippy::unwrap_used, clippy::panic)]
//! Synthetic custody failures are independent of the public typed lowering.
use super::super::super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells};
use super::super::super::super::owned_product::{OwnedProduct, StorageObservation as Products};
use super::super::metadata_tests::fixture::{Fixture, token};
use super::super::{BorrowMetadata, BorrowResultTarget, BorrowSource};
use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::kernel::{
    DeclarationReference, FieldReference, StructuralTypeField, TypeObject, encode_type_object,
};
use crate::platform::semantic_id::{DeclarationId, FieldId};
use std::collections::BTreeMap;
use std::sync::Arc;

fn owner(f: &Fixture, count: usize) -> Value {
    let raw = f.admit(f.raw(count)).unwrap().into_raw();
    let token = f.storage(raw);
    token
        .inspect_transfer(f.domain, |fields| {
            let NormalizedValue::OwnedI64Cell(cell) = &fields[1] else {
                panic!("fixture cell")
            };
            cell.establish_admission(f.program.value_origin)
        })
        .unwrap();
    token.establish_admission(f.program.value_origin).unwrap();
    Value::memory(&f.program, NormalizedValue::OwnedProduct(token)).unwrap()
}

fn owned_child(
    f: &Fixture,
    raw: NormalizedValue,
    ty: TypeObjectDigest,
) -> Result<Value, ExecutionError> {
    let expected = super::super::super::direct_memory_type(
        &f.program,
        ty,
        &BTreeMap::new(),
        &ExecutionControl::uncancelled(),
    )?;
    if expected.is_none() || raw.memory_form() != expected {
        return Err(admission_error(
            "test extraction child has a foreign memory form",
        ));
    }
    raw.memory_validate(f.domain, true)?;
    Value::memory(&f.program, raw)
}

fn extract(f: &Fixture, value: Value) -> Result<ProductExtraction, ExecutionError> {
    value.unpack_owned(
        &f.program,
        f.domain,
        f.product,
        &ExecutionControl::uncancelled(),
    )
}

#[test]
fn consuming_extraction_moves_large_payload_and_checks_child_custody_without_descendant_walks() {
    for depth in [0, 8, 24] {
        let f = Fixture::new(depth);
        let products = Products::start();
        let cells = Cells::start();
        for count in [0, 8, 1024, 65536] {
            let value = owner(&f, count);
            let child_identity = token(&value)
                .inspect_transfer(f.domain, |fields| {
                    let NormalizedValue::OwnedI64Cell(cell) = &fields[1] else {
                        panic!("cell")
                    };
                    Ok(cell.allocation_identity())
                })
                .unwrap();
            let before = super::super::super::super::list::Work::current();
            let mut extraction = extract(&f, value).unwrap();
            let mut checks = 0;
            let payload = extraction
                .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
                    panic!("ordinary immutable field must keep its admitted proof")
                })
                .unwrap()
                .unwrap();
            let child = extraction
                .next(
                    &f.program,
                    &ExecutionControl::uncancelled(),
                    &mut |raw, ty| {
                        checks += 1;
                        owned_child(&f, raw, ty)
                    },
                )
                .unwrap()
                .unwrap();
            assert_eq!(checks, 1);
            assert_eq!(before.since().node_visits, 0);
            assert_eq!(payload.class, Class::Free);
            assert!(payload.borrow.is_none());
            assert_eq!(payload.raw(), &f.raw(count));
            assert_eq!(child.class, Class::Memory);
            let NormalizedValue::OwnedI64Cell(cell) = child.raw() else {
                panic!("checked child")
            };
            assert_eq!(cell.allocation_identity(), child_identity);
            assert!(
                extraction
                    .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
                        panic!("exhausted extraction")
                    })
                    .unwrap()
                    .is_none()
            );
            drop(extraction);
            drop(child);
            assert_eq!(products.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
            // The immutable result outlives both the original owner and sibling.
            assert_eq!(payload.raw(), &f.raw(count));
        }
        products.assert_transfers_preserve_allocations();
    }
}

#[test]
fn consuming_extraction_partial_drop_releases_unreturned_owners_and_retains_no_loan() {
    let f = Fixture::new(8);
    let products = Products::start();
    let cells = Cells::start();
    let mut extraction = extract(&f, owner(&f, 1024)).unwrap();
    let payload = extraction
        .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
            panic!("ordinary field")
        })
        .unwrap()
        .unwrap();
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (1, 0));
    drop(extraction);
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(payload.raw(), &f.raw(1024));
    assert!(payload.borrow.is_none());
}

#[test]
fn consuming_extraction_rejects_unadmitted_raw_storage_even_with_valid_shape_or_checked_slot() {
    let f = Fixture::new(0);
    let malformed =
        NormalizedValue::list(vec![NormalizedValue::I64(1), NormalizedValue::Bool(false)]).unwrap();
    assert!(f.admit(malformed.clone()).is_err());
    let products = Products::start();
    let cells = Cells::start();
    for raw in [f.raw(2), malformed] {
        let forged = Value {
            raw: NormalizedValue::OwnedProduct(f.storage(raw)),
            origin: f.program.value_origin,
            class: Class::Memory,
            borrow: None,
        };
        assert!(extract(&f, forged).is_err());
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
}

#[test]
fn consuming_extraction_rejects_foreign_program_and_corrupted_slot_origin() {
    let f = Fixture::new(0);
    let foreign = Fixture::new(0);
    assert_eq!(f.product, foreign.product);
    assert!(extract(&foreign, owner(&f, 2)).is_err());
    let mut forged = owner(&f, 2);
    forged.origin = foreign.program.value_origin;
    assert!(extract(&foreign, forged).is_err());
    let mut extraction = extract(&f, owner(&f, 2)).unwrap();
    assert!(
        extraction
            .next(
                &foreign.program,
                &ExecutionControl::uncancelled(),
                &mut |_, _| { panic!("foreign extraction must reject before exposing a child") }
            )
            .is_err()
    );
    assert!(
        extraction
            .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
                panic!("ordinary field")
            })
            .unwrap()
            .is_some()
    );
}

#[test]
fn consuming_extraction_rejects_raw_mutation_restoring_shape_and_revocation() {
    let f = Fixture::new(0);
    let products = Products::start();
    let cells = Cells::start();
    let value = owner(&f, 2);
    let control = ExecutionControl::uncancelled();
    let child = token(&value).pop(f.domain, &control).unwrap().unwrap();
    token(&value)
        .push(f.domain, child, &control, &mut |_| Ok(()))
        .unwrap();
    assert_eq!(token(&value).len(f.domain).unwrap(), 2);
    assert!(extract(&f, value).is_err());
    let mut value = owner(&f, 2);
    let NormalizedValue::OwnedProduct(storage) = &mut value.raw else {
        panic!("product")
    };
    let revoked = storage.revoke().unwrap();
    assert!(extract(&f, value).is_err());
    super::super::super::super::value::release_raw_values(revoked);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn consuming_extraction_rejects_interrupted_transfer_and_accepts_completed_custody_transfer() {
    let f = Fixture::new(0);
    let products = Products::start();
    let cells = Cells::start();
    let destination = ValueOrigin::fresh().unwrap();
    let mut value = owner(&f, 2);
    let NormalizedValue::OwnedProduct(storage) = &mut value.raw else {
        panic!("product")
    };
    assert!(
        storage
            .adopt_transfer(f.domain, destination, |fields| {
                let NormalizedValue::OwnedI64Cell(cell) = &mut fields[1] else {
                    panic!("cell")
                };
                cell.adopt_transfer(f.domain, destination)?;
                Err(ExecutionError::resource(
                    "test_extraction_adoption",
                    "interrupted",
                ))
            })
            .is_err()
    );
    assert!(extract(&f, value).is_err());
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    let mut value = owner(&f, 2);
    let NormalizedValue::OwnedProduct(storage) = &mut value.raw else {
        panic!("product")
    };
    storage
        .adopt_transfer(f.domain, destination, |fields| {
            let NormalizedValue::OwnedI64Cell(cell) = &mut fields[1] else {
                panic!("cell")
            };
            cell.adopt_transfer(f.domain, destination)
        })
        .unwrap();
    let mut extraction = value
        .unpack_owned(
            &f.program,
            destination,
            f.product,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    let payload = extraction
        .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
            panic!("ordinary field")
        })
        .unwrap()
        .unwrap();
    let child = extraction
        .next(
            &f.program,
            &ExecutionControl::uncancelled(),
            &mut |raw, _| {
                raw.memory_validate(destination, true)?;
                assert!(raw.memory_validate(f.domain, true).is_err());
                Value::memory(&f.program, raw)
            },
        )
        .unwrap()
        .unwrap();
    drop(extraction);
    drop(child);
    assert_eq!(payload.raw(), &f.raw(2));
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn consuming_extraction_rejects_read_inert_foreign_domain_and_live_loan_placements() {
    let f = Fixture::new(0);
    let value = owner(&f, 2);
    assert!(extract(&f, f.read(&value)).is_err());
    let inert = Value {
        raw: value.raw().clone(),
        origin: f.program.value_origin,
        class: Class::Memory,
        borrow: None,
    };
    assert!(extract(&f, inert).is_err());
    assert!(
        value
            .unpack_owned(
                &f.program,
                ValueOrigin::fresh().unwrap(),
                f.product,
                &ExecutionControl::uncancelled()
            )
            .is_err()
    );
    let value = owner(&f, 2);
    let read = f.read(&value);
    assert!(extract(&f, value).is_err());
    drop(read);
    assert!(extract(&f, owner(&f, 2)).is_ok());
}

#[test]
fn consuming_extraction_rejects_result_packets_and_ancestor_guards_but_allows_owner_frame_markers()
{
    let f = Fixture::new(0);
    let products = Products::start();
    let cells = Cells::start();
    let source = BorrowSource { frame: 7, local: 3 };
    let mut value = owner(&f, 2);
    value.borrow = Some(Box::new(BorrowMetadata {
        source: Some(source),
        placement: Some(source),
        packet: Some(BorrowResultTarget {
            source,
            position: 0,
        }),
        guards: Vec::new(),
    }));
    assert!(extract(&f, value).is_err());
    let mut value = owner(&f, 2);
    value.borrow = Some(Box::new(BorrowMetadata {
        source: Some(source),
        placement: Some(source),
        packet: None,
        guards: vec![owner(&f, 3)],
    }));
    assert!(extract(&f, value).is_err());
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    let mut value = owner(&f, 2);
    value.borrow = Some(Box::new(BorrowMetadata {
        source: Some(source),
        placement: Some(source),
        packet: None,
        guards: Vec::new(),
    }));
    assert!(extract(&f, value).is_ok());
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

fn nominal(f: &mut Fixture, ordinal: u64) -> TypeObjectDigest {
    let declaration = DeclarationReference {
        package: f.program.root_package,
        declaration: DeclarationId::migrate(b"checked-extraction-nominal", ordinal),
    };
    let object = TypeObject::new(TypeForm::Named { declaration }).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    f.program.types.insert(ty, object);
    f.program.owned_record_declarations.insert(declaration);
    let cell = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
        .unwrap()
        .0;
    let index = super::super::super::super::value::RecordLayoutIndex(
        f.program.records.len() as u32,
        f.program.value_origin,
    );
    let mut records = f.program.records.to_vec();
    records.push(
        super::super::super::super::prepare::NormalizedRecordLayout {
            declaration,
            owned: true,
            type_parameters: Arc::from([]),
            type_parameter_constraints: Arc::from([]),
            arguments: Arc::from([]),
            // Exact declaration order differs from structural field-name order.
            fields: [("rest", cell), ("payload", f.payload)]
                .into_iter()
                .enumerate()
                .map(|(index, (name, ty))| {
                    super::super::super::super::prepare::NormalizedRecordField {
                        reference: FieldReference {
                            package: declaration.package,
                            field: FieldId::migrate(
                                b"checked-extraction-field",
                                ordinal * 2 + index as u64,
                            ),
                        },
                        name: Name::new(name).unwrap(),
                        ty,
                    }
                })
                .collect::<Vec<_>>()
                .into(),
        },
    );
    f.program.records = records.into();
    f.program.record_instances.insert(ty, index);
    ty
}

fn nominal_owner(f: &Fixture, ty: TypeObjectDigest) -> Value {
    let cell = OwnedI64Cell::new(f.domain, 37);
    cell.establish_admission(f.program.value_origin).unwrap();
    let storage = OwnedProduct::create(
        f.domain,
        ty,
        vec![
            NormalizedValue::OwnedI64Cell(cell),
            f.admit(f.raw(1024)).unwrap().into_raw(),
        ],
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    storage.establish_admission(f.program.value_origin).unwrap();
    Value::memory(&f.program, NormalizedValue::OwnedProduct(storage)).unwrap()
}

#[test]
fn consuming_extraction_uses_exact_nominal_identity_and_declaration_field_order() {
    let mut f = Fixture::new(0);
    let ty = nominal(&mut f, 1);
    let foreign = nominal(&mut f, 2);
    assert!(
        nominal_owner(&f, ty)
            .unpack_owned(
                &f.program,
                f.domain,
                foreign,
                &ExecutionControl::uncancelled()
            )
            .is_err()
    );
    let mut extraction = nominal_owner(&f, ty)
        .unpack_owned(&f.program, f.domain, ty, &ExecutionControl::uncancelled())
        .unwrap();
    let mut checks = 0;
    let cell = extraction
        .next(
            &f.program,
            &ExecutionControl::uncancelled(),
            &mut |raw, child_ty| {
                checks += 1;
                owned_child(&f, raw, child_ty)
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(checks, 1);
    assert_eq!(cell.class, Class::Memory);
    let payload = extraction
        .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
            panic!("nominal ordinary field")
        })
        .unwrap()
        .unwrap();
    assert_eq!(payload.raw(), &f.raw(1024));
    drop(extraction);
    drop(cell);
    assert!(payload.borrow.is_none());
    // A same-shaped foreign layout cannot repair a nominal lookup.
    let foreign_index = f.program.record_instances[&foreign];
    f.program.record_instances.insert(ty, foreign_index);
    assert!(
        nominal_owner(&f, ty)
            .unpack_owned(&f.program, f.domain, ty, &ExecutionControl::uncancelled())
            .is_err()
    );
}

#[test]
fn consuming_extraction_accepts_an_empty_exact_nominal_owner() {
    let mut f = Fixture::new(0);
    let ty = nominal(&mut f, 1);
    let index = f.program.record_instances[&ty];
    Arc::make_mut(&mut f.program.records)[index.0 as usize].fields = Arc::from([]);
    let storage = OwnedProduct::create(
        f.domain,
        ty,
        Vec::new(),
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    storage.establish_admission(f.program.value_origin).unwrap();
    let value = Value::memory(&f.program, NormalizedValue::OwnedProduct(storage)).unwrap();
    assert_eq!(Value::owned_product_field_count(&f.program, ty).unwrap(), 0);
    let mut extraction = value
        .unpack_owned(&f.program, f.domain, ty, &ExecutionControl::uncancelled())
        .unwrap();
    assert!(
        extraction
            .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
                panic!("empty product has no child")
            })
            .unwrap()
            .is_none()
    );
}

#[test]
fn consuming_extraction_rejects_foreign_selected_type_missing_layout_and_complete_count_mismatch() {
    let mut f = Fixture::new(0);
    let object = TypeObject::new(TypeForm::OwnedProduct {
        fields: vec![
            StructuralTypeField {
                name: Name::new("other").unwrap(),
                ty: f.payload,
            },
            StructuralTypeField {
                name: Name::new("rest").unwrap(),
                ty: encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
                    .unwrap()
                    .0,
            },
        ],
    })
    .unwrap();
    let foreign = encode_type_object(&object).unwrap().0;
    f.program.types.insert(foreign, object);
    assert!(
        owner(&f, 2)
            .unpack_owned(
                &f.program,
                f.domain,
                foreign,
                &ExecutionControl::uncancelled()
            )
            .is_err()
    );
    let value = owner(&f, 2);
    let exact = f.program.types.remove(&f.product).unwrap();
    assert!(extract(&f, value).is_err());
    f.program.types.insert(f.product, exact);
    let products = Products::start();
    let cells = Cells::start();
    for extra in [false, true] {
        let mut fields = vec![f.raw(2)];
        if extra {
            let cell = OwnedI64Cell::new(f.domain, 37);
            cell.establish_admission(f.program.value_origin).unwrap();
            fields.extend([NormalizedValue::OwnedI64Cell(cell), f.raw(3)]);
        }
        let storage = OwnedProduct::create(
            f.domain,
            f.product,
            fields,
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap();
        // Synthetic bad parent certificates must expose neither a valid prefix
        // nor a child owner when a later field makes the complete count differ.
        storage.establish_admission(f.program.value_origin).unwrap();
        assert!(
            extract(
                &f,
                Value::memory(&f.program, NormalizedValue::OwnedProduct(storage)).unwrap()
            )
            .is_err()
        );
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
}

#[test]
fn consuming_extraction_preserves_independent_owned_child_custody_checks() {
    let f = Fixture::new(0);
    let products = Products::start();
    let cells = Cells::start();
    for kind in 0..5 {
        let domain = if kind == 1 {
            ValueOrigin::fresh().unwrap()
        } else {
            f.domain
        };
        let cell = OwnedI64Cell::new(domain, 37);
        if kind != 0 {
            cell.establish_admission(f.program.value_origin).unwrap();
        }
        let loan = if kind == 2 {
            Some(cell.borrow().unwrap())
        } else {
            None
        };
        let (child, guard) = if kind == 3 {
            (cell.clone(), Some(cell))
        } else {
            (cell, None)
        };
        let raw = if kind == 4 {
            drop(child);
            f.raw(2)
        } else {
            NormalizedValue::OwnedI64Cell(child)
        };
        // Unadmitted, foreign-domain, loaned, inert and wrong-form children all
        // remain insufficient under a fault-injected parent certificate.
        let storage = OwnedProduct::create(
            f.domain,
            f.product,
            vec![f.raw(2), raw],
            &ExecutionControl::uncancelled(),
            &mut |_| Ok(()),
        )
        .unwrap();
        storage.establish_admission(f.program.value_origin).unwrap();
        let mut extraction = extract(
            &f,
            Value::memory(&f.program, NormalizedValue::OwnedProduct(storage)).unwrap(),
        )
        .unwrap();
        let payload = extraction
            .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
                panic!("ordinary field")
            })
            .unwrap()
            .unwrap();
        assert!(
            extraction
                .next(
                    &f.program,
                    &ExecutionControl::uncancelled(),
                    &mut |raw, ty| owned_child(&f, raw, ty)
                )
                .is_err()
        );
        drop(extraction);
        drop(loan);
        drop(guard);
        assert_eq!(payload.raw(), &f.raw(2));
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
}

#[test]
fn consuming_extraction_lost_ordinary_eligibility_uses_the_independent_admission_path() {
    let mut f = Fixture::new(0);
    let value = owner(&f, 2);
    f.program.ordinary_types.remove(&f.payload);
    let mut extraction = extract(&f, value).unwrap();
    let mut admissions = 0;
    let payload = extraction
        .next(
            &f.program,
            &ExecutionControl::uncancelled(),
            &mut |raw, ty| {
                assert_eq!(ty, f.payload);
                admissions += 1;
                f.admit(raw)
            },
        )
        .unwrap()
        .unwrap();
    assert_eq!(admissions, 1);
    assert_eq!(payload.raw(), &f.raw(2));
}

#[test]
fn consuming_extraction_cancellation_and_child_refusal_release_every_remaining_field() {
    let f = Fixture::new(8);
    let products = Products::start();
    let cells = Cells::start();
    let mut failed = 0;
    let mut succeeded = 0;
    for checks in 0..24 {
        let value = owner(&f, 16);
        let control = ExecutionControl::cancel_after_checks(checks);
        let result = (|| {
            let mut extraction = value.unpack_owned(&f.program, f.domain, f.product, &control)?;
            let first = extraction.next(&f.program, &control, &mut |raw, ty| {
                owned_child(&f, raw, ty)
            })?;
            let second = extraction.next(&f.program, &control, &mut |raw, ty| {
                owned_child(&f, raw, ty)
            })?;
            Ok::<_, ExecutionError>((first, second))
        })();
        match result {
            Ok(values) => {
                succeeded += 1;
                drop(values);
            }
            Err(error) => {
                failed += 1;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
    assert!(failed > 0 && succeeded > 0);
    let mut extraction = extract(&f, owner(&f, 16)).unwrap();
    let first = extraction
        .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
            panic!("ordinary field")
        })
        .unwrap();
    assert!(
        extraction
            .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
                Err(ExecutionError::resource(
                    "test_extraction_refusal",
                    "refused owned child",
                ))
            })
            .is_err()
    );
    drop(extraction);
    drop(first);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn consuming_extraction_remaining_raw_cleanup_is_iterative_on_a_small_native_stack() {
    std::thread::Builder::new()
        .stack_size(128 * 1024)
        .spawn(|| {
            let mut deep = NormalizedValue::Unit;
            for _ in 0..100_000 {
                deep = NormalizedValue::Option(Some(Box::new(deep)));
            }
            // Deliberately unadmitted fault data exercises cleanup only. It never
            // passes extraction or becomes a checked ordinary child.
            let extraction = ProductExtraction {
                children: RawArguments::new(vec![deep]),
                program: ValueOrigin::fresh().unwrap(),
                ty: encode_type_object(&TypeObject::new(TypeForm::Unit).unwrap())
                    .unwrap()
                    .0,
                index: 0,
            };
            drop(extraction);
        })
        .unwrap()
        .join()
        .unwrap();
}
