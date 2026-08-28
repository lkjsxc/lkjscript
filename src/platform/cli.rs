//! Strict, bounded public graph-native command projection.

use super::change::{AuthoredChange, AuthoredChangeSet, OwnerSelector};
use super::compiler::{MAXIMUM_ARTIFACT_BUNDLE_BYTES, build_incremental, load_current_compilation};
use super::contract::{
    MAXIMUM_CLI_RESPONSE_BYTES, MAXIMUM_CLI_RESPONSE_RECORDS, PublicOperation, RegistrySection,
    RegistrySnapshot, diagnostic_class_name, generated_documents, operation_descriptors,
    operation_record, registry_snapshot,
};
use super::control::{
    ChangePlanToken, CompactChangeOperation, CompactResponseLimits, CompactResponseWriter,
    LogicalChangePlan, LogicalPlanEncoding, MAXIMUM_COMPACT_INPUT_BYTES,
    MAXIMUM_LOGICAL_PLAN_BYTES, NormalizedChangeRequest, compact_change_operation_descriptor,
    decode_compact_change, encode_logical_change_plan, normalize_change_request,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::ExecutionControl;
use super::execution::normalized::NormalizedCommandPolicy;
use super::kernel::{Name, OwnerKey as KernelOwnerKey, OwnerKind as KernelOwnerKind, PackageId};
use super::normalized_lifecycle::{PreparedApplication, prepare_repository};
use super::normalized_query::{execute_normalized_query, parse_query_arguments};
use super::owned_output::publish_create_new;
use super::project_creation::{ProjectTemplate, create_project};
use super::project_discovery::discover_project;
use super::publication::{
    GraphRepository, PreparedAuthoredPublication, PublicationOptions,
    PublicationOutcome as GraphPublicationOutcome,
};
use super::semantic_id::{RepositoryId, RevisionId};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn execute_new(arguments: &[String]) -> Result<Vec<u8>, Diagnostic> {
    let destination = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("new requires one destination directory"))?;
    ensure_options(&arguments[1..], &["--template", "--name"], &[])?;
    let template_name =
        option_value(&arguments[1..], "--template")?.unwrap_or_else(|| "minimal".to_owned());
    let template = ProjectTemplate::parse(&template_name).ok_or_else(|| {
        usage_error(format!(
            "unknown normalized project template '{template_name}'; expected minimal, command, or http"
        ))
    })?;
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
    let created = create_project(Path::new(destination), &package_name, template)?;
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
            ("template", created.template.name().to_owned()),
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
            ("owners", created.owners.to_string()),
            ("dependencies", created.dependencies.to_string()),
            ("retirements", "0".to_owned()),
            ("targets", created.targets.to_string()),
            ("tests", created.tests.to_string()),
        ],
    )?;
    if let Some(deployment) = &created.deployment {
        append_compact_record(
            &mut output,
            "deployment",
            &[
                ("descriptor", deployment.descriptor.display().to_string()),
                (
                    "artifact-output",
                    deployment.recommended_artifact_output.display().to_string(),
                ),
                ("target", deployment.target.to_owned()),
                ("runner", deployment.runner.to_owned()),
                ("listener", deployment.configured_listener.to_owned()),
            ],
        )?;
    }
    let project = created.project.display().to_string();
    append_compact_record(
        &mut output,
        "next",
        &[
            ("order", "1".to_owned()),
            ("kind", "status".to_owned()),
            ("operation", "status".to_owned()),
            ("project", project.clone()),
        ],
    )?;
    append_compact_record(
        &mut output,
        "next",
        &[
            ("order", "2".to_owned()),
            ("kind", "check".to_owned()),
            ("operation", "check".to_owned()),
            ("project", project.clone()),
        ],
    )?;
    if let Some(deployment) = &created.deployment {
        append_compact_record(
            &mut output,
            "next",
            &[
                ("order", "3".to_owned()),
                ("kind", "response-change-plan".to_owned()),
                ("operation", "change".to_owned()),
                ("mode", "plan".to_owned()),
                ("project", project.clone()),
                ("base", created.revision.to_string()),
                ("expression", "expression.static-text".to_owned()),
                ("binding", "$response".to_owned()),
                ("function", "application/response-text".to_owned()),
                ("replacement", "replace.body".to_owned()),
            ],
        )?;
        append_compact_record(
            &mut output,
            "next",
            &[
                ("order", "4".to_owned()),
                ("kind", "response-change-apply".to_owned()),
                ("operation", "change".to_owned()),
                ("mode", "apply".to_owned()),
                ("project", project.clone()),
                ("input", "same-normalized-request".to_owned()),
                ("plan", "token-from-order-3".to_owned()),
            ],
        )?;
        append_compact_record(
            &mut output,
            "next",
            &[
                ("order", "5".to_owned()),
                ("kind", "build".to_owned()),
                ("operation", "build".to_owned()),
                ("project", project.clone()),
                (
                    "output",
                    deployment.recommended_artifact_output.display().to_string(),
                ),
            ],
        )?;
        append_compact_record(
            &mut output,
            "next",
            &[
                ("order", "6".to_owned()),
                ("kind", "serve".to_owned()),
                ("operation", "serve".to_owned()),
                ("deployment", deployment.descriptor.display().to_string()),
            ],
        )?;
    }
    Ok(output.finish())
}

