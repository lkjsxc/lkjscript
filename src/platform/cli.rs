//! Strict, bounded public graph-native command projection.

use super::artifact::{MAXIMUM_ARTIFACT_BYTES, load_artifact};
use super::bootstrap::{builtin_package_info, export_builtin_standard};
use super::change::{AuthoredChange, AuthoredChangeSet, OwnerSelector};
use super::contract::{
    CLI_CONTRACT_VERSION, MAXIMUM_CLI_RESPONSE_BYTES, MAXIMUM_CLI_RESPONSE_RECORDS,
    MAXIMUM_TRANSACTION_REQUEST_BYTES, PublicOperation, RegistrySection, RegistrySnapshot,
    generated_documents, operation_descriptors, operation_record, outcome_exit_status,
    registry_snapshot,
};
use super::control::{
    ChangePlanDigest, CompactChangeOperation, CompactResponseLimits, CompactResponseWriter,
    MAXIMUM_COMPACT_INPUT_BYTES, NormalizedChangeRequest, compact_change_operation_descriptor,
    decode_compact_change, normalize_change_request,
};
use super::deployment::{MAXIMUM_DEPLOYMENT_BYTES, decode_deployment};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::{PreparedProgram, ReferenceInterpreter, RunPolicy, Vm};
use super::json::{JsonLimits, decode_strict, decode_typed, encode_typed};
use super::kernel::{Name, OwnerKey as KernelOwnerKey, OwnerKind as KernelOwnerKind, PackageId};
use super::meaning::RelationRole;
use super::package::RunnerKind;
use super::project_creation::create_minimal_project;
use super::project_discovery::discover_project;
use super::publication::{
    GraphRepository, PreparedAuthoredPublication, PublicationOptions,
    PublicationOutcome as GraphPublicationOutcome,
};
use super::repository::SemanticRepository;
use super::revision::{AffectedOwner, TransactionReceipt, ValidationFacts};
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
        outcome_exit_status(self.status)
    }
}

pub fn execute(arguments: Vec<String>) -> Result<CliSuccess, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.is_empty() {
        return Err(usage_error(
            "capabilities uses the compact executable process boundary",
        ));
    }
    let command = PublicOperation::parse(&arguments[0]).ok_or_else(|| {
        usage_error(format!(
            "unknown command '{}'; use 'capabilities'",
            arguments[0]
        ))
    })?;
    match command {
        PublicOperation::Capabilities => Err(usage_error(
            "capabilities uses the compact executable process boundary",
        )),
        PublicOperation::New => Err(usage_error(
            "new uses the compact executable process boundary",
        )),
        PublicOperation::Status => Err(usage_error(
            "status uses the compact executable process boundary",
        )),
        PublicOperation::Inspect => inspect_command(&arguments[1..], project.as_deref()),
        PublicOperation::Query => public_query_command(&arguments[1..], project.as_deref()),
        PublicOperation::Change => Err(usage_error(
            "change uses the compact executable process boundary",
        )),
        PublicOperation::Draft => public_draft_command(&arguments[1..], project.as_deref()),
        PublicOperation::History => public_history_command(&arguments[1..], project.as_deref()),
        PublicOperation::Package => package_command(&arguments[1..], project.as_deref()),
        PublicOperation::Check => {
            exact_arguments(&arguments, 1, "check")?;
            let workspace = open_workspace(project.as_deref())?;
            run_package_tests(&workspace.prepare()?)
        }
        PublicOperation::Build => build_command(&arguments[1..], project.as_deref()),
        PublicOperation::Run => run_target_command(&arguments[1..], project.as_deref()),
        PublicOperation::Serve | PublicOperation::Worker => Err(usage_error(format!(
            "'{}' is a resident runner command and must use the executable process boundary",
            command.name()
        ))),
        PublicOperation::Review => {
            text_projection_command("review", &arguments[1..], project.as_deref())
        }
        PublicOperation::Backup => backup_command("backup", &arguments[1..], project.as_deref()),
        PublicOperation::Restore => restore_command(&arguments[1..], project.as_deref()),
        PublicOperation::Doctor => doctor_command(&arguments[1..], project.as_deref()),
    }
}

