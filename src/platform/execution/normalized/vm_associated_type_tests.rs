//! Private prepared models exercise runtime readers behind the global gate.
//! They do not claim associated artifact or public source admission.
use super::*;
use crate::platform::execution::normalized::{
    prepare::{
        NormalizedCode, NormalizedEntryPoint, NormalizedFunctionBody, NormalizedInstruction,
    },
    prepared_types,
    vm::{NormalizedRunPolicy, NormalizedVm},
};
use crate::platform::kernel::{TypeObject, encode_type_object};
use crate::platform::semantic_id::AssociatedTypeMemberId;
use std::sync::{Arc, Mutex};

struct Fixture {
    program: NormalizedProgram,
    left: FunctionIndex,
    right: FunctionIndex,
    bindings: BTreeMap<TypeParameterId, TypeObjectDigest>,
    projection: TypeObjectDigest,
    sequence: TypeObjectDigest,
    buffer: TypeObjectDigest,
    left_sequence: TypeObjectDigest,
    right_sequence: TypeObjectDigest,
}

fn intern(program: &mut NormalizedProgram, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    program.types.insert(digest, object);
    digest
}

fn prepared_fixture(task: bool) -> Fixture {
    let model = prepared_types::prepared_associated_model();
    let mut program = model.program;
    let sequence = intern(
        &mut program,
        TypeForm::OwnedSequence {
            item: model.projection,
        },
    );
    let scalar = intern(&mut program, TypeForm::I64);
    // These literal expected objects do not come from the materializer's cache.
    let left_sequence =
        encode_type_object(&TypeObject::new(TypeForm::OwnedSequence { item: model.cell }).unwrap())
            .unwrap()
            .0;
    let right_sequence = encode_type_object(
        &TypeObject::new(TypeForm::OwnedSequence { item: model.buffer }).unwrap(),
    )
    .unwrap()
    .0;
    let code = NormalizedCode {
        parameter_count: 0,
        local_count: 0,
        instructions: Arc::from([
            NormalizedInstruction::SequenceEmpty {
                sequence_type: sequence,
            },
            NormalizedInstruction::Drop,
            NormalizedInstruction::I64(7),
            NormalizedInstruction::Return,
        ]),
    };
    for function in Arc::make_mut(&mut program.functions) {
        function.parameter_count = 0;
        function.parameters = Arc::from([]);
        function.result = if task { sequence } else { scalar };
        function.result_borrow = None;
        function.effect = if task {
            FunctionEffect::Task {
                requirements: vec![],
                effect_parameters: vec![],
            }
        } else {
            FunctionEffect::Pure
        };
        function.callsites = Arc::from([]);
        function.body = NormalizedFunctionBody::Code(code.clone());
    }
    // Original fixture entry tables reference its former function inventory.
    program.tests.clear();
    program.ports = Arc::from([]);
    program.targets.clear();
    program.root_target_names.clear();
    let mut entry = program.functions[model.left.0 as usize].clone();
    entry.type_parameters = Arc::from([]);
    entry.type_arguments = Arc::from([]);
    entry.type_parameter_constraints = Arc::from([]);
    entry.implementation_parameters = Arc::from([]);
    entry.implementation_arguments = Arc::from([]);
    entry.result = scalar;
    entry.effect = if task {
        FunctionEffect::Task {
            requirements: vec![],
            effect_parameters: vec![],
        }
    } else {
        FunctionEffect::Pure
    };
    entry.body = NormalizedFunctionBody::Code(NormalizedCode {
        parameter_count: 0,
        local_count: 0,
        instructions: Arc::from([
            NormalizedInstruction::Call {
                function: model.left,
                type_arguments: Arc::clone(
                    &program.functions[model.left.0 as usize].type_arguments,
                ),
                arguments: 0,
                effect_arguments: Arc::from([]),
                requirement_arguments: Arc::from([]),
            },
            NormalizedInstruction::Drop,
            NormalizedInstruction::Call {
                function: model.right,
                type_arguments: Arc::clone(
                    &program.functions[model.right.0 as usize].type_arguments,
                ),
                arguments: 0,
                effect_arguments: Arc::from([]),
                requirement_arguments: Arc::from([]),
            },
            NormalizedInstruction::Drop,
            NormalizedInstruction::I64(7),
            NormalizedInstruction::Return,
        ]),
    });
    let mut functions = program.functions.to_vec();
    functions.push(entry);
    program.functions = functions.into();
    prepared_types::complete(&mut program).unwrap();
    prepared_types::share_code(&mut program, &ExecutionControl::uncancelled()).unwrap();
    Fixture {
        program,
        left: model.left,
        right: model.right,
        bindings: model.bindings,
        projection: model.projection,
        sequence,
        buffer: model.buffer,
        left_sequence,
        right_sequence,
    }
}

