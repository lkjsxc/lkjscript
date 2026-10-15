#![allow(clippy::unwrap_used, clippy::panic)]
//! Synthetic hostile layouts and storage observations, independent of lowering.
use super::super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells};
use super::super::super::owned_product::{OwnedProduct, StorageObservation as Products};
use super::metadata_tests::fixture::Fixture;
use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::kernel::{
    DeclarationReference, FieldReference, TypeObject, encode_type_object,
};
use crate::platform::semantic_id::{DeclarationId, FieldId, TypeParameterId};

fn nominal(f: &mut Fixture, ordinal: u64, fields: bool) -> (TypeObjectDigest, RecordLayoutIndex) {
    let declaration = DeclarationReference {
        package: f.program.root_package,
        declaration: DeclarationId::migrate(b"runtime-owned-record", ordinal),
    };
    let object = TypeObject::new(TypeForm::Named { declaration }).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    f.program.types.insert(ty, object);
    f.program.owned_record_declarations.insert(declaration);
    let cell = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
        .unwrap()
        .0;
    let layout = RecordLayoutIndex(f.program.records.len() as u32, f.program.value_origin);
    let mut records = f.program.records.to_vec();
    records.push(super::super::super::prepare::NormalizedRecordLayout {
        declaration,
        owned: true,
        type_parameters: Arc::from([]),
        type_parameter_constraints: Arc::from([]),
        arguments: Arc::from([]),
        fields: if fields {
            // Nominal order deliberately differs from structural name order.
            vec![("rest", cell), ("payload", f.payload)]
                .into_iter()
                .enumerate()
                .map(
                    |(index, (name, ty))| super::super::super::prepare::NormalizedRecordField {
                        reference: FieldReference {
                            package: declaration.package,
                            field: FieldId::migrate(
                                b"runtime-owned-record-field",
                                ordinal * 2 + index as u64,
                            ),
                        },
                        name: Name::new(name).unwrap(),
                        ty,
                    },
                )
                .collect::<Vec<_>>()
                .into()
        } else {
            Arc::from([])
        },
    });
    f.program.records = records.into();
    f.program.record_instances.insert(ty, layout);
    (ty, layout)
}

fn owner(f: &Fixture, ty: TypeObjectDigest, count: usize) -> Value {
    let payload = f.admit(f.raw(count)).unwrap().into_raw();
    let token = OwnedProduct::create(
        f.domain,
        ty,
        vec![
            NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(f.domain, 37)),
            payload,
        ],
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    token.establish_admission(f.program.value_origin).unwrap();
    Value::memory(&f.program, NormalizedValue::OwnedProduct(token)).unwrap()
}