pub fn execute_new(arguments: &[String]) -> Result<Vec<u8>, Diagnostic> {
    let destination = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("new requires one destination directory"))?;
    ensure_options(&arguments[1..], &["--template", "--name"], &[])?;
    let template =
        option_value(&arguments[1..], "--template")?.unwrap_or_else(|| "minimal".to_owned());
    if template == "command" {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "predecessor_contract",
            "the command template belongs to predecessor authority; normalized standard-package and command-target bootstrap are not available yet",
        ));
    }
    if template != "minimal" {
        return Err(usage_error(format!(
            "unknown normalized project template '{template}'; expected minimal"
        )));
    }
    let package_name = option_value(&arguments[1..], "--name")?.unwrap_or_else(|| {
        Path::new(destination)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("")
            .to_owned()
    });
    if package_name.is_empty() {
        return Err(usage_error(
            "new requires --name when the destination name is not valid UTF-8",
        ));
    }
    let created = create_minimal_project(Path::new(destination), &package_name)?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "new".to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "project",
        &[
            ("path", created.project.display().to_string()),
            ("template", "minimal".to_owned()),
            ("name", created.package_name.as_str().to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "repository",
        &[("id", created.repository.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "package",
        &[("id", created.package.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "revision",
        &[("id", created.revision.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "state",
        &[("digest", created.semantic_state.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "root",
        &[("digest", created.semantic_root.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "receipt",
        &[
            ("digest", created.receipt.to_string()),
            ("revision-record", created.revision_record.to_string()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "summary",
        &[
            ("owners", "0".to_owned()),
            ("dependencies", "0".to_owned()),
            ("retirements", "0".to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "next",
        &[
            ("kind", "discovery".to_owned()),
            ("command", "lkjscript capabilities inspect".to_owned()),
        ],
    )?;
    Ok(output.finish())
}

/// Reports the exact current normalized authority without consulting a predecessor reader.
pub fn execute_status(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.as_slice() != ["status"] {
        return Err(usage_error("status accepts no additional arguments"));
    }
    let repository = open_normalized_repository(project)?;
    let current = repository.current()?;
    let registry = registry_snapshot().map_err(contract_registry_error)?;

    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "status".to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "project",
        &[
            ("path", repository.root().display().to_string()),
            (
                "name",
                current.semantic_root.package_name.as_str().to_owned(),
            ),
        ],
    )?;
    append_compact_record(
        &mut output,
        "repository",
        &[("id", current.head.repository_id.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "package",
        &[("id", current.semantic_root.package_id.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "revision",
        &[
            ("id", current.head.revision.to_string()),
            ("record", current.head.record.to_string()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "state",
        &[("digest", current.accepted.semantic_state.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "root",
        &[("digest", current.accepted.semantic_root.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "evidence",
        &[
            ("witness", current.accepted.validation_witness.to_string()),
            (
                "certificate",
                current.accepted.validation_certificate.to_string(),
            ),
            ("validator", current.accepted.validator_contract.to_string()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "receipt",
        &[("digest", current.accepted.receipt.to_string())],
    )?;
    append_compact_record(
        &mut output,
        "summary",
        &[
            ("owners", current.semantic_root.owners.entries().to_string()),
            (
                "dependencies",
                current.semantic_root.dependencies.entries().to_string(),
            ),
            (
                "retirements",
                current.semantic_root.retirements.entries().to_string(),
            ),
        ],
    )?;
    append_compact_record(&mut output, "schema", &[("registry", registry.digest)])?;
    append_compact_record(
        &mut output,
        "next",
        &[
            ("kind", "discovery".to_owned()),
            ("command", "lkjscript capabilities status".to_owned()),
        ],
    )?;
    Ok(output.finish())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChangeAction {
    Plan,
    Apply,
}

struct ChangeCommandRequest {
    normalized: NormalizedChangeRequest,
    reviewed: Option<ChangePlanDigest>,
    input_file: Option<String>,
}

/// Plans or applies one transport-neutral authored request through the normalized repository
/// engine. Compact records and direct flags converge before plan comparison, repository access,
/// preparation, response generation, or publication.
pub fn execute_change(arguments: Vec<String>) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let (arguments, project) = extract_global_project(arguments).map_err(single_diagnostic)?;
    if arguments.first().map(String::as_str) != Some("change") {
        return Err(single_diagnostic(usage_error(
            "change dispatch requires the change command",
        )));
    }
    let action = arguments.get(1).map(String::as_str).ok_or_else(|| {
        single_diagnostic(usage_error(
            "change requires plan or apply; use 'capabilities change'",
        ))
    })?;
    let action = match action {
        "plan" => ChangeAction::Plan,
        "apply" => ChangeAction::Apply,
        other => {
            return Err(single_diagnostic(usage_error(format!(
                "unknown change action '{other}'; use plan or apply"
            ))));
        }
    };
    let adapter_arguments = &arguments[2..];
    let direct_operation = adapter_arguments
        .first()
        .and_then(|name| compact_change_operation_descriptor(name))
        .and_then(|descriptor| descriptor.direct.map(|_| descriptor.operation));
    let request = match direct_operation {
        Some(CompactChangeOperation::RenameOwner) => {
            decode_direct_rename(action, &adapter_arguments[1..]).map_err(single_diagnostic)?
        }
        Some(_) => {
            return Err(single_diagnostic(internal_error(
                "registered direct change operation has no typed CLI adapter",
            )));
        }
        None => decode_record_change(action, adapter_arguments)?,
    };
    require_reviewed_change_plan(action, request.reviewed, request.normalized.plan)
        .map_err(single_diagnostic)?;
    execute_normalized_change(project, action, request)
}

fn decode_record_change(
    action: ChangeAction,
    options: &[String],
) -> Result<ChangeCommandRequest, Vec<Diagnostic>> {
    let allowed = match action {
        ChangeAction::Plan => &["--input", "--input-file"][..],
        ChangeAction::Apply => &["--input", "--input-file", "--plan"][..],
    };
    ensure_options(options, allowed, &[]).map_err(single_diagnostic)?;
    let inline = option_value(options, "--input").map_err(single_diagnostic)?;
    let file = option_value(options, "--input-file").map_err(single_diagnostic)?;
    let (source, bytes) = match (inline, file) {
        (Some(value), None) if value.len() <= MAXIMUM_COMPACT_INPUT_BYTES => {
            ("<change-input>".to_owned(), value.into_bytes())
        }
        (Some(_), None) => {
            return Err(single_diagnostic(Diagnostic::new(
                DiagnosticClass::Resource,
                "control_input_bytes",
                format!(
                    "compact change input exceeds the {MAXIMUM_COMPACT_INPUT_BYTES}-byte format bound"
                ),
            )));
        }
        (None, Some(path)) => {
            let bytes = read_bounded(
                Path::new(&path),
                MAXIMUM_COMPACT_INPUT_BYTES,
                "compact change input",
            )
            .map_err(single_diagnostic)?;
            (path, bytes)
        }
        (Some(_), Some(_)) => {
            return Err(single_diagnostic(usage_error(
                "supply exactly one of --input or --input-file",
            )));
        }
        (None, None) => {
            return Err(single_diagnostic(usage_error(
                "change requires --input RECORDS or --input-file PATH",
            )));
        }
    };
    let normalized = decode_compact_change(&source, &bytes)?;
    let reviewed = option_value(options, "--plan")
        .map_err(single_diagnostic)?
        .map(|value| value.parse::<ChangePlanDigest>())
        .transpose()
        .map_err(single_diagnostic)?;
    Ok(ChangeCommandRequest {
        normalized,
        reviewed,
        input_file: Some(source),
    })
}

fn decode_direct_rename(
    action: ChangeAction,
    options: &[String],
) -> Result<ChangeCommandRequest, Diagnostic> {
    let allowed = match action {
        ChangeAction::Plan => &["--base", "--owner", "--name", "--idempotency", "--intent"][..],
        ChangeAction::Apply => &[
            "--base",
            "--owner",
            "--name",
            "--idempotency",
            "--intent",
            "--plan",
        ][..],
    };
    ensure_options(options, allowed, &[])?;
    let base = required_option(options, "--base")?
        .parse::<RevisionId>()
        .map_err(|diagnostic| direct_option_error("--base", diagnostic))?;
    let owner = required_option(options, "--owner")?
        .parse::<KernelOwnerKey>()
        .map_err(|diagnostic| direct_option_error("--owner", diagnostic))?;
    let name = Name::new(&required_option(options, "--name")?)
        .map_err(|diagnostic| direct_option_error("--name", diagnostic))?;
    let publication_options = PublicationOptions {
        idempotency_key: option_value(options, "--idempotency")?,
        intent: option_value(options, "--intent")?,
    };
    let semantic = AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::RenameOwner {
            owner: OwnerSelector::Exact { owner },
            name,
        }],
        budget: Default::default(),
    };
    let normalized = normalize_change_request(semantic, publication_options)?;
    let reviewed = option_value(options, "--plan")?
        .map(|value| value.parse::<ChangePlanDigest>())
        .transpose()?;
    Ok(ChangeCommandRequest {
        normalized,
        reviewed,
        input_file: None,
    })
}

fn require_reviewed_change_plan(
    action: ChangeAction,
    reviewed: Option<ChangePlanDigest>,
    expected: ChangePlanDigest,
) -> Result<(), Diagnostic> {
    match (action, reviewed) {
        (ChangeAction::Plan, None) => Ok(()),
        (ChangeAction::Plan, Some(_)) => Err(usage_error("change plan does not accept --plan")),
        (ChangeAction::Apply, None) => Err(usage_error(
            "change apply requires the exact --plan DIGEST returned by change plan",
        )),
        (ChangeAction::Apply, Some(reviewed)) if reviewed != expected => Err(Diagnostic::new(
            DiagnosticClass::Semantic,
            "change_plan_mismatch",
            format!("reviewed plan {reviewed} does not match normalized input {expected}"),
        )),
        (ChangeAction::Apply, Some(_)) => Ok(()),
    }
}

fn execute_normalized_change(
    project: Option<PathBuf>,
    action: ChangeAction,
    request: ChangeCommandRequest,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let repository = open_normalized_repository(project).map_err(single_diagnostic)?;
    let prepared = repository
        .prepare_authored_change(&request.normalized.semantic, request.normalized.options)?;
    if action == ChangeAction::Plan {
        return compact_change_response(
            &repository,
            &prepared,
            "prepared",
            request.normalized.plan,
            request.input_file.as_deref(),
        )
        .map_err(single_diagnostic);
    }
    let outcome = repository
        .publish(&prepared.publication)
        .map_err(single_diagnostic)?;
    let status = match &outcome {
        GraphPublicationOutcome::Accepted { .. } => "accepted",
        GraphPublicationOutcome::AlreadyAccepted { .. } => "already-accepted",
        GraphPublicationOutcome::Stale { expected, current } => {
            return Err(single_diagnostic(Diagnostic::new(
                DiagnosticClass::Semantic,
                "change_stale_base",
                format!(
                    "publication base changed after preparation: expected {}, observed {}",
                    expected
                        .as_ref()
                        .map_or_else(|| "absent".to_owned(), |head| head.revision.to_string()),
                    current
                        .as_ref()
                        .map_or_else(|| "absent".to_owned(), |head| head.revision.to_string())
                ),
            )));
        }
    };
    compact_change_response(
        &repository,
        &prepared,
        status,
        request.normalized.plan,
        None,
    )
    .map_err(single_diagnostic)
}

fn compact_change_response(
    repository: &GraphRepository,
    prepared: &PreparedAuthoredPublication,
    status: &str,
    plan: ChangePlanDigest,
    input_file: Option<&str>,
) -> Result<Vec<u8>, Diagnostic> {
    let publication = &prepared.publication;
    let [base] = publication.receipt.bases.as_slice() else {
        return Err(Diagnostic::new(
            DiagnosticClass::Corrupt,
            "change_prepared_base",
            "prepared authored change does not bind one exact accepted base",
        ));
    };
    let registry = registry_snapshot().map_err(contract_registry_error)?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", status.to_owned()),
            (
                "command",
                if status == "prepared" {
                    "change.plan"
                } else {
                    "change.apply"
                }
                .to_owned(),
            ),
        ],
    )?;
    append_compact_record(
        &mut output,
        "project",
        &[
            ("path", repository.root().display().to_string()),
            ("repository", publication.head.repository_id.to_string()),
            (
                "package",
                publication.authority.semantic.root.package_id.to_string(),
            ),
        ],
    )?;
    append_compact_record(
        &mut output,
        "revision",
        &[
            ("base", base.to_string()),
            ("result", publication.head.revision.to_string()),
        ],
    )?;
    append_compact_record(&mut output, "plan", &[("digest", plan.to_string())])?;
    append_compact_record(
        &mut output,
        "change",
        &[
            ("transaction", publication.transaction_digest.to_string()),
            (
                "semantic-diff",
                publication.semantic_diff_digest.to_string(),
            ),
        ],
    )?;
    for (symbol, owner) in &prepared.allocated {
        append_compact_record(
            &mut output,
            "identity",
            &[("symbol", symbol.clone()), ("id", owner.to_string())],
        )?;
    }
    let counts = publication.receipt.counts;
    append_compact_record(
        &mut output,
        "summary",
        &[
            ("created", counts.owners_created.to_string()),
            ("updated", counts.owners_updated.to_string()),
            ("deleted", counts.owners_deleted.to_string()),
            ("types", counts.type_objects_added.to_string()),
            ("dependencies", counts.dependencies_changed.to_string()),
            ("retirements", counts.retirements_changed.to_string()),
            ("witness", counts.witness_entries_changed.to_string()),
        ],
    )?;
    let validation = publication.receipt.validation;
    append_compact_record(
        &mut output,
        "validation",
        &[
            (
                "structural-owners",
                validation.structurally_checked.to_string(),
            ),
            (
                "semantic-owners",
                validation.semantically_checked.to_string(),
            ),
            ("summaries-reused", validation.summaries_reused.to_string()),
            (
                "relation-edges",
                validation.reverse_edges_visited.to_string(),
            ),
            ("tests-selected", validation.tests_selected.to_string()),
            ("tests-passed", validation.tests_passed.to_string()),
            (
                "compiler-units",
                validation.compiler_units_planned.to_string(),
            ),
        ],
    )?;
    append_compact_record(
        &mut output,
        "receipt",
        &[
            ("digest", publication.receipt_digest.to_string()),
            ("revision-record", publication.revision_digest.to_string()),
        ],
    )?;
    append_compact_record(&mut output, "schema", &[("registry", registry.digest)])?;
    if status == "prepared" {
        let mut next = vec![("kind", "apply".to_owned()), ("plan", plan.to_string())];
        if let Some(path) = input_file
            && path != "<change-input>"
        {
            next.push(("input-file", path.to_owned()));
        }
        append_compact_record(&mut output, "next", &next)?;
    }
    Ok(output.finish())
}

fn single_diagnostic(diagnostic: Diagnostic) -> Vec<Diagnostic> {
    vec![diagnostic]
}

/// Reads one exact owner from the accepted normalized authority observed by a revision-pinned
/// repository view. The selector never consults predecessor workspace or query indexes.
pub fn execute_inspect_owner(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    execute_inspect_owner_with_limits(
        arguments,
        CompactResponseLimits {
            maximum_bytes: MAXIMUM_CLI_RESPONSE_BYTES,
            maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
        },
    )
}

/// Dispatches the complete released inspect family without falling back to predecessor JSON.
/// Only exact coarse-owner summaries are current while other inspect actions remain removed.
pub fn execute_inspect(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (filtered, _) = extract_global_project(arguments.clone())?;
    if filtered.first().map(String::as_str) != Some("inspect") {
        return Err(usage_error("inspect dispatch requires the inspect command"));
    }
    match filtered.get(1).map(String::as_str) {
        Some("owner") => execute_inspect_owner(arguments),
        Some(
            action @ ("status" | "project" | "targets" | "revision" | "artifact" | "deployment"),
        ) => Err(owner_inspection_error(
            DiagnosticClass::Source,
            "predecessor_contract",
            format!(
                "inspect action '{action}' belongs to the removed predecessor control contract; use 'capabilities inspect'"
            ),
        )),
        Some(action) => Err(usage_error(format!(
            "unknown inspect action '{action}'; use 'capabilities inspect'"
        ))),
        None => Err(usage_error(
            "inspect requires an action; use 'capabilities inspect'",
        )),
    }
}

fn execute_inspect_owner_with_limits(
    arguments: Vec<String>,
    limits: CompactResponseLimits,
) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.first().map(String::as_str) == Some("inspect")
        && arguments.get(1).map(String::as_str) == Some("owner")
        && arguments
            .get(2)
            .and_then(|value| value.parse::<KernelOwnerKey>().ok())
            .is_some()
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "predecessor_contract",
            "the predecessor 'inspect owner ID' syntax is removed; use 'inspect owner KIND ID'",
        ));
    }
    let kind_name = arguments
        .get(2)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| {
            usage_error("inspect owner requires KIND and ID; use 'capabilities inspect'")
        })?;
    let identity = arguments
        .get(3)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| {
            usage_error("inspect owner requires KIND and ID; use 'capabilities inspect'")
        })?;
    if arguments.first().map(String::as_str) != Some("inspect")
        || arguments.get(1).map(String::as_str) != Some("owner")
    {
        return Err(usage_error(
            "normalized owner inspection requires 'inspect owner KIND ID'",
        ));
    }
    ensure_options(&arguments[4..], &["--package"], &[])?;
    let requested_kind = parse_kernel_owner_kind(kind_name)?;
    let owner = parse_kernel_owner_key(identity)?;
    if !requested_kind.accepts_owner(owner) {
        return Err(owner_inspection_error(
            DiagnosticClass::Semantic,
            "owner_wrong_kind",
            format!(
                "owner identity '{identity}' cannot identify semantic kind '{}'",
                requested_kind.name()
            ),
        ));
    }

    let repository = open_normalized_repository(project)?;
    let view = repository.view_current()?;
    let requested_package = option_value(&arguments[4..], "--package")?
        .map(|value| parse_kernel_package(&value))
        .transpose()?
        .unwrap_or_else(|| view.package());
    if requested_package != view.package() {
        return Err(owner_inspection_error(
            DiagnosticClass::Semantic,
            "owner_foreign_package",
            format!(
                "owner selector names package '{requested_package}', but the observed project package is '{}'",
                view.package()
            ),
        ));
    }

    let owner_read = view.owner(owner)?;
    let record = owner_read.value.as_ref().ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Semantic,
            "owner_not_found",
            format!(
                "owner '{identity}' is not live at revision '{}'",
                owner_read.revision
            ),
        )
    })?;
    if record.kind() != requested_kind {
        return Err(owner_inspection_error(
            DiagnosticClass::Semantic,
            "owner_wrong_kind",
            format!(
                "owner '{identity}' has kind '{}', not requested kind '{}'",
                record.kind().name(),
                requested_kind.name()
            ),
        ));
    }
    let summary_read = view.bound_owner_summary(owner)?;
    let summary = summary_read.value.as_ref().ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Corrupt,
            "publication_summary_binding",
            "accepted owner has no bound validation summary",
        )
    })?;
    if summary_read.revision != owner_read.revision {
        return Err(owner_inspection_error(
            DiagnosticClass::Corrupt,
            "publication_summary_binding",
            "owner record and validation summary were read at different revisions",
        ));
    }

    let mut output = CompactResponseWriter::new(limits)?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "inspect.owner".to_owned()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "project",
        &[
            ("path", repository.root().display().to_string()),
            (
                "name",
                view.current()
                    .semantic_root
                    .package_name
                    .as_str()
                    .to_owned(),
            ),
            ("repository", view.current().head.repository_id.to_string()),
            ("package", view.package().to_string()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "revision",
        &[("observed", owner_read.revision.to_string())],
    )?;
    let mut owner_fields = vec![
        ("id", owner.to_string()),
        ("kind", record.kind().name().to_owned()),
        ("detail", "summary".to_owned()),
        ("record", summary.summary.record.to_string()),
        ("summary", summary.digest.to_string()),
    ];
    if let Some(name) = record.name() {
        owner_fields.push(("name", name.as_str().to_owned()));
    }
    append_compact_record(&mut output, "owner", &owner_fields)?;
    let summary_fields = vec![
        ("type-roots", record.type_roots().len().to_string()),
        (
            "expression-roots",
            record.expression_roots().len().to_string(),
        ),
        ("blob-roots", record.blob_roots().len().to_string()),
        (
            "test",
            if summary.summary.test.is_some() {
                "present"
            } else {
                "absent"
            }
            .to_owned(),
        ),
    ];
    append_compact_record(&mut output, "summary", &summary_fields)?;
    Ok(output.finish())
}

fn open_normalized_repository(project: Option<PathBuf>) -> Result<GraphRepository, Diagnostic> {
    let start = match project {
        Some(path) => path,
        None => std::env::current_dir().map_err(|error| {
            Diagnostic::new(
                DiagnosticClass::Infrastructure,
                "project_io",
                format!("current directory is unavailable: {error}"),
            )
        })?,
    };
    discover_project(&start)
}

fn parse_kernel_owner_kind(value: &str) -> Result<KernelOwnerKind, Diagnostic> {
    KernelOwnerKind::PUBLIC_EXACT
        .into_iter()
        .find(|kind| kind.name() == value)
        .ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "owner_selector_kind",
                format!("owner kind '{value}' is not a current public exact-owner kind"),
            )
        })
}

fn parse_kernel_package(value: &str) -> Result<PackageId, Diagnostic> {
    value.parse().map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "owner_selector_identity",
            format!("package identity '{value}' is malformed"),
        )
    })
}

fn parse_kernel_owner_key(value: &str) -> Result<KernelOwnerKey, Diagnostic> {
    value.parse().map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "owner_selector_identity",
            format!("owner identity '{value}' is malformed"),
        )
    })
}

fn owner_inspection_error(
    class: DiagnosticClass,
    code: &'static str,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
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

pub fn execute_capabilities(arguments: &[String]) -> Result<Vec<u8>, Diagnostic> {
    let snapshot = registry_snapshot().map_err(contract_registry_error)?;
    let command = arguments.first().filter(|value| !value.starts_with("--"));
    if let Some(command) = command {
        exact_arguments(arguments, 1, "capabilities COMMAND")?;
        let operation = PublicOperation::parse(command)
            .ok_or_else(|| usage_error(format!("unknown public command '{command}'")))?;
        let descriptor = operation_descriptors()
            .iter()
            .find(|descriptor| descriptor.operation == operation)
            .ok_or_else(|| internal_error("registered public operation has no descriptor"))?;
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.command".to_owned()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        let record = operation_record(descriptor).map_err(contract_registry_error)?;
        output.append_serialized_records(record.as_bytes())?;
        return Ok(output.finish());
    }

    ensure_options(
        arguments,
        &[
            "--known-registry",
            "--known-section",
            "--section",
            "--output",
            "--generate-docs",
            "--verify-generated",
        ],
        &[],
    )?;
    let selected_section = option_value(arguments, "--section")?
        .map(|value| {
            RegistrySection::parse(&value)
                .ok_or_else(|| usage_error(format!("unknown registry section '{value}'")))
        })
        .transpose()?;
    let known_registry = option_value(arguments, "--known-registry")?;
    if let Some(digest) = &known_registry {
        validate_capability_digest(digest, "--known-registry")?;
    }
    let known_sections = parse_known_sections(&option_values(arguments, "--known-section")?)?;
    let output_path = option_value(arguments, "--output")?;
    let generated_directory = option_value(arguments, "--generate-docs")?;
    let verify_directory = option_value(arguments, "--verify-generated")?;
    if [
        output_path.is_some(),
        generated_directory.is_some(),
        verify_directory.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count()
        > 1
    {
        return Err(usage_error(
            "--output, --generate-docs, and --verify-generated are mutually exclusive",
        ));
    }
    if selected_section.is_some() && !known_sections.is_empty() {
        return Err(usage_error(
            "--section and --known-section are mutually exclusive",
        ));
    }
    if (generated_directory.is_some() || verify_directory.is_some())
        && (selected_section.is_some() || !known_sections.is_empty() || known_registry.is_some())
    {
        return Err(usage_error(
            "generated-document operations do not accept registry selection or digest options",
        ));
    }

    if let Some(directory) = generated_directory {
        let documents = generated_documents().map_err(contract_registry_error)?;
        let directory = PathBuf::from(directory);
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.generate-docs".to_owned()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        for document in documents {
            ensure_capability_export_bound(&document.bytes)?;
            let path = directory.join(document.relative_path);
            let status = write_derived_output(
                &path,
                &document.bytes,
                MAXIMUM_CLI_RESPONSE_BYTES,
                "generated contract document",
            )?;
            append_file_record(
                &mut output,
                "markdown",
                &path,
                &document.bytes,
                &generated_document_digest(&document.bytes),
                status,
            )?;
        }
        return Ok(output.finish());
    }

    if let Some(directory) = verify_directory {
        let documents = generated_documents().map_err(contract_registry_error)?;
        let directory = PathBuf::from(directory);
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.verify-generated".to_owned()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        for document in &documents {
            let path = directory.join(document.relative_path);
            let observed = read_bounded(
                &path,
                MAXIMUM_CLI_RESPONSE_BYTES,
                "generated contract document",
            )?;
            if observed != document.bytes {
                return Err(Diagnostic::new(
                    DiagnosticClass::Source,
                    "contract_generated_drift",
                    format!(
                        "generated contract document '{}' is stale; run 'lkjscript capabilities --generate-docs {}'",
                        path.display(),
                        directory.display()
                    ),
                ));
            }
            append_file_record(
                &mut output,
                "markdown",
                &path,
                &document.bytes,
                &generated_document_digest(&document.bytes),
                "current",
            )?;
        }
        return Ok(output.finish());
    }

    if let Some(path) = output_path {
        let (kind, bytes, digest) = match selected_section {
            Some(section) => {
                let section = snapshot
                    .section(section)
                    .ok_or_else(|| internal_error("registered section is missing"))?;
                (
                    section.section.name(),
                    section.bytes.as_slice(),
                    section.digest.as_str(),
                )
            }
            None => (
                "registry",
                snapshot.bytes.as_slice(),
                snapshot.digest.as_str(),
            ),
        };
        ensure_capability_export_bound(bytes)?;
        let path = PathBuf::from(path);
        let status = write_derived_output(
            &path,
            bytes,
            MAXIMUM_CLI_RESPONSE_BYTES,
            "compact registry output",
        )?;
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.output".to_owned()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        append_file_record(&mut output, kind, &path, bytes, digest, status)?;
        return Ok(output.finish());
    }

    if let Some(section) = selected_section {
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.section".to_owned()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        append_section(&mut output, &snapshot, section, true, None)?;
        return Ok(output.finish());
    }

    if !known_sections.is_empty() {
        let unchanged = known_sections.iter().all(|(section, known)| {
            snapshot
                .section(*section)
                .is_some_and(|current| current.digest == *known)
        });
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.changed-sections".to_owned()),
                ("unchanged", unchanged.to_string()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        for (section, known) in known_sections {
            let changed = snapshot
                .section(section)
                .is_some_and(|current| current.digest != known);
            append_section(&mut output, &snapshot, section, changed, Some(changed))?;
        }
        return Ok(output.finish());
    }

    if known_registry.as_deref() == Some(snapshot.digest.as_str()) {
        let mut output = compact_response_writer()?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities".to_owned()),
                ("unchanged", "true".to_owned()),
            ],
        )?;
        append_registry_summary(&mut output, &snapshot)?;
        return Ok(output.finish());
    }

    let mut output = compact_response_writer()?;
    append_capability_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "capabilities".to_owned()),
            ("unchanged", "false".to_owned()),
        ],
    )?;
    append_registry_summary(&mut output, &snapshot)?;
    append_capability_record(
        &mut output,
        "summary",
        &[
            ("operations", operation_descriptors().len().to_string()),
            ("sections", RegistrySection::ALL.len().to_string()),
        ],
    )?;
    for section in RegistrySection::ALL {
        append_section(&mut output, &snapshot, section, false, None)?;
    }
    for descriptor in operation_descriptors() {
        append_capability_record(
            &mut output,
            "command",
            &[("name", descriptor.operation.name().to_owned())],
        )?;
    }
    for (kind, command) in [
        ("command", "lkjscript capabilities COMMAND"),
        ("section", "lkjscript capabilities --section SECTION"),
        ("export", "lkjscript capabilities --output PATH"),
    ] {
        append_capability_record(
            &mut output,
            "next",
            &[("kind", kind.to_owned()), ("command", command.to_owned())],
        )?;
    }
    Ok(output.finish())
}

