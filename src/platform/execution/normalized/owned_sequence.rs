//! A runtime-sized vector of owned children or immutable data, with one sealed owner.
use super::owned_choice::OwnedChoice;
use super::owned_product::OwnedProduct;
use super::owned_storage::OwnedStorage;
use super::value::{NormalizedValue, ValueOrigin};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::TypeObjectDigest;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OwnedSequence {
    pub(super) storage: OwnedStorage,
}

impl OwnedSequence {
    pub(super) fn create(
        origin: ValueOrigin,
        ty: TypeObjectDigest,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        Ok(Self {
            storage: OwnedStorage::create(origin, ty, Vec::new(), control, reserve)?,
        })
    }
    pub(super) fn ty(&self) -> TypeObjectDigest {
        self.storage.ty()
    }
    pub(super) fn validate(
        &self,
        origin: ValueOrigin,
        consume: bool,
    ) -> Result<(), ExecutionError> {
        self.storage.validate(origin, consume)
    }
    pub(super) fn borrow(&self) -> Result<Self, ExecutionError> {
        Ok(Self {
            storage: self.storage.borrow()?,
        })
    }
    pub(super) fn establish_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        self.storage.establish_admission(program)
    }
    pub(super) fn validate_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        self.storage.validate_admission(program)
    }
    pub(super) fn admitted_program(&self) -> Option<ValueOrigin> {
        self.storage.admitted_program()
    }
    pub(super) fn inherit_admission(&self, program: ValueOrigin) -> Result<(), ExecutionError> {
        self.storage.inherit_admission(program)
    }
    pub(super) fn adopt_scoped_read(
        &mut self,
        source: ValueOrigin,
        destination: ValueOrigin,
    ) -> Result<(), ExecutionError> {
        self.storage.adopt_scoped_read(source, destination)
    }
    pub(super) fn is_borrowed(&self) -> bool {
        self.storage.is_borrowed()
    }
    pub(super) fn owns_live_loans(&self) -> bool {
        self.storage.owns_live_loans()
    }
    pub(super) fn len(
        &self,
        origin: ValueOrigin,
        control: &ExecutionControl,
    ) -> Result<usize, ExecutionError> {
        control.check()?;
        self.storage.len(origin)
    }
    pub(super) fn push(
        self,
        origin: ValueOrigin,
        value: NormalizedValue,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<Self, ExecutionError> {
        self.storage.push(origin, value, control, reserve)?;
        control.check()?;
        Ok(self)
    }
    /// Reserve every result envelope before detaching the last child. The
    /// original vector stays attached to the returned sequence in both cases.
    pub(super) fn pop(
        self,
        origin: ValueOrigin,
        result_type: TypeObjectDigest,
        item_product_type: TypeObjectDigest,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.validate(origin, true)?;
        let admitted_program = self.admitted_program();
        let empty = self.len(origin, control)? == 0;
        let item_bytes = if empty {
            0
        } else {
            OwnedProduct::ALLOCATION_BYTES + 2 * std::mem::size_of::<NormalizedValue>() as u64
        };
        reserve(OwnedChoice::ALLOCATION_BYTES + item_bytes)?;
        control.check()?;
        let value = self.storage.pop(origin, control)?;
        if let Some(program) = admitted_program {
            self.establish_admission(program)?;
        }
        let (case, payload) = if let Some(value) = value {
            let product = OwnedProduct::create(
                origin,
                item_product_type,
                vec![NormalizedValue::OwnedSequence(self), value],
                control,
                &mut |_| Ok(()),
            )?;
            (1, NormalizedValue::OwnedProduct(product))
        } else {
            (0, NormalizedValue::OwnedSequence(self))
        };
        let choice =
            OwnedChoice::create(origin, result_type, case, payload, control, &mut |_| Ok(()))?;
        // These supplied wrapper digests require independent typed admission by
        // the evaluator; the saved sequence proof certifies only the remaining self.
        Ok(NormalizedValue::OwnedChoice(choice))
    }
    pub(super) fn borrow_item(
        &self,
        origin: ValueOrigin,
        index: i64,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.validate(origin, false)?;
        control.check()?;
        let index = self.index(origin, index)?;
        self.storage.borrow_field(origin, index, control, reserve)
    }
    /// The caller retains a short parent read loan while immutable backing is
    /// cloned. The returned data has no loan on this sequence.
    pub(super) fn get(
        &self,
        origin: ValueOrigin,
        index: i64,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.validate(origin, false)?;
        control.check()?;
        let index = self.index(origin, index)?;
        self.storage.read_metadata(origin, index, control, reserve)
    }
    /// Reserve the exact result envelope before swapping the child. Raw
    /// replacement invalidates admission, including admission of the remaining
    /// sequence; the evaluator restores it from the prior proof and new child.
    pub(super) fn replace(
        self,
        origin: ValueOrigin,
        index: i64,
        value: NormalizedValue,
        result_type: TypeObjectDigest,
        control: &ExecutionControl,
        reserve: &mut impl FnMut(u64) -> Result<(), ExecutionError>,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.validate(origin, true)?;
        control.check()?;
        let index = self.index(origin, index)?;
        reserve(
            OwnedProduct::ALLOCATION_BYTES + 2 * std::mem::size_of::<NormalizedValue>() as u64,
        )?;
        control.check()?;
        let displaced = self.storage.replace(origin, index, value, control)?;
        let product = OwnedProduct::create(
            origin,
            result_type,
            vec![NormalizedValue::OwnedSequence(self), displaced],
            control,
            &mut |_| Ok(()),
        )?;
        Ok(NormalizedValue::OwnedProduct(product))
    }
    fn index(&self, origin: ValueOrigin, index: i64) -> Result<usize, ExecutionError> {
        usize::try_from(index)
            .ok()
            .filter(|index| self.storage.len(origin).is_ok_and(|length| *index < length))
            .ok_or_else(|| {
                ExecutionError::new(
                    ExecutionFailureClass::Trap,
                    "normalized_sequence_index",
                    "owned sequence index is outside its elements",
                )
            })
    }
    pub(super) fn inspect_transfer<R>(
        &self,
        source: ValueOrigin,
        inspect: impl FnOnce(&[NormalizedValue]) -> Result<R, ExecutionError>,
    ) -> Result<R, ExecutionError> {
        self.storage.inspect_transfer(source, inspect)
    }
    pub(super) fn adopt_transfer(
        &mut self,
        source: ValueOrigin,
        destination: ValueOrigin,
        adopt: impl FnOnce(&mut [NormalizedValue]) -> Result<(), ExecutionError>,
    ) -> Result<(), ExecutionError> {
        self.storage.adopt_transfer(source, destination, adopt)
    }
}