pub fn execute_check(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.as_slice() != ["check"] {
        return Err(usage_error("check accepts no additional arguments"));
    }
    let repository = open_normalized_repository(project)?;
    let prepared = prepare_repository(repository)?;
    let checked = prepared.check(&ExecutionControl::default())?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "check".to_owned()),
        ],
    )?;
    append_lifecycle_records(&mut output, &prepared)?;
    append_compact_record(
        &mut output,
        "tests",
        &[
            ("passed", checked.passed.to_string()),
            ("failed", checked.failed.to_string()),
            ("differential", checked.differential.to_owned()),
            (
                "production-instructions",
                checked.production_instructions.to_string(),
            ),
            (
                "reference-expressions",
                checked.reference_expressions.to_string(),
            ),
        ],
    )?;
    Ok(output.finish())
}

pub fn execute_build(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.first().map(String::as_str) != Some("build") {
        return Err(usage_error("build requires the build operation name"));
    }
    ensure_options(&arguments[1..], &["--output"], &[])?;
    let output_path = required_option(&arguments[1..], "--output")?;
    let repository = open_normalized_repository(project)?;
    let prepared = prepare_repository(repository)?;
    let publication = publish_create_new(
        Path::new(&output_path),
        &prepared.artifact_bytes,
        maximum_artifact_output_bytes()?,
        "normalized graph artifact",
    )?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "build".to_owned()),
        ],
    )?;
    append_lifecycle_records(&mut output, &prepared)?;
    append_compact_record(
        &mut output,
        "output",
        &[
            ("path", publication.path.display().to_string()),
            ("bytes", publication.bytes.to_string()),
            ("visibility", publication.visibility.to_owned()),
            ("durability", publication.durability.to_owned()),
            ("stage-cleanup", publication.stage_cleanup.to_owned()),
        ],
    )?;
    Ok(output.finish())
}

fn maximum_artifact_output_bytes() -> Result<usize, Diagnostic> {
    usize::try_from(MAXIMUM_ARTIFACT_BUNDLE_BYTES)
        .ok()
        .and_then(|maximum| maximum.checked_add(50))
        .ok_or_else(|| {
            Diagnostic::new(
                DiagnosticClass::Resource,
                "artifact_output_limit",
                "artifact output limit is not representable on this platform",
            )
        })
}

