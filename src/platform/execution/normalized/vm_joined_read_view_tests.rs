//! Joined custody rejects substituted applications and live scopes independently
//! of the evaluator that originally built the descriptor.

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use super::*;
use crate::platform::execution::normalized::prepare::{
    NormalizedReadViewField, NormalizedReadViewLayout,
};
use crate::platform::execution::normalized::read_view_storage::{
    AdmissionRequest, HeldRead, ReadView, ReadViewAdmission, RuntimeScopeOwner, SourceAnchor,
    SourceCoordinate, SourceLoans, ViewBuilder, ViewConstruction, ViewIdentity,
};
use crate::platform::kernel::{
    CallableResult, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter, ReadScopeReference,
    ReadScopeSource, ReadViewFieldKind, ReadViewFieldReference, ResultRelation, TypeObject,
};
use crate::platform::semantic_id::{BindingId, FieldId, ReadScopeParameterId};
use std::sync::Arc;

fn fresh() -> ValueOrigin {
    ValueOrigin::fresh().expect("test identity")
}

fn fixture() -> (
    NormalizedProgram,
    FunctionIndex,
    TypeObjectDigest,
    ReadScopeArgument,
) {
    let mut program =
        crate::platform::execution::normalized::tests::mixed_storage_contract_tests::prepared_mixed(
        );
    let function = FunctionIndex(0, program.value_origin);
    let declaration = program.functions[0].declaration;
    let scope = ReadScopeArgument::Lexical {
        callable: declaration,
        binding: BindingId::migrate(b"joined-read-view", 0),
    };
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration,
        arguments: vec![],
        read_scope: scope,
    })
    .unwrap();
    let ty = crate::platform::kernel::encode_type_object(&object)
        .unwrap()
        .0;
    program.types.insert(ty, object);
    let index = program.read_views.len() as u32;
    let mut views = program.read_views.to_vec();
    views.push(NormalizedReadViewLayout {
        declaration,
        type_parameters: Arc::from([]),
        type_parameter_constraints: Arc::from([]),
        arguments: Arc::from([]),
        scope_parameter: ReadScopeParameter {
            id: ReadScopeParameterId::migrate(b"joined-read-view", 0),
            name: crate::platform::kernel::Name::new("read").unwrap(),
        },
        read_scope: scope,
        fields: Arc::from([]),
    });
    program.read_views = views.into();
    program.read_view_instances.insert(ty, index);
    let mut parameter = program
        .functions
        .iter()
        .find_map(|function| function.parameters.first())
        .unwrap()
        .clone();
    parameter.ty = ty;
    parameter.use_mode = ParameterUse::Borrow;
    parameter.resource_requirement = None;
    let scalar = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
        .unwrap();
    let selected = &mut Arc::make_mut(&mut program.functions)[0];
    selected.graph_function = true;
    selected.effect = FunctionEffect::Task {
        requirements: vec![],
        effect_parameters: vec![],
    };
    selected.type_parameters = Arc::from([]);
    selected.type_parameter_constraints = Arc::from([]);
    selected.type_arguments = Arc::from([]);
    selected.implementation_parameters = Arc::from([]);
    selected.implementation_arguments = Arc::from([]);
    selected.task_requirements = Arc::from([]);
    selected.effect_parameters = Arc::from([]);
    selected.requirement_parameters = Arc::from([]);
    selected.read_scope_parameters = Arc::from([]);
    selected.read_scope_sources = Arc::from([]);
    selected.read_scope_arguments = Arc::from([]);
    selected.parameters = Arc::from([parameter]);
    selected.parameter_count = 1;
    selected.result = CallableResult {
        ty: scalar,
        relation: ResultRelation::Value,
    };
    (program, function, ty, scope)
}

struct Admission {
    identity: ViewIdentity,
    source: SourceAnchor,
    scope: ReadScopeArgument,
}

