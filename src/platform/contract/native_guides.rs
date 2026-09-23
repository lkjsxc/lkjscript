//! Read-only registry input for the ordinary native guide program.
//!
//! Layout and escaping belong to tools/native-guides/project. This adapter does
//! not render prose or tables, load a host interpreter, or open a user project.

use super::registry::{
    CapabilitiesSnapshot, RegistrySection, diagnostic_class_name, diagnostic_descriptors,
    exit_status_descriptors, operation_descriptors,
};
use crate::platform::compiler::load_artifact;
use crate::platform::execution::ExecutionControl;
use crate::platform::execution::normalized::{
    NormalizedCommandPolicy, NormalizedProgram, NormalizedRunPolicy, run_pure_artifact_command,
};
use crate::platform::kernel::Name;
use crate::platform::project_creation::ProjectTemplate;
use serde_json::{Value, json};
use std::sync::OnceLock;

const ARTIFACT: &[u8] = include_bytes!("../../../tools/native-guides/generated/guides.lkja");
static PROGRAM: OnceLock<Result<NormalizedProgram, String>> = OnceLock::new();

fn build(snapshot: &CapabilitiesSnapshot) -> Value {
    json!({
        "product": snapshot.product_name,
        "version": snapshot.product_version,
        "capabilities": snapshot.digest,
    })
}

pub(super) fn operations(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    let operations = operation_descriptors()
        .iter()
        .map(|item| {
            json!({
                "name": item.operation.name(),
                "request": item.request_model.name(),
                "response": item.response_model.name(),
                "authority": item.authority_effect.name(),
                "usage": item.usage,
            })
        })
        .collect::<Vec<_>>();
    let templates = ProjectTemplate::ALL
        .into_iter()
        .map(|item| {
            json!({
                "name": item.name(),
                "purpose": item.purpose(),
                "runner": item.runner(),
                "deployment": item.emits_deployment(),
                "artifact": item.recommended_artifact_output().unwrap_or("none"),
            })
        })
        .collect::<Vec<_>>();
    let runners = snapshot
        .section(RegistrySection::Runners)
        .ok_or_else(|| "generated operations lack runner observations".to_owned())?;
    let runners = std::str::from_utf8(&runners.bytes)
        .map_err(|_| "runner observations are not UTF-8".to_owned())?;
    render(
        "operations",
        json!([build(snapshot), operations, templates, runners]),
    )
}

pub(super) fn diagnostics(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    let diagnostics = diagnostic_descriptors()
        .iter()
        .map(|item| {
            json!({
                "code": item.code,
                "class": diagnostic_class_name(item.class),
                "meaning": item.meaning,
                "retry": item.retry,
            })
        })
        .collect::<Vec<_>>();
    let exits = exit_status_descriptors()
        .iter()
        .map(|item| json!({"status": item.status, "meaning": item.meaning}))
        .collect::<Vec<_>>();
    render("diagnostics", json!([build(snapshot), diagnostics, exits]))
}

fn program() -> Result<&'static NormalizedProgram, String> {
    PROGRAM
        .get_or_init(|| {
            let artifact = load_artifact(ARTIFACT).map_err(|error| error.to_string())?;
            NormalizedProgram::prepare_with_control(artifact, &ExecutionControl::uncancelled())
                .map_err(|error| error.to_string())
        })
        .as_ref()
        .map_err(Clone::clone)
}

fn render(target: &str, input: Value) -> Result<String, String> {
    let input = serde_json::to_vec(&input).map_err(|error| error.to_string())?;
    let target = Name::new(target).map_err(|error| error.to_string())?;
    let result = run_pure_artifact_command(
        program()?,
        &target,
        &input,
        NormalizedCommandPolicy {
            execution: NormalizedRunPolicy::foreground(),
            ..NormalizedCommandPolicy::default()
        },
        &ExecutionControl::uncancelled(),
    )
    .map_err(|error| error.to_string())?;
    serde_json::from_slice::<String>(&result).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests;
