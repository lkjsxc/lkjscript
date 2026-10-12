//! Canonical-reader runtime controls, independent of public artifact admission.
use super::super::reference_types::associated_preparation::{Limits, test_support};
use super::super::tests::FaultedReferenceRead;
use super::*;
use crate::platform::kernel::{
    BindingRecord, ExpressionRecord, ExternalDeclaration, OwnerHeader, OwnerKind, ParameterParent,
    TypeObject, TypeParameterConstraints, TypeParameterRecord, encode_type_object,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::DeclarationId;
use std::sync::{OnceLock, atomic::Ordering};

const SEED: &[u8] = b"canonical-indirect-runtime";

fn program() -> Arc<NormalizedProgram> {
    static PROGRAM: OnceLock<Arc<NormalizedProgram>> = OnceLock::new();
    Arc::clone(PROGRAM.get_or_init(|| {
        // A genuine ordinary program supplies only physical value origins. All
        // expected callable meaning comes from the separate canonical snapshot.
        let source = test_support::base_source();
        let directory = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &directory.path().join("ordinary-indirect-context"),
            &source,
            None,
        )
        .unwrap()
        .repository;
        crate::platform::normalized_lifecycle::prepare_repository(repository)
            .unwrap()
            .program
    }))
}

fn scalar(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    source.types.insert(ty, object);
    ty
}

fn function(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut FunctionDeclaration {
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function
}

fn expression(
    source: &mut KernelSnapshot,
    id: ExpressionId,
) -> &mut crate::platform::kernel::ExpressionRecord {
    let OwnerRecord::Expression(expression) =
        source.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        unreachable!()
    };
    expression
}

fn fixture(tail: bool) -> test_support::InvokeFixture {
    let mut case: test_support::InvokeFixture = test_support::invoke_fixture();
    let source = &mut case.fixture.source;
    let i64_type = scalar(source, TypeForm::I64);
    let bool_type = scalar(source, TypeForm::Bool);
    let formals = [
        TypeParameterId::migrate(SEED, 0),
        TypeParameterId::migrate(SEED, 1),
    ];
    for (id, name) in formals.into_iter().zip(["T", "Unused"]) {
        source.owners.insert(
            OwnerKey::TypeParameter(id),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
                declaration: case.target.declaration,
                name: Name::new(name).unwrap(),
                constraints: TypeParameterConstraints::Transferable,
            }),
        );
    }
    let template = scalar(
        source,
        TypeForm::TypeParameter {
            parameter: formals[0],
        },
    );
    let target = function(source, case.target);
    target.type_parameters = formals.to_vec();
    target.result = template;
    let parameters = target.parameters.clone();
    for parameter in parameters {
        let OwnerRecord::Parameter(parameter) = source
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        parameter.ty = template;
    }
    let ExpressionOperation::FunctionValue { type_arguments, .. } =
        &mut expression(source, case.callee).operation
    else {
        unreachable!()
    };
    *type_arguments = vec![i64_type, bool_type];
    if !tail {
        // The binding's value is evaluated by the normal evaluator; a direct
        // caller body Invoke takes evaluate_tail's separate branch.
        let binding = BindingId::migrate(SEED, 0);
        let local = ExpressionId::migrate(SEED, 0);
        let body = ExpressionId::migrate(SEED, 1);
        source.owners.insert(
            OwnerKey::Binding(binding),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("called").unwrap(),
                kind: BindingKind::Let,
                value: Some(case.invocation),
                declared_type: Some(i64_type),
            }),
        );
        source.owners.insert(
            OwnerKey::Expression(local),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    local,
                    ExpressionOperation::Local {
                        value: LocalValueReference::LexicalBinding(binding),
                    },
                )
                .unwrap(),
            ),
        );
        source.owners.insert(
            OwnerKey::Expression(body),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    body,
                    ExpressionOperation::Let {
                        bindings: vec![binding],
                        body: local,
                    },
                )
                .unwrap(),
            ),
        );
        function(source, case.caller).body = body;
    }
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        source.root.owners.content(),
    );
    case
}

