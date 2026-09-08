//! Source-bound admission and corruption negatives; public authoring remains in pure-tail.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::super::super::{
    reference::{NormalizedReferenceHost, NormalizedReferenceInterpreter, ReferenceSignature},
    reference_schema::NormalizedReferenceSchema,
    value_oracle::{self, Affinity},
    value_schema::NormalizedValueSchema,
};
use super::*;
use crate::platform::compiler::load_artifact;
use crate::platform::kernel::{KernelSnapshot, TypeObject, encode_type_object};
use crate::platform::publication::GraphRepository;
use std::sync::atomic::{AtomicU64, Ordering};

fn fixture() -> (NormalizedProgram, KernelSnapshot) {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("packages/standard");
    let program = NormalizedProgram::prepare(
        load_artifact(
            &std::fs::read(root.join("generated/standard.lkja"))
                .expect("retained standard artifact"),
        )
        .expect("strict artifact"),
    )
    .expect("prepared standard");
    let repository = GraphRepository::open(&root).expect("standard authority");
    let snapshot = repository
        .view_current()
        .expect("exact current revision")
        .reconstruct_full_oracle()
        .expect("independent canonical owners")
        .value;
    (program, snapshot)
}

#[test]
fn raw_bound_callables_require_exact_environments_and_bounded_admission() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let boolean = internal_type(&mut program, &mut schema, TypeForm::Bool);
    let static_text = internal_type(&mut program, &mut schema, TypeForm::StaticText);
    let secret = internal_type(&mut program, &mut schema, TypeForm::Secret);
    let result = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Result {
            ok: integer,
            error: static_text,
        },
    );
    let unsafe_result = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Result {
            ok: integer,
            error: secret,
        },
    );
    let thunk = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Function {
            parameters: vec![],
            result: integer,
        },
    );
    let unary = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Function {
            parameters: vec![integer],
            result: integer,
        },
    );
    let wrong_suffix = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Function {
            parameters: vec![boolean],
            result: integer,
        },
    );
    snapshot.types = schema.types.clone();
    let find = |name: &str| {
        program.functions.iter().enumerate().find(|(_, function)|
        function.type_parameters.len() == 1 && matches!(&function.body, super::super::super::prepare::NormalizedFunctionBody::External(actual) if actual.as_str() == name))
        .map(|(index, function)| (FunctionIndex(u32::try_from(index).unwrap(), program.value_origin), function.declaration)).unwrap()
    };
    let (length, length_declaration) = find("core.list.length");
    let length_closure = |ty, elements| NormalizedValue::Function {
        function: length,
        type_arguments: Arc::from([ty]),
        bound_arguments: Some(Arc::new(vec![NormalizedValue::List(Arc::new(elements))])),
    };
    let (affine_index, affine_variant) = program
        .variants
        .iter()
        .enumerate()
        .find(|(index, _)| program.affine_variants[*index])
        .unwrap();
    let affine_type = program.types.iter().find_map(|(ty, object)| matches!(&object.form, TypeForm::Named { declaration } if *declaration == affine_variant.declaration).then_some(*ty)).unwrap();
    let absent_case = affine_variant
        .cases
        .iter()
        .position(|case| case.payload.is_none())
        .unwrap();
    let resource_scope = NormalizedResourceScope::new().unwrap();
    let interface = program
        .types
        .values()
        .find_map(|ty| match ty.form {
            TypeForm::CapabilityResource { interface } => Some(interface),
            _ => None,
        })
        .unwrap();
    let handle = resource_scope
        .reserve_queue_lease(
            RequirementReference {
                package: program.root_package,
                requirement: crate::platform::semantic_id::RequirementId::migrate(
                    b"bound-raw-negative",
                    0,
                ),
            },
            interface,
        )
        .unwrap()
        .commit(crate::platform::queue::JobLease {
            job_id: "owned-binding-test".to_owned(),
            attempt_id: "attempt".to_owned(),
            worker_id: "worker".to_owned(),
            payload: vec![],
            attempt_number: 1,
            lease_until_milliseconds: 1,
        })
        .unwrap();
    let add = program
        .functions
        .iter()
        .enumerate()
        .find(|(_, function)| {
            function.type_parameters.is_empty()
                && function.parameters.len() == 2
                && function.result == integer
                && function
                    .parameters
                    .iter()
                    .all(|parameter| parameter.ty == integer)
                && matches!(
                    &function.body,
                    super::super::super::prepare::NormalizedFunctionBody::External(_)
                )
        })
        .map(|(index, _)| FunctionIndex(u32::try_from(index).unwrap(), program.value_origin))
        .unwrap();
    let helper = program
        .functions
        .iter()
        .enumerate()
        .find(|(_, function)| {
            function.type_parameters.len() == 3
                && function.parameters.len() == 3
                && function.pure_graph
        })
        .map(|(index, _)| FunctionIndex(u32::try_from(index).unwrap(), program.value_origin))
        .unwrap();
    let leaf = || NormalizedValue::Function {
        function: add,
        type_arguments: Arc::from([]),
        bound_arguments: Some(Arc::new(vec![NormalizedValue::I64(3)])),
    };
    let nested = |count| {
        let mut value = leaf();
        for _ in 0..count {
            value = NormalizedValue::Function {
                function: helper,
                type_arguments: Arc::from([integer, integer, integer]),
                bound_arguments: Some(Arc::new(vec![value, leaf()])),
            };
        }
        value
    };
    let invoke = |reference: bool, ty, value, policy, control: &ExecutionControl| {
        let arguments = vec![NormalizedValue::List(Arc::new(vec![value]))];
        if reference {
            let sink = std::sync::Mutex::new(None);
            let result = NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                .observing(
                    &sink,
                    &super::super::super::reference::CoreNormalizedReferenceHost,
                )
                .invoke_instantiated(length_declaration, &[ty], arguments, control)
                .map(|(value, _)| value);
            let observation = sink.into_inner().unwrap().unwrap();
            assert_eq!(
                observation.live_call_frames_after
                    + observation.live_control_frames_after
                    + observation.live_local_scopes_after
                    + observation.live_type_scopes_after
                    + observation.live_transactions_after
                    + observation.live_handles_after,
                0
            );
            (
                result,
                observation.value_work,
                observation.allocated_bytes,
                observation.collection_items,
            )
        } else {
            let sink = std::sync::Mutex::new(None);
            let result = super::super::NormalizedVm::new(&program, policy)
                .observing(&sink, &super::super::CoreNormalizedHost)
                .invoke_entry(
                    super::super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                        length,
                        Arc::from([ty]),
                    ),
                    arguments,
                    None,
                    control,
                )
                .map(|(value, _)| value);
            let observation = sink.into_inner().unwrap().unwrap();
            assert_eq!(
                observation.live_call_frames_after
                    + observation.live_locals_after
                    + observation.live_type_bindings_after
                    + observation.live_operands_after
                    + observation.live_transactions_after
                    + observation.live_handles_after,
                0
            );
            (
                result,
                observation.value_work,
                observation.allocated_bytes,
                observation.collection_items,
            )
        }
    };
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        for (ty, raw) in [
            (
                static_text,
                NormalizedValue::StaticText(Arc::from("retained")),
            ),
            (
                result,
                NormalizedValue::Result {
                    success: true,
                    value: Box::new(NormalizedValue::I64(42)),
                },
            ),
            (
                result,
                NormalizedValue::Result {
                    success: false,
                    value: Box::new(NormalizedValue::StaticText(Arc::from("error"))),
                },
            ),
        ] {
            assert_eq!(
                invoke(
                    reference,
                    thunk,
                    length_closure(ty, vec![raw]),
                    Default::default(),
                    &control
                )
                .0
                .unwrap(),
                NormalizedValue::I64(1)
            );
        }
        let mut hidden = vec![NormalizedValue::I64(1); 128];
        hidden.push(NormalizedValue::Resource(handle));
        for (name, raw) in [
            (
                "absent-result-secret",
                length_closure(
                    unsafe_result,
                    vec![NormalizedValue::Result {
                        success: true,
                        value: Box::new(NormalizedValue::I64(1)),
                    }],
                ),
            ),
            (
                "absent-affine-variant",
                length_closure(
                    affine_type,
                    vec![NormalizedValue::Variant {
                        layout: VariantLayoutIndex(
                            u32::try_from(affine_index).unwrap(),
                            program.value_origin,
                        ),
                        case: u32::try_from(absent_case).unwrap(),
                        payload: None,
                    }],
                ),
            ),
            (
                "empty-list-affine-variant",
                length_closure(affine_type, vec![]),
            ),
            ("hidden-last-resource", length_closure(integer, hidden)),
        ] {
            let rejected = invoke(reference, thunk, raw, Default::default(), &control);
            assert_eq!(
                rejected.0.unwrap_err().code,
                if reference {
                    "normalized_reference_value_admission"
                } else {
                    "normalized_value_admission"
                },
                "{name}"
            );
            if name == "hidden-last-resource" {
                assert!(rejected.1.capture_admission_nodes > 128);
            }
            assert_eq!(
                resource_scope.live_resources(),
                1,
                "foreign ingress cannot acquire or release unrelated authority"
            );
        }
        let good = invoke(reference, unary, leaf(), Default::default(), &control);
        assert_eq!(good.0.unwrap(), NormalizedValue::I64(1));
        assert!(good.1.capture_admission_nodes > 0);
        assert_eq!(good.1.internal_guard_descendant_visits, 0);
        let malformed = [
            (
                "wrong-prefix",
                unary,
                NormalizedValue::Function {
                    function: add,
                    type_arguments: Arc::from([]),
                    bound_arguments: Some(Arc::new(vec![NormalizedValue::Bool(true)])),
                },
            ),
            (
                "too-long-prefix",
                unary,
                NormalizedValue::Function {
                    function: add,
                    type_arguments: Arc::from([]),
                    bound_arguments: Some(Arc::new(vec![NormalizedValue::I64(0); 3])),
                },
            ),
            (
                "noncanonical-empty",
                unary,
                NormalizedValue::Function {
                    function: add,
                    type_arguments: Arc::from([]),
                    bound_arguments: Some(Arc::new(vec![])),
                },
            ),
            (
                "unexpected-substitution",
                unary,
                NormalizedValue::Function {
                    function: add,
                    type_arguments: Arc::from([integer]),
                    bound_arguments: Some(Arc::new(vec![NormalizedValue::I64(3)])),
                },
            ),
            ("wrong-suffix", wrong_suffix, leaf()),
            (
                "absent-target",
                unary,
                NormalizedValue::Function {
                    function: FunctionIndex(u32::MAX, program.value_origin),
                    type_arguments: Arc::from([]),
                    bound_arguments: None,
                },
            ),
            (
                "foreign-origin",
                unary,
                NormalizedValue::Function {
                    function: FunctionIndex(add.0, ValueOrigin::default()),
                    type_arguments: Arc::from([]),
                    bound_arguments: Some(Arc::new(vec![NormalizedValue::I64(3)])),
                },
            ),
        ];
        for (name, ty, value) in malformed {
            let error = invoke(reference, ty, value, Default::default(), &control)
                .0
                .unwrap_err();
            assert_eq!(
                error.code,
                if reference {
                    "normalized_reference_value_admission"
                } else {
                    "normalized_value_admission"
                },
                "{name}"
            );
        }
        for (depth, accepted) in [(254, true), (255, false), (10_000, false)] {
            let result = invoke(
                reference,
                unary,
                nested(depth),
                Default::default(),
                &control,
            )
            .0;
            assert_eq!(
                result.is_ok(),
                accepted,
                "nested environment {depth}, reference {reference}"
            );
            if let Err(error) = result {
                assert_eq!(
                    error.code,
                    if reference {
                        "normalized_reference_value_depth"
                    } else {
                        "normalized_value_depth"
                    }
                );
            }
        }
        let measured = invoke(reference, unary, nested(8), Default::default(), &control);
        assert_eq!(measured.0.unwrap(), NormalizedValue::I64(1));
        for (bytes, items, accepted) in [
            (measured.2, measured.3, true),
            (measured.2 - 1, measured.3, false),
            (measured.2, measured.3 - 1, false),
        ] {
            let result = invoke(
                reference,
                unary,
                nested(8),
                super::super::NormalizedRunPolicy {
                    maximum_allocated_bytes: bytes,
                    maximum_collection_items: items,
                    ..Default::default()
                },
                &control,
            )
            .0;
            assert_eq!(result.is_ok(), accepted);
            if let Err(error) = result {
                assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Resource
                );
            }
        }
        let cancelled = invoke(
            reference,
            unary,
            nested(128),
            Default::default(),
            &ExecutionControl::cancel_after_checks(37),
        );
        assert_eq!(cancelled.0.unwrap_err().code, "execution_cancelled");
        assert!(cancelled.1.capture_admission_nodes > 0);
        assert_eq!(
            invoke(reference, unary, leaf(), Default::default(), &control)
                .0
                .unwrap(),
            NormalizedValue::I64(1)
        );
        println!(
            "{}",
            serde_json::json!({"case":"bound-environment-admission", "reference":reference,
            "exact_fit_depth":256,"one_over_depth":257,"disposed_raw_environment_depth":10002,
            "exact_fit_bytes":measured.2,"exact_fit_items":measured.3,"work":measured.1,
            "cancelled_work":cancelled.1,"cleanup_owned":0})
        );
    }
}

