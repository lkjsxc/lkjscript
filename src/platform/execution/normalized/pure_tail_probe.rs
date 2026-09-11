//! Contributor-only resource observations over publicly authored, accepted programs.
//! The copied executable supplies public behavior; this owner injects lower resource limits
//! and counts live evaluator ownership in a subprocess-contained, bounded-stack thread.

use super::prepare::{
    NormalizedFunction, NormalizedFunctionBody, NormalizedInstruction, NormalizedProgram,
};
use super::reference::{
    CoreNormalizedReferenceHost, NormalizedReferenceHost, NormalizedReferenceInterpreter,
    NormalizedReferenceRead, ReferenceSignature,
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
        let control = ExecutionControl::cancel_after_checks(20_000);
        let arguments = vec![raw_list((1..=8192).map(NormalizedValue::I64).collect())?, NormalizedValue::Text(Arc::from("cancelled")), NormalizedValue::I64(1), NormalizedValue::I64(0)];
        let result = NormalizedVm::new(resident.program(), NormalizedRunPolicy { maximum_call_depth: 8, ..Default::default() })
            .observing_checked(&sink).invoke_entry(super::prepare::NormalizedEntryPoint::Function(super::value::FunctionIndex(index, resident.program().value_origin)), arguments, Some(resident.deployment().capabilities()), &control);
        let error = result.err().ok_or_else(|| failure("transaction cancellation did not fail"))?;
        let observation = sink.into_inner().map_err(|_| failure("transaction observation poisoned"))?.ok_or_else(|| failure("transaction observation missing"))?;
        require(error.code == "execution_cancelled" && observation.tail_transfers > 0 && observation.capability_calls == 2 && observation.maximum_live_transactions == 1 && observation.live_transactions_after == 0 && observation.live_call_frames_after == 0 && observation.live_operands_after == 0 && observation.live_locals_after == 0 && observation.live_type_bindings_after == 0, "cancelled helper retained state or skipped staged work")?;
        let recovery_sink = Mutex::new(None);
        let recovery = NormalizedVm::new(resident.program(), NormalizedRunPolicy { maximum_call_depth: 8, ..Default::default() }).observing_checked(&recovery_sink)
            .invoke_entry(super::prepare::NormalizedEntryPoint::Function(super::value::FunctionIndex(index, resident.program().value_origin)), vec![raw_list((1..=8192).map(NormalizedValue::I64).collect())?, NormalizedValue::Text(Arc::from("after-cancel")), NormalizedValue::I64(1), NormalizedValue::I64(0)], Some(resident.deployment().capabilities()), &ExecutionControl::uncancelled())
            .map_err(|error| failure(&format!("healthy task after cancellation: {}", error.code)))?;
        require(matches!(&recovery.0, NormalizedValue::List(items) if items.len() == 8192 && items.iter().enumerate().all(|(index, value)| *value == NormalizedValue::I64(index as i64 + 1))) && recovery.1.live_transactions_after == 0, "healthy task failed after cancellation")?;
        Ok(json!({"classification":"fresh passed","failure":error,"observation":observation,"recovery_observation":recovery.1,"recovery_length":8192,"recovery_sum":33_558_528,"cancellation_checks":20_000,"stack_bytes":STACK_BYTES,"cleanup_complete":true,"effects_replayed":false}))
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

/// One source-bound production effect execution, with cancellation after a staged recursive
/// write. No reference evaluator replays these effects. The public owner checks unchanged data.
pub(crate) fn observe_recursive_transaction(
    path: &Path,
    function: &str,
) -> Result<Value, Diagnostic> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|e| failure(&e.to_string()))?;
    let deployment =
        crate::platform::deployment::PreparedDeployment::load(path, runtime.handle().clone())?;
    let resident = deployment.resident()?;
    let declaration = crate::platform::semantic_id::DeclarationId::parse(function)?;
    let index = resident
        .program()
        .functions
        .iter()
        .position(|f| {
            f.declaration.package == resident.program().root_package
                && f.declaration.declaration == declaration
        })
        .ok_or_else(|| failure("recursive write helper absent"))?;
    let ty = resident.program().functions[index]
        .parameters
        .first()
        .ok_or_else(|| failure("recursive write parameter absent"))?
        .ty;
    let input = json!({"case":"branch","value":[{"case":"leaf","value":8},{"case":"branch","value":[{"case":"leaf","value":11},{"case":"leaf","value":17}]},{"case":"branch","value":[]}]});
    let value = super::codec::decode_value(resident.program(), &input, ty, Default::default())?;
    std::thread::Builder::new().name("recursive-transaction".into()).stack_size(STACK_BYTES).spawn(move || {
        let sink=Mutex::new(None);
        let control=ExecutionControl::cancel_after_checks(20_000);
        let result=NormalizedVm::new(resident.program(),NormalizedRunPolicy::default()).observing_checked(&sink).invoke_entry(super::prepare::NormalizedEntryPoint::Function(super::value::FunctionIndex(u32::try_from(index).map_err(|_|failure("function index overflow"))?,resident.program().value_origin)),vec![value,NormalizedValue::text("cancel")],Some(resident.deployment().capabilities()),&control);
        let error=result.err().ok_or_else(||failure("recursive cancelled transaction committed"))?;
        let observation=sink.into_inner().map_err(|_|failure("recursive transaction observation poisoned"))?.ok_or_else(||failure("recursive transaction observation absent"))?;
        require(error.code=="execution_cancelled" && observation.capability_calls==3 && observation.tail_transfers>0 && observation.maximum_live_transactions==1 && observation.live_transactions_after==0 && observation.live_call_frames_after==0 && observation.live_operands_after==0 && observation.live_locals_after==0 && observation.live_type_bindings_after==0,"recursive cancellation skipped staged work or retained live execution state")?;
        Ok(json!({"source_bound":true,"effects_replayed":false,"cancellation_checks":20000,"failure":error,"observation":observation,"cleanup_complete":true}))
    }).map_err(|e|failure(&e.to_string()))?.join().map_err(|_|failure("recursive transaction observation stack failed"))?
}