fn schema(case: &test_support::InvokeFixture) -> Arc<NormalizedReferenceSchema> {
    let control = ExecutionControl::uncancelled();
    let proof = test_support::prove(&case.fixture, &[], Limits::default(), &control).unwrap();
    Arc::new(
        super::super::reference_types::associated_schema_fixture(
            &[&case.fixture.source],
            proof,
            &control,
        )
        .unwrap(),
    )
}

fn run(
    case: &test_support::InvokeFixture,
    source: &KernelSnapshot,
    schema: Arc<NormalizedReferenceSchema>,
) -> Result<NormalizedReferenceInvocation, ExecutionError> {
    let reader = FaultedReferenceRead { source, schema };
    let program = program();
    NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    )
    .execute(None, &ExecutionControl::uncancelled(), |state| {
        state.call_declaration(case.caller, &[], &[], &[], vec![])
    })
}

fn healthy(
    case: &test_support::InvokeFixture,
    schema: &Arc<NormalizedReferenceSchema>,
    tail: bool,
) {
    let (value, observation) = run(case, &case.fixture.source, Arc::clone(schema)).unwrap();
    assert_eq!(value, NormalizedValue::I64(17));
    assert_eq!(observation.calls, 2);
    assert_eq!(observation.tail_transfers, u64::from(tail));
    assert_eq!(observation.live_call_frames_after, 0);
    assert_eq!(observation.live_control_frames_after, 0);
    assert_eq!(observation.live_type_scopes_after, 0);
}

#[test]
fn reference_associated_invocations_execute_polymorphic_normal_and_tail_routes() {
    for tail in [false, true] {
        let case = fixture(tail);
        let schema = schema(&case);
        let proof = schema.associated_proof().unwrap();
        let caller = proof
            .applications
            .keys()
            .find(|application| application.function == case.caller)
            .unwrap();
        let target = proof
            .call_target(caller, case.callee, &ExecutionControl::uncancelled())
            .unwrap();
        assert_eq!(target.types.len(), 2);
        assert!(target.witnesses.is_empty());
        assert_ne!(
            proof.symbolic[&case.target].result,
            proof
                .application(target, &ExecutionControl::uncancelled())
                .unwrap()
                .result
        );
        healthy(&case, &schema, tail);
    }
}

#[test]
fn reference_associated_invocations_reject_changed_ordered_children_and_generations() {
    for tail in [false, true] {
        let case = fixture(tail);
        let schema = schema(&case);
        healthy(&case, &schema, tail);
        for mutation in 0..6 {
            let mut changed = case.fixture.source.clone();
            let invocation = expression(&mut changed, case.invocation);
            let ExpressionOperation::Invoke { callee, arguments } = &mut invocation.operation
            else {
                unreachable!()
            };
            match mutation {
                0 => *callee = case.argument, // Another present canonical child.
                1 => {
                    assert_eq!(arguments, &[case.argument, case.second_argument]);
                    arguments.swap(0, 1); // Both arguments remain well-typed I64.
                }
                2 => invocation.contract_version = 32,
                3 => invocation.contract_version = 34,
                4 | 5 => {
                    let OwnerRecord::Declaration(target) = changed
                        .owners
                        .get_mut(&OwnerKey::Declaration(case.target.declaration))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    target.header.contract_version = if mutation == 4 { 32 } else { 34 };
                }
                _ => unreachable!(),
            }
            let error = run(&case, &changed, Arc::clone(&schema)).unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            assert_eq!(error.code, "normalized_reference_type");
            healthy(&case, &schema, tail);
        }
    }
}

