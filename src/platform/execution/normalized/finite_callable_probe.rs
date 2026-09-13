//! Deterministic, disjoint adapters for the publicly authored finite callable witness.
//! This observer creates no graph meaning and never opens a live deployment or data store.
use super::capability::*;
use super::effect_probe::{execution, failure, require, select};
use super::prepare::{NormalizedEntryPoint, NormalizedProgram};
use super::reference::{NormalizedReferenceInterpreter, NormalizedReferenceRead};
use super::resource::NormalizedResourceScope;
use super::value::{NormalizedRecord, NormalizedValue, RecordLayoutIndex};
use super::vm::{NormalizedRunPolicy, NormalizedVm};
use crate::platform::diagnostic::Diagnostic;
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, Name, OperationReference, ResourceUnit, TypeForm,
};
use crate::platform::normalized_lifecycle::{PreparedApplication, prepare_repository};
use crate::platform::publication::GraphRepository;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct State {
    value: Option<Vec<u8>>,
    commits: Vec<String>,
    events: Vec<String>,
    active: usize,
}

fn script_error() -> ExecutionError {
    ExecutionError::new(
        ExecutionFailureClass::Infrastructure,
        "finite_neutral_script",
        "finite witness diverged from its independent adapter script",
    )
}

#[derive(Clone)]
struct Script {
    interface: DeclarationReference,
    operations: BTreeSet<OperationReference>,
    configuration: bool,
    entry: RecordLayoutIndex,
    state: Arc<Mutex<State>>,
    fault: &'static str,
}

impl NormalizedCapabilityAdapter for Script {
    fn kind(&self) -> NormalizedAdapterKind {
        if self.configuration {
            NormalizedAdapterKind::Configuration
        } else {
            NormalizedAdapterKind::Data
        }
    }
    fn interface(&self) -> DeclarationReference {
        self.interface
    }
    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.operations
    }
    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        if !self.configuration
            || policy.operation_name.as_str() != "text"
            || !matches!(arguments.as_slice(), [NormalizedValue::StaticText(name)] if name.as_ref() == "left-prefix")
        {
            return Err(script_error());
        }
        self.state
            .lock()
            .map_err(|_| script_error())?
            .events
            .push("configuration".into());
        Ok(NormalizedValue::Text(Arc::from("I")))
    }
    fn begin_transaction(
        &self,
        _policy: &NormalizedTransactionPolicy,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<Box<dyn NormalizedCapabilityTransaction>, ExecutionError> {
        control.check()?;
        let mut state = self.state.lock().map_err(|_| script_error())?;
        if self.configuration || state.active != 0 {
            return Err(script_error());
        }
        state.active = 1;
        state.events.push("begin".into());
        Ok(Box::new(Transaction {
            script: self.clone(),
            pending: None,
            closed: false,
        }))
    }
}