fn append_registry_summary(
    output: &mut CompactResponseWriter,
    snapshot: &RegistrySnapshot,
) -> Result<(), Diagnostic> {
    append_capability_record(
        output,
        "registry",
        &[
            ("contract", snapshot.contract.to_owned()),
            ("version", snapshot.version.to_string()),
            ("digest", snapshot.digest.clone()),
            ("graph", snapshot.graph_contract.to_owned()),
            ("cli", snapshot.cli_contract_version.to_string()),
        ],
    )
}

fn append_section(
    output: &mut CompactResponseWriter,
    registry: &RegistrySnapshot,
    section: RegistrySection,
    include_records: bool,
    changed: Option<bool>,
) -> Result<(), Diagnostic> {
    let snapshot = registry
        .section(section)
        .ok_or_else(|| internal_error("registered section is missing"))?;
    let mut fields = vec![
        ("name", section.name().to_owned()),
        ("digest", snapshot.digest.clone()),
        ("records", snapshot.records.to_string()),
        ("bytes", snapshot.bytes.len().to_string()),
    ];
    if let Some(changed) = changed {
        fields.push(("changed", changed.to_string()));
    }
    append_capability_record(output, "section", &fields)?;
    if include_records {
        output.append_serialized_records(&snapshot.bytes)?;
    }
    Ok(())
}

