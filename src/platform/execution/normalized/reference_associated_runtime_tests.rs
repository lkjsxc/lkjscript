//! Private canonical-context consumer tests; source/artifact admission is proved separately.
use super::super::reference_types::associated_preparation::{Limits, test_support};
use super::super::tests::FaultedReferenceRead;
use super::*;
use crate::platform::publication::GraphRepository;
use std::sync::OnceLock;

fn program() -> Arc<NormalizedProgram> {
    static PROGRAM: OnceLock<Arc<NormalizedProgram>> = OnceLock::new();
    Arc::clone(PROGRAM.get_or_init(|| {
        // Only the original ordinary source enters public preparation. The
        // resulting program supplies a physical value origin, never expected types.
        let source = test_support::base_source();
        let directory = tempfile::tempdir().unwrap();
        let repository = GraphRepository::create(
            &directory.path().join("ordinary-reference-context"),
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

fn fixture() -> (test_support::Fixture, Arc<NormalizedReferenceSchema>) {
    fixture_at_generation(crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION)
}

// Synthetic canonical-reader matrix; this private helper provides no retained
// transport or artifact acceptance evidence.
fn fixture_at_generation(
    generation: u16,
) -> (test_support::Fixture, Arc<NormalizedReferenceSchema>) {
    let mut fixture = test_support::make_fixture();
    fixture.source.root.graph_contract_version = generation;
    for owner in fixture.source.owners.values_mut() {
        owner.set_encoding_for_edit(generation);
    }
    let control = ExecutionControl::uncancelled();
    let proof = test_support::prove(&fixture, &[0, 1, 2], Limits::default(), &control).unwrap();
    let schema = super::super::reference_types::associated_schema_fixture(
        &[&fixture.source],
        proof,
        &control,
    )
    .unwrap();
    (fixture, Arc::new(schema))
}

fn owner(
    state: &mut ReferenceState<'_>,
    ty: TypeObjectDigest,
) -> Result<CheckedValue, ExecutionError> {
    let raw = match state.schema.types.get(&ty).unwrap().form {
        TypeForm::OwnedI64Cell => {
            let value = super::super::owned_i64_cell::OwnedI64Cell::new(state.memory_domain, 41);
            value.establish_admission(state.schema.value_origin)?;
            NormalizedValue::OwnedI64Cell(value)
        }
        TypeForm::OwnedSequence { .. } => {
            let value = super::super::owned_sequence::OwnedSequence::create(
                state.memory_domain,
                ty,
                state.control,
                &mut |_| Ok(()),
            )?;
            value.establish_admission(state.schema.value_origin)?;
            NormalizedValue::OwnedSequence(value)
        }
        _ => unreachable!(),
    };
    CheckedValue::memory(&state.schema, raw)
}

fn read_application(
    state: &mut ReferenceState<'_>,
    fixture: &test_support::Fixture,
    selected: usize,
    actual: TypeObjectDigest,
) -> Result<CheckedValue, ExecutionError> {
    let owner = owner(state, actual)?;
    let argument = owner.duplicate(ParameterUse::Borrow)?;
    let selected = state.resolve_implementation(test_support::operand(fixture, selected))?;
    let admitted = state.admit_resolved_witness_call(
        fixture.consumer,
        &[fixture.cell],
        &[],
        &[],
        &[selected],
        vec![argument],
    )?;
    let value = state.execute_witness_call(admitted)?;
    assert!(value.raw().memory_is_borrowed());
    let observed = match value.raw() {
        NormalizedValue::OwnedI64Cell(cell) => cell.read()?,
        NormalizedValue::OwnedSequence(sequence) => {
            sequence.len(state.memory_domain, state.control)? as i64
        }
        _ => unreachable!(),
    };
    drop(value);
    // Returning and dropping the view must release the exact source loan.
    owner.raw().memory_validate(state.memory_domain, true)?;
    drop(owner);
    CheckedValue::primitive(&state.schema, NormalizedValue::I64(observed))
}

#[test]
fn reference_associated_runtime_reads_distinct_witness_outputs_and_releases_source_loans() {
    let (fixture, schema) = fixture();
    // The witness helper bypasses full source loading. A completed canonical schema is
    // independent metadata, not an associated artifact seal or this fixture's value origin.
    let completed =
        NormalizedReferenceSchema::independently_admit_witness_source(&fixture.source).unwrap();
    let scoped = &completed.associated_proof().unwrap().symbolic[&fixture.consumer];
    assert_eq!(
        scoped.parameters,
        [(fixture.projection, ParameterUse::Borrow)]
    );
    assert_eq!(scoped.result.ty, fixture.projection);
    assert!(!completed.ordinary_types.contains(&fixture.projection));
    assert!(!completed.transferable_types.contains(&fixture.projection));
    assert!(!completed.shareable_types.contains(&fixture.projection));
    let reader = FaultedReferenceRead {
        source: &fixture.source,
        schema,
    };
    let program = program();
    let interpreter = NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    );
    for (selected, ty, expected) in [
        (0, fixture.cell, 41),
        (1, fixture.block, 0),
        (2, fixture.cell, 41),
        (0, fixture.cell, 41),
    ] {
        let (value, observed) = interpreter
            .execute(None, &ExecutionControl::uncancelled(), |state| {
                read_application(state, &fixture, selected, ty)
            })
            .unwrap();
        assert_eq!(value, NormalizedValue::I64(expected));
        assert_eq!(observed.live_call_frames_after, 0);
        assert_eq!(observed.live_control_frames_after, 0);
        assert_eq!(observed.live_type_scopes_after, 0);
    }
}

#[test]
fn reference_associated_runtime_refuses_equal_self_with_another_member_output() {
    let (fixture, schema) = fixture();
    let reader = FaultedReferenceRead {
        source: &fixture.source,
        schema,
    };
    let program = program();
    let interpreter = NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    );
    let observed = super::super::owned_i64_cell::StorageObservation::start();
    let error = interpreter
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            read_application(state, &fixture, 1, fixture.cell)
        })
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(observed.live(), (0, 0));
    interpreter
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            read_application(state, &fixture, 0, fixture.cell)
        })
        .unwrap();
    assert_eq!(observed.live(), (0, 0));
}