#[test]
fn aggregate_equality_cannot_skip_a_callable_after_an_unequal_prefix_or_shape() {
    let (program, _) = fixture();
    let callable = NormalizedValue::Function {
        function: FunctionIndex(0, program.value_origin),
        type_arguments: Arc::from([]),
        bound_arguments: None,
    };
    let list = |values| NormalizedValue::List(Arc::new(values));
    let cases = [
        (
            list(vec![NormalizedValue::I64(1), callable.clone()]),
            list(vec![NormalizedValue::I64(2), callable.clone()]),
        ),
        (list(vec![callable.clone()]), list(vec![])),
        (
            NormalizedValue::Option(Some(Box::new(callable.clone()))),
            NormalizedValue::Option(None),
        ),
        (
            NormalizedValue::Map(Arc::new(BTreeMap::from([(
                NormalizedMapKey::I64(1),
                callable.clone(),
            )]))),
            NormalizedValue::Map(Arc::new(BTreeMap::new())),
        ),
        (list(vec![callable]), NormalizedValue::I64(0)),
    ];
    for (left, right) in cases {
        for (a, b) in [(&left, &right), (&right, &left)] {
            assert_eq!(
                super::super::normalized_equal(a, b).unwrap_err().code,
                "normalized_value_not_comparable"
            );
            assert_eq!(
                super::super::super::reference::reference_equal(a, b)
                    .unwrap_err()
                    .code,
                "normalized_reference_value_not_comparable"
            );
        }
    }
}