struct Transaction {
    script: Script,
    pending: Option<Vec<u8>>,
    closed: bool,
}
impl NormalizedCapabilityTransaction for Transaction {
    fn call(
        &mut self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        _resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        let mut state = self.script.state.lock().map_err(|_| script_error())?;
        if self.closed || state.active != 1 || state.events.len() > 64 {
            return Err(script_error());
        }
        match (policy.operation_name.as_str(), arguments.as_slice()) {
            ("get", [NormalizedValue::StaticText(space), NormalizedValue::List(_)])
                if space.as_ref() == "finite-callable" =>
            {
                state.events.push("get".into());
                let values = state
                    .value
                    .as_ref()
                    .map(|value| {
                        NormalizedValue::Record(NormalizedRecord::Nominal {
                            layout: self.script.entry,
                            fields: Arc::new(vec![
                                NormalizedValue::bytes(vec![state.commits.len() as u8; 32]),
                                NormalizedValue::bytes(value.clone()),
                            ]),
                        })
                    })
                    .into_iter()
                    .collect();
                NormalizedValue::list(values)
            }
            (
                "put",
                [
                    NormalizedValue::StaticText(space),
                    NormalizedValue::List(_),
                    NormalizedValue::Bytes(value),
                    NormalizedValue::Variant { payload, .. },
                ],
            ) if space.as_ref() == "finite-callable" => {
                let expected_revision = vec![state.commits.len() as u8; 32];
                if (state.value.is_none() && payload.is_some())
                    || (state.value.is_some()
                        && !matches!(payload.as_deref(), Some(NormalizedValue::Bytes(bytes)) if bytes.as_ref() == expected_revision))
                {
                    return Err(script_error());
                }
                let decoded: Value = serde_json::from_slice(value).map_err(|_| script_error())?;
                let expected: String = (0..=state.commits.len())
                    .map(|i| if i % 2 == 0 { "I7;" } else { "Tx;" })
                    .collect();
                if decoded != json!(expected) || self.pending.is_some() {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Infrastructure,
                        "finite_neutral_value",
                        format!("expected accumulated JSON text {expected:?}, received {decoded}"),
                    ));
                }
                state.events.push("put".into());
                self.pending = Some(value.to_vec());
                if self.script.fault == "cancelled" && state.commits.len() == 2 {
                    control.cancel();
                }
                Ok(NormalizedValue::Bool(true))
            }
            _ => Err(script_error()),
        }
    }
    fn commit(&mut self, control: &ExecutionControl) -> Result<(), ExecutionError> {
        control.check()?;
        let mut state = self.script.state.lock().map_err(|_| script_error())?;
        if self.closed || state.active != 1 {
            return Err(script_error());
        }
        let value = self.pending.take().ok_or_else(script_error)?;
        let label = if state.commits.len() % 2 == 0 {
            "I7;"
        } else {
            "Tx;"
        };
        state.commits.push(label.into());
        state.value = Some(value);
        state.events.push("commit".into());
        state.active = 0;
        self.closed = true;
        Ok(())
    }
    fn rollback(&mut self) -> Result<(), ExecutionError> {
        if !self.closed {
            let mut state = self.script.state.lock().map_err(|_| script_error())?;
            state.active = 0;
            state.events.push("rollback".into());
            self.pending = None;
            self.closed = true;
        }
        Ok(())
    }
}

