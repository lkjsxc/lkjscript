//! Exact evidence, adversarial transforms and independent raw boundary controls.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::metadata_tests::fixture::{Fixture, token};
use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::kernel::{
    DeclarationReference, FieldReference, ReadScopeArgument, TypeObject, encode_type_object,
};
use crate::platform::semantic_id::{BindingId, DeclarationId, FieldId, TypeParameterId};

struct Outcome<T> {
    result: Result<T, ExecutionError>,
    work: ValueWork,
    bytes: u64,
    items: u64,
}

fn with_admission<T>(
    f: &Fixture,
    substitutions: &Bindings,
    policy: super::super::NormalizedRunPolicy,
    control: &ExecutionControl,
    run: impl FnOnce(&mut Admission<'_>) -> Result<T, ExecutionError>,
) -> Outcome<T> {
    let resources = NormalizedResourceScope::new().unwrap();
    let mut work = ValueWork::default();
    let mut bytes = 0;
    let mut charges = 0;
    let mut items = 0;
    let mut allocated_saturated = false;
    let mut items_saturated = false;
    let result = run(&mut Admission {
        shared_budget: None,
        program: &f.program,
        substitutions,
        resources: &resources,
        control,
        policy,
        work: &mut work,
        allocated: &mut bytes,
        allocated_saturated: &mut allocated_saturated,
        allocation_charges: &mut charges,
        items: &mut items,
        items_saturated: &mut items_saturated,
        admission_bytes: 0,
        admission_bytes_saturated: false,
    });
    assert_eq!(resources.live_resources(), 0);
    Outcome {
        result,
        work,
        bytes,
        items,
    }
}

fn admit(
    f: &Fixture,
    raw: NormalizedValue,
    ty: TypeObjectDigest,
    bindings: &Bindings,
    input: bool,
) -> Outcome<Value> {
    with_admission(
        f,
        bindings,
        super::super::NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
        |admission| admission.value(raw, ty, None, input),
    )
}

fn add(f: &mut Fixture, form: TypeForm, ordinary: bool) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    f.program.types.insert(ty, object);
    if ordinary {
        f.program.ordinary_types.insert(ty);
    }
    ty
}

fn exact(f: &Fixture, value: &Value, ty: TypeObjectDigest) -> bool {
    value
        .has_exact_ordinary_type(&f.program, ty, &mut ValueWork::default())
        .unwrap()
}

#[test]
fn raw_admission_pair_refusal_preserves_subtotals_counts_and_saturation() {
    let fixture = Fixture::new(0);
    let outcome = with_admission(
        &fixture,
        &Bindings::new(),
        super::super::NormalizedRunPolicy {
            maximum_allocated_bytes: Some(0),
            ..super::super::NormalizedRunPolicy::foreground()
        },
        &ExecutionControl::uncancelled(),
        |admission| {
            admission.admission_bytes = u64::MAX;
            *admission.items = u64::MAX;
            *admission.allocation_charges = 7;
            assert!(admission.reserve_storage(1, 1).is_err());
            assert_eq!(admission.admission_bytes, u64::MAX);
            assert!(!admission.admission_bytes_saturated);
            assert_eq!(*admission.items, u64::MAX);
            assert!(!*admission.items_saturated);
            assert_eq!(*admission.allocated, 0);
            assert!(!*admission.allocated_saturated);
            assert_eq!(*admission.allocation_charges, 7);

            admission.policy.maximum_allocated_bytes = None;
            admission.reserve_storage(1, 1)?;
            assert!(admission.admission_bytes_saturated);
            assert!(*admission.items_saturated);
            assert_eq!(*admission.allocated, 1);
            assert!(!*admission.allocated_saturated);
            assert_eq!(*admission.allocation_charges, 8);
            Ok(())
        },
    );
    outcome.result.unwrap();
}

