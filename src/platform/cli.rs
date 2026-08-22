//! Strict, bounded public graph-native command projection.

use super::artifact::{MAXIMUM_ARTIFACT_BYTES, load_artifact};
use super::bootstrap::{
    ProjectTemplate, builtin_package_info, create_project, export_builtin_standard,
};
use super::deployment::{MAXIMUM_DEPLOYMENT_BYTES, decode_deployment};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{PreparedProgram, ReferenceInterpreter, RunPolicy, Vm};
use super::json::{JsonLimits, decode_strict, decode_typed, encode_typed};
use super::meaning::{GRAPH_CONTRACT_IDENTITY, RelationRole};
use super::package::RunnerKind;
use super::repository::{
    BACKUP_SEGMENT_ENTRY_LIMIT, MAXIMUM_BACKUP_MANIFEST_BYTES, MAXIMUM_BACKUP_SEGMENT_BYTES,
    SemanticRepository,
};
use super::revision::{AffectedOwner, TransactionReceipt, ValidationFacts};
use super::semantic_change::{ChangeRequest, execute_change};
use super::semantic_diff::diff_revisions;
use super::semantic_digest::{ReceiptDigest, SemanticDiffDigest, TransactionDigest};
use super::semantic_draft::SemanticDraftStore;
use super::semantic_id::{DraftId, ModuleId, RepositoryId, RevisionId};
use super::semantic_merge::{
    SEMANTIC_MERGE_CONTRACT_VERSION, SemanticMergeRequest, SemanticMergeResult,
    SemanticMergeStatus, merge_revisions,
};
use super::semantic_projection::{MAXIMUM_REVIEW_PROJECTION_BYTES, render_review_projection};
use super::semantic_query::{OwnerKind, QueryBudget, SemanticQueryIndex};
use super::semantic_transaction::{
    TransactionMode, TransactionRequest, TransactionResult, TransactionStatus,
};
use super::workspace::{DEFAULT_ORIENTATION_ITEMS, SemanticWorkspace};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Instant;

pub const CLI_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_CLI_RESPONSE_BYTES: usize = 4 * 1_048_576;
pub const MAXIMUM_TRANSACTION_REQUEST_BYTES: usize = 16 * 1_048_576;
const MAXIMUM_INLINE_AFFECTED_OWNERS: usize = 64;

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CliSuccess {
    pub contract_version: u16,
    pub ok: bool,
    pub status: &'static str,
    pub command: String,
    pub result: serde_json::Value,
}

impl CliSuccess {
    pub fn process_exit_code(&self) -> u8 {
        match self.status {
            "stale_base" | "stale_head" => 7,
            "invalid_graph" => 8,
            "resource_exhausted" => 4,
            "precondition_failed" | "foreign_identity" | "conflicted" => 2,
            _ => 0,
        }
    }
}

pub fn execute(arguments: Vec<String>) -> Result<CliSuccess, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    let command = arguments
        .first()
        .map(String::as_str)
        .unwrap_or("capabilities");
    match command {
        "capabilities" => capabilities_command(&arguments[1..]),
        "new" => new_project_command(&arguments[1..]),
        "inspect" => inspect_command(&arguments[1..], project.as_deref()),
        "query" => public_query_command(&arguments[1..], project.as_deref()),
        "change" => change_command(&arguments[1..], project.as_deref()),
        "draft" => public_draft_command(&arguments[1..], project.as_deref()),
        "history" => public_history_command(&arguments[1..], project.as_deref()),
        "package" => package_command(&arguments[1..], project.as_deref()),
        "check" => {
            exact_arguments(&arguments, 1, "check")?;
            let workspace = open_workspace(project.as_deref())?;
            run_package_tests(&workspace.prepare()?)
        }
        "build" => build_command(&arguments[1..], project.as_deref()),
        "run" => run_target_command(&arguments[1..], project.as_deref()),
        "review" => text_projection_command("review", &arguments[1..], project.as_deref()),
        "backup" => backup_command("backup", &arguments[1..], project.as_deref()),
        "restore" => restore_command(&arguments[1..], project.as_deref()),
        "doctor" => doctor_command(&arguments[1..], project.as_deref()),
        other => Err(usage_error(format!(
            "unknown command '{other}'; use 'capabilities'"
        ))),
    }
}

fn new_project_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    let destination = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("new requires one destination directory"))?;
    ensure_options(&arguments[1..], &["--template", "--name"], &[])?;
    let template = ProjectTemplate::parse(
        option_value(&arguments[1..], "--template")?
            .as_deref()
            .unwrap_or("minimal"),
    )?;
    let package_name = option_value(&arguments[1..], "--name")?.unwrap_or_else(|| {
        Path::new(destination)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("app")
            .to_owned()
    });
    serialized(
        "new",
        &create_project(Path::new(destination), &package_name, template)?,
    )
}

fn builtin_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    match arguments.first().map(String::as_str) {
        Some("inspect") => {
            exact_arguments(arguments, 1, "builtin inspect")?;
            serialized("builtin.inspect", &builtin_package_info()?)
        }
        Some("export") => {
            let output = option_value(&arguments[1..], "--output")?
                .ok_or_else(|| usage_error("builtin export requires --output PATH"))?;
            ensure_options(&arguments[1..], &["--output"], &[])?;
            let (package, bytes) = export_builtin_standard(Path::new(&output))?;
            success(
                "builtin.export",
                json!({"package": package, "output": output, "bytes": bytes}),
            )
        }
        Some(other) => Err(usage_error(format!(
            "unknown builtin action '{other}'; expected inspect or export"
        ))),
        None => Err(usage_error("builtin requires inspect or export")),
    }
}

fn inspect_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let action = arguments.first().map(String::as_str).ok_or_else(|| {
        usage_error("inspect requires status, project, owner, targets, artifact, or revision")
    })?;
    match action {
        "status" => {
            exact_arguments(arguments, 1, "inspect status")?;
            let workspace = open_workspace(project)?;
            serialized("inspect.status", &workspace.status()?)
        }
        "project" => {
            let limit = optional_usize(&arguments[1..], "--limit", DEFAULT_ORIENTATION_ITEMS)?;
            ensure_options(&arguments[1..], &["--limit"], &[])?;
            let workspace = open_workspace(project)?;
            serialized("inspect.project", &workspace.orient(limit)?)
        }
        "owner" => show_command(&arguments[1..], project),
        "targets" => targets_command(&arguments[1..], project),
        "artifact" => artifact_inspect_command(&arguments[1..]),
        "deployment" => deployment_inspect_command(&arguments[1..]),
        "revision" => revision_show_command(&arguments[1..], project),
        other => Err(usage_error(format!(
            "unknown inspect action '{other}'; use 'capabilities inspect'"
        ))),
    }
}

fn deployment_inspect_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error(
            "inspect deployment requires one descriptor path",
        ));
    }
    let bytes = read_bounded(
        Path::new(&arguments[0]),
        MAXIMUM_DEPLOYMENT_BYTES,
        "deployment descriptor",
    )?;
    serialized("inspect.deployment", &decode_deployment(&bytes)?)
}

