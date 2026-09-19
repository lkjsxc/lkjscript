//! Independent VM/reference map admission, retained captures and ledger boundaries.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use super::*;
use crate::platform::execution::normalized;
use normalized::prepare::{NormalizedEntryPoint, NormalizedFunctionBody};
use normalized::reference::NormalizedReferenceRead;
use normalized::vm::{NormalizedRunPolicy, NormalizedVm};

#[derive(Debug)]
struct Observed {
    value: Result<NormalizedValue, ExecutionError>,
    work: ValueWork,
    bytes: u64,
    items: u64,
    calls: u64,
}

struct ControlledMapHost {
    calls: AtomicU64,
    result: NormalizedValue,
}

impl ControlledMapHost {
    fn new(result: NormalizedValue) -> Self {
        Self {
            calls: AtomicU64::new(0),
            result,
        }
    }
}

impl normalized::vm::NormalizedHost for ControlledMapHost {
    fn call(
        &self,
        _: &NormalizedProgram,
        _: &normalized::prepare::NormalizedFunction,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        _: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.result.clone())
    }
}

impl NormalizedReferenceHost for ControlledMapHost {
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
        Ok(self.result.clone())
    }
}

fn external(
    program: &NormalizedProgram,
    implementation: &str,
    type_count: usize,
    parameter_count: usize,
) -> (FunctionIndex, crate::platform::kernel::DeclarationReference) {
    program.functions.iter().enumerate().find_map(|(index, function)| {
        (function.type_parameters.len() == type_count && function.parameters.len() == parameter_count
            && matches!(&function.body, NormalizedFunctionBody::External(name) if name.as_str() == implementation))
            .then_some((FunctionIndex(index as u32, program.value_origin), function.declaration))
    }).expect("exact generic and value arities select the maintained external signature")
}

#[allow(clippy::too_many_arguments)]
fn invoke(
    program: &NormalizedProgram,
    reader: &dyn NormalizedReferenceRead,
    reference: bool,
    implementation: &str,
    types: &[TypeObjectDigest],
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
    host: Option<&ControlledMapHost>,
) -> Observed {
    let (function, declaration) = external(program, implementation, types.len(), arguments.len());
    if reference {
        let sink = std::sync::Mutex::new(None);
        let interpreter = NormalizedReferenceInterpreter::from_reader(reader, program, policy);
        let interpreter = if let Some(host) = host {
            interpreter.observing(&sink, host)
        } else {
            interpreter.observing_checked(&sink)
        };
        let value = interpreter
            .invoke_instantiated(declaration, types, arguments, control)
            .map(|(value, _)| value);
        let observation = sink.into_inner().unwrap().unwrap();
        assert_eq!(
            observation.live_call_frames_after
                + observation.live_control_frames_after
                + observation.live_local_scopes_after
                + observation.live_type_scopes_after
                + observation.live_effect_scopes_after
                + observation.live_allowances_after
                + observation.live_transactions_after
                + observation.live_handles_after,
            0,
            "all reference-owned resources are joined"
        );
        Observed {
            value,
            work: observation.value_work,
            bytes: observation.allocated_bytes,
            items: observation.collection_items,
            calls: observation.calls,
        }
    } else {
        let sink = std::sync::Mutex::new(None);
        let interpreter = NormalizedVm::new(program, policy);
        let interpreter = if let Some(host) = host {
            interpreter.observing(&sink, host)
        } else {
            interpreter.observing_checked(&sink)
        };
        let value = interpreter
            .invoke_entry(
                NormalizedEntryPoint::InstantiatedFunction(function, Arc::from(types)),
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
            0,
            "all VM-owned resources are joined"
        );
        Observed {
            value,
            work: observation.value_work,
            bytes: observation.allocated_bytes,
            items: observation.collection_items,
            calls: observation.calls,
        }
    }
}

fn map(entries: impl IntoIterator<Item = (NormalizedMapKey, NormalizedValue)>) -> NormalizedValue {
    NormalizedValue::map(entries.into_iter().collect()).unwrap()
}

fn integers(count: i64) -> NormalizedValue {
    map((0..count).map(|n| (NormalizedMapKey::I64(n), NormalizedValue::I64(n))))
}

fn scalar_type(program: &NormalizedProgram, form: TypeForm) -> TypeObjectDigest {
    program
        .types
        .iter()
        .find_map(|(digest, object)| (object.form == form).then_some(*digest))
        .unwrap()
}

fn result_placement_bytes(reference: bool) -> u64 {
    // Machine::push installs one checked operand after the intrinsic succeeds.
    // Reference external calls return ReferenceStep::Value directly; their call
    // argument proof headers have already been reserved before the intrinsic.
    if reference {
        0
    } else {
        (std::mem::size_of::<Value>() - std::mem::size_of::<NormalizedValue>()) as u64
    }
}