fn resolve_fixture(
    fixture: &Fixture,
    function: FunctionIndex,
    control: &ExecutionControl,
) -> Result<TypeObjectDigest, ExecutionError> {
    let context =
        PreparedApplicationContext::new(&fixture.program, function, &fixture.bindings, control)
            .map_err(context_error)?;
    resolve_application(&fixture.program, fixture.sequence, &context, control)
}

fn mutate_selected(
    fixture: &mut Fixture,
    mutate: impl FnOnce(&mut super::super::super::super::prepare::NormalizedImplementationApplication),
) {
    let identity = fixture.program.functions[fixture.left.0 as usize].implementation_arguments[0]
        .identity as usize;
    let mut node = (*fixture.program.implementation_applications[identity]).clone();
    mutate(&mut node);
    let node = Arc::new(node);
    Arc::make_mut(&mut fixture.program.implementation_applications)[identity] = Arc::clone(&node);
    Arc::make_mut(&mut fixture.program.functions)[fixture.left.0 as usize]
        .implementation_arguments = Arc::from([node]);
}

#[test]
fn associated_runtime_hits_bind_exact_witness_application_and_recheck_raw_rhs() {
    let mut fixture = prepared_fixture(false);
    let control = ExecutionControl::uncancelled();
    assert_eq!(
        resolve_fixture(&fixture, fixture.left, &control).unwrap(),
        fixture.left_sequence
    );
    assert_eq!(
        resolve_fixture(&fixture, fixture.right, &control).unwrap(),
        fixture.right_sequence
    );
    assert_ne!(fixture.left_sequence, fixture.right_sequence);
    assert_ne!(
        fixture.program.functions[0].implementation_arguments[0].identity,
        fixture.program.functions[1].implementation_arguments[0].identity
    );
    assert!(
        PreparedApplicationContext::new(&fixture.program, fixture.left, &BTreeMap::new(), &control)
            .is_err()
    );
    assert!(
        resolve(
            &fixture.program,
            fixture.sequence,
            &fixture.bindings,
            &control
        )
        .is_err()
    );
    let retained = fixture.program.types.len();
    let prepared_work = fixture.program.work.type_derivation_steps;
    for _ in 0..3 {
        assert_eq!(
            resolve_fixture(&fixture, fixture.left, &control).unwrap(),
            fixture.left_sequence
        );
    }
    assert_eq!(fixture.program.types.len(), retained);
    assert_eq!(fixture.program.work.type_derivation_steps, prepared_work);
    fixture.program.prepared_type_lookup.corrupt_application(
        fixture.left,
        fixture.sequence,
        &fixture.bindings,
        fixture.right_sequence,
    );
    assert!(resolve_fixture(&fixture, fixture.left, &control).is_err());
    assert_eq!(
        resolve_fixture(&fixture, fixture.right, &control).unwrap(),
        fixture.right_sequence
    );
}

#[test]
fn associated_runtime_refuses_each_independent_member_scope_and_origin_corruption() {
    let control = ExecutionControl::uncancelled();
    for fault in 0..8 {
        let mut fixture = prepared_fixture(false);
        let buffer = fixture.buffer;
        let projection = fixture.projection;
        match fault {
            0 => mutate_selected(&mut fixture, |node| {
                Arc::make_mut(&mut node.associated_types)[1].ty = buffer
            }),
            1 => mutate_selected(&mut fixture, |node| {
                Arc::make_mut(&mut node.associated_types)[1].template = buffer
            }),
            2 => mutate_selected(&mut fixture, |node| {
                Arc::make_mut(&mut node.associated_types)[1].template = projection
            }),
            3 => mutate_selected(&mut fixture, |node| {
                Arc::make_mut(&mut node.associated_types)[1].member =
                    AssociatedTypeMemberId::migrate(b"foreign-runtime-member", 0)
            }),
            4 => mutate_selected(&mut fixture, |node| node.contract = node.implementation),
            5 => mutate_selected(&mut fixture, |node| node.implementation_generation -= 1),
            6 => {
                let foreign = fixture.program.functions[fixture.left.0 as usize]
                    .implementation_arguments[0]
                    .implementation;
                Arc::make_mut(&mut fixture.program.functions)[fixture.left.0 as usize]
                    .declaration = foreign;
            }
            _ => {
                fixture.program.value_origin =
                    super::super::super::super::value::ValueOrigin::fresh().unwrap()
            }
        }
        assert!(
            resolve_fixture(&fixture, fixture.left, &control).is_err(),
            "fault {fault}"
        );
    }
}