fn public_query_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let action = arguments.first().map(String::as_str).ok_or_else(|| {
        usage_error(
            "query requires owners, find, relations, callers, callees, types, capabilities, context, impact, or request",
        )
    })?;
    match action {
        "owners" => owners_command(&arguments[1..], project),
        "find" => find_command(&arguments[1..], project),
        "relations" => relation_command(
            "query.relations",
            &arguments[1..],
            project,
            true,
            true,
            BTreeSet::new(),
        ),
        "callers" => relation_command(
            "query.callers",
            &arguments[1..],
            project,
            true,
            false,
            BTreeSet::from([RelationRole::Call]),
        ),
        "callees" => relation_command(
            "query.callees",
            &arguments[1..],
            project,
            false,
            true,
            BTreeSet::from([RelationRole::Call]),
        ),
        "types" => relation_command(
            "query.types",
            &arguments[1..],
            project,
            true,
            true,
            BTreeSet::from([
                RelationRole::TypeUse,
                RelationRole::FieldUse,
                RelationRole::VariantConstruction,
                RelationRole::VariantPattern,
            ]),
        ),
        "capabilities" => relation_command(
            "query.capabilities",
            &arguments[1..],
            project,
            true,
            true,
            BTreeSet::from([
                RelationRole::CapabilityInterface,
                RelationRole::CapabilityOperation,
            ]),
        ),
        "context" => context_command(&arguments[1..], project, false),
        "impact" => context_command(&arguments[1..], project, true),
        "request" => query_command(&arguments[1..], project),
        other => Err(usage_error(format!(
            "unknown query action '{other}'; use 'capabilities query'"
        ))),
    }
}

fn public_draft_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let action = arguments.first().map(String::as_str).ok_or_else(|| {
        usage_error("draft requires create, status, append, rebase, publish, or drop")
    })?;
    match action {
        "create" => draft_create_command(&arguments[1..], project),
        "status" => draft_status_command(&arguments[1..], project),
        "append" => transaction_command(&arguments[1..], project, TransactionMode::Apply),
        "rebase" => draft_rebase_command(&arguments[1..], project),
        "publish" => draft_publish_command(&arguments[1..], project),
        "drop" => draft_drop_command(&arguments[1..], project),
        other => Err(usage_error(format!(
            "unknown draft action '{other}'; use 'capabilities draft'"
        ))),
    }
}

fn public_history_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let action = arguments
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage_error("history requires list, show, diff, or merge"))?;
    match action {
        "list" => history_command(&arguments[1..], project),
        "show" => revision_show_command(&arguments[1..], project),
        "diff" => diff_command(&arguments[1..], project),
        "merge" => merge_command(&arguments[1..], project),
        other => Err(usage_error(format!(
            "unknown history action '{other}'; use 'capabilities history'"
        ))),
    }
}

fn package_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let action = arguments
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage_error("package requires stage or builtin"))?;
    match action {
        "stage" => dependency_stage_command(&arguments[1..], project),
        "builtin" => builtin_command(&arguments[1..]),
        other => Err(usage_error(format!(
            "unknown package action '{other}'; use 'capabilities package'"
        ))),
    }
}

fn change_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(
        arguments,
        &["--request", "--request-file"],
        &["--dry-run", "--commit"],
    )?;
    let dry_run = flag_present(arguments, "--dry-run")?;
    let commit = flag_present(arguments, "--commit")?;
    if dry_run && commit {
        return Err(usage_error(
            "change accepts only one of --dry-run or --commit",
        ));
    }
    let inline = option_value(arguments, "--request")?;
    let file = option_value(arguments, "--request-file")?;
    let bytes = match (inline, file) {
        (Some(value), None) if value.len() <= MAXIMUM_TRANSACTION_REQUEST_BYTES => {
            value.into_bytes()
        }
        (Some(_), None) => {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "change_request_limit",
                format!("change request exceeds {MAXIMUM_TRANSACTION_REQUEST_BYTES} bytes"),
            ));
        }
        (None, Some(path)) => read_bounded(
            Path::new(&path),
            MAXIMUM_TRANSACTION_REQUEST_BYTES,
            "change request",
        )?,
        (Some(_), Some(_)) => {
            return Err(usage_error(
                "supply exactly one of --request or --request-file",
            ));
        }
        (None, None) => {
            return Err(usage_error(
                "change requires --request JSON or --request-file PATH",
            ));
        }
    };
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    let request = ChangeRequest::deserialize(&mut deserializer).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "change_request",
            format!("change request is not strict current JSON: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "change_request_trailing",
            format!("change request has trailing input: {error}"),
        )
    })?;
    let workspace = open_workspace(project)?;
    let change = execute_change(workspace.repository(), &request, commit)?;
    let mut output = transaction_result("change", &change.transaction)?;
    let fields = output.result.as_object_mut().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "change_projection",
            "normalized transaction projection was not an object",
        )
    })?;
    fields.insert(
        "change_contract_version".to_owned(),
        serde_json::Value::from(change.contract_version),
    );
    fields.insert(
        "base_revision".to_owned(),
        serde_json::to_value(change.base_revision).map_err(internal_json)?,
    );
    fields.insert(
        "allocated_identities".to_owned(),
        serde_json::to_value(change.allocated_identities).map_err(internal_json)?,
    );
    Ok(output)
}

fn capabilities_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    let commands = command_registry();
    let command = arguments.first().filter(|value| !value.starts_with("--"));
    if let Some(command) = command {
        exact_arguments(arguments, 1, "capabilities COMMAND")?;
        let entry = commands
            .iter()
            .find(|entry| entry.name == command.as_str())
            .ok_or_else(|| usage_error(format!("unknown public command '{command}'")))?;
        return serialized("capabilities.command", entry);
    }
    ensure_options(arguments, &["--known-schema"], &[])?;
    let schema_digest = command_schema_digest();
    if option_value(arguments, "--known-schema")?.as_deref() == Some(schema_digest.as_str()) {
        return success(
            "capabilities",
            json!({"schema_digest": schema_digest, "unchanged": true}),
        );
    }
    success(
        "capabilities",
        json!({
            "usage": "lkjscript [--project PATH] COMMAND [options]",
            "graph_contract": GRAPH_CONTRACT_IDENTITY,
            "cli_contract_version": CLI_CONTRACT_VERSION,
            "schema_digest": schema_digest,
            "commands": commands.iter().map(|entry| entry.name).collect::<Vec<_>>(),
            "project_templates": ["minimal", "command"],
            "change_forms": change_form_names(),
            "type_forms": type_form_names(),
            "expression_forms": expression_form_names(),
            "declaration_reference_forms": declaration_reference_form_names(),
            "declaration_reference_syntax": declaration_reference_syntax(),
            "owner_kinds": owner_kind_names(),
            "relation_roles": relation_role_names(),
            "limits": limit_registry(),
            "details": "lkjscript capabilities COMMAND",
        }),
    )
}

#[derive(Serialize)]
struct CommandHelp {
    name: &'static str,
    purpose: &'static str,
    usage: &'static str,
    project: &'static str,
    authority_effect: &'static str,
}