#[test]
fn maps_reject_final_children_foreign_origins_and_hidden_authority_before_callbacks() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let boolean = internal_type(&mut program, &mut schema, TypeForm::Bool);
    let callable_type = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Function {
            parameters: vec![integer, integer],
            result: integer,
        },
    );
    let (add, _) = external(&program, "core.i64.add", 0, 2);
    let foreign = NormalizedProgram::prepare(program.artifact().clone()).unwrap();
    let callable = |origin| NormalizedValue::Function {
        function: FunctionIndex(add.0, origin),
        type_arguments: Arc::from([]),
        effect_arguments: Arc::from([]),
        requirement_arguments: Arc::from([]),
        bound_arguments: None,
    };
    let (record_index, record) = program
        .records
        .iter()
        .enumerate()
        .find(|(_, record)| {
            !record.fields.is_empty()
                && record.fields.iter().all(|field| {
                    matches!(
                        program.types[&field.ty].form,
                        TypeForm::Text | TypeForm::Bytes
                    )
                })
        })
        .map(|(index, record)| (index, record.clone()))
        .unwrap();
    let record_type = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Named {
            declaration: record.declaration,
        },
    );
    let record_value = |origin| {
        NormalizedValue::Record(NormalizedRecord::Nominal {
            layout: RecordLayoutIndex(record_index as u32, origin),
            fields: Arc::new(
                record
                    .fields
                    .iter()
                    .map(|field| match program.types[&field.ty].form {
                        TypeForm::Text => NormalizedValue::text("record field"),
                        _ => NormalizedValue::bytes([3_u8]),
                    })
                    .collect(),
            ),
        })
    };
    let (affine_index, affine) = program
        .variants
        .iter()
        .enumerate()
        .find(|(index, _)| program.affine_variants[*index])
        .unwrap();
    let empty_case = affine
        .cases
        .iter()
        .position(|case| case.payload.is_none())
        .unwrap();
    let affine_type = program.types.iter().find_map(|(ty, object)| matches!(object.form, TypeForm::Named { declaration } if declaration == affine.declaration).then_some(*ty)).unwrap();
    let hidden_affine = NormalizedValue::Variant {
        layout: VariantLayoutIndex(affine_index as u32, program.value_origin),
        case: empty_case as u32,
        payload: None,
    };
    let scope = NormalizedResourceScope::new().unwrap();
    let interface = program
        .types
        .values()
        .find_map(|object| match object.form {
            TypeForm::CapabilityResource { interface } => Some(interface),
            _ => None,
        })
        .unwrap();
    let handle = scope
        .reserve_queue_lease(
            RequirementReference {
                package: program.root_package,
                requirement: crate::platform::semantic_id::RequirementId::migrate(
                    b"persistent-map-raw-authority",
                    0,
                ),
            },
            interface,
        )
        .unwrap()
        .commit(crate::platform::queue::JobLease {
            job_id: "owned-map-test".to_owned(),
            attempt_id: "attempt".to_owned(),
            worker_id: "worker".to_owned(),
            payload: vec![],
            attempt_number: 1,
            lease_until_milliseconds: 1,
        })
        .unwrap();
    let scope_value = NormalizedValue::Resource(handle);
    let mut invalids = Vec::new();
    for (name, ty, valid, invalid) in [
        (
            "wrong-final-scalar",
            integer,
            NormalizedValue::I64(7),
            NormalizedValue::Bool(true),
        ),
        (
            "foreign-final-callable",
            callable_type,
            callable(program.value_origin),
            callable(foreign.value_origin),
        ),
        (
            "foreign-final-record",
            record_type,
            record_value(program.value_origin),
            record_value(foreign.value_origin),
        ),
        (
            "hidden-affine-empty-case",
            integer,
            NormalizedValue::I64(7),
            hidden_affine.clone(),
        ),
        (
            "hidden-resource",
            integer,
            NormalizedValue::I64(7),
            scope_value,
        ),
    ] {
        let raw = map((0..1023)
            .map(|n| (NormalizedMapKey::I64(n), valid.clone()))
            .chain(std::iter::once((NormalizedMapKey::I64(1023), invalid))));
        invalids.push((name, ty, raw));
    }
    invalids.push((
        "wrong-key-kind",
        integer,
        map([(NormalizedMapKey::Bool(false), NormalizedValue::I64(7))]),
    ));
    invalids.push(("wrong-value-kind", boolean, integers(1)));
    invalids.push((
        "exact-affine-value-type",
        affine_type,
        map([(NormalizedMapKey::I64(0), hidden_affine)]),
    ));
    snapshot.types = schema.types.clone();
    let reader = boundary_reader(&snapshot, &schema);
    for reference in [false, true] {
        for (name, ty, raw) in &invalids {
            let retained = raw.clone();
            let host = ControlledMapHost::new(NormalizedValue::I64(0));
            let result = invoke(
                &program,
                &reader,
                reference,
                "core.map.length",
                &[integer, *ty],
                vec![raw.clone()],
                Default::default(),
                &ExecutionControl::uncancelled(),
                Some(&host),
            );
            assert!(result.value.is_err(), "{reference}/{name}");
            assert_eq!(
                result.value.as_ref().unwrap_err().code,
                if reference {
                    "normalized_reference_value_admission"
                } else {
                    "normalized_value_admission"
                },
                "{reference}/{name}: the selected signature reaches complete value admission"
            );
            assert_eq!(
                host.calls.load(Ordering::Relaxed),
                0,
                "input admission precedes the callback"
            );
            assert_eq!(result.calls, 0);
            assert_eq!(
                scope.live_resources(),
                1,
                "foreign map ingress cannot release another owner's lease"
            );
            assert_eq!(&retained, raw);
            if *name == "wrong-final-scalar" {
                assert_eq!(
                    result.work.input_admission_nodes, 1025,
                    "the final child is checked even when lookup needs no values"
                );
            }
        }
        for (ty, child) in [
            (callable_type, callable(program.value_origin)),
            (record_type, record_value(program.value_origin)),
        ] {
            let result = invoke(
                &program,
                &reader,
                reference,
                "core.map.length",
                &[integer, ty],
                vec![map([(NormalizedMapKey::I64(0), child)])],
                Default::default(),
                &ExecutionControl::uncancelled(),
                None,
            );
            assert_eq!(result.value.unwrap(), NormalizedValue::I64(1));
        }
    }
    scope.release_all();
    assert_eq!(scope.live_resources(), 0);
}

