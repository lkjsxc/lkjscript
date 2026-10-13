//! Adversarial raw boundary controls. Forged layouts are confined to these tests;
//! they do not stand in for canonical or evaluator admission of a read view.

use super::super::owned_storage::OwnedStorage;
use super::super::read_view_storage::{
    AdmissionRequest, HeldRead, ReadView, ReadViewAdmission, RuntimeScopeOwner, SourceAnchor,
    SourceCoordinate, SourceLoans, ViewBuilder, ViewConstruction, ViewIdentity,
};
use super::super::value::{FunctionIndex, NormalizedRecord, RecordLayoutIndex, ValueOrigin};
use super::super::value_schema::NormalizedValueSchema;
use super::super::{codec, data_codec, data_codec_reference};
use super::*;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{PackageId, ReadScopeArgument, ReadScopeBinder, ReadScopeReference};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, ReadScopeParameterId,
};
use std::cell::Cell;

fn scope() -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration()),
        parameter: ReadScopeParameterId::migrate(b"codec-view-scope", 0),
    })
}

fn declaration() -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(b"codec-view-declaration", 0),
        declaration: DeclarationId::migrate(b"codec-view-declaration", 0),
    }
}

fn fixture() -> (NormalizedProgram, TypeObjectDigest, TypeObjectDigest) {
    let snapshot = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create boundaries
  (record create Phantom (visibility private)
    (type-parameter create T) (field create tag (type I64)))
  (variant create Choice (visibility private)
    (case create safe) (case create hidden (payload I64)))
  (function create accept (visibility private) (effect pure)
    (parameter create phantom (type (Phantom I64)))
    (parameter create choice (type Choice))
    (returns Unit) (body (unit)))))
declarations.end"#,
    )
    .expect("ordinary native fixture");
    let mut program = prepare_snapshot(&snapshot);
    let unit = admit_runtime_type(&mut program, TypeForm::Unit);
    let view = admit_runtime_type(
        &mut program,
        TypeForm::ReadViewApplied {
            declaration: declaration(),
            arguments: vec![],
            read_scope: scope(),
        },
    );
    (program, unit, view)
}

fn assert_escape(error: Diagnostic) {
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "normalized_read_view_escape");
}

fn rejects_all_codecs(program: &NormalizedProgram, value: &NormalizedValue, ty: TypeObjectDigest) {
    assert_escape(codec::encode_typed(program, value, ty, JsonLimits::default()).unwrap_err());
    assert_escape(data_codec::encode_typed(program, value, ty).unwrap_err());
    assert_escape(data_codec_reference::encode_typed(program, value, ty).unwrap_err());
    // Invalid input must not reach JSON parsing, checksums, or envelope decoding.
    assert_escape(codec::decode_typed(program, b"{broken", ty, JsonLimits::default()).unwrap_err());
    assert_escape(data_codec::decode_typed(program, b"broken", ty).unwrap_err());
    assert_escape(data_codec_reference::decode_typed(program, b"broken", ty).unwrap_err());
    assert_escape(super::super::runner::execution_diagnostic(
        super::super::session::retained_map_meter_probe(
            program,
            value,
            ty,
            1_000_000,
            4 * 1_048_576,
        )
        .unwrap_err(),
    ));
}

#[test]
fn direct_and_abstract_view_forms_refuse_before_input_or_output_work() {
    let (mut program, _, view) = fixture();
    let member = AssociatedTypeMemberId::migrate(b"codec-view-member", 0);
    let associated = admit_runtime_type(
        &mut program,
        TypeForm::AssociatedReadViewMember {
            contract: declaration(),
            member,
            read_scope: scope(),
        },
    );
    let projection = admit_runtime_type(
        &mut program,
        TypeForm::AssociatedReadViewProjection {
            scope: declaration(),
            parameter: ImplementationParameterId::migrate(b"codec-view-witness", 0),
            contract: declaration(),
            member,
            read_scope: scope(),
        },
    );
    for ty in [view, associated, projection] {
        rejects_all_codecs(&program, &NormalizedValue::Unit, ty);
    }
}

