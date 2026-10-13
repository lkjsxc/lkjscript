//! Production admission/custody controls. These do not replace native consumer acceptance.
#![allow(clippy::expect_used, clippy::unwrap_used)]
use super::*;
use crate::platform::execution::normalized::prepare::{
    NormalizedReadViewField, NormalizedReadViewLayout,
};
use crate::platform::kernel::{ReadScopeParameter, ReadViewFieldReference, TypeObject};
use crate::platform::semantic_id::{BindingId, DeclarationId, FieldId, ReadScopeParameterId};

fn executable_view() -> (NormalizedProgram, FunctionIndex) {
    let (base, functions) = super::super::borrow_result_tests::fixture();
    let mut program = (*base).clone();
    let main = functions["main"];
    let declaration = program.functions[main.0 as usize].declaration;
    let family = crate::platform::kernel::DeclarationReference {
        package: declaration.package,
        declaration: DeclarationId::migrate(b"production-view-execution-family", 1),
    };
    let scope_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(b"production-view-execution", 1),
        name: Name::new("loan").unwrap(),
    };
    let formal = ReadScopeArgument::Parameter(crate::platform::kernel::ReadScopeReference {
        binder: ReadScopeBinder::Declaration(family),
        parameter: scope_parameter.id,
    });
    let scope = ReadScopeArgument::Lexical {
        callable: declaration,
        binding: BindingId::migrate(b"production-view-execution", 1),
    };
    let cell_ty = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::OwnedI64Cell).then_some(*ty))
        .unwrap();
    let scalar = program
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::I64).then_some(*ty))
        .unwrap();
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration: family,
        arguments: vec![],
        read_scope: scope,
    })
    .unwrap();
    let view_ty = crate::platform::kernel::encode_type_object(&object)
        .unwrap()
        .0;
    program.types.insert(view_ty, object);
    let field = FieldId::migrate(b"production-view-execution", 1);
    let mut layouts = program.read_views.to_vec();
    // Preparation derives a lexical instance from this genuine family template.
    // An already-instantiated layout cannot replace the family's formal scope.
    layouts.push(NormalizedReadViewLayout {
        declaration: family,
        type_parameters: Arc::from([]),
        type_parameter_constraints: Arc::from([]),
        arguments: Arc::from([]),
        scope_parameter,
        read_scope: formal,
        fields: Arc::from([NormalizedReadViewField {
            reference: ReadViewFieldReference {
                declaration: family,
                field,
            },
            name: Name::new("referent").unwrap(),
            ty: cell_ty,
            kind: ReadViewFieldKind::ReadReference { read_scope: formal },
        }]),
    });
    program.read_views = layouts.into();
    let call = |function| NormalizedInstruction::Call {
        function,
        type_arguments: Arc::from([]),
        read_scope_arguments: Arc::from([]),
        effect_arguments: Arc::from([]),
        requirement_arguments: Arc::from([]),
        arguments: 1,
    };
    let target = &mut Arc::make_mut(&mut program.functions)[main.0 as usize];
    target.parameters = Arc::from([]);
    target.parameter_count = 0;
    target.result = crate::platform::kernel::CallableResult {
        ty: scalar,
        relation: ResultRelation::Value,
    };
    target.callsites = Arc::from([]);
    target.body = NormalizedFunctionBody::Code(NormalizedCode {
        parameter_count: 0,
        local_count: 3,
        instructions: Arc::from([
            NormalizedInstruction::I64(73),
            call(functions["new-cell"]),
            NormalizedInstruction::StoreLocal(0),
            NormalizedInstruction::BeginReadViewScope { read_scope: scope },
            NormalizedInstruction::BeginReadView {
                view_type: view_ty,
                source_local: 0,
                fields: Arc::from([0]),
            },
            NormalizedInstruction::LoadLocal {
                local: 0,
                use_mode: ParameterUse::Borrow,
            },
            NormalizedInstruction::FinishReadView {
                view_type: view_ty,
                fields: Arc::from([0]),
            },
            NormalizedInstruction::StoreLocal(1),
            NormalizedInstruction::BorrowReadViewField {
                view_type: view_ty,
                source_local: 1,
                field: 0,
                binding_local: 2,
                binding_type: cell_ty,
            },
            NormalizedInstruction::LoadLocal {
                local: 2,
                use_mode: ParameterUse::Borrow,
            },
            call(functions["read-cell"]),
            NormalizedInstruction::EndOwnedBorrow { binding_local: 2 },
            NormalizedInstruction::Drop,
            NormalizedInstruction::DropReadView {
                view_type: view_ty,
                source_local: 1,
            },
            NormalizedInstruction::Drop,
            NormalizedInstruction::LoadLocal {
                local: 0,
                use_mode: ParameterUse::Consume,
            },
            call(functions["finish-cell"]),
            NormalizedInstruction::EndReadViewScope { read_scope: scope },
            NormalizedInstruction::Return,
        ]),
    });
    crate::platform::execution::normalized::prepared_types::complete_controlled(
        &mut program,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    let derived = program
        .read_view_layout(view_ty)
        .expect("derived lexical view instance");
    assert_eq!(derived.declaration, family);
    assert_eq!(derived.read_scope, scope);
    assert_eq!(
        derived.fields[0].kind,
        ReadViewFieldKind::ReadReference { read_scope: scope },
    );
    (program, main)
}