pub(crate) fn observe_recursive(project: &Path) -> Result<Value, Diagnostic> {
    let started = std::time::Instant::now();
    let prepared = prepare_repository(GraphRepository::open(project)?)?;
    let preparation_nanoseconds = started.elapsed().as_nanos();
    std::thread::Builder::new().name("recursive-observations".into()).stack_size(STACK_BYTES).spawn(move || {
        let counts=(prepared.program.record_instances.len(),prepared.program.variant_instances.len(),prepared.program.types.len());
        let canonical=prepared.reference.schema().map_err(|error|failure(&format!("recursive canonical preparation: {}", error.code)))?;
        require(prepared.program.record_instances.keys().eq(canonical.record_instances.keys()) && prepared.program.variant_instances.keys().eq(canonical.variant_instances.keys()),"independent recursive instance inventories differ")?;
        let instance_edges=prepared.program.record_instances.values().map(|index| { let layout=&prepared.program.records[index.0 as usize]; layout.arguments.len()+layout.fields.len() }).sum::<usize>()
            +prepared.program.variant_instances.values().map(|index| { let layout=&prepared.program.variants[index.0 as usize]; layout.arguments.len()+layout.cases.iter().filter(|case|case.payload.is_some()).count() }).sum::<usize>();
        let structural_edges=prepared.program.types.values().map(|object|object.child_types().len()).sum::<usize>();
        let mut observations=Vec::new();
        for count in [32_i64,4096] {
            let sum=count*(count+1)/2;
            for reference in [false,true] {
                for (target,expected) in [("scale-sum",sum),("scale-mapped-sum",3*sum+5*count)] {
                    let started=std::time::Instant::now();
                    let (result,mut observation)=invocation(&prepared,reference,target,vec![NormalizedValue::I64(1),NormalizedValue::I64(count)],NormalizedRunPolicy::default(),u64::MAX)?;
                    require(result.is_ok_and(|value|value==NormalizedValue::I64(expected)),"recursive expected scale result differs")?;
                    observation["steady_nanoseconds"]=json!(started.elapsed().as_nanos());
                    observation["leaves"]=json!(count);
                    observation["expected"]=json!(expected);
                    // Optional observation sinks are disabled in this second pure execution.
                    // Required fuel/allocation accounting remains enabled in both measurements.
                    let started=std::time::Instant::now();
                    let name=Name::new(target)?;
                    let arguments=vec![NormalizedValue::I64(1),NormalizedValue::I64(count)];
                    let unobserved=if reference {
                        NormalizedReferenceInterpreter::from_reader(&prepared.reference,&prepared.program,NormalizedRunPolicy::default()).invoke_root_target(&name,arguments,None,&ExecutionControl::uncancelled()).map(|result|result.0)
                    } else {
                        NormalizedVm::new(&prepared.program,NormalizedRunPolicy::default()).invoke_root_target(&name,arguments,None,&ExecutionControl::uncancelled()).map(|result|result.0)
                    }.map_err(|error|failure(&format!("recursive observation-disabled execution: {}", error.code)))?;
                    require(unobserved==NormalizedValue::I64(expected),"recursive observation-disabled result differs")?;
                    observation["observation_sink_disabled_nanoseconds"]=json!(started.elapsed().as_nanos());
                    observations.push(observation);
                }
                let (result,mut cancelled)=invocation_control(&prepared,reference,"scale-mapped-sum",vec![NormalizedValue::I64(1),NormalizedValue::I64(count)],NormalizedRunPolicy::default(),u64::MAX,&ExecutionControl::cancel_after_checks(100))?;
                let error=result.err().ok_or_else(||failure("bounded recursive cancellation completed"))?;
                require(error.code=="execution_cancelled","recursive cancellation was misclassified")?;
                cancelled["leaves"]=json!(count);
                cancelled["failure"]=json!(error);
                observations.push(cancelled);
            }
        }
        require(counts==(prepared.program.record_instances.len(),prepared.program.variant_instances.len(),prepared.program.types.len()),"payload growth changed the type instance table")?;
        Ok(json!({"source_bound":true,"effects_replayed":false,"preparation_nanoseconds":preparation_nanoseconds,"record_instances":counts.0,"variant_instances":counts.1,"types":counts.2,"instance_edges":instance_edges,"structural_edges":structural_edges,"preparation":{"type_derivation_steps":prepared.program.work.type_derivation_steps,"type_metadata_bytes":prepared.program.work.type_metadata_bytes},"canonical_preparation":{"type_derivation_steps":canonical.type_derivation_steps,"type_metadata_bytes":canonical.type_metadata_bytes,"record_instances":canonical.record_instances.len(),"variant_instances":canonical.variant_instances.len(),"types":canonical.types.len()},"observations":observations,"cleanup_complete":true}))
    }).map_err(|e|failure(&e.to_string()))?.join().map_err(|_|failure("recursive observation stack failed"))?
}