#[test]
fn ordinary_evidence_keeps_exact_admission_and_immutable_aliases_without_raw_boundary_reuse() {
    let f = Fixture::new(8);
    for count in [0, 8, 1024, 65536] {
        let admitted = admit(&f, f.raw(count), f.payload, &Bindings::new(), true);
        assert_eq!(admitted.work.input_admission_nodes, count as u64 + 9);
        assert_eq!(admitted.work.raw_result_admission_nodes, 0);
        let value = admitted.result.unwrap();
        assert_eq!(value.ordinary_type, Some(f.payload));
        assert!(exact(&f, &value, f.payload));
        let copy = value.duplicate(ParameterUse::Unrestricted).unwrap();
        assert!(exact(&f, &copy, f.payload));
        force_strict_ordinary_children(|| assert!(!exact(&f, &copy, f.payload)));
        assert!(exact(&f, &copy, f.payload));
        drop(value);
        assert_eq!(copy.raw(), &f.raw(count));
        // A raw host output has no proof packet, even when it shares immutable
        // backing with a previously checked value. Every descendant is admitted.
        let returned = admit(&f, copy.into_raw(), f.payload, &Bindings::new(), false);
        assert_eq!(returned.work.raw_result_admission_nodes, count as u64 + 9);
        assert!(exact(&f, &returned.result.unwrap(), f.payload));
    }
    let mut malformed =
        NormalizedValue::list(vec![NormalizedValue::I64(1), NormalizedValue::Bool(false)]).unwrap();
    for _ in 0..8 {
        malformed = NormalizedValue::Option(Some(Box::new(malformed)));
    }
    let rejected = admit(&f, malformed, f.payload, &Bindings::new(), true);
    assert!(rejected.result.is_err());
    assert_eq!(rejected.work.input_admission_nodes, 11);
}

#[test]
fn ordinary_evidence_uses_resolved_generic_identity_and_rejects_other_preparations() {
    let mut f = Fixture::new(0);
    let parameter = TypeParameterId::migrate(b"checked-ordinary-generic", 1);
    let variable = add(&mut f, TypeForm::TypeParameter { parameter }, false);
    let template = add(&mut f, TypeForm::List { item: variable }, false);
    let integer = add(&mut f, TypeForm::I64, true);
    let boolean = add(&mut f, TypeForm::Bool, true);
    let other = add(&mut f, TypeForm::List { item: boolean }, true);
    let bindings = Bindings::from([(parameter, integer)]);
    let value = admit(&f, f.raw(3), template, &bindings, true)
        .result
        .unwrap();
    assert_eq!(value.ordinary_type, Some(f.payload));
    assert!(exact(&f, &value, f.payload));
    assert!(!exact(&f, &value, template));
    assert!(!exact(&f, &value, other));
    assert!(
        admit(
            &f,
            value.raw().clone(),
            template,
            &Bindings::from([(parameter, boolean)]),
            false,
        )
        .result
        .is_err()
    );
    assert!(
        admit(&f, value.raw().clone(), template, &Bindings::new(), true)
            .result
            .is_err()
    );
    // Unselected generic roots may be admitted without having a retained closed
    // property root. Success without that evidence must remain valid.
    f.program.ordinary_types.remove(&f.payload);
    f.program.application_free_types.insert(template);
    let uncached = admit(&f, f.raw(3), template, &bindings, true)
        .result
        .unwrap();
    assert_eq!(uncached.ordinary_type, None);
    assert!(!exact(&f, &value, f.payload));
    f.program.ordinary_types.insert(f.payload);
    let foreign = Fixture::new(0);
    assert_eq!(foreign.payload, f.payload);
    assert!(
        value
            .has_exact_ordinary_type(&foreign.program, f.payload, &mut ValueWork::default())
            .is_err()
    );
}