pub fn execute_run(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.first().map(String::as_str) != Some("run") {
        return Err(usage_error("run requires the run operation name"));
    }
    let target = arguments
        .get(1)
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("run requires one target name"))?;
    ensure_options(&arguments[2..], &["--arguments"], &[])?;
    let encoded_arguments =
        option_value(&arguments[2..], "--arguments")?.unwrap_or_else(|| "[]".to_owned());
    let repository = open_normalized_repository(project)?;
    let prepared = prepare_repository(repository)?;
    let run = prepared.run(
        &Name::new(target.clone())?,
        encoded_arguments.as_bytes(),
        NormalizedCommandPolicy::default(),
        &ExecutionControl::default(),
    )?;
    let result = String::from_utf8(run.result_json).map_err(|_| {
        Diagnostic::new(
            DiagnosticClass::Corrupt,
            "run_result_utf8",
            "normalized typed result is not canonical UTF-8 JSON",
        )
    })?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "run".to_owned()),
        ],
    )?;
    append_lifecycle_records(&mut output, &prepared)?;
    append_compact_record(
        &mut output,
        "execution",
        &[
            ("target", run.target.as_str().to_owned()),
            ("value", result),
            ("differential", run.differential.to_owned()),
            (
                "production-instructions",
                run.production.instructions.to_string(),
            ),
            (
                "reference-expressions",
                run.reference.expressions.to_string(),
            ),
        ],
    )?;
    Ok(output.finish())
}

pub fn execute_package_builtin(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if project.is_some() {
        return Err(usage_error("package builtin does not accept --project"));
    }
    if arguments.first().map(String::as_str) != Some("package")
        || arguments.get(1).map(String::as_str) != Some("builtin")
    {
        return Err(usage_error("package requires the exact builtin action"));
    }
    let action = arguments.get(2).map(String::as_str).unwrap_or("inspect");
    let standard = super::builtin_standard::BuiltinStandard::load()?;
    let mut output = compact_response_writer()?;
    append_compact_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", format!("package.builtin.{action}")),
        ],
    )?;
    append_compact_record(
        &mut output,
        "package",
        &[
            ("id", standard.package.to_string()),
            ("revision", standard.semantic_revision.to_string()),
            ("package-revision", standard.package_revision.to_string()),
            ("transport", standard.package_transport.to_string()),
            (
                "artifact-manifest",
                standard.artifact.manifest_digest.to_string(),
            ),
            (
                "artifact-bundle",
                standard.artifact.bundle_digest.to_string(),
            ),
        ],
    )?;
    match action {
        "inspect" => {
            exact_arguments(&arguments, 3, "package builtin inspect")?;
            append_compact_record(
                &mut output,
                "interface",
                &[
                    ("owners", standard.interface_owners.len().to_string()),
                    ("types", standard.interface_types.len().to_string()),
                    (
                        "transport-bytes",
                        standard.transport_bytes().len().to_string(),
                    ),
                    (
                        "artifact-bytes",
                        standard.artifact_bytes().len().to_string(),
                    ),
                ],
            )?;
        }
        "export" => {
            ensure_options(&arguments[3..], &["--kind", "--output"], &[])?;
            let kind = required_option(&arguments[3..], "--kind")?;
            let path = required_option(&arguments[3..], "--output")?;
            let (bytes, maximum, digest) = match kind.as_str() {
                "transport" => (
                    standard.transport_bytes(),
                    maximum_artifact_output_bytes()?,
                    standard.package_transport.to_string(),
                ),
                "artifact" => (
                    standard.artifact_bytes(),
                    maximum_artifact_output_bytes()?,
                    standard.artifact.bundle_digest.to_string(),
                ),
                _ => {
                    return Err(usage_error(
                        "package builtin export --kind expects transport or artifact",
                    ));
                }
            };
            let publication =
                publish_create_new(Path::new(&path), bytes, maximum, "built-in standard asset")?;
            append_compact_record(
                &mut output,
                "output",
                &[
                    ("kind", kind),
                    ("path", publication.path.display().to_string()),
                    ("bytes", publication.bytes.to_string()),
                    ("digest", digest),
                    ("visibility", publication.visibility.to_owned()),
                    ("durability", publication.durability.to_owned()),
                    ("stage-cleanup", publication.stage_cleanup.to_owned()),
                ],
            )?;
        }
        _ => {
            return Err(usage_error("package builtin accepts inspect or export"));
        }
    }
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
    reviewed: Option<ChangePlanToken>,
    input_file: Option<String>,
    output_file: Option<String>,
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
    require_reviewed_change_request(
        action,
        request.reviewed,
        request.normalized.request_commitment,
    )
    .map_err(single_diagnostic)?;
    execute_normalized_change(project, action, request)
}