fn internal_type(
    program: &mut NormalizedProgram,
    schema: &mut NormalizedReferenceSchema,
    form: TypeForm,
) -> TypeObjectDigest {
    let object = TypeObject::new(form).expect("internal boundary fixture type");
    let (digest, _) = encode_type_object(&object).expect("canonical type identity");
    program.types.insert(digest, object.clone());
    schema.types.insert(digest, object);
    digest
}

#[test]
fn checked_construction_and_slow_canonical_oracle_discriminate_affine_empty_cases() {
    let (mut program, snapshot) = fixture();
    let mut schema =
        NormalizedReferenceSchema::reconstruct([&snapshot]).expect("independent schema");
    let control = ExecutionControl::uncancelled();
    let mut work = ValueWork::default();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let list_type = internal_type(&mut program, &mut schema, TypeForm::List { item: integer });
    let option_type = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Option { item: list_type },
    );
    let map_type = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Map {
            key: integer,
            value: option_type,
        },
    );
    let left = Name::new("left".to_owned()).expect("field");
    let right = Name::new("right".to_owned()).expect("field");
    let record_type = internal_type(
        &mut program,
        &mut schema,
        TypeForm::StructuralRecord {
            fields: vec![
                crate::platform::kernel::StructuralTypeField {
                    name: left.clone(),
                    ty: map_type,
                },
                crate::platform::kernel::StructuralTypeField {
                    name: right.clone(),
                    ty: map_type,
                },
            ],
        },
    );
    let items = (0..32)
        .map(|n| Value::scalar(&program, NormalizedValue::I64(n)).expect("scalar"))
        .collect();
    let list = Value::list(&program, items, &mut work).expect("checked list");
    let option = Value::option(&program, Some(list), &mut work).expect("checked option");
    let map = Value::map(
        &program,
        BTreeMap::from([(NormalizedMapKey::I64(0), option)]),
        &mut work,
    )
    .expect("checked map");
    let record = Value::record(
        &program,
        None,
        vec![
            (
                left,
                map.duplicate(ParameterUse::Unrestricted)
                    .expect("shared child"),
            ),
            (right, map),
        ],
        &mut work,
    )
    .expect("checked record");
    assert_eq!(
        value_oracle::inspect(
            &schema,
            program.value_origin,
            record.raw(),
            record_type,
            &control
        )
        .expect("oracle"),
        (Affinity::Free, 71)
    );
    assert_eq!(
        record.class(&program, &mut work).expect("root proof"),
        Class::Free
    );
    let selector = NormalizedFieldSelector::Structural(Name::new("left".to_owned()).unwrap());
    let child = record
        .field(&selector)
        .expect("field proof")
        .map_get(&NormalizedMapKey::I64(0))
        .expect("map proof")
        .expect("entry")
        .option_get()
        .expect("option proof")
        .expect("some");
    assert_eq!(
        child.list_get(31).expect("list proof").raw(),
        &NormalizedValue::I64(31)
    );
    assert_eq!(work.internal_guard_descendant_visits, 0);

    let (index, variant) = schema
        .variants
        .iter()
        .enumerate()
        .find(|(_, variant)| {
            variant.cases.iter().any(|case| {
                case.payload
                    .and_then(|ty| schema.types.get(&ty))
                    .is_some_and(|ty| matches!(ty.form, TypeForm::CapabilityResource { .. }))
            })
        })
        .expect("maintained affine variant");
    let empty = u32::try_from(
        variant
            .cases
            .iter()
            .position(|case| case.payload.is_none())
            .expect("empty case"),
    )
    .unwrap();
    let declaration = variant.declaration;
    let layout = VariantLayoutIndex(u32::try_from(index).unwrap(), program.value_origin);
    let nominal_type = internal_type(&mut program, &mut schema, TypeForm::Named { declaration });
    let mut owner =
        Value::variant(&program, layout, empty, None, &mut work).expect("empty affine owner");
    assert_eq!(owner.class(&program, &mut work).unwrap(), Class::Variant);
    assert!(owner.duplicate(ParameterUse::Unrestricted).is_err());
    assert_eq!(
        value_oracle::inspect(
            &schema,
            program.value_origin,
            owner.raw(),
            nominal_type,
            &control
        )
        .unwrap()
        .0,
        Affinity::Variant
    );
    // A deliberately restored raw-slot classification is detected independently.
    owner.class = Class::Free;
    assert_ne!(
        owner.class(&program, &mut work).unwrap() == Class::Free,
        value_oracle::inspect(
            &schema,
            program.value_origin,
            owner.raw(),
            nominal_type,
            &control
        )
        .unwrap()
        .0 == Affinity::Free
    );
    assert!(Value::scalar(&program, owner.into_raw()).is_err());
    let owner = Value::variant(&program, layout, empty, None, &mut work).unwrap();
    assert!(Value::list(&program, vec![owner], &mut work).is_err());
}