#[test]
fn ordinary_evidence_is_invalidated_by_untyped_list_map_and_callable_transforms() {
    let mut f = Fixture::new(0);
    let list = f.admit(f.raw(3)).unwrap();
    let alias = list.duplicate(ParameterUse::Unrestricted).unwrap();
    let changed = list
        .append(
            &f.program,
            Value::scalar(&f.program, NormalizedValue::Bool(false)).unwrap(),
            &mut ValueWork::default(),
            None,
            &mut |_| Ok(()),
        )
        .unwrap();
    assert_eq!(changed.ordinary_type, None);
    assert!(
        admit(&f, changed.into_raw(), f.payload, &Bindings::new(), false)
            .result
            .is_err()
    );
    assert!(exact(&f, &alias, f.payload));
    assert_eq!(alias.raw(), &f.raw(3));

    let integer = add(&mut f, TypeForm::I64, true);
    let payload_ty = f.payload;
    let map_ty = add(
        &mut f,
        TypeForm::Map {
            key: integer,
            value: payload_ty,
        },
        true,
    );
    for bad_key in [false, true] {
        let map = admit(
            &f,
            NormalizedValue::Map(Default::default()),
            map_ty,
            &Bindings::new(),
            true,
        )
        .result
        .unwrap();
        let retained = map.duplicate(ParameterUse::Unrestricted).unwrap();
        let changed = map
            .edit_map(
                &f.program,
                if bad_key {
                    NormalizedMapKey::Bool(true)
                } else {
                    NormalizedMapKey::I64(7)
                },
                Some(if bad_key {
                    f.admit(f.raw(2)).unwrap()
                } else {
                    Value::scalar(&f.program, NormalizedValue::Bool(false)).unwrap()
                }),
                &mut ValueWork::default(),
                &mut |_| Ok(()),
            )
            .unwrap();
        assert_eq!(changed.ordinary_type, None);
        assert!(
            admit(&f, changed.into_raw(), map_ty, &Bindings::new(), false)
                .result
                .is_err()
        );
        assert!(exact(&f, &retained, map_ty));
        assert_eq!(retained.raw(), &NormalizedValue::Map(Default::default()));
    }

    let function = f
        .program
        .functions
        .iter()
        .enumerate()
        .find_map(|(index, target)| {
            (target.type_parameters.is_empty()
                && target.parameters.len() == 2
                && matches!(&target.body,
                    super::super::super::prepare::NormalizedFunctionBody::External(name)
                    if name.as_str() == "core.i64.add"))
            .then_some(FunctionIndex(index as u32, f.program.value_origin))
        })
        .expect("fixture integer addition");
    let function_ty = add(
        &mut f,
        TypeForm::Function {
            parameters: vec![integer, integer],
            result: integer,
        },
        true,
    );
    let callee = Value::function(
        &f.program,
        function,
        Arc::from([]),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    let callee = admit(&f, callee.into_raw(), function_ty, &Bindings::new(), true)
        .result
        .unwrap();
    assert!(exact(&f, &callee, function_ty));
    let bound = with_admission(
        &f,
        &Bindings::new(),
        super::super::NormalizedRunPolicy::foreground(),
        &ExecutionControl::uncancelled(),
        |admission| {
            let callee = admission.begin_bind(callee, 1)?;
            assert_eq!(callee.ordinary_type, None);
            let capture = admission.capture(
                &callee,
                Value::scalar(&f.program, NormalizedValue::I64(7))?,
                0,
            )?;
            assert_eq!(capture.ordinary_type, None);
            admission.bind(callee, vec![capture])
        },
    )
    .result
    .unwrap();
    assert_eq!(bound.ordinary_type, None);
    assert!(
        admit(&f, bound.into_raw(), function_ty, &Bindings::new(), false)
            .result
            .is_err()
    );
}

#[test]
fn ordinary_evidence_distinguishes_same_shaped_nominal_layouts() {
    use super::super::super::prepare::{NormalizedRecordField, NormalizedRecordLayout};

    let mut f = Fixture::new(0);
    let mut types = Vec::new();
    for ordinal in 0..2 {
        let declaration = DeclarationReference {
            package: f.program.root_package,
            declaration: DeclarationId::migrate(b"ordinary-proof-nominal", ordinal),
        };
        let ty = add(&mut f, TypeForm::Named { declaration }, true);
        let layout = RecordLayoutIndex(f.program.records.len() as u32, f.program.value_origin);
        let mut records = f.program.records.to_vec();
        records.push(NormalizedRecordLayout {
            declaration,
            owned: false,
            type_parameters: Arc::from([]),
            type_parameter_constraints: Arc::from([]),
            arguments: Arc::from([]),
            fields: Arc::from([NormalizedRecordField {
                reference: FieldReference {
                    package: declaration.package,
                    field: FieldId::migrate(b"ordinary-proof-nominal", ordinal),
                },
                name: Name::new("payload").unwrap(),
                ty: f.payload,
            }]),
        });
        f.program.records = records.into();
        f.program.record_instances.insert(ty, layout);
        types.push((ty, layout));
    }
    let value = admit(
        &f,
        NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: types[0].1,
            fields: Arc::new(vec![f.raw(8)]),
        }),
        types[0].0,
        &Bindings::new(),
        true,
    )
    .result
    .unwrap();
    assert!(exact(&f, &value, types[0].0));
    assert!(!exact(&f, &value, types[1].0));
    assert!(
        admit(&f, value.into_raw(), types[1].0, &Bindings::new(), false)
            .result
            .is_err()
    );
}