fn add_equal_target(case: &mut test_support::InvokeFixture) -> DeclarationReference {
    let source = &mut case.fixture.source;
    let target = DeclarationReference {
        package: case.target.package,
        declaration: DeclarationId::migrate(SEED, 2),
    };
    let parameters = [ParameterId::migrate(SEED, 2), ParameterId::migrate(SEED, 3)];
    let local = ExpressionId::migrate(SEED, 2);
    let i64_type = scalar(source, TypeForm::I64);
    let OwnerRecord::Declaration(mut owner) =
        source.owners[&OwnerKey::Declaration(case.target.declaration)].clone()
    else {
        unreachable!()
    };
    owner.header = OwnerHeader::new(
        OwnerKey::Declaration(target.declaration),
        OwnerKind::PureFunction,
    );
    owner.name = Name::new("sameClosedSignature").unwrap();
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.type_parameters.clear();
    function.parameters = parameters.to_vec();
    function.result = i64_type;
    function.body = local;
    source.owners.insert(
        OwnerKey::Declaration(target.declaration),
        OwnerRecord::Declaration(owner),
    );
    for (id, name) in parameters.into_iter().zip(["first", "second"]) {
        source.owners.insert(
            OwnerKey::Parameter(id),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                parent: ParameterParent::Function(target.declaration),
                name: Name::new(name).unwrap(),
                ty: i64_type,
                use_mode: ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
    }
    source.owners.insert(
        OwnerKey::Expression(local),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                local,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(parameters[0]),
                },
            )
            .unwrap(),
        ),
    );
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        source.root.owners.content(),
    );
    target
}

#[test]
fn reference_associated_function_values_reject_equal_signature_target_substitution() {
    for tail in [false, true] {
        let mut case = fixture(tail);
        let other = add_equal_target(&mut case);
        let schema = schema(&case);
        assert!(schema.functions.contains(&other));
        let proof = schema.associated_proof().unwrap();
        let caller = proof
            .applications
            .keys()
            .find(|application| application.function == case.caller)
            .unwrap();
        let target = proof
            .call_target(caller, case.callee, &ExecutionControl::uncancelled())
            .unwrap();
        let closed = proof
            .application(target, &ExecutionControl::uncancelled())
            .unwrap();
        assert_eq!(proof.symbolic[&other].parameters, closed.parameters);
        assert_eq!(proof.symbolic[&other].result, closed.result);
        healthy(&case, &schema, tail);
        let mut changed = case.fixture.source.clone();
        let ExpressionOperation::FunctionValue {
            function,
            type_arguments,
            ..
        } = &mut expression(&mut changed, case.callee).operation
        else {
            unreachable!()
        };
        *function = other;
        type_arguments.clear();
        let error = run(&case, &changed, Arc::clone(&schema)).unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
        assert_eq!(error.code, "normalized_reference_type");
        healthy(&case, &schema, tail);
    }
}

#[test]
fn reference_associated_invocations_preserve_no_witness_and_no_borrow_callable_boundary() {
    for tail in [false, true] {
        let case = fixture(tail);
        let schema = schema(&case);
        for borrow in [false, true] {
            let mut changed = case.fixture.source.clone();
            if borrow {
                let target = function(&mut changed, case.target);
                target.result_borrow = Some(target.parameters[0]);
            } else {
                let formals = function(&mut changed, case.fixture.consumer)
                    .implementation_parameters
                    .clone();
                function(&mut changed, case.target).implementation_parameters = formals;
            }
            let error = run(&case, &changed, Arc::clone(&schema)).unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            assert_eq!(error.code, "normalized_reference_type");
            healthy(&case, &schema, tail);
        }
    }
}

fn invocation_context(state: &mut ReferenceState<'_>, case: &test_support::InvokeFixture) {
    // Reuse an already derived exact application. This is a private consumer
    // input, not a source/artifact feature admission token.
    let caller = state
        .schema
        .associated_proof()
        .unwrap()
        .applications
        .keys()
        .find(|application| application.function == case.caller)
        .unwrap()
        .clone();
    state.associated_applications.push(Some(caller));
    state.active_package = case.caller.package;
    state.type_occurrence = Some((case.caller.package, OwnerKey::Expression(case.invocation)));
}

