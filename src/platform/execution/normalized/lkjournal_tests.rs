//! Maintained `lkjournal` contract tests over the checked-in artifact bundle.

use super::prepare::{
    NormalizedFunctionBody, NormalizedInstruction, NormalizedProgram, NormalizedRequirement,
};
use crate::platform::compiler::load_artifact;
use crate::platform::kernel::{Name, ParameterUse};
use std::collections::BTreeSet;
use std::sync::{Arc, OnceLock};

const LKJOURNAL_ARTIFACT: &[u8] =
    include_bytes!("../../../../applications/lkjournal/generated/lkjournal.lkja");
const DATA_INTERFACE: &str = "decl_640e96fa57dee1c09557eb4bc7b53398";
const WORKER_ENTRY: &str = "decl_a914bb78de075ff44a857ac028d704f3";
const WORKER_HELPER: &str = "decl_7f443401f4946c55fa239c5430e8ad93";
const WORKER_REQUIREMENT: &str = "req_0cebded5cb056cda5484e39aa40594ad";

fn program() -> Arc<NormalizedProgram> {
    static PROGRAM: OnceLock<Arc<NormalizedProgram>> = OnceLock::new();
    Arc::clone(PROGRAM.get_or_init(|| {
        let loaded = load_artifact(LKJOURNAL_ARTIFACT).expect("strict maintained artifact");
        Arc::new(NormalizedProgram::prepare(loaded).expect("prepared maintained artifact"))
    }))
}

fn requirements<'a>(
    program: &'a NormalizedProgram,
    target: &str,
) -> Vec<&'a NormalizedRequirement> {
    let target = program
        .root_target(&Name::new(target).expect("target name"))
        .expect("maintained target");
    program.components[target.component.0 as usize]
        .requirements
        .iter()
        .map(|index| &program.requirements[index.0 as usize])
        .collect()
}

fn requirement<'a>(
    program: &'a NormalizedProgram,
    target: &str,
    name: &str,
) -> &'a NormalizedRequirement {
    requirements(program, target)
        .into_iter()
        .find(|requirement| requirement.name.as_str() == name)
        .expect("maintained requirement")
}

fn operation_names(
    program: &NormalizedProgram,
    requirement: &NormalizedRequirement,
) -> BTreeSet<String> {
    requirement
        .operations
        .iter()
        .map(|index| program.operations[index.0 as usize].name.to_string())
        .collect()
}

#[test]
fn artifact_uses_one_data_requirement_and_no_database_requirement() {
    let program = program();
    let names = requirements(&program, "serve")
        .into_iter()
        .map(|requirement| requirement.name.to_string())
        .collect::<BTreeSet<_>>();
    assert_eq!(
        names,
        [
            "bootstrap",
            "clock",
            "config",
            "data",
            "identifiers",
            "jobs",
            "objects",
            "passwords",
            "random",
            "streams",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
    assert!(!names.contains("db"));
    assert!(!names.contains("database"));
}

#[test]
fn artifact_data_requirement_is_the_complete_exact_standard_contract() {
    let program = program();
    let data = requirement(&program, "serve", "data");
    assert_eq!(data.interface.declaration.to_string(), DATA_INTERFACE);
    assert_eq!(
        operation_names(&program, data),
        [
            "delete",
            "get",
            "put",
            "scan",
            "schema-read",
            "schema-set",
            "transaction",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect()
    );
}

#[test]
fn artifact_has_no_sql_shaped_capability_operation() {
    let program = program();
    for target in ["serve", "work"] {
        for requirement in requirements(&program, target) {
            for name in operation_names(&program, requirement) {
                assert!(
                    !matches!(name.as_str(), "execute" | "query" | "migration"),
                    "{target} retained SQL-shaped operation {name}"
                );
            }
        }
    }
}

#[test]
fn artifact_service_and_worker_share_the_durable_queue_contract() {
    let program = program();
    let service_jobs = requirement(&program, "serve", "jobs");
    let worker_jobs = requirement(&program, "work", "jobs");
    assert_eq!(service_jobs.interface, worker_jobs.interface);
    assert!(operation_names(&program, service_jobs).contains("enqueue"));
    let worker_operations = operation_names(&program, worker_jobs);
    assert!(worker_operations.contains("claim"));
    assert!(worker_operations.contains("complete"));
}

#[test]
fn artifact_prepares_one_exact_worker_handoff_without_host_authority() {
    let program = program();
    let entry = program
        .functions
        .iter()
        .position(|function| function.declaration.declaration.to_string() == WORKER_ENTRY)
        .expect("maintained worker entry");
    let helper = program
        .functions
        .iter()
        .position(|function| function.declaration.declaration.to_string() == WORKER_HELPER)
        .expect("maintained worker helper");
    let helper_function = &program.functions[helper];
    assert_eq!(helper_function.parameters.len(), 2);
    assert!(helper_function.parameters[0].resource_requirement.is_none());
    assert_eq!(
        helper_function.parameters[0].use_mode,
        ParameterUse::Unrestricted
    );
    let lease = &helper_function.parameters[1];
    assert_eq!(lease.use_mode, ParameterUse::Consume);
    let requirement = lease
        .resource_requirement
        .expect("exact helper resource requirement");
    assert_eq!(
        program.requirements[requirement.0 as usize]
            .reference
            .requirement
            .to_string(),
        WORKER_REQUIREMENT
    );
    assert_eq!(helper_function.task_requirements.as_ref(), [requirement]);

    let NormalizedFunctionBody::Code(entry_code) = &program.functions[entry].body else {
        panic!("maintained worker entry must be graph code");
    };
    assert!(entry_code.instructions.windows(2).any(|instructions| {
        matches!(
            instructions,
            [
                NormalizedInstruction::LoadLocal {
                    use_mode: ParameterUse::Consume,
                    ..
                },
                NormalizedInstruction::Call {
                    function,
                    arguments: 2,
                    ..
                }
            ] if function.0 as usize == helper
        )
    }));
    assert!(entry_code.instructions.iter().all(|instruction| {
        !matches!(instruction, NormalizedInstruction::FunctionValue { function, .. } if function.0 as usize == helper)
    }));
}