fn grants(
    program: &NormalizedProgram,
    fault: &'static str,
) -> Result<(NormalizedCapabilities, Arc<Mutex<State>>), Diagnostic> {
    let target = program
        .root_target(&Name::new("task")?)
        .ok_or_else(|| failure("finite task target absent"))?;
    let state = Arc::new(Mutex::new(State::default()));
    let mut grants = Vec::new();
    for index in program.components[target.component.0 as usize]
        .requirements
        .iter()
    {
        let requirement = &program.requirements[index.0 as usize];
        let configuration = requirement.name.as_str() == "config";
        require(
            configuration || requirement.name.as_str() == "data",
            "unexpected finite requirement",
        )?;
        let operations: BTreeSet<_> = requirement
            .operations
            .iter()
            .map(|i| program.operations[i.0 as usize].reference)
            .collect();
        let mut entry = RecordLayoutIndex(0, program.value_origin);
        if !configuration {
            let operation = requirement
                .operations
                .iter()
                .map(|i| &program.operations[i.0 as usize])
                .find(|o| o.name.as_str() == "get")
                .ok_or_else(|| failure("get operation absent"))?;
            let item = match program.types.get(&operation.result).map(|o| &o.form) {
                Some(TypeForm::List { item }) => *item,
                _ => return Err(failure("get result type")),
            };
            let declaration = match program.types.get(&item).map(|o| &o.form) {
                Some(TypeForm::Named { declaration }) => *declaration,
                _ => return Err(failure("get entry type")),
            };
            let (index, layout) = program
                .records
                .iter()
                .enumerate()
                .find(|(_, r)| r.declaration == declaration)
                .ok_or_else(|| failure("entry layout"))?;
            require(
                layout
                    .fields
                    .iter()
                    .map(|f| f.name.as_str())
                    .eq(["revision", "value"]),
                "entry fields differ",
            )?;
            entry = RecordLayoutIndex(
                u32::try_from(index).map_err(|_| failure("entry index"))?,
                program.value_origin,
            );
        }
        let adapter = Script {
            interface: requirement.interface,
            operations: operations.clone(),
            configuration,
            entry,
            state: state.clone(),
            fault,
        };
        let mut limits: BTreeMap<_, _> = requirement
            .limits
            .iter()
            .map(|limit| {
                (
                    limit.name.clone(),
                    NormalizedGrantLimit {
                        maximum: limit.maximum,
                        unit: limit.unit,
                    },
                )
            })
            .collect();
        if fault == "quota" && !configuration {
            limits.insert(
                Name::new("maximum_calls")?,
                NormalizedGrantLimit {
                    maximum: 6,
                    unit: ResourceUnit::Calls,
                },
            );
        }
        grants.push(NormalizedCapabilityGrant {
            requirement: requirement.reference,
            descriptor: NormalizedCapabilityGrantDescriptor {
                interface: requirement.interface,
                adapter_kind: adapter.kind(),
                sharing_domain: NormalizedSharingDomain::new(format!(
                    "finite-neutral-{}",
                    requirement.name
                ))?,
                authority_revision: NormalizedGrantAuthorityRevision::of(
                    b"finite isolated neutral script",
                ),
                descriptor_digest: NormalizedGrantDescriptorDigest::of(
                    requirement.reference.requirement.to_string().as_bytes(),
                ),
                operations,
                limits,
            },
            adapter: Arc::new(adapter),
        });
    }
    Ok((
        NormalizedCapabilities::bind(program, target.component, grants).map_err(execution)?,
        state,
    ))
}