struct ProgressHost {
    calls: AtomicU64,
    cancel_after: u64,
    mapper_items: Mutex<Vec<i64>>,
}

impl ProgressHost {
    fn mapper(
        &self,
        implementation: &ImplementationName,
        arguments: &[NormalizedValue],
    ) -> Result<(), ExecutionError> {
        if implementation.as_str() == "core.i64.multiply"
            && let Some(NormalizedValue::I64(item)) = arguments.get(1)
        {
            let mut items = self.mapper_items.lock().map_err(|_| {
                ExecutionError::resource("probe_lock", "mapper observation lock poisoned")
            })?;
            if items.len() < 16 {
                items.push(*item);
            }
        }
        Ok(())
    }
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
        self.mapper(implementation, &arguments)?;
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
        self.mapper(implementation, &arguments)?;
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
    invocation_control(
        prepared,
        reference,
        target,
        arguments,
        policy,
        cancel_after,
        &ExecutionControl::uncancelled(),
    )
}

fn invocation_control(
    prepared: &PreparedApplication,
    reference: bool,
    target: &str,
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    cancel_after: u64,
    control: &ExecutionControl,
) -> Result<(Result<NormalizedValue, ExecutionError>, Value), Diagnostic> {
    let host = ProgressHost {
        calls: AtomicU64::new(0),
        cancel_after,
        mapper_items: Mutex::new(Vec::new()),
    };
    let name = Name::new(target)?;
    let (result, observation) = if reference {
        let sink = Mutex::new(None);
        let evaluator = NormalizedReferenceInterpreter::from_reader(
            &prepared.reference,
            &prepared.program,
            policy,
        );
        let evaluator =
            if (target == "map" || target.starts_with("scale-")) && cancel_after == u64::MAX {
                evaluator.observing_checked(&sink)
            } else {
                evaluator.observing(&sink, &host)
            };
        let result = evaluator.invoke_root_target(&name, arguments, None, control);
        let observed = sink
            .into_inner()
            .map_err(|_| failure("reference observation poisoned"))?
            .ok_or_else(|| failure("reference omitted execution observation"))?;
        require(
            observed.live_call_frames_after == 0
                && observed.live_control_frames_after == 0
                && observed.live_local_scopes_after == 0
                && observed.live_type_scopes_after == 0
                && observed.live_transactions_after == 0
                && observed.live_handles_after == 0,
            "reference retained owned execution state",
        )?;
        (result.map(|(value, _)| value), json!(observed))
    } else {
        let sink = Mutex::new(None);
        let evaluator = NormalizedVm::new(&prepared.program, policy);
        let evaluator =
            if (target == "map" || target.starts_with("scale-")) && cancel_after == u64::MAX {
                evaluator.observing_checked(&sink)
            } else {
                evaluator.observing(&sink, &host)
            };
        let result = evaluator.invoke_root_target(&name, arguments, None, control);
        let observed = sink
            .into_inner()
            .map_err(|_| failure("production observation poisoned"))?
            .ok_or_else(|| failure("production omitted execution observation"))?;
        require(
            observed.live_call_frames_after == 0
                && observed.live_locals_after == 0
                && observed.live_type_bindings_after == 0
                && observed.live_operands_after == 0
                && observed.live_transactions_after == 0
                && observed.live_handles_after == 0,
            "production retained owned execution state",
        )?;
        (result.map(|(value, _)| value), json!(observed))
    };
    Ok((
        result,
        json!({"tier":if reference {"canonical-reference"} else {"production"},"target":target,"observation":observation,"host_calls":host.calls.load(Ordering::Relaxed),"cancelled":control.is_cancelled(),"mapper_items":host.mapper_items.into_inner().map_err(|_| failure("mapper trace poisoned"))?}),
    ))
}