fn append_file_record(
    output: &mut CompactResponseWriter,
    kind: &str,
    path: &Path,
    bytes: &[u8],
    digest: &str,
    status: &str,
) -> Result<(), Diagnostic> {
    append_capability_record(
        output,
        "file",
        &[
            ("kind", kind.to_owned()),
            ("path", path.display().to_string()),
            ("bytes", bytes.len().to_string()),
            ("digest", digest.to_owned()),
            ("status", status.to_owned()),
        ],
    )
}

fn append_capability_record(
    output: &mut CompactResponseWriter,
    operation: &str,
    fields: &[(&str, String)],
) -> Result<(), Diagnostic> {
    append_compact_record(output, operation, fields)
}

fn append_compact_record(
    output: &mut CompactResponseWriter,
    operation: &str,
    fields: &[(&str, String)],
) -> Result<(), Diagnostic> {
    let borrowed = fields
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    output.append_record(operation, &borrowed)
}

fn compact_response_writer() -> Result<CompactResponseWriter, Diagnostic> {
    CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: MAXIMUM_CLI_RESPONSE_BYTES,
        maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
    })
}

fn ensure_capability_export_bound(bytes: &[u8]) -> Result<(), Diagnostic> {
    if bytes.len() > MAXIMUM_CLI_RESPONSE_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "contract_output_budget",
            format!(
                "compact registry output exceeds the hard {MAXIMUM_CLI_RESPONSE_BYTES}-byte bound"
            ),
        ));
    }
    Ok(())
}

