//! Contributor-only resource observations over publicly authored, accepted programs.
//! The copied executable supplies public behavior; this owner injects lower resource limits
//! and counts live evaluator ownership in a subprocess-contained, bounded-stack thread.

use super::prepare::{
    NormalizedFunction, NormalizedFunctionBody, NormalizedInstruction, NormalizedProgram,
};
use super::reference::{
    CoreNormalizedReferenceHost, NormalizedReferenceHost, NormalizedReferenceInterpreter,
    ReferenceSignature,
};
use super::value::NormalizedValue;
use super::value_schema::NormalizedValueSchema;
use super::vm::{CoreNormalizedHost, NormalizedHost, NormalizedRunPolicy, NormalizedVm};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{ImplementationName, Name, TypeObjectDigest};
use crate::platform::normalized_lifecycle::{PreparedApplication, prepare_repository};
use crate::platform::publication::GraphRepository;
use serde_json::{Value, json};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

const STACK_BYTES: usize = 2_097_152;

/// Inject cancellation into the publicly authored task's pure helper after a staged write.
/// This executes production effects once and reads no project authority.
pub(crate) fn observe_transaction(path: &Path, function: &str) -> Result<Value, Diagnostic> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|error| failure(&format!("transaction probe runtime: {error}")))?;
    let deployment =
        crate::platform::deployment::PreparedDeployment::load(path, runtime.handle().clone())?;
    let resident = deployment.resident()?;
    let mut empty_tasks = 0;
    for function in resident.program().functions.iter() {
        let owner = resident.program().artifact().reference_owner(
            function.declaration.package,
            crate::platform::kernel::OwnerKey::Declaration(function.declaration.declaration),
            &mut Default::default(),
            &mut Default::default(),
        )?;
        if matches!(&owner, Some(crate::platform::kernel::OwnerRecord::Declaration(record)) if matches!(&record.payload, crate::platform::kernel::DeclarationPayload::Function(body) if matches!(&body.effect, crate::platform::kernel::FunctionEffect::Task { requirements } if requirements.is_empty())))
        {
            require(
                !function.pure_graph,
                "empty task requirements incorrectly imply purity",
            )?;
            if let NormalizedFunctionBody::Code(code) = &function.body {
                require(
                    !code.instructions.iter().any(|instruction| {
                        matches!(
                            instruction,
                            NormalizedInstruction::TailCall { .. }
                                | NormalizedInstruction::TailInvoke { .. }
                        )
                    }),
                    "task body acquired tail eligibility",
                )?;
            }
            empty_tasks += 1;
        }
    }
    require(
        empty_tasks > 0,
        "empty-requirement task discrimination is absent",
    )?;
    let declaration = crate::platform::semantic_id::DeclarationId::parse(function)?;
    let index = resident
        .program()
        .functions
        .iter()
        .position(|function| {
            function.declaration.package == resident.program().root_package
                && function.declaration.declaration == declaration
        })
        .and_then(|index| u32::try_from(index).ok())
        .ok_or_else(|| failure("exact public helper is absent"))?;
    let worker = std::thread::Builder::new().name("pure-tail-transaction".to_owned()).stack_size(STACK_BYTES).spawn(move || {
        let sink = Mutex::new(None);
        let host = ProgressHost { calls: AtomicU64::new(0), cancel_after: 37 };
        let control = ExecutionControl::uncancelled();
        let arguments = vec![NormalizedValue::List(Arc::new((1..=8192).map(NormalizedValue::I64).collect())), NormalizedValue::Text(Arc::from("cancelled"))];
        let result = NormalizedVm::new(resident.program(), NormalizedRunPolicy { maximum_call_depth: 8, ..Default::default() })
            .observing(&sink, &host).invoke_entry(super::prepare::NormalizedEntryPoint::Function(super::value::FunctionIndex(index)), arguments, Some(resident.deployment().capabilities()), &control);
        let error = result.err().ok_or_else(|| failure("transaction cancellation did not fail"))?;
        let observation = sink.into_inner().map_err(|_| failure("transaction observation poisoned"))?.ok_or_else(|| failure("transaction observation missing"))?;
        require(error.code == "execution_cancelled" && observation.tail_transfers > 0 && observation.capability_calls == 2 && observation.maximum_live_transactions == 1 && observation.live_transactions_after == 0 && observation.live_call_frames_after == 0 && observation.live_operands_after == 0 && observation.live_locals_after == 0 && observation.live_type_bindings_after == 0, "cancelled helper retained state or skipped staged work")?;
        let recovery_sink = Mutex::new(None);
        let recovery = NormalizedVm::new(resident.program(), NormalizedRunPolicy { maximum_call_depth: 8, ..Default::default() }).observing(&recovery_sink, &CoreNormalizedHost)
            .invoke_entry(super::prepare::NormalizedEntryPoint::Function(super::value::FunctionIndex(index)), vec![NormalizedValue::List(Arc::new((1..=8192).map(NormalizedValue::I64).collect())), NormalizedValue::Text(Arc::from("after-cancel"))], Some(resident.deployment().capabilities()), &ExecutionControl::uncancelled())
            .map_err(|error| failure(&format!("healthy task after cancellation: {}", error.code)))?;
        require(recovery.0 == NormalizedValue::I64(33_558_528) && recovery.1.live_transactions_after == 0, "healthy task failed after cancellation")?;
        Ok(json!({"classification":"fresh passed","failure":error,"observation":observation,"recovery_observation":recovery.1,"recovery_value":33_558_528,"host_calls":host.calls.load(Ordering::Relaxed),"stack_bytes":STACK_BYTES,"cleanup_complete":true,"effects_replayed":false}))
    }).map_err(|error| failure(&format!("transaction probe thread: {error}")))?;
    worker
        .join()
        .map_err(|_| failure("transaction probe thread failed"))?
}

