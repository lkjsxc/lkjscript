//! Source-bound observations of the public effect-library fixture. All grants below are
//! disposable neutral scripts. Live configuration/data deployments are never replayed.
use super::capability::*;
use super::prepare::{NormalizedEntryPoint, NormalizedProgram};
use super::reference::{
    BoundReferenceSchema, NormalizedReferenceInterpreter, NormalizedReferenceRead,
};
use super::resource::NormalizedResourceScope;
use super::value::{FunctionIndex, NormalizedValue};
use super::vm::{NormalizedRunPolicy, NormalizedVm};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::execution::{ExecutionControl, ExecutionError, ExecutionFailureClass};
use crate::platform::kernel::{
    DeclarationReference, Name, OperationReference, OwnerRecord, ResourceUnit,
};
use crate::platform::normalized_lifecycle::{PreparedApplication, prepare_repository};
use crate::platform::publication::GraphRepository;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

const STACK_BYTES: usize = 2_097_152;
type NeutralEvents = Arc<Mutex<Vec<String>>>;

fn failure(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, "effect_probe", message)
}
fn require(ok: bool, message: &str) -> Result<(), Diagnostic> {
    if ok { Ok(()) } else { Err(failure(message)) }
}
fn execution(error: ExecutionError) -> Diagnostic {
    failure(format!("{}: {}", error.code, error.message))
}

struct Script {
    interface: DeclarationReference,
    operations: BTreeSet<OperationReference>,
    configuration: bool,
    events: Arc<Mutex<Vec<String>>>,
    fault: &'static str,
}
impl NormalizedCapabilityAdapter for Script {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Configuration
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
        let bad = || {
            ExecutionError::new(
                ExecutionFailureClass::Infrastructure,
                "effect_probe_script",
                "unexpected neutral grant event",
            )
        };
        control.check()?;
        if !self.configuration
            || policy.operation_name.as_str() != "i64"
            || policy.requirement != policy.grant_requirement
        {
            return Err(bad());
        }
        let [NormalizedValue::StaticText(name)] = arguments.as_slice() else {
            return Err(bad());
        };
        let mut events = self.events.lock().map_err(|_| bad())?;
        if events.len() >= 32_768
            || name.as_ref()
                != if events.len() % 2 == 0 {
                    "scale"
                } else {
                    "bias"
                }
        {
            return Err(bad());
        }
        events.push(name.to_string());
        if events.len() == 3 {
            match self.fault {
                "cancelled" => control.cancel(),
                "callback-failure" => {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Capability,
                        "effect_probe_callback",
                        "selected neutral callback failure",
                    ));
                }
                "malformed-result" => return Ok(NormalizedValue::Text(Arc::from("wrong-kind"))),
                _ => {}
            }
        }
        Ok(NormalizedValue::I64(if name.as_ref() == "scale" {
            3
        } else {
            5
        }))
    }
}

fn grants(
    program: &NormalizedProgram,
    fault: &'static str,
    maximum: u64,
) -> Result<(NormalizedCapabilities, NeutralEvents), Diagnostic> {
    let target = program
        .root_target(&Name::new("serve")?)
        .ok_or_else(|| failure("serve target absent"))?;
    let events = Arc::new(Mutex::new(Vec::new()));
    let mut grants = Vec::new();
    for requirement in program.components[target.component.0 as usize]
        .requirements
        .iter()
    {
        let requirement = &program.requirements[requirement.0 as usize];
        let configuration = requirement.name.as_str() == "config";
        let operations = requirement
            .operations
            .iter()
            .map(|index| program.operations[index.0 as usize].reference)
            .collect::<BTreeSet<_>>();
        let mut limits = requirement
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
            .collect::<BTreeMap<_, _>>();
        if configuration {
            limits.insert(
                Name::new("maximum_calls")?,
                NormalizedGrantLimit {
                    maximum,
                    unit: ResourceUnit::Calls,
                },
            );
        }
        let descriptor = NormalizedCapabilityGrantDescriptor {
            interface: requirement.interface,
            adapter_kind: NormalizedAdapterKind::Configuration,
            sharing_domain: NormalizedSharingDomain::new(format!(
                "effect-probe-{}",
                requirement.name.as_str()
            ))?,
            authority_revision: NormalizedGrantAuthorityRevision::of(
                b"isolated neutral effect script",
            ),
            descriptor_digest: NormalizedGrantDescriptorDigest::of(
                requirement.reference.requirement.to_string().as_bytes(),
            ),
            operations: operations.clone(),
            limits,
        };
        grants.push(NormalizedCapabilityGrant {
            requirement: requirement.reference,
            descriptor,
            adapter: Arc::new(Script {
                interface: requirement.interface,
                operations,
                configuration,
                events: Arc::clone(&events),
                fault,
            }),
        });
    }
    Ok((
        NormalizedCapabilities::bind(program, target.component, grants).map_err(execution)?,
        events,
    ))
}