#[derive(Default)]
struct RejectingHost {
    calls: AtomicU64,
}

impl super::super::NormalizedHost for RejectingHost {
    fn call(
        &self,
        _: &NormalizedProgram,
        _: &super::super::super::prepare::NormalizedFunction,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        _: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(NormalizedValue::List(Arc::new(Vec::new())))
    }
}
impl NormalizedReferenceHost for RejectingHost {
    fn call(
        &self,
        _: &dyn NormalizedValueSchema,
        _: &ReferenceSignature,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        _: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(NormalizedValue::List(Arc::new(Vec::new())))
    }
}

#[test]
fn raw_tail_element_and_host_results_reject_in_both_tiers_before_downstream_work() {
    let (program, snapshot) = fixture();
    let control = ExecutionControl::uncancelled();
    let (index, function) = program.functions.iter().enumerate().find(|(_, function)| function.type_parameters.len() == 1 && matches!(&function.body, super::super::super::prepare::NormalizedFunctionBody::External(name) if name.as_str() == "core.list.length")).unwrap();
    let index = FunctionIndex(u32::try_from(index).unwrap(), program.value_origin);
    let integer = program
        .types
        .iter()
        .find_map(|(digest, ty)| matches!(ty.form, TypeForm::I64).then_some(*digest))
        .unwrap();
    let entry = super::super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
        index,
        Arc::from([integer]),
    );
    let (affine_index, affine) = program
        .variants
        .iter()
        .enumerate()
        .find(|(index, _)| program.affine_variants[*index])
        .unwrap();
    let tag = u32::try_from(
        affine
            .cases
            .iter()
            .position(|case| case.payload.is_none())
            .unwrap(),
    )
    .unwrap();
    let foreign_program = NormalizedProgram::prepare(program.artifact().clone()).unwrap();
    let scope = NormalizedResourceScope::new().unwrap();
    let requirement = RequirementReference {
        package: program.root_package,
        requirement: crate::platform::semantic_id::RequirementId::migrate(
            b"checked-boundary-lease",
            0,
        ),
    };
    let interface = program
        .types
        .values()
        .find_map(|ty| match ty.form {
            TypeForm::CapabilityResource { interface } => Some(interface),
            _ => None,
        })
        .unwrap();
    let handle = scope
        .reserve_queue_lease(requirement, interface)
        .unwrap()
        .commit(crate::platform::queue::JobLease {
            job_id: "boundary-job".to_owned(),
            attempt_id: "boundary-attempt".to_owned(),
            worker_id: "boundary-worker".to_owned(),
            payload: Vec::new(),
            attempt_number: 1,
            lease_until_milliseconds: 1,
        })
        .unwrap();
    assert_eq!(scope.live_resources(), 1);
    let negatives = [
        NormalizedValue::Resource(handle),
        NormalizedValue::Bool(true),
        NormalizedValue::Variant {
            layout: VariantLayoutIndex(u32::try_from(affine_index).unwrap(), program.value_origin),
            case: tag,
            payload: None,
        },
        NormalizedValue::Variant {
            layout: VariantLayoutIndex(
                u32::try_from(affine_index).unwrap(),
                foreign_program.value_origin,
            ),
            case: tag,
            payload: None,
        },
    ];
    for last in negatives {
        let mut values = vec![NormalizedValue::I64(1); 4095];
        values.push(last);
        let arguments = vec![NormalizedValue::List(Arc::new(values))];
        let host = RejectingHost::default();
        let vm_sink = std::sync::Mutex::new(None);
        let reference_sink = std::sync::Mutex::new(None);
        let production =
            super::super::NormalizedVm::new(&program, super::super::NormalizedRunPolicy::default())
                .observing(&vm_sink, &host)
                .invoke_entry(entry.clone(), arguments.clone(), None, &control)
                .expect_err("raw VM input rejects");
        let reference = NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            super::super::NormalizedRunPolicy::default(),
        )
        .observing(&reference_sink, &host)
        .invoke_instantiated(function.declaration, &[integer], arguments, &control)
        .expect_err("raw reference input rejects");
        assert_eq!(production.code, "normalized_value_admission");
        assert_eq!(reference.code, "normalized_reference_value_admission");
        assert_eq!(host.calls.load(Ordering::Relaxed), 0);
        let production = vm_sink.into_inner().unwrap().unwrap();
        let reference = reference_sink.into_inner().unwrap().unwrap();
        assert_eq!(production.value_work.input_admission_nodes, 4097);
        assert_eq!(reference.value_work.input_admission_nodes, 4097);
        println!(
            "{}",
            serde_json::json!({"case":"raw-last-element", "production": production.value_work, "reference": reference.value_work, "callback_calls":0})
        );
        assert_eq!(
            production.live_handles_after
                + production.live_operands_after
                + production.live_call_frames_after
                + production.live_transactions_after,
            0
        );
        assert_eq!(
            reference.live_handles_after
                + reference.live_call_frames_after
                + reference.live_transactions_after,
            0
        );
    }
    scope.release_all();
    assert_eq!(scope.live_resources(), 0);
    for reference in [false, true] {
        let host = RejectingHost::default();
        let arguments = vec![NormalizedValue::List(Arc::new(vec![NormalizedValue::I64(
            1,
        )]))];
        if reference {
            let sink = std::sync::Mutex::new(None);
            let error = NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                super::super::NormalizedRunPolicy::default(),
            )
            .observing(&sink, &host)
            .invoke_instantiated(function.declaration, &[integer], arguments, &control)
            .expect_err("malformed host result");
            assert_eq!(error.code, "normalized_reference_value_admission");
            assert_eq!(
                sink.into_inner()
                    .unwrap()
                    .unwrap()
                    .value_work
                    .raw_result_admission_nodes,
                1
            );
        } else {
            let sink = std::sync::Mutex::new(None);
            let error = super::super::NormalizedVm::new(
                &program,
                super::super::NormalizedRunPolicy::default(),
            )
            .observing(&sink, &host)
            .invoke_entry(entry.clone(), arguments, None, &control)
            .expect_err("malformed host result");
            assert_eq!(error.code, "normalized_value_admission");
            assert_eq!(
                sink.into_inner()
                    .unwrap()
                    .unwrap()
                    .value_work
                    .raw_result_admission_nodes,
                1
            );
        }
        assert_eq!(host.calls.load(Ordering::Relaxed), 1);
    }
    assert_eq!(
        super::super::NormalizedVm::new(&program, super::super::NormalizedRunPolicy::default())
            .invoke_entry(
                entry,
                vec![NormalizedValue::List(Arc::new(vec![NormalizedValue::I64(
                    1
                )]))],
                None,
                &control
            )
            .unwrap()
            .0,
        NormalizedValue::I64(1)
    );
}

