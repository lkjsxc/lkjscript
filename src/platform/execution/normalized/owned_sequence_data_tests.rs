//! Physical custody, reservation and immutable-snapshot obligations for data sequences.
use super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells};
use super::super::owned_product::StorageObservation as Composites;
use super::*;

fn types() -> [TypeObjectDigest; 3] {
    [
        TypeObjectDigest::from_bytes([181; 32]),
        TypeObjectDigest::from_bytes([182; 32]),
        TypeObjectDigest::from_bytes([183; 32]),
    ]
}

fn unpack_pair(
    raw: NormalizedValue,
    origin: ValueOrigin,
    ty: TypeObjectDigest,
    control: &ExecutionControl,
) -> (OwnedSequence, NormalizedValue) {
    let NormalizedValue::OwnedProduct(product) = raw else {
        panic!("result product")
    };
    let mut fields = product.unpack(origin, ty, control).unwrap().into_iter();
    let NormalizedValue::OwnedSequence(rest) = fields.next().unwrap() else {
        panic!("remaining sequence")
    };
    let value = fields.next().unwrap();
    assert!(fields.next().is_none());
    (rest, value)
}

fn remove(
    sequence: OwnedSequence,
    origin: ValueOrigin,
    control: &ExecutionControl,
) -> (OwnedSequence, Option<NormalizedValue>) {
    let NormalizedValue::OwnedChoice(choice) = sequence
        .pop(origin, types()[1], types()[2], control, &mut |_| Ok(()))
        .unwrap()
    else {
        panic!("pop choice")
    };
    let (case, payload) = choice.select(origin, types()[1], control).unwrap();
    if case == 0 {
        let NormalizedValue::OwnedSequence(rest) = payload else {
            panic!("empty rest")
        };
        (rest, None)
    } else {
        let (rest, value) = unpack_pair(payload, origin, types()[2], control);
        (rest, Some(value))
    }
}

#[test]
fn data_sequence_scalar_storage_reuses_capacity_without_cell_allocations() {
    let cells = Cells::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut sequence =
        OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(())).unwrap();
    let mut growth = 0;
    for number in 0..4096 {
        sequence = sequence
            .push(origin, NormalizedValue::I64(number), &control, &mut |_| {
                growth += 1;
                Ok(())
            })
            .unwrap();
    }
    assert_eq!(growth, 10, "8 then geometric growth through 4096 slots");
    assert_eq!(cells.created(), 0);
    let vector = sequence.storage.vector_identity();
    let capacity = sequence.storage.vector_capacity();
    for number in (0..4096).rev() {
        let (rest, value) = remove(sequence, origin, &control);
        assert_eq!(value, Some(NormalizedValue::I64(number)));
        sequence = rest;
        assert_eq!(sequence.storage.vector_identity(), vector);
        assert_eq!(sequence.storage.vector_capacity(), capacity);
    }
    let (rest, value) = remove(sequence, origin, &control);
    assert!(value.is_none());
    let sequence = rest
        .push(
            origin,
            NormalizedValue::I64(i64::MIN),
            &control,
            &mut |_| panic!("empty storage must retain capacity"),
        )
        .unwrap();
    let read = sequence.borrow().unwrap();
    assert_eq!(
        read.get(origin, 0, &control, &mut |_| panic!(
            "scalar clone allocates no storage"
        ))
        .unwrap(),
        NormalizedValue::I64(i64::MIN)
    );
    drop(read);
    assert_eq!(sequence.storage.vector_identity(), vector);
    assert_eq!(cells.created(), 0);
}

#[test]
fn data_sequence_retained_read_survives_replace_pop_transfer_and_destruction() {
    let composites = Composites::start();
    let origin = ValueOrigin::fresh().unwrap();
    let destination = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let original = NormalizedValue::Option(Some(Box::new(
        NormalizedValue::list(vec![NormalizedValue::I64(17), NormalizedValue::I64(23)]).unwrap(),
    )));
    let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
        .unwrap()
        .push(origin, original.clone(), &control, &mut |_| Ok(()))
        .unwrap();
    let read = sequence.borrow().unwrap();
    let retained = read.get(origin, 0, &control, &mut |_| Ok(())).unwrap();
    drop(read);
    assert!(!sequence.owns_live_loans());
    let replacement = NormalizedValue::Option(None);
    let raw = sequence
        .replace(
            origin,
            0,
            replacement.clone(),
            types()[2],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
    let (mut sequence, displaced) = unpack_pair(raw, origin, types()[2], &control);
    assert_eq!(displaced, original);
    assert_eq!(retained, original);
    sequence
        .adopt_transfer(origin, destination, |_| Ok(()))
        .unwrap();
    let read = sequence.borrow().unwrap();
    assert_eq!(
        read.get(destination, 0, &control, &mut |_| Ok(())).unwrap(),
        replacement
    );
    assert!(read.get(origin, 0, &control, &mut |_| Ok(())).is_err());
    drop(read);
    let (rest, removed) = remove(sequence, destination, &control);
    assert_eq!(removed, Some(replacement));
    drop(rest);
    assert_eq!(composites.live(), (0, 0));
    assert_eq!(retained, original);
}

#[test]
fn data_sequence_get_reserves_only_inline_sum_spine_and_checks_all_index_extremes() {
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let raw = NormalizedValue::Option(Some(Box::new(NormalizedValue::Result {
        success: true,
        value: Box::new(NormalizedValue::Option(Some(Box::new(
            NormalizedValue::I64(71),
        )))),
    })));
    let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
        .unwrap()
        .push(origin, raw.clone(), &control, &mut |_| Ok(()))
        .unwrap();
    let read = sequence.borrow().unwrap();
    let mut charges = Vec::new();
    let retained = read
        .get(origin, 0, &control, &mut |bytes| {
            charges.push(bytes);
            Ok(())
        })
        .unwrap();
    assert_eq!(
        charges,
        vec![3 * std::mem::size_of::<NormalizedValue>() as u64]
    );
    assert_eq!(retained, raw);
    for index in [i64::MIN, -1, 1, i64::MAX] {
        let error = read
            .get(origin, index, &control, &mut |_| {
                panic!("invalid index cannot reserve")
            })
            .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Trap);
        assert_eq!(error.code, "normalized_sequence_index");
    }
    assert!(sequence.get(origin, 0, &control, &mut |_| Ok(())).is_err());
    drop(read);
    sequence.validate(origin, true).unwrap();
}