#[test]
fn retained_map_captures_are_readmitted_independently_and_host_results_are_complete() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let thunk = internal_type(
        &mut program,
        &mut schema,
        TypeForm::Function {
            parameters: vec![],
            result: integer,
        },
    );
    let (length, _) = external(&program, "core.map.length", 2, 1);
    snapshot.types = schema.types.clone();
    let reader = boundary_reader(&snapshot, &schema);
    let capture = |captured| NormalizedValue::Function {
        function: length,
        type_arguments: Arc::from([integer, integer]),
        effect_arguments: Arc::from([]),
        requirement_arguments: Arc::from([]),
        bound_arguments: Some(Arc::new(vec![captured])),
    };
    let original = integers(128);
    let NormalizedValue::Map(entries) = &original else {
        unreachable!()
    };
    let changed = NormalizedValue::Map(
        entries
            .insert(
                NormalizedMapKey::I64(127),
                NormalizedValue::Bool(true),
                1_000_000,
                &mut |_| Ok(()),
            )
            .unwrap(),
    );
    for reference in [false, true] {
        let control = ExecutionControl::uncancelled();
        let good = invoke(
            &program,
            &reader,
            reference,
            "core.list.length",
            &[thunk],
            vec![NormalizedValue::list(vec![capture(original.clone())]).unwrap()],
            Default::default(),
            &control,
            None,
        );
        assert_eq!(good.value.unwrap(), NormalizedValue::I64(1));
        assert!(good.work.capture_admission_nodes >= 129);
        let host = ControlledMapHost::new(NormalizedValue::I64(1));
        let bad = invoke(
            &program,
            &reader,
            reference,
            "core.list.length",
            &[thunk],
            vec![NormalizedValue::list(vec![capture(changed.clone())]).unwrap()],
            Default::default(),
            &control,
            Some(&host),
        );
        assert!(bad.value.is_err());
        assert!(bad.work.capture_admission_nodes >= 129);
        assert_eq!(host.calls.load(Ordering::Relaxed), 0);
        // A controlled raw adapter returns an invalid final child. Complete result
        // admission rejects it even though all earlier payloads are valid aliases.
        let result_host = ControlledMapHost::new(changed.clone());
        let result = invoke(
            &program,
            &reader,
            reference,
            "core.map.insert",
            &[integer, integer],
            vec![
                original.clone(),
                NormalizedValue::I64(129),
                NormalizedValue::I64(129),
            ],
            Default::default(),
            &control,
            Some(&result_host),
        );
        assert!(result.value.is_err());
        assert_eq!(result_host.calls.load(Ordering::Relaxed), 1);
        assert_eq!(result.work.raw_result_admission_nodes, 129);
        assert_eq!(
            invoke(
                &program,
                &reader,
                reference,
                "core.map.length",
                &[integer, integer],
                vec![original.clone()],
                Default::default(),
                &control,
                None
            )
            .value
            .unwrap(),
            NormalizedValue::I64(128)
        );
    }
}