fn validate_capability_digest(value: &str, option: &str) -> Result<(), Diagnostic> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(usage_error(format!(
            "{option} digest must be 64 lowercase hexadecimal characters"
        )));
    }
    Ok(())
}

fn generated_document_digest(bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.generated-document.v1");
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    hasher.finalize().to_hex().to_string()
}

fn parse_known_sections(
    values: &[String],
) -> Result<std::collections::BTreeMap<RegistrySection, String>, Diagnostic> {
    let mut output = std::collections::BTreeMap::new();
    for value in values {
        let (name, digest) = value
            .split_once('=')
            .ok_or_else(|| usage_error("--known-section requires the exact SECTION=DIGEST form"))?;
        let section = RegistrySection::parse(name)
            .ok_or_else(|| usage_error(format!("unknown registry section '{name}'")))?;
        validate_capability_digest(digest, "--known-section")?;
        if output.insert(section, digest.to_owned()).is_some() {
            return Err(usage_error(format!(
                "known section '{}' may be supplied only once",
                section.name()
            )));
        }
    }
    Ok(output)
}

fn contract_registry_error(message: String) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "contract_registry_invalid",
        message,
    )
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

fn required_option(arguments: &[String], name: &str) -> Result<String, Diagnostic> {
    option_value(arguments, name)?.ok_or_else(|| usage_error(format!("{name} is required")))
}