struct ViewOwner {
    _scope: RuntimeScopeOwner,
    _cell: crate::platform::execution::normalized::owned_i64_cell::OwnedI64Cell,
}

impl ReadViewAdmission for Admission {
    fn validate(
        &self,
        request: AdmissionRequest<'_>,
        control: &ExecutionControl,
    ) -> Result<(), ExecutionError> {
        control.check()?;
        match request {
            AdmissionRequest::Scope {
                key,
                source,
                parent: None,
            } if key == self.scope && source == &self.source => Ok(()),
            AdmissionRequest::Source {
                construction,
                loans,
            } if construction.identity == self.identity
                && construction.source == self.source
                && construction.scope.key() == self.scope
                && construction.fields == 0
                && loans.reads().len() == 1 =>
            {
                Ok(())
            }
            AdmissionRequest::Finish {
                construction,
                fields,
            } if construction.identity == self.identity && fields.is_empty() => Ok(()),
            AdmissionRequest::Reborrow { source, target }
                if source.identity() == self.identity
                    && source.source() == &self.source
                    && target.source.program == self.identity.program
                    && target.sharing =>
            {
                Ok(())
            }
            _ => Err(reject()),
        }
    }
}

fn view(
    program: &NormalizedProgram,
    ty: TypeObjectDigest,
    scope: ReadScopeArgument,
    source: ValueOrigin,
) -> (NormalizedValue, ViewOwner, Admission) {
    let control = ExecutionControl::uncancelled();
    let cell =
        crate::platform::execution::normalized::owned_i64_cell::OwnedI64Cell::new(source, 73);
    cell.establish_admission(program.value_origin).unwrap();
    let anchor = SourceAnchor {
        program: program.value_origin,
        application: fresh(),
        invocation: fresh(),
        coordinate: SourceCoordinate::Parameter(0),
        origin: source,
        path: Arc::from([]),
    };
    let admission = Admission {
        identity: ViewIdentity {
            program: program.value_origin,
            ty,
        },
        source: anchor.clone(),
        scope,
    };
    let (owner, lease) =
        RuntimeScopeOwner::open(scope, &anchor, None, &admission, &control, &mut |_| Ok(()))
            .unwrap();
    let loans = SourceLoans::new(
        HeldRead {
            origin: source,
            value: NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
        },
        0,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let construction = ViewConstruction {
        identity: admission.identity,
        source: anchor,
        scope: lease,
        fields: 0,
    };
    let builder = ViewBuilder::from_owned(
        construction,
        loans,
        0,
        &admission,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    (
        NormalizedValue::ReadView(
            builder
                .finish(&admission, &control, &mut |_| Ok(()))
                .unwrap(),
        ),
        ViewOwner {
            _scope: owner,
            _cell: cell,
        },
        admission,
    )
}

fn lend(
    parent: &NormalizedValue,
    application: &TaskApplication,
    destination: ValueOrigin,
    admission: &Admission,
) -> NormalizedValue {
    let NormalizedValue::ReadView(parent) = parent else {
        panic!("view");
    };
    let anchor = SourceAnchor {
        program: admission.identity.program,
        application: application.application_identity(),
        invocation: destination,
        coordinate: SourceCoordinate::Parameter(0),
        origin: destination,
        path: Arc::from([]),
    };
    let control = ExecutionControl::uncancelled();
    let reservation = ReadView::reserve_descriptor(&control, &mut |_| Ok(())).unwrap();
    NormalizedValue::ReadView(
        parent
            .reborrow(anchor, true, admission, &control, reservation)
            .unwrap(),
    )
}

#[test]
fn joined_view_custody_binds_exact_application_destination_position_and_backing() {
    let (program, function, ty, scope) = fixture();
    let control = ExecutionControl::uncancelled();
    let source = fresh();
    let destination = fresh();
    let application =
        TaskApplication::bind(&program, function, Arc::from([]), &control, &mut |_| Ok(()))
            .unwrap();
    let (parent, owner, admission) = view(&program, ty, scope, source);
    let mut child = lend(&parent, &application, destination, &admission);
    let lease = ScopedReadLease::seal_view(
        &program,
        source,
        destination,
        &application,
        0,
        &child,
        admission.source.clone(),
        ScopedReadCustody::new(parent),
        &control,
    )
    .unwrap();
    assert!(
        lease
            .adopt(&program, destination, &application, &mut child, &control)
            .is_ok()
    );
    let other_application =
        TaskApplication::bind(&program, function, Arc::from([]), &control, &mut |_| Ok(()))
            .unwrap();
    assert!(
        lease
            .adopt(
                &program,
                destination,
                &other_application,
                &mut child,
                &control
            )
            .is_err()
    );
    assert!(
        lease
            .adopt(&program, fresh(), &application, &mut child, &control)
            .is_err()
    );
    let inert = child.clone();
    let mut inert = inert;
    assert!(
        lease
            .adopt(&program, destination, &application, &mut inert, &control)
            .is_err()
    );
    let (other_parent, other_owner, other_admission) = view(&program, ty, scope, source);
    let mut other_child = lend(&other_parent, &application, destination, &other_admission);
    assert!(
        lease
            .adopt(
                &program,
                destination,
                &application,
                &mut other_child,
                &control
            )
            .is_err()
    );
    assert!(
        ScopedReadLease::seal_view(
            &program,
            source,
            destination,
            &application,
            1,
            &child,
            admission.source.clone(),
            ScopedReadCustody::new(()),
            &control
        )
        .is_err()
    );
    drop(other_child);
    drop(other_parent);
    drop(other_owner);
    drop(owner);
    assert!(
        lease
            .adopt(&program, destination, &application, &mut child, &control)
            .is_err()
    );
}

#[test]
fn joined_scope_only_application_resolves_the_parameter_type() {
    let (mut program, function, ty, actual) = fixture();
    let declaration = program.functions[function.0 as usize].declaration;
    let parameter = program
        .read_view_layout(ty)
        .unwrap()
        .scope_parameter
        .clone();
    let formal = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: parameter.id,
    });
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration,
        arguments: vec![],
        read_scope: formal,
    })
    .unwrap();
    let template = crate::platform::kernel::encode_type_object(&object)
        .unwrap()
        .0;
    program.types.insert(template, object);
    let selected = &mut Arc::make_mut(&mut program.functions)[function.0 as usize];
    Arc::make_mut(&mut selected.parameters)[0].ty = template;
    selected.read_scope_parameters = Arc::from([parameter]);
    selected.read_scope_sources = Arc::from([ReadScopeSource {
        source: 0,
        scope: formal,
    }]);
    selected.read_scope_arguments = Arc::from([actual]);
    let application = TaskApplication::bind(
        &program,
        function,
        Arc::from([]),
        &ExecutionControl::uncancelled(),
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(application.parameter(&program, 0).unwrap(), ty);
    assert_eq!(application.read_scope_source_count(0), 1);
}