pub(crate) fn observe(project: &Path) -> Result<Value, Diagnostic> {
    let prepared = prepare_repository(GraphRepository::open(project)?)?;
    let worker = std::thread::Builder::new()
        .name("pure-tail-evaluators".to_owned())
        .stack_size(STACK_BYTES)
        .spawn(move || matrix(prepared))
        .map_err(|error| failure(&format!("bounded-stack thread creation: {error}")))?;
    worker
        .join()
        .map_err(|_| failure("bounded-stack evaluator thread failed"))?
}

struct ProgressHost {
    calls: AtomicU64,
    cancel_after: u64,
}

impl ProgressHost {
    fn progress(&self, control: &ExecutionControl) {
        if self.calls.fetch_add(1, Ordering::Relaxed).saturating_add(1) == self.cancel_after {
            control.cancel();
        }
    }
}

impl NormalizedHost for ProgressHost {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &NormalizedFunction,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.progress(control);
        CoreNormalizedHost.call(program, function, implementation, types, arguments, control)
    }
}

impl NormalizedReferenceHost for ProgressHost {
    fn call(
        &self,
        schema: &dyn NormalizedValueSchema,
        function: &ReferenceSignature,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.progress(control);
        CoreNormalizedReferenceHost.call(
            schema,
            function,
            implementation,
            types,
            arguments,
            control,
        )
    }
}