#[cfg(test)]
#[path = "owned_sequence_data_tests.rs"]
mod data_tests;

#[cfg(test)]
mod tests {
    use super::super::owned_i64_cell::{OwnedI64Cell, StorageObservation as Cells};
    use super::super::owned_product::StorageObservation as Composites;
    use super::*;

    fn types() -> [TypeObjectDigest; 3] {
        [
            TypeObjectDigest::from_bytes([151; 32]),
            TypeObjectDigest::from_bytes([152; 32]),
            TypeObjectDigest::from_bytes([153; 32]),
        ]
    }
    fn cell(origin: ValueOrigin, number: i64) -> NormalizedValue {
        NormalizedValue::OwnedI64Cell(OwnedI64Cell::new(origin, number))
    }
    fn remove(
        sequence: OwnedSequence,
        origin: ValueOrigin,
        control: &ExecutionControl,
    ) -> (OwnedSequence, Option<OwnedI64Cell>) {
        let [_, result, item] = types();
        let NormalizedValue::OwnedChoice(choice) = sequence
            .pop(origin, result, item, control, &mut |_| Ok(()))
            .unwrap()
        else {
            panic!("pop result")
        };
        let (case, payload) = choice.select(origin, result, control).unwrap();
        if case == 0 {
            let NormalizedValue::OwnedSequence(sequence) = payload else {
                panic!("empty sequence")
            };
            return (sequence, None);
        }
        let NormalizedValue::OwnedProduct(product) = payload else {
            panic!("item product")
        };
        let mut fields = product.unpack(origin, item, control).unwrap().into_iter();
        let NormalizedValue::OwnedSequence(sequence) = fields.next().unwrap() else {
            panic!("rest")
        };
        let NormalizedValue::OwnedI64Cell(value) = fields.next().unwrap() else {
            panic!("value")
        };
        assert!(fields.next().is_none());
        (sequence, Some(value))
    }