#[test]
fn checked_map_updates_charge_only_reserved_paths_with_exact_fit_and_refusal() {
    let (program, snapshot) = fixture();
    let integer = scalar_type(&program, TypeForm::I64);
    let control = ExecutionControl::uncancelled();
    let word = std::mem::size_of::<usize>() as u64;
    let node_bytes = 6 * word;
    let entry_bytes = (std::mem::size_of::<NormalizedMapKey>()
        + std::mem::size_of::<NormalizedValue>()) as u64
        + 2 * word;
    for reference in [false, true] {
        for count in [0, 1, 8, 1024] {
            let original = integers(count);
            // Bulk construction picks count/2 as its root. Replacement at that
            // root reserves one node regardless of retained map cardinality.
            let root_key = count / 2;
            let args = || {
                vec![
                    original.clone(),
                    NormalizedValue::I64(root_key),
                    NormalizedValue::I64(77),
                ]
            };
            let baseline = invoke(
                &program,
                &snapshot,
                reference,
                "core.map.get-or",
                &[integer, integer],
                args(),
                Default::default(),
                &control,
                None,
            );
            assert!(baseline.value.is_ok());
            let result = invoke(
                &program,
                &snapshot,
                reference,
                "core.map.insert",
                &[integer, integer],
                args(),
                Default::default(),
                &control,
                None,
            );
            let changed = result.value.as_ref().unwrap();
            assert_eq!(
                result.bytes - baseline.bytes,
                entry_bytes + node_bytes,
                "{reference}/{count}: payloads and key buffers are not deep copied"
            );
            assert_eq!(result.items - baseline.items, 1);
            assert_eq!(result.work.maps.entry_handles_allocated, 1);
            assert_eq!(result.work.maps.nodes_allocated, 1);
            let NormalizedValue::Map(changed) = changed else {
                unreachable!()
            };
            assert_eq!(
                changed.get(&NormalizedMapKey::I64(root_key)),
                Some(&NormalizedValue::I64(77))
            );
            for (limit, success) in [(result.bytes, true), (result.bytes - 1, false)] {
                let checked = invoke(
                    &program,
                    &snapshot,
                    reference,
                    "core.map.insert",
                    &[integer, integer],
                    args(),
                    NormalizedRunPolicy {
                        maximum_allocated_bytes: Some(limit),
                        ..Default::default()
                    },
                    &control,
                    None,
                );
                assert_eq!(
                    checked.value.is_ok(),
                    success,
                    "{reference}/{count}/{limit}"
                );
                if !success {
                    let placement = result_placement_bytes(reference);
                    assert_eq!(
                        checked.bytes,
                        if placement == 0 {
                            baseline.bytes + entry_bytes
                        } else {
                            result.bytes - placement
                        },
                        "accepted map work remains charged when its next node or result placement refuses"
                    );
                    assert_eq!(
                        checked.items,
                        if placement == 0 {
                            baseline.items
                        } else {
                            result.items
                        }
                    );
                }
            }
            let placement = result_placement_bytes(reference);
            let refused_node = invoke(
                &program,
                &snapshot,
                reference,
                "core.map.insert",
                &[integer, integer],
                args(),
                NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(result.bytes - placement - 1),
                    ..Default::default()
                },
                &control,
                None,
            );
            assert!(refused_node.value.is_err());
            assert_eq!(
                refused_node.bytes,
                baseline.bytes - placement + entry_bytes,
                "entry storage was reserved; node storage and later result placement were refused"
            );
            assert_eq!(refused_node.items, baseline.items);
            if result.items > 1 {
                let checked = invoke(
                    &program,
                    &snapshot,
                    reference,
                    "core.map.insert",
                    &[integer, integer],
                    args(),
                    NormalizedRunPolicy {
                        maximum_collection_items: Some(result.items - 1),
                        ..Default::default()
                    },
                    &control,
                    None,
                );
                assert!(checked.value.is_err());
                assert_eq!(checked.bytes, baseline.bytes - placement + entry_bytes);
            }
            let NormalizedValue::Map(original) = &original else {
                unreachable!()
            };
            assert_eq!(
                original.get(&NormalizedMapKey::I64(root_key)),
                (count != 0).then_some(&NormalizedValue::I64(root_key))
            );
        }
    }
}