fn invocation(
    prepared: &PreparedApplication,
    reference: bool,
    target: &str,
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    cancel_after: u64,
) -> Result<(Result<NormalizedValue, ExecutionError>, Value), Diagnostic> {
    let control = ExecutionControl::uncancelled();
    let host = ProgressHost {
        calls: AtomicU64::new(0),
        cancel_after,
    };
    let name = Name::new(target)?;
    let (result, observation) = if reference {
        let sink = Mutex::new(None);
        let result = NormalizedReferenceInterpreter::from_reader(
            &prepared.reference,
            &prepared.program,
            policy,
        )
        .observing(&sink, &host)
        .invoke_root_target(&name, arguments, None, &control);
        let observed = sink
            .into_inner()
            .map_err(|_| failure("reference observation poisoned"))?
            .ok_or_else(|| failure("reference omitted execution observation"))?;
        require(
            observed.live_call_frames_after == 0
                && observed.live_control_frames_after == 0
                && observed.live_local_scopes_after == 0
                && observed.live_type_scopes_after == 0
                && observed.live_transactions_after == 0,
            "reference retained owned execution state",
        )?;
        (result.map(|(value, _)| value), json!(observed))
    } else {
        let sink = Mutex::new(None);
        let result = NormalizedVm::new(&prepared.program, policy)
            .observing(&sink, &host)
            .invoke_root_target(&name, arguments, None, &control);
        let observed = sink
            .into_inner()
            .map_err(|_| failure("production observation poisoned"))?
            .ok_or_else(|| failure("production omitted execution observation"))?;
        require(
            observed.live_call_frames_after == 0
                && observed.live_locals_after == 0
                && observed.live_type_bindings_after == 0
                && observed.live_operands_after == 0
                && observed.live_transactions_after == 0,
            "production retained owned execution state",
        )?;
        (result.map(|(value, _)| value), json!(observed))
    };
    Ok((
        result,
        json!({"tier":if reference {"canonical-reference"} else {"production"},"target":target,"observation":observation,"host_calls":host.calls.load(Ordering::Relaxed),"cancelled":control.is_cancelled()}),
    ))
}