#[test]
fn reference_associated_invocations_preserve_shared_work_cancel_and_package_boundaries() {
    let case = fixture(true);
    let schema = schema(&case);
    let reader = FaultedReferenceRead {
        source: &case.fixture.source,
        schema: Arc::clone(&schema),
    };
    let program = program();
    let interpreter = NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    );
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64;
    let arguments = [case.argument, case.second_argument];
    for remaining in [3, 2] {
        let result = interpreter.execute(None, &ExecutionControl::uncancelled(), |state| {
            invocation_context(state, &case);
            state
                .associated_work
                .store(maximum - remaining, Ordering::Relaxed);
            let result = state.check_canonical_invocation(case.callee, &arguments);
            if remaining == 3 {
                assert_eq!(state.associated_work.load(Ordering::Relaxed), maximum);
            } else {
                assert_eq!(
                    state.associated_work.load(Ordering::Relaxed),
                    maximum - remaining
                );
            }
            state.associated_applications.pop();
            state.type_occurrence = None;
            result?;
            CheckedValue::primitive(&state.schema, NormalizedValue::I64(17))
        });
        if remaining == 3 {
            assert_eq!(result.unwrap().0, NormalizedValue::I64(17));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Resource);
            assert_eq!(error.code, "reference_associated_runtime_work");
        }
    }
    let control = ExecutionControl::uncancelled();
    let error = interpreter
        .execute(None, &control, |state| {
            invocation_context(state, &case);
            let before = state.associated_work.load(Ordering::Relaxed);
            control.cancel();
            let result = state.check_canonical_invocation(case.callee, &arguments);
            assert_eq!(state.associated_work.load(Ordering::Relaxed), before);
            result?;
            unreachable!()
        })
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    for active in [false, true] {
        let error = interpreter
            .execute(None, &ExecutionControl::uncancelled(), |state| {
                invocation_context(state, &case);
                let other = PackageId::migrate(SEED, 10);
                assert_ne!(other, case.caller.package);
                if active {
                    state.active_package = other;
                } else {
                    state.type_occurrence = Some((other, OwnerKey::Expression(case.invocation)));
                }
                state.check_canonical_invocation(case.callee, &arguments)?;
                unreachable!()
            })
            .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
        assert_eq!(error.code, "normalized_reference_type");
    }
    healthy(&case, &schema, true);
}

fn add_external_target(case: &mut test_support::InvokeFixture) -> DeclarationReference {
    let target = add_equal_target(case);
    let source = &mut case.fixture.source;
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(target.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        unreachable!()
    };
    let old_body = function.body;
    let external = ExternalDeclaration {
        type_parameters: function.type_parameters.clone(),
        parameters: function.parameters.clone(),
        result: function.result,
        implementation: ImplementationName::new("core.i64.add").unwrap(),
    };
    owner.header.kind = OwnerKind::External;
    owner.payload = DeclarationPayload::External(external);
    // This exact new local existed solely as the replaced function's body.
    assert!(matches!(
        source.owners.remove(&OwnerKey::Expression(old_body)),
        Some(OwnerRecord::Expression(_))
    ));
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        source.root.owners.content(),
    );
    target
}

