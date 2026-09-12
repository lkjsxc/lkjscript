//! Independent neutral expectations for the publicly authored nominal state machine.
use super::effect_probe::{execution, failure, iteration_grants, require, select};
use super::prepare::NormalizedEntryPoint;
use super::reference::{
    BoundReferenceSchema, NormalizedReferenceInterpreter, NormalizedReferenceRead,
};
use super::vm::{NormalizedRunPolicy, NormalizedVm};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::execution::ExecutionControl;
use crate::platform::normalized_lifecycle::PreparedApplication;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};

// Deliberately narrowed raw canonical scope. This reader does not consult normalized tail flags,
// prepared requirement indices or production admission proof to decide the outgoing allowance.
struct Narrow<'a> {
    source: &'a dyn NormalizedReferenceRead,
    target: crate::platform::kernel::DeclarationReference,
}
impl NormalizedReferenceRead for Narrow<'_> {
    fn binding(
        &self,
    ) -> Result<
        super::reference::NormalizedReferenceBinding,
        crate::platform::execution::ExecutionError,
    > {
        self.source.binding()
    }
    fn schema(
        &self,
    ) -> Result<
        Arc<super::reference_schema::NormalizedReferenceSchema>,
        crate::platform::execution::ExecutionError,
    > {
        self.source.schema()
    }
    fn blob(
        &self,
        digest: crate::platform::kernel::BlobObjectDigest,
    ) -> Result<Vec<u8>, crate::platform::execution::ExecutionError> {
        self.source.blob(digest)
    }
    fn owner(
        &self,
        owner: crate::platform::kernel::OwnerKey,
    ) -> Result<
        super::reference::NormalizedReferenceOwnerRead,
        crate::platform::execution::ExecutionError,
    > {
        self.owner_in_package(self.binding()?.package, owner)
    }
    fn owner_in_package(
        &self,
        package: crate::platform::kernel::PackageId,
        owner: crate::platform::kernel::OwnerKey,
    ) -> Result<
        super::reference::NormalizedReferenceOwnerRead,
        crate::platform::execution::ExecutionError,
    > {
        let mut read = self.source.owner_in_package(package, owner)?;
        if package == self.target.package
            && owner == crate::platform::kernel::OwnerKey::Declaration(self.target.declaration)
            && let Some(crate::platform::kernel::OwnerRecord::Declaration(record)) =
                &mut read.record
            && let crate::platform::kernel::DeclarationPayload::Function(function) =
                &mut record.payload
        {
            function.effect = crate::platform::kernel::FunctionEffect::Task {
                requirements: vec![],
                effect_parameters: vec![],
            };
        }
        Ok(read)
    }
}

fn outgoing_authority(prepared: &PreparedApplication) -> Result<Value, Diagnostic> {
    use crate::platform::kernel::{DeclarationPayload, FunctionEffect, OwnerKey, OwnerRecord};
    let entry = select(prepared, "iterate")?;
    let declaration = prepared.program.functions[entry.0 as usize].declaration;
    let owner = prepared
        .reference
        .owner_in_package(
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        )
        .map_err(execution)?;
    let Some(OwnerRecord::Declaration(owner)) = owner.record else {
        return Err(failure("iteration caller missing"));
    };
    let DeclarationPayload::Function(function) = owner.payload else {
        return Err(failure("iteration caller is not a graph function"));
    };
    let mut pending = vec![function.body];
    let mut target = None;
    while let Some(expression) = pending.pop() {
        let read = prepared
            .reference
            .owner_in_package(declaration.package, OwnerKey::Expression(expression))
            .map_err(execution)?;
        let Some(OwnerRecord::Expression(expression)) = read.record else {
            return Err(failure("iteration canonical expression missing"));
        };
        if let crate::platform::kernel::ExpressionOperation::Call { function, .. } =
            expression.operation
        {
            let called = prepared
                .reference
                .owner_in_package(
                    function.package,
                    OwnerKey::Declaration(function.declaration),
                )
                .map_err(execution)?;
            if matches!(called.record,Some(OwnerRecord::Declaration(ref record)) if record.name.as_str()=="task-iterate")
            {
                target = Some(function);
                break;
            }
        }
        pending.extend(
            expression
                .children()
                .into_iter()
                .map(|child| child.expression),
        );
    }
    let target =
        target.ok_or_else(|| failure("canonical caller's exact iteration helper missing"))?;
    let mut rows = Vec::new();
    for reference in [false, true] {
        let (capabilities, events) =
            iteration_grants(&prepared.program, "none", 50_000, Some((1, 31)))?;
        let control = ExecutionControl::uncancelled();
        let result = if reference {
            let read = Narrow {
                source: &prepared.reference,
                target,
            };
            NormalizedReferenceInterpreter::from_reader(
                &read,
                &prepared.program,
                Default::default(),
            )
            .invoke(
                prepared.program.functions[entry.0 as usize].declaration,
                vec![],
                Some(&capabilities),
                &control,
            )
            .map(|(value, _)| value)
        } else {
            let mut program = prepared.program.clone();
            for function in Arc::make_mut(&mut program.functions) {
                if function.declaration == target && function.effect_parameters.is_empty() {
                    function.effect = FunctionEffect::Task {
                        requirements: vec![],
                        effect_parameters: vec![],
                    };
                    function.task_requirements = Arc::from([]);
                }
            }
            NormalizedVm::new(&program, Default::default())
                .invoke_entry(
                    NormalizedEntryPoint::Function(entry),
                    vec![],
                    Some(&capabilities),
                    &control,
                )
                .map(|(value, _)| value)
        };
        let error = result.err().ok_or_else(|| {
            failure("narrow iteration helper acquired its ancestor's wider permission")
        })?;
        let events = events
            .lock()
            .map_err(|_| failure("authority events lock"))?
            .clone();
        require(
            error.message.contains("allowance") && events == ["stride", "threshold"],
            "forbidden callback executed or authority rejected at another boundary",
        )?;
        rows.push(json!({"tier":if reference {"reference"}else{"production"},"case":"outgoing-narrow-before-tail-invoke","events":events,"failure":error,"successful_result":false,"component_has_grant":true}));
    }
    Ok(json!(rows))
}