#[test]
fn reference_associated_runtime_rechecks_parameter_and_local_generations_on_context_hits() {
    for generation in [
        crate::platform::kernel::contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION,
        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
    ] {
        let (fixture, schema) = fixture_at_generation(generation);
        let OwnerRecord::Declaration(declaration) =
            &fixture.source.owners[&OwnerKey::Declaration(fixture.consumer.declaration)]
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &declaration.payload else {
            unreachable!()
        };
        let keys = [
            OwnerKey::Parameter(function.parameters[0]),
            OwnerKey::Expression(function.body),
        ];
        let program = program();
        let healthy = FaultedReferenceRead {
            source: &fixture.source,
            schema: Arc::clone(&schema),
        };
        NormalizedReferenceInterpreter::from_reader(
            &healthy,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            read_application(state, &fixture, 0, fixture.cell)
        })
        .unwrap();
        for key in keys {
            for changed_generation in [
                32,
                crate::platform::kernel::contract::MIXED_STORAGE_GRAPH_CONTRACT_VERSION,
                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
                crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION + 1,
            ] {
                if changed_generation == generation {
                    continue;
                }
                let mut source = fixture.source.clone();
                source
                    .owners
                    .get_mut(&key)
                    .unwrap()
                    .set_encoding_for_edit(changed_generation);
                let reader = FaultedReferenceRead {
                    source: &source,
                    schema: Arc::clone(&schema),
                };
                let interpreter = NormalizedReferenceInterpreter::from_reader(
                    &reader,
                    &program,
                    NormalizedRunPolicy::foreground(),
                );
                let error = interpreter
                    .execute(None, &ExecutionControl::uncancelled(), |state| {
                        read_application(state, &fixture, 0, fixture.cell)
                    })
                    .unwrap_err();
                assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            }
        }
        for mutation in 0..3 {
            let mut source = fixture.source.clone();
            let OwnerRecord::Declaration(owner) = source
                .owners
                .get_mut(&OwnerKey::Declaration(fixture.consumer.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::Function(function) = &mut owner.payload else {
                unreachable!()
            };
            let formal = &mut function.implementation_parameters[0];
            match mutation {
                0 => {
                    formal.id = crate::platform::semantic_id::ImplementationParameterId::migrate(
                        b"reference-runtime-foreign-formal",
                        0,
                    )
                }
                1 => formal.self_type = fixture.block,
                2 => formal.type_arguments.push(fixture.cell),
                _ => unreachable!(),
            }
            let reader = FaultedReferenceRead {
                source: &source,
                schema: Arc::clone(&schema),
            };
            let error = NormalizedReferenceInterpreter::from_reader(
                &reader,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .execute(None, &ExecutionControl::uncancelled(), |state| {
                read_application(state, &fixture, 0, fixture.cell)
            })
            .unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            assert_eq!(error.code, "normalized_reference_type");
            assert!(error.message.contains("ordered witness formal templates"));
        }
    }
}

#[test]
fn reference_associated_runtime_rechecks_selected_mapping_template_before_dispatch() {
    let (fixture, schema) = fixture();
    let program = program();
    for mutation in 0..4 {
        let mut source = fixture.source.clone();
        let OwnerRecord::Declaration(declaration) = source
            .owners
            .get_mut(&OwnerKey::Declaration(
                fixture.implementations[0].declaration,
            ))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(implementation) = &mut declaration.payload
        else {
            unreachable!()
        };
        let method = implementation.methods[0].method;
        match mutation {
            0 => {}
            1 => implementation.methods[0].type_arguments[0] = fixture.block,
            2 => implementation.methods.clear(),
            3 => implementation
                .methods
                .push(implementation.methods[0].clone()),
            _ => unreachable!(),
        }
        let reader = FaultedReferenceRead {
            source: &source,
            schema: Arc::clone(&schema),
        };
        let interpreter = NormalizedReferenceInterpreter::from_reader(
            &reader,
            &program,
            NormalizedRunPolicy::foreground(),
        );
        let result = interpreter.execute(None, &ExecutionControl::uncancelled(), |state| {
            let (target, types, _) = state.method_target(
                test_support::operand(&fixture, 0),
                fixture.contract,
                method,
            )?;
            assert_eq!(target, fixture.reader);
            assert_eq!(types, [fixture.cell]);
            CheckedValue::primitive(&state.schema, NormalizedValue::I64(1))
        });
        assert_eq!(result.is_err(), mutation != 0);
        let result = interpreter.execute(None, &ExecutionControl::uncancelled(), |state| {
            // The consumer only returns its local view; no method dispatch can
            // hide a changed, missing or duplicate unused mapping.
            read_application(state, &fixture, 0, fixture.cell)
        });
        if mutation == 0 {
            assert_eq!(result.unwrap().0, NormalizedValue::I64(41));
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
            assert_eq!(error.code, "normalized_reference_type");
        }
    }
}

#[test]
fn reference_associated_runtime_preserves_capacity_and_cancelled_cache_hits() {
    let (fixture, schema) = fixture();
    let reader = FaultedReferenceRead {
        source: &fixture.source,
        schema,
    };
    let program = program();
    let interpreter = NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    );
    let error = interpreter
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            state.associated_work.store(
                crate::platform::kernel::contract::MAXIMUM_VALIDATION_WORK as u64,
                std::sync::atomic::Ordering::Relaxed,
            );
            read_application(state, &fixture, 0, fixture.cell)
        })
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Resource);
    assert_eq!(error.code, "reference_associated_runtime_work");
    let control = ExecutionControl::uncancelled();
    let error = interpreter
        .execute(None, &control, |state| {
            state.resolve_implementation(test_support::operand(&fixture, 0))?;
            control.cancel();
            state.resolve_implementation(test_support::operand(&fixture, 0))?;
            unreachable!()
        })
        .unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
    interpreter
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            read_application(state, &fixture, 0, fixture.cell)
        })
        .unwrap();
}