#[test]
fn reference_associated_invocations_reject_raw_operation_routes_before_children() {
    for tail in [false, true] {
        let mut case = fixture(tail);
        let external = add_external_target(&mut case);
        let schema = schema(&case);
        healthy(&case, &schema, tail);
        let types = match &expression(&mut case.fixture.source, case.callee).operation {
            ExpressionOperation::FunctionValue { type_arguments, .. } => type_arguments.clone(),
            _ => unreachable!(),
        };
        for operation in [
            // Without route authority this well-typed replacement would return 42,
            // skipping the sealed callee and reversing two existing I64 children.
            ExpressionOperation::Call {
                function: case.target,
                type_arguments: types.clone(),
                effect_arguments: vec![],
                requirement_arguments: vec![],
                arguments: vec![case.second_argument, case.argument],
            },
            ExpressionOperation::FunctionValue {
                function: case.target,
                type_arguments: types.clone(),
                effect_arguments: vec![],
                requirement_arguments: vec![],
            },
            ExpressionOperation::Call {
                function: external,
                type_arguments: vec![],
                effect_arguments: vec![],
                requirement_arguments: vec![],
                arguments: vec![case.second_argument, case.argument],
            },
        ] {
            let mut changed = case.fixture.source.clone();
            expression(&mut changed, case.invocation).operation = operation;
            let reader = FaultedReferenceRead {
                source: &changed,
                schema: Arc::clone(&schema),
            };
            let observed = std::sync::Mutex::new(None);
            let program = program();
            let error = NormalizedReferenceInterpreter::from_reader(
                &reader,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .observing_checked(&observed)
            .execute(None, &ExecutionControl::uncancelled(), |state| {
                state.call_declaration(case.caller, &[], &[], &[], vec![])
            })
            .unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            assert_eq!(error.code, "normalized_reference_type");
            assert_eq!(
                error.message,
                "canonical invocation operation changed before child evaluation"
            );
            let observed = observed.lock().unwrap();
            let observed = observed.as_ref().unwrap();
            // Only the caller body (and normal route's Let body) was read. Neither
            // the FunctionValue nor either argument/host operation ran.
            assert_eq!(observed.expressions, if tail { 1 } else { 2 });
            assert_eq!(observed.external_calls, 0);
            assert_eq!(observed.live_call_frames_after, 0);
            assert_eq!(observed.live_control_frames_after, 0);
            assert_eq!(observed.live_type_scopes_after, 0);
            healthy(&case, &schema, tail);
        }

        // Genuine ordinary external Invoke remains valid on both evaluator
        // routes, and a direct ordinary entry without active context still works.
        let mut external_case = fixture(tail);
        let external = add_external_target(&mut external_case);
        let ExpressionOperation::FunctionValue {
            function,
            type_arguments,
            ..
        } = &mut expression(&mut external_case.fixture.source, external_case.callee).operation
        else {
            unreachable!()
        };
        *function = external;
        type_arguments.clear();
        let external_schema = self::schema(&external_case);
        let (value, observed) = run(
            &external_case,
            &external_case.fixture.source,
            Arc::clone(&external_schema),
        )
        .unwrap();
        assert_eq!(value, NormalizedValue::I64(59));
        assert_eq!(observed.calls, 2);
        assert_eq!(observed.external_calls, 1);
        assert_eq!(observed.live_call_frames_after, 0);
        assert_eq!(observed.live_control_frames_after, 0);
        assert_eq!(observed.live_type_scopes_after, 0);
        let reader = FaultedReferenceRead {
            source: &external_case.fixture.source,
            schema: external_schema,
        };
        let program = program();
        let (value, observed) = NormalizedReferenceInterpreter::from_reader(
            &reader,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            assert!(state.associated_applications.is_empty());
            let arguments = [17, 42]
                .into_iter()
                .map(|value| CheckedValue::primitive(&state.schema, NormalizedValue::I64(value)))
                .collect::<Result<Vec<_>, _>>()?;
            state.call_declaration(external, &[], &[], &[], arguments)
        })
        .unwrap();
        assert_eq!(value, NormalizedValue::I64(59));
        assert_eq!(observed.external_calls, 1);
    }
}