#[test]
fn admission_limits_and_deterministic_cancellation_retain_progress_and_allow_reuse() {
    let (mut program, mut snapshot) = fixture();
    let (index, declaration) = program.functions.iter().enumerate().find(|(_, function)| function.type_parameters.len() == 1 && matches!(&function.body, super::super::super::prepare::NormalizedFunctionBody::External(name) if name.as_str() == "core.list.length"))
        .map(|(index, function)| (FunctionIndex(u32::try_from(index).unwrap(), program.value_origin), function.declaration)).unwrap();
    let integer = program
        .types
        .iter()
        .find_map(|(digest, ty)| matches!(ty.form, TypeForm::I64).then_some(*digest))
        .unwrap();
    // Internal boundary types exercise depth without adding a public authoring form.
    let mut nested_types = vec![integer];
    let mut ty = integer;
    for _ in 0..256 {
        let object = TypeObject::new(TypeForm::Option { item: ty }).unwrap();
        let (digest, _) = encode_type_object(&object).unwrap();
        program.types.insert(digest, object.clone());
        snapshot.types.insert(digest, object);
        ty = digest;
        nested_types.push(ty);
    }
    let invoke = |reference: bool, ty, raw: NormalizedValue, policy, control: &ExecutionControl| {
        if reference {
            let sink = std::sync::Mutex::new(None);
            let result = NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                .observing(
                    &sink,
                    &super::super::super::reference::CoreNormalizedReferenceHost,
                )
                .invoke_instantiated(declaration, &[ty], vec![raw], control)
                .map(|(value, _)| value);
            let observation = sink.into_inner().unwrap().unwrap();
            assert_eq!(
                observation.live_call_frames_after
                    + observation.live_control_frames_after
                    + observation.live_local_scopes_after
                    + observation.live_type_scopes_after
                    + observation.live_transactions_after
                    + observation.live_handles_after,
                0
            );
            (result, observation.value_work, observation.allocated_bytes)
        } else {
            let sink = std::sync::Mutex::new(None);
            let entry = super::super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                index,
                Arc::from([ty]),
            );
            let result = super::super::NormalizedVm::new(&program, policy)
                .observing(&sink, &super::super::CoreNormalizedHost)
                .invoke_entry(entry, vec![raw], None, control)
                .map(|(value, _)| value);
            let observation = sink.into_inner().unwrap().unwrap();
            assert_eq!(
                observation.live_call_frames_after
                    + observation.live_locals_after
                    + observation.live_type_bindings_after
                    + observation.live_operands_after
                    + observation.live_transactions_after
                    + observation.live_handles_after,
                0
            );
            (result, observation.value_work, observation.allocated_bytes)
        }
    };
    let list = |count| NormalizedValue::List(Arc::new(vec![NormalizedValue::I64(1); count]));
    for reference in [false, true] {
        let control = ExecutionControl::uncancelled();
        let policy = super::super::NormalizedRunPolicy {
            maximum_collection_items: 8,
            ..Default::default()
        };
        assert_eq!(
            invoke(reference, integer, list(8), policy, &control)
                .0
                .unwrap(),
            NormalizedValue::I64(8)
        );
        assert_eq!(
            invoke(reference, integer, list(9), policy, &control)
                .0
                .unwrap_err()
                .class,
            crate::platform::execution::ExecutionFailureClass::Resource
        );
        let allocated = invoke(reference, integer, list(8), policy, &control).2;
        for (limit, success) in [(allocated, true), (allocated - 1, false)] {
            let result = invoke(
                reference,
                integer,
                list(8),
                super::super::NormalizedRunPolicy {
                    maximum_allocated_bytes: limit,
                    ..policy
                },
                &control,
            )
            .0;
            assert_eq!(result.is_ok(), success);
            if let Err(error) = result {
                assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Resource
                );
            }
        }
        for depth in [255, 256] {
            let mut value = NormalizedValue::I64(1);
            for _ in 0..depth {
                value = NormalizedValue::Option(Some(Box::new(value)));
            }
            let result = invoke(
                reference,
                nested_types[depth],
                NormalizedValue::List(Arc::new(vec![value])),
                Default::default(),
                &control,
            );
            if depth == 255 {
                assert_eq!(result.0.unwrap(), NormalizedValue::I64(1));
            } else {
                assert_eq!(
                    result.0.unwrap_err().code,
                    if reference {
                        "normalized_reference_value_depth"
                    } else {
                        "normalized_value_depth"
                    }
                );
            }
        }
        let cancelled = ExecutionControl::cancel_after_checks(37);
        let (result, work, _) = invoke(
            reference,
            integer,
            list(4096),
            Default::default(),
            &cancelled,
        );
        assert_eq!(result.unwrap_err().code, "execution_cancelled");
        assert!(work.input_admission_nodes > 0 && work.input_admission_nodes < 4097);
        assert_eq!(work.raw_result_admission_nodes, 0);
        println!(
            "{}",
            serde_json::json!({"case":"admission-cancellation", "reference": reference, "work": work, "cleanup_owned":0, "exact_fit_items":8, "one_over_items":9, "exact_fit_bytes":allocated, "exact_fit_depth":256, "one_over_depth":257})
        );
        assert_eq!(
            invoke(reference, integer, list(1), Default::default(), &control)
                .0
                .unwrap(),
            NormalizedValue::I64(1)
        );
    }
}