#[test]
fn raw_sequence_replace_invalidates_prior_admission_and_never_certifies_wrapper() {
    let origin = ValueOrigin::fresh().unwrap();
    let program = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
        .unwrap()
        .push(origin, NormalizedValue::I64(13), &control, &mut |_| Ok(()))
        .unwrap();
    sequence.establish_admission(program).unwrap();
    let NormalizedValue::OwnedProduct(product) = sequence
        .replace(
            origin,
            0,
            NormalizedValue::Bool(false),
            types()[2],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap()
    else {
        panic!("result product")
    };
    assert!(product.validate_admission(program).is_err());
    let (rest, displaced) = unpack_pair(
        NormalizedValue::OwnedProduct(product),
        origin,
        types()[2],
        &control,
    );
    assert_eq!(displaced, NormalizedValue::I64(13));
    assert_eq!(rest.admitted_program(), None);
    assert!(rest.validate_admission(program).is_err());
    let raw = rest
        .replace(
            origin,
            0,
            NormalizedValue::I64(13),
            types()[2],
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
    let (rest, _) = unpack_pair(raw, origin, types()[2], &control);
    assert!(
        rest.validate_admission(program).is_err(),
        "restored raw shape is not restored proof"
    );
}

#[test]
fn sequence_replace_reserves_before_swap_preserves_owned_identity_and_joins_failures() {
    let composites = Composites::start();
    let cells = Cells::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    for mode in 0..3 {
        let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
            .unwrap()
            .push(
                origin,
                NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, 31)),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
        let replacement = NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, 37));
        let before = composites.created();
        let cancelled = ExecutionControl::uncancelled();
        let mut reservations = 0;
        let error = sequence
            .replace(
                origin,
                if mode == 2 { -1 } else { 0 },
                replacement,
                types()[2],
                &cancelled,
                &mut |_| {
                    reservations += 1;
                    if mode == 0 {
                        Err(ExecutionError::resource("replace_quota", "refused"))
                    } else {
                        cancelled.cancel();
                        Ok(())
                    }
                },
            )
            .unwrap_err();
        assert_eq!(reservations, usize::from(mode != 2));
        assert_eq!(composites.created(), before);
        assert_eq!(
            error.class,
            [
                ExecutionFailureClass::Resource,
                ExecutionFailureClass::Cancelled,
                ExecutionFailureClass::Trap
            ][mode]
        );
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }
    let old = OwnedI64Cell::new(origin, 41);
    let old_identity = old.allocation_identity();
    let new = OwnedI64Cell::new(origin, 43);
    let new_identity = new.allocation_identity();
    let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
        .unwrap()
        .push(
            origin,
            NormalizedValue::OwnedI64Cell(old),
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
    let vector = sequence.storage.vector_identity();
    let raw = sequence
        .replace(
            origin,
            0,
            NormalizedValue::OwnedI64Cell(new),
            types()[2],
            &control,
            &mut |bytes| {
                assert_eq!(
                    bytes,
                    OwnedProduct::ALLOCATION_BYTES
                        + 2 * std::mem::size_of::<NormalizedValue>() as u64
                );
                Ok(())
            },
        )
        .unwrap();
    let (sequence, displaced) = unpack_pair(raw, origin, types()[2], &control);
    assert_eq!(sequence.storage.vector_identity(), vector);
    let NormalizedValue::OwnedI64Cell(displaced) = displaced else {
        panic!("old cell")
    };
    assert_eq!(displaced.allocation_identity(), old_identity);
    assert_eq!(displaced.extract().unwrap(), 41);
    let (rest, value) = remove(sequence, origin, &control);
    let Some(NormalizedValue::OwnedI64Cell(value)) = value else {
        panic!("new cell")
    };
    assert_eq!(value.allocation_identity(), new_identity);
    assert_eq!(value.extract().unwrap(), 43);
    drop(rest);
    assert_eq!(composites.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn sequence_replace_cancellation_sweep_releases_both_children_and_result_envelope() {
    let composites = Composites::start();
    let cells = Cells::start();
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut failures = 0;
    let mut successes = 0;
    for checks in 0..24 {
        let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
            .unwrap()
            .push(
                origin,
                NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, 47)),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
        match sequence.replace(
            origin,
            0,
            NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, 53)),
            types()[2],
            &ExecutionControl::cancel_after_checks(checks),
            &mut |_| Ok(()),
        ) {
            Ok(raw) => {
                successes += 1;
                drop(raw);
            }
            Err(error) => {
                failures += 1;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
        assert_eq!(composites.live(), (0, 0), "checks={checks}");
        assert_eq!(cells.live(), (0, 0), "checks={checks}");
    }
    assert!(failures > 0 && successes > 0);
}