fn matrix(prepared: PreparedApplication) -> Result<Value, Diagnostic> {
    let policy = NormalizedRunPolicy {
        maximum_call_depth: 8,
        ..NormalizedRunPolicy::default()
    };
    let mut cases = Vec::new();
    let list_type = prepared
        .program
        .types
        .iter()
        .find_map(|(identity, object)| {
            if let crate::platform::kernel::TypeForm::List { item } = object.form
                && matches!(
                    prepared.program.types.get(&item).map(|ty| &ty.form),
                    Some(crate::platform::kernel::TypeForm::I64)
                )
            {
                Some(*identity)
            } else {
                None
            }
        })
        .ok_or_else(|| failure("public mapping List<I64> type absent"))?;
    for n in [1, 32, 33, 256, 1024, 4096, 8192] {
        let started = std::time::Instant::now();
        let before = super::list::Work::current();
        let input = raw_list((0..n).map(NormalizedValue::I64).collect())?;
        let input_work = before.since();
        let input_nanoseconds = started.elapsed().as_nanos();
        let started = std::time::Instant::now();
        let before = super::list::Work::current();
        let encoded =
            super::codec::encode_typed(&prepared.program, &input, list_type, Default::default())?;
        let output_work = before.since();
        let output_nanoseconds = started.elapsed().as_nanos();
        require(
            encoded
                == serde_json::to_vec(&(0..n).collect::<Vec<_>>())
                    .map_err(|_| failure("independent boundary sequence encoding"))?,
            "list boundary encoding changed order",
        )?;
        require(
            input_work.element_handle_allocations == n as u64
                && input_work.element_handle_copies == 0
                && output_work.full_materializations == 1
                && output_work.materialized_elements == n as u64,
            "boundary conversion work was hidden or bulk construction copied payloads",
        )?;
        let NormalizedValue::List(items) = &input else {
            return Err(failure("list boundary shape"));
        };
        let started = std::time::Instant::now();
        let before = super::list::Work::current();
        let mut indexed_sum = 0_i64;
        for i in 0..n as usize {
            if let Some(NormalizedValue::I64(item)) = items.get(i) {
                indexed_sum += *item;
            }
        }
        let indexed_work = before.since();
        let indexed_nanoseconds = started.elapsed().as_nanos();
        let flat = (0..n).collect::<Vec<_>>();
        let started = std::time::Instant::now();
        let flat_sum: i64 = (0..n as usize)
            .map(|i| std::hint::black_box(&flat)[i])
            .sum();
        let flat_indexed_nanoseconds = started.elapsed().as_nanos();
        require(
            indexed_sum == n * (n - 1) / 2
                && flat_sum == indexed_sum
                && indexed_work.node_visits <= 4 * n as u64,
            "indexed list work or independent sum changed",
        )?;
        cases.push(json!({"case":"list-boundary-storage-work","n":n,"input_work":input_work,"output_work":output_work,"indexed_work":indexed_work,"input_nanoseconds":input_nanoseconds,"output_nanoseconds":output_nanoseconds,"indexed_nanoseconds":indexed_nanoseconds,"flat_indexed_nanoseconds":flat_indexed_nanoseconds,"output_bytes":encoded.len(),"sum":indexed_sum}));
    }
    for reference in [false, true] {
        for (input, expected, trap) in [
            (vec![], vec![], false),
            (vec![1, 2, 4], vec![1, 2, 4], false),
            (vec![1, i64::MAX, 4], vec![1, i64::MAX], true),
        ] {
            let (result, mut observed) = invocation(
                &prepared,
                reference,
                "map",
                vec![
                    raw_list(input.into_iter().map(NormalizedValue::I64).collect())?,
                    NormalizedValue::I64(3),
                    NormalizedValue::I64(5),
                ],
                policy,
                u64::MAX - 1,
            )?;
            require(
                result.is_err() == trap && observed["mapper_items"] == json!(expected),
                "mapper callback count, ascending order, or trap boundary changed",
            )?;
            observed["case"] = json!("list-mapper-ordered-callbacks");
            observed["expected_callbacks"] = json!(expected);
            if let Err(error) = result {
                observed["failure"] = json!(error);
            }
            cases.push(observed);
        }
        let arguments = vec![
            raw_list((0..1057).map(NormalizedValue::I64).collect())?,
            NormalizedValue::I64(3),
            NormalizedValue::I64(5),
        ];
        let (result, measured) = invocation(
            &prepared,
            reference,
            "map",
            arguments.clone(),
            policy,
            u64::MAX,
        )?;
        require(result.is_ok_and(|value| matches!(value, NormalizedValue::List(ref items) if items.len() == 1057 && items.iter().enumerate().all(|(i,v)| *v == NormalizedValue::I64(3 * i as i64 + 5)))), "mapping fixed sequence failed before budget probes")?;
        let items = measured["observation"]["collection_items"]
            .as_u64()
            .ok_or_else(|| failure("list item charge absent"))?;
        let bytes = measured["observation"]["allocated_bytes"]
            .as_u64()
            .ok_or_else(|| failure("list byte charge absent"))?;
        for (slot_limit, byte_limit, success) in [
            (items, bytes, true),
            (items - 1, bytes, false),
            (items, bytes - 1, false),
        ] {
            let bounded = NormalizedRunPolicy {
                maximum_collection_items: slot_limit,
                maximum_allocated_bytes: byte_limit,
                ..policy
            };
            let (result, mut observed) = invocation(
                &prepared,
                reference,
                "map",
                arguments.clone(),
                bounded,
                u64::MAX,
            )?;
            require(
                result.is_ok() == success,
                "list evaluator exact-fit or one-over budget disagrees",
            )?;
            observed["case"] = json!("list-evaluator-exact-bound");
            observed["slot_limit"] = json!(slot_limit);
            observed["byte_limit"] = json!(byte_limit);
            observed["success"] = json!(success);
            if let Err(error) = result {
                observed["failure"] = json!(error);
            }
            cases.push(observed);
        }
        let (result, mut observed) = invocation_control(
            &prepared,
            reference,
            "map",
            arguments.clone(),
            policy,
            u64::MAX,
            &ExecutionControl::cancel_after_checks(5000),
        )?;
        let error = result
            .err()
            .ok_or_else(|| failure("mapping cancellation did not interrupt"))?;
        require(
            error.code == "execution_cancelled"
                && observed["observation"]["value_work"]["lists"]["element_handle_allocations"]
                    .as_u64()
                    .is_some_and(|n| n > 0),
            "mapping cancellation missed real construction progress",
        )?;
        observed["case"] = json!("list-construction-cancellation");
        observed["failure"] = json!(error);
        cases.push(observed);
        require(
            matches!(&arguments[0], NormalizedValue::List(items) if items.iter().enumerate().all(|(i,v)| *v == NormalizedValue::I64(i as i64))),
            "failed mapping changed an old alias",
        )?;
        require(
            invocation(&prepared, reference, "map", arguments, policy, u64::MAX)?
                .0
                .is_ok(),
            "healthy mapping failed after cancellation",
        )?;
        for (target, expected, calls) in [
            ("binding-thunk-created", 42, 0),
            ("binding-capture-once", 9, 3),
        ] {
            let (result, mut observed) =
                invocation(&prepared, reference, target, vec![], policy, u64::MAX)?;
            require(
                result.is_ok_and(|value| value == NormalizedValue::I64(expected))
                    && observed["host_calls"] == calls,
                "binding executed its target early or evaluated a capture more than once",
            )?;
            observed["case"] = json!("binding-evaluation-order-and-count");
            observed["expected_host_calls"] = json!(calls);
            cases.push(observed);
        }
        for n in [1, 256, 4096] {
            let mut peak = None;
            let mut admission = None;
            for k in [1, 64, 1024] {
                let (result, mut observed) = invocation(
                    &prepared,
                    reference,
                    "bound-forward",
                    vec![
                        NormalizedValue::I64(k),
                        raw_list(vec![NormalizedValue::I64(1); n])?,
                    ],
                    policy,
                    u64::MAX,
                )?;
                require(
                    result.is_ok_and(|value| value == NormalizedValue::I64(n as i64)),
                    "bound tail forwarding lost its retained list",
                )?;
                let frames = observed["observation"]["maximum_call_depth"].clone();
                let capture =
                    observed["observation"]["value_work"]["capture_admission_nodes"].clone();
                require(
                    capture.as_u64().is_some_and(|visits| visits > n as u64)
                        && frames.as_u64().is_some_and(|frames| frames <= 8)
                        && admission.as_ref().is_none_or(|prior| prior == &capture)
                        && peak.as_ref().is_none_or(|prior| prior == &frames)
                        && observed["observation"]["value_work"]["internal_guard_descendant_visits"]
                            == 0,
                    "bound invocation repeated capture admission or grew live frames",
                )?;
                peak = Some(frames);
                admission = Some(capture);
                observed["case"] = json!("bound-forward-eight-frame-matrix");
                observed["n"] = json!(n);
                observed["k"] = json!(k);
                cases.push(observed);
            }
        }
        let (result, mut observed) = invocation_control(
            &prepared,
            reference,
            "bound-forward",
            vec![
                NormalizedValue::I64(64),
                raw_list(vec![NormalizedValue::I64(1); 256])?,
            ],
            policy,
            u64::MAX,
            &ExecutionControl::cancel_after_checks(700),
        )?;
        let error = result
            .err()
            .ok_or_else(|| failure("capture cancellation did not interrupt"))?;
        require(
            error.code == "execution_cancelled"
                && observed["host_calls"] == 0
                && observed["observation"]["value_work"]["capture_admission_nodes"]
                    .as_u64()
                    .is_some_and(|visits| visits > 0),
            "capture cancellation executed its target or skipped retention admission",
        )?;
        observed["case"] = json!("capture-construction-cancellation");
        observed["failure"] = json!(error);
        cases.push(observed);
        let (result, measured) = invocation(
            &prepared,
            reference,
            "bound-forward",
            vec![
                NormalizedValue::I64(1),
                raw_list(vec![NormalizedValue::I64(1); 256])?,
            ],
            policy,
            u64::MAX,
        )?;
        require(result.is_ok(), "capture allocation calibration failed")?;
        let allocated = measured["observation"]["allocated_bytes"]
            .as_u64()
            .filter(|bytes| *bytes > 0)
            .ok_or_else(|| failure("capture accounting absent"))?;
        let items = measured["observation"]["collection_items"]
            .as_u64()
            .filter(|items| *items > 0)
            .ok_or_else(|| failure("capture item accounting absent"))?;
        for (byte_limit, item_limit, accepted) in [
            (allocated, items, true),
            (allocated - 1, items, false),
            (allocated, items - 1, false),
        ] {
            let (result, mut observed) = invocation(
                &prepared,
                reference,
                "bound-forward",
                vec![
                    NormalizedValue::I64(1),
                    raw_list(vec![NormalizedValue::I64(1); 256])?,
                ],
                NormalizedRunPolicy {
                    maximum_allocated_bytes: byte_limit,
                    maximum_collection_items: item_limit,
                    ..policy
                },
                u64::MAX,
            )?;
            require(
                result.is_ok() == accepted,
                "capture exact-fit/one-over bound is not enforced",
            )?;
            if let Err(error) = result {
                require(
                    error.class == crate::platform::execution::ExecutionFailureClass::Resource,
                    "capture bound has wrong failure class",
                )?;
                observed["failure"] = json!(error);
            }
            observed["case"] = json!("capture-construction-exact-bound");
            observed["allocated_limit"] = json!(byte_limit);
            observed["item_limit"] = json!(item_limit);
            cases.push(observed);
        }
        let control = ExecutionControl::cancel_after_checks(37);
        let (result, mut observed) = invocation_control(
            &prepared,
            reference,
            "sum",
            vec![raw_list(vec![NormalizedValue::I64(1); 4096])?],
            policy,
            u64::MAX,
            &control,
        )?;
        let error = result
            .err()
            .ok_or_else(|| failure("admission cancellation escaped"))?;
        let nodes = observed["observation"]["value_work"]["input_admission_nodes"]
            .as_u64()
            .unwrap_or(0);
        require(
            error.code == "execution_cancelled"
                && nodes > 0
                && nodes < 4097
                && observed["host_calls"] == 0,
            "admission cancellation did not stop before callbacks",
        )?;
        observed["case"] = json!("input-admission-cancellation");
        observed["failure"] = json!(error);
        cases.push(observed);
        let mut raw = NormalizedValue::I64(1);
        for _ in 0..100_000 {
            raw = NormalizedValue::Option(Some(Box::new(raw)));
        }
        let (result, mut observed) = invocation(
            &prepared,
            reference,
            "sum",
            vec![raw_list(vec![raw])?],
            policy,
            u64::MAX,
        )?;
        let error = result
            .err()
            .ok_or_else(|| failure("deep malformed ingress escaped"))?;
        require(
            error.code
                == if reference {
                    "normalized_reference_value_admission"
                } else {
                    "normalized_value_admission"
                },
            "deep ingress has wrong admission failure",
        )?;
        require(
            observed["host_calls"] == 0,
            "malformed ingress called a host",
        )?;
        observed["case"] = json!("deep-raw-rejection-stack-safe-disposal");
        observed["raw_depth"] = json!(100_001);
        observed["failure"] = json!(error);
        cases.push(observed);
        for (count, success) in [(8, true), (9, false)] {
            let (result, mut observed) = invocation(
                &prepared,
                reference,
                "sum",
                vec![raw_list(vec![NormalizedValue::I64(1); count])?],
                NormalizedRunPolicy {
                    maximum_collection_items: 8,
                    ..policy
                },
                u64::MAX,
            )?;
            require(
                result.is_ok() == success,
                "raw aggregate item admission is off by one",
            )?;
            if let Err(error) = result {
                require(
                    error.class == crate::platform::execution::ExecutionFailureClass::Resource,
                    "raw item exhaustion has wrong class",
                )?;
                observed["failure"] = json!(error);
            }
            observed["case"] = json!("raw-items-exact-fit-one-over");
            observed["items"] = json!(count);
            cases.push(observed);
        }
        let mut fold_peaks = None;
        for n in [256_i64, 4096, 8192] {
            let values = (1..=n).map(NormalizedValue::I64).collect::<Vec<_>>();
            let (result, mut observation) = invocation(
                &prepared,
                reference,
                "sum",
                vec![raw_list(values)?],
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
        for target in [
            "argument-order",
            "callee-order",
            "binding-thunk-trap",
            "binding-capture-order",
            "binding-callee-order",
        ] {
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
            if target.starts_with("binding-") {
                require(
                    observation["host_calls"] == 1,
                    "failed bind continued to later expressions",
                )?;
            }
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
    // Safe wrong-prefix fault: alter the captured scalar only in disposable production code.
    let mut wrong_prefix = prepared.clone();
    for function in Arc::make_mut(&mut wrong_prefix.program.functions) {
        if let NormalizedFunctionBody::Code(code) = &mut function.body {
            for instruction in Arc::make_mut(&mut code.instructions) {
                if matches!(instruction, NormalizedInstruction::I64(4)) {
                    *instruction = NormalizedInstruction::I64(6);
                }
            }
        }
    }
    for reference in [false, true] {
        let (result, mut observed) = invocation(
            &wrong_prefix,
            reference,
            "binding-partial",
            vec![],
            policy,
            u64::MAX,
        )?;
        let value = result
            .map_err(|error| failure(&format!("wrong-prefix fault trapped: {}", error.code)))?;
        require(
            value == NormalizedValue::I64(if reference { 9 } else { 11 }),
            "wrong-prefix fixed expectation did not discriminate production from canonical meaning",
        )?;
        observed["fault"] = json!("wrong-bound-prefix-fixed-expectation");
        observed["expected"] = json!(9);
        observed["actual"] = json!(if reference { 9 } else { 11 });
        cases.push(observed);
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

fn raw_list(items: Vec<NormalizedValue>) -> Result<NormalizedValue, Diagnostic> {
    NormalizedValue::list(items).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Resource,
            "normalized_list_storage",
            error.message,
        )
    })
}
