//! Read-only observations and execution for the ordinary native guide program.
//! Page prose, layout, escaping and reference selection belong to its meaning.

mod metadata;

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
        .map(metadata::template)
        .collect::<Vec<_>>();
    render(
        "operations",
        json!([
            build(snapshot),
            operations,
            templates,
            metadata::section(snapshot, RegistrySection::Runners)?
        ]),
    )
}

pub(super) fn diagnostics(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    let diagnostics = diagnostic_descriptors()
        .iter()
        .map(|item| {
            json!({"code": item.code, "class": diagnostic_class_name(item.class),
            "meaning": item.meaning, "retry": item.retry})
        })
        .collect::<Vec<_>>();
    let exits = exit_status_descriptors()
        .iter()
        .map(|item| json!({"status": item.status, "meaning": item.meaning}))
        .collect::<Vec<_>>();
    render("diagnostics", json!([build(snapshot), diagnostics, exits]))
}

pub(super) fn change_grammar(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    render(
        "change-grammar",
        json!([
            build(snapshot),
            [
                metadata::section(snapshot, RegistrySection::Change)?,
                metadata::section(snapshot, RegistrySection::Type)?,
                metadata::section(snapshot, RegistrySection::Expression)?,
            ]
        ]),
    )
}

pub(super) fn function_definition(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    render(
        "function-definition",
        json!([
            build(snapshot),
            metadata::section(snapshot, RegistrySection::Inspection)?
        ]),
    )
}

pub(super) fn deployment(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    render(
        "deployment",
        json!([
            build(snapshot),
            metadata::section(snapshot, RegistrySection::Deployment)?
        ]),
    )
}

pub(super) fn builtin_standard(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    render(
        "builtin-standard",
        json!([build(snapshot), metadata::standard()?]),
    )
}

pub(super) fn stateful_http(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    render_document(
        "stateful-http",
        json!([build(snapshot), metadata::owners()?]),
    )
}

pub(super) fn relay_information(snapshot: &CapabilitiesSnapshot) -> Result<String, String> {
    render_document(
        "relay-information",
        json!([
            build(snapshot),
            metadata::template(ProjectTemplate::NostrRelayInfo),
            metadata::owners()?,
            metadata::limits()
        ]),
    )
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

fn execute(target: &str, input: Value) -> Result<Vec<u8>, String> {
    let input = serde_json::to_vec(&input).map_err(|error| error.to_string())?;
    let target = Name::new(target).map_err(|error| error.to_string())?;
    run_pure_artifact_command(
        program()?,
        &target,
        &input,
        NormalizedCommandPolicy {
            execution: NormalizedRunPolicy::foreground(),
            ..NormalizedCommandPolicy::default()
        },
        &ExecutionControl::uncancelled(),
    )
    .map_err(|error| error.to_string())
}

fn render(target: &str, input: Value) -> Result<String, String> {
    serde_json::from_slice::<String>(&execute(target, input)?).map_err(|error| error.to_string())
}

fn render_document(target: &str, input: Value) -> Result<String, String> {
    #[derive(serde::Deserialize)]
    #[serde(deny_unknown_fields)]
    struct Document {
        valid: bool,
        content: String,
    }
    let document: Document =
        serde_json::from_slice(&execute(target, input)?).map_err(|error| error.to_string())?;
    if document.valid {
        Ok(document.content)
    } else {
        Err(document.content)
    }
}

#[cfg(test)]
mod tests;