#[test]
fn map_admission_keeps_depth_and_aggregate_item_bounds_in_foreground() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let (mut program, mut snapshot) = fixture();
            let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
            let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
            let list_integer =
                internal_type(&mut program, &mut schema, TypeForm::List { item: integer });
            let mut depths = vec![integer];
            for _ in 0..256 {
                let item = *depths.last().unwrap();
                depths.push(internal_type(
                    &mut program,
                    &mut schema,
                    TypeForm::Option { item },
                ));
            }
            snapshot.types = schema.types.clone();
            let reader = boundary_reader(&snapshot, &schema);
            for reference in [false, true] {
                let control = ExecutionControl::uncancelled();
                for depth in [255, 256] {
                    let mut raw = NormalizedValue::I64(1);
                    for _ in 0..depth {
                        raw = NormalizedValue::Option(Some(Box::new(raw)));
                    }
                    let result = invoke(
                        &program,
                        &reader,
                        reference,
                        "core.map.length",
                        &[integer, depths[depth]],
                        vec![map([(NormalizedMapKey::I64(0), raw)])],
                        NormalizedRunPolicy::foreground(),
                        &control,
                        None,
                    );
                    if depth == 255 {
                        assert_eq!(result.value.unwrap(), NormalizedValue::I64(1));
                    } else {
                        assert_eq!(
                            result.value.unwrap_err().code,
                            if reference {
                                "normalized_reference_value_depth"
                            } else {
                                "normalized_value_depth"
                            }
                        );
                    }
                }
                // Physical aliases do not discount logical raw admission occurrences:
                // 1000 outer entries + 1000 * 999 list entries is exactly 1,000,000.
                for (children, success) in [(999, true), (1000, false)] {
                    let child =
                        NormalizedValue::list(vec![NormalizedValue::I64(7); children]).unwrap();
                    let raw = map((0..1000).map(|n| (NormalizedMapKey::I64(n), child.clone())));
                    let result = invoke(
                        &program,
                        &reader,
                        reference,
                        "core.map.length",
                        &[integer, list_integer],
                        vec![raw],
                        NormalizedRunPolicy::foreground(),
                        &control,
                        None,
                    );
                    assert_eq!(
                        result.value.is_ok(),
                        success,
                        "{reference}/{children}: finite ingress is independent of cumulative quotas"
                    );
                    if success {
                        assert_eq!(result.value.unwrap(), NormalizedValue::I64(1000));
                    }
                }
                let cancelled = invoke(
                    &program,
                    &reader,
                    reference,
                    "core.map.length",
                    &[integer, integer],
                    vec![integers(4096)],
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::cancel_after_checks(37),
                    None,
                );
                assert_eq!(cancelled.value.unwrap_err().code, "execution_cancelled");
                assert_eq!(cancelled.calls, 0);
                // Malformed raw maps may nest far beyond admitted depth. Rejection and
                // remaining ownership disposal run on the same joined bounded thread.
                let mut raw = NormalizedValue::I64(1);
                for _ in 0..10_000 {
                    raw = NormalizedValue::Option(Some(Box::new(map([(
                        NormalizedMapKey::I64(0),
                        raw,
                    )]))));
                }
                let host = ControlledMapHost::new(NormalizedValue::I64(0));
                let rejected = invoke(
                    &program,
                    &reader,
                    reference,
                    "core.map.length",
                    &[integer, integer],
                    vec![map([(NormalizedMapKey::I64(0), raw)])],
                    NormalizedRunPolicy::foreground(),
                    &control,
                    Some(&host),
                );
                assert!(rejected.value.is_err());
                assert_eq!(host.calls.load(Ordering::Relaxed), 0);
                assert_eq!(
                    invoke(
                        &program,
                        &reader,
                        reference,
                        "core.map.length",
                        &[integer, integer],
                        vec![integers(2)],
                        Default::default(),
                        &control,
                        None
                    )
                    .value
                    .unwrap(),
                    NormalizedValue::I64(2)
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

struct RawOwnershipProbe {
    case: u8,
    payload: Arc<[u8]>,
}

impl RawOwnershipProbe {
    fn arguments(&self) -> (Vec<NormalizedValue>, ExecutionControl) {
        let mut deep = NormalizedValue::Bytes(Arc::clone(&self.payload));
        for _ in 0..20_000 {
            deep = NormalizedValue::Option(Some(Box::new(deep)));
        }
        let mut arguments = if self.case < 2 {
            vec![integers(2), NormalizedValue::I64(1), deep]
        } else {
            vec![integers(2), deep, NormalizedValue::I64(7)]
        };
        if self.case == 1 {
            arguments.push(NormalizedValue::Unit);
        }
        let control = if self.case == 3 {
            ExecutionControl::cancel_after_checks(1)
        } else {
            ExecutionControl::uncancelled()
        };
        if self.case == 2 {
            control.cancel();
        }
        (arguments, control)
    }
}

impl normalized::vm::NormalizedHost for RawOwnershipProbe {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &normalized::prepare::NormalizedFunction,
        implementation: &crate::platform::kernel::ImplementationName,
        types: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        _: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let (arguments, control) = self.arguments();
        normalized::vm::CoreNormalizedHost.call(
            program,
            function,
            implementation,
            types,
            arguments,
            &control,
        )
    }
}

impl NormalizedReferenceHost for RawOwnershipProbe {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        signature: &ReferenceSignature,
        implementation: &crate::platform::kernel::ImplementationName,
        types: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        _: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let (arguments, control) = self.arguments();
        normalized::reference::CoreNormalizedReferenceHost.call(
            schema,
            signature,
            implementation,
            types,
            arguments,
            &control,
        )
    }
}