fn decode_record_change(
    action: ChangeAction,
    options: &[String],
) -> Result<ChangeCommandRequest, Vec<Diagnostic>> {
    let allowed = match action {
        ChangeAction::Plan => &["--input", "--input-file", "--output"][..],
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
        .map(|value| value.parse::<ChangePlanToken>())
        .transpose()
        .map_err(single_diagnostic)?;
    let output_file = option_value(options, "--output").map_err(single_diagnostic)?;
    Ok(ChangeCommandRequest {
        normalized,
        reviewed,
        input_file: Some(source),
        output_file,
    })
}

fn decode_direct_rename(
    action: ChangeAction,
    options: &[String],
) -> Result<ChangeCommandRequest, Diagnostic> {
    let allowed = match action {
        ChangeAction::Plan => &[
            "--base",
            "--owner",
            "--name",
            "--idempotency",
            "--intent",
            "--output",
        ][..],
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
        .map(|value| value.parse::<ChangePlanToken>())
        .transpose()?;
    Ok(ChangeCommandRequest {
        normalized,
        reviewed,
        input_file: None,
        output_file: option_value(options, "--output")?,
    })
}

fn require_reviewed_change_request(
    action: ChangeAction,
    reviewed: Option<ChangePlanToken>,
    expected: super::control::ChangeRequestCommitment,
) -> Result<(), Diagnostic> {
    match (action, reviewed) {
        (ChangeAction::Plan, None) => Ok(()),
        (ChangeAction::Plan, Some(_)) => Err(usage_error("change plan does not accept --plan")),
        (ChangeAction::Apply, None) => Err(usage_error(
            "change apply requires the exact --plan TOKEN returned by change plan",
        )),
        (ChangeAction::Apply, Some(reviewed)) if reviewed.request != expected => {
            Err(Diagnostic::new(
                DiagnosticClass::Semantic,
                "change_request_commitment_mismatch",
                format!(
                    "reviewed request commitment {} does not match normalized input {expected}",
                    reviewed.request
                ),
            ))
        }
        (ChangeAction::Apply, Some(_)) => Ok(()),
    }
}

fn execute_normalized_change(
    project: Option<PathBuf>,
    action: ChangeAction,
    request: ChangeCommandRequest,
) -> Result<Vec<u8>, Vec<Diagnostic>> {
    let ChangeCommandRequest {
        normalized,
        reviewed,
        input_file,
        output_file,
    } = request;
    let request_commitment = normalized.request_commitment;
    let repository = open_normalized_repository(project).map_err(single_diagnostic)?;
    let retry_base = if action == ChangeAction::Apply {
        normalized
            .options
            .idempotency_key
            .as_deref()
            .map(|key| repository.view_idempotency_base(key, normalized.semantic.base))
            .transpose()
            .map_err(single_diagnostic)?
            .flatten()
    } else {
        None
    };
    let prepared = match retry_base {
        Some(view) => view.prepare_authored_change(&normalized.semantic, normalized.options)?,
        None => repository.prepare_authored_change(&normalized.semantic, normalized.options)?,
    };
    let logical_plan =
        LogicalChangePlan::new(request_commitment, &prepared).map_err(single_diagnostic)?;
    if action == ChangeAction::Plan {
        let (encoding, plan_output) = match output_file.as_deref() {
            Some(path) => {
                let (encoding, output) =
                    write_logical_plan_output(repository.root(), Path::new(path), &logical_plan)
                        .map_err(single_diagnostic)?;
                (encoding, Some(output))
            }
            None => (
                encode_logical_change_plan(&logical_plan, |_| Ok(())).map_err(single_diagnostic)?,
                None,
            ),
        };
        return compact_change_response(
            &repository,
            &prepared,
            "prepared",
            encoding,
            input_file.as_deref(),
            plan_output.as_ref(),
            None,
        )
        .map_err(single_diagnostic);
    }
    let encoding =
        encode_logical_change_plan(&logical_plan, |_| Ok(())).map_err(single_diagnostic)?;
    let reviewed = reviewed.ok_or_else(|| {
        single_diagnostic(Diagnostic::new(
            DiagnosticClass::Infrastructure,
            "change_reviewed_plan_missing",
            "validated apply request lost its reviewed plan token",
        ))
    })?;
    if reviewed.prepared != encoding.token.prepared {
        return Err(single_diagnostic(Diagnostic::new(
            DiagnosticClass::Semantic,
            "change_prepared_plan_mismatch",
            format!(
                "reviewed prepared-plan commitment {} does not match reprepared logical plan {}",
                reviewed.prepared, encoding.token.prepared
            ),
        )));
    }
    // The cache is disposable derived state. Capture an exact base binding while the prepared
    // publication is still in memory, but never let cache discovery or maintenance decide the
    // semantic publication outcome.
    let base_cache = match load_current_compilation(&repository) {
        Ok(Some(compilation)) => DerivedCacheHandoff::Available(compilation.digest),
        Ok(None) => DerivedCacheHandoff::Unavailable,
        Err(diagnostic) => DerivedCacheHandoff::Failed(diagnostic),
    };
    let outcome = repository
        .publish(&prepared.publication)
        .map_err(single_diagnostic)?;
    let (status, cache) = match &outcome {
        GraphPublicationOutcome::Accepted { .. } => {
            let cache = match base_cache {
                DerivedCacheHandoff::Available(base) => {
                    match build_incremental(&repository, base, &prepared.publication) {
                        Ok(receipt) => DerivedCacheObservation {
                            status: "updated",
                            manifest: Some(receipt.manifest_digest.to_string()),
                            compiled: Some(receipt.units_compiled),
                            reused: Some(receipt.units_reused),
                            removed: Some(receipt.units_removed),
                            diagnostic: None,
                        },
                        Err(diagnostic) => DerivedCacheObservation::failed(diagnostic),
                    }
                }
                DerivedCacheHandoff::Unavailable => DerivedCacheObservation::unavailable(),
                DerivedCacheHandoff::Failed(diagnostic) => {
                    DerivedCacheObservation::failed(diagnostic)
                }
            };
            ("accepted", cache)
        }
        GraphPublicationOutcome::AlreadyAccepted { .. } => (
            "already-accepted",
            DerivedCacheObservation {
                status: "not-attempted-replay",
                manifest: None,
                compiled: None,
                reused: None,
                removed: None,
                diagnostic: None,
            },
        ),
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
        encoding,
        None,
        None,
        Some(&cache),
    )
    .map_err(single_diagnostic)
}

enum DerivedCacheHandoff {
    Available(super::compiler::CompilationManifestDigest),
    Unavailable,
    Failed(Diagnostic),
}

struct DerivedCacheObservation {
    status: &'static str,
    manifest: Option<String>,
    compiled: Option<u64>,
    reused: Option<u64>,
    removed: Option<u64>,
    diagnostic: Option<(String, String)>,
}

impl DerivedCacheObservation {
    fn unavailable() -> Self {
        Self {
            status: "not-available",
            manifest: None,
            compiled: None,
            reused: None,
            removed: None,
            diagnostic: None,
        }
    }

    fn failed(diagnostic: Diagnostic) -> Self {
        Self {
            status: "failed",
            manifest: None,
            compiled: None,
            reused: None,
            removed: None,
            diagnostic: Some((
                diagnostic_class_name(diagnostic.class).to_owned(),
                diagnostic.code,
            )),
        }
    }
}

struct LogicalPlanOutputPublication {
    path: String,
    status: &'static str,
    bytes: u64,
    records: u64,
}

fn write_logical_plan_output(
    project_root: &Path,
    requested: &Path,
    plan: &LogicalChangePlan<'_>,
) -> Result<(LogicalPlanEncoding, LogicalPlanOutputPublication), Diagnostic> {
    let file_name = requested
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| usage_error("logical plan output requires a portable UTF-8 file name"))?;
    let requested_parent = requested
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let parent = fs::canonicalize(requested_parent).map_err(|error| {
        plan_output_io_error("change_plan_output_parent", requested_parent, error)
    })?;
    let target = parent.join(file_name);
    let project = fs::canonicalize(project_root)
        .map_err(|error| plan_output_io_error("change_plan_output_project", project_root, error))?;
    if target.starts_with(&project) {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "change_plan_output_project_path",
            format!(
                "logical plan output '{}' must be outside normalized project root '{}'",
                target.display(),
                project.display()
            ),
        ));
    }
    validate_plan_output_target(&target)?;

    let temporary = parent.join(format!(".{file_name}.stage-{}", RepositoryId::generate()?));
    let result = (|| {
        let mut options = OpenOptions::new();
        options.create_new(true).write(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options
            .open(&temporary)
            .map_err(|error| plan_output_io_error("change_plan_output_create", &target, error))?;
        let encoding = encode_logical_change_plan(plan, |bytes| {
            file.write_all(bytes)
                .map_err(|error| plan_output_io_error("change_plan_output_write", &target, error))
        })?;
        file.sync_all()
            .map_err(|error| plan_output_io_error("change_plan_output_sync", &target, error))?;
        drop(file);

        validate_plan_output_target(&target)?;
        let unchanged = target.exists() && plan_output_files_equal(&temporary, &target)?;
        let status = if unchanged {
            fs::remove_file(&temporary).map_err(|error| {
                plan_output_io_error("change_plan_output_stage_remove", &target, error)
            })?;
            "unchanged"
        } else {
            fs::rename(&temporary, &target).map_err(|error| {
                plan_output_io_error("change_plan_output_publish", &target, error)
            })?;
            "published"
        };
        File::open(&parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                plan_output_io_error("change_plan_output_parent_sync", &parent, error)
            })?;
        Ok((
            encoding,
            LogicalPlanOutputPublication {
                path: target.display().to_string(),
                status,
                bytes: encoding.bytes,
                records: encoding.records,
            },
        ))
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn validate_plan_output_target(path: &Path) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(Diagnostic::new(
                DiagnosticClass::Source,
                "change_plan_output_type",
                format!(
                    "logical plan output '{}' is not an ordinary regular file",
                    path.display()
                ),
            ))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(plan_output_io_error(
            "change_plan_output_metadata",
            path,
            error,
        )),
    }
}