fn select(prepared: &PreparedApplication, name: &str) -> Result<FunctionIndex, Diagnostic> {
    for (index, function) in prepared.program.functions.iter().enumerate() {
        if function.declaration.package != prepared.package {
            continue;
        }
        let owner = prepared
            .reference
            .owner(crate::platform::kernel::OwnerKey::Declaration(
                function.declaration.declaration,
            ))
            .map_err(execution)?;
        if matches!(owner.record, Some(OwnerRecord::Declaration(ref record)) if record.name.as_str() == name)
        {
            return Ok(FunctionIndex(
                u32::try_from(index).map_err(|_| failure("function index"))?,
                prepared.program.value_origin,
            ));
        }
    }
    Err(failure(format!(
        "publicly authored fixture function {name} is absent"
    )))
}

fn invoke(
    prepared: &PreparedApplication,
    reference: bool,
    target: &str,
    count: i64,
    fault: &'static str,
) -> Result<Value, Diagnostic> {
    let function = select(prepared, target)?;
    let signature = &prepared.program.functions[function.0 as usize];
    require(
        signature.parameters.len() == 1
            && signature.type_parameters.is_empty()
            && signature.effect_parameters.is_empty(),
        "effect probe target is not a closed unary function",
    )?;
    let input = json!({"mode":"map", "prefix":7, "values":(1..=count).map(|n|json!({"case":"item","value":{"value":n}})).collect::<Vec<_>>()});
    let schema = BoundReferenceSchema {
        canonical: prepared.reference.schema().map_err(execution)?,
        value_origin: prepared.program.value_origin,
    };
    let value = super::codec::decode_value(
        if reference {
            &schema
        } else {
            &prepared.program
        },
        &input,
        signature.parameters[0].ty,
        Default::default(),
    )?;
    let alias = value.clone();
    let healthy_policy = NormalizedRunPolicy {
        maximum_call_depth: 64,
        ..Default::default()
    };
    let mut policy = healthy_policy;
    if fault == "fuel" {
        policy.instruction_steps = 1_000;
    }
    if fault == "allocation" {
        policy.maximum_allocated_bytes = if reference { 280_000 } else { 500_000 };
    }
    let maximum = if fault == "quota" {
        9
    } else {
        (count as u64 * 2).max(1)
    };
    let (capabilities, events) = grants(&prepared.program, fault, maximum)?;
    let control = ExecutionControl::uncancelled();
    let started = std::time::Instant::now();
    let (result, observation) = if reference {
        let sink = Mutex::new(None);
        let result = NormalizedReferenceInterpreter::from_reader(
            &prepared.reference,
            &prepared.program,
            policy,
        )
        .observing_checked(&sink)
        .invoke(
            signature.declaration,
            vec![value],
            Some(&capabilities),
            &control,
        );
        let observed = sink
            .into_inner()
            .map_err(|_| failure("reference observation poisoned"))?
            .ok_or_else(|| failure("reference observation absent"))?;
        require(
            observed.live_call_frames_after == 0
                && observed.live_control_frames_after == 0
                && observed.live_local_scopes_after == 0
                && observed.live_type_scopes_after == 0
                && observed.live_effect_scopes_after == 0
                && observed.live_allowances_after == 0
                && observed.live_transactions_after == 0
                && observed.live_handles_after == 0,
            "reference retained invocation state",
        )?;
        (result.map(|(value, _)| value), json!(observed))
    } else {
        let sink = Mutex::new(None);
        let result = NormalizedVm::new(&prepared.program, policy)
            .observing_checked(&sink)
            .invoke_entry(
                NormalizedEntryPoint::Function(function),
                vec![value],
                Some(&capabilities),
                &control,
            );
        let observed = sink
            .into_inner()
            .map_err(|_| failure("production observation poisoned"))?
            .ok_or_else(|| failure("production observation absent"))?;
        require(
            observed.live_call_frames_after == 0
                && observed.live_locals_after == 0
                && observed.live_type_bindings_after == 0
                && observed.live_operands_after == 0
                && observed.live_transactions_after == 0
                && observed.live_handles_after == 0,
            "production retained invocation state",
        )?;
        (result.map(|(value, _)| value), json!(observed))
    };
    let execution_nanoseconds = started.elapsed().as_nanos();
    let events = events
        .lock()
        .map_err(|_| failure("events poisoned"))?
        .clone();
    let expected = (1..=count).map(|n| n * 3 + 12).collect::<Vec<_>>();
    let output = if fault == "none" {
        let value = result.map_err(execution)?;
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
        let expected_output = match target {
            "folded-jobs" => json!(expected.iter().sum::<i64>()),
            "results-jobs" => json!(
                expected
                    .iter()
                    .map(|n| json!({"case":if *n == 15 {"error"} else {"ok"},"value":n}))
                    .collect::<Vec<_>>()
            ),
            _ => json!(expected),
        };
        require(
            actual == expected_output,
            "complete neutral traversal result differs",
        )?;
        require(
            events.len() == count as usize * 2,
            "neutral callback count differs",
        )?;
        let range_frames = if count == 0 {
            1
        } else {
            (count as u64).next_power_of_two().ilog2() as u64 + 1
        };
        require(
            observation["maximum_call_depth"]
                .as_u64()
                .is_some_and(|frames| frames <= range_frames + 12),
            "task traversal retained linear call depth",
        )?;
        let mut output = json!({"value":actual,"expected_range_frames":range_frames});
        if count == 8192 {
            // This second execution uses its own neutral grants, never a live deployment.
            let (unobserved, unobserved_events) =
                grants(&prepared.program, "none", count as u64 * 2)?;
            let control = ExecutionControl::uncancelled();
            let started = std::time::Instant::now();
            let value = if reference {
                NormalizedReferenceInterpreter::from_reader(
                    &prepared.reference,
                    &prepared.program,
                    policy,
                )
                .invoke(
                    signature.declaration,
                    vec![alias],
                    Some(&unobserved),
                    &control,
                )
                .map(|r| r.0)
            } else {
                NormalizedVm::new(&prepared.program, policy)
                    .invoke_entry(
                        NormalizedEntryPoint::Function(function),
                        vec![alias],
                        Some(&unobserved),
                        &control,
                    )
                    .map(|r| r.0)
            }
            .map_err(execution)?;
            output["observation_sink_disabled_nanoseconds"] = json!(started.elapsed().as_nanos());
            let value = super::codec::encode_value(
                if reference {
                    &schema
                } else {
                    &prepared.program
                },
                &value,
                signature.result,
                Default::default(),
            )?;
            require(
                value == actual
                    && *unobserved_events
                        .lock()
                        .map_err(|_| failure("unobserved events poisoned"))?
                        == events,
                "disabled observation changed the complete result or grant trace",
            )?;
        }
        output
    } else {
        let error = result
            .err()
            .ok_or_else(|| failure("failed traversal emitted a successful result"))?;
        let bounded_exhaustion = matches!(fault, "fuel" | "allocation");
        require(
            if bounded_exhaustion {
                !events.is_empty() && events.len() < count as usize * 2
            } else {
                events.len() == if fault == "quota" { 9 } else { 3 }
            },
            &format!(
                "callback stopping point differs: reference={reference}, fault={fault}, events={}, error={}",
                events.len(),
                error.code
            ),
        )?;
        require(
            match fault {
                "cancelled" => error.code == "execution_cancelled",
                "callback-failure" => error.code == "effect_probe_callback",
                "quota" => error.class == ExecutionFailureClass::Resource,
                "fuel" => {
                    error.class == ExecutionFailureClass::Resource
                        && (error.code.contains("steps") || error.code.contains("expressions"))
                }
                "allocation" => {
                    error.class == ExecutionFailureClass::Resource
                        && error.code.contains("allocation")
                }
                "malformed-result" => error.code.contains("value_admission"),
                _ => false,
            },
            &format!(
                "neutral traversal failure changed classification: reference={reference}, fault={fault}, error={}",
                error.code
            ),
        )?;
        // Reuse the old immutable input alias with a fresh neutral grant script after failure.
        let (healthy, healthy_events) = grants(&prepared.program, "none", count as u64 * 2)?;
        let control = ExecutionControl::uncancelled();
        let recovered = if reference {
            NormalizedReferenceInterpreter::from_reader(
                &prepared.reference,
                &prepared.program,
                healthy_policy,
            )
            .invoke(signature.declaration, vec![alias], Some(&healthy), &control)
            .map(|r| r.0)
        } else {
            NormalizedVm::new(&prepared.program, healthy_policy)
                .invoke_entry(
                    NormalizedEntryPoint::Function(function),
                    vec![alias],
                    Some(&healthy),
                    &control,
                )
                .map(|r| r.0)
        }
        .map_err(execution)?;
        let actual = super::codec::encode_value(
            if reference {
                &schema
            } else {
                &prepared.program
            },
            &recovered,
            signature.result,
            Default::default(),
        )?;
        require(
            actual == json!(expected)
                && healthy_events
                    .lock()
                    .map_err(|_| failure("recovery events poisoned"))?
                    .len()
                    == count as usize * 2,
            "old input alias was damaged by failed traversal",
        )?;
        json!({"failure":error,"reused_alias_complete":true})
    };
    Ok(
        json!({"tier":if reference {"reference"} else {"production"},"target":target,"count":count,"case":fault,"instruction_steps_policy":policy.instruction_steps,"allocated_bytes_policy":policy.maximum_allocated_bytes,"execution_nanoseconds":execution_nanoseconds,"observation":observation,"events":events,"output":output,"cleanup_complete":true}),
    )
}