fn direct_option_error(name: &str, mut diagnostic: Diagnostic) -> Diagnostic {
    diagnostic.message = format!("{name} has an invalid typed value: {}", diagnostic.message);
    diagnostic
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
    use crate::platform::control::parse_records;

    #[test]
    fn unknown_options_and_noncanonical_numbers_reject() {
        assert!(execute(vec!["unknown".to_owned()]).is_err());
        assert!(parse_usize("01", "limit").is_err());
        assert!(ensure_options(&["--wat".to_owned()], &["--limit"], &[]).is_err());
    }

    #[test]
    fn command_registry_has_unique_names_and_stable_digest() {
        let commands = operation_descriptors();
        let names = commands
            .iter()
            .map(|entry| entry.operation.name())
            .collect::<BTreeSet<_>>();
        assert_eq!(commands.len(), names.len());
        let registry = registry_snapshot().expect("compact registry");
        assert_eq!(registry.digest.len(), 64);
        assert!(
            std::str::from_utf8(&registry.bytes)
                .expect("UTF-8 registry")
                .contains("operation name=change")
        );
    }

    #[test]
    fn direct_rename_and_compact_records_normalize_to_identical_typed_intent() {
        let base = RevisionId::from_digest([0x41; 32]);
        let owner = KernelOwnerKey::Module(ModuleId::migrate(b"direct-rename-owner", 1));
        for (idempotency, intent) in [
            (None, None),
            (
                Some("transport-equality"),
                Some("rename through either adapter"),
            ),
        ] {
            let mut direct = vec![
                "--base".to_owned(),
                base.to_string(),
                "--owner".to_owned(),
                owner.to_string(),
                "--name".to_owned(),
                "renamed".to_owned(),
            ];
            let mut request = format!("request base={base}");
            if let Some(value) = idempotency {
                direct.extend(["--idempotency".to_owned(), value.to_owned()]);
                request.push_str(&format!(" idempotency={value}"));
            }
            if let Some(value) = intent {
                direct.extend(["--intent".to_owned(), value.to_owned()]);
                request.push_str(&format!(" intent=\"{value}\""));
            }
            request.push_str(&format!("\nrename.owner owner={owner} name=renamed\n"));

            let direct = decode_direct_rename(ChangeAction::Plan, &direct)
                .expect("direct rename normalization")
                .normalized;
            let compact = decode_compact_change("request", request.as_bytes())
                .expect("compact rename normalization");
            assert_eq!(direct.semantic, compact.semantic);
            assert_eq!(direct.options, compact.options);
            assert_eq!(direct.plan, compact.plan);
            assert_eq!(
                crate::platform::change::canonical_authored_intent_bytes(&direct.semantic)
                    .expect("direct canonical intent"),
                crate::platform::change::canonical_authored_intent_bytes(&compact.semantic)
                    .expect("compact canonical intent")
            );
        }
    }

    #[test]
    fn normalized_owner_inspection_is_exact_compact_and_independently_bounded() {
        let temporary = tempfile::TempDir::new().expect("temporary repository parent");
        let destination = temporary.path().join("project");
        let snapshot = crate::platform::kernel::tests::witness_snapshot();
        let owner = *snapshot
            .owners
            .iter()
            .find(|(_, record)| KernelOwnerKind::PUBLIC_EXACT.contains(&record.kind()))
            .map(|(owner, _)| owner)
            .expect("coarse fixture owner");
        let expected = snapshot.owners[&owner].clone();
        let expected_id = owner.to_string();
        GraphRepository::create(&destination, &snapshot, None).expect("normalized repository");
        let arguments = vec![
            "--project".to_owned(),
            destination.display().to_string(),
            "inspect".to_owned(),
            "owner".to_owned(),
            expected.kind().name().to_owned(),
            expected_id.clone(),
        ];

        let bytes = execute_inspect_owner(arguments.clone()).expect("exact owner inspection");
        let records = parse_records("response", &bytes).expect("compact response");
        assert_eq!(records.len(), 5);
        assert_eq!(records[0].operation, "result");
        assert_eq!(records[1].operation, "project");
        assert_eq!(records[2].operation, "revision");
        assert_eq!(records[3].operation, "owner");
        assert_eq!(records[4].operation, "summary");
        assert_eq!(
            records[3]
                .fields
                .iter()
                .find(|field| field.name == "id")
                .map(|field| field.value.as_str()),
            Some(expected_id.as_str())
        );

        let record_error = execute_inspect_owner_with_limits(
            arguments.clone(),
            CompactResponseLimits {
                maximum_bytes: MAXIMUM_CLI_RESPONSE_BYTES,
                maximum_records: 3,
            },
        )
        .expect_err("record budget");
        assert_eq!(record_error.code, "control_response_record_budget");
        let byte_error = execute_inspect_owner_with_limits(
            arguments,
            CompactResponseLimits {
                maximum_bytes: 1,
                maximum_records: MAXIMUM_CLI_RESPONSE_RECORDS,
            },
        )
        .expect_err("byte budget");
        assert_eq!(byte_error.code, "control_response_byte_budget");
    }

    #[test]
    fn normalized_owner_selector_rejects_unknown_domains_before_repository_access() {
        let error = execute_inspect_owner(vec![
            "inspect".to_owned(),
            "owner".to_owned(),
            "module".to_owned(),
            "decl_not-hex".to_owned(),
        ])
        .expect_err("malformed identity");
        assert_eq!(error.code, "owner_selector_identity");

        let error = execute_inspect_owner(vec![
            "inspect".to_owned(),
            "owner".to_owned(),
            "repository".to_owned(),
            "repo_00000000000000000000000000000001".to_owned(),
        ])
        .expect_err("unknown owner kind");
        assert_eq!(error.code, "owner_selector_kind");

        let error = execute_inspect_owner(vec![
            "inspect".to_owned(),
            "owner".to_owned(),
            "field".to_owned(),
            "field_00000000000000000000000000000001".to_owned(),
        ])
        .expect_err("fine owner identity remains private");
        assert_eq!(error.code, "owner_selector_kind");
    }
}