#[test]
fn corrupt_production_layout_class_is_caught_by_canonical_reference_and_oracle() {
    let (mut program, snapshot) = fixture();
    let schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let (index, declaration) = program.functions.iter().enumerate()
        .find(|(_, function)| function.type_parameters.len() == 1 && matches!(&function.body, super::super::super::prepare::NormalizedFunctionBody::External(name) if name.as_str() == "core.list.length"))
        .map(|(index, function)| (FunctionIndex(u32::try_from(index).unwrap(), program.value_origin), function.declaration)).unwrap();
    let (layout, variant) = program
        .variants
        .iter()
        .enumerate()
        .find(|(index, _)| program.affine_variants[*index])
        .unwrap();
    let variant_type = program
        .types
        .iter()
        .find_map(|(digest, ty)| {
            matches!(ty.form, TypeForm::Named { declaration } if declaration == variant.declaration)
                .then_some(*digest)
        })
        .unwrap();
    let case = u32::try_from(
        variant
            .cases
            .iter()
            .position(|case| case.payload.is_none())
            .unwrap(),
    )
    .unwrap();
    let raw = NormalizedValue::Variant {
        layout: VariantLayoutIndex(u32::try_from(layout).unwrap(), program.value_origin),
        case,
        payload: None,
    };
    assert_eq!(
        value_oracle::inspect(
            &schema,
            program.value_origin,
            &raw,
            variant_type,
            &ExecutionControl::uncancelled()
        )
        .unwrap()
        .0,
        Affinity::Variant
    );
    Arc::make_mut(&mut program.affine_variants)[layout] = false; // Safe fault in disposable production metadata only.
    let arguments = vec![NormalizedValue::List(Arc::new(vec![raw]))];
    let production = super::super::NormalizedVm::new(&program, Default::default())
        .invoke_entry(
            super::super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                index,
                Arc::from([variant_type]),
            ),
            arguments.clone(),
            None,
            &ExecutionControl::uncancelled(),
        )
        .expect("corrupt classification makes the wrong decision");
    assert_eq!(production.0, NormalizedValue::I64(1));
    let reference = NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
        .invoke_instantiated(
            declaration,
            &[variant_type],
            arguments,
            &ExecutionControl::uncancelled(),
        )
        .expect_err("reference independently retains nominal affinity");
    assert_eq!(reference.code, "normalized_reference_value_admission");
    println!(
        "{}",
        serde_json::json!({"case":"production-classification-corruption","production_wrongly_accepted":true,"reference_failure":reference,"independent_oracle":"affine-variant"})
    );
}