#[test]
fn phantom_actuals_and_inactive_variant_payloads_cannot_escape() {
    let (mut program, _, view) = fixture();
    let mut record = program.records[0].clone();
    let phantom = admit_runtime_type(
        &mut program,
        TypeForm::Applied {
            declaration: record.declaration,
            arguments: vec![view],
        },
    );
    // Even a supplier that erases its phantom layout actual cannot hide the form actual.
    record.arguments = Arc::from([]);
    let index = RecordLayoutIndex(program.records.len() as u32, program.value_origin);
    program.records = program
        .records
        .iter()
        .cloned()
        .chain([record])
        .collect::<Vec<_>>()
        .into();
    program.record_instances.insert(phantom, index);
    let value = NormalizedValue::Record(NormalizedRecord::Nominal {
        layout: index,
        fields: Arc::new(vec![NormalizedValue::I64(17)]),
    });
    rejects_all_codecs(&program, &value, phantom);

    let declaration = program.variants[0].declaration;
    let choice = admit_runtime_type(&mut program, TypeForm::Named { declaration });
    let layout = program.variant_instances[&choice];
    let nested = admit_runtime_type(&mut program, TypeForm::List { item: view });
    let cases = &mut Arc::make_mut(&mut program.variants)[layout.0 as usize].cases;
    let safe = cases
        .iter()
        .position(|case| case.payload.is_none())
        .unwrap() as u32;
    let hidden = cases
        .iter()
        .position(|case| case.payload.is_some())
        .unwrap();
    Arc::make_mut(cases)[hidden].payload = Some(nested);
    let value = NormalizedValue::Variant {
        layout,
        case: safe,
        payload: None,
    };
    rejects_all_codecs(&program, &value, choice);
}

#[test]
fn complete_nested_forms_reject_views_even_when_runtime_contents_are_empty() {
    let (mut program, unit, view) = fixture();
    let optional = admit_runtime_type(&mut program, TypeForm::Option { item: view });
    let list = admit_runtime_type(&mut program, TypeForm::List { item: optional });
    let result = admit_runtime_type(
        &mut program,
        TypeForm::Result {
            ok: unit,
            error: view,
        },
    );
    let owned = admit_runtime_type(&mut program, TypeForm::OwnedSequence { item: list });
    for ty in [optional, list, result, owned] {
        rejects_all_codecs(&program, &NormalizedValue::Unit, ty);
    }
}

/// A fixed zero-field mechanism adapter, used only to manufacture a hostile raw
/// carrier. Its admitted source is a real checked scalar read, not an owner clone.
struct ZeroFieldAdmission {
    identity: ViewIdentity,
    source: SourceAnchor,
}

impl ReadViewAdmission for ZeroFieldAdmission {
    fn validate(
        &self,
        request: AdmissionRequest<'_>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        let construction = |construction: &ViewConstruction| {
            construction.identity == self.identity
                && construction.source == self.source
                && construction.fields == 0
                && construction.scope.key() == scope()
        };
        let valid = match request {
            AdmissionRequest::Scope {
                key,
                source,
                parent,
            } => key == scope() && source == &self.source && parent.is_none(),
            AdmissionRequest::Source {
                construction: candidate,
                loans,
            } => construction(candidate) && loans.reads().len() == 1,
            AdmissionRequest::Finish {
                construction: candidate,
                fields,
            } => construction(candidate) && fields.is_empty(),
            _ => false,
        };
        if valid {
            Ok(())
        } else {
            Err(ExecutionError::new(
                ExecutionFailureClass::Trap,
                "test_view_admission",
                "fixed zero-field test adapter refused",
            ))
        }
    }
}

fn raw_view(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
) -> (
    super::super::owned_i64_cell::OwnedI64Cell,
    RuntimeScopeOwner,
    ReadView,
) {
    let origin = ValueOrigin::fresh().unwrap();
    let cell = super::super::owned_i64_cell::OwnedI64Cell::new(origin, 41);
    cell.establish_admission(program.value_origin).unwrap();
    let source = SourceAnchor {
        program: program.value_origin,
        application: ValueOrigin::fresh().unwrap(),
        invocation: ValueOrigin::fresh().unwrap(),
        coordinate: SourceCoordinate::Parameter(0),
        origin,
        path: Arc::from([]),
    };
    let identity = ViewIdentity {
        ty,
        program: program.value_origin,
    };
    let admission = ZeroFieldAdmission {
        identity,
        source: source.clone(),
    };
    let control = ExecutionControl::uncancelled();
    let (scope_owner, scope) = RuntimeScopeOwner::open(
        scope(),
        &source,
        None,
        &admission,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let selected = HeldRead {
        origin,
        value: NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
    };
    let loans = SourceLoans::new(selected, 0, &control, &mut |_| Ok(())).unwrap();
    let builder = ViewBuilder::from_owned(
        ViewConstruction {
            identity,
            source,
            scope,
            fields: 0,
        },
        loans,
        0,
        &admission,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let view = builder
        .finish(&admission, &control, &mut |_| Ok(()))
        .unwrap();
    (cell, scope_owner, view)
}

struct CountingSchema<'a> {
    program: &'a NormalizedProgram,
    reads: Cell<usize>,
}

impl NormalizedValueSchema for CountingSchema<'_> {
    fn value_origin(&self) -> ValueOrigin {
        self.program.value_origin
    }
    fn records(&self) -> &[super::super::prepare::NormalizedRecordLayout] {
        &self.program.records
    }
    fn variants(&self) -> &[super::super::prepare::NormalizedVariantLayout] {
        &self.program.variants
    }
    fn types(&self) -> &BTreeMap<TypeObjectDigest, TypeObject> {
        self.reads.set(self.reads.get() + 1);
        &self.program.types
    }
    fn record_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.program.record_index(ty)
    }
    fn variant_index(&self, ty: TypeObjectDigest) -> Option<usize> {
        self.program.variant_index(ty)
    }
    fn comparable(&self, ty: TypeObjectDigest) -> bool {
        self.program.comparable(ty)
    }
    fn application_free(&self, ty: TypeObjectDigest) -> bool {
        self.program.application_free(ty)
    }
}