#[test]
fn associated_runtime_hit_work_bound_and_cancellation_preserve_healthy_retry() {
    let fixture = prepared_fixture(false);
    let healthy = ExecutionControl::uncancelled();
    let context = PreparedApplicationContext::new(
        &fixture.program,
        fixture.left,
        &fixture.bindings,
        &healthy,
    )
    .unwrap();
    let mut measured = Work {
        control: &healthy,
        nodes: 0,
    };
    assert_eq!(
        resolve_application_in(&fixture.program, fixture.sequence, &context, &mut measured)
            .unwrap(),
        fixture.left_sequence
    );
    assert!(measured.nodes > 0);
    for short in [false, true] {
        let mut work = Work {
            control: &healthy,
            nodes: MAXIMUM_ADMISSION_ITEMS - measured.nodes + u64::from(short),
        };
        let result =
            resolve_application_in(&fixture.program, fixture.sequence, &context, &mut work);
        if short {
            assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Resource);
        } else {
            assert_eq!(result.unwrap(), fixture.left_sequence);
            assert_eq!(work.nodes, MAXIMUM_ADMISSION_ITEMS);
        }
    }
    let mut refused = 0;
    let mut accepted = 0;
    for checks in 0..64 {
        match resolve_fixture(
            &fixture,
            fixture.left,
            &ExecutionControl::cancel_after_checks(checks),
        ) {
            Ok(actual) => {
                accepted += 1;
                assert_eq!(actual, fixture.left_sequence);
            }
            Err(error) => {
                refused += 1;
                assert_eq!(error.class, ExecutionFailureClass::Cancelled);
            }
        }
        assert_eq!(
            resolve_fixture(&fixture, fixture.left, &healthy).unwrap(),
            fixture.left_sequence
        );
    }
    assert!(refused > 0 && accepted > 0);
}

#[test]
fn associated_runtime_shared_instruction_body_uses_each_active_function_application() {
    let fixture = prepared_fixture(false);
    let storage =
        crate::platform::execution::normalized::owned_product::StorageObservation::start();
    let (NormalizedFunctionBody::Code(left), NormalizedFunctionBody::Code(right)) = (
        &fixture.program.functions[0].body,
        &fixture.program.functions[1].body,
    ) else {
        panic!("shared code");
    };
    assert!(Arc::ptr_eq(&left.instructions, &right.instructions));
    let observations = Mutex::new(None);
    let result = NormalizedVm::for_test(&fixture.program, NormalizedRunPolicy::default())
        .observing_checked(&observations)
        .invoke_entry(
            NormalizedEntryPoint::Function(FunctionIndex(2, fixture.program.value_origin)),
            vec![],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(result.0, NormalizedValue::I64(7));
    let observation = observations.into_inner().unwrap().unwrap();
    assert_eq!(observation.live_call_frames_after, 0);
    assert_eq!(observation.live_operands_after, 0);
    assert_eq!(observation.live_handles_after, 0);
    assert_eq!(storage.created(), 2);
    assert_eq!(storage.live(), (0, 0));
}

#[test]
fn associated_task_certificate_revalidates_member_and_signature_before_adoption() {
    let mut fixture = prepared_fixture(true);
    let control = ExecutionControl::uncancelled();
    let types = Arc::clone(&fixture.program.functions[0].type_arguments);
    let application = application::TaskApplication::bind(
        &fixture.program,
        fixture.left,
        types,
        &control,
        &mut |_| Ok(()),
    )
    .unwrap();
    assert_eq!(application.result(), fixture.left_sequence);
    assert!(application.matches(&fixture.program, &control).unwrap());
    assert_eq!(
        application
            .matches(&fixture.program, &ExecutionControl::cancel_after_checks(0))
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    assert!(application.matches(&fixture.program, &control).unwrap());
    let projection = fixture.projection;
    mutate_selected(&mut fixture, |node| {
        Arc::make_mut(&mut node.associated_types)[1].template = projection
    });
    assert!(
        !application
            .matches(&fixture.program, &control)
            .unwrap_or(false)
    );
    let types = Arc::clone(&fixture.program.functions[0].type_arguments);
    assert!(
        application::TaskApplication::bind(
            &fixture.program,
            fixture.left,
            types,
            &control,
            &mut |_| Ok(())
        )
        .is_err()
    );

    let mut fixture = prepared_fixture(true);
    let projection = fixture.projection;
    // This member is unused by the signature and body; complete witness
    // admission must still reject a producer's malformed stored RHS.
    mutate_selected(&mut fixture, |node| {
        Arc::make_mut(&mut node.associated_types)[0].template = projection
    });
    let types = Arc::clone(&fixture.program.functions[0].type_arguments);
    assert!(
        application::TaskApplication::bind(
            &fixture.program,
            fixture.left,
            types,
            &control,
            &mut |_| Ok(())
        )
        .is_err()
    );
}
