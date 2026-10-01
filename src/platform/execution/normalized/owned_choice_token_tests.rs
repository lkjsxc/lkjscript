//! Independent sealed-token, encoding and mixed-depth cleanup controls.
use super::super::{
    byte_buffer::ByteBuffer,
    owned_choice::OwnedChoice,
    owned_product::{OwnedProduct, StorageObservation},
    value::ValueOrigin,
};
use super::*;
use crate::platform::kernel::{
    StructuralTypeField, TypeObject, contract, decode_type_object, encode_type_object,
};

#[test]
fn owned_choice_type_encoding_is_disjoint_and_cases_are_canonical() {
    let child = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let cases = vec![
        StructuralTypeField {
            name: Name::new("accepted").unwrap(),
            ty: child,
        },
        StructuralTypeField {
            name: Name::new("rejected").unwrap(),
            ty: child,
        },
    ];
    let choice = TypeObject::new(TypeForm::OwnedChoice {
        cases: cases.clone(),
    })
    .unwrap();
    let (digest, bytes) = encode_type_object(&choice).unwrap();
    assert_eq!(&bytes[..8], b"LKJCHO01");
    assert_eq!(decode_type_object(&bytes, digest).unwrap(), choice);
    let product = TypeObject::new(TypeForm::OwnedProduct {
        fields: cases.clone(),
    })
    .unwrap();
    assert_ne!(digest, encode_type_object(&product).unwrap().0);
    let mut disguised = choice;
    disguised.contract_version = 10;
    let bytes = crate::platform::packed::encode(
        contract::TYPE_OBJECT_MAGIC,
        contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
        &disguised,
        contract::MAXIMUM_TYPE_OBJECT_BYTES,
    )
    .unwrap();
    assert!(decode_type_object(&bytes, TypeObjectDigest::of(&bytes)).is_err());
    for cases in [
        vec![],
        vec![cases[0].clone(); 2],
        vec![cases[1].clone(), cases[0].clone()],
        vec![cases[0].clone(); contract::MAXIMUM_CHILDREN + 1],
    ] {
        assert!(TypeObject::new(TypeForm::OwnedChoice { cases }).is_err());
    }
}