#[test]
fn reference_associated_runtime_retains_uncalled_ordinary_generic_witness_path() {
    let fixture = test_support::make_fixture();
    let mut source = test_support::base_source();
    source
        .types
        .insert(fixture.cell, fixture.source.types[&fixture.cell].clone());
    let named = |name: &str| {
        source
            .owners
            .iter()
            .find_map(|(key, owner)| match (key, owner) {
                (OwnerKey::Declaration(id), OwnerRecord::Declaration(owner))
                    if owner.name.as_str() == name =>
                {
                    Some(DeclarationReference {
                        package: source.root.package_id,
                        declaration: *id,
                    })
                }
                _ => None,
            })
            .unwrap()
    };
    let consumer = named("consumer");
    let implementation = named("Cell");
    // Keep the original consumer and Cell generic and uncalled. An unrelated
    // ordinary ground implementation makes the concrete owner type reachable.
    let OwnerRecord::Declaration(mut ground) =
        source.owners[&OwnerKey::Declaration(implementation.declaration)].clone()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(value) = &mut ground.payload else {
        unreachable!()
    };
    value.type_parameters.clear();
    value.self_type = fixture.cell;
    value.methods[0].type_arguments = vec![fixture.cell];
    let ground_id = crate::platform::semantic_id::DeclarationId::migrate(
        b"reference-runtime-ordinary-ground-cell",
        0,
    );
    ground.header = crate::platform::kernel::OwnerHeader::new(
        OwnerKey::Declaration(ground_id),
        crate::platform::kernel::OwnerKind::OwnedImplementation,
    );
    ground.name = crate::platform::kernel::Name::new("GroundCell").unwrap();
    source.owners.insert(
        OwnerKey::Declaration(ground_id),
        OwnerRecord::Declaration(ground),
    );
    source.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        source.root.owners.page(),
        source.owners.len() as u64,
        source.root.owners.content(),
    );
    let schema = Arc::new(NormalizedReferenceSchema::reconstruct([&source]).unwrap());
    assert!(schema.associated_proof().is_none());
    let reader = FaultedReferenceRead {
        source: &source,
        schema,
    };
    let program = program();
    let interpreter = NormalizedReferenceInterpreter::from_reader(
        &reader,
        &program,
        NormalizedRunPolicy::foreground(),
    );
    let (value, _) = interpreter
        .execute(None, &ExecutionControl::uncancelled(), |state| {
            let owner = owner(state, fixture.cell)?;
            let selected = state.resolve_implementation(
                crate::platform::kernel::ImplementationOperand::Concrete {
                    implementation,
                    type_arguments: vec![fixture.cell],
                    implementations: vec![],
                },
            )?;
            assert!(selected.canonical.is_none());
            let target = state.admit_resolved_witness_call(
                consumer,
                &[fixture.cell],
                &[],
                &[],
                &[selected],
                vec![owner.duplicate(ParameterUse::Borrow)?],
            )?;
            assert!(target.associated.is_none());
            let value = state.execute_witness_call(target)?;
            drop(value);
            owner.raw().memory_validate(state.memory_domain, true)?;
            CheckedValue::primitive(&state.schema, NormalizedValue::I64(41))
        })
        .unwrap();
    assert_eq!(value, NormalizedValue::I64(41));
}