fn command_registry() -> Vec<CommandHelp> {
    vec![
        CommandHelp {
            name: "capabilities",
            purpose: "Discover the exact offline command and language contracts.",
            usage: "capabilities [COMMAND] [--known-schema DIGEST]",
            project: "none",
            authority_effect: "none",
        },
        CommandHelp {
            name: "new",
            purpose: "Create fresh canonical authority in an empty destination.",
            usage: "new DEST [--template minimal|command] [--name NAME]",
            project: "destination",
            authority_effect: "accepted",
        },
        CommandHelp {
            name: "inspect",
            purpose: "Inspect project, owner, target, revision, artifact, or deployment state.",
            usage: "inspect status|project|owner|targets|revision|artifact|deployment ...",
            project: "required_by_action",
            authority_effect: "none",
        },
        CommandHelp {
            name: "query",
            purpose: "Select bounded owners, relations, context, and impact.",
            usage: "query owners|find|relations|callers|callees|types|capabilities|context|impact|request ...",
            project: "required",
            authority_effect: "none",
        },
        CommandHelp {
            name: "change",
            purpose: "Normalize and validate or atomically commit one semantic change.",
            usage: "change (--request JSON | --request-file PATH) [--dry-run|--commit]",
            project: "required",
            authority_effect: "accepted_on_commit",
        },
        CommandHelp {
            name: "draft",
            purpose: "Create, inspect, append, rebase, publish, or drop non-executable work.",
            usage: "draft create|status|append|rebase|publish|drop ...",
            project: "required",
            authority_effect: "draft_or_accepted",
        },
        CommandHelp {
            name: "history",
            purpose: "List, inspect, diff, or merge exact revisions.",
            usage: "history list|show|diff|merge ...",
            project: "required",
            authority_effect: "accepted_only_for_merge_commit",
        },
        CommandHelp {
            name: "package",
            purpose: "Stage an exact dependency or inspect/export a built-in package.",
            usage: "package stage PATH | package builtin inspect|export ...",
            project: "required_for_stage",
            authority_effect: "operational_or_external_output",
        },
        CommandHelp {
            name: "check",
            purpose: "Run graph-owned tests through production and independent execution.",
            usage: "check",
            project: "required",
            authority_effect: "none",
        },
        CommandHelp {
            name: "build",
            purpose: "Build a deterministic graph-native artifact.",
            usage: "build [--output PATH]",
            project: "required",
            authority_effect: "external_output",
        },
        CommandHelp {
            name: "run",
            purpose: "Run a pure command, batch, or test target through both execution tiers.",
            usage: "run TARGET [--arguments JSON]",
            project: "required",
            authority_effect: "none",
        },
        CommandHelp {
            name: "serve",
            purpose: "Run one plaintext HTTP deployment with bounded shutdown.",
            usage: "serve --deployment DESCRIPTOR",
            project: "descriptor_bound",
            authority_effect: "external_runtime",
        },
        CommandHelp {
            name: "worker",
            purpose: "Run one bounded worker deployment.",
            usage: "worker --deployment DESCRIPTOR",
            project: "descriptor_bound",
            authority_effect: "external_runtime",
        },
        CommandHelp {
            name: "review",
            purpose: "Write a deterministic non-authoritative review projection.",
            usage: "review [--revision REV] [--output PATH]",
            project: "required",
            authority_effect: "external_output",
        },
        CommandHelp {
            name: "backup",
            purpose: "Capture one exact reachable canonical authority bundle.",
            usage: "backup [--output PATH]",
            project: "required",
            authority_effect: "external_output",
        },
        CommandHelp {
            name: "restore",
            purpose: "Verify and atomically restore a canonical authority bundle.",
            usage: "restore --backup PATH (--output PROJECT | --project PROJECT)",
            project: "destination",
            authority_effect: "accepted",
        },
        CommandHelp {
            name: "doctor",
            purpose: "Validate authority or preview exact retained/reclaimable storage.",
            usage: "doctor [--deep] | doctor cleanup",
            project: "required",
            authority_effect: "operational_indexes_only",
        },
    ]
}

fn command_schema_digest() -> String {
    let bytes = serde_json::to_vec(&json!({
        "commands": command_registry(),
        "project_templates": ["minimal", "command"],
        "change_forms": change_form_names(),
        "type_forms": type_form_names(),
        "expression_forms": expression_form_names(),
        "declaration_reference_forms": declaration_reference_form_names(),
        "declaration_reference_syntax": declaration_reference_syntax(),
        "owner_kinds": owner_kind_names(),
        "relation_roles": relation_role_names(),
        "limits": limit_registry(),
    }))
    .unwrap_or_default();
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.cli-registry.v4");
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(&bytes);
    hasher.finalize().to_hex().to_string()
}

fn limit_registry() -> serde_json::Value {
    json!({
        "cli_response_bytes": {
            "value": MAXIMUM_CLI_RESPONSE_BYTES,
            "class": "hostile_output_boundary",
        },
        "change_request_bytes": {
            "value": MAXIMUM_TRANSACTION_REQUEST_BYTES,
            "class": "hostile_input_boundary",
        },
        "backup_manifest_bytes": {
            "value": MAXIMUM_BACKUP_MANIFEST_BYTES,
            "class": "segmented_decoder_boundary",
        },
        "backup_segment_bytes": {
            "value": MAXIMUM_BACKUP_SEGMENT_BYTES,
            "class": "segmented_decoder_boundary",
        },
        "backup_segment_entries": {
            "value": BACKUP_SEGMENT_ENTRY_LIMIT,
            "class": "segmented_encoding_boundary",
        },
    })
}

fn owner_kind_names() -> Vec<&'static str> {
    vec![
        "repository",
        "package",
        "module",
        "record",
        "variant",
        "interface",
        "external",
        "pure_function",
        "task_function",
        "constant",
        "component",
        "test",
        "field",
        "case",
        "operation",
        "type_parameter",
        "parameter",
        "binding",
        "expression",
        "requirement",
        "port",
        "target",
        "documentation",
        "annotation",
    ]
}

fn change_form_names() -> Vec<&'static str> {
    vec![
        "add_dependency",
        "replace_dependency",
        "remove_dependency",
        "create_module",
        "create_record",
        "create_variant",
        "create_function",
        "create_component",
        "create_test",
        "create_target",
        "rename_module",
        "rename_declaration",
        "replace_body",
    ]
}

fn type_form_names() -> Vec<&'static str> {
    vec![
        "unit",
        "bool",
        "i64",
        "bytes",
        "text",
        "static_text",
        "secret",
        "parameter",
        "named",
        "record",
        "list",
        "map",
        "option",
        "result",
        "stream",
        "function",
    ]
}

fn expression_form_names() -> Vec<&'static str> {
    vec![
        "unit",
        "bool",
        "i64",
        "text",
        "static_text",
        "variable",
        "constant",
        "call",
        "function",
        "invoke",
        "record",
        "list",
    ]
}

fn declaration_reference_form_names() -> Vec<&'static str> {
    vec![
        "request_local_symbol",
        "local_declaration_id",
        "exact_package_module_declaration",
    ]
}

fn declaration_reference_syntax() -> serde_json::Value {
    json!({
        "request_local_symbol": "$NAME",
        "local_declaration_id": "decl_HEX",
        "exact_package_module_declaration": "exact:PACKAGE_HEX/mod_HEX/decl_HEX",
    })
}

fn relation_role_names() -> Vec<&'static str> {
    vec![
        "import",
        "export",
        "type_use",
        "value_reference",
        "call",
        "field_use",
        "variant_construction",
        "variant_pattern",
        "capability_interface",
        "capability_operation",
        "component_port_function",
        "target_component",
        "target_port",
        "test_dependency",
    ]
}

fn dependency_stage_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error(
            "package stage requires one graph artifact path",
        ));
    }
    let bytes = read_bounded(
        Path::new(&arguments[0]),
        MAXIMUM_ARTIFACT_BYTES + 50,
        "graph dependency artifact",
    )?;
    let artifact = load_artifact(&bytes)?;
    let workspace = open_workspace(project)?;
    let mut staged = 0usize;
    for digest in artifact.package_artifacts.values() {
        let package = artifact.package_object(*digest).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Corrupt,
                "semantic_dependency_object_missing",
                "validated artifact closure lost one exact package object",
            )
        })?;
        workspace
            .repository()
            .write_artifact_object(*digest, package)?;
        staged = staged.checked_add(1).ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "semantic_dependency_count",
                "dependency package object count overflowed",
            )
        })?;
    }
    success(
        "package.stage",
        json!({
            "status": "staged",
            "package_id": artifact.root_package_id,
            "semantic_revision": artifact.root_revision,
            "artifact": artifact.root_package_artifact,
            "objects": staged,
            "current_revision": workspace.status()?.revision,
        }),
    )
}