#[test]
fn ordinary_evidence_is_minted_only_for_actual_parent_children_and_keeps_custody_separate() {
    use super::super::super::owned_i64_cell::StorageObservation as Cells;
    use super::super::super::owned_product::StorageObservation as Products;
    use super::super::super::owned_sequence::OwnedSequence;

    let mut f = Fixture::new(8);
    let products = Products::start();
    let cells = Cells::start();
    let owner = f.owner(32);
    let read = f.read(&owner);
    let projected = f.project(&read).unwrap();
    assert!(exact(&f, &projected, f.payload));
    drop(read);
    token(&owner).validate(f.domain, true).unwrap();
    let mut extracted = owner
        .unpack_owned(
            &f.program,
            f.domain,
            f.product,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    let child = extracted
        .next(&f.program, &ExecutionControl::uncancelled(), &mut |_, _| {
            panic!("ordinary extraction keeps the exact parent proof")
        })
        .unwrap()
        .unwrap();
    assert!(exact(&f, &child, f.payload));
    drop(extracted);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(child.raw(), &f.raw(32));

    // Projection to an ordinary Option child must not accidentally retain the
    // parent's aggregate type certificate.
    let smaller = child.option_get().unwrap().unwrap();
    assert_eq!(smaller.ordinary_type, None);
    assert!(!exact(&f, &smaller, f.payload));

    let payload_ty = f.payload;
    let sequence_ty = add(&mut f, TypeForm::OwnedSequence { item: payload_ty }, false);
    f.program.comparable_types.insert(payload_ty);
    let control = ExecutionControl::uncancelled();
    let sequence = OwnedSequence::create(f.domain, sequence_ty, &control, &mut |_| Ok(()))
        .unwrap()
        .push(f.domain, projected.into_raw(), &control, &mut |_| Ok(()))
        .unwrap();
    sequence
        .establish_admission(f.program.value_origin)
        .unwrap();
    let owner = Value::memory(&f.program, NormalizedValue::OwnedSequence(sequence)).unwrap();
    let read = owner.duplicate(ParameterUse::Borrow).unwrap();
    let independent = read
        .sequence_item(&f.program, f.domain, 0, &control, &mut |_| Ok(()))
        .unwrap();
    assert!(exact(&f, &independent, f.payload));
    drop(read);
    drop(owner);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(independent.raw(), &f.raw(32));

    let owner = f.owner(2);
    let read = f.read(&owner);
    let mut guarded = f.admit(f.raw(2)).unwrap();
    let mut metadata = BorrowMetadata::default();
    metadata.guards.push(read);
    guarded.borrow = Some(Box::new(metadata));
    assert!(!exact(&f, &guarded, f.payload));
    assert!(guarded.validate_raw_consumption().is_err());
    assert!(guarded.duplicate(ParameterUse::Unrestricted).is_err());
    assert!(token(&owner).validate(f.domain, true).is_err());
    drop(guarded);
    token(&owner).validate(f.domain, true).unwrap();
    drop(owner);
    assert_eq!(products.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));

    for packet in [false, true] {
        let mut guarded = f.admit(f.raw(2)).unwrap();
        let mut metadata = BorrowMetadata::default();
        metadata.packet = packet.then_some(BorrowResultTarget {
            source: BorrowSource { frame: 1, local: 0 },
            position: 0,
        });
        metadata.view_borrowed = !packet;
        guarded.borrow = Some(Box::new(metadata));
        assert!(!exact(&f, &guarded, f.payload));
        assert!(guarded.duplicate(ParameterUse::Unrestricted).is_err());
    }

    for class in [
        Class::Memory,
        Class::ReadView,
        Class::Direct,
        Class::Variant,
        Class::Binding(0),
        Class::Retained(0),
    ] {
        let mut stale = f.admit(f.raw(2)).unwrap();
        stale.class = class;
        assert!(!exact(&f, &stale, f.payload));
    }
}