fn plan_output_files_equal(left: &Path, right: &Path) -> Result<bool, Diagnostic> {
    let right_metadata = fs::symlink_metadata(right).map_err(|error| {
        plan_output_io_error("change_plan_output_compare_metadata", right, error)
    })?;
    if right_metadata.file_type().is_symlink() || !right_metadata.is_file() {
        return Err(Diagnostic::new(
            DiagnosticClass::Source,
            "change_plan_output_type",
            format!(
                "logical plan output '{}' changed to a non-regular file before publication",
                right.display()
            ),
        ));
    }
    if right_metadata.len() > MAXIMUM_LOGICAL_PLAN_BYTES {
        return Err(Diagnostic::new(
            DiagnosticClass::Resource,
            "change_plan_output_existing_bytes",
            format!(
                "existing logical plan output '{}' exceeds the {MAXIMUM_LOGICAL_PLAN_BYTES}-byte comparison bound",
                right.display()
            ),
        ));
    }
    let left_metadata = fs::metadata(left).map_err(|error| {
        plan_output_io_error("change_plan_output_compare_metadata", left, error)
    })?;
    if left_metadata.len() != right_metadata.len() {
        return Ok(false);
    }
    let mut left_file = File::open(left)
        .map_err(|error| plan_output_io_error("change_plan_output_compare_open", left, error))?;
    let mut right_file = File::open(right)
        .map_err(|error| plan_output_io_error("change_plan_output_compare_open", right, error))?;
    let mut left_buffer = [0_u8; 16 * 1_024];
    let mut right_buffer = [0_u8; 16 * 1_024];
    let mut compared = 0_u64;
    loop {
        let left_read = left_file.read(&mut left_buffer).map_err(|error| {
            plan_output_io_error("change_plan_output_compare_read", left, error)
        })?;
        let right_read = right_file.read(&mut right_buffer).map_err(|error| {
            plan_output_io_error("change_plan_output_compare_read", right, error)
        })?;
        compared = compared
            .checked_add(u64::try_from(left_read.max(right_read)).unwrap_or(u64::MAX))
            .ok_or_else(|| {
                Diagnostic::new(
                    DiagnosticClass::Resource,
                    "change_plan_output_existing_bytes",
                    "logical plan output comparison byte count overflowed",
                )
            })?;
        if compared > MAXIMUM_LOGICAL_PLAN_BYTES {
            return Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "change_plan_output_existing_bytes",
                "logical plan output changed beyond its bounded comparison size",
            ));
        }
        if left_read != right_read || left_buffer[..left_read] != right_buffer[..right_read] {
            return Ok(false);
        }
        if left_read == 0 {
            return Ok(true);
        }
    }
}