fn owners_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &query_value_options(["--kind", "--module"]), &[])?;
    let kind = option_value(arguments, "--kind")?
        .map(|value| OwnerKind::parse(&value))
        .transpose()?;
    let module = option_value(arguments, "--module")?
        .map(|value| value.parse::<ModuleId>())
        .transpose()?;
    let workspace = open_workspace(project)?;
    let index = query_index(&workspace, arguments)?;
    let page = index.owners(
        kind,
        module,
        option_value(arguments, "--continue")?.as_deref(),
        query_budget(arguments)?,
    )?;
    serialized("query.owners", &page)
}

fn find_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let text = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("query find requires one search text"))?;
    ensure_options(&arguments[1..], &query_value_options([]), &["--exact"])?;
    let workspace = open_workspace(project)?;
    let exact = flag_present(&arguments[1..], "--exact")?;
    let continuation = option_value(&arguments[1..], "--continue")?;
    let budget = query_budget(&arguments[1..])?;
    let page = if exact {
        let revision = selected_revision(&workspace, &arguments[1..])?;
        SemanticQueryIndex::exact_find_revision(
            workspace.repository(),
            revision,
            text,
            continuation.as_deref(),
            budget,
        )?
    } else {
        query_index(&workspace, &arguments[1..])?.find(
            text,
            false,
            continuation.as_deref(),
            budget,
        )?
    };
    serialized("query.find", &page)
}

fn show_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("inspect owner requires one exact owner ID"))?;
    ensure_options(&arguments[1..], &["--revision"], &["--body"])?;
    let workspace = open_workspace(project)?;
    serialized(
        "inspect.owner",
        &SemanticQueryIndex::show_revision(
            workspace.repository(),
            selected_revision(&workspace, &arguments[1..])?,
            id,
            flag_present(&arguments[1..], "--body")?,
        )?,
    )
}

fn relation_command(
    command: &str,
    arguments: &[String],
    project: Option<&Path>,
    incoming: bool,
    outgoing: bool,
    roles: BTreeSet<RelationRole>,
) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error(format!("{command} requires one exact owner ID")))?;
    ensure_options(&arguments[1..], &query_value_options([]), &[])?;
    let workspace = open_workspace(project)?;
    let index = query_index(&workspace, &arguments[1..])?;
    let page = index.relations(
        id,
        incoming,
        outgoing,
        &roles,
        option_value(&arguments[1..], "--continue")?.as_deref(),
        query_budget(&arguments[1..])?,
    )?;
    serialized(command, &page)
}

fn context_command(
    arguments: &[String],
    project: Option<&Path>,
    impact: bool,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &query_value_options(["--seed"]), &[])?;
    let seeds = option_values(arguments, "--seed")?;
    let workspace = open_workspace(project)?;
    let index = query_index(&workspace, arguments)?;
    let continuation = option_value(arguments, "--continue")?;
    let page = if impact {
        index.impact(&seeds, continuation.as_deref(), query_budget(arguments)?)?
    } else {
        index.context(&seeds, continuation.as_deref(), query_budget(arguments)?)?
    };
    serialized(
        if impact {
            "query.impact"
        } else {
            "query.context"
        },
        &page,
    )
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ClosedQuery {
    contract_version: u16,
    #[serde(default)]
    revision: Option<RevisionId>,
    #[serde(default)]
    continuation: Option<String>,
    #[serde(default)]
    budget: QueryBudget,
    #[serde(flatten)]
    selection: ClosedSelection,
}

#[derive(Deserialize)]
#[serde(tag = "select", rename_all = "snake_case")]
enum ClosedSelection {
    Owners {
        #[serde(default)]
        kind: Option<OwnerKind>,
        #[serde(default)]
        module: Option<ModuleId>,
    },
    Find {
        text: String,
        #[serde(default)]
        exact: bool,
    },
    Relations {
        id: String,
        incoming: bool,
        outgoing: bool,
        #[serde(default)]
        roles: BTreeSet<RelationRole>,
    },
    Context {
        seeds: Vec<String>,
    },
    Impact {
        seeds: Vec<String>,
    },
}

fn query_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let request = option_value(arguments, "--request")?
        .ok_or_else(|| usage_error("query request requires --request JSON"))?;
    ensure_options(arguments, &["--request"], &[])?;
    let mut deserializer = serde_json::Deserializer::from_str(&request);
    let request = ClosedQuery::deserialize(&mut deserializer).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_query_request",
            format!("query request is not strict current JSON: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_query_trailing",
            format!("query request has trailing input: {error}"),
        )
    })?;
    if request.contract_version != super::semantic_query::QUERY_CONTRACT_VERSION {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_query_contract",
            "query request uses an unknown contract",
        ));
    }
    let workspace = open_workspace(project)?;
    let index = match request.revision {
        Some(revision) => SemanticQueryIndex::revision(workspace.repository(), revision)?,
        None => SemanticQueryIndex::current(workspace.repository())?,
    };
    let result = match request.selection {
        ClosedSelection::Owners { kind, module } => serde_json::to_value(index.owners(
            kind,
            module,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Find { text, exact } => serde_json::to_value(index.find(
            &text,
            exact,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Relations {
            id,
            incoming,
            outgoing,
            roles,
        } => serde_json::to_value(index.relations(
            &id,
            incoming,
            outgoing,
            &roles,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Context { seeds } => serde_json::to_value(index.context(
            &seeds,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
        ClosedSelection::Impact { seeds } => serde_json::to_value(index.impact(
            &seeds,
            request.continuation.as_deref(),
            request.budget,
        )?)
        .map_err(internal_json)?,
    };
    success("query.request", result)
}

fn transaction_command(
    arguments: &[String],
    project: Option<&Path>,
    mode: TransactionMode,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--request", "--request-file"], &[])?;
    let inline = option_value(arguments, "--request")?;
    let file = option_value(arguments, "--request-file")?;
    let bytes = match (inline, file) {
        (Some(value), None) => {
            if value.len() > MAXIMUM_TRANSACTION_REQUEST_BYTES {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "semantic_transaction_request_limit",
                    format!(
                        "transaction request exceeds {MAXIMUM_TRANSACTION_REQUEST_BYTES} bytes"
                    ),
                ));
            }
            value.into_bytes()
        }
        (None, Some(path)) => read_bounded(
            Path::new(&path),
            MAXIMUM_TRANSACTION_REQUEST_BYTES,
            "semantic transaction request",
        )?,
        (Some(_), Some(_)) => {
            return Err(usage_error(
                "supply exactly one of --request or --request-file",
            ));
        }
        (None, None) => {
            return Err(usage_error(
                "semantic transaction requires --request JSON or --request-file PATH",
            ));
        }
    };
    let mut deserializer = serde_json::Deserializer::from_slice(&bytes);
    let request = TransactionRequest::deserialize(&mut deserializer).map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_transaction_request",
            format!("transaction request is not strict current JSON: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_transaction_trailing",
            format!("transaction request has trailing input: {error}"),
        )
    })?;
    let workspace = open_workspace(project)?;
    if request.draft.is_some() {
        let drafts = SemanticDraftStore::new(workspace.repository());
        return match mode {
            TransactionMode::Apply => serialized("draft.append", &drafts.append(&request)?),
            TransactionMode::Plan | TransactionMode::Validate => transaction_result(
                if mode == TransactionMode::Plan {
                    "draft.plan"
                } else {
                    "draft.validate"
                },
                &drafts.evaluate(&request, mode)?,
            ),
        };
    }
    Err(usage_error(
        "draft append requires a transaction request bound to one draft",
    ))
}

fn draft_create_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--base", "--intent"], &[])?;
    let base = option_value(arguments, "--base")?
        .map(|value| value.parse::<RevisionId>())
        .transpose()?;
    let intent = option_value(arguments, "--intent")?;
    let workspace = open_workspace(project)?;
    serialized(
        "draft.create",
        &SemanticDraftStore::new(workspace.repository()).create(base, intent)?,
    )
}

fn draft_status_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error("draft status requires one draft ID"));
    }
    let id = arguments[0].parse::<DraftId>()?;
    let workspace = open_workspace(project)?;
    serialized(
        "draft.status",
        &SemanticDraftStore::new(workspace.repository()).status(id)?,
    )
}