fn matrix(prepared: PreparedApplication) -> Result<Value, Diagnostic> {
    let policy = NormalizedRunPolicy {
        maximum_call_depth: 8,
        ..NormalizedRunPolicy::default()
    };
    let mut cases = Vec::new();
    for reference in [false, true] {
        let mut fold_peaks = None;
        for n in [256_i64, 4096, 8192] {
            let values = (1..=n).map(NormalizedValue::I64).collect::<Vec<_>>();
            let (result, mut observation) = invocation(
                &prepared,
                reference,
                "sum",
                vec![NormalizedValue::List(Arc::new(values))],
                policy,
                u64::MAX,
            )?;
            require(
                result.is_ok_and(|value| value == NormalizedValue::I64(n * (n + 1) / 2)),
                "bounded-stack fold fixed result failed",
            )?;
            let obs = &observation["observation"];
            let peaks = (
                obs["maximum_call_depth"].clone(),
                obs["maximum_control_frames"].clone(),
                obs["maximum_live_locals"].clone(),
                obs["maximum_live_type_bindings"].clone(),
            );
            require(
                fold_peaks
                    .as_ref()
                    .is_none_or(|previous| previous == &peaks),
                "actual live ownership grows with tail-chain length",
            )?;
            fold_peaks = Some(peaks);
            observation["iterations"] = json!(n);
            cases.push(observation);
        }
        for (name, args, expected) in [
            (
                "count",
                vec![NormalizedValue::I64(8192)],
                NormalizedValue::I64(0),
            ),
            (
                "even",
                vec![NormalizedValue::I64(8192)],
                NormalizedValue::Bool(true),
            ),
            (
                "even",
                vec![NormalizedValue::I64(8191)],
                NormalizedValue::Bool(false),
            ),
            (
                "generic-i64",
                vec![
                    NormalizedValue::I64(8192),
                    NormalizedValue::I64(-17),
                    NormalizedValue::Bool(true),
                ],
                NormalizedValue::I64(-17),
            ),
            (
                "generic-bool",
                vec![
                    NormalizedValue::I64(8192),
                    NormalizedValue::Bool(true),
                    NormalizedValue::I64(-17),
                ],
                NormalizedValue::Bool(true),
            ),
            ("unselected-trap", vec![], NormalizedValue::I64(7)),
        ] {
            let (result, observation) =
                invocation(&prepared, reference, name, args, policy, u64::MAX)?;
            require(
                result.is_ok_and(|value| value == expected),
                "bounded-stack positive fixture failed",
            )?;
            cases.push(observation);
        }
        for name in ["non-tail", "pending-record", "pending-sequence"] {
            let (result, mut observation) = invocation(
                &prepared,
                reference,
                name,
                vec![NormalizedValue::I64(20)],
                policy,
                u64::MAX,
            )?;
            let error = result
                .err()
                .ok_or_else(|| failure("non-tail pending work escaped frame admission"))?;
            require(
                error.code
                    == if reference {
                        "normalized_reference_call_depth"
                    } else {
                        "normalized_call_depth"
                    },
                "non-tail frame failure has wrong diagnostic",
            )?;
            observation["failure"] = json!(error);
            cases.push(observation);
        }
        let (result, mut observation) = invocation(
            &prepared,
            reference,
            "forever",
            vec![],
            NormalizedRunPolicy {
                instruction_steps: 1000,
                ..policy
            },
            u64::MAX,
        )?;
        let error = result
            .err()
            .ok_or_else(|| failure("infinite tail recursion escaped fuel"))?;
        require(
            error.code
                == if reference {
                    "normalized_reference_expression_steps"
                } else {
                    "normalized_instruction_steps"
                },
            "infinite recursion failed at wrong bound",
        )?;
        require(
            observation["observation"][if reference {
                "expressions"
            } else {
                "instructions"
            }] == 1000,
            "tail recursion reset or fabricated work accounting",
        )?;
        observation["failure"] = json!(error);
        cases.push(observation);

        let (result, mut observation) = invocation(
            &prepared,
            reference,
            "count",
            vec![NormalizedValue::I64(8192)],
            policy,
            37,
        )?;
        let error = result
            .err()
            .ok_or_else(|| failure("deterministic cancellation did not cancel"))?;
        require(
            error.code == "execution_cancelled"
                && observation["host_calls"] == 37
                && observation["observation"]["tail_transfers"]
                    .as_u64()
                    .is_some_and(|count| count > 0),
            "cancellation did not follow measurable progress",
        )?;
        observation["failure"] = json!(error);
        cases.push(observation);

        let (result, measured) = invocation(
            &prepared,
            reference,
            "allocate",
            vec![NormalizedValue::I64(16)],
            policy,
            u64::MAX,
        )?;
        require(result.is_ok(), "allocation calibration failed")?;
        let allocated = measured["observation"]["allocated_bytes"]
            .as_u64()
            .filter(|bytes| *bytes > 1)
            .ok_or_else(|| failure("allocation fixture did not allocate"))?;
        cases.push(measured);
        for limit in [allocated, allocated - 1] {
            let (result, mut observation) = invocation(
                &prepared,
                reference,
                "allocate",
                vec![NormalizedValue::I64(16)],
                NormalizedRunPolicy {
                    maximum_allocated_bytes: limit,
                    ..policy
                },
                u64::MAX,
            )?;
            require(
                result.is_ok() == (limit == allocated),
                "cumulative allocation exact-fit/one-over failed",
            )?;
            if let Err(error) = result {
                require(
                    error.code
                        == if reference {
                            "normalized_reference_allocation"
                        } else {
                            "normalized_allocation"
                        },
                    "allocation failed at wrong bound",
                )?;
                observation["failure"] = json!(error);
            }
            observation["allocated_limit"] = json!(limit);
            cases.push(observation);
        }
        if !reference {
            let (_, measured) = invocation(
                &prepared,
                false,
                "allocate",
                vec![NormalizedValue::I64(16)],
                policy,
                u64::MAX,
            )?;
            let peak = measured["observation"]["maximum_value_stack"]
                .as_u64()
                .filter(|values| *values > 1)
                .ok_or_else(|| failure("operand fixture omitted peak"))?;
            for limit in [peak, peak - 1] {
                let limit =
                    usize::try_from(limit).map_err(|_| failure("operand limit overflow"))?;
                let (result, mut observation) = invocation(
                    &prepared,
                    false,
                    "allocate",
                    vec![NormalizedValue::I64(16)],
                    NormalizedRunPolicy {
                        maximum_value_stack: limit,
                        ..policy
                    },
                    u64::MAX,
                )?;
                require(
                    result.is_ok() == (limit as u64 == peak),
                    "operand exact-fit/one-over failed",
                )?;
                if let Err(error) = result {
                    require(
                        error.code == "normalized_value_stack",
                        "operand failed at wrong bound",
                    )?;
                    observation["failure"] = json!(error);
                }
                observation["operand_limit"] = json!(limit);
                cases.push(observation);
            }
        }
        for target in ["argument-order", "callee-order"] {
            let (result, mut observation) =
                invocation(&prepared, reference, target, vec![], policy, u64::MAX)?;
            let error = result
                .err()
                .ok_or_else(|| failure("ordered trap did not trap"))?;
            require(
                error.code
                    == if reference {
                        "reference_integer_division"
                    } else {
                        "normalized_integer_division"
                    },
                "later argument ran before selected early trap",
            )?;
            observation["failure"] = json!(error);
            cases.push(observation);
        }
        let (result, observation) = invocation(
            &prepared,
            reference,
            "count",
            vec![NormalizedValue::I64(8192)],
            policy,
            u64::MAX,
        )?;
        require(
            result.is_ok_and(|value| value == NormalizedValue::I64(0)),
            "fresh healthy invocation failed after exhaustion/cancellation",
        )?;
        cases.push(observation);
    }
    // Safe fault: remove derived tail dispatch only in this private prepared copy. Canonical
    // reference execution must still succeed; production must now hit the independent bound.
    let mut faulty = prepared.clone();
    for function in Arc::make_mut(&mut faulty.program.functions) {
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            for instruction in Arc::make_mut(&mut code.instructions) {
                *instruction = match instruction {
                    NormalizedInstruction::TailCall {
                        function,
                        type_arguments,
                        arguments,
                    } => NormalizedInstruction::Call {
                        function: *function,
                        type_arguments: type_arguments.clone(),
                        arguments: *arguments,
                    },
                    NormalizedInstruction::TailInvoke { arguments } => {
                        NormalizedInstruction::Invoke {
                            arguments: *arguments,
                        }
                    }
                    _ => continue,
                };
            }
        }
    }
    let (result, mut fault) = invocation(
        &faulty,
        false,
        "count",
        vec![NormalizedValue::I64(8192)],
        policy,
        u64::MAX,
    )?;
    require(
        result.is_err_and(|error| error.code == "normalized_call_depth"),
        "oracle failed to detect forced ordinary frame growth",
    )?;
    fault["fault"] = json!("forced-ordinary-frame-growth-detected");
    cases.push(fault);
    let (result, mut fault) = invocation(
        &faulty,
        true,
        "count",
        vec![NormalizedValue::I64(8192)],
        policy,
        u64::MAX,
    )?;
    require(
        result.is_ok_and(|value| value == NormalizedValue::I64(0)),
        "reference read production dispatch fault",
    )?;
    fault["fault"] = json!("canonical-reference-independent-of-dispatch");
    cases.push(fault);
    Ok(
        json!({"classification":"fresh passed","stack_bytes":STACK_BYTES,"call_frame_limit":8,"cases":cases,"cleanup_complete":true,"reference_operand_model":"canonical expression/value accounting; no VM operand stack"}),
    )
}

fn require(condition: bool, message: &str) -> Result<(), Diagnostic> {
    if condition {
        Ok(())
    } else {
        Err(failure(message))
    }
}

fn failure(message: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, "pure_tail_probe", message)
}