#[test]
fn reference_associated_invocation_seals_bind_route_scope_proof_and_retention_budget() {
    let case = fixture(true);
    let schema = schema(&case);
    let other_schema = self::schema(&case);
    assert!(!Arc::ptr_eq(
        schema.associated_proof().unwrap(),
        other_schema.associated_proof().unwrap()
    ));
    let reader = FaultedReferenceRead {
        source: &case.fixture.source,
        schema: Arc::clone(&schema),
    };
    let program = program();
    let interpreter = NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    );
    let arguments = [case.argument, case.second_argument];
    let types = match &case.fixture.source.owners[&OwnerKey::Expression(case.callee)] {
        OwnerRecord::Expression(record) => match &record.operation {
            ExpressionOperation::FunctionValue { type_arguments, .. } => type_arguments.clone(),
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    for mutation in 0..8 {
        let error = interpreter
            .execute(None, &ExecutionControl::uncancelled(), |state| {
                invocation_context(state, &case);
                let seal = state
                    .admit_canonical_invocation(case.callee, &arguments)?
                    .unwrap();
                let route = match mutation {
                    0 | 7 => ReferenceCallRoute::Direct,
                    1 => ReferenceCallRoute::Invocation(None),
                    _ => ReferenceCallRoute::Invocation(Some(&seal)),
                };
                match mutation {
                    2 => {
                        state.type_occurrence =
                            Some((case.caller.package, OwnerKey::Expression(case.callee)))
                    }
                    3 => {
                        let target = state
                            .schema
                            .associated_proof()
                            .unwrap()
                            .applications
                            .keys()
                            .find(|application| application.function == case.target)
                            .unwrap()
                            .clone();
                        *state.associated_applications.last_mut().unwrap() = Some(target);
                    }
                    4 => {
                        state.schema = Arc::new(BoundReferenceSchema {
                            canonical: Arc::clone(&other_schema),
                            value_origin: state.schema.value_origin,
                        })
                    }
                    5 => {
                        state.associated_applications.pop();
                    }
                    6 | 7 => {
                        state.schema = Arc::new(BoundReferenceSchema {
                            canonical: Arc::new(NormalizedReferenceSchema::default()),
                            value_origin: state.schema.value_origin,
                        })
                    }
                    _ => {}
                }
                state.canonical_application_on_route(
                    case.target,
                    ReferenceApplication {
                        types: &types,
                        effects: &[],
                        requirements: &[],
                        implementations: &[],
                    },
                    route,
                )?;
                unreachable!()
            })
            .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
        assert_eq!(error.code, "normalized_reference_type");
        assert_eq!(
            error.message,
            match mutation {
                0 => "canonical invocation requires its checked evaluator route",
                1 => "canonical invocation route has no checked seal",
                5 => "canonical invocation route lost its active caller",
                6 | 7 => "canonical call route proof is missing",
                _ => "canonical invocation route escaped its checked caller",
            }
        );
    }
    let maximum = crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64;
    // Three child-proof steps precede one full empty-caller-key copy step.
    // The failed clone reservation retains prior read work and no clone storage.
    for remaining in [4, 3] {
        let result = interpreter.execute(None, &ExecutionControl::uncancelled(), |state| {
            invocation_context(state, &case);
            state
                .associated_work
                .store(maximum - remaining, Ordering::Relaxed);
            let before = state.observation.allocated_bytes;
            let result = state.admit_canonical_invocation(case.callee, &arguments);
            assert_eq!(state.associated_work.load(Ordering::Relaxed), maximum);
            if remaining == 3 {
                assert_eq!(state.observation.allocated_bytes, before);
            }
            let seal = result?;
            assert!(seal.is_some());
            drop(seal);
            state.associated_applications.pop();
            state.type_occurrence = None;
            CheckedValue::primitive(&state.schema, NormalizedValue::I64(17))
        });
        if remaining == 4 {
            assert_eq!(result.unwrap().0, NormalizedValue::I64(17));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Resource);
            assert_eq!(error.code, "reference_associated_runtime_work");
        }
    }
    let control = ExecutionControl::uncancelled();
    let error = interpreter
        .execute(None, &control, |state| {
            invocation_context(state, &case);
            let seal = state
                .admit_canonical_invocation(case.callee, &arguments)?
                .unwrap();
            let before = state.associated_work.load(Ordering::Relaxed);
            control.cancel();
            let result = state.canonical_application_on_route(
                case.target,
                ReferenceApplication {
                    types: &types,
                    effects: &[],
                    requirements: &[],
                    implementations: &[],
                },
                ReferenceCallRoute::Invocation(Some(&seal)),
            );
            assert_eq!(state.associated_work.load(Ordering::Relaxed), before);
            result?;
            unreachable!()
        })
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    healthy(&case, &schema, true);
}