    #[test]
    fn sequence_pop_preserves_parent_proof_without_certifying_raw_wrappers() {
        let origin = ValueOrigin::fresh().unwrap();
        let program = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let [sequence_type, result_type, item_type] = types();
        for admitted in [false, true] {
            for empty in [false, true] {
                let mut sequence =
                    OwnedSequence::create(origin, sequence_type, &control, &mut |_| Ok(()))
                        .unwrap();
                if !empty {
                    sequence = sequence
                        .push(origin, cell(origin, 19), &control, &mut |_| Ok(()))
                        .unwrap();
                }
                if admitted {
                    sequence.establish_admission(program).unwrap();
                }
                let expected = admitted.then_some(program);
                assert_eq!(sequence.admitted_program(), expected);
                assert_eq!(sequence.clone().admitted_program(), None);
                let NormalizedValue::OwnedChoice(choice) = sequence
                    .pop(origin, result_type, item_type, &control, &mut |_| Ok(()))
                    .unwrap()
                else {
                    panic!("pop choice")
                };
                assert_eq!(choice.storage.admitted_program(), None);
                assert!(choice.validate_admission(program).is_err());
                let (case, payload) = choice.select(origin, result_type, &control).unwrap();
                assert_eq!(case, u32::from(!empty));
                let rest = if empty {
                    let NormalizedValue::OwnedSequence(rest) = payload else {
                        panic!("empty sequence")
                    };
                    rest
                } else {
                    let NormalizedValue::OwnedProduct(product) = payload else {
                        panic!("generated item product")
                    };
                    assert_eq!(product.admitted_program(), None);
                    let mut fields = product
                        .unpack(origin, item_type, &control)
                        .unwrap()
                        .into_iter();
                    let NormalizedValue::OwnedSequence(rest) = fields.next().unwrap() else {
                        panic!("remaining sequence")
                    };
                    let NormalizedValue::OwnedI64Cell(value) = fields.next().unwrap() else {
                        panic!("removed value")
                    };
                    assert_eq!(value.extract().unwrap(), 19);
                    assert!(fields.next().is_none());
                    rest
                };
                assert_eq!(rest.admitted_program(), expected);
                assert_eq!(rest.len(origin, &control).unwrap(), 0);
            }
        }
    }

    #[test]
    fn raw_sequence_pop_cannot_certify_arbitrary_wrapper_type_digests() {
        let origin = ValueOrigin::fresh().unwrap();
        let program = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let unrelated_choice_type = TypeObjectDigest::from_bytes([211; 32]);
        let unrelated_product_type = TypeObjectDigest::from_bytes([212; 32]);
        let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
            .unwrap()
            .push(origin, cell(origin, 37), &control, &mut |_| Ok(()))
            .unwrap();
        sequence.establish_admission(program).unwrap();
        let NormalizedValue::OwnedChoice(choice) = sequence
            .pop(
                origin,
                unrelated_choice_type,
                unrelated_product_type,
                &control,
                &mut |_| Ok(()),
            )
            .unwrap()
        else {
            panic!("raw choice")
        };
        assert_eq!(choice.ty(), unrelated_choice_type);
        assert!(choice.validate_admission(program).is_err());
        let (case, payload) = choice
            .select(origin, unrelated_choice_type, &control)
            .unwrap();
        assert_eq!(case, 1);
        let NormalizedValue::OwnedProduct(product) = payload else {
            panic!("raw product")
        };
        assert_eq!(product.ty(), unrelated_product_type);
        assert!(product.validate_admission(program).is_err());
        let fields = product
            .unpack(origin, unrelated_product_type, &control)
            .unwrap();
        let NormalizedValue::OwnedSequence(rest) = &fields[0] else {
            panic!("remaining sequence")
        };
        rest.validate_admission(program).unwrap();
    }