#[test]
fn nominal_record_uses_one_envelope_and_preserves_children_through_unpack_repack() {
    let mut f = Fixture::new(0);
    let (ty, _) = nominal(&mut f, 1, true);
    let products = Products::start();
    let cells = Cells::start();
    let owner = owner(&f, ty, 1024);
    assert_eq!(products.created(), 1);
    let NormalizedValue::OwnedProduct(token) = owner.into_raw() else {
        panic!("owned record");
    };
    let vector = token.vector_identity();
    let read = token.borrow().unwrap();
    assert_eq!(read.vector_identity(), vector);
    assert_eq!(products.created(), 1);
    assert!(token.validate(f.domain, true).is_err());
    drop(read);
    let values = token
        .unpack(f.domain, ty, &ExecutionControl::uncancelled())
        .unwrap();
    let NormalizedValue::OwnedI64Cell(cell) = &values[0] else {
        panic!("cell");
    };
    let child_identity = cell.allocation_identity();
    let repacked = OwnedProduct::create(
        f.domain,
        ty,
        values,
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(products.created(), 2);
    let values = repacked
        .unpack(f.domain, ty, &ExecutionControl::uncancelled())
        .unwrap();
    let NormalizedValue::OwnedI64Cell(cell) = &values[0] else {
        panic!("cell");
    };
    assert_eq!(cell.allocation_identity(), child_identity);
    assert_eq!(values[1], f.raw(1024));
    products.assert_transfers_preserve_allocations();
    drop(values);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn nominal_metadata_projection_is_bounded_and_rejects_same_shape_foreign_identity() {
    let mut f = Fixture::new(0);
    let (ty, layout) = nominal(&mut f, 1, true);
    let (_, foreign) = nominal(&mut f, 2, true);
    let products = Products::start();
    let cells = Cells::start();
    for count in [0, 8, 1024, 65536] {
        let owner = owner(&f, ty, count);
        let read = owner.duplicate(ParameterUse::Borrow).unwrap();
        let before = products.created();
        for _ in 0..32 {
            let work = super::super::super::list::Work::current();
            let projected = read
                .owned_record_metadata(
                    &f.program,
                    f.domain,
                    layout,
                    1,
                    &ExecutionControl::uncancelled(),
                    &mut |_| panic!("immutable projection has no backing allocation"),
                )
                .unwrap();
            assert_eq!(work.since().node_visits, 0);
            assert_eq!(projected.class, Class::Free);
            assert!(projected.borrow.is_none());
            assert_eq!(projected.raw(), &f.raw(count));
            assert_eq!(projected.ordinary_type, Some(f.payload));
        }
        assert_eq!(products.created(), before);
        assert!(
            read.owned_record_metadata(
                &f.program,
                f.domain,
                foreign,
                1,
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(())
            )
            .is_err()
        );
        assert!(
            read.owned_record_metadata(
                &f.program,
                f.domain,
                layout,
                0,
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(())
            )
            .is_err()
        );
        assert!(
            read.owned_metadata(
                &f.program,
                f.domain,
                &Name::new("payload").unwrap(),
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(())
            )
            .is_err()
        );
        let retained = read
            .owned_record_metadata(
                &f.program,
                f.domain,
                layout,
                1,
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(()),
            )
            .unwrap();
        drop(read);
        drop(owner);
        assert_eq!(products.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(retained.raw(), &f.raw(count));
    }
}

#[test]
fn empty_nominal_record_is_affine_and_cannot_enter_ordinary_codecs() {
    let mut f = Fixture::new(0);
    let (ty, layout) = nominal(&mut f, 3, false);
    assert!(f.program.is_owned_type(ty));
    let token = OwnedProduct::create(
        f.domain,
        ty,
        Vec::new(),
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    token.establish_admission(f.program.value_origin).unwrap();
    let owner = Value::memory(&f.program, NormalizedValue::OwnedProduct(token)).unwrap();
    assert!(owner.duplicate(ParameterUse::Unrestricted).is_err());
    assert!(
        Value::record(
            &f.program,
            Some(layout),
            Vec::new(),
            &mut ValueWork::default()
        )
        .is_err()
    );
    assert!(
        super::super::super::codec::decode_value(
            &f.program,
            &serde_json::json!({}),
            ty,
            crate::platform::json::JsonLimits::default()
        )
        .is_err()
    );
    assert!(
        super::super::super::data_codec::encode_typed_with_control(
            &f.program,
            owner.raw(),
            ty,
            &ExecutionControl::uncancelled()
        )
        .is_err()
    );
}

#[test]
fn nominal_projection_rejects_raw_mutation_and_failed_adoption_after_shape_restore() {
    let mut f = Fixture::new(0);
    let (ty, layout) = nominal(&mut f, 1, true);
    let first_owner = owner(&f, ty, 8);
    let NormalizedValue::OwnedProduct(token) = first_owner.into_raw() else {
        panic!("owned record");
    };
    token
        .replace(
            f.domain,
            1,
            NormalizedValue::Bool(false),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    token
        .replace(f.domain, 1, f.raw(8), &ExecutionControl::uncancelled())
        .unwrap();
    let forged = Value {
        raw: NormalizedValue::OwnedProduct(token.borrow().unwrap()),
        origin: f.program.value_origin,
        class: Class::Memory,
        ordinary_type: None,
        borrow: None,
    };
    assert!(
        forged
            .owned_record_metadata(
                &f.program,
                f.domain,
                layout,
                1,
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(())
            )
            .is_err()
    );
    drop(forged);
    drop(token);

    let owner = owner(&f, ty, 8);
    let NormalizedValue::OwnedProduct(mut token) = owner.into_raw() else {
        panic!("owned record");
    };
    assert!(
        token
            .adopt_transfer(f.domain, ValueOrigin::fresh().unwrap(), |_| {
                Err(admission_error("injected interrupted adoption"))
            })
            .is_err()
    );
    let forged = Value {
        raw: NormalizedValue::OwnedProduct(token.borrow().unwrap()),
        origin: f.program.value_origin,
        class: Class::Memory,
        ordinary_type: None,
        borrow: None,
    };
    assert!(
        forged
            .owned_record_metadata(
                &f.program,
                f.domain,
                layout,
                1,
                &ExecutionControl::uncancelled(),
                &mut |_| Ok(())
            )
            .is_err()
    );
}

#[test]
fn nominal_projection_reserves_inline_spines_and_cancellation_retains_owner() {
    let mut f = Fixture::new(3);
    let (ty, layout) = nominal(&mut f, 1, true);
    let products = Products::start();
    let cells = Cells::start();
    let owner = owner(&f, ty, 1024);
    let read = owner.duplicate(ParameterUse::Borrow).unwrap();
    let mut completed = false;
    let mut cancelled = false;
    for checks in 0..24 {
        match read.owned_record_metadata(
            &f.program,
            f.domain,
            layout,
            1,
            &ExecutionControl::cancel_after_checks(checks),
            &mut |_| Ok(()),
        ) {
            Ok(value) => {
                completed = true;
                assert_eq!(value.raw(), &f.raw(1024));
            }
            Err(error) => {
                cancelled = true;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
    }
    assert!(completed && cancelled);
    let mut reservations = Vec::new();
    read.owned_record_metadata(
        &f.program,
        f.domain,
        layout,
        1,
        &ExecutionControl::uncancelled(),
        &mut |bytes| {
            reservations.push(bytes);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(
        reservations,
        vec![3 * std::mem::size_of::<NormalizedValue>() as u64]
    );
    let error = read
        .owned_record_metadata(
            &f.program,
            f.domain,
            layout,
            1,
            &ExecutionControl::uncancelled(),
            &mut |_| Err(ExecutionError::resource("nominal_clone_quota", "refused")),
        )
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    drop(read);
    let NormalizedValue::OwnedProduct(token) = owner.raw() else {
        panic!("owned record");
    };
    token.validate(f.domain, true).unwrap();
    token.validate_admission(f.program.value_origin).unwrap();
    assert_eq!(products.created(), 1);
    drop(owner);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn empty_nominal_transfer_and_share_include_unused_actual_arguments() {
    let mut f = Fixture::new(0);
    let (empty, index) = nominal(&mut f, 3, false);
    assert!(
        super::super::transfer::admit_type(&f.program, empty, &ExecutionControl::uncancelled())
            .unwrap()
    );
    assert!(
        super::super::transfer::admit_shareable_type(
            &f.program,
            empty,
            &ExecutionControl::uncancelled()
        )
        .unwrap()
    );
    let declaration = f.program.records[index.0 as usize].declaration;
    let secret_object = TypeObject::new(TypeForm::Secret).unwrap();
    let secret = encode_type_object(&secret_object).unwrap().0;
    f.program.types.insert(secret, secret_object);
    let application = TypeObject::new(TypeForm::Applied {
        declaration,
        arguments: vec![secret],
    })
    .unwrap();
    let ty = encode_type_object(&application).unwrap().0;
    f.program.types.insert(ty, application);
    let mut records = f.program.records.to_vec();
    let mut phantom = records[index.0 as usize].clone();
    phantom.type_parameters = Arc::from([TypeParameterId::migrate(b"nominal-runtime-phantom", 0)]);
    phantom.arguments = Arc::from([secret]);
    let index = RecordLayoutIndex(records.len() as u32, f.program.value_origin);
    records.push(phantom);
    f.program.records = records.into();
    f.program.record_instances.insert(ty, index);
    assert!(f.program.is_owned_type(ty));
    assert!(
        super::super::transfer::admit_type(&f.program, ty, &ExecutionControl::uncancelled())
            .is_err()
    );
    assert!(
        super::super::transfer::admit_shareable_type(
            &f.program,
            ty,
            &ExecutionControl::uncancelled()
        )
        .is_err()
    );
}