fn draft_drop_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error("draft drop requires one draft ID"));
    }
    let id = arguments[0].parse::<DraftId>()?;
    let workspace = open_workspace(project)?;
    SemanticDraftStore::new(workspace.repository()).drop(id)?;
    success(
        "draft.drop",
        json!({
            "status": "draft_dropped",
            "draft": id,
            "revision": workspace.repository().current_binding()?.head.revision,
        }),
    )
}

fn draft_rebase_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("draft rebase requires one draft ID"))?
        .parse::<DraftId>()?;
    ensure_options(&arguments[1..], &["--base"], &[])?;
    let base = option_value(&arguments[1..], "--base")?
        .ok_or_else(|| usage_error("draft rebase requires --base REV"))?
        .parse::<RevisionId>()?;
    let workspace = open_workspace(project)?;
    serialized(
        "draft.rebase",
        &SemanticDraftStore::new(workspace.repository()).rebase(id, base)?,
    )
}

fn draft_publish_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let id = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("draft publish requires one draft ID"))?
        .parse::<DraftId>()?;
    ensure_options(&arguments[1..], &["--idempotency-key"], &[])?;
    let idempotency_key = option_value(&arguments[1..], "--idempotency-key")?;
    let workspace = open_workspace(project)?;
    transaction_result(
        "draft.publish",
        &SemanticDraftStore::new(workspace.repository()).publish(id, idempotency_key)?,
    )
}

fn targets_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    let limit = optional_usize(arguments, "--limit", DEFAULT_ORIENTATION_ITEMS)?;
    ensure_options(arguments, &["--limit"], &[])?;
    let workspace = open_workspace(project)?;
    let orientation = workspace.orient(limit)?;
    success(
        "inspect.targets",
        json!({
            "revision": orientation.revision,
            "returned_items": orientation.targets.len(),
            "total_items": orientation.target_count,
            "truncated": orientation.target_count > orientation.targets.len(),
            "items": orientation.targets,
        }),
    )
}

fn build_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--output"], &[])?;
    let workspace = open_workspace(project)?;
    let output = option_value(arguments, "--output")?
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().join("target/application.lkja"));
    let (bytes, receipt) = workspace.build_artifact()?;
    let publication = write_derived_output(
        &output,
        &bytes,
        MAXIMUM_ARTIFACT_BYTES + 50,
        "graph artifact",
    )?;
    success(
        "build",
        json!({
            "receipt": receipt,
            "output": output.display().to_string(),
            "publication": publication,
        }),
    )
}

fn run_package_tests(program: &PreparedProgram) -> Result<CliSuccess, Diagnostic> {
    let mut production_instructions = 0u64;
    let mut oracle_instructions = 0u64;
    let mut production_elapsed_nanoseconds = 0u64;
    let mut oracle_elapsed_nanoseconds = 0u64;
    for test in program.tests() {
        let production_started = Instant::now();
        let (actual, actual_observation) = Vm::new(program, RunPolicy::default())
            .invoke_test_expression(&test.actual, Vec::new())
            .map_err(execution_diagnostic)?;
        let (expected, expected_observation) = Vm::new(program, RunPolicy::default())
            .invoke_test_expression(&test.expected, Vec::new())
            .map_err(execution_diagnostic)?;
        production_elapsed_nanoseconds = production_elapsed_nanoseconds.saturating_add(
            u64::try_from(production_started.elapsed().as_nanos()).unwrap_or(u64::MAX),
        );
        let oracle_started = Instant::now();
        let (oracle_actual, oracle_actual_observation) =
            ReferenceInterpreter::new(program, RunPolicy::default())
                .invoke_test_expression(&test.actual, Vec::new())
                .map_err(execution_diagnostic)?;
        let (oracle_expected, oracle_expected_observation) =
            ReferenceInterpreter::new(program, RunPolicy::default())
                .invoke_test_expression(&test.expected, Vec::new())
                .map_err(execution_diagnostic)?;
        oracle_elapsed_nanoseconds = oracle_elapsed_nanoseconds
            .saturating_add(u64::try_from(oracle_started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        production_instructions = production_instructions
            .saturating_add(actual_observation.instructions)
            .saturating_add(expected_observation.instructions);
        oracle_instructions = oracle_instructions
            .saturating_add(oracle_actual_observation.instructions)
            .saturating_add(oracle_expected_observation.instructions);
        let actual = actual.canonical_json();
        let expected = expected.canonical_json();
        let oracle_actual = oracle_actual.canonical_json();
        let oracle_expected = oracle_expected.canonical_json();
        if actual != oracle_actual || expected != oracle_expected {
            return Err(Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "semantic_test_differential",
                format!(
                    "production and oracle execution disagree for {}::{}::{}",
                    test.package.as_str(),
                    test.module,
                    test.name
                ),
            ));
        }
        if actual != expected || oracle_actual != oracle_expected {
            return Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "semantic_test_failed",
                format!(
                    "test {}::{}::{} did not equal its expected value",
                    test.package.as_str(),
                    test.module,
                    test.name
                ),
            ));
        }
    }
    success(
        "check",
        json!({
            "revision": program.artifact().root_revision,
            "passed": program.tests().len(),
            "failed": 0,
            "production_tier": "bytecode_v1",
            "oracle_tier": "semantic_reference_v1",
            "production_instructions": production_instructions,
            "oracle_instructions": oracle_instructions,
            "production_elapsed_nanoseconds": production_elapsed_nanoseconds,
            "oracle_elapsed_nanoseconds": oracle_elapsed_nanoseconds,
            "differential": "equal",
        }),
    )
}