#[test]
fn raw_nested_view_and_scoped_callable_refuse_before_schema_callbacks() {
    let (program, unit, ty) = fixture();
    let (cell, scope_owner, view) = raw_view(&program, ty);
    let source_origin = view.source().origin;
    let value = NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(vec![(
            Name::new("hidden").unwrap(),
            NormalizedValue::Option(Some(Box::new(NormalizedValue::ReadView(view)))),
        )]),
    });
    let schema = CountingSchema {
        program: &program,
        reads: Cell::new(0),
    };
    assert_escape(codec::encode_value(&schema, &value, unit, JsonLimits::default()).unwrap_err());
    assert_escape(data_codec::encode_typed(&program, &value, unit).unwrap_err());
    assert_escape(data_codec_reference::encode_typed(&schema, &value, unit).unwrap_err());
    assert_eq!(schema.reads.get(), 0);
    assert!(cell.validate(source_origin, true).is_err());
    drop(value);
    drop(scope_owner);
    cell.validate(source_origin, true).unwrap();

    let scoped = NormalizedValue::Function {
        function: FunctionIndex(0, program.value_origin),
        type_arguments: Arc::from([]),
        effect_arguments: Arc::from([]),
        requirement_arguments: Arc::from([]),
        read_scope_arguments: Arc::from([scope()]),
        bound_arguments: None,
    };
    assert_escape(codec::encode_value(&schema, &scoped, unit, JsonLimits::default()).unwrap_err());
    assert_escape(data_codec_reference::encode_typed(&schema, &scoped, unit).unwrap_err());
    assert_eq!(schema.reads.get(), 0);
}

#[test]
fn owned_storage_refuses_nested_views_before_reserve_or_mutation() {
    let (program, unit, ty) = fixture();
    let (_cell, _scope_owner, view) = raw_view(&program, ty);
    let origin = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let reservations = Cell::new(0);
    let mut reserve = |_| {
        reservations.set(reservations.get() + 1);
        Ok(())
    };
    let hostile =
        || NormalizedValue::Option(Some(Box::new(NormalizedValue::ReadView(view.clone()))));
    let error =
        OwnedStorage::create(origin, unit, vec![hostile()], &control, &mut reserve).unwrap_err();
    assert_eq!(error.code, "normalized_read_view_escape");
    assert_eq!(reservations.get(), 0);
    let owner = OwnedStorage::create(
        origin,
        unit,
        vec![NormalizedValue::I64(7)],
        &control,
        &mut reserve,
    )
    .unwrap();
    owner.establish_admission(program.value_origin).unwrap();
    let before = reservations.get();
    assert_eq!(
        owner
            .push(origin, hostile(), &control, &mut reserve)
            .unwrap_err()
            .code,
        "normalized_read_view_escape"
    );
    assert_eq!(
        owner
            .replace(origin, 0, hostile(), &control)
            .unwrap_err()
            .code,
        "normalized_read_view_escape"
    );
    assert_eq!(reservations.get(), before);
    owner.validate_admission(program.value_origin).unwrap();
    let fields = owner.unpack(origin, unit, &control).unwrap();
    assert_eq!(fields, vec![NormalizedValue::I64(7)]);
}