#[test]
fn joined_scope_keeps_the_original_lifetime_separate_from_rebased_placement() {
    let (program, _function, ty, scope) = fixture();
    let source = fresh();
    let control = ExecutionControl::uncancelled();
    let (raw, _owner, admission) = view(&program, ty, scope, source);
    let NormalizedValue::ReadView(view) = raw else {
        panic!("view");
    };
    let mut placement = admission.source.clone();
    placement.origin = fresh();
    placement.application = fresh();
    placement.invocation = fresh();
    assert!(
        ScopedReadScope::new(
            scope,
            view.scope().unwrap().reborrow(&control).unwrap(),
            placement.clone(),
            admission.source.clone()
        )
        .is_ok()
    );
    let (foreign, _foreign_owner, _foreign_admission) = self::view(&program, ty, scope, fresh());
    let NormalizedValue::ReadView(foreign) = foreign else {
        panic!("view");
    };
    assert!(
        ScopedReadScope::new(
            scope,
            foreign.scope().unwrap().reborrow(&control).unwrap(),
            placement,
            admission.source
        )
        .is_err(),
        "equal symbolic key cannot swap the original source lifetime"
    );
}

#[test]
fn joined_view_ancestor_custody_lasts_until_the_lease_closes() {
    let (program, function, ty, scope) = fixture();
    let control = ExecutionControl::uncancelled();
    let source = fresh();
    let destination = fresh();
    let application =
        TaskApplication::bind(&program, function, Arc::from([]), &control, &mut |_| Ok(()))
            .unwrap();
    let (parent, owner, admission) = view(&program, ty, scope, source);
    let mut child = lend(&parent, &application, destination, &admission);
    let lease = ScopedReadLease::seal_view(
        &program,
        source,
        destination,
        &application,
        0,
        &child,
        admission.source.clone(),
        ScopedReadCustody::new(parent),
        &control,
    )
    .unwrap();
    lease
        .adopt(&program, destination, &application, &mut child, &control)
        .unwrap();
    drop(child);
    assert!(
        owner._cell.validate(source, true).is_err(),
        "parent source remains borrowed through retained custody"
    );
    drop(lease);
    assert!(
        owner._cell.validate(source, true).is_ok(),
        "joined cleanup released the complete retained source packet"
    );
}