fn invoke(
    prepared: &PreparedApplication,
    reference: bool,
    fault: &'static str,
) -> Result<Value, Diagnostic> {
    let function = select(prepared, "task")?;
    let signature = &prepared.program.functions[function.0 as usize];
    require(
        signature.parameters.len() == 2
            && signature.type_parameters.is_empty()
            && signature.effect_parameters.is_empty(),
        "finite task signature differs",
    )?;
    let count = if fault == "empty" { 0 } else { 6 };
    let input = vec![
        NormalizedValue::I64(count),
        NormalizedValue::Text(Arc::from(if matches!(fault, "trap" | "empty") {
            "trap"
        } else {
            "x"
        })),
    ];
    let (capabilities, state) = grants(&prepared.program, fault)?;
    let control = ExecutionControl::uncancelled();
    let policy = NormalizedRunPolicy::default();
    let started = std::time::Instant::now();
    let (result, observation) = if reference {
        let sink = Mutex::new(None);
        let result = NormalizedReferenceInterpreter::from_reader(
            &prepared.reference,
            &prepared.program,
            policy,
        )
        .observing_checked(&sink)
        .invoke(signature.declaration, input, Some(&capabilities), &control);
        let observed = sink
            .into_inner()
            .map_err(|_| failure("reference observation poisoned"))?
            .ok_or_else(|| failure("reference observation missing"))?;
        (result.map(|r| r.0), json!(observed))
    } else {
        let sink = Mutex::new(None);
        let result = NormalizedVm::new(&prepared.program, policy)
            .observing_checked(&sink)
            .invoke_entry(
                NormalizedEntryPoint::Function(function),
                input,
                Some(&capabilities),
                &control,
            );
        let observed = sink
            .into_inner()
            .map_err(|_| failure("production observation poisoned"))?
            .ok_or_else(|| failure("production observation missing"))?;
        (result.map(|r| r.0), json!(observed))
    };
    let elapsed = started.elapsed().as_nanos();
    require(
        capabilities.shutdown().is_empty(),
        "neutral adapter cleanup failed",
    )?;
    if matches!(fault, "none" | "empty")
        && let Err(error) = &result
    {
        return Err(execution(error.clone()));
    }
    let state = state
        .lock()
        .map_err(|_| failure("neutral state poisoned"))?;
    let committed = match fault {
        "empty" => 0,
        "trap" => 1,
        "cancelled" | "quota" => 2,
        _ => 6,
    };
    let expected: Vec<_> = (0..committed)
        .map(|i| if i % 2 == 0 { "I7;" } else { "Tx;" })
        .collect();
    let mut events: Vec<&str> = Vec::new();
    for i in 0..committed {
        if i % 2 == 0 {
            events.push("configuration");
        }
        events.extend(["begin", "get", "put", "commit"]);
    }
    if fault == "cancelled" {
        events.extend(["configuration", "begin", "get", "put", "rollback"]);
    }
    if fault == "quota" {
        events.push("configuration");
    }
    require(
        state.commits == expected && state.events == events && state.active == 0,
        &format!(
            "neutral callback order or transaction cleanup differs ({fault}, reference={reference}): {:?}",
            state.events
        ),
    )?;
    let counters = observation
        .as_object()
        .ok_or_else(|| failure("observation object"))?;
    let live: Vec<_> = counters
        .iter()
        .filter(|(key, _)| key.starts_with("live_") && key.ends_with("_after"))
        .collect();
    require(
        live.len() == if reference { 8 } else { 6 }
            && live.iter().all(|(_, value)| value.as_u64() == Some(0)),
        "finite invocation retained state",
    )?;
    let output = match fault {
        "none" | "empty" => {
            let value = result.map_err(execution)?;
            require(
                matches!(&value, NormalizedValue::Text(text) if text.as_ref() == expected.concat()),
                "finite neutral value differs",
            )?;
            json!({"value": expected.concat()})
        }
        _ => {
            let error = result
                .err()
                .ok_or_else(|| failure("finite failure returned success"))?;
            require(
                match fault {
                    "trap" => {
                        error.code
                            == if reference {
                                "reference_integer_division"
                            } else {
                                "normalized_integer_division"
                            }
                    }
                    "cancelled" => error.class == ExecutionFailureClass::Cancelled,
                    "quota" => error.class == ExecutionFailureClass::Resource,
                    _ => false,
                },
                &format!("finite failure differs: {error:?}"),
            )?;
            json!({"failure": error})
        }
    };
    Ok(
        json!({"tier": if reference {"reference"} else {"production"}, "case":fault, "commits":state.commits, "events":state.events, "output":output, "observation":observation, "execution_nanoseconds":elapsed, "cleanup_complete":true}),
    )
}

pub(crate) fn observe(project: &Path) -> Result<Value, Diagnostic> {
    let started = std::time::Instant::now();
    let prepared = prepare_repository(GraphRepository::open(project)?)?;
    let preparation = started.elapsed().as_nanos();
    std::thread::Builder::new().name("finite-callable-neutral-observer".into()).stack_size(2_097_152).spawn(move || {
        let mut rows = Vec::new();
        for reference in [false, true] { for fault in ["none", "empty", "trap", "cancelled", "quota"] { rows.push(invoke(&prepared, reference, fault)?); } }
        let schema = prepared.reference.schema().map_err(execution)?;
        Ok(json!({"package":prepared.package.to_string(), "revision":prepared.revision.to_string(), "artifact":prepared.artifact_bundle.to_string(), "preparation_nanoseconds":preparation, "admission_bytes":prepared.program.work.admission_bytes, "type_derivation_steps":prepared.program.work.type_derivation_steps, "type_metadata_bytes":prepared.program.work.type_metadata_bytes, "reference_source_admission_steps":schema.source_admission_steps, "rows":rows, "live_effects_replayed":false, "cleanup_complete":true}))
    }).map_err(|error| failure(error.to_string()))?.join().map_err(|_| failure("finite observation thread failed"))?
}
