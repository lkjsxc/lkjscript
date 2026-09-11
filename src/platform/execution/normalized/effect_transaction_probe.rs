//! One owned live-data cancellation run. The offline HTTP owner independently reads committed data.
use super::prepare::{NormalizedEntryPoint, NormalizedFunction, NormalizedProgram};
use super::value::{FunctionIndex, NormalizedValue};
use super::vm::{CoreNormalizedHost, NormalizedHost, NormalizedRunPolicy, NormalizedVm};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError};
use crate::platform::kernel::{ImplementationName, TypeObjectDigest};
use serde_json::{Value, json};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

fn failure(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "effect_transaction_probe",
        message,
    )
}

struct AfterAppend(AtomicU64);
impl NormalizedHost for AfterAppend {
    fn call(
        &self,
        program: &NormalizedProgram,
        function: &NormalizedFunction,
        implementation: &ImplementationName,
        types: &[TypeObjectDigest],
        arguments: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        if implementation.as_str() == "core.list.append"
            && !matches!(
                arguments.as_slice(),
                [NormalizedValue::List(_), NormalizedValue::I64(1001)]
            )
        {
            return Err(ExecutionError::resource(
                "effect_transaction_append",
                "selected cancellation did not follow the first callback value",
            ));
        }
        let value = CoreNormalizedHost.call(
            program,
            function,
            implementation,
            types,
            arguments,
            control,
        )?;
        if implementation.as_str() == "core.list.append" {
            self.0.fetch_add(1, Ordering::Relaxed);
            control.cancel();
        }
        Ok(value)
    }
}

pub(crate) fn observe(path: &Path, function: &str) -> Result<Value, Diagnostic> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .map_err(|error| failure(error.to_string()))?;
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
        .ok_or_else(|| failure("exact public transaction helper absent"))?;
    let signature = &resident.program().functions[index];
    if signature.parameters.len() != 1
        || !signature.type_parameters.is_empty()
        || !signature.effect_parameters.is_empty()
    {
        return Err(failure("transaction helper is not closed and unary"));
    }
    let input = json!({"mode":"transaction","prefix":1000,"values":(1..=31).map(|n|json!({"case":"item","value":{"value":n}})).collect::<Vec<_>>()});
    let value = super::codec::decode_value(
        resident.program(),
        &input,
        signature.parameters[0].ty,
        Default::default(),
    )?;
    std::thread::Builder::new().name("effect-transaction-cancellation".into()).stack_size(2_097_152).spawn(move || {
        let sink = Mutex::new(None);
        let host = AfterAppend(AtomicU64::new(0));
        let control = ExecutionControl::uncancelled();
        let result = NormalizedVm::new(resident.program(), NormalizedRunPolicy { maximum_call_depth:64, ..Default::default() })
            .observing(&sink, &host).invoke_entry(NormalizedEntryPoint::Function(FunctionIndex(u32::try_from(index).map_err(|_| failure("function index overflow"))?, resident.program().value_origin)),
                vec![value], Some(resident.deployment().capabilities()), &control);
        let error = result.err().ok_or_else(|| failure("cancelled traversal emitted a result"))?;
        let observation = sink.into_inner().map_err(|_| failure("observation poisoned"))?.ok_or_else(|| failure("observation absent"))?;
        if error.code != "execution_cancelled" || host.0.load(Ordering::Relaxed) != 1 || observation.capability_calls != 3
            || observation.maximum_live_transactions != 1 || observation.live_transactions_after != 0 || observation.live_call_frames_after != 0
            || observation.live_locals_after != 0 || observation.live_type_bindings_after != 0 || observation.live_operands_after != 0 || observation.live_handles_after != 0 {
            return Err(failure(format!("cancellation did not follow one staged callback or retained state: {error:?}; {observation:?}")));
        }
        Ok(json!({"source_bound":true,"package":resident.program().root_package.to_string(),"revision":resident.program().root_revision.to_string(),
            "function":declaration.to_string(),"input":input,"cancellation":"after first completed callback, before installing appended result",
            "failure":error,"observation":observation,"callbacks_completed":1,"effects_replayed":false,"cleanup_complete":true}))
    }).map_err(|error| failure(error.to_string()))?.join().map_err(|_| failure("transaction observation thread failed"))?
}