#[test]
fn hostile_transfer_callback_cannot_restore_admission_or_reach_later_callbacks() {
    let (program, unit, ty) = fixture();
    let (_cell, _scope_owner, view) = raw_view(&program, ty);
    let origin = ValueOrigin::fresh().unwrap();
    let destination = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut owner = OwnedStorage::create(
        origin,
        unit,
        vec![NormalizedValue::I64(7)],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    owner.establish_admission(program.value_origin).unwrap();
    let error = owner
        .adopt_transfer(origin, destination, |fields| {
            fields[0] = NormalizedValue::ReadView(view.clone());
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "normalized_read_view_escape");
    assert!(owner.validate_admission(program.value_origin).is_err());
    let callbacks = Cell::new(0);
    let error = owner
        .inspect_transfer(origin, |_| {
            callbacks.set(callbacks.get() + 1);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "normalized_read_view_escape");
    assert_eq!(callbacks.get(), 0);
    assert_eq!(
        owner.borrow().unwrap_err().code,
        "normalized_read_view_escape"
    );
    assert_eq!(
        owner
            .establish_admission(program.value_origin)
            .unwrap_err()
            .code,
        "normalized_read_view_escape"
    );
    assert_eq!(
        owner.pop(origin, &control).unwrap_err().code,
        "normalized_read_view_escape"
    );
    assert_eq!(
        owner
            .replace(origin, 0, NormalizedValue::I64(9), &control)
            .unwrap_err()
            .code,
        "normalized_read_view_escape"
    );
    // An unadmitted opaque child from a failed callback cannot hide the view
    // behind a new, initially view-free envelope or trigger its reservation.
    let error = OwnedStorage::create(
        origin,
        unit,
        vec![NormalizedValue::OwnedProduct(owner)],
        &control,
        &mut |_| {
            callbacks.set(callbacks.get() + 1);
            Ok(())
        },
    )
    .unwrap_err();
    assert_eq!(error.code, "normalized_read_view_escape");
    assert_eq!(callbacks.get(), 0);
}

#[test]
fn raw_inert_self_observation_refuses_before_recursive_storage_lock() {
    let (program, unit, _) = fixture();
    let origin = ValueOrigin::fresh().unwrap();
    let destination = ValueOrigin::fresh().unwrap();
    let control = ExecutionControl::uncancelled();
    let mut owner = OwnedStorage::create(
        origin,
        unit,
        vec![NormalizedValue::I64(7)],
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    owner.establish_admission(program.value_origin).unwrap();
    let observation = owner.clone();
    let error = owner
        .adopt_transfer(origin, destination, |fields| {
            // The callback holds owner's mutex. Each retained inert-observation
            // API must refuse before trying to acquire that same allocation lock.
            assert_eq!(
                observation.validate(origin, false).unwrap_err().code,
                "normalized_product_token"
            );
            assert_eq!(
                observation
                    .establish_admission(program.value_origin)
                    .unwrap_err()
                    .code,
                "normalized_product_token"
            );
            assert_eq!(
                observation
                    .validate_admission(program.value_origin)
                    .unwrap_err()
                    .code,
                "normalized_product_token"
            );
            assert!(observation.admitted_program().is_none());
            assert_eq!(
                observation.borrow().unwrap_err().code,
                "normalized_product_token"
            );
            assert_eq!(
                observation.pop(origin, &control).unwrap_err().code,
                "normalized_product_token"
            );
            assert_eq!(
                observation
                    .replace(origin, 0, NormalizedValue::I64(9), &control)
                    .unwrap_err()
                    .code,
                "normalized_product_token"
            );
            fields[0] = NormalizedValue::OwnedProduct(observation);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "normalized_product_token");
    assert!(owner.validate_admission(program.value_origin).is_err());
    let callbacks = Cell::new(0);
    let error = owner
        .inspect_transfer(origin, |_| {
            callbacks.set(callbacks.get() + 1);
            Ok(())
        })
        .unwrap_err();
    assert_eq!(error.code, "normalized_product_token");
    assert_eq!(callbacks.get(), 0);
}

#[test]
fn cancelled_view_boundaries_preserve_cancellation_class() {
    let (program, _, view) = fixture();
    let control = ExecutionControl::uncancelled();
    control.cancel();
    for error in [
        codec::decode_typed_with_control(
            &program,
            b"broken",
            view,
            JsonLimits::default(),
            &control,
        )
        .unwrap_err(),
        data_codec::decode_typed_with_control(&program, b"broken", view, &control).unwrap_err(),
        data_codec_reference::decode_typed_with_control(&program, b"broken", view, &control)
            .unwrap_err(),
    ] {
        assert_eq!(error.class, DiagnosticClass::Cancelled);
    }
}