#[test]
fn view_read_sharing_checks_unused_declared_fields_and_never_grants_transfer() {
    let (program, function, ty, scope) = fixture();
    let control = ExecutionControl::uncancelled();
    assert!(admit_shareable_type(&program, ty, &control).unwrap());
    assert!(admit_type(&program, ty, &control).is_err());
    let scalar = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
        .unwrap();
    let owned = TypeObject::new(TypeForm::OwnedI64Cell).unwrap();
    let owned_ty = crate::platform::kernel::encode_type_object(&owned)
        .unwrap()
        .0;
    for (kind, field_ty, accepted) in [
        (ReadViewFieldKind::Ordinary, scalar, true),
        (ReadViewFieldKind::Ordinary, owned_ty, false),
        (
            ReadViewFieldKind::ReadReference { read_scope: scope },
            scalar,
            false,
        ),
        (
            ReadViewFieldKind::ReadReference { read_scope: scope },
            owned_ty,
            true,
        ),
        (
            ReadViewFieldKind::ReadReference {
                read_scope: ReadScopeArgument::Lexical {
                    callable: program.functions[function.0 as usize].declaration,
                    binding: BindingId::migrate(b"foreign-scope", 0),
                },
            },
            owned_ty,
            false,
        ),
    ] {
        let mut candidate = program.clone();
        candidate.types.insert(owned_ty, owned.clone());
        let index = candidate.read_view_instances[&ty] as usize;
        Arc::make_mut(&mut candidate.read_views)[index].fields =
            Arc::from([NormalizedReadViewField {
                reference: ReadViewFieldReference {
                    declaration: program.functions[function.0 as usize].declaration,
                    field: FieldId::migrate(b"unused-field", 0),
                },
                name: crate::platform::kernel::Name::new("unused").unwrap(),
                ty: field_ty,
                kind,
            }]);
        assert_eq!(
            admit_shareable_type(&candidate, ty, &control).is_ok(),
            accepted
        );
        assert!(admit_type(&candidate, ty, &control).is_err());
    }
    let source = fresh();
    let (raw, _owner, _admission) = view(&program, ty, scope, source);
    assert!(
        TransferResult::seal_applied(
            &program,
            source,
            fresh(),
            TaskApplication::bind(&program, function, Arc::from([]), &control, &mut |_| Ok(()))
                .unwrap(),
            raw,
            &control,
            &mut |_, _| panic!("views never enter metadata admission")
        )
        .is_err()
    );
}