    #[test]
    fn owned_sequence_reads_lifo_and_empty_reuse_preserve_allocations() {
        let composites = Composites::start();
        let cells = Cells::start();
        let origin = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let mut sequence =
            OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(())).unwrap();
        let mut identities = Vec::new();
        let mut growth = Vec::new();
        for number in [2, 3, 7] {
            let value = OwnedI64Cell::new(origin, number);
            identities.push(value.allocation_identity());
            sequence = sequence
                .push(
                    origin,
                    NormalizedValue::OwnedI64Cell(value),
                    &control,
                    &mut |bytes| {
                        growth.push(bytes);
                        Ok(())
                    },
                )
                .unwrap();
        }
        assert_eq!(growth.len(), 1);
        assert_eq!(growth[0], 8 * std::mem::size_of::<NormalizedValue>() as u64);
        let inert = NormalizedValue::OwnedSequence(sequence.clone());
        assert_eq!(
            super::super::vm::normalized_equal(&inert, &inert)
                .unwrap_err()
                .class,
            ExecutionFailureClass::Trap,
        );
        let error = super::super::reference::reference_equal(&inert, &inert).unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Trap);
        assert_eq!(error.code, "normalized_reference_value_not_comparable");
        drop(inert);
        let vector = sequence.storage.vector_identity();
        let capacity = sequence.storage.vector_capacity();
        let parent = sequence.borrow().unwrap();
        let mut total = 0;
        for (index, identity) in identities.into_iter().enumerate() {
            let NormalizedValue::OwnedI64Cell(child) = parent
                .borrow_item(origin, index as i64, &control, &mut |_| {
                    panic!("owned child loan allocates no vector")
                })
                .unwrap()
            else {
                panic!("cell view")
            };
            assert_eq!(child.allocation_identity(), identity);
            total += child.read().unwrap();
            assert!(child.validate(origin, true).is_err());
            assert!(sequence.validate(origin, true).is_err());
            assert_eq!(parent.len(origin, &control).unwrap(), 3);
            drop(child);
        }
        assert_eq!(total, 12);
        for index in [-1, 3, i64::MAX] {
            let error = parent
                .borrow_item(origin, index, &control, &mut |_| Ok(()))
                .unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Trap);
            assert_eq!(error.code, "normalized_sequence_index");
        }
        drop(parent);
        let (rest, value) = remove(sequence, origin, &control);
        assert_eq!(value.unwrap().extract().unwrap(), 7);
        sequence = rest
            .push(origin, cell(origin, 11), &control, &mut |_| {
                panic!("existing capacity must be reused")
            })
            .unwrap();
        for number in [11, 3, 2] {
            let (rest, value) = remove(sequence, origin, &control);
            assert_eq!(value.unwrap().extract().unwrap(), number);
            sequence = rest;
            assert_eq!(sequence.storage.vector_identity(), vector);
            assert_eq!(sequence.storage.vector_capacity(), capacity);
        }
        let (rest, value) = remove(sequence, origin, &control);
        assert!(value.is_none());
        sequence = rest
            .push(origin, cell(origin, 19), &control, &mut |_| {
                panic!("empty sequence keeps its allocation")
            })
            .unwrap();
        assert_eq!(sequence.storage.vector_identity(), vector);
        assert_eq!(sequence.storage.vector_capacity(), capacity);
        drop(sequence);
        composites.assert_owners_released_after_loans();
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }

    #[test]
    fn owned_sequence_reservation_refusal_and_cancellation_release_all_custody() {
        let composites = Composites::start();
        let cells = Cells::start();
        let origin = ValueOrigin::fresh().unwrap();
        let cancelled = ExecutionControl::uncancelled();
        cancelled.cancel();
        assert_eq!(
            OwnedSequence::create(origin, types()[0], &cancelled, &mut |_| panic!(
                "pre-cancelled creation cannot reserve"
            ))
            .unwrap_err()
            .class,
            ExecutionFailureClass::Cancelled
        );
        let control = ExecutionControl::uncancelled();
        assert!(
            OwnedSequence::create(origin, types()[0], &control, &mut |_| Err(
                ExecutionError::resource("quota", "refused")
            ))
            .is_err()
        );
        assert_eq!(composites.created(), 0);
        for cancel in [false, true] {
            let control = ExecutionControl::uncancelled();
            let sequence =
                OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(())).unwrap();
            let error = sequence
                .push(origin, cell(origin, 37), &control, &mut |_| {
                    if cancel {
                        control.cancel();
                        Ok(())
                    } else {
                        Err(ExecutionError::resource("quota", "refused"))
                    }
                })
                .unwrap_err();
            assert_eq!(
                error.class,
                if cancel {
                    ExecutionFailureClass::Cancelled
                } else {
                    ExecutionFailureClass::Resource
                }
            );
            assert_eq!(composites.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
            let control = ExecutionControl::uncancelled();
            let sequence = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
                .unwrap()
                .push(origin, cell(origin, 41), &control, &mut |_| Ok(()))
                .unwrap();
            let created = composites.created();
            let error = sequence
                .pop(origin, types()[1], types()[2], &control, &mut |_| {
                    if cancel {
                        control.cancel();
                        Ok(())
                    } else {
                        Err(ExecutionError::resource("quota", "refused"))
                    }
                })
                .unwrap_err();
            assert_eq!(
                error.class,
                if cancel {
                    ExecutionFailureClass::Cancelled
                } else {
                    ExecutionFailureClass::Resource
                }
            );
            assert_eq!(composites.created(), created);
            assert_eq!(composites.live(), (0, 0));
            assert_eq!(cells.live(), (0, 0));
        }
        let healthy = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
            .unwrap()
            .push(origin, cell(origin, 43), &control, &mut |_| Ok(()))
            .unwrap();
        let (rest, value) = remove(healthy, origin, &control);
        assert_eq!(value.unwrap().extract().unwrap(), 43);
        drop(rest);
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }

    #[test]
    fn owned_sequence_cleanup_walks_wide_nested_composites_without_recursing() {
        let composites = Composites::start();
        let cells = Cells::start();
        let origin = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let mut outer =
            OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(())).unwrap();
        for row in 0..128 {
            let mut inner =
                OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(())).unwrap();
            for column in 0..64 {
                inner = inner
                    .push(
                        origin,
                        cell(origin, row * 64 + column),
                        &control,
                        &mut |_| Ok(()),
                    )
                    .unwrap();
            }
            let choice = OwnedChoice::create(
                origin,
                types()[1],
                0,
                NormalizedValue::OwnedSequence(inner),
                &control,
                &mut |_| Ok(()),
            )
            .unwrap();
            outer = outer
                .push(
                    origin,
                    NormalizedValue::OwnedChoice(choice),
                    &control,
                    &mut |_| Ok(()),
                )
                .unwrap();
        }
        assert_eq!(cells.live(), (8192, 0));
        drop(outer);
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }

    #[test]
    fn owned_sequence_pop_cancellation_sweep_releases_partial_result_envelopes() {
        let composites = Composites::start();
        let cells = Cells::start();
        let origin = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let mut interrupted_product = false;
        let mut completed = false;
        for empty in [true, false] {
            for checks in 0..16 {
                let mut sequence =
                    OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(())).unwrap();
                if !empty {
                    sequence = sequence
                        .push(origin, cell(origin, 59), &control, &mut |_| Ok(()))
                        .unwrap();
                }
                let before = composites.created();
                let cancelled = ExecutionControl::cancel_after_checks(checks);
                match sequence.pop(origin, types()[1], types()[2], &cancelled, &mut |_| Ok(())) {
                    Err(error) => {
                        assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                        interrupted_product |= !empty && composites.created() == before + 1;
                    }
                    Ok(value) => {
                        completed = true;
                        drop(value);
                    }
                }
                assert_eq!(composites.live(), (0, 0), "empty={empty} checks={checks}");
                assert_eq!(cells.live(), (0, 0), "empty={empty} checks={checks}");
            }
        }
        assert!(
            interrupted_product,
            "sweep must cancel after item product creation but before choice construction"
        );
        assert!(completed, "sweep must include healthy completion");
        let healthy = OwnedSequence::create(origin, types()[0], &control, &mut |_| Ok(()))
            .unwrap()
            .push(origin, cell(origin, 61), &control, &mut |_| Ok(()))
            .unwrap();
        let (rest, value) = remove(healthy, origin, &control);
        assert_eq!(value.unwrap().extract().unwrap(), 61);
        drop(rest);
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
    }

    #[test]
    fn owned_sequence_mixed_maximum_admitted_depth_releases_surviving_markers() {
        use crate::platform::kernel::{
            OwnerRecord, StructuralTypeField, TypeForm, TypeObject, contract, encode_type_object,
        };
        let composites = Composites::start();
        let cells = Cells::start();
        let origin = ValueOrigin::fresh().unwrap();
        let control = ExecutionControl::uncancelled();
        let mut source = super::super::tests::byte_buffer_tests::author_only(
            r#"declarations.begin
(units (module create depth
  (function create discard (visibility public) (effect pure)
    (parameter create input (type OwnedI64Cell) (use consume))
    (returns Unit) (body (unit)))))
declarations.end"#,
        )
        .unwrap();
        let mut ty = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
            .unwrap()
            .0;
        let leaf = OwnedI64Cell::new(origin, 173);
        let leaf_marker = leaf.clone();
        let mut value = NormalizedValue::OwnedI64Cell(leaf);
        let mut markers = Vec::new();
        for depth in 0..contract::MAXIMUM_TYPE_DEPTH {
            let field = StructuralTypeField {
                name: crate::platform::kernel::Name::new("child").unwrap(),
                ty,
            };
            let object = TypeObject::new(match depth % 3 {
                0 => TypeForm::OwnedSequence { item: ty },
                1 => TypeForm::OwnedProduct {
                    fields: vec![field],
                },
                _ => TypeForm::OwnedChoice { cases: vec![field] },
            })
            .unwrap();
            let next = encode_type_object(&object).unwrap().0;
            source.types.insert(next, object);
            value = match depth % 3 {
                0 => NormalizedValue::OwnedSequence(
                    OwnedSequence::create(origin, next, &control, &mut |_| Ok(()))
                        .unwrap()
                        .push(origin, value, &control, &mut |_| Ok(()))
                        .unwrap(),
                ),
                1 => NormalizedValue::OwnedProduct(
                    OwnedProduct::create(origin, next, vec![value], &control, &mut |_| Ok(()))
                        .unwrap(),
                ),
                _ => NormalizedValue::OwnedChoice(
                    OwnedChoice::create(origin, next, 0, value, &control, &mut |_| Ok(())).unwrap(),
                ),
            };
            markers.push(value.clone());
            ty = next;
        }
        for owner in source.owners.values_mut() {
            if let OwnerRecord::Parameter(parameter) = owner {
                parameter.ty = ty;
            }
        }
        crate::platform::kernel::validate_full(&source).unwrap();
        assert!(crate::platform::kernel::memory_reference::accepts(&source));
        assert_eq!(composites.live(), (contract::MAXIMUM_TYPE_DEPTH, 0));
        drop(value);
        assert_eq!(composites.live(), (0, 0));
        assert_eq!(cells.live(), (0, 0));
        assert!(leaf_marker.validate(origin, false).is_err());
        assert!(
            markers
                .iter()
                .all(|marker| marker.memory_validate(origin, false).is_err())
        );
        drop(markers);
        assert_eq!(composites.live(), (0, 0));
    }
}