#[test]
fn prepared_vm_constructs_and_reads_real_view_then_drains_source_loans() {
    let (program, main) = executable_view();
    let cells = super::super::super::owned_i64_cell::StorageObservation::start();
    let observed = std::sync::Mutex::new(None);
    let result = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .observing_checked(&observed)
        .invoke_entry(
            NormalizedEntryPoint::Function(main),
            vec![],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .0;
    assert_eq!(result, NormalizedValue::I64(73));
    assert_eq!(cells.live(), (0, 0));
    let observed = observed.into_inner().unwrap().unwrap();
    assert_eq!(observed.live_call_frames_after, 0);
    assert_eq!(observed.live_locals_after, 0);
    assert_eq!(observed.live_operands_after, 0);
}

#[test]
fn prepared_vm_fuel_exits_drain_pending_and_published_view_custody() {
    let (program, main) = executable_view();
    for steps in [5, 7, 9, 11] {
        let cells = super::super::super::owned_i64_cell::StorageObservation::start();
        let observed = std::sync::Mutex::new(None);
        let policy = NormalizedRunPolicy {
            instruction_steps: Some(steps),
            ..NormalizedRunPolicy::foreground()
        };
        let result = NormalizedVm::for_test(&program, policy)
            .observing_checked(&observed)
            .invoke_entry(
                NormalizedEntryPoint::Function(main),
                vec![],
                None,
                &ExecutionControl::uncancelled(),
            );
        assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        assert_eq!(cells.live(), (0, 0));
        let observed = observed.into_inner().unwrap().unwrap();
        assert_eq!(observed.live_call_frames_after, 0);
        assert_eq!(observed.live_locals_after, 0);
        assert_eq!(observed.live_operands_after, 0);
    }
}

fn fresh() -> super::super::super::value::ValueOrigin {
    super::super::super::value::ValueOrigin::fresh().unwrap()
}

struct Fixture {
    program: NormalizedProgram,
    view_ty: TypeObjectDigest,
    cell_ty: TypeObjectDigest,
    scope: ReadScopeArgument,
    source: SourceAnchor,
    field: FieldId,
}

impl Fixture {
    fn new() -> Self {
        let mut program = crate::platform::execution::normalized::tests::mixed_storage_contract_tests::prepared_mixed();
        let declaration = program.functions[0].declaration;
        let scope = ReadScopeArgument::Lexical {
            callable: declaration,
            binding: BindingId::migrate(b"production-view-admission", 1),
        };
        let cell = TypeObject::new(TypeForm::OwnedI64Cell).unwrap();
        let cell_ty = crate::platform::kernel::encode_type_object(&cell)
            .unwrap()
            .0;
        program.types.insert(cell_ty, cell);
        let object = TypeObject::new(TypeForm::ReadViewApplied {
            declaration,
            arguments: vec![],
            read_scope: scope,
        })
        .unwrap();
        let view_ty = crate::platform::kernel::encode_type_object(&object)
            .unwrap()
            .0;
        program.types.insert(view_ty, object);
        let field = FieldId::migrate(b"production-view-admission", 1);
        let layout = NormalizedReadViewLayout {
            declaration,
            type_parameters: Arc::from([]),
            type_parameter_constraints: Arc::from([]),
            arguments: Arc::from([]),
            scope_parameter: ReadScopeParameter {
                id: ReadScopeParameterId::migrate(b"production-view-admission", 1),
                name: Name::new("loan").unwrap(),
            },
            read_scope: scope,
            fields: Arc::from([NormalizedReadViewField {
                reference: ReadViewFieldReference { declaration, field },
                name: Name::new("referent").unwrap(),
                ty: cell_ty,
                kind: ReadViewFieldKind::ReadReference { read_scope: scope },
            }]),
        };
        let mut layouts = program.read_views.to_vec();
        program
            .read_view_instances
            .insert(view_ty, layouts.len() as u32);
        layouts.push(layout);
        program.read_views = layouts.into();
        let source = SourceAnchor {
            program: program.value_origin,
            application: fresh(),
            invocation: fresh(),
            coordinate: SourceCoordinate::Parameter(0),
            origin: fresh(),
            path: Arc::from([]),
        };
        Self {
            program,
            view_ty,
            cell_ty,
            scope,
            source,
            field,
        }
    }

    fn permit<'a>(&'a self, source: &'a SourceAnchor, lifetime: SourceAnchor) -> Admission<'a> {
        Admission {
            program: &self.program,
            selected: source,
            target: ViewIdentity {
                ty: self.view_ty,
                program: self.program.value_origin,
            },
            scope: self.scope,
            lifetime,
            nodes: std::cell::Cell::new(0),
        }
    }

    fn cell(&self) -> super::super::super::owned_i64_cell::OwnedI64Cell {
        let cell = super::super::super::owned_i64_cell::OwnedI64Cell::new(self.source.origin, 73);
        cell.establish_admission(self.program.value_origin).unwrap();
        cell
    }

    fn view(
        &self,
        cell: &super::super::super::owned_i64_cell::OwnedI64Cell,
    ) -> (storage::ReadView, storage::RuntimeScopeOwner) {
        let control = ExecutionControl::uncancelled();
        let permit = self.permit(&self.source, self.source.clone());
        let (owner, scope) = storage::RuntimeScopeOwner::open(
            self.scope,
            &self.source,
            None,
            &permit,
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        let loans = storage::SourceLoans::new(
            HeldRead {
                origin: self.source.origin,
                value: NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
            },
            0,
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        let construction = storage::ViewConstruction {
            identity: permit.target,
            source: self.source.clone(),
            scope,
            fields: 1,
        };
        let mut builder = storage::ViewBuilder::from_owned(
            construction,
            loans,
            0,
            &permit,
            &control,
            &mut |_| Ok(()),
        )
        .unwrap();
        builder
            .owned_reference(
                self.field,
                self.cell_ty,
                self.source.clone(),
                NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
                &permit,
                &control,
            )
            .unwrap();
        (
            builder.finish(&permit, &control, &mut |_| Ok(())).unwrap(),
            owner,
        )
    }
}

#[test]
fn projected_owned_source_preserves_containing_custody_after_parent_handle_drop() {
    let fixture = Fixture::new();
    let cell = fixture.cell();
    let control = ExecutionControl::uncancelled();
    let (parent, owner) = fixture.view(&cell);
    let mut projected = fixture.source.clone();
    projected.path = Arc::from([ReadPathStep::ViewReference {
        ty: fixture.view_ty,
        field: fixture.field,
    }]);
    let parent_permit = fixture.permit(&fixture.source, fixture.source.clone());
    let packet = parent
        .project_owned(fixture.field, projected.clone(), &parent_permit, &control)
        .unwrap();
    let permit = fixture.permit(&projected, fixture.source.clone());
    let construction = storage::ViewConstruction {
        identity: permit.target,
        source: projected.clone(),
        scope: packet.scope().unwrap().reborrow(&control).unwrap(),
        fields: 1,
    };
    let loans = storage::SourceLoans::new(
        HeldRead {
            origin: projected.origin,
            value: packet.raw(&control).unwrap().memory_borrow().unwrap(),
        },
        0,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let mut builder = storage::ViewBuilder::from_owned(
        construction,
        loans,
        1,
        &permit,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    builder
        .retain_projected(&packet, &permit, &control)
        .unwrap();
    builder
        .owned_reference(
            fixture.field,
            fixture.cell_ty,
            projected.clone(),
            packet.raw(&control).unwrap().memory_borrow().unwrap(),
            &permit,
            &control,
        )
        .unwrap();
    let child = builder.finish(&permit, &control, &mut |_| Ok(())).unwrap();
    drop(packet);
    drop(parent);
    child.validate(&control).unwrap();
    let read = child
        .project_owned(fixture.field, projected.clone(), &permit, &control)
        .unwrap();
    read.raw(&control)
        .unwrap()
        .memory_validate(projected.origin, false)
        .unwrap();
    drop(owner);
    assert!(read.raw(&control).is_err());
    assert!(child.validate(&control).is_err());
}

#[test]
fn forwarded_alias_preserves_original_field_path_and_rejects_other_selected_argument() {
    let fixture = Fixture::new();
    let cell = fixture.cell();
    let control = ExecutionControl::uncancelled();
    let (parent, _owner) = fixture.view(&cell);
    let mut alias = fixture.source.clone();
    alias.application = fresh();
    alias.invocation = fresh();
    alias.coordinate = SourceCoordinate::Parameter(1);
    let original = fixture.permit(&fixture.source, fixture.source.clone());
    let forwarded = parent
        .reborrow_as(
            storage::ViewTarget {
                identity: original.target,
                source: alias.clone(),
                scope: parent.scope().unwrap().reborrow(&control).unwrap(),
                sharing: false,
            },
            &original,
            &control, crate::platform::execution::normalized::read_view_storage::ReadView::reserve_descriptor(&control, &mut |_| Ok(())).unwrap())
        .unwrap();
    let permit = fixture.permit(&alias, fixture.source.clone());
    let read = forwarded
        .project_owned(fixture.field, alias.clone(), &permit, &control)
        .unwrap();
    read.raw(&control).unwrap();
    let wrong = fixture.permit(&fixture.source, fixture.source.clone());
    assert!(
        forwarded
            .project_owned(fixture.field, alias.clone(), &wrong, &control)
            .is_err()
    );
    assert!(forwarded.clone().validate(&control).is_err());
}

#[test]
fn same_source_wrong_owned_referent_type_and_other_argument_field_are_refused() {
    let fixture = Fixture::new();
    let cell = fixture.cell();
    let control = ExecutionControl::uncancelled();
    let permit = fixture.permit(&fixture.source, fixture.source.clone());
    let (_owner, scope) = storage::RuntimeScopeOwner::open(
        fixture.scope,
        &fixture.source,
        None,
        &permit,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    let construction = storage::ViewConstruction {
        identity: permit.target,
        source: fixture.source.clone(),
        scope,
        fields: 1,
    };
    let buffer = super::super::super::byte_buffer::ByteBuffer::create(
        fixture.source.origin,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    buffer
        .establish_admission(fixture.program.value_origin)
        .unwrap();
    use storage::ReadViewAdmission;
    assert!(
        permit
            .validate(
                AdmissionRequest::Field {
                    construction: &construction,
                    input: FieldAdmission::Owned {
                        field: fixture.field,
                        ty: fixture.cell_ty,
                        source: &fixture.source,
                        value: &NormalizedValue::ByteBuffer(buffer.borrow().unwrap())
                    }
                },
                &control
            )
            .is_err()
    );
    let mut other = fixture.source.clone();
    other.coordinate = SourceCoordinate::Parameter(1);
    assert!(
        permit
            .validate(
                AdmissionRequest::Field {
                    construction: &construction,
                    input: FieldAdmission::Owned {
                        field: fixture.field,
                        ty: fixture.cell_ty,
                        source: &other,
                        value: &NormalizedValue::OwnedI64Cell(cell.borrow().unwrap())
                    }
                },
                &control
            )
            .is_err()
    );
}

#[test]
fn expired_projected_packet_is_checked_before_any_owned_read_use() {
    let fixture = Fixture::new();
    let cell = fixture.cell();
    let control = ExecutionControl::uncancelled();
    let (parent, owner) = fixture.view(&cell);
    let permit = fixture.permit(&fixture.source, fixture.source.clone());
    let packet = parent
        .project_owned(fixture.field, fixture.source.clone(), &permit, &control)
        .unwrap();
    let raw = packet.raw(&control).unwrap().memory_borrow().unwrap();
    let mut checked = CheckedValue::memory(&fixture.program, raw).unwrap();
    checked.retain_projected_read(Arc::new(packet));
    let duplicate = checked.duplicate(ParameterUse::Borrow).unwrap();
    drop(parent);
    checked
        .validate_read_custody(&control, &mut |_| Ok(()))
        .unwrap();
    duplicate
        .validate_read_custody(&control, &mut |_| Ok(()))
        .unwrap();
    drop(owner);
    assert!(
        checked
            .validate_read_custody(&control, &mut |_| Ok(()))
            .is_err()
    );
    assert!(
        duplicate
            .validate_read_custody(&control, &mut |_| Ok(()))
            .is_err()
    );
}

#[test]
fn expired_projected_packet_beyond_type_depth_is_still_validated_and_cleaned() {
    let fixture = Fixture::new();
    let cell = fixture.cell();
    let control = ExecutionControl::uncancelled();
    let (parent, owner) = fixture.view(&cell);
    let permit = fixture.permit(&fixture.source, fixture.source.clone());
    let packet = parent
        .project_owned(fixture.field, fixture.source.clone(), &permit, &control)
        .unwrap();
    let raw = packet.raw(&control).unwrap().memory_borrow().unwrap();
    let mut checked = CheckedValue::memory(&fixture.program, raw).unwrap();
    checked.retain_projected_read(Arc::new(packet));
    for depth in 0..600 {
        let mut outer = CheckedValue::memory(
            &fixture.program,
            NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
        )
        .unwrap();
        outer.set_borrow_source(Some(super::BorrowSource {
            frame: 0,
            local: depth,
        }));
        outer.reserve_borrow_guards(1).unwrap();
        outer.retain_borrow_guard_from(|| Ok(checked)).unwrap();
        checked = outer;
    }
    drop(parent);
    let mut healthy_bytes = 0;
    checked
        .validate_read_custody(&control, &mut |bytes| {
            healthy_bytes += bytes;
            Ok(())
        })
        .unwrap();
    assert!(healthy_bytes > 0);
    drop(owner);
    let error = checked
        .validate_read_custody(&control, &mut |_| Ok(()))
        .unwrap_err();
    assert_eq!(
        error.class,
        crate::platform::execution::ExecutionFailureClass::Resource
    );
    assert_eq!(error.code, "normalized_read_view_token");
    drop(checked);
    assert!(cell.validate(fixture.source.origin, true).is_ok());
    assert_eq!(cell.extract().unwrap(), 73);
}

#[test]
fn view_source_custody_stops_at_moved_owning_memory_and_keeps_borrowed_edges_exact() {
    use crate::platform::kernel::{OwnerKey, OwnerRecord};
    use crate::platform::publication::GraphRepository;

    let source = r#"declarations.begin
(units (module create moved-view-source
  (read-view create CellView (visibility private)
    (scope-parameter read_scope_d8900000000000000000000000000001 loan)
    (view-field field_d8900000000000000000000000000001 cell (type OwnedI64Cell)
      (read-reference (read-scope (declaration CellView) read_scope_d8900000000000000000000000000001))))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create left (type I64)) (parameter create right (type I64)) (returns I64))
  (function create make-view (visibility private) (effect pure)
    (scope-parameter read_scope_d8900000000000000000000000000002 loan)
    (parameter create cell (type OwnedI64Cell) (use borrow))
    (scope-source cell (read-scope (declaration make-view) read_scope_d8900000000000000000000000000002))
    (returns (read-view-application CellView (types)
      (read-scope (declaration make-view) read_scope_d8900000000000000000000000000002))
      (view-from cell (read-scope (declaration make-view) read_scope_d8900000000000000000000000000002)))
    (body (view-create CellView (types)
      (scope (read-scope (declaration make-view) read_scope_d8900000000000000000000000000002)) (source cell)
      (field field_d8900000000000000000000000000001 (local cell)))))
  (function create from-parameter (visibility private) (effect pure)
    (parameter create incoming (type OwnedI64Cell) (use consume)) (returns I64 (value))
    (body (let
      (binding view (as $parameter-view)
        (type (read-view-application CellView (types) (lexical-read-scope from-parameter $parameter-view)))
        (borrow-call (call make-view (scopes (lexical-read-scope from-parameter $parameter-view)) (local incoming))
          (binding constructed (type (read-view-application CellView (types)
            (lexical-read-scope from-parameter $parameter-view))))
          (in (view-move (local constructed)))))
      (binding observed (type I64)
        (borrow-view-field (local view) CellView field_d8900000000000000000000000000001
          (binding read (type OwnedI64Cell)) (in (call read-cell (local read)))))
      (in (sequence (view-drop (local view)) (call add (local observed) (call finish-cell (local incoming))))))))
  (function create from-sibling (visibility private) (effect pure)
    (parameter create incoming (type OwnedI64Cell) (use consume)) (returns I64 (value))
    (body (let
      (binding owner (type OwnedI64Cell) (local incoming))
      (binding view (as $sibling-view)
        (type (read-view-application CellView (types) (lexical-read-scope from-sibling $sibling-view)))
        (borrow-call (call make-view (scopes (lexical-read-scope from-sibling $sibling-view)) (local owner))
          (binding constructed (type (read-view-application CellView (types)
            (lexical-read-scope from-sibling $sibling-view))))
          (in (view-move (local constructed)))))
      (binding observed (type I64)
        (borrow-view-field (local view) CellView field_d8900000000000000000000000000001
          (binding read (type OwnedI64Cell)) (in (call read-cell (local read)))))
      (in (sequence (view-drop (local view)) (call add (local observed) (call finish-cell (local owner))))))))
  (function create main (visibility public) (effect pure)
    (parameter create sibling (type Bool)) (returns I64 (value))
    (body (let (binding owner (type OwnedI64Cell) (call new-cell (i64 73)))
      (in (if (local sibling) (call from-sibling (local owner)) (call from-parameter (local owner)))))))))
declarations.end"#;
    let snapshot = super::super::super::tests::byte_buffer_tests::author_only(source).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let repository =
        GraphRepository::create(&temporary.path().join("moved-view-source"), &snapshot, None)
            .unwrap()
            .repository;
    // Exercise real source validation, artifact construction and complete preparation.
    let prepared = crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
    let program = prepared.program;
    let main = snapshot
        .owners
        .iter()
        .find_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                if owner.name == Name::new("main").unwrap() =>
            {
                program.function(crate::platform::kernel::DeclarationReference {
                    package: snapshot.root.package_id,
                    declaration: *id,
                })
            }
            _ => None,
        })
        .unwrap();
    for sibling in [false, true] {
        let cells = super::super::super::owned_i64_cell::StorageObservation::start();
        let observation = std::sync::Mutex::new(None);
        let result = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .observing_checked(&observation)
            .invoke_entry(
                NormalizedEntryPoint::Function(main),
                vec![NormalizedValue::Bool(sibling)],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap()
            .0;
        assert_eq!(result, NormalizedValue::I64(146));
        assert_eq!(cells.created(), 1);
        assert_eq!(cells.live(), (0, 0));
        let observation = observation.into_inner().unwrap().unwrap();
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_locals_after, 0);
        assert_eq!(observation.live_operands_after, 0);
        assert_eq!(observation.capability_calls, 0);
        let halfway = observation.instructions / 2;
        drop(cells);
        let cells = super::super::super::owned_i64_cell::StorageObservation::start();
        let observation = std::sync::Mutex::new(None);
        let result = NormalizedVm::for_test(
            &program,
            NormalizedRunPolicy {
                instruction_steps: Some(halfway),
                ..NormalizedRunPolicy::foreground()
            },
        )
        .observing_checked(&observation)
        .invoke_entry(
            NormalizedEntryPoint::Function(main),
            vec![NormalizedValue::Bool(sibling)],
            None,
            &ExecutionControl::uncancelled(),
        );
        let error = result.unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
        assert_eq!(error.code, "normalized_instruction_steps");
        assert_eq!(cells.live(), (0, 0));
        let observation = observation.into_inner().unwrap().unwrap();
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_locals_after, 0);
        assert_eq!(observation.live_operands_after, 0);
    }

    // Historical owner identity is not permission to omit a borrowed read edge.
    // These private placement mutants use admitted real tokens; no raw seal is fabricated.
    let control = ExecutionControl::uncancelled();
    let domain = fresh();
    let cell = super::super::super::owned_i64_cell::OwnedI64Cell::new(domain, 73);
    cell.establish_admission(program.value_origin).unwrap();
    let read = |root, placement| {
        let mut value = CheckedValue::memory(
            &program,
            NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
        )
        .unwrap();
        value.set_borrow_source(Some(root));
        if let Some(placement) = placement {
            value.set_borrow_placement(placement);
        }
        value
    };
    let frame = |locals: Vec<Option<CheckedValue>>| Frame {
        id: 7,
        function: None,
        code: NormalizedCode {
            parameter_count: 0,
            local_count: locals.len() as u32,
            instructions: Arc::from([]),
        },
        instruction: 0,
        locals,
        owned_loans: vec![],
        pending_borrow_call: None,
        borrow_result_target: None,
        view_scopes: vec![],
        view_builders: vec![],
        lexical_view_scopes: vec![],
        application_identity: fresh(),
        invocation_identity: fresh(),
        type_arguments: BTreeMap::new(),
        stack_base: 0,
    };
    for (locals, message) in [
        (
            vec![None, Some(read(BorrowSource { frame: 7, local: 0 }, None))],
            "view source custody lost its exact local",
        ),
        (
            vec![Some(read(
                BorrowSource { frame: 7, local: 0 },
                Some(BorrowSource { frame: 6, local: 0 }),
            ))],
            "view source placement lost its caller activation",
        ),
        (
            vec![
                Some(read(BorrowSource { frame: 7, local: 1 }, None)),
                Some(read(BorrowSource { frame: 7, local: 0 }, None)),
            ],
            "view source placements contain a cycle",
        ),
    ] {
        let frames = [frame(locals)];
        let local = if message == "view source custody lost its exact local" {
            1
        } else {
            0
        };
        let error = visit_source_custody(&frames, 0, local, &control, &mut |value| {
            value.raw().memory_validate(domain, false)
        })
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
        assert_eq!(error.code, "normalized_runtime_type");
        assert_eq!(error.message, message);
    }
    let frames = [frame(vec![Some(read(
        BorrowSource { frame: 7, local: 0 },
        None,
    ))])];
    assert_eq!(
        visit_source_custody(
            &frames,
            0,
            0,
            &ExecutionControl::cancel_after_checks(0),
            &mut |_| Ok(()),
        )
        .unwrap_err()
        .class,
        ExecutionFailureClass::Cancelled,
    );
    drop(frames);
    cell.validate(domain, true).unwrap();
    assert_eq!(cell.extract().unwrap(), 73);
}

#[test]
fn owning_memory_value_return_releases_only_dead_placement_before_caller_view_use() {
    use super::super::super::reference::NormalizedReferenceInterpreter;
    use crate::platform::kernel::{OwnerKey, OwnerRecord};
    use crate::platform::publication::GraphRepository;

    let source = r#"declarations.begin
(units (module create returned-view-source
  (read-view create CellView (visibility private)
    (scope-parameter read_scope_d8910000000000000000000000000001 loan)
    (view-field field_d8910000000000000000000000000001 cell (type OwnedI64Cell)
      (read-reference (read-scope (declaration CellView) read_scope_d8910000000000000000000000000001))))
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create cell (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create finish-cell (visibility private) (implementation core.cell.extract)
    (parameter create cell (type OwnedI64Cell) (use consume)) (returns I64))
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create left (type I64)) (parameter create right (type I64)) (returns I64))
  (function create return-owner (visibility private) (effect pure)
    (parameter create incoming (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell (value)) (body (local incoming)))
  (function create return-again (visibility private) (effect pure)
    (parameter create incoming (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell (value)) (body (call return-owner (local incoming))))
  (function create make-view (visibility private) (effect pure)
    (scope-parameter read_scope_d8910000000000000000000000000002 loan)
    (parameter create cell (type OwnedI64Cell) (use borrow))
    (scope-source cell (read-scope (declaration make-view) read_scope_d8910000000000000000000000000002))
    (returns (read-view-application CellView (types)
      (read-scope (declaration make-view) read_scope_d8910000000000000000000000000002))
      (view-from cell (read-scope (declaration make-view) read_scope_d8910000000000000000000000000002)))
    (body (view-create CellView (types)
      (scope (read-scope (declaration make-view) read_scope_d8910000000000000000000000000002)) (source cell)
      (field field_d8910000000000000000000000000001 (local cell)))))
  (function create main (visibility public) (effect pure)
    (parameter create multiple (type Bool)) (returns I64 (value))
    (body (let
      (binding initial (type OwnedI64Cell) (call new-cell (i64 73)))
      (binding owner (type OwnedI64Cell)
        (if (local multiple) (call return-again (local initial)) (call return-owner (local initial))))
      (binding view (as $returned-view)
        (type (read-view-application CellView (types) (lexical-read-scope main $returned-view)))
        (borrow-call (call make-view (scopes (lexical-read-scope main $returned-view)) (local owner))
          (binding constructed (type (read-view-application CellView (types)
            (lexical-read-scope main $returned-view))))
          (in (view-move (local constructed)))))
      (binding observed (type I64)
        (borrow-view-field (local view) CellView field_d8910000000000000000000000000001
          (binding read (type OwnedI64Cell)) (in (call read-cell (local read)))))
      (in (sequence (view-drop (local view)) (call add (local observed) (call finish-cell (local owner))))))))))
declarations.end"#;
    let snapshot = super::super::super::tests::byte_buffer_tests::author_only(source).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(
        &temporary.path().join("returned-view-source"),
        &snapshot,
        None,
    )
    .unwrap()
    .repository;
    let prepared = crate::platform::normalized_lifecycle::prepare_repository(repository).unwrap();
    let program = prepared.program;
    let functions = snapshot
        .owners
        .iter()
        .filter_map(|(key, owner)| match (key, owner) {
            (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner)) => {
                let declaration = crate::platform::kernel::DeclarationReference {
                    package: snapshot.root.package_id,
                    declaration: *id,
                };
                program
                    .function(declaration)
                    .map(|index| (owner.name.to_string(), (declaration, index)))
            }
            _ => None,
        })
        .collect::<BTreeMap<_, _>>();
    let (main_declaration, main) = functions["main"];
    for multiple in [false, true] {
        for reference in [false, true] {
            let cells = super::super::super::owned_i64_cell::StorageObservation::start();
            let control = ExecutionControl::uncancelled();
            let value = if reference {
                let (value, observation) = NormalizedReferenceInterpreter::from_reader(
                    &prepared.reference,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(
                    main_declaration,
                    vec![NormalizedValue::Bool(multiple)],
                    None,
                    &control,
                )
                .unwrap();
                assert!(observation.canonical_owner_reads > 0);
                assert_eq!(observation.live_call_frames_after, 0);
                assert_eq!(observation.live_local_scopes_after, 0);
                assert_eq!(observation.capability_calls, 0);
                value
            } else {
                let observation = std::sync::Mutex::new(None);
                let value = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .observing_checked(&observation)
                    .invoke_entry(
                        NormalizedEntryPoint::Function(main),
                        vec![NormalizedValue::Bool(multiple)],
                        None,
                        &control,
                    )
                    .unwrap()
                    .0;
                let observation = observation.into_inner().unwrap().unwrap();
                assert_eq!(observation.live_call_frames_after, 0);
                assert_eq!(observation.live_locals_after, 0);
                assert_eq!(observation.live_operands_after, 0);
                assert_eq!(observation.capability_calls, 0);
                value
            };
            assert_eq!(value, NormalizedValue::I64(146));
            assert_eq!(cells.created(), 1);
            assert_eq!(cells.live(), (0, 0));
        }
    }
    // The metadata reset follows consuming admission; it cannot launder a real
    // borrowed token or let a ViewFrom result cross the ordinary return path.
    for view_return in [false, true] {
        let mut bad = (*program).clone();
        let declaration = functions[if view_return {
            "make-view"
        } else {
            "return-owner"
        }]
        .0;
        let mut changed = 0;
        // Scope specialization keeps the same exact canonical declaration.
        // Mutate its complete instance inventory, including the invoked body.
        for target in Arc::make_mut(&mut bad.functions)
            .iter_mut()
            .filter(|target| target.declaration == declaration)
        {
            let NormalizedFunctionBody::Code(code) = &mut target.body else {
                panic!("native graph body")
            };
            let instructions = Arc::make_mut(&mut code.instructions);
            if view_return {
                let instruction = instructions
                    .iter_mut()
                    .find(|instruction| matches!(instruction, NormalizedInstruction::ReturnView))
                    .unwrap();
                *instruction = NormalizedInstruction::Return;
            } else {
                let instruction = instructions
                    .iter_mut()
                    .find(|instruction| {
                        matches!(instruction, NormalizedInstruction::LoadLocal { .. })
                    })
                    .unwrap();
                let NormalizedInstruction::LoadLocal { use_mode, .. } = instruction else {
                    unreachable!()
                };
                *use_mode = ParameterUse::Borrow;
            }
            changed += 1;
        }
        assert!(changed > 0);
        let cells = super::super::super::owned_i64_cell::StorageObservation::start();
        let observation = std::sync::Mutex::new(None);
        let error = NormalizedVm::for_test(&bad, NormalizedRunPolicy::foreground())
            .observing_checked(&observation)
            .invoke_entry(
                NormalizedEntryPoint::Function(main),
                vec![NormalizedValue::Bool(false)],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap_err();
        if view_return {
            assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            assert_eq!(error.code, "normalized_runtime_type");
            assert_eq!(
                error.message,
                "borrowed result requires its sealed return path"
            );
        } else {
            assert_eq!(error.class, ExecutionFailureClass::Resource);
            assert_eq!(error.code, "normalized_cell_token");
        }
        assert_eq!(cells.live(), (0, 0));
        let observation = observation.into_inner().unwrap().unwrap();
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_locals_after, 0);
        assert_eq!(observation.live_operands_after, 0);
    }

    // This private metadata probe is not an admitted owning return. It proves
    // clearing location hints never removes actual projected or ancestor leases.
    let fixture = Fixture::new();
    let cell = fixture.cell();
    let control = ExecutionControl::uncancelled();
    let (parent, lifetime) = fixture.view(&cell);
    let permit = fixture.permit(&fixture.source, fixture.source.clone());
    let packet = Arc::new(
        parent
            .project_owned(fixture.field, fixture.source.clone(), &permit, &control)
            .unwrap(),
    );
    let mut leaf = CheckedValue::memory(
        &fixture.program,
        packet.raw(&control).unwrap().memory_borrow().unwrap(),
    )
    .unwrap();
    leaf.retain_projected_read(Arc::clone(&packet));
    let path: Arc<[ReadPathStep]> = Arc::from([ReadPathStep::ViewReference {
        ty: fixture.view_ty,
        field: fixture.field,
    }]);
    leaf.set_read_path(Arc::clone(&path));
    let source = BorrowSource {
        frame: 99,
        local: 0,
    };
    let target = BorrowResultTarget {
        source,
        position: 0,
    };
    leaf.set_borrow_source(Some(source));
    leaf.set_borrow_placement(source);
    leaf.set_borrow_packet(target).unwrap();
    leaf.reserve_borrow_guards(1).unwrap();
    let guard = CheckedValue::memory(
        &fixture.program,
        NormalizedValue::OwnedI64Cell(cell.borrow().unwrap()),
    )
    .unwrap();
    leaf.retain_borrow_guard_from(|| Ok(guard)).unwrap();
    leaf.clear_owning_memory_placement();
    assert_eq!(leaf.borrow_source(), None);
    assert_eq!(leaf.borrow_placement(), None);
    assert_eq!(leaf.take_borrow_packet(), Some(target));
    assert!(std::ptr::eq(
        leaf.projected_read().unwrap(),
        packet.as_ref()
    ));
    assert!(Arc::ptr_eq(&leaf.shared_read_path().unwrap(), &path));
    let mut ancestors = 0;
    leaf.visit_ancestor_reads(&control, &mut |_| Ok(()), &mut |guard, _| {
        guard.raw().memory_validate(fixture.source.origin, false)?;
        ancestors += 1;
        Ok(())
    })
    .unwrap();
    assert_eq!(ancestors, 1);
    assert!(leaf.raw().memory_is_borrowed());
    assert_eq!(
        leaf.raw()
            .memory_validate(fixture.source.origin, true)
            .unwrap_err()
            .code,
        "normalized_cell_token"
    );
    drop(parent);
    leaf.validate_read_custody(&control, &mut |_| Ok(()))
        .unwrap();
    drop(lifetime);
    assert_eq!(
        leaf.validate_read_custody(&control, &mut |_| Ok(()))
            .unwrap_err()
            .code,
        "normalized_read_view_token"
    );
    drop(leaf);
    drop(packet);
    cell.validate(fixture.source.origin, true).unwrap();
    assert_eq!(cell.extract().unwrap(), 73);
}