#[test]
fn owned_choice_token_preserves_one_owner_and_rejects_foreign_or_borrowed_selection() {
    let composites = StorageObservation::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let ty = TypeObjectDigest::from_bytes([53; 32]);
    let buffer = ByteBuffer::create(origin, &control, &mut |_| Ok(()))
        .unwrap()
        .push(255, &control, &mut |_| Ok(()))
        .unwrap();
    let pointer = buffer.allocation_identity();
    let child_marker = buffer.clone();
    let choice = OwnedChoice::create(
        origin,
        ty,
        1,
        NormalizedValue::ByteBuffer(buffer),
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let inert = choice.clone();
    assert!(inert.validate(origin, false).is_err());
    assert!(
        choice
            .validate(ValueOrigin::fresh().unwrap(), true)
            .is_err()
    );
    let loan = choice.borrow().unwrap();
    let reborrow = loan.borrow().unwrap();
    assert!(choice.validate(origin, true).is_err());
    assert!(loan.validate(origin, true).is_err());
    assert!(reborrow.select(origin, ty, &control).is_err());
    drop(loan);
    let (selected, payload) = choice.select(origin, ty, &control).unwrap();
    assert_eq!(selected, 1);
    let NormalizedValue::ByteBuffer(buffer) = payload else {
        panic!("selected buffer");
    };
    assert_eq!(buffer.allocation_identity(), pointer);
    assert_eq!(&*buffer.freeze().unwrap(), &[255]);
    assert!(child_marker.validate(origin, false).is_err());
    assert!(inert.validate(origin, false).is_err());
    assert_eq!(composites.live(), (0, 0));
    for wrong_origin in [false, true] {
        let buffer = ByteBuffer::create(origin, &control, &mut |_| Ok(())).unwrap();
        let child_marker = buffer.clone();
        let choice = OwnedChoice::create(
            origin,
            ty,
            1,
            NormalizedValue::ByteBuffer(buffer),
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        let inert = choice.clone();
        let other = ValueOrigin::fresh().unwrap();
        let other_ty = TypeObjectDigest::from_bytes([54; 32]);
        assert!(
            choice
                .select(
                    if wrong_origin { other } else { origin },
                    if wrong_origin { ty } else { other_ty },
                    &control
                )
                .is_err()
        );
        assert!(inert.validate(origin, false).is_err());
        assert!(child_marker.validate(origin, false).is_err());
        assert_eq!(composites.live(), (0, 0));
    }
    // Even a selected ordinary case remains affine until complete case analysis.
    let choice = OwnedChoice::create(origin, ty, 0, NormalizedValue::Unit, &control, &mut |_| {
        Ok(())
    })
    .unwrap();
    let inert = choice.clone();
    assert!(inert.validate(origin, false).is_err());
    assert_eq!(
        choice.select(origin, ty, &control).unwrap(),
        (0, NormalizedValue::Unit)
    );
    assert_eq!(composites.live(), (0, 0));
}

#[test]
fn owned_choice_cancellation_during_each_reservation_revokes_payload_without_storage_growth() {
    let composites = StorageObservation::start();
    let origin = ValueOrigin::fresh().unwrap();
    let ty = TypeObjectDigest::from_bytes([53; 32]);
    for stop in [1, 2] {
        let control = ExecutionControl::uncancelled();
        let buffer = ByteBuffer::create(origin, &control, &mut |_| Ok(())).unwrap();
        let marker = buffer.clone();
        let mut reservations = 0;
        let before = composites.created();
        let error = OwnedChoice::create(
            origin,
            ty,
            1,
            NormalizedValue::ByteBuffer(buffer),
            &control,
            &mut |_| {
                reservations += 1;
                if reservations == stop {
                    control.cancel();
                }
                Ok(())
            },
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Cancelled);
        assert_eq!(reservations, stop);
        assert_eq!(composites.created(), before);
        assert_eq!(composites.live(), (0, 0));
        assert!(marker.validate(origin, false).is_err());
    }
}

#[test]
fn owned_choice_mixed_maximum_depth_drops_iteratively_with_surviving_markers() {
    let composites = StorageObservation::start();
    let buffers = super::super::byte_buffer::StorageObservation::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let child = ByteBuffer::create(origin, &control, &mut |_| Ok(())).unwrap();
    let child_marker = child.clone();
    let mut value = NormalizedValue::ByteBuffer(child);
    let mut markers = Vec::new();
    for depth in 0..contract::MAXIMUM_TYPE_DEPTH {
        let fields = vec![StructuralTypeField {
            name: Name::new("child").unwrap(),
            ty,
        }];
        let form = if depth % 2 == 0 {
            TypeForm::OwnedChoice { cases: fields }
        } else {
            TypeForm::OwnedProduct { fields }
        };
        ty = encode_type_object(&TypeObject::new(form).unwrap())
            .unwrap()
            .0;
        value = if depth % 2 == 0 {
            NormalizedValue::OwnedChoice(
                OwnedChoice::create(origin, ty, 0, value, &control, &mut |_| Ok(())).unwrap(),
            )
        } else {
            NormalizedValue::OwnedProduct(
                OwnedProduct::create(origin, ty, vec![value], &control, &mut |_| Ok(())).unwrap(),
            )
        };
        markers.push(value.clone());
    }
    assert_eq!(composites.live(), (contract::MAXIMUM_TYPE_DEPTH, 0));
    drop(value);
    assert_eq!(composites.live(), (0, 0));
    assert_eq!(buffers.live(), (0, 0));
    assert!(child_marker.validate(origin, false).is_err());
    assert!(
        markers
            .iter()
            .all(|marker| marker.memory_validate(origin, false).is_err())
    );
}