#[test]
fn raw_map_hosts_release_unused_deep_fallbacks_and_cancelled_keys_iteratively() {
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(|| {
            let (program, snapshot) = fixture();
            let integer = scalar_type(&program, TypeForm::I64);
            let (index, declaration) = external(&program, "core.map.get-or", 2, 3);
            let payload: Arc<[u8]> = Arc::from([19_u8; 3]);
            for reference in [false, true] {
                for case in 0..4 {
                    let host = RawOwnershipProbe {
                        case,
                        payload: Arc::clone(&payload),
                    };
                    let arguments = vec![
                        integers(2),
                        NormalizedValue::I64(1),
                        NormalizedValue::I64(7),
                    ];
                    let control = ExecutionControl::uncancelled();
                    let result = if reference {
                        let sink = std::sync::Mutex::new(None);
                        NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                            .observing(&sink, &host)
                            .invoke_instantiated(
                                declaration,
                                &[integer, integer],
                                arguments,
                                &control,
                            )
                            .map(|(value, _)| value)
                    } else {
                        let sink = std::sync::Mutex::new(None);
                        NormalizedVm::new(&program, Default::default())
                            .observing(&sink, &host)
                            .invoke_entry(
                                NormalizedEntryPoint::InstantiatedFunction(
                                    index,
                                    Arc::from([integer, integer]),
                                ),
                                arguments,
                                None,
                                &control,
                            )
                            .map(|(value, _)| value)
                    };
                    if case == 0 {
                        assert_eq!(result.unwrap(), NormalizedValue::I64(1));
                    } else if case == 1 {
                        assert!(result.is_err());
                    } else {
                        assert_eq!(result.unwrap_err().code, "execution_cancelled");
                    }
                    assert_eq!(
                        Arc::strong_count(&payload),
                        2,
                        "the test's retained buffer and host are the only remaining owners"
                    );
                }
                assert_eq!(Arc::strong_count(&payload), 1);
                assert_eq!(
                    invoke(
                        &program,
                        &snapshot,
                        reference,
                        "core.map.get-or",
                        &[integer, integer],
                        vec![
                            integers(2),
                            NormalizedValue::I64(1),
                            NormalizedValue::I64(7)
                        ],
                        Default::default(),
                        &ExecutionControl::uncancelled(),
                        None
                    )
                    .value
                    .unwrap(),
                    NormalizedValue::I64(1)
                );
            }
        })
        .unwrap()
        .join()
        .unwrap();
}

struct OrderedMapCallbacks {
    calls: std::sync::Mutex<Vec<(String, Vec<i64>)>>,
    trap_at: Option<usize>,
}

impl OrderedMapCallbacks {
    fn observe(
        &self,
        implementation: &crate::platform::kernel::ImplementationName,
        arguments: &[NormalizedValue],
    ) -> Result<(), ExecutionError> {
        let mut calls = self.calls.lock().unwrap();
        calls.push((
            implementation.as_str().to_owned(),
            arguments
                .iter()
                .filter_map(|argument| match argument {
                    NormalizedValue::I64(value) => Some(*value),
                    _ => None,
                })
                .collect(),
        ));
        if self.trap_at == Some(calls.len()) {
            Err(ExecutionError::new(
                crate::platform::execution::ExecutionFailureClass::Trap,
                "map_order_probe",
                "controlled disposable callback trap",
            ))
        } else {
            Ok(())
        }
    }
}

impl normalized::vm::NormalizedHost for OrderedMapCallbacks {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &normalized::prepare::NormalizedFunction,
        implementation: &crate::platform::kernel::ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.observe(implementation, &arguments)?;
        normalized::vm::NormalizedHost::call(
            &normalized::vm::CoreNormalizedHost,
            program,
            function,
            implementation,
            types,
            arguments,
            control,
        )
    }
}