fn artifact_inspect_command(arguments: &[String]) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error("inspect artifact requires one artifact path"));
    }
    let bytes = read_bounded(
        Path::new(&arguments[0]),
        MAXIMUM_ARTIFACT_BYTES + 50,
        "graph artifact",
    )?;
    let artifact = load_artifact(&bytes)?;
    let program = PreparedProgram::prepare(artifact)?;
    let packages = program
        .artifact()
        .packages
        .values()
        .map(|package| {
            json!({
                "package_id": package.descriptor.package_id,
                "name": package.descriptor.name,
                "semantic_revision": package.accepted_revision,
                "package_artifact": program.artifact().package_artifacts.get(&package.descriptor.package_id),
                "modules": package.modules.iter().map(|module| json!({
                    "id": module.module_id,
                    "name": module.module.name,
                })).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let targets = program
        .targets()
        .values()
        .map(|target| {
            let requirements = program
                .components()
                .get(&target.component)
                .map(|component| component.requirements.keys().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            json!({
                "name": target.name,
                "runner": target.runner,
                "component": target.component,
                "port": target.port.name,
                "parameters": target.port.signature.parameters,
                "result": target.port.signature.result,
                "requirements": requirements,
            })
        })
        .collect::<Vec<_>>();
    success(
        "inspect.artifact",
        json!({
            "artifact_digest": program.artifact().artifact_digest,
            "root_package_artifact": program.artifact().root_package_artifact,
            "root_package_id": program.artifact().root_package_id,
            "root_revision": program.artifact().root_revision,
            "packages": packages,
            "targets": targets,
        }),
    )
}

fn text_projection_command(
    command: &str,
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--revision", "--output"], &[])?;
    let workspace = open_workspace(project)?;
    let revision = option_value(arguments, "--revision")?
        .map(|value| value.parse())
        .transpose()?;
    let output = option_value(arguments, "--output")?
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().join("target/meaning-review.json"));
    let (bytes, receipt) = render_review_projection(workspace.repository(), revision)?;
    let publication = write_derived_output(
        &output,
        &bytes,
        MAXIMUM_REVIEW_PROJECTION_BYTES + 1,
        "semantic review projection",
    )?;
    success(
        command,
        json!({
            "receipt": receipt,
            "output": output.display().to_string(),
            "publication": publication,
            "importable": false,
            "recovery_command": "lkjscript backup --output target/meaning-backup.lkjb",
        }),
    )
}

fn run_target_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    let target_name = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("run requires one target name"))?;
    ensure_options(&arguments[1..], &["--arguments"], &[])?;
    let encoded_arguments =
        option_value(&arguments[1..], "--arguments")?.unwrap_or_else(|| "[]".to_owned());
    let workspace = open_workspace(project)?;
    let program = workspace.prepare()?;
    let target = program.target(target_name)?;
    if !matches!(
        target.runner,
        RunnerKind::Command | RunnerKind::Batch | RunnerKind::Test
    ) {
        return Err(usage_error(format!(
            "target '{}' uses {:?} runner; use its topology-specific command",
            target.name, target.runner
        )));
    }
    let component = program
        .components()
        .get(&target.component)
        .ok_or_else(|| internal_error("target component disappeared"))?;
    if !component.requirements.is_empty() {
        return Err(Diagnostic::new(
            DiagnosticClass::Capability,
            "target_grants_required",
            "effectful target requires an exact deployment descriptor",
        ));
    }
    let json_arguments = decode_strict(encoded_arguments.as_bytes(), JsonLimits::default())?;
    let items = json_arguments.as_array().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "target_arguments_array",
            "target arguments must be one JSON array",
        )
    })?;
    if items.len() != target.port.signature.parameters.len() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "target_argument_count",
            format!(
                "target expects {} arguments; {} were supplied",
                target.port.signature.parameters.len(),
                items.len()
            ),
        ));
    }
    let values = items
        .iter()
        .zip(&target.port.signature.parameters)
        .map(|(value, ty)| {
            let bytes = serde_json::to_vec(value).map_err(internal_json)?;
            decode_typed(
                &bytes,
                ty,
                &program.artifact().packages,
                JsonLimits::default(),
            )
        })
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let (value, production) = Vm::new(&program, RunPolicy::default())
        .invoke(&target.port.function, values.clone())
        .map_err(execution_diagnostic)?;
    let (reference, oracle) = ReferenceInterpreter::new(&program, RunPolicy::default())
        .invoke(&target.port.function, values)
        .map_err(execution_diagnostic)?;
    if value.canonical_json() != reference.canonical_json() {
        return Err(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "target_differential",
            "production and reference execution disagree",
        ));
    }
    let result_bytes = encode_typed(
        &value,
        &target.port.signature.result,
        &program.artifact().packages,
        JsonLimits::default(),
    )?;
    let result = decode_strict(&result_bytes, JsonLimits::default())?;
    success(
        "run",
        json!({
            "revision": program.artifact().root_revision,
            "target": target.name,
            "result": result,
            "production": production,
            "oracle": oracle,
            "differential": "equal",
        }),
    )
}

fn history_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--limit", "--before"], &[])?;
    let limit = optional_usize(arguments, "--limit", 50)?;
    let before = option_value(arguments, "--before")?
        .map(|value| value.parse::<RevisionId>())
        .transpose()?;
    let workspace = open_workspace(project)?;
    let records = workspace.repository().history(before, limit)?;
    success(
        "history.list",
        json!({
            "revision": workspace.status()?.revision,
            "returned_items": records.len(),
            "items": records,
        }),
    )
}

fn diff_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(
        arguments,
        &query_value_options(["--base", "--result", "--offset"]),
        &[],
    )?;
    let base = option_value(arguments, "--base")?
        .ok_or_else(|| usage_error("history diff requires --base REV"))?
        .parse::<RevisionId>()?;
    let result = option_value(arguments, "--result")?
        .ok_or_else(|| usage_error("history diff requires --result REV"))?
        .parse::<RevisionId>()?;
    let offset = optional_usize(arguments, "--offset", 0)?;
    let workspace = open_workspace(project)?;
    serialized(
        "history.diff",
        &diff_revisions(
            workspace.repository(),
            base,
            result,
            offset,
            query_budget(arguments)?,
        )?,
    )
}

fn merge_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(
        arguments,
        &["--base", "--left", "--right", "--work", "--intent"],
        &["--apply"],
    )?;
    let revision = |name: &str| -> Result<RevisionId, Diagnostic> {
        option_value(arguments, name)?
            .ok_or_else(|| usage_error(format!("history merge requires {name} REV")))?
            .parse()
    };
    let request = SemanticMergeRequest {
        contract_version: SEMANTIC_MERGE_CONTRACT_VERSION,
        base_revision: revision("--base")?,
        left_revision: revision("--left")?,
        right_revision: revision("--right")?,
        maximum_work: optional_usize(
            arguments,
            "--work",
            super::repository::MAXIMUM_HISTORY_ITEMS,
        )?,
        intent: option_value(arguments, "--intent")?,
    };
    let workspace = open_workspace(project)?;
    merge_result(
        "history.merge",
        &merge_revisions(
            workspace.repository(),
            &request,
            flag_present(arguments, "--apply")?,
        )?,
    )
}

fn revision_show_command(
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    if arguments.len() != 1 {
        return Err(usage_error("history show requires one exact revision ID"));
    }
    let revision = arguments[0].parse::<RevisionId>()?;
    let workspace = open_workspace(project)?;
    let snapshot = workspace.repository().reconstruct_revision(revision)?;
    success(
        "history.show",
        json!({
            "record": snapshot.record,
            "receipt": snapshot.receipt,
            "root": {
                "repository_id": snapshot.root.repository_id,
                "package_id": snapshot.root.package_id,
                "package_name": snapshot.root.package_name,
                "modules": snapshot.root.modules.len(),
                "dependencies": snapshot.root.dependencies.len(),
                "targets": snapshot.root.targets.len(),
                "tombstones": snapshot.root.tombstones.len(),
            },
        }),
    )
}

fn doctor_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    if arguments.first().map(String::as_str) == Some("cleanup") {
        exact_arguments(arguments, 1, "doctor cleanup")?;
        let workspace = open_workspace(project)?;
        return serialized(
            "doctor.cleanup",
            &workspace.repository().retention_preview()?,
        );
    }
    ensure_options(arguments, &[], &["--deep"])?;
    let workspace = open_workspace(project)?;
    serialized(
        "doctor",
        &workspace
            .repository()
            .doctor(flag_present(arguments, "--deep")?)?,
    )
}