fn invoke(
    prepared: &PreparedApplication,
    reference: bool,
    stride: i64,
    threshold: i64,
    fault: &'static str,
    observed: bool,
) -> Result<Value, Diagnostic> {
    let function = select(prepared, "iterate")?;
    let signature = &prepared.program.functions[function.0 as usize];
    require(
        signature.parameters.is_empty()
            && signature.type_parameters.is_empty()
            && signature.effect_parameters.is_empty(),
        "iteration entry must be an exact zero-argument task",
    )?;
    let mut policy = NormalizedRunPolicy {
        maximum_call_depth: 64,
        ..Default::default()
    };
    if fault == "fuel" {
        policy.instruction_steps = 2_000;
    }
    // These source-bound stress budgets include preparation metadata and admit several callbacks.
    if fault == "allocation" {
        policy.maximum_allocated_bytes =
            prepared.program.work.type_metadata_bytes.max(1_000_000) + 200_000;
    }
    let (capabilities, events) = iteration_grants(
        &prepared.program,
        fault,
        if fault == "quota" { 7 } else { 50_000 },
        Some((stride, threshold)),
    )?;
    let control = ExecutionControl::uncancelled();
    let started = std::time::Instant::now();
    let (result, work) = if reference {
        let sink = Mutex::new(None);
        let interpreter = NormalizedReferenceInterpreter::from_reader(
            &prepared.reference,
            &prepared.program,
            policy,
        );
        let interpreter = if observed {
            interpreter.observing_checked(&sink)
        } else {
            interpreter
        };
        let result = interpreter
            .invoke(signature.declaration, vec![], Some(&capabilities), &control)
            .map(|(v, _)| v);
        (
            result,
            sink.into_inner()
                .map_err(|_| failure("reference iteration sink"))?
                .map_or(Value::Null, |w| json!(w)),
        )
    } else {
        let sink = Mutex::new(None);
        let vm = NormalizedVm::new(&prepared.program, policy);
        let vm = if observed {
            vm.observing_checked(&sink)
        } else {
            vm
        };
        let result = vm
            .invoke_entry(
                NormalizedEntryPoint::Function(function),
                vec![],
                Some(&capabilities),
                &control,
            )
            .map(|(v, _)| v);
        (
            result,
            sink.into_inner()
                .map_err(|_| failure("production iteration sink"))?
                .map_or(Value::Null, |w| json!(w)),
        )
    };
    let nanoseconds = started.elapsed().as_nanos();
    let events = events
        .lock()
        .map_err(|_| failure("iteration events"))?
        .clone();
    if observed {
        for name in if reference {
            &[
                "live_call_frames_after",
                "live_control_frames_after",
                "live_local_scopes_after",
                "live_type_scopes_after",
                "live_effect_scopes_after",
                "live_allowances_after",
                "live_transactions_after",
                "live_handles_after",
            ][..]
        } else {
            &[
                "live_call_frames_after",
                "live_locals_after",
                "live_type_bindings_after",
                "live_operands_after",
                "live_transactions_after",
                "live_handles_after",
            ][..]
        } {
            require(
                work[*name] == 0,
                "iteration retained invocation-owned state after completion",
            )?;
        }
        require(
            work["maximum_call_depth"].as_u64().is_some_and(|n| n <= 6)
                && work["maximum_live_locals"]
                    .as_u64()
                    .is_some_and(|n| n <= 20)
                && work["maximum_live_type_bindings"]
                    .as_u64()
                    .is_some_and(|n| n <= 6)
                && work["maximum_live_allowances"]
                    .as_u64()
                    .is_some_and(|n| n <= 6),
            "nominal terminal iteration retained growing control state",
        )?;
        if reference {
            require(
                work["maximum_live_effect_bindings"]
                    .as_u64()
                    .is_some_and(|n| n <= 4),
                "iteration retained old effect substitutions",
            )?;
        }
    }
    let expected_failure = if stride <= 0 || threshold < 0 {
        Some("integer_division")
    } else if fault == "overflow" {
        Some("integer_overflow")
    } else if fault == "none" {
        None
    } else {
        Some(fault)
    };
    let output = if let Some(case) = expected_failure {
        let error = result
            .err()
            .ok_or_else(|| failure("iteration failure emitted done"))?;
        let count = events.len();
        let correct = match case {
            "integer_division" => count == 2 && error.code.contains(case),
            "integer_overflow" => count == 4 && error.code.contains(case),
            "cancelled" => count == 5 && error.code == "execution_cancelled",
            "callback-failure" => count == 5 && error.code == "effect_probe_callback",
            "malformed-result" => count == 5 && error.code.contains("value_admission"),
            "quota" => count == 7 && error.code.contains("grant_calls"),
            "fuel" => {
                count > 2
                    && count < 8196
                    && (error.code.contains("steps") || error.code.contains("expressions"))
            }
            "allocation" => count > 2 && count < 8196 && error.code.contains("allocation"),
            _ => false,
        };
        require(
            correct,
            &format!(
                "iteration stopping/classification differs: reference={reference} case={case} count={count} error={error:?}"
            ),
        )?;
        json!({"failure":error})
    } else {
        let value = result.map_err(execution)?;
        let schema = BoundReferenceSchema {
            canonical: prepared.reference.schema().map_err(execution)?,
            value_origin: prepared.program.value_origin,
        };
        let actual = super::codec::encode_value(
            if reference {
                &schema
            } else {
                &prepared.program
            },
            &value,
            signature.result,
            Default::default(),
        )?;
        let updates = (threshold + stride - 1) / stride;
        let expected =
            json!({"position":updates * stride, "total":stride * updates * (updates - 1) / 2});
        require(
            actual == expected && events.len() == updates as usize + 3,
            "iteration complete result or final callback count differs",
        )?;
        json!({"value":actual,"updates":updates,"callbacks":updates + 1})
    };
    let expected_events = (0..events.len())
        .map(|n| match n {
            0 => "stride",
            1 => "threshold",
            _ => "tick",
        })
        .collect::<Vec<_>>();
    require(
        json!(events) == json!(expected_events),
        "configuration/callback order differs",
    )?;
    Ok(
        json!({"tier":if reference {"reference"} else {"production"},"stride":stride,"threshold":threshold,"case":fault,"observed":observed,"policy":{"maximum_call_depth":policy.maximum_call_depth,"instruction_steps":policy.instruction_steps,"maximum_allocated_bytes":policy.maximum_allocated_bytes},"execution_nanoseconds":nanoseconds,"events":events,"work":work,"output":output,"cleanup_complete":true}),
    )
}