impl NormalizedReferenceHost for OrderedMapCallbacks {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        signature: &ReferenceSignature,
        implementation: &crate::platform::kernel::ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.observe(implementation, &arguments)?;
        normalized::reference::CoreNormalizedReferenceHost.call(
            schema,
            signature,
            implementation,
            types,
            arguments,
            control,
        )
    }
}

fn authored_map_order_fixture() -> (NormalizedProgram, KernelSnapshot) {
    let (standard_program, standard) = fixture();
    let add = external(&standard_program, "core.i64.add", 0, 2)
        .1
        .declaration;
    let get_or = external(&standard_program, "core.map.get-or", 2, 3)
        .1
        .declaration;
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("map-order"), &standard, None)
        .unwrap()
        .repository;
    let base = repository.view_current().unwrap().revision();
    let request = format!(
        r#"request base={base}
create.module as=$module name=map-order-probe
type.map as=@Map key=i64 value=i64
expression.block as=$literal-body
  (map i64 i64
    (entry (call {add} (i64 2) (i64 0)) (call {add} (i64 20) (i64 0)))
    (entry (call {add} (i64 1) (i64 0)) (call {add} (i64 10) (i64 0))))
expression.end
create.function as=$literal module=$module name=literal-order-probe visibility=public result=@Map effect=pure body=$literal-body
expression.block as=$fallback-body
  (call {get_or} (types i64 i64)
    (map i64 i64 (entry (i64 1) (i64 7)))
    (i64 1)
    (call {add} (i64 99) (i64 0)))
expression.end
create.function as=$fallback module=$module name=fallback-order-probe visibility=public result=i64 effect=pure body=$fallback-body
"#
    );
    let decoded =
        crate::platform::control::decode_compact_change("persistent-map-order", request.as_bytes())
            .unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        crate::platform::publication::PublicationOutcome::Accepted { .. }
    ));
    let compilation = crate::platform::compiler::build_clean(
        &repository,
        crate::platform::compiler::OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let linked =
        crate::platform::compiler::link_artifact(&repository, compilation.manifest_digest, &[])
            .unwrap();
    let program =
        NormalizedProgram::prepare(load_artifact(&linked.artifact.bytes).unwrap()).unwrap();
    let snapshot = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    (program, snapshot)
}

#[test]
fn map_literals_keep_authored_evaluation_order_and_get_or_evaluates_present_fallback() {
    let (program, snapshot) = authored_map_order_fixture();
    for (name, expected, expected_calls) in [
        (
            "literal-order-probe",
            map([
                (NormalizedMapKey::I64(1), NormalizedValue::I64(10)),
                (NormalizedMapKey::I64(2), NormalizedValue::I64(20)),
            ]),
            vec![
                ("core.i64.add".to_owned(), vec![2, 0]),
                ("core.i64.add".to_owned(), vec![20, 0]),
                ("core.i64.add".to_owned(), vec![1, 0]),
                ("core.i64.add".to_owned(), vec![10, 0]),
            ],
        ),
        (
            "fallback-order-probe",
            NormalizedValue::I64(7),
            vec![
                ("core.i64.add".to_owned(), vec![99, 0]),
                ("core.map.get-or".to_owned(), vec![1, 99]),
            ],
        ),
    ] {
        let declaration = snapshot
            .owners
            .iter()
            .find_map(|(owner, record)| match (owner, record) {
                (
                    crate::platform::kernel::OwnerKey::Declaration(declaration),
                    crate::platform::kernel::OwnerRecord::Declaration(record),
                ) if record.name.as_str() == name => {
                    Some(crate::platform::kernel::DeclarationReference {
                        package: snapshot.root.package_id,
                        declaration: *declaration,
                    })
                }
                _ => None,
            })
            .unwrap();
        for reference in [false, true] {
            for trap_at in [None, Some(1), Some(2)] {
                let host = OrderedMapCallbacks {
                    calls: std::sync::Mutex::new(Vec::new()),
                    trap_at,
                };
                let control = ExecutionControl::uncancelled();
                let result = if reference {
                    let sink = std::sync::Mutex::new(None);
                    let result = NormalizedReferenceInterpreter::new(
                        &snapshot,
                        &program,
                        Default::default(),
                    )
                    .observing(&sink, &host)
                    .invoke(declaration, vec![], None, &control)
                    .map(|(value, _)| value);
                    let work = sink.into_inner().unwrap().unwrap();
                    assert_eq!(
                        work.live_handles_after
                            + work.live_transactions_after
                            + work.live_call_frames_after
                            + work.live_local_scopes_after,
                        0
                    );
                    result
                } else {
                    let sink = std::sync::Mutex::new(None);
                    let result = NormalizedVm::new(&program, Default::default())
                        .observing(&sink, &host)
                        .invoke(declaration, vec![], None, &control)
                        .map(|(value, _)| value);
                    let work = sink.into_inner().unwrap().unwrap();
                    assert_eq!(
                        work.live_handles_after
                            + work.live_transactions_after
                            + work.live_call_frames_after
                            + work.live_operands_after,
                        0
                    );
                    result
                };
                if trap_at.is_some() {
                    assert_eq!(result.unwrap_err().code, "map_order_probe");
                } else {
                    assert_eq!(result.unwrap(), expected);
                }
                assert_eq!(
                    host.calls.into_inner().unwrap(),
                    expected_calls[..trap_at.unwrap_or(expected_calls.len())],
                    "{reference}/{name}"
                );
            }
        }
    }
}