fn backup_command(
    command: &str,
    arguments: &[String],
    project: Option<&Path>,
) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--output"], &[])?;
    let workspace = open_workspace(project)?;
    let output = option_value(arguments, "--output")?
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace.root().join("target/meaning-backup.lkjb"));
    let receipt = workspace.repository().backup_to(&output)?;
    success(
        command,
        json!({
            "receipt": receipt,
            "output": output.display().to_string(),
            "publication": "published",
        }),
    )
}

fn restore_command(arguments: &[String], project: Option<&Path>) -> Result<CliSuccess, Diagnostic> {
    ensure_options(arguments, &["--backup", "--output"], &[])?;
    let backup = option_value(arguments, "--backup")?
        .ok_or_else(|| usage_error("restore requires --backup PATH"))?;
    let explicit_output = option_value(arguments, "--output")?.map(PathBuf::from);
    if explicit_output.is_some() && project.is_some() {
        return Err(usage_error(
            "restore accepts either --output or global --project, not both",
        ));
    }
    let output = explicit_output
        .or_else(|| project.map(Path::to_path_buf))
        .ok_or_else(|| usage_error("restore requires --output PROJECT or --project PROJECT"))?;
    let output = fs::canonicalize(&output)
        .map_err(|error| io_error("semantic_restore_output", &output, error))?;
    let (repository, receipt) =
        SemanticRepository::restore_backup_from(&output, Path::new(&backup))?;
    success(
        "restore",
        json!({
            "receipt": receipt,
            "status": SemanticWorkspace::open(repository.project_root())?.status()?,
        }),
    )
}

fn query_index(
    workspace: &SemanticWorkspace,
    arguments: &[String],
) -> Result<SemanticQueryIndex, Diagnostic> {
    match option_value(arguments, "--revision")? {
        Some(value) => SemanticQueryIndex::revision(workspace.repository(), value.parse()?),
        None => SemanticQueryIndex::current(workspace.repository()),
    }
}

fn selected_revision(
    workspace: &SemanticWorkspace,
    arguments: &[String],
) -> Result<RevisionId, Diagnostic> {
    match option_value(arguments, "--revision")? {
        Some(value) => value.parse(),
        None => Ok(workspace.status()?.revision),
    }
}

fn query_budget(arguments: &[String]) -> Result<QueryBudget, Diagnostic> {
    QueryBudget {
        maximum_items: optional_usize(arguments, "--limit", QueryBudget::default().maximum_items)?,
        maximum_bytes: optional_usize(arguments, "--bytes", QueryBudget::default().maximum_bytes)?,
        maximum_work: optional_usize(arguments, "--work", QueryBudget::default().maximum_work)?,
        maximum_depth: optional_usize(arguments, "--depth", QueryBudget::default().maximum_depth)?,
        maximum_fanout: optional_usize(
            arguments,
            "--fanout",
            QueryBudget::default().maximum_fanout,
        )?,
    }
    .validate()
}

fn query_value_options<const N: usize>(extra: [&str; N]) -> Vec<&str> {
    let mut values = vec![
        "--revision",
        "--limit",
        "--bytes",
        "--work",
        "--depth",
        "--fanout",
        "--continue",
    ];
    values.extend(extra);
    values
}

fn extract_global_project(
    arguments: Vec<String>,
) -> Result<(Vec<String>, Option<PathBuf>), Diagnostic> {
    let mut output = Vec::new();
    let mut project = None;
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == "--project" {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error("--project requires a path"))?;
            if project.replace(PathBuf::from(value)).is_some() {
                return Err(usage_error("--project may be supplied only once"));
            }
            index += 2;
        } else {
            output.push(arguments[index].clone());
            index += 1;
        }
    }
    Ok((output, project))
}

fn option_value(arguments: &[String], name: &str) -> Result<Option<String>, Diagnostic> {
    let values = option_values(arguments, name)?;
    if values.len() > 1 {
        return Err(usage_error(format!("{name} may be supplied only once")));
    }
    Ok(values.into_iter().next())
}

fn option_values(arguments: &[String], name: &str) -> Result<Vec<String>, Diagnostic> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < arguments.len() {
        if arguments[index] == name {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error(format!("{name} requires a value")))?;
            if value.starts_with("--") {
                return Err(usage_error(format!("{name} requires a value")));
            }
            values.push(value.clone());
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(values)
}

fn optional_usize(arguments: &[String], name: &str, default: usize) -> Result<usize, Diagnostic> {
    option_value(arguments, name)?
        .map(|value| parse_usize(&value, name))
        .transpose()
        .map(|value| value.unwrap_or(default))
}

fn flag_present(arguments: &[String], name: &str) -> Result<bool, Diagnostic> {
    let count = arguments
        .iter()
        .filter(|value| value.as_str() == name)
        .count();
    if count > 1 {
        return Err(usage_error(format!("{name} may be supplied only once")));
    }
    Ok(count == 1)
}

fn ensure_options(arguments: &[String], valued: &[&str], flags: &[&str]) -> Result<(), Diagnostic> {
    let mut index = 0;
    while index < arguments.len() {
        let option = arguments[index].as_str();
        if valued.contains(&option) {
            let value = arguments
                .get(index + 1)
                .ok_or_else(|| usage_error(format!("{option} requires a value")))?;
            if value.starts_with("--") {
                return Err(usage_error(format!("{option} requires a value")));
            }
            index += 2;
        } else if flags.contains(&option) {
            index += 1;
        } else {
            return Err(usage_error(format!(
                "unknown option or argument '{option}'"
            )));
        }
    }
    Ok(())
}

fn exact_arguments(arguments: &[String], expected: usize, command: &str) -> Result<(), Diagnostic> {
    if arguments.len() != expected {
        return Err(usage_error(format!(
            "{command} received an unexpected argument"
        )));
    }
    Ok(())
}

fn open_workspace(project: Option<&Path>) -> Result<SemanticWorkspace, Diagnostic> {
    SemanticWorkspace::open(&project_or_current(project)?)
}

fn project_or_current(project: Option<&Path>) -> Result<PathBuf, Diagnostic> {
    let path = match project {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir().map_err(|error| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "semantic_current_directory",
                format!("current directory is unavailable: {error}"),
            )
        })?,
    };
    fs::canonicalize(&path).map_err(|error| io_error("semantic_project_path", &path, error))
}

fn write_derived_output(
    path: &Path,
    bytes: &[u8],
    maximum_existing_bytes: usize,
    label: &str,
) -> Result<&'static str, Diagnostic> {
    let parent = path.parent().ok_or_else(|| {
        Diagnostic::new(
            DiagnosticClass::Source,
            "semantic_output_parent",
            "output path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error("semantic_output_directory", parent, error))?;
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(Diagnostic::new(
                DiagnosticClass::Source,
                "semantic_output_type",
                format!("output '{}' is not a regular file", path.display()),
            ));
        }
        if read_bounded(path, maximum_existing_bytes, label)? == bytes {
            return Ok("unchanged");
        }
    }
    let file_name = path
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| usage_error("output file name is not portable UTF-8"))?;
    let temporary = parent.join(format!(".{file_name}.stage-{}", RepositoryId::generate()?));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)
            .map_err(|error| io_error("semantic_output_create", &temporary, error))?;
        file.write_all(bytes)
            .map_err(|error| io_error("semantic_output_write", &temporary, error))?;
        file.sync_all()
            .map_err(|error| io_error("semantic_output_sync", &temporary, error))?;
        fs::rename(&temporary, path)
            .map_err(|error| io_error("semantic_output_publish", path, error))?;
        File::open(parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| io_error("semantic_output_parent_sync", parent, error))?;
        Ok("published")
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn read_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| io_error("read_open", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "read_type",
            format!("{label} '{}' is not a regular file", path.display()),
        ));
    }
    let mut file = File::open(path).map_err(|error| io_error("read_open", path, error))?;
    let mut bytes = Vec::new();
    Read::by_ref(&mut file)
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("read_bytes", path, error))?;
    if bytes.len() > maximum {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "read_limit",
            format!("{label} exceeds {maximum} bytes"),
        ));
    }
    Ok(bytes)
}