pub(super) fn observe(prepared: &PreparedApplication) -> Result<Value, Diagnostic> {
    let mut rows = Vec::new();
    for reference in [false, true] {
        for (stride, threshold) in [
            (1, 0),
            (1, 1),
            (1, 31),
            (1, 4097),
            (1, 8193),
            (2, 33),
            (0, 3),
            (-1, 3),
            (1, -1),
        ] {
            rows.push(invoke(
                prepared, reference, stride, threshold, "none", true,
            )?);
        }
        rows.push(invoke(
            prepared,
            reference,
            1_i64 << 62,
            i64::MAX,
            "overflow",
            true,
        )?);
        for fault in [
            "cancelled",
            "callback-failure",
            "malformed-result",
            "quota",
            "fuel",
            "allocation",
        ] {
            rows.push(invoke(prepared, reference, 1, 8193, fault, true)?);
        }
        // A single selected timing cell, with one warm-up and three samples; every script is disjoint.
        rows.push(invoke(prepared, reference, 1, 8193, "none", false)?);
        for _ in 0..3 {
            rows.push(invoke(prepared, reference, 1, 8193, "none", false)?);
        }
    }
    Ok(
        json!({"rows":rows,"authority":outgoing_authority(prepared)?,"timing":"one warm-up followed by three samples per evaluator, neutral adapters, observation sink disabled","maximum_control_bound":6,"maximum_local_bound":20,"effects_replayed":false}),
    )
}