#[test]
fn checked_map_text_keys_and_boxed_projections_reserve_their_actual_owned_bytes() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let text = internal_type(&mut program, &mut schema, TypeForm::Text);
    let list_integer = internal_type(&mut program, &mut schema, TypeForm::List { item: integer });
    let mut composite_type = list_integer;
    for _ in 0..12 {
        composite_type = internal_type(
            &mut program,
            &mut schema,
            TypeForm::Option {
                item: composite_type,
            },
        );
    }
    snapshot.types = schema.types.clone();
    let reader = boundary_reader(&snapshot, &schema);
    let text_key = "owned-文字列-key".repeat(31);
    let missing_key = "x".repeat(text_key.len());
    assert_eq!(text_key.len(), missing_key.len());
    let mut payload = NormalizedValue::list(vec![NormalizedValue::I64(17); 256]).unwrap();
    for _ in 0..12 {
        payload = NormalizedValue::Option(Some(Box::new(payload)));
    }
    let retained = map([(NormalizedMapKey::Text(text_key.clone()), payload.clone())]);
    let word = std::mem::size_of::<usize>() as u64;
    let boxes = 12 * std::mem::size_of::<NormalizedValue>() as u64;
    let entry_and_node = (std::mem::size_of::<NormalizedMapKey>()
        + std::mem::size_of::<NormalizedValue>()) as u64
        + 8 * word;
    for reference in [false, true] {
        let control = ExecutionControl::uncancelled();
        let arguments = |key: &str| {
            vec![
                retained.clone(),
                NormalizedValue::text(key),
                payload.clone(),
            ]
        };
        // Both paths invoke the same maintained generic get-or signature with
        // identically sized raw inputs. Missing moves its already-admitted fallback;
        // present must reserve and clone only the twelve owned payload boxes.
        let missing = invoke(
            &program,
            &reader,
            reference,
            "core.map.get-or",
            &[text, composite_type],
            arguments(&missing_key),
            Default::default(),
            &control,
            None,
        );
        assert_eq!(missing.value.unwrap(), payload);
        assert_eq!(missing.work.maps.key_bytes_copied, text_key.len() as u64);
        let projected = invoke(
            &program,
            &reader,
            reference,
            "core.map.get-or",
            &[text, composite_type],
            arguments(&text_key),
            Default::default(),
            &control,
            None,
        );
        assert_eq!(projected.value.as_ref().unwrap(), &payload);
        assert_eq!(
            projected.bytes - missing.bytes,
            boxes,
            "only the twelve owned boxes are copied; the list remains shared"
        );
        assert_eq!(projected.items, missing.items);
        let placement = result_placement_bytes(reference);
        for (bytes, success) in [
            (projected.bytes, true),
            (projected.bytes - placement - 1, false),
        ] {
            let checked = invoke(
                &program,
                &reader,
                reference,
                "core.map.get-or",
                &[text, composite_type],
                arguments(&text_key),
                NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(bytes),
                    ..Default::default()
                },
                &control,
                None,
            );
            assert_eq!(checked.value.is_ok(), success);
            if !success {
                assert_eq!(
                    checked.bytes,
                    missing.bytes - placement,
                    "projection reservation refuses before cloning boxes or installing the result slot"
                );
            }
        }
        let replaced = invoke(
            &program,
            &reader,
            reference,
            "core.map.insert",
            &[text, composite_type],
            arguments(&text_key),
            Default::default(),
            &control,
            None,
        );
        assert!(replaced.value.is_ok());
        assert_eq!(
            replaced.bytes + boxes - projected.bytes,
            entry_and_node,
            "replacement moves its admitted payload and shared old entries remain untouched"
        );
        assert_eq!(replaced.work.maps.key_bytes_copied, text_key.len() as u64);
        assert_eq!(
            invoke(
                &program,
                &reader,
                reference,
                "core.map.get-or",
                &[text, composite_type],
                arguments(&text_key),
                Default::default(),
                &control,
                None
            )
            .value
            .unwrap(),
            payload
        );
    }
}