fn plan_output_io_error(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Infrastructure,
        code,
        format!("logical plan output '{}' failed: {error}", path.display()),
    )
}

fn compact_change_response(
    repository: &GraphRepository,
    prepared: &PreparedAuthoredPublication,
    status: &str,
    plan: LogicalPlanEncoding,
    input_file: Option<&str>,
    plan_output: Option<&LogicalPlanOutputPublication>,
    cache: Option<&DerivedCacheObservation>,
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
    append_compact_record(
        &mut output,
        "plan",
        &[
            ("token", plan.token.to_string()),
            ("request-commitment", plan.token.request.to_string()),
            ("prepared-commitment", plan.token.prepared.to_string()),
        ],
    )?;
    if let Some(plan_output) = plan_output {
        append_compact_record(
            &mut output,
            "plan-output",
            &[
                ("path", plan_output.path.clone()),
                ("status", plan_output.status.to_owned()),
                ("bytes", plan_output.bytes.to_string()),
                ("records", plan_output.records.to_string()),
            ],
        )?;
    }
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
    if let Some(cache) = cache {
        let mut fields = vec![("status", cache.status.to_owned())];
        if let Some(manifest) = &cache.manifest {
            fields.push(("manifest", manifest.clone()));
        }
        if let Some(compiled) = cache.compiled {
            fields.push(("compiled", compiled.to_string()));
        }
        if let Some(reused) = cache.reused {
            fields.push(("reused", reused.to_string()));
        }
        if let Some(removed) = cache.removed {
            fields.push(("removed", removed.to_string()));
        }
        if let Some((class, code)) = &cache.diagnostic {
            fields.push(("diagnostic-class", class.clone()));
            fields.push(("diagnostic-code", code.clone()));
        }
        append_compact_record(&mut output, "derived-cache", &fields)?;
    }
    append_compact_record(&mut output, "schema", &[("registry", registry.digest)])?;
    if status == "prepared" {
        let mut next = vec![
            ("kind", "apply".to_owned()),
            ("plan", plan.token.to_string()),
        ];
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

/// Executes the exhaustive normalized query grammar through one compact revision-pinned path.
pub fn execute_query(arguments: Vec<String>) -> Result<Vec<u8>, Diagnostic> {
    let (arguments, project) = extract_global_project(arguments)?;
    if arguments.first().map(String::as_str) != Some("query") {
        return Err(usage_error("query dispatch requires the query command"));
    }
    let request = parse_query_arguments(&arguments[1..])?;
    let repository = open_normalized_repository(project)?;
    let view = repository.view_current()?;
    execute_normalized_query(&repository, &view, &request)
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
        let focused_section = match operation {
            PublicOperation::New => Some(RegistrySection::Templates),
            PublicOperation::Query => Some(RegistrySection::Query),
            _ => None,
        };
        if let Some(section_name) = focused_section {
            let section = snapshot.section(section_name).ok_or_else(|| {
                internal_error("registered operation has no focused registry section")
            })?;
            output.append_serialized_records(&section.bytes)?;
        }
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

fn append_lifecycle_records(
    output: &mut CompactResponseWriter,
    prepared: &PreparedApplication,
) -> Result<(), Diagnostic> {
    append_compact_record(
        output,
        "authority",
        &[
            ("repository", prepared.repository_id.to_string()),
            ("package", prepared.package.to_string()),
            ("revision", prepared.revision.to_string()),
            ("state", prepared.semantic_state.to_string()),
        ],
    )?;
    let mut compilation = vec![
        ("cache", prepared.cache_profile.name().to_owned()),
        ("manifest", prepared.compilation.to_string()),
        ("compiled", prepared.units_compiled.to_string()),
        ("reused", prepared.units_reused.to_string()),
        ("removed", prepared.units_removed.to_string()),
    ];
    if let Some(diagnostic) = &prepared.cache_recovery {
        compilation.push((
            "recovered-class",
            diagnostic_class_name(diagnostic.class).to_owned(),
        ));
        compilation.push(("recovered-code", diagnostic.code.clone()));
    }
    append_compact_record(output, "compilation", &compilation)?;
    append_compact_record(
        output,
        "artifact",
        &[
            ("manifest", prepared.artifact_manifest.to_string()),
            ("bundle", prepared.artifact_bundle.to_string()),
            ("bytes", prepared.artifact_bytes.len().to_string()),
            ("packages", prepared.link_work.packages.to_string()),
            (
                "closure-objects",
                prepared.link_work.closure_objects.to_string(),
            ),
            (
                "compiler-units",
                prepared.program.work.compiler_units.to_string(),
            ),
            (
                "manifest-objects",
                prepared
                    .program
                    .artifact()
                    .manifest
                    .object_count
                    .to_string(),
            ),
            (
                "manifest-object-bytes",
                prepared
                    .program
                    .artifact()
                    .manifest
                    .object_bytes
                    .to_string(),
            ),
            (
                "segments",
                prepared.program.artifact().segment_count.to_string(),
            ),
            (
                "load-objects",
                prepared.program.artifact().work.objects.to_string(),
            ),
            (
                "load-object-bytes",
                prepared.program.artifact().work.object_bytes.to_string(),
            ),
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

fn usage_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Source, "cli_usage", message)
}

fn internal_error(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Infrastructure, "cli_internal", message)
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
    use crate::platform::semantic_id::ModuleId;
    use std::collections::BTreeSet;

    #[test]
    fn unknown_options_reject_before_repository_access() {
        assert!(ensure_options(&["--wat".to_owned()], &["--limit"], &[]).is_err());
        assert!(execute_package_builtin(vec!["package".to_owned(), "stage".to_owned()]).is_err());
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
            assert_eq!(direct.request_commitment, compact.request_commitment);
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