#[test]
fn independent_oracle_covers_scalar_and_nominal_constructors_and_foreign_callbacks() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let control = ExecutionControl::uncancelled();
    let mut work = ValueWork::default();
    let mut cases = Vec::new();
    for (raw, form) in [
        (NormalizedValue::Unit, TypeForm::Unit),
        (NormalizedValue::Bool(true), TypeForm::Bool),
        (NormalizedValue::I64(-7), TypeForm::I64),
        (NormalizedValue::bytes(vec![1, 2]), TypeForm::Bytes),
        (NormalizedValue::text("text"), TypeForm::Text),
        (
            NormalizedValue::StaticText(Arc::from("fixed")),
            TypeForm::StaticText,
        ),
    ] {
        let ty = internal_type(&mut program, &mut schema, form);
        cases.push((Value::scalar(&program, raw).unwrap(), ty));
    }
    let (index, record) = program
        .records
        .iter()
        .enumerate()
        .find(|(_, record)| {
            !record.fields.is_empty()
                && record.fields.iter().all(|field| {
                    matches!(
                        program.types.get(&field.ty).map(|ty| &ty.form),
                        Some(TypeForm::Text | TypeForm::Bytes)
                    )
                })
        })
        .map(|(i, r)| (i, r.clone()))
        .unwrap();
    let fields = record
        .fields
        .iter()
        .map(|field| {
            let raw = if matches!(program.types[&field.ty].form, TypeForm::Text) {
                NormalizedValue::text("field")
            } else {
                NormalizedValue::bytes(vec![3])
            };
            (field.name.clone(), Value::scalar(&program, raw).unwrap())
        })
        .collect();
    let ty = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Named {
            declaration: record.declaration,
        },
    );
    cases.push((
        Value::record(
            &program,
            Some(RecordLayoutIndex(
                u32::try_from(index).unwrap(),
                program.value_origin,
            )),
            fields,
            &mut work,
        )
        .unwrap(),
        ty,
    ));
    let (index, variant, case) = program
        .variants
        .iter()
        .enumerate()
        .filter(|(index, _)| !program.affine_variants[*index])
        .find_map(|(i, variant)| {
            variant
                .cases
                .iter()
                .position(|case| case.payload.is_none())
                .map(|case| (i, variant.clone(), case))
        })
        .unwrap();
    let ty = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Named {
            declaration: variant.declaration,
        },
    );
    cases.push((
        Value::variant(
            &program,
            VariantLayoutIndex(u32::try_from(index).unwrap(), program.value_origin),
            u32::try_from(case).unwrap(),
            None,
            &mut work,
        )
        .unwrap(),
        ty,
    ));
    snapshot.types = program.types.clone();
    let (length, declaration) = program.functions.iter().enumerate().find(|(_, function)| function.type_parameters.len() == 1 && matches!(&function.body, super::super::super::prepare::NormalizedFunctionBody::External(name) if name.as_str() == "core.list.length"))
        .map(|(i, f)| (FunctionIndex(u32::try_from(i).unwrap(), program.value_origin), f.declaration)).unwrap();
    for (value, ty) in cases {
        assert_eq!(value.class(&program, &mut work).unwrap(), Class::Free);
        assert_eq!(
            value_oracle::inspect(&schema, program.value_origin, value.raw(), ty, &control)
                .unwrap()
                .0,
            Affinity::Free
        );
        let raw = NormalizedValue::List(Arc::new(vec![value.into_raw()]));
        let production = super::super::NormalizedVm::new(&program, Default::default())
            .invoke_entry(
                super::super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                    length,
                    Arc::from([ty]),
                ),
                vec![raw.clone()],
                None,
                &control,
            )
            .unwrap();
        let reference =
            NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                .invoke_instantiated(declaration, &[ty], vec![raw], &control)
                .unwrap();
        assert_eq!(production.0, NormalizedValue::I64(1));
        assert_eq!(production.0, reference.0);
        assert_eq!(
            production.1.value_work.internal_guard_descendant_visits
                + reference.1.value_work.internal_guard_descendant_visits,
            0
        );
    }
    let fold = program
        .functions
        .iter()
        .enumerate()
        .find(|(_, function)| {
            function.type_parameters.len() == 2
                && function.parameters.len() == 3
                && function.parameters.last().is_some_and(|parameter| {
                    matches!(program.types[&parameter.ty].form, TypeForm::Function { .. })
                })
        })
        .map(|(i, f)| {
            (
                FunctionIndex(u32::try_from(i).unwrap(), program.value_origin),
                f.declaration,
            )
        })
        .unwrap();
    let integer = program
        .types
        .iter()
        .find_map(|(digest, ty)| matches!(ty.form, TypeForm::I64).then_some(*digest))
        .unwrap();
    let foreign = NormalizedProgram::prepare(program.artifact().clone()).unwrap();
    let invalid_callback = NormalizedValue::Function {
        function: FunctionIndex(0, foreign.value_origin),
        type_arguments: Arc::from([]),
        bound_arguments: None,
    };
    for reference in [false, true] {
        let host = RejectingHost::default();
        let args = vec![
            NormalizedValue::List(Arc::new(Vec::new())),
            NormalizedValue::I64(0),
            invalid_callback.clone(),
        ];
        let error = if reference {
            let sink = std::sync::Mutex::new(None);
            NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                .observing(&sink, &host)
                .invoke_instantiated(fold.1, &[integer, integer], args, &control)
                .unwrap_err()
        } else {
            let sink = std::sync::Mutex::new(None);
            super::super::NormalizedVm::new(&program, Default::default())
                .observing(&sink, &host)
                .invoke_entry(
                    super::super::super::prepare::NormalizedEntryPoint::InstantiatedFunction(
                        fold.0,
                        Arc::from([integer, integer]),
                    ),
                    args,
                    None,
                    &control,
                )
                .unwrap_err()
        };
        assert_eq!(
            error.code,
            if reference {
                "normalized_reference_value_admission"
            } else {
                "normalized_value_admission"
            }
        );
        assert_eq!(host.calls.load(Ordering::Relaxed), 0);
    }
    println!(
        "{}",
        serde_json::json!({"case":"independent-oracle-shapes", "scalar_kinds":6, "nominal_record":true, "ordinary_variant":true, "foreign_callbacks_rejected_tiers":2})
    );
}

#[test]
fn canonical_affinity_derivation_rejects_a_missing_payload_type() {
    let (_, mut snapshot) = fixture();
    let ty = snapshot
        .types
        .iter()
        .find_map(|(digest, ty)| {
            matches!(ty.form, TypeForm::CapabilityResource { .. }).then_some(*digest)
        })
        .unwrap();
    snapshot.types.remove(&ty);
    assert!(NormalizedReferenceSchema::reconstruct([&snapshot]).is_err());
}