pub(crate) fn observe(project: &Path) -> Result<Value, Diagnostic> {
    let started = std::time::Instant::now();
    let prepared = prepare_repository(GraphRepository::open(project)?)?;
    let preparation_nanoseconds = started.elapsed().as_nanos();
    std::thread::Builder::new().name("effect-library-observations".into()).stack_size(STACK_BYTES).spawn(move || {
        let mut observations = Vec::new();
        for count in [0,1,31,32,33,4097,8192] {
            for reference in [false,true] {
                for name in ["mapped-jobs", "folded-jobs", "concrete-jobs"] {
                    observations.push(invoke(&prepared, reference, name, count, "none")?);
                }
                if count == 31 { observations.push(invoke(&prepared, reference, "results-jobs", count, "none")?); }
            }
        }
        for reference in [false,true] {
            for fault in ["cancelled", "callback-failure", "quota", "malformed-result", "fuel", "allocation"] {
                observations.push(invoke(&prepared, reference, "mapped-jobs", 31, fault)?);
            }
        }
        Ok(json!({"source_bound":true,"package":prepared.package.to_string(),"revision":prepared.revision.to_string(),"artifact":prepared.artifact_bundle.to_string(),"preparation_nanoseconds":preparation_nanoseconds,"preparation":{"type_derivation_steps":prepared.program.work.type_derivation_steps,"type_metadata_bytes":prepared.program.work.type_metadata_bytes},"observations":observations,"stack_bytes":STACK_BYTES,"maximum_call_depth_policy":64,"other_policies":"unchanged defaults","live_effects_replayed":false,"cleanup_complete":true}))
    }).map_err(|error| failure(error.to_string()))?.join().map_err(|_| failure("effect observation thread failed"))?
}