fn parse_usize(value: &str, label: &str) -> Result<usize, Diagnostic> {
    if value.is_empty()
        || (value.len() > 1 && value.starts_with('0'))
        || !value.bytes().all(|byte| byte.is_ascii_digit())
    {
        return Err(usage_error(format!(
            "{label} must be a canonical unsigned integer"
        )));
    }
    value
        .parse::<usize>()
        .map_err(|_| usage_error(format!("{label} is outside the supported range")))
}

fn serialized(command: &str, value: &impl Serialize) -> Result<CliSuccess, Diagnostic> {
    success(command, serde_json::to_value(value).map_err(internal_json)?)
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TransactionCliProjection<'a> {
    contract_version: u16,
    graph_contract: &'static str,
    repository_id: RepositoryId,
    requested_base: RevisionId,
    observed_current: RevisionId,
    status: TransactionStatus,
    transaction: TransactionDigest,
    #[serde(skip_serializing_if = "Option::is_none")]
    semantic_diff: Option<SemanticDiffDigest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    predicted_revision: Option<RevisionId>,
    #[serde(skip_serializing_if = "Option::is_none")]
    published_revision: Option<RevisionId>,
    affected_owner_count: usize,
    affected_owners: &'a [AffectedOwner],
    affected_owners_truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    receipt: Option<TransactionReceiptProjection<'a>>,
    diagnostics: &'a [Diagnostic],
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct TransactionReceiptProjection<'a> {
    digest: ReceiptDigest,
    result: RevisionId,
    semantic_diff: SemanticDiffDigest,
    validation: &'a ValidationFacts,
    expansion: String,
}

fn transaction_result(command: &str, value: &TransactionResult) -> Result<CliSuccess, Diagnostic> {
    let (status, ok) = match value.status {
        TransactionStatus::Planned => ("planned", true),
        TransactionStatus::Validated => ("validated", true),
        TransactionStatus::AcceptedChange => ("accepted_change", true),
        TransactionStatus::Replayed => ("replayed", true),
        TransactionStatus::SemanticNoChange => ("semantic_no_change", true),
        TransactionStatus::StaleBase => ("stale_base", false),
        TransactionStatus::PreconditionFailed => ("precondition_failed", false),
        TransactionStatus::ForeignIdentity => ("foreign_identity", false),
        TransactionStatus::InvalidGraph => ("invalid_graph", false),
        TransactionStatus::ResourceExhausted => ("resource_exhausted", false),
    };
    let affected_limit = value
        .affected_owners
        .len()
        .min(MAXIMUM_INLINE_AFFECTED_OWNERS);
    let receipt = value
        .receipt
        .as_ref()
        .map(transaction_receipt_projection)
        .transpose()?;
    let projection = TransactionCliProjection {
        contract_version: value.contract_version,
        graph_contract: value.graph_contract,
        repository_id: value.repository_id,
        requested_base: value.requested_base,
        observed_current: value.observed_current,
        status: value.status,
        transaction: value.transaction,
        semantic_diff: value.semantic_diff,
        predicted_revision: value.predicted_revision,
        published_revision: value.published_revision,
        affected_owner_count: value.affected_owners.len(),
        affected_owners: &value.affected_owners[..affected_limit],
        affected_owners_truncated: affected_limit != value.affected_owners.len(),
        receipt,
        diagnostics: &value.diagnostics,
    };
    outcome(
        command,
        status,
        ok,
        serde_json::to_value(projection).map_err(internal_json)?,
    )
}

fn transaction_receipt_projection(
    receipt: &TransactionReceipt,
) -> Result<TransactionReceiptProjection<'_>, Diagnostic> {
    Ok(TransactionReceiptProjection {
        digest: receipt.digest()?,
        result: receipt.result,
        semantic_diff: receipt.semantic_diff,
        validation: &receipt.validation,
        expansion: format!("history show {}", receipt.result),
    })
}

fn merge_result(command: &str, value: &SemanticMergeResult) -> Result<CliSuccess, Diagnostic> {
    let (status, ok) = match value.status {
        SemanticMergeStatus::Ready => ("ready", true),
        SemanticMergeStatus::Conflicted => ("conflicted", false),
        SemanticMergeStatus::AcceptedChange => ("accepted_change", true),
        SemanticMergeStatus::SemanticNoChange => ("semantic_no_change", true),
        SemanticMergeStatus::StaleHead => ("stale_head", false),
    };
    outcome(
        command,
        status,
        ok,
        serde_json::to_value(value).map_err(internal_json)?,
    )
}

fn success(command: &str, result: serde_json::Value) -> Result<CliSuccess, Diagnostic> {
    outcome(command, "success", true, result)
}

fn outcome(
    command: &str,
    status: &'static str,
    ok: bool,
    result: serde_json::Value,
) -> Result<CliSuccess, Diagnostic> {
    let response = CliSuccess {
        contract_version: CLI_CONTRACT_VERSION,
        ok,
        status,
        command: command.to_owned(),
        result,
    };
    let bytes = serde_json::to_vec(&response).map_err(internal_json)?;
    if bytes.len().saturating_add(1) > MAXIMUM_CLI_RESPONSE_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "cli_output_budget",
            format!("command response exceeds the hard {MAXIMUM_CLI_RESPONSE_BYTES}-byte bound"),
        ));
    }
    Ok(response)
}

fn execution_diagnostic(error: super::execution::ExecutionError) -> Diagnostic {
    let class = match error.class {
        super::execution::ExecutionFailureClass::Trap => DiagnosticClass::Semantic,
        super::execution::ExecutionFailureClass::Capability
        | super::execution::ExecutionFailureClass::PossibleVisibility => {
            DiagnosticClass::Capability
        }
        super::execution::ExecutionFailureClass::Resource => DiagnosticClass::Resource,
        super::execution::ExecutionFailureClass::Cancelled => DiagnosticClass::Cancelled,
        super::execution::ExecutionFailureClass::Infrastructure => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
}

fn usage_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "cli_usage", message)
}

fn internal_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, "cli_internal", message)
}

fn internal_json(error: serde_json::Error) -> Diagnostic {
    internal_error(format!("machine JSON projection failed: {error}"))
}

fn io_error(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_options_and_noncanonical_numbers_reject() {
        assert!(execute(vec!["unknown".to_owned()]).is_err());
        assert!(parse_usize("01", "limit").is_err());
        assert!(ensure_options(&["--wat".to_owned()], &["--limit"], &[]).is_err());
    }

    #[test]
    fn command_registry_has_unique_names_and_stable_digest() {
        let commands = command_registry();
        let names = commands
            .iter()
            .map(|entry| entry.name)
            .collect::<BTreeSet<_>>();
        assert_eq!(commands.len(), names.len());
        assert_eq!(command_schema_digest().len(), 64);
    }
}