#[test]
fn ordinary_evidence_admission_reserves_before_minting_and_rejects_phantom_read_views() {
    let mut f = Fixture::new(8);
    let bindings = Bindings::new();
    let control = ExecutionControl::uncancelled();
    let baseline = admit(&f, f.raw(64), f.payload, &bindings, true);
    assert!(baseline.result.is_ok());
    for (maximum, accepted) in [(baseline.bytes, true), (baseline.bytes - 1, false)] {
        let result = with_admission(
            &f,
            &bindings,
            super::super::NormalizedRunPolicy {
                maximum_allocated_bytes: Some(maximum),
                ..super::super::NormalizedRunPolicy::foreground()
            },
            &control,
            |admission| admission.value(f.raw(64), f.payload, None, true),
        );
        assert_eq!(result.result.is_ok(), accepted);
        assert!(result.bytes <= maximum);
        match result.result {
            Ok(value) => assert!(exact(&f, &value, f.payload)),
            Err(error) => assert_eq!(error.class, ExecutionFailureClass::Resource),
        }
    }
    for (maximum, accepted) in [(baseline.items, true), (baseline.items - 1, false)] {
        let result = with_admission(
            &f,
            &bindings,
            super::super::NormalizedRunPolicy {
                maximum_collection_items: Some(maximum),
                ..super::super::NormalizedRunPolicy::foreground()
            },
            &control,
            |admission| admission.value(f.raw(64), f.payload, None, true),
        );
        assert_eq!(result.result.is_ok(), accepted);
        assert!(result.items <= maximum);
    }
    let cancelled = with_admission(
        &f,
        &bindings,
        super::super::NormalizedRunPolicy::foreground(),
        &ExecutionControl::cancel_after_input_nodes(10),
        |admission| admission.value(f.raw(64), f.payload, None, true),
    );
    assert_eq!(
        cancelled.result.unwrap_err().class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(cancelled.work.input_admission_nodes, 10);

    let declaration: DeclarationReference = f.program.functions[0].declaration;
    let view = add(
        &mut f,
        TypeForm::ReadViewApplied {
            declaration,
            arguments: vec![],
            read_scope: ReadScopeArgument::Lexical {
                callable: declaration,
                binding: BindingId::migrate(b"ordinary-proof-no-view", 1),
            },
        },
        false,
    );
    let phantom = add(&mut f, TypeForm::List { item: view }, false);
    let rejected = admit(
        &f,
        NormalizedValue::list(vec![]).unwrap(),
        phantom,
        &bindings,
        true,
    );
    assert!(rejected.result.is_err());
    assert_eq!(rejected.work.input_admission_nodes, 0);
    assert!(
        admit(&f, f.raw(64), f.payload, &bindings, true)
            .result
            .is_ok()
    );
}
