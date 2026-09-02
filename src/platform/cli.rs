//! Strict, bounded public graph-native command projection.

use super::builtin_discovery::{
    BUILTIN_QUERY_DEFAULT_BYTES, BUILTIN_QUERY_DEFAULT_ITEMS, BUILTIN_QUERY_ORDERING,
    BuiltinOwnerSelector, inspect_builtin_owner, parse_interface_owner_kind, query_builtin_owners,
};
use super::change::{AuthoredChange, AuthoredChangeSet, BoundOwnerSummary, OwnerSelector};
use super::compiler::{MAXIMUM_ARTIFACT_BUNDLE_BYTES, build_incremental, load_current_compilation};
use super::contract::{
    CapabilitiesSnapshot, FUNCTION_DEFINITION_CONTINUATION_INTEGRITY_DOMAIN,
    FUNCTION_DEFINITION_CONTINUATION_MAGIC_TEXT, FUNCTION_DEFINITION_DEFAULT_ITEMS,
    FUNCTION_DEFINITION_DEFAULT_OUTPUT_BYTES, FUNCTION_DEFINITION_LOGICAL_DIGEST_DOMAIN,
    FUNCTION_DEFINITION_PROJECTION_CONTRACT_IDENTITY,
    FUNCTION_DEFINITION_PROJECTION_CONTRACT_VERSION, FUNCTION_DEFINITION_RECORD_KEY_DOMAIN,
    FUNCTION_DEFINITION_RESPONSE_FIELDS, MAXIMUM_CLI_RESPONSE_BYTES, MAXIMUM_CLI_RESPONSE_RECORDS,
    MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS, MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_BYTES,
    MAXIMUM_FUNCTION_DEFINITION_DEPTH, MAXIMUM_FUNCTION_DEFINITION_EDGES,
    MAXIMUM_FUNCTION_DEFINITION_FACT_READS, MAXIMUM_FUNCTION_DEFINITION_ITEMS,
    MAXIMUM_FUNCTION_DEFINITION_LITERAL_FRAGMENT_BYTES, MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES,
    MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES, MINIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES,
    PublicOperation, RegistrySection, capabilities_snapshot, diagnostic_class_name,
    generated_documents, operation_descriptors, operation_record,
};
use super::control::{
    ChangePlanToken, CompactChangeOperation, CompactResponseLimits, CompactResponseWriter,
    LogicalChangePlan, LogicalPlanEncoding, MAXIMUM_COMPACT_INPUT_BYTES,
    MAXIMUM_LOGICAL_PLAN_BYTES, NormalizedChangeRequest, compact_change_operation_descriptor,
    decode_compact_change, encode_logical_change_plan, normalize_change_request, render_record,
};
use super::data::{DataLimits, DataStore};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::execution::ExecutionControl;
use super::execution::normalized::NormalizedCommandPolicy;
use super::kernel::{
    BindingKind, DeclarationPayload, DeclarationReference, DeclarationVisibility, EncodedOwnerKey,
    ExpressionChildRole, ExpressionOperation, FieldSelector, FunctionEffect, LocalValueReference,
    Name, OperationReference, OwnerKey as KernelOwnerKey, OwnerKind as KernelOwnerKind,
    OwnerRecord, PackageId, PackageTransportDigest, ParameterParent, ParameterUse,
    RequirementReference, ResourceUnit, TextValue, TypeObjectDigest, encode_owner,
};
use super::normalized_lifecycle::{PreparedApplication, prepare_repository};
use super::normalized_query::{execute_normalized_query, parse_query_arguments};
use super::owned_output::publish_create_new;
use super::project_creation::{ProjectTemplate, create_project, create_project_with_relay};
use super::project_discovery::discover_project;
use super::publication::{
    GraphRepository, PreparedAuthoredPublication, PublicationOptions,
    PublicationOutcome as GraphPublicationOutcome, RepositoryDefinitionReader, RepositoryReadWork,
    RepositoryView,
};
use super::semantic_id::{RepositoryId, RevisionId, encode_hex};
use super::storage::contract::MAXIMUM_PACK_BYTES;
use super::witness::{
    BindingContainerRole, ExpressionRootRole, OwnershipEntry, OwnershipParent, OwnershipRole,
    SemanticDigest,
};
use base64::Engine;
use std::collections::BTreeSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub fn execute_data(arguments: &[String]) -> Result<Vec<u8>, Diagnostic> {
    let action = arguments
        .first()
        .map(String::as_str)
        .ok_or_else(|| usage_error("data requires initialize, verify, backup, or restore"))?;
    let options = &arguments[1..];
    let mut output = compact_response_writer()?;
    match action {
        "initialize" => {
            ensure_options(options, &["--root"], &[])?;
            let root = required_option(options, "--root")?;
            let receipt = DataStore::initialize(Path::new(&root))?;
            append_compact_record(
                &mut output,
                "result",
                &[
                    ("status", "success".to_owned()),
                    ("command", "data.initialize".to_owned()),
                    (
                        "outcome",
                        format!("{:?}", receipt.outcome).to_ascii_lowercase(),
                    ),
                ],
            )?;
            append_compact_record(
                &mut output,
                "data",
                &[
                    ("root", root),
                    ("store", receipt.store),
                    ("revision", receipt.revision),
                ],
            )?;
        }
        "verify" => {
            ensure_options(options, &["--root"], &[])?;
            let root = required_option(options, "--root")?;
            let store = DataStore::open(Path::new(&root), "lifecycle", DataLimits::default())?;
            let receipt = store.verify()?;
            append_compact_record(
                &mut output,
                "result",
                &[
                    ("status", "success".to_owned()),
                    ("command", "data.verify".to_owned()),
                ],
            )?;
            append_compact_record(
                &mut output,
                "data",
                &[
                    ("root", root),
                    ("store", receipt.store),
                    ("revision", receipt.revision),
                    ("revisions", receipt.revisions.to_string()),
                    ("objects", receipt.objects.to_string()),
                    ("schemas", receipt.schemas.to_string()),
                    ("records", receipt.records.to_string()),
                    ("staging-leftovers", receipt.staging_leftovers.to_string()),
                    ("bytes-read", receipt.bytes_read.to_string()),
                ],
            )?;
        }
        "backup" => {
            ensure_options(options, &["--root", "--output"], &[])?;
            let root = required_option(options, "--root")?;
            let destination = required_option(options, "--output")?;
            let store = DataStore::open(Path::new(&root), "lifecycle", DataLimits::default())?;
            let receipt = store.backup(Path::new(&destination))?;
            append_compact_record(
                &mut output,
                "result",
                &[
                    ("status", "success".to_owned()),
                    ("command", "data.backup".to_owned()),
                ],
            )?;
            append_compact_record(
                &mut output,
                "backup",
                &[
                    ("root", root),
                    ("output", destination),
                    ("store", receipt.store),
                    ("revision", receipt.revision),
                    ("digest", receipt.digest),
                    ("schemas", receipt.schemas.to_string()),
                    ("records", receipt.records.to_string()),
                    ("bytes", receipt.bytes.to_string()),
                ],
            )?;
        }
        "restore" => {
            ensure_options(options, &["--backup", "--root"], &[])?;
            let backup = required_option(options, "--backup")?;
            let root = required_option(options, "--root")?;
            let receipt = DataStore::restore(Path::new(&backup), Path::new(&root))?;
            append_compact_record(
                &mut output,
                "result",
                &[
                    ("status", "success".to_owned()),
                    ("command", "data.restore".to_owned()),
                    ("outcome", "created".to_owned()),
                ],
            )?;
            append_compact_record(
                &mut output,
                "data",
                &[
                    ("backup", backup),
                    ("root", root),
                    ("store", receipt.store),
                    ("revision", receipt.revision),
                ],
            )?;
        }
        _ => {
            return Err(usage_error(format!(
                "unknown data action '{action}'; expected initialize, verify, backup, or restore"
            )));
        }
    }
    Ok(output.finish())
}

pub fn execute_new(arguments: &[String]) -> Result<Vec<u8>, Diagnostic> {
    let destination = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .ok_or_else(|| usage_error("new requires one destination directory"))?;
    ensure_options(
        &arguments[1..],
        &["--template", "--name", "--relay-url"],
        &[],
    )?;
    let template_name =
        option_value(&arguments[1..], "--template")?.unwrap_or_else(|| "minimal".to_owned());
    let template = ProjectTemplate::parse(&template_name).ok_or_else(|| {
        usage_error(format!(
            "unknown normalized project template '{template_name}'; expected minimal, command, http, or nostr-relay-info"
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
    let relay_url = option_value(&arguments[1..], "--relay-url")?;
    let created = match (template, relay_url.as_deref()) {
        (ProjectTemplate::NostrRelayInfo, Some(relay_url)) => {
            create_project_with_relay(Path::new(destination), &package_name, relay_url)?
        }
        (ProjectTemplate::NostrRelayInfo, None) => {
            return Err(usage_error(
                "nostr-relay-info requires exactly one --relay-url URL",
            ));
        }
        (_, Some(_)) => {
            return Err(usage_error(
                "--relay-url is accepted only with --template nostr-relay-info",
            ));
        }
        (_, None) => create_project(Path::new(destination), &package_name, template)?,
    };
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
    if created.deployment.is_some() && created.template == ProjectTemplate::Http {
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
    }
    if let Some(deployment) = &created.deployment {
        let build_order = if created.template == ProjectTemplate::Http {
            "5"
        } else {
            "3"
        };
        let serve_order = if created.template == ProjectTemplate::Http {
            "6"
        } else {
            "4"
        };
        append_compact_record(
            &mut output,
            "next",
            &[
                ("order", build_order.to_owned()),
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
                ("order", serve_order.to_owned()),
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
    if arguments.first().map(String::as_str) != Some("package") {
        return Err(usage_error(
            "package requires builtin, current, or dependency",
        ));
    }
    match arguments.get(1).map(String::as_str) {
        Some("current") => {
            if arguments.get(2).map(String::as_str) != Some("export") {
                return Err(usage_error(
                    "package current requires the exact export action",
                ));
            }
            ensure_options(&arguments[3..], &["--kind", "--output"], &[])?;
            if required_option(&arguments[3..], "--kind")? != "transport" {
                return Err(usage_error(
                    "package current export --kind requires transport",
                ));
            }
            let path = required_option(&arguments[3..], "--output")?;
            let repository = open_normalized_repository(project)?;
            let exported = repository.export_package_transport()?;
            let [pack] = exported.packs.as_slice() else {
                return Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "package_transport_pack_count",
                    "current public package export requires one bounded interface pack",
                ));
            };
            let publication = publish_create_new(
                Path::new(&path),
                pack,
                MAXIMUM_PACK_BYTES,
                "current package transport",
            )?;
            let mut output = compact_response_writer()?;
            append_compact_record(
                &mut output,
                "result",
                &[
                    ("status", "success".to_owned()),
                    ("command", "package.current.export".to_owned()),
                ],
            )?;
            append_compact_record(
                &mut output,
                "package",
                &[
                    ("id", exported.revision.package.to_string()),
                    (
                        "revision",
                        exported.revision.revision.revision_id()?.to_string(),
                    ),
                    ("package-revision", exported.revision_digest.to_string()),
                    ("transport", exported.transport_digest.to_string()),
                    ("owners", exported.interface_owner_count.to_string()),
                    ("types", exported.interface_type_count.to_string()),
                ],
            )?;
            append_compact_record(
                &mut output,
                "output",
                &[
                    ("kind", "transport".to_owned()),
                    ("path", publication.path.display().to_string()),
                    ("bytes", publication.bytes.to_string()),
                    ("digest", exported.transport_digest.to_string()),
                    ("visibility", publication.visibility.to_owned()),
                    ("durability", publication.durability.to_owned()),
                    ("stage-cleanup", publication.stage_cleanup.to_owned()),
                ],
            )?;
            return Ok(output.finish());
        }
        Some("dependency") => {
            if arguments.get(2).map(String::as_str) != Some("stage") {
                return Err(usage_error(
                    "package dependency requires the exact stage action",
                ));
            }
            ensure_options(&arguments[3..], &["--transport", "--input-file"], &[])?;
            let digest = required_option(&arguments[3..], "--transport")?
                .parse::<PackageTransportDigest>()
                .map_err(|error| direct_option_error("--transport", error))?;
            let path = required_option(&arguments[3..], "--input-file")?;
            let pack = read_bounded(
                Path::new(&path),
                MAXIMUM_PACK_BYTES,
                "package transport pack",
            )?;
            let repository = open_normalized_repository(project)?;
            let receipt = repository.stage_package_transport(digest, &[pack])?;
            let mut output = compact_response_writer()?;
            append_compact_record(
                &mut output,
                "result",
                &[
                    ("status", "success".to_owned()),
                    ("command", "package.dependency.stage".to_owned()),
                    (
                        "outcome",
                        format!("{:?}", receipt.outcome).to_ascii_lowercase(),
                    ),
                ],
            )?;
            append_compact_record(
                &mut output,
                "package",
                &[
                    ("id", receipt.package.to_string()),
                    ("revision", receipt.semantic_revision.to_string()),
                    ("package-revision", receipt.package_revision.to_string()),
                    ("transport", receipt.package_transport.to_string()),
                    (
                        "previous-transport",
                        receipt
                            .previous_transport
                            .map(|value| value.to_string())
                            .unwrap_or_else(|| "none".to_owned()),
                    ),
                ],
            )?;
            append_compact_record(
                &mut output,
                "authority",
                &[
                    ("current-revision", receipt.current_revision.to_string()),
                    ("semantic-head-changed", "false".to_owned()),
                ],
            )?;
            append_compact_record(
                &mut output,
                "work",
                &[
                    ("objects-staged", receipt.work.objects_staged.to_string()),
                    ("objects-reused", receipt.work.objects_reused.to_string()),
                    ("bytes-staged", receipt.work.bytes_staged.to_string()),
                    ("packs-sealed", receipt.work.packs_sealed.to_string()),
                ],
            )?;
            return Ok(output.finish());
        }
        Some("builtin") => {}
        _ => {
            return Err(usage_error(
                "package requires builtin, current, or dependency",
            ));
        }
    }
    if project.is_some() {
        return Err(usage_error("package builtin does not accept --project"));
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
        "inspect" => match arguments.get(3).map(String::as_str) {
            None => {
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
            Some("owner") => {
                exact_arguments(&arguments, 6, "package builtin inspect owner KIND ID")?;
                let kind = parse_interface_owner_kind(&arguments[4])?;
                let owner = parse_kernel_owner_key(&arguments[5])?;
                for record in inspect_builtin_owner(standard, kind, owner)? {
                    append_dynamic_record(&mut output, &record.operation, &record.fields)?;
                }
            }
            Some(_) => {
                return Err(usage_error(
                    "package builtin inspect accepts no selector or owner KIND ID",
                ));
            }
        },
        "query" => {
            if arguments.get(3).map(String::as_str) != Some("owners") {
                return Err(usage_error(
                    "package builtin query requires the exact owners action",
                ));
            }
            ensure_options(
                &arguments[4..],
                &[
                    "--kind",
                    "--name",
                    "--parent",
                    "--limit",
                    "--bytes",
                    "--continuation",
                ],
                &[],
            )?;
            let kind = option_value(&arguments[4..], "--kind")?
                .map(|value| parse_interface_owner_kind(&value))
                .transpose()?;
            let name = option_value(&arguments[4..], "--name")?
                .map(|value| {
                    Name::new(value)
                        .map(|name| name.as_str().to_owned())
                        .map_err(|error| {
                            Diagnostic::new(error.class, "builtin_owner_name", error.message)
                        })
                })
                .transpose()?;
            let parent = option_value(&arguments[4..], "--parent")?
                .map(|value| parse_kernel_owner_key(&value))
                .transpose()?;
            let limit = parse_usize_option(
                option_value(&arguments[4..], "--limit")?,
                "--limit",
                BUILTIN_QUERY_DEFAULT_ITEMS,
            )?;
            let bytes = parse_usize_option(
                option_value(&arguments[4..], "--bytes")?,
                "--bytes",
                BUILTIN_QUERY_DEFAULT_BYTES,
            )?;
            let continuation = option_value(&arguments[4..], "--continuation")?;
            let selector = BuiltinOwnerSelector { kind, name, parent };
            let page =
                query_builtin_owners(standard, &selector, limit, bytes, continuation.as_deref())?;
            append_compact_record(
                &mut output,
                "query",
                &[
                    ("operation", "owners".to_owned()),
                    ("selector-digest", page.selector_digest.clone()),
                    ("package-revision", standard.package_revision.to_string()),
                    ("ordering", BUILTIN_QUERY_ORDERING.to_owned()),
                    (
                        "kind",
                        selector
                            .kind
                            .map(KernelOwnerKind::name)
                            .unwrap_or("any")
                            .to_owned(),
                    ),
                    (
                        "name",
                        selector.name.clone().unwrap_or_else(|| "any".to_owned()),
                    ),
                    (
                        "parent",
                        selector
                            .parent
                            .map(|parent| parent.to_string())
                            .unwrap_or_else(|| "any".to_owned()),
                    ),
                ],
            )?;
            for record in page.records {
                append_dynamic_record(&mut output, &record.operation, &record.fields)?;
            }
            append_compact_record(
                &mut output,
                "summary",
                &[
                    ("matched", page.matched.to_string()),
                    ("returned", page.returned.to_string()),
                    ("truncated", page.truncated.to_string()),
                    ("owner-record-bytes", page.rendered_owner_bytes.to_string()),
                ],
            )?;
            if let Some(token) = page.continuation {
                append_compact_record(&mut output, "continuation", &[("token", token)])?;
            }
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
            return Err(usage_error(
                "package builtin accepts inspect, query, or export",
            ));
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
    let capabilities = capabilities_snapshot().map_err(capabilities_projection_error)?;

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
    append_compact_record(
        &mut output,
        "schema",
        &[("capabilities", capabilities.digest)],
    )?;
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
    let capabilities = capabilities_snapshot().map_err(capabilities_projection_error)?;
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
    append_compact_record(
        &mut output,
        "schema",
        &[("capabilities", capabilities.digest)],
    )?;
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct DefinitionPageRequest {
    items: u64,
    output_bytes: usize,
    continuation: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum OwnerInspectionDetail {
    Summary,
    Definition(DefinitionPageRequest),
}

fn parse_owner_inspection_detail(
    arguments: &[String],
) -> Result<OwnerInspectionDetail, Diagnostic> {
    ensure_options(
        arguments,
        &[
            "--package",
            "--detail",
            "--limit",
            "--bytes",
            "--continuation",
        ],
        &[],
    )?;
    let detail = option_value(arguments, "--detail")?;
    let has_page_option = ["--limit", "--bytes", "--continuation"]
        .into_iter()
        .any(|name| arguments.iter().any(|argument| argument == name));
    let Some(detail) = detail else {
        if has_page_option {
            return Err(owner_inspection_error(
                DiagnosticClass::Source,
                "definition_detail_required",
                "--limit, --bytes, and --continuation require --detail definition",
            ));
        }
        return Ok(OwnerInspectionDetail::Summary);
    };
    if detail != "definition" {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_detail_value",
            format!("inspection detail '{detail}' is unknown; expected definition"),
        ));
    }
    let items = option_value(arguments, "--limit")?
        .map(|value| parse_definition_item_limit(&value))
        .transpose()?
        .unwrap_or(FUNCTION_DEFINITION_DEFAULT_ITEMS);
    let output_bytes = option_value(arguments, "--bytes")?
        .map(|value| parse_definition_byte_limit(&value))
        .transpose()?
        .unwrap_or(FUNCTION_DEFINITION_DEFAULT_OUTPUT_BYTES);
    let continuation = option_value(arguments, "--continuation")?;
    if continuation
        .as_ref()
        .is_some_and(|token| token.len() > MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_BYTES)
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_oversized",
            format!(
                "definition continuation exceeds {MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_BYTES} encoded bytes"
            ),
        ));
    }
    Ok(OwnerInspectionDetail::Definition(DefinitionPageRequest {
        items,
        output_bytes,
        continuation,
    }))
}

fn parse_definition_item_limit(value: &str) -> Result<u64, Diagnostic> {
    let parsed = value.parse::<u64>().map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_invalid_limit",
            format!("definition item limit '{value}' is not a canonical positive integer"),
        )
    })?;
    if parsed == 0 || parsed > MAXIMUM_FUNCTION_DEFINITION_ITEMS || parsed.to_string() != value {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_invalid_limit",
            format!("definition item limit must be 1 through {MAXIMUM_FUNCTION_DEFINITION_ITEMS}"),
        ));
    }
    Ok(parsed)
}

fn parse_definition_byte_limit(value: &str) -> Result<usize, Diagnostic> {
    let parsed = value.parse::<usize>().map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_invalid_byte_limit",
            format!("definition output-byte limit '{value}' is not canonical"),
        )
    })?;
    if !(MINIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES..=MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES)
        .contains(&parsed)
        || parsed.to_string() != value
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_invalid_byte_limit",
            format!(
                "definition output-byte limit must be {MINIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES} through {MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES}"
            ),
        ));
    }
    Ok(parsed)
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
    let detail = parse_owner_inspection_detail(&arguments[4..])?;
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
        if matches!(detail, OwnerInspectionDetail::Definition(_)) {
            return Err(owner_inspection_error(
                DiagnosticClass::Capability,
                "definition_dependency_body",
                format!(
                    "definition detail is local-only; package '{requested_package}' is not the current local package '{}'",
                    view.package()
                ),
            ));
        }
        return Err(owner_inspection_error(
            DiagnosticClass::Semantic,
            "owner_foreign_package",
            format!(
                "owner selector names package '{requested_package}', but the observed project package is '{}'",
                view.package()
            ),
        ));
    }

    if let OwnerInspectionDetail::Definition(request) = detail {
        if !matches!(
            requested_kind,
            KernelOwnerKind::PureFunction | KernelOwnerKind::TaskFunction
        ) {
            return Err(owner_inspection_error(
                DiagnosticClass::Semantic,
                "definition_owner_kind",
                format!(
                    "definition detail requires pure_function or task_function, not '{}'",
                    requested_kind.name()
                ),
            ));
        }
        let control = ExecutionControl::uncancelled();
        let mut cancellation = || check_definition_control(&control);
        return execute_function_definition(
            &repository,
            &view,
            requested_kind,
            owner,
            &request,
            limits,
            &mut cancellation,
        );
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

const FUNCTION_DEFINITION_ORDERING: u8 = 1;
const FUNCTION_DEFINITION_ORDERING_NAME: &str =
    "header-contract-structural-preorder-reference-fact-v1";
const FUNCTION_DEFINITION_CONTINUATION_MAGIC: [u8; 8] = *b"LKJICT01";
const FUNCTION_DEFINITION_CONTINUATION_VERSION: u16 = 1;
const FUNCTION_DEFINITION_CONTINUATION_ENVELOPE_VERSION: u16 = 1;
const FUNCTION_DEFINITION_CONTINUATION_HEADER_BYTES: usize = 18;
const FUNCTION_DEFINITION_CONTINUATION_CHECKSUM_BYTES: usize = 32;
const MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_DECODED_BYTES: usize = 224;
const FUNCTION_DEFINITION_CONTINUATION_PREFIX: &str = "icont_";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum DefinitionSection {
    Header,
    Contract,
    Body,
    Reference,
    Fact,
}

impl DefinitionSection {
    const fn tag(self) -> u8 {
        match self {
            Self::Header => 1,
            Self::Contract => 2,
            Self::Body => 3,
            Self::Reference => 4,
            Self::Fact => 5,
        }
    }

    const fn name(self) -> &'static str {
        match self {
            Self::Header => "header",
            Self::Contract => "contract",
            Self::Body => "body",
            Self::Reference => "reference",
            Self::Fact => "fact",
        }
    }

    fn from_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::Header),
            2 => Some(Self::Contract),
            3 => Some(Self::Body),
            4 => Some(Self::Reference),
            5 => Some(Self::Fact),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DefinitionDigest([u8; 32]);

impl DefinitionDigest {
    const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl fmt::Display for DefinitionDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("definition_")?;
        formatter.write_str(&encode_hex(&self.0))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DefinitionLogicalRecord {
    section: DefinitionSection,
    bytes: Vec<u8>,
    key: [u8; 32],
}

#[derive(Clone, Debug)]
struct DefinitionProjection {
    records: Vec<DefinitionLogicalRecord>,
    digest: DefinitionDigest,
    contract_records: u64,
    body_records: u64,
    reference_records: u64,
    fact_records: u64,
    structural_edges: u64,
    reference_edges: u64,
    fact_reads: u64,
    maximum_depth: u64,
    logical_bytes: usize,
    validator: String,
    certificate: String,
    work: RepositoryReadWork,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct DefinitionReferenceProjection {
    role: String,
    target_kind: String,
    target: String,
    source: String,
    ordinal: u32,
}

#[derive(Clone, Debug)]
struct DefinitionFactProjection {
    owner: KernelOwnerKey,
    kind: KernelOwnerKind,
    summary: BoundOwnerSummary,
}

#[derive(Clone, Debug)]
struct DefinitionPosition {
    parent: KernelOwnerKey,
    ownership_role: OwnershipRole,
    slot: &'static str,
    index: u32,
    label: Option<String>,
    depth: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct DefinitionBinding {
    repository: RepositoryId,
    package: PackageId,
    revision: RevisionId,
    function: KernelOwnerKey,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DecodedDefinitionContinuation {
    binding: DefinitionBinding,
    projection: DefinitionDigest,
    ordering: u8,
    section: DefinitionSection,
    index: u32,
    resume_key: [u8; 32],
}

fn definition_record(
    section: DefinitionSection,
    operation: &'static str,
    fields: &[(&'static str, String)],
) -> Result<DefinitionLogicalRecord, Diagnostic> {
    for (name, _) in fields {
        if !FUNCTION_DEFINITION_RESPONSE_FIELDS.contains(&(operation, *name)) {
            return Err(owner_inspection_error(
                DiagnosticClass::Infrastructure,
                "definition_response_field_inventory",
                format!(
                    "definition renderer field '{operation}.{name}' is absent from its executable inventory"
                ),
            ));
        }
    }
    let borrowed = fields
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    let rendered = render_record(operation, &borrowed)?;
    let bytes = rendered.into_bytes();
    let mut key_hasher = blake3::Hasher::new_derive_key(FUNCTION_DEFINITION_RECORD_KEY_DOMAIN);
    key_hasher.update(&[section.tag()]);
    key_hasher.update(&(bytes.len() as u64).to_be_bytes());
    key_hasher.update(&bytes);
    Ok(DefinitionLogicalRecord {
        section,
        bytes,
        key: *key_hasher.finalize().as_bytes(),
    })
}

fn definition_logical_digest(
    records: &[DefinitionLogicalRecord],
) -> Result<(DefinitionDigest, usize), Diagnostic> {
    let mut logical_bytes = 0_usize;
    let mut hasher = blake3::Hasher::new_derive_key(FUNCTION_DEFINITION_LOGICAL_DIGEST_DOMAIN);
    hasher.update(&(records.len() as u64).to_be_bytes());
    for record in records {
        logical_bytes = logical_bytes
            .checked_add(record.bytes.len())
            .ok_or_else(|| {
                owner_inspection_error(
                    DiagnosticClass::Resource,
                    "definition_logical_byte_limit",
                    "definition logical byte accounting overflowed",
                )
            })?;
        if logical_bytes > MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES {
            return Err(owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_logical_byte_limit",
                format!(
                    "definition logical encoding exceeds {MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES} bytes"
                ),
            ));
        }
        hasher.update(&[record.section.tag()]);
        hasher.update(&(record.bytes.len() as u64).to_be_bytes());
        hasher.update(&record.bytes);
    }
    Ok((
        DefinitionDigest(*hasher.finalize().as_bytes()),
        logical_bytes,
    ))
}

fn check_definition_control(control: &ExecutionControl) -> Result<(), Diagnostic> {
    control.check().map_err(|error| {
        owner_inspection_error(
            DiagnosticClass::Cancelled,
            "definition_cancelled",
            format!("function-definition projection: {}", error.message),
        )
    })
}

fn encode_definition_continuation(
    binding: DefinitionBinding,
    projection: DefinitionDigest,
    record_index: usize,
    record: &DefinitionLogicalRecord,
) -> Result<String, Diagnostic> {
    let index = u32::try_from(record_index).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_continuation_resume_key",
            "definition record index cannot be represented by the continuation contract",
        )
    })?;
    let mut payload = Vec::with_capacity(155);
    payload.extend_from_slice(&FUNCTION_DEFINITION_CONTINUATION_VERSION.to_be_bytes());
    payload.extend_from_slice(&FUNCTION_DEFINITION_PROJECTION_CONTRACT_VERSION.to_be_bytes());
    payload.extend_from_slice(&binding.repository.bytes());
    payload.extend_from_slice(&binding.package.bytes());
    payload.extend_from_slice(&binding.revision.bytes());
    payload.extend_from_slice(&EncodedOwnerKey::new(binding.function).bytes());
    payload.extend_from_slice(&projection.bytes());
    payload.push(FUNCTION_DEFINITION_ORDERING);
    payload.push(record.section.tag());
    payload.extend_from_slice(&index.to_be_bytes());
    payload.extend_from_slice(&record.key);
    let payload_length = u64::try_from(payload.len()).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_continuation_oversized",
            "definition continuation payload length cannot be represented",
        )
    })?;
    let mut bytes = Vec::with_capacity(
        FUNCTION_DEFINITION_CONTINUATION_HEADER_BYTES
            .saturating_add(payload.len())
            .saturating_add(FUNCTION_DEFINITION_CONTINUATION_CHECKSUM_BYTES),
    );
    bytes.extend_from_slice(&FUNCTION_DEFINITION_CONTINUATION_MAGIC);
    bytes.extend_from_slice(&FUNCTION_DEFINITION_CONTINUATION_ENVELOPE_VERSION.to_le_bytes());
    bytes.extend_from_slice(&payload_length.to_le_bytes());
    bytes.extend_from_slice(&payload);
    bytes.extend_from_slice(&definition_domain_digest(
        FUNCTION_DEFINITION_CONTINUATION_INTEGRITY_DOMAIN,
        &bytes,
    ));
    let token = format!(
        "{FUNCTION_DEFINITION_CONTINUATION_PREFIX}{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    );
    if token.len() > MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_BYTES {
        return Err(owner_inspection_error(
            DiagnosticClass::Infrastructure,
            "definition_continuation_oversized",
            "canonical definition continuation exceeds its declared textual bound",
        ));
    }
    Ok(token)
}

fn decode_definition_continuation(
    token: &str,
) -> Result<DecodedDefinitionContinuation, Diagnostic> {
    if token.starts_with("cont_") || token.starts_with("qcont_") {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "predecessor_contract",
            "the supplied continuation does not belong to the definition projection contract",
        ));
    }
    if token.len() > MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_BYTES {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_oversized",
            format!(
                "definition continuation exceeds {MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_BYTES} encoded bytes"
            ),
        ));
    }
    let encoded = token
        .strip_prefix(FUNCTION_DEFINITION_CONTINUATION_PREFIX)
        .ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation has an unknown textual domain",
            )
        })?;
    if encoded.is_empty() || encoded.contains('=') {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_noncanonical",
            "definition continuation must use canonical unpadded URL-safe base64",
        ));
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation contains malformed URL-safe base64",
            )
        })?;
    if bytes.len() > MAXIMUM_FUNCTION_DEFINITION_CONTINUATION_DECODED_BYTES {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_oversized",
            "decoded definition continuation exceeds its strict canonical bound",
        ));
    }
    if base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&bytes) != encoded {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_noncanonical",
            "definition continuation base64 does not reproduce its canonical text",
        ));
    }
    if bytes.len()
        < FUNCTION_DEFINITION_CONTINUATION_HEADER_BYTES
            + FUNCTION_DEFINITION_CONTINUATION_CHECKSUM_BYTES
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_malformed",
            "definition continuation is truncated",
        ));
    }
    if bytes[..8] != FUNCTION_DEFINITION_CONTINUATION_MAGIC {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_contract",
            format!(
                "definition continuation uses an unknown contract magic; expected {FUNCTION_DEFINITION_CONTINUATION_MAGIC_TEXT}"
            ),
        ));
    }
    let envelope = u16::from_le_bytes([bytes[8], bytes[9]]);
    if envelope != FUNCTION_DEFINITION_CONTINUATION_ENVELOPE_VERSION {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_contract",
            "definition continuation uses a foreign envelope version",
        ));
    }
    let length_bytes: [u8; 8] = bytes[10..18].try_into().map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_malformed",
            "definition continuation has a malformed length field",
        )
    })?;
    let payload_length = usize::try_from(u64::from_le_bytes(length_bytes)).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_oversized",
            "definition continuation payload length cannot be represented",
        )
    })?;
    let expected_length = FUNCTION_DEFINITION_CONTINUATION_HEADER_BYTES
        .checked_add(payload_length)
        .and_then(|length| length.checked_add(FUNCTION_DEFINITION_CONTINUATION_CHECKSUM_BYTES))
        .ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_oversized",
                "definition continuation length overflowed",
            )
        })?;
    if expected_length != bytes.len() {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_malformed",
            "definition continuation length does not match its canonical envelope",
        ));
    }
    let checksum_start = FUNCTION_DEFINITION_CONTINUATION_HEADER_BYTES + payload_length;
    if bytes[checksum_start..]
        != definition_domain_digest(
            FUNCTION_DEFINITION_CONTINUATION_INTEGRITY_DOMAIN,
            &bytes[..checksum_start],
        )
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_integrity",
            "definition continuation integrity digest does not match its payload",
        ));
    }
    let mut decoder = DefinitionContinuationDecoder::new(
        &bytes[FUNCTION_DEFINITION_CONTINUATION_HEADER_BYTES..checksum_start],
    );
    if decoder.u16()? != FUNCTION_DEFINITION_CONTINUATION_VERSION
        || decoder.u16()? != FUNCTION_DEFINITION_PROJECTION_CONTRACT_VERSION
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_contract",
            "definition continuation uses a foreign continuation or projection version",
        ));
    }
    let repository = RepositoryId::from_bytes(decoder.array_16()?).ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_reserved_identity",
            "definition continuation contains the reserved repository identity",
        )
    })?;
    let package = PackageId::from_bytes(decoder.array_16()?).ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_reserved_identity",
            "definition continuation contains the reserved package identity",
        )
    })?;
    let revision_bytes = decoder.array_32()?;
    if revision_bytes == [0; 32] {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_reserved_identity",
            "definition continuation contains the reserved revision identity",
        ));
    }
    let revision = RevisionId::from_digest(revision_bytes);
    let function = EncodedOwnerKey::decode(&decoder.array_17()?).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_malformed",
            "definition continuation function identity is malformed",
        )
    })?;
    if !matches!(function, KernelOwnerKey::Declaration(_)) {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_mismatch",
            "definition continuation does not identify a function declaration domain",
        ));
    }
    let projection = DefinitionDigest(decoder.array_32()?);
    let ordering = decoder.u8()?;
    let section = DefinitionSection::from_tag(decoder.u8()?).ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_resume_key",
            "definition continuation contains an unknown output section",
        )
    })?;
    let index = decoder.u32()?;
    let resume_key = decoder.array_32()?;
    decoder.finish()?;
    Ok(DecodedDefinitionContinuation {
        binding: DefinitionBinding {
            repository,
            package,
            revision,
            function,
        },
        projection,
        ordering,
        section,
        index,
        resume_key,
    })
}

fn bind_definition_continuation(
    request: &DefinitionPageRequest,
    binding: DefinitionBinding,
) -> Result<Option<DecodedDefinitionContinuation>, Diagnostic> {
    let Some(token) = request.continuation.as_deref() else {
        return Ok(None);
    };
    let continuation = decode_definition_continuation(token)?;
    if continuation.binding.repository != binding.repository
        || continuation.binding.package != binding.package
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_foreign",
            "definition continuation belongs to a foreign repository or package",
        ));
    }
    if continuation.binding.revision != binding.revision {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_stale",
            format!(
                "definition continuation observes revision '{}', but current HEAD is '{}'",
                continuation.binding.revision, binding.revision
            ),
        ));
    }
    if continuation.binding.function != binding.function
        || continuation.ordering != FUNCTION_DEFINITION_ORDERING
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_mismatch",
            "definition continuation does not match the selected function or ordering",
        ));
    }
    Ok(Some(continuation))
}

fn definition_resume_index(
    projection: &DefinitionProjection,
    continuation: Option<&DecodedDefinitionContinuation>,
) -> Result<usize, Diagnostic> {
    let Some(continuation) = continuation else {
        return Ok(0);
    };
    if continuation.projection != projection.digest {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_mismatch",
            "definition continuation projection digest does not match the recomputed definition",
        ));
    }
    let index = usize::try_from(continuation.index).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_resume_key",
            "definition continuation index cannot be represented",
        )
    })?;
    let record = projection.records.get(index).ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_resume_key",
            "definition continuation index is outside the complete projection",
        )
    })?;
    if index.saturating_add(1) >= projection.records.len()
        || record.section != continuation.section
        || record.key != continuation.resume_key
    {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_resume_key",
            "definition continuation exclusive resume key is impossible for this projection",
        ));
    }
    Ok(index + 1)
}

struct DefinitionContinuationDecoder<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> DefinitionContinuationDecoder<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], Diagnostic> {
        let end = self.position.checked_add(length).ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation decoder position overflowed",
            )
        })?;
        let value = self.bytes.get(self.position..end).ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation payload is truncated",
            )
        })?;
        self.position = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, Diagnostic> {
        self.take(1).map(|bytes| bytes[0])
    }

    fn u16(&mut self) -> Result<u16, Diagnostic> {
        self.take(2)
            .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, Diagnostic> {
        self.take(4)
            .map(|bytes| u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn array_16(&mut self) -> Result<[u8; 16], Diagnostic> {
        self.take(16)?.try_into().map_err(|_| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation identity has the wrong width",
            )
        })
    }

    fn array_17(&mut self) -> Result<[u8; 17], Diagnostic> {
        self.take(17)?.try_into().map_err(|_| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation owner has the wrong width",
            )
        })
    }

    fn array_32(&mut self) -> Result<[u8; 32], Diagnostic> {
        self.take(32)?.try_into().map_err(|_| {
            owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_malformed",
                "definition continuation digest has the wrong width",
            )
        })
    }

    fn finish(self) -> Result<(), Diagnostic> {
        if self.position != self.bytes.len() {
            return Err(owner_inspection_error(
                DiagnosticClass::Source,
                "definition_continuation_trailing",
                "definition continuation payload contains trailing bytes",
            ));
        }
        Ok(())
    }
}

fn definition_domain_digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

struct DefinitionMaterializer<'reader, 'view, 'cancel> {
    reader: &'reader mut RepositoryDefinitionReader<'view>,
    package: PackageId,
    cancellation: &'cancel mut dyn FnMut() -> Result<(), Diagnostic>,
    records: Vec<DefinitionLogicalRecord>,
    facts: Vec<DefinitionFactProjection>,
    references: BTreeSet<DefinitionReferenceProjection>,
    seen: BTreeSet<KernelOwnerKey>,
    contract_records: u64,
    body_records: u64,
    structural_edges: u64,
    maximum_depth: u64,
    logical_bytes: usize,
}

impl<'reader, 'view, 'cancel> DefinitionMaterializer<'reader, 'view, 'cancel> {
    fn new(
        reader: &'reader mut RepositoryDefinitionReader<'view>,
        package: PackageId,
        cancellation: &'cancel mut dyn FnMut() -> Result<(), Diagnostic>,
    ) -> Self {
        Self {
            reader,
            package,
            cancellation,
            records: Vec::new(),
            facts: Vec::new(),
            references: BTreeSet::new(),
            seen: BTreeSet::new(),
            contract_records: 0,
            body_records: 0,
            structural_edges: 0,
            maximum_depth: 0,
            logical_bytes: 0,
        }
    }

    fn check(&mut self) -> Result<(), Diagnostic> {
        (self.cancellation)()
    }

    fn push_record(&mut self, record: DefinitionLogicalRecord) -> Result<(), Diagnostic> {
        let next = self
            .logical_bytes
            .checked_add(record.bytes.len())
            .ok_or_else(|| {
                owner_inspection_error(
                    DiagnosticClass::Resource,
                    "definition_logical_byte_limit",
                    "definition logical byte accounting overflowed",
                )
            })?;
        if next > MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES {
            return Err(owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_logical_byte_limit",
                format!(
                    "definition logical encoding exceeds {MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES} bytes"
                ),
            ));
        }
        if record.section == DefinitionSection::Contract {
            self.contract_records = self.contract_records.saturating_add(1);
        }
        self.logical_bytes = next;
        self.records.push(record);
        Ok(())
    }

    fn push_fields(
        &mut self,
        section: DefinitionSection,
        operation: &'static str,
        fields: &[(&'static str, String)],
    ) -> Result<(), Diagnostic> {
        self.check()?;
        self.push_record(definition_record(section, operation, fields)?)
    }

    fn admit_structural_edge(&mut self) -> Result<(), Diagnostic> {
        let next = self.structural_edges.checked_add(1).ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_edge_limit",
                "definition structural edge accounting overflowed",
            )
        })?;
        self.require_edge_total(next, self.references.len() as u64)?;
        self.structural_edges = next;
        Ok(())
    }

    fn require_edge_total(&self, structural: u64, references: u64) -> Result<(), Diagnostic> {
        let total = structural.checked_add(references).ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_edge_limit",
                "definition edge accounting overflowed",
            )
        })?;
        if total > MAXIMUM_FUNCTION_DEFINITION_EDGES {
            return Err(owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_edge_limit",
                format!(
                    "definition contains {total} structural/reference edges, exceeding {MAXIMUM_FUNCTION_DEFINITION_EDGES}"
                ),
            ));
        }
        Ok(())
    }

    fn admit_body_record(&mut self, depth: u64) -> Result<(), Diagnostic> {
        if depth > MAXIMUM_FUNCTION_DEFINITION_DEPTH {
            return Err(owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_depth_limit",
                format!(
                    "definition body depth {depth} exceeds {MAXIMUM_FUNCTION_DEFINITION_DEPTH}"
                ),
            ));
        }
        let next = self.body_records.checked_add(1).ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_body_record_limit",
                "definition body-record accounting overflowed",
            )
        })?;
        if next > MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS {
            return Err(owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_body_record_limit",
                format!(
                    "definition body contains {next} records, exceeding {MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS}"
                ),
            ));
        }
        self.body_records = next;
        self.maximum_depth = self.maximum_depth.max(depth);
        Ok(())
    }

    fn require_fact_count(&self, count: u64) -> Result<(), Diagnostic> {
        if count > MAXIMUM_FUNCTION_DEFINITION_FACT_READS {
            return Err(owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_fact_limit",
                format!(
                    "definition requires {count} bound facts, exceeding {MAXIMUM_FUNCTION_DEFINITION_FACT_READS}"
                ),
            ));
        }
        Ok(())
    }

    fn add_reference(
        &mut self,
        role: impl Into<String>,
        source: KernelOwnerKey,
        ordinal: usize,
        target_kind: impl Into<String>,
        target: impl Into<String>,
    ) -> Result<(), Diagnostic> {
        let ordinal = u32::try_from(ordinal).map_err(|_| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_edge_limit",
                "definition reference ordinal cannot be represented",
            )
        })?;
        let reference = DefinitionReferenceProjection {
            role: role.into(),
            target_kind: target_kind.into(),
            target: target.into(),
            source: local_definition_reference(self.package, source),
            ordinal,
        };
        if !self.references.contains(&reference) {
            let next = (self.references.len() as u64)
                .checked_add(1)
                .ok_or_else(|| {
                    owner_inspection_error(
                        DiagnosticClass::Resource,
                        "definition_edge_limit",
                        "definition reference edge accounting overflowed",
                    )
                })?;
            self.require_edge_total(self.structural_edges, next)?;
            self.references.insert(reference);
        }
        Ok(())
    }

    fn add_local_reference(
        &mut self,
        role: impl Into<String>,
        source: KernelOwnerKey,
        ordinal: usize,
        target: KernelOwnerKey,
    ) -> Result<(), Diagnostic> {
        self.add_reference(
            role,
            source,
            ordinal,
            definition_identity_kind_name(target),
            local_definition_reference(self.package, target),
        )
    }

    fn add_type_reference(
        &mut self,
        role: impl Into<String>,
        source: KernelOwnerKey,
        ordinal: usize,
        target: TypeObjectDigest,
    ) -> Result<(), Diagnostic> {
        self.add_reference(role, source, ordinal, "type", target.to_string())
    }

    fn add_declaration_reference(
        &mut self,
        role: impl Into<String>,
        source: KernelOwnerKey,
        ordinal: usize,
        target: DeclarationReference,
    ) -> Result<(), Diagnostic> {
        self.add_reference(
            role,
            source,
            ordinal,
            "declaration",
            format!("{}/{}", target.package, target.declaration),
        )
    }

    fn add_requirement_reference(
        &mut self,
        role: impl Into<String>,
        source: KernelOwnerKey,
        ordinal: usize,
        target: RequirementReference,
    ) -> Result<(), Diagnostic> {
        self.add_reference(
            role,
            source,
            ordinal,
            "requirement",
            format!("{}/{}", target.package, target.requirement),
        )
    }

    fn add_operation_reference(
        &mut self,
        role: impl Into<String>,
        source: KernelOwnerKey,
        ordinal: usize,
        target: OperationReference,
    ) -> Result<(), Diagnostic> {
        self.add_reference(
            role,
            source,
            ordinal,
            "operation",
            format!("{}/{}", target.package, target.operation),
        )
    }

    fn add_fact(&mut self, owner: KernelOwnerKey, record: &OwnerRecord) -> Result<(), Diagnostic> {
        let next = (self.facts.len() as u64).checked_add(1).ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_fact_limit",
                "definition fact accounting overflowed",
            )
        })?;
        self.require_fact_count(next)?;
        let summary = self.reader.summary(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_summary_missing",
                format!("definition owner '{owner}' has no bound validation summary"),
            )
        })?;
        let (record_digest, _) = encode_owner(record)?;
        if summary.summary.owner != owner
            || summary.summary.kind != record.kind()
            || summary.summary.record != record_digest
        {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_summary_mismatch",
                format!("definition owner '{owner}' disagrees with its bound validation summary"),
            ));
        }
        self.facts.push(DefinitionFactProjection {
            owner,
            kind: record.kind(),
            summary,
        });
        Ok(())
    }

    fn load_structural_owner(
        &mut self,
        owner: KernelOwnerKey,
        expected: OwnershipEntry,
        body_depth: Option<u64>,
    ) -> Result<OwnerRecord, Diagnostic> {
        self.check()?;
        if !self.seen.insert(owner) {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_shared_or_cyclic",
                format!("definition owner '{owner}' is structurally shared or cyclic"),
            ));
        }
        self.admit_structural_edge()?;
        if let Some(depth) = body_depth {
            self.admit_body_record(depth)?;
        }
        let record = self.reader.owner(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_missing",
                format!("structurally owned definition owner '{owner}' is absent"),
            )
        })?;
        if record.owner() != owner {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("definition owner key '{owner}' disagrees with its record"),
            ));
        }
        let ownership = self.reader.ownership(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_missing",
                format!("definition owner '{owner}' has no ownership witness"),
            )
        })?;
        if ownership != expected {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_mismatch",
                format!(
                    "definition owner '{owner}' ownership {ownership:?} does not match {expected:?}"
                ),
            ));
        }
        self.add_fact(owner, &record)?;
        Ok(record)
    }

    fn load_function_root(
        &mut self,
        owner: KernelOwnerKey,
        requested_kind: KernelOwnerKind,
    ) -> Result<OwnerRecord, Diagnostic> {
        self.check()?;
        if !self.seen.insert(owner) {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_shared_or_cyclic",
                "function root identity is duplicated before projection",
            ));
        }
        let record = self.reader.owner(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Semantic,
                "definition_owner_not_found",
                format!("function '{owner}' is not live at the observed revision"),
            )
        })?;
        if record.owner() != owner || record.kind() != requested_kind {
            return Err(owner_inspection_error(
                DiagnosticClass::Semantic,
                "definition_owner_kind",
                format!(
                    "owner '{owner}' has kind '{}', not requested function kind '{}'",
                    record.kind().name(),
                    requested_kind.name()
                ),
            ));
        }
        let OwnerRecord::Declaration(declaration) = &record else {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                "function identity does not bind a declaration record",
            ));
        };
        if !matches!(declaration.payload, DeclarationPayload::Function(_)) {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_body_missing",
                "function declaration has no canonical function payload",
            ));
        }
        self.admit_structural_edge()?;
        let expected = OwnershipEntry::new(
            OwnershipParent::Owner(KernelOwnerKey::Module(declaration.module)),
            OwnershipRole::ModuleDeclaration,
        );
        let ownership = self.reader.ownership(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_missing",
                "function declaration has no ownership witness",
            )
        })?;
        if ownership != expected {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_mismatch",
                "function declaration does not have its canonical module parent",
            ));
        }
        self.add_fact(owner, &record)?;
        Ok(record)
    }

    fn load_local_requirement_if_owned(
        &mut self,
        owner: KernelOwnerKey,
        function: KernelOwnerKey,
    ) -> Result<Option<OwnerRecord>, Diagnostic> {
        self.check()?;
        let ownership = self.reader.ownership(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_missing",
                format!("local requirement '{owner}' has no ownership witness"),
            )
        })?;
        let expected = OwnershipEntry::new(
            OwnershipParent::Owner(function),
            OwnershipRole::DeclarationRequirement,
        );
        if ownership != expected {
            return Ok(None);
        }
        if !self.seen.insert(owner) {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_shared_or_cyclic",
                format!("function-owned requirement '{owner}' is duplicated"),
            ));
        }
        self.admit_structural_edge()?;
        let record = self.reader.owner(owner)?.ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_missing",
                format!("function-owned requirement '{owner}' is absent"),
            )
        })?;
        if record.owner() != owner {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("requirement key '{owner}' disagrees with its record"),
            ));
        }
        self.add_fact(owner, &record)?;
        Ok(Some(record))
    }

    fn visit_expression(
        &mut self,
        expression: super::semantic_id::ExpressionId,
        position: DefinitionPosition,
    ) -> Result<(), Diagnostic> {
        let owner = KernelOwnerKey::Expression(expression);
        let record = self.load_structural_owner(
            owner,
            OwnershipEntry::new(
                OwnershipParent::Owner(position.parent),
                position.ownership_role,
            ),
            Some(position.depth),
        )?;
        let OwnerRecord::Expression(record) = record else {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("body expression '{owner}' has the wrong owner record"),
            ));
        };
        if record.id != expression {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("body expression '{owner}' disagrees with its record identity"),
            ));
        }
        let mut fields = vec![
            ("id", owner.to_string()),
            ("parent", position.parent.to_string()),
            ("slot", position.slot.to_owned()),
            ("index", position.index.to_string()),
        ];
        if let Some(label) = &position.label {
            fields.push(("label", label.clone()));
        }
        fields.push(("depth", position.depth.to_string()));
        let mut literal_fragments = None;
        match &record.operation {
            ExpressionOperation::Unit {} => fields.push(("form", "unit".to_owned())),
            ExpressionOperation::Bool { value } => {
                fields.push(("form", "bool".to_owned()));
                fields.push(("value", value.to_string()));
            }
            ExpressionOperation::I64 { value } => {
                fields.push(("form", "i64".to_owned()));
                fields.push(("value", value.to_string()));
            }
            ExpressionOperation::Text { value } | ExpressionOperation::StaticText { value } => {
                fields.push((
                    "form",
                    if matches!(&record.operation, ExpressionOperation::Text { .. }) {
                        "text"
                    } else {
                        "static_text"
                    }
                    .to_owned(),
                ));
                match value {
                    TextValue::Inline { text } => {
                        let fragments = definition_text_fragments(text);
                        fields.push(("text-storage", "inline".to_owned()));
                        fields.push(("text-bytes", text.len().to_string()));
                        fields.push(("text-fragments", fragments.len().to_string()));
                        literal_fragments = Some(fragments);
                    }
                    TextValue::Blob { digest, bytes } => {
                        fields.push(("text-storage", "blob".to_owned()));
                        fields.push(("text-bytes", bytes.to_string()));
                        fields.push(("text-fragments", "0".to_owned()));
                        fields.push(("blob", digest.to_string()));
                        self.add_reference("text_blob", owner, 0, "blob", digest.to_string())?;
                    }
                }
            }
            ExpressionOperation::Local { value } => {
                fields.push(("form", "local".to_owned()));
                let target = definition_local_value_owner(*value);
                let exact = local_definition_reference(self.package, target);
                fields.push(("value", exact.clone()));
                self.add_reference(
                    "local_value",
                    owner,
                    0,
                    definition_local_value_kind(*value),
                    exact,
                )?;
            }
            ExpressionOperation::Constant { declaration } => {
                fields.push(("form", "constant".to_owned()));
                let reference = format!("{}/{}", declaration.package, declaration.declaration);
                fields.push(("value", reference));
                self.add_declaration_reference("constant_declaration", owner, 0, *declaration)?;
            }
            ExpressionOperation::If { .. } => fields.push(("form", "if".to_owned())),
            ExpressionOperation::Let { bindings, .. } => {
                fields.push(("form", "let".to_owned()));
                fields.push(("bindings", bindings.len().to_string()));
            }
            ExpressionOperation::Sequence { items } => {
                fields.push(("form", "sequence".to_owned()));
                fields.push(("items", items.len().to_string()));
            }
            ExpressionOperation::Call {
                function,
                type_arguments,
                arguments,
            } => {
                fields.push(("form", "call".to_owned()));
                fields.push((
                    "function",
                    format!("{}/{}", function.package, function.declaration),
                ));
                fields.push(("type-arguments", type_arguments.len().to_string()));
                fields.push(("arguments", arguments.len().to_string()));
                self.add_declaration_reference("call_function", owner, 0, *function)?;
                for (index, argument) in type_arguments.iter().copied().enumerate() {
                    self.add_type_reference("call_type_argument", owner, index, argument)?;
                }
            }
            ExpressionOperation::FunctionValue {
                function,
                type_arguments,
            } => {
                fields.push(("form", "function_value".to_owned()));
                fields.push((
                    "function",
                    format!("{}/{}", function.package, function.declaration),
                ));
                fields.push(("type-arguments", type_arguments.len().to_string()));
                self.add_declaration_reference("function_value", owner, 0, *function)?;
                for (index, argument) in type_arguments.iter().copied().enumerate() {
                    self.add_type_reference(
                        "function_value_type_argument",
                        owner,
                        index,
                        argument,
                    )?;
                }
            }
            ExpressionOperation::Invoke { arguments, .. } => {
                fields.push(("form", "invoke".to_owned()));
                fields.push(("arguments", arguments.len().to_string()));
            }
            ExpressionOperation::Record {
                nominal_type,
                fields: values,
            } => {
                fields.push(("form", "record".to_owned()));
                fields.push(("fields", values.len().to_string()));
                if let Some(nominal) = nominal_type {
                    fields.push((
                        "nominal-type",
                        format!("{}/{}", nominal.package, nominal.declaration),
                    ));
                    self.add_declaration_reference("record_nominal_type", owner, 0, *nominal)?;
                } else {
                    fields.push(("nominal-type", "structural".to_owned()));
                }
                for (index, field) in values.iter().enumerate() {
                    if let FieldSelector::Nominal(reference) = &field.selector {
                        self.add_reference(
                            "record_field",
                            owner,
                            index,
                            "field",
                            format!("{}/{}", reference.package, reference.field),
                        )?;
                    }
                }
            }
            ExpressionOperation::Variant { case, payload } => {
                fields.push(("form", "variant".to_owned()));
                fields.push(("case", format!("{}/{}", case.package, case.case)));
                fields.push(("payload", payload.is_some().to_string()));
                self.add_reference(
                    "variant_case",
                    owner,
                    0,
                    "case",
                    format!("{}/{}", case.package, case.case),
                )?;
            }
            ExpressionOperation::Field { selector, .. } => {
                fields.push(("form", "field".to_owned()));
                match selector {
                    FieldSelector::Nominal(reference) => {
                        fields.push(("selector-kind", "nominal".to_owned()));
                        fields.push((
                            "selector",
                            format!("{}/{}", reference.package, reference.field),
                        ));
                        self.add_reference(
                            "field_selector",
                            owner,
                            0,
                            "field",
                            format!("{}/{}", reference.package, reference.field),
                        )?;
                    }
                    FieldSelector::Structural(name) => {
                        fields.push(("selector-kind", "structural".to_owned()));
                        fields.push(("selector", name.as_str().to_owned()));
                    }
                }
            }
            ExpressionOperation::List { item_type, items } => {
                fields.push(("form", "list".to_owned()));
                fields.push(("item-type", item_type.to_string()));
                fields.push(("items", items.len().to_string()));
                self.add_type_reference("list_item_type", owner, 0, *item_type)?;
            }
            ExpressionOperation::Map {
                key_type,
                value_type,
                entries,
            } => {
                fields.push(("form", "map".to_owned()));
                fields.push(("key-type", key_type.to_string()));
                fields.push(("value-type", value_type.to_string()));
                fields.push(("entries", entries.len().to_string()));
                self.add_type_reference("map_key_type", owner, 0, *key_type)?;
                self.add_type_reference("map_value_type", owner, 0, *value_type)?;
            }
            ExpressionOperation::Match { arms, .. } => {
                fields.push(("form", "match".to_owned()));
                fields.push(("arms", arms.len().to_string()));
                for (index, arm) in arms.iter().enumerate() {
                    self.add_reference(
                        "match_case",
                        owner,
                        index,
                        "case",
                        format!("{}/{}", arm.case.package, arm.case.case),
                    )?;
                }
            }
            ExpressionOperation::CapabilityCall {
                requirement,
                operation,
                arguments,
            } => {
                fields.push(("form", "capability_call".to_owned()));
                fields.push((
                    "requirement",
                    format!("{}/{}", requirement.package, requirement.requirement),
                ));
                fields.push((
                    "operation",
                    format!("{}/{}", operation.package, operation.operation),
                ));
                fields.push(("arguments", arguments.len().to_string()));
                self.add_requirement_reference("capability_requirement", owner, 0, *requirement)?;
                self.add_operation_reference("capability_operation", owner, 0, *operation)?;
            }
            ExpressionOperation::Transaction {
                requirement,
                binding,
                ..
            } => {
                fields.push(("form", "transaction".to_owned()));
                fields.push((
                    "requirement",
                    format!("{}/{}", requirement.package, requirement.requirement),
                ));
                fields.push(("binding", binding.to_string()));
                self.add_requirement_reference("transaction_requirement", owner, 0, *requirement)?;
                self.add_local_reference(
                    "transaction_binding",
                    owner,
                    0,
                    KernelOwnerKey::Binding(*binding),
                )?;
            }
        }
        self.push_fields(DefinitionSection::Body, "definition.expression", &fields)?;
        if let Some(fragments) = literal_fragments {
            for (index, fragment) in fragments.into_iter().enumerate() {
                self.push_fields(
                    DefinitionSection::Body,
                    "definition.literal",
                    &[
                        ("owner", owner.to_string()),
                        ("index", index.to_string()),
                        ("bytes", fragment.len().to_string()),
                        ("value", fragment),
                    ],
                )?;
            }
        }

        let child_depth = definition_child_depth(position.depth)?;
        match record.operation {
            ExpressionOperation::If {
                condition,
                when_true,
                when_false,
            } => {
                self.visit_expression_child(
                    owner,
                    condition,
                    (ExpressionChildRole::Condition, "condition"),
                    0,
                    None,
                    child_depth,
                )?;
                self.visit_expression_child(
                    owner,
                    when_true,
                    (ExpressionChildRole::TrueBranch, "true_branch"),
                    0,
                    None,
                    child_depth,
                )?;
                self.visit_expression_child(
                    owner,
                    when_false,
                    (ExpressionChildRole::FalseBranch, "false_branch"),
                    0,
                    None,
                    child_depth,
                )?;
            }
            ExpressionOperation::Let { bindings, body } => {
                for (index, binding) in bindings.into_iter().enumerate() {
                    self.visit_binding(
                        binding,
                        DefinitionPosition {
                            parent: owner,
                            ownership_role: OwnershipRole::ExpressionBinding {
                                role: BindingContainerRole::Let,
                                ordinal: definition_ordinal(index)?,
                            },
                            slot: "let_binding",
                            index: definition_ordinal(index)?,
                            label: None,
                            depth: child_depth,
                        },
                        BindingKind::Let,
                    )?;
                }
                self.visit_expression_child(
                    owner,
                    body,
                    (ExpressionChildRole::LetBody, "let_body"),
                    0,
                    None,
                    child_depth,
                )?;
            }
            ExpressionOperation::Sequence { items } => {
                for (index, item) in items.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        item,
                        (ExpressionChildRole::SequenceItem, "sequence_item"),
                        index,
                        None,
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Call { arguments, .. } => {
                for (index, argument) in arguments.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        argument,
                        (ExpressionChildRole::CallArgument, "call_argument"),
                        index,
                        None,
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Invoke { callee, arguments } => {
                self.visit_expression_child(
                    owner,
                    callee,
                    (ExpressionChildRole::InvokeCallee, "invoke_callee"),
                    0,
                    None,
                    child_depth,
                )?;
                for (index, argument) in arguments.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        argument,
                        (ExpressionChildRole::InvokeArgument, "invoke_argument"),
                        index,
                        None,
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Record { fields, .. } => {
                for (index, field) in fields.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        field.value,
                        (ExpressionChildRole::RecordField, "record_field"),
                        index,
                        Some(definition_field_selector(&field.selector)),
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Variant {
                case,
                payload: Some(payload),
            } => self.visit_expression_child(
                owner,
                payload,
                (ExpressionChildRole::VariantPayload, "variant_payload"),
                0,
                Some(format!("{}/{}", case.package, case.case)),
                child_depth,
            )?,
            ExpressionOperation::Field { value, selector } => self.visit_expression_child(
                owner,
                value,
                (ExpressionChildRole::FieldValue, "field_value"),
                0,
                Some(definition_field_selector(&selector)),
                child_depth,
            )?,
            ExpressionOperation::List { items, .. } => {
                for (index, item) in items.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        item,
                        (ExpressionChildRole::ListItem, "list_item"),
                        index,
                        None,
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Map { entries, .. } => {
                for (index, entry) in entries.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        entry.key,
                        (ExpressionChildRole::MapKey, "map_key"),
                        index,
                        None,
                        child_depth,
                    )?;
                    self.visit_expression_child(
                        owner,
                        entry.value,
                        (ExpressionChildRole::MapValue, "map_value"),
                        index,
                        None,
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Match { value, arms } => {
                self.visit_expression_child(
                    owner,
                    value,
                    (ExpressionChildRole::MatchValue, "match_value"),
                    0,
                    None,
                    child_depth,
                )?;
                for (index, arm) in arms.into_iter().enumerate() {
                    let label = format!("{}/{}", arm.case.package, arm.case.case);
                    if let Some(binding) = arm.payload_binding {
                        self.visit_binding(
                            binding,
                            DefinitionPosition {
                                parent: owner,
                                ownership_role: OwnershipRole::ExpressionBinding {
                                    role: BindingContainerRole::MatchPayload,
                                    ordinal: definition_ordinal(index)?,
                                },
                                slot: "match_payload",
                                index: definition_ordinal(index)?,
                                label: Some(label.clone()),
                                depth: child_depth,
                            },
                            BindingKind::MatchPayload,
                        )?;
                    }
                    self.visit_expression_child(
                        owner,
                        arm.body,
                        (ExpressionChildRole::MatchArmBody, "match_arm"),
                        index,
                        Some(label),
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::CapabilityCall { arguments, .. } => {
                for (index, argument) in arguments.into_iter().enumerate() {
                    self.visit_expression_child(
                        owner,
                        argument,
                        (
                            ExpressionChildRole::CapabilityArgument,
                            "capability_argument",
                        ),
                        index,
                        None,
                        child_depth,
                    )?;
                }
            }
            ExpressionOperation::Transaction { binding, body, .. } => {
                self.visit_binding(
                    binding,
                    DefinitionPosition {
                        parent: owner,
                        ownership_role: OwnershipRole::ExpressionBinding {
                            role: BindingContainerRole::Transaction,
                            ordinal: 0,
                        },
                        slot: "transaction_binding",
                        index: 0,
                        label: None,
                        depth: child_depth,
                    },
                    BindingKind::Transaction,
                )?;
                self.visit_expression_child(
                    owner,
                    body,
                    (ExpressionChildRole::TransactionBody, "transaction_body"),
                    0,
                    None,
                    child_depth,
                )?;
            }
            ExpressionOperation::Unit {}
            | ExpressionOperation::Bool { .. }
            | ExpressionOperation::I64 { .. }
            | ExpressionOperation::Text { .. }
            | ExpressionOperation::StaticText { .. }
            | ExpressionOperation::Local { .. }
            | ExpressionOperation::Constant { .. }
            | ExpressionOperation::FunctionValue { .. }
            | ExpressionOperation::Variant { payload: None, .. } => {}
        }
        Ok(())
    }

    fn visit_expression_child(
        &mut self,
        parent: KernelOwnerKey,
        expression: super::semantic_id::ExpressionId,
        child: (ExpressionChildRole, &'static str),
        index: usize,
        label: Option<String>,
        depth: u64,
    ) -> Result<(), Diagnostic> {
        let (role, slot) = child;
        let child = KernelOwnerKey::Expression(expression);
        self.add_local_reference("expression_child", parent, index, child)?;
        let index = definition_ordinal(index)?;
        self.visit_expression(
            expression,
            DefinitionPosition {
                parent,
                ownership_role: OwnershipRole::ExpressionChild {
                    role,
                    ordinal: index,
                },
                slot,
                index,
                label,
                depth,
            },
        )
    }

    fn visit_binding(
        &mut self,
        binding: super::semantic_id::BindingId,
        position: DefinitionPosition,
        expected_kind: BindingKind,
    ) -> Result<(), Diagnostic> {
        let owner = KernelOwnerKey::Binding(binding);
        let reference_index = usize::try_from(position.index).map_err(|_| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_edge_limit",
                "definition binding ordinal cannot be represented",
            )
        })?;
        self.add_local_reference(
            "expression_binding",
            position.parent,
            reference_index,
            owner,
        )?;
        let record = self.load_structural_owner(
            owner,
            OwnershipEntry::new(
                OwnershipParent::Owner(position.parent),
                position.ownership_role,
            ),
            Some(position.depth),
        )?;
        let OwnerRecord::Binding(record) = record else {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("body binding '{owner}' has the wrong owner record"),
            ));
        };
        if record.kind != expected_kind {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_mismatch",
                format!("body binding '{owner}' has an unexpected lexical kind"),
            ));
        }
        if let Some(ty) = record.declared_type {
            self.add_type_reference("binding_declared_type", owner, 0, ty)?;
        }
        if let Some(value) = record.value {
            self.add_local_reference("binding_value", owner, 0, KernelOwnerKey::Expression(value))?;
        }
        let mut fields = vec![
            ("id", owner.to_string()),
            ("parent", position.parent.to_string()),
            ("slot", position.slot.to_owned()),
            ("index", position.index.to_string()),
        ];
        if let Some(label) = position.label {
            fields.push(("label", label));
        }
        fields.extend([
            ("depth", position.depth.to_string()),
            ("kind", definition_binding_kind_name(record.kind).to_owned()),
            ("name", record.name.as_str().to_owned()),
            (
                "declared-type",
                record
                    .declared_type
                    .map_or_else(|| "absent".to_owned(), |value| value.to_string()),
            ),
            (
                "value",
                record
                    .value
                    .map_or_else(|| "absent".to_owned(), |value| value.to_string()),
            ),
        ]);
        self.push_fields(DefinitionSection::Body, "definition.binding", &fields)?;
        if let Some(value) = record.value {
            self.visit_expression(
                value,
                DefinitionPosition {
                    parent: owner,
                    ownership_role: OwnershipRole::ExpressionRoot(ExpressionRootRole::BindingValue),
                    slot: "binding_value",
                    index: 0,
                    label: None,
                    depth: definition_child_depth(position.depth)?,
                },
            )?;
        }
        Ok(())
    }

    fn finish(
        mut self,
        validator: String,
        certificate: String,
    ) -> Result<DefinitionProjection, Diagnostic> {
        let references = std::mem::take(&mut self.references);
        let reference_records = references.len() as u64;
        for (index, reference) in references.into_iter().enumerate() {
            self.push_fields(
                DefinitionSection::Reference,
                "definition.reference",
                &[
                    ("index", index.to_string()),
                    ("role", reference.role),
                    ("ordinal", reference.ordinal.to_string()),
                    ("source", reference.source),
                    ("target-kind", reference.target_kind),
                    ("target", reference.target),
                ],
            )?;
        }
        let facts = std::mem::take(&mut self.facts);
        let fact_records = facts.len() as u64;
        for (index, fact) in facts.into_iter().enumerate() {
            let summary_digest = fact.summary.digest.to_string();
            let summary = fact.summary.summary;
            self.push_fields(
                DefinitionSection::Fact,
                "definition.fact",
                &[
                    ("index", index.to_string()),
                    ("owner", fact.owner.to_string()),
                    ("kind", fact.kind.name().to_owned()),
                    ("record", summary.record.to_string()),
                    ("summary", summary_digest),
                    (
                        "semantic-interface",
                        definition_semantic_digest(summary.semantic_interface),
                    ),
                    (
                        "implementation",
                        definition_semantic_digest(summary.implementation),
                    ),
                    ("type", definition_semantic_digest(summary.type_digest)),
                    ("effect", definition_semantic_digest(summary.effect)),
                    ("capability", definition_semantic_digest(summary.capability)),
                    ("relations", definition_semantic_digest(summary.relations)),
                    (
                        "presentation",
                        definition_semantic_digest(summary.presentation),
                    ),
                    (
                        "test",
                        summary
                            .test
                            .map_or_else(|| "absent".to_owned(), definition_semantic_digest),
                    ),
                    (
                        "validation-dependencies",
                        definition_semantic_digest(summary.validation_dependencies),
                    ),
                ],
            )?;
        }
        let (digest, logical_bytes) = definition_logical_digest(&self.records)?;
        if logical_bytes != self.logical_bytes {
            return Err(owner_inspection_error(
                DiagnosticClass::Infrastructure,
                "definition_logical_byte_limit",
                "definition logical byte accounting disagrees with canonical records",
            ));
        }
        Ok(DefinitionProjection {
            records: self.records,
            digest,
            contract_records: self.contract_records,
            body_records: self.body_records,
            reference_records,
            fact_records,
            structural_edges: self.structural_edges,
            reference_edges: reference_records,
            fact_reads: fact_records,
            maximum_depth: self.maximum_depth,
            logical_bytes,
            validator,
            certificate,
            work: self.reader.work(),
        })
    }
}

fn local_definition_reference(package: PackageId, owner: KernelOwnerKey) -> String {
    format!("{package}/{owner}")
}

fn definition_semantic_digest(value: SemanticDigest) -> String {
    format!("semantic_{}", encode_hex(&value.bytes()))
}

fn definition_identity_kind_name(owner: KernelOwnerKey) -> &'static str {
    match owner {
        KernelOwnerKey::Module(_) => "module",
        KernelOwnerKey::Declaration(_) => "declaration",
        KernelOwnerKey::TypeParameter(_) => "type_parameter",
        KernelOwnerKey::Field(_) => "field",
        KernelOwnerKey::Case(_) => "case",
        KernelOwnerKey::Operation(_) => "operation",
        KernelOwnerKey::Parameter(_) => "parameter",
        KernelOwnerKey::Binding(_) => "binding",
        KernelOwnerKey::Expression(_) => "expression",
        KernelOwnerKey::Requirement(_) => "requirement",
        KernelOwnerKey::Port(_) => "port",
        KernelOwnerKey::Target(_) => "target",
        KernelOwnerKey::Documentation(_) => "documentation",
        KernelOwnerKey::Annotation(_) => "annotation",
    }
}

fn materialize_function_definition(
    view: &RepositoryView,
    requested_kind: KernelOwnerKind,
    function_owner: KernelOwnerKey,
    cancellation: &mut dyn FnMut() -> Result<(), Diagnostic>,
) -> Result<DefinitionProjection, Diagnostic> {
    let mut reader = view.definition_reader();
    let mut materializer = DefinitionMaterializer::new(&mut reader, view.package(), cancellation);
    let root = materializer.load_function_root(function_owner, requested_kind)?;
    let OwnerRecord::Declaration(declaration) = root else {
        return Err(owner_inspection_error(
            DiagnosticClass::Corrupt,
            "definition_owner_binding",
            "function root is not a declaration record",
        ));
    };
    let DeclarationPayload::Function(function) = declaration.payload else {
        return Err(owner_inspection_error(
            DiagnosticClass::Corrupt,
            "definition_body_missing",
            "function declaration has no function payload",
        ));
    };
    let KernelOwnerKey::Declaration(function_id) = function_owner else {
        return Err(owner_inspection_error(
            DiagnosticClass::Corrupt,
            "definition_owner_binding",
            "function selector is not in the declaration identity domain",
        ));
    };

    materializer.push_fields(
        DefinitionSection::Header,
        "definition.header",
        &[
            ("repository", view.current().head.repository_id.to_string()),
            ("package", view.package().to_string()),
            ("revision", view.revision().to_string()),
            ("function", function_owner.to_string()),
            (
                "contract",
                FUNCTION_DEFINITION_PROJECTION_CONTRACT_IDENTITY.to_owned(),
            ),
            ("ordering", FUNCTION_DEFINITION_ORDERING_NAME.to_owned()),
        ],
    )?;
    let effect_name = match &function.effect {
        FunctionEffect::Pure => "pure",
        FunctionEffect::Task { .. } => "task",
    };
    let requirements = match &function.effect {
        FunctionEffect::Pure => &[][..],
        FunctionEffect::Task { requirements } => requirements.as_slice(),
    };
    materializer.push_fields(
        DefinitionSection::Contract,
        "definition.function",
        &[
            ("id", function_owner.to_string()),
            ("kind", requested_kind.name().to_owned()),
            (
                "module",
                local_definition_reference(
                    view.package(),
                    KernelOwnerKey::Module(declaration.module),
                ),
            ),
            ("name", declaration.name.as_str().to_owned()),
            (
                "visibility",
                definition_visibility_name(declaration.visibility).to_owned(),
            ),
            (
                "type-parameters",
                function.type_parameters.len().to_string(),
            ),
            ("parameters", function.parameters.len().to_string()),
            ("result", function.result.to_string()),
            ("effect", effect_name.to_owned()),
            ("requirements", requirements.len().to_string()),
            ("body", function.body.to_string()),
        ],
    )?;
    materializer.add_local_reference(
        "function_module",
        function_owner,
        0,
        KernelOwnerKey::Module(declaration.module),
    )?;
    materializer.add_type_reference("function_result", function_owner, 0, function.result)?;
    materializer.add_local_reference(
        "function_body",
        function_owner,
        0,
        KernelOwnerKey::Expression(function.body),
    )?;

    for (index, type_parameter) in function.type_parameters.iter().copied().enumerate() {
        let owner = KernelOwnerKey::TypeParameter(type_parameter);
        materializer.add_local_reference(
            "function_type_parameter",
            function_owner,
            index,
            owner,
        )?;
        let record = materializer.load_structural_owner(
            owner,
            OwnershipEntry::new(
                OwnershipParent::Owner(function_owner),
                OwnershipRole::DeclarationTypeParameter,
            ),
            None,
        )?;
        let OwnerRecord::TypeParameter(record) = record else {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("function type parameter '{owner}' has the wrong owner record"),
            ));
        };
        if KernelOwnerKey::Declaration(record.declaration) != function_owner {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_mismatch",
                format!("function type parameter '{owner}' names another declaration"),
            ));
        }
        materializer.push_fields(
            DefinitionSection::Contract,
            "definition.type-parameter",
            &[
                ("id", owner.to_string()),
                ("parent", function_owner.to_string()),
                ("index", index.to_string()),
                ("name", record.name.as_str().to_owned()),
            ],
        )?;
    }

    for (index, parameter) in function.parameters.iter().copied().enumerate() {
        let owner = KernelOwnerKey::Parameter(parameter);
        materializer.add_local_reference("function_parameter", function_owner, index, owner)?;
        let record = materializer.load_structural_owner(
            owner,
            OwnershipEntry::new(
                OwnershipParent::Owner(function_owner),
                OwnershipRole::DeclarationParameter,
            ),
            None,
        )?;
        let OwnerRecord::Parameter(record) = record else {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("function parameter '{owner}' has the wrong owner record"),
            ));
        };
        if record.parent != ParameterParent::Function(function_id) {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_ownership_mismatch",
                format!("function parameter '{owner}' names another declaration"),
            ));
        }
        materializer.add_type_reference("parameter_type", owner, 0, record.ty)?;
        materializer.push_fields(
            DefinitionSection::Contract,
            "definition.parameter",
            &[
                ("id", owner.to_string()),
                ("parent", function_owner.to_string()),
                ("index", index.to_string()),
                ("name", record.name.as_str().to_owned()),
                ("type", record.ty.to_string()),
                (
                    "use",
                    definition_parameter_use_name(record.use_mode).to_owned(),
                ),
            ],
        )?;
    }

    for (index, requirement) in requirements.iter().copied().enumerate() {
        materializer.add_requirement_reference(
            "function_requirement",
            function_owner,
            index,
            requirement,
        )?;
        if requirement.package != view.package() {
            continue;
        }
        let owner = KernelOwnerKey::Requirement(requirement.requirement);
        let Some(record) = materializer.load_local_requirement_if_owned(owner, function_owner)?
        else {
            continue;
        };
        let OwnerRecord::Requirement(record) = record else {
            return Err(owner_inspection_error(
                DiagnosticClass::Corrupt,
                "definition_owner_binding",
                format!("function requirement '{owner}' has the wrong owner record"),
            ));
        };
        materializer.add_declaration_reference(
            "requirement_interface",
            owner,
            0,
            record.interface,
        )?;
        materializer.push_fields(
            DefinitionSection::Contract,
            "definition.requirement",
            &[
                ("id", owner.to_string()),
                ("parent", function_owner.to_string()),
                ("index", index.to_string()),
                ("name", record.name.as_str().to_owned()),
                (
                    "interface",
                    format!(
                        "{}/{}",
                        record.interface.package, record.interface.declaration
                    ),
                ),
                ("operations", record.operations.len().to_string()),
                ("limits", record.limits.len().to_string()),
            ],
        )?;
        for (operation_index, operation) in record.operations.iter().copied().enumerate() {
            materializer.add_operation_reference(
                "requirement_operation",
                owner,
                operation_index,
                operation,
            )?;
            materializer.push_fields(
                DefinitionSection::Contract,
                "definition.requirement-operation",
                &[
                    ("parent", owner.to_string()),
                    ("index", operation_index.to_string()),
                    (
                        "reference",
                        format!("{}/{}", operation.package, operation.operation),
                    ),
                ],
            )?;
        }
        for (limit_index, limit) in record.limits.iter().enumerate() {
            materializer.push_fields(
                DefinitionSection::Contract,
                "definition.requirement-limit",
                &[
                    ("parent", owner.to_string()),
                    ("index", limit_index.to_string()),
                    ("name", limit.name.as_str().to_owned()),
                    ("maximum", limit.maximum.to_string()),
                    ("unit", definition_resource_unit_name(limit.unit).to_owned()),
                ],
            )?;
        }
    }

    materializer.visit_expression(
        function.body,
        DefinitionPosition {
            parent: function_owner,
            ownership_role: OwnershipRole::ExpressionRoot(ExpressionRootRole::FunctionBody),
            slot: "function_body",
            index: 0,
            label: None,
            depth: 0,
        },
    )?;
    materializer.finish(
        view.current().witness.validator_contract.to_string(),
        view.current().witness.certificate.to_string(),
    )
}

fn definition_visibility_name(value: DeclarationVisibility) -> &'static str {
    match value {
        DeclarationVisibility::Private => "private",
        DeclarationVisibility::Package => "package",
        DeclarationVisibility::Public => "public",
    }
}

fn definition_parameter_use_name(value: ParameterUse) -> &'static str {
    match value {
        ParameterUse::Unrestricted => "unrestricted",
        ParameterUse::Borrow => "borrow",
        ParameterUse::Consume => "consume",
    }
}

fn definition_resource_unit_name(value: ResourceUnit) -> &'static str {
    match value {
        ResourceUnit::Bytes => "bytes",
        ResourceUnit::Items => "items",
        ResourceUnit::Calls => "calls",
        ResourceUnit::Tasks => "tasks",
        ResourceUnit::Milliseconds => "milliseconds",
    }
}

fn definition_binding_kind_name(value: BindingKind) -> &'static str {
    match value {
        BindingKind::Let => "let",
        BindingKind::MatchPayload => "match_payload",
        BindingKind::Transaction => "transaction",
    }
}

fn definition_local_value_owner(value: LocalValueReference) -> KernelOwnerKey {
    match value {
        LocalValueReference::FunctionParameter(value)
        | LocalValueReference::OperationParameter(value) => KernelOwnerKey::Parameter(value),
        LocalValueReference::LexicalBinding(value)
        | LocalValueReference::MatchPayload(value)
        | LocalValueReference::TransactionBinding(value) => KernelOwnerKey::Binding(value),
    }
}

fn definition_local_value_kind(value: LocalValueReference) -> &'static str {
    match value {
        LocalValueReference::FunctionParameter(_) => "function_parameter",
        LocalValueReference::OperationParameter(_) => "operation_parameter",
        LocalValueReference::LexicalBinding(_) => "lexical_binding",
        LocalValueReference::MatchPayload(_) => "match_payload",
        LocalValueReference::TransactionBinding(_) => "transaction_binding",
    }
}

fn definition_field_selector(selector: &FieldSelector) -> String {
    match selector {
        FieldSelector::Nominal(reference) => {
            format!("{}/{}", reference.package, reference.field)
        }
        FieldSelector::Structural(name) => name.as_str().to_owned(),
    }
}

fn definition_child_depth(depth: u64) -> Result<u64, Diagnostic> {
    depth.checked_add(1).ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_depth_limit",
            "definition body depth accounting overflowed",
        )
    })
}

fn definition_ordinal(index: usize) -> Result<u32, Diagnostic> {
    u32::try_from(index).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_edge_limit",
            "definition structural ordinal cannot be represented",
        )
    })
}

fn definition_text_fragments(value: &str) -> Vec<String> {
    if value.is_empty() {
        return vec![String::new()];
    }
    let mut fragments = Vec::new();
    let mut start = 0_usize;
    while start < value.len() {
        let mut end = start
            .saturating_add(MAXIMUM_FUNCTION_DEFINITION_LITERAL_FRAGMENT_BYTES)
            .min(value.len());
        while end > start && !value.is_char_boundary(end) {
            end -= 1;
        }
        if end == start {
            end = value[start..]
                .char_indices()
                .nth(1)
                .map_or(value.len(), |(offset, _)| start.saturating_add(offset));
        }
        fragments.push(value[start..end].to_owned());
        start = end;
    }
    fragments
}

fn execute_function_definition(
    repository: &GraphRepository,
    view: &RepositoryView,
    requested_kind: KernelOwnerKind,
    function: KernelOwnerKey,
    request: &DefinitionPageRequest,
    response_limits: CompactResponseLimits,
    cancellation: &mut dyn FnMut() -> Result<(), Diagnostic>,
) -> Result<Vec<u8>, Diagnostic> {
    cancellation()?;
    let binding = DefinitionBinding {
        repository: view.current().head.repository_id,
        package: view.package(),
        revision: view.revision(),
        function,
    };
    let continuation = bind_definition_continuation(request, binding)?;
    let projection = materialize_function_definition(view, requested_kind, function, cancellation)
        .map_err(classify_definition_read_diagnostic)?;
    let start = definition_resume_index(&projection, continuation.as_ref())?;
    let capabilities = capabilities_snapshot().map_err(|message| {
        owner_inspection_error(
            DiagnosticClass::Infrastructure,
            "capabilities_projection_invalid",
            message,
        )
    })?;
    let output_limit = request
        .output_bytes
        .min(response_limits.maximum_bytes)
        .min(MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES)
        .min(MAXIMUM_CLI_RESPONSE_BYTES);
    let record_limit = response_limits
        .maximum_records
        .min(MAXIMUM_CLI_RESPONSE_RECORDS);
    render_definition_page(
        repository,
        view,
        binding,
        requested_kind,
        &projection,
        start,
        request.items,
        output_limit,
        record_limit,
        &capabilities.digest,
        cancellation,
    )
}

#[allow(clippy::too_many_arguments)]
fn render_definition_page(
    repository: &GraphRepository,
    view: &RepositoryView,
    binding: DefinitionBinding,
    requested_kind: KernelOwnerKind,
    projection: &DefinitionProjection,
    start: usize,
    requested_items: u64,
    output_limit: usize,
    record_limit: usize,
    capabilities_digest: &str,
    cancellation: &mut dyn FnMut() -> Result<(), Diagnostic>,
) -> Result<Vec<u8>, Diagnostic> {
    cancellation()?;
    if start >= projection.records.len() {
        return Err(owner_inspection_error(
            DiagnosticClass::Source,
            "definition_continuation_resume_key",
            "definition page begins outside the complete logical projection",
        ));
    }
    const FIXED_RECORDS_WITH_CONTINUATION: usize = 8;
    let item_record_capacity = record_limit
        .checked_sub(FIXED_RECORDS_WITH_CONTINUATION)
        .ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_output_envelope_too_large",
                "compact response record capacity cannot hold the definition page envelope",
            )
        })?;
    if item_record_capacity == 0 {
        return Err(owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_output_item_too_large",
            "compact response record capacity cannot hold one definition item",
        ));
    }
    let requested_items = usize::try_from(requested_items).map_err(|_| {
        owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_invalid_limit",
            "definition item limit cannot be represented",
        )
    })?;
    let maximum_end = start
        .checked_add(requested_items.min(item_record_capacity))
        .map(|end| end.min(projection.records.len()))
        .ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_invalid_limit",
                "definition page range accounting overflowed",
            )
        })?;
    if maximum_end <= start {
        return Err(owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_output_item_too_large",
            "definition page has no admitted logical record slot",
        ));
    }

    let mut render = |end: usize| {
        cancellation()?;
        render_definition_page_exact(
            repository,
            view,
            binding,
            requested_kind,
            projection,
            start,
            end,
            output_limit,
            record_limit,
            capabilities_digest,
        )
    };
    match render(maximum_end) {
        Ok(bytes) => return Ok(bytes),
        Err(error) if definition_page_budget_error(&error) => {}
        Err(error) => return Err(classify_definition_output_diagnostic(error)),
    }
    let mut lower = start.saturating_add(1);
    let mut upper = maximum_end.saturating_sub(1);
    let mut best = None;
    while lower <= upper {
        let middle = lower + (upper - lower) / 2;
        match render(middle) {
            Ok(bytes) => {
                best = Some(bytes);
                lower = middle.saturating_add(1);
            }
            Err(error) if definition_page_budget_error(&error) => {
                if middle == 0 {
                    break;
                }
                upper = middle - 1;
            }
            Err(error) => return Err(classify_definition_output_diagnostic(error)),
        }
    }
    best.ok_or_else(|| {
        owner_inspection_error(
            DiagnosticClass::Resource,
            "definition_output_item_too_large",
            format!(
                "one definition record plus its revision-pinned envelope cannot fit {output_limit} output bytes"
            ),
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn render_definition_page_exact(
    _repository: &GraphRepository,
    view: &RepositoryView,
    binding: DefinitionBinding,
    requested_kind: KernelOwnerKind,
    projection: &DefinitionProjection,
    start: usize,
    end: usize,
    output_limit: usize,
    record_limit: usize,
    capabilities_digest: &str,
) -> Result<Vec<u8>, Diagnostic> {
    let mut rendered_bytes = 0_usize;
    for _ in 0..4 {
        let bytes = render_definition_page_once(
            view,
            binding,
            requested_kind,
            projection,
            start,
            end,
            output_limit,
            record_limit,
            capabilities_digest,
            rendered_bytes,
        )?;
        if bytes.len() == rendered_bytes {
            return Ok(bytes);
        }
        rendered_bytes = bytes.len();
    }
    Err(owner_inspection_error(
        DiagnosticClass::Infrastructure,
        "definition_output_size_convergence",
        "compact definition response byte count did not converge",
    ))
}

#[allow(clippy::too_many_arguments)]
fn render_definition_page_once(
    view: &RepositoryView,
    binding: DefinitionBinding,
    requested_kind: KernelOwnerKind,
    projection: &DefinitionProjection,
    start: usize,
    end: usize,
    output_limit: usize,
    record_limit: usize,
    capabilities_digest: &str,
    rendered_bytes: usize,
) -> Result<Vec<u8>, Diagnostic> {
    if end <= start || end > projection.records.len() {
        return Err(owner_inspection_error(
            DiagnosticClass::Infrastructure,
            "definition_continuation_resume_key",
            "definition renderer received an invalid logical page range",
        ));
    }
    let complete = end == projection.records.len();
    let continuation = if complete {
        None
    } else {
        let index = end - 1;
        Some(encode_definition_continuation(
            binding,
            projection.digest,
            index,
            &projection.records[index],
        )?)
    };
    let rendered_records = 7_usize
        .checked_add(end - start)
        .and_then(|records| records.checked_add(usize::from(continuation.is_some())))
        .ok_or_else(|| {
            owner_inspection_error(
                DiagnosticClass::Resource,
                "definition_output_envelope_too_large",
                "definition response record accounting overflowed",
            )
        })?;
    let mut writer = CompactResponseWriter::new(CompactResponseLimits {
        maximum_bytes: output_limit,
        maximum_records: record_limit,
    })?;
    append_definition_fields(
        &mut writer,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "inspect.owner.definition".to_owned()),
        ],
    )?;
    append_definition_fields(
        &mut writer,
        "project",
        &[
            (
                "name",
                view.current()
                    .semantic_root
                    .package_name
                    .as_str()
                    .to_owned(),
            ),
            ("repository", binding.repository.to_string()),
            ("package", binding.package.to_string()),
        ],
    )?;
    append_definition_fields(
        &mut writer,
        "revision",
        &[("observed", binding.revision.to_string())],
    )?;
    append_definition_fields(
        &mut writer,
        "projection",
        &[
            ("detail", "definition".to_owned()),
            (
                "contract",
                FUNCTION_DEFINITION_PROJECTION_CONTRACT_IDENTITY.to_owned(),
            ),
            (
                "version",
                FUNCTION_DEFINITION_PROJECTION_CONTRACT_VERSION.to_string(),
            ),
            ("function", binding.function.to_string()),
            ("kind", requested_kind.name().to_owned()),
            ("digest", projection.digest.to_string()),
            ("ordering", FUNCTION_DEFINITION_ORDERING_NAME.to_owned()),
            ("total-records", projection.records.len().to_string()),
            ("contract-records", projection.contract_records.to_string()),
            ("body-records", projection.body_records.to_string()),
            (
                "reference-records",
                projection.reference_records.to_string(),
            ),
            ("fact-records", projection.fact_records.to_string()),
            ("structural-edges", projection.structural_edges.to_string()),
            ("reference-edges", projection.reference_edges.to_string()),
            ("fact-reads", projection.fact_reads.to_string()),
            ("maximum-depth", projection.maximum_depth.to_string()),
            ("logical-bytes", projection.logical_bytes.to_string()),
            ("validator", projection.validator.clone()),
            ("certificate", projection.certificate.clone()),
        ],
    )?;
    append_definition_fields(
        &mut writer,
        "page",
        &[
            ("start", start.to_string()),
            ("end", end.to_string()),
            ("returned", (end - start).to_string()),
            ("complete", complete.to_string()),
            (
                "first-section",
                projection.records[start].section.name().to_owned(),
            ),
            (
                "last-section",
                projection.records[end - 1].section.name().to_owned(),
            ),
        ],
    )?;
    for record in &projection.records[start..end] {
        writer.append_serialized_records(&record.bytes)?;
    }
    if let Some(token) = continuation {
        append_definition_fields(&mut writer, "continuation", &[("token", token)])?;
    }
    append_definition_fields(
        &mut writer,
        "work",
        &[
            ("map-pages-read", projection.work.map.pages_read.to_string()),
            ("map-bytes-read", projection.work.map.bytes_read.to_string()),
            (
                "map-entries-visited",
                projection.work.map.entries_visited.to_string(),
            ),
            (
                "catalog-lookups",
                projection.work.store.catalog_lookups.to_string(),
            ),
            (
                "store-objects-read",
                projection.work.store.objects_read.to_string(),
            ),
            (
                "store-bytes-read",
                projection.work.store.bytes_read.to_string(),
            ),
            (
                "canonical-records-decoded",
                projection.work.canonical_records_decoded.to_string(),
            ),
            (
                "witness-records-decoded",
                projection.work.witness_records_decoded.to_string(),
            ),
            ("fact-reads", projection.fact_reads.to_string()),
            ("rendered-records", rendered_records.to_string()),
            ("rendered-output-bytes", rendered_bytes.to_string()),
        ],
    )?;
    append_definition_fields(
        &mut writer,
        "schema",
        &[("capabilities", capabilities_digest.to_owned())],
    )?;
    Ok(writer.finish())
}

fn append_definition_fields(
    writer: &mut CompactResponseWriter,
    operation: &'static str,
    fields: &[(&'static str, String)],
) -> Result<(), Diagnostic> {
    for (name, _) in fields {
        if !FUNCTION_DEFINITION_RESPONSE_FIELDS.contains(&(operation, *name)) {
            return Err(owner_inspection_error(
                DiagnosticClass::Infrastructure,
                "definition_response_field_inventory",
                format!(
                    "definition response field '{operation}.{name}' is absent from capabilities"
                ),
            ));
        }
    }
    let borrowed = fields
        .iter()
        .map(|(name, value)| (*name, value.as_str()))
        .collect::<Vec<_>>();
    writer.append_record(operation, &borrowed)
}

fn definition_page_budget_error(error: &Diagnostic) -> bool {
    matches!(
        error.code.as_str(),
        "control_response_byte_budget" | "control_response_record_budget"
    )
}

fn classify_definition_output_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    let replacement = match diagnostic.code.as_str() {
        "control_response_byte_budget" | "control_response_record_budget" => {
            Some("definition_output_envelope_too_large")
        }
        "control_render_record_bytes" | "control_render_value_bytes" => {
            Some("definition_output_item_too_large")
        }
        _ => None,
    };
    if let Some(code) = replacement {
        diagnostic.code = code.to_owned();
    }
    diagnostic
}

fn classify_definition_read_diagnostic(mut diagnostic: Diagnostic) -> Diagnostic {
    let replacement = match diagnostic.code.as_str() {
        "persistent_map_admission_pages_read" => Some("definition_admission_map_pages"),
        "persistent_map_admission_bytes_read" => Some("definition_admission_map_bytes"),
        "persistent_map_admission_entries_visited" => Some("definition_admission_map_entries"),
        "object_read_catalog_lookups_exhausted" => Some("definition_admission_catalog_lookups"),
        "object_read_objects_exhausted" => Some("definition_admission_store_objects"),
        "object_read_bytes_exhausted" => Some("definition_admission_store_bytes"),
        "persistent_map_page_missing" => Some("definition_required_map_page_missing"),
        "publication_read_object_missing" => Some("definition_required_object_missing"),
        "control_render_record_bytes" | "control_render_value_bytes" => {
            Some("definition_output_item_too_large")
        }
        _ => None,
    };
    if let Some(code) = replacement {
        diagnostic.code = code.to_owned();
    }
    diagnostic
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
    let snapshot = capabilities_snapshot().map_err(capabilities_projection_error)?;
    let command = arguments.first().filter(|value| !value.starts_with("--"));
    if let Some(command) = command {
        exact_arguments(arguments, 1, "capabilities COMMAND")?;
        let operation = PublicOperation::parse(command)
            .ok_or_else(|| usage_error(format!("unknown public command '{command}'")))?;
        let descriptor = operation_descriptors()
            .iter()
            .find(|descriptor| descriptor.operation == operation)
            .ok_or_else(|| internal_error("registered public operation has no descriptor"))?;
        let mut output = capabilities_response_writer(&snapshot, None)?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.command".to_owned()),
            ],
        )?;
        let record = operation_record(descriptor).map_err(capabilities_projection_error)?;
        output.append_serialized_records(record.as_bytes())?;
        let focused_section = match operation {
            PublicOperation::New => Some(RegistrySection::Templates),
            PublicOperation::Inspect => Some(RegistrySection::Inspection),
            PublicOperation::Query => Some(RegistrySection::Query),
            _ => None,
        };
        if let Some(section_name) = focused_section {
            let section = snapshot.section(section_name).ok_or_else(|| {
                internal_error("registered operation has no focused capability section")
            })?;
            output.append_serialized_records(&section.bytes)?;
        }
        return Ok(output.finish());
    }

    ensure_options(
        arguments,
        &[
            "--known-capabilities",
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
            RegistrySection::parse_public(&value)
                .ok_or_else(|| usage_error(format!("unknown capability section '{value}'")))
        })
        .transpose()?;
    let known_capabilities = option_value(arguments, "--known-capabilities")?;
    if let Some(digest) = &known_capabilities {
        validate_capability_digest(digest, "--known-capabilities")?;
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
        && (selected_section.is_some()
            || !known_sections.is_empty()
            || known_capabilities.is_some())
    {
        return Err(usage_error(
            "generated-document operations do not accept capability selection or digest options",
        ));
    }

    if let Some(directory) = generated_directory {
        let documents = generated_documents().map_err(capabilities_projection_error)?;
        let directory = PathBuf::from(directory);
        let mut output = capabilities_response_writer(&snapshot, None)?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.generate-docs".to_owned()),
            ],
        )?;
        for document in documents {
            ensure_capability_export_bound(&document.bytes)?;
            let path = directory.join(document.relative_path);
            let status = write_derived_output(
                &path,
                &document.bytes,
                MAXIMUM_CLI_RESPONSE_BYTES,
                "generated public guide",
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
        let documents = generated_documents().map_err(capabilities_projection_error)?;
        let directory = PathBuf::from(directory);
        let mut output = capabilities_response_writer(&snapshot, None)?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.verify-generated".to_owned()),
            ],
        )?;
        for document in &documents {
            let path = directory.join(document.relative_path);
            let observed =
                read_bounded(&path, MAXIMUM_CLI_RESPONSE_BYTES, "generated public guide")?;
            if observed != document.bytes {
                return Err(Diagnostic::new(
                    DiagnosticClass::Source,
                    "capabilities_generated_drift",
                    format!(
                        "generated public guide '{}' is stale; run 'lkjscript capabilities --generate-docs {}'",
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
                "capabilities",
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
            "compact capabilities output",
        )?;
        let mut output = capabilities_response_writer(&snapshot, None)?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.output".to_owned()),
            ],
        )?;
        append_file_record(&mut output, kind, &path, bytes, digest, status)?;
        return Ok(output.finish());
    }

    if let Some(section) = selected_section {
        let mut output = capabilities_response_writer(&snapshot, None)?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.section".to_owned()),
            ],
        )?;
        append_section(&mut output, &snapshot, section, true, None)?;
        return Ok(output.finish());
    }

    if !known_sections.is_empty() {
        let unchanged = known_sections.iter().all(|(section, known)| {
            snapshot
                .section(*section)
                .is_some_and(|current| current.digest == *known)
        });
        let mut output = capabilities_response_writer(&snapshot, Some(unchanged))?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities.changed-sections".to_owned()),
                ("unchanged", unchanged.to_string()),
            ],
        )?;
        for (section, known) in known_sections {
            let changed = snapshot
                .section(section)
                .is_some_and(|current| current.digest != known);
            append_section(&mut output, &snapshot, section, changed, Some(changed))?;
        }
        return Ok(output.finish());
    }

    if known_capabilities.as_deref() == Some(snapshot.digest.as_str()) {
        let mut output = capabilities_response_writer(&snapshot, Some(true))?;
        append_capability_record(
            &mut output,
            "result",
            &[
                ("status", "success".to_owned()),
                ("command", "capabilities".to_owned()),
            ],
        )?;
        return Ok(output.finish());
    }

    let mut output = capabilities_response_writer(&snapshot, Some(false))?;
    append_capability_record(
        &mut output,
        "result",
        &[
            ("status", "success".to_owned()),
            ("command", "capabilities".to_owned()),
        ],
    )?;
    append_capability_record(
        &mut output,
        "summary",
        &[
            ("operations", operation_descriptors().len().to_string()),
            ("sections", RegistrySection::PUBLIC.len().to_string()),
        ],
    )?;
    for section in RegistrySection::PUBLIC {
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

fn capabilities_response_writer(
    snapshot: &CapabilitiesSnapshot,
    unchanged: Option<bool>,
) -> Result<CompactResponseWriter, Diagnostic> {
    let mut output = compact_response_writer()?;
    append_capability_record(
        &mut output,
        "product",
        &[
            ("name", snapshot.product_name.to_owned()),
            ("version", snapshot.product_version.to_owned()),
        ],
    )?;
    let mut fields = vec![("digest", snapshot.digest.clone())];
    if let Some(unchanged) = unchanged {
        fields.push(("unchanged", unchanged.to_string()));
    }
    append_capability_record(&mut output, "capabilities", &fields)?;
    Ok(output)
}

fn append_section(
    output: &mut CompactResponseWriter,
    capabilities: &CapabilitiesSnapshot,
    section: RegistrySection,
    include_records: bool,
    changed: Option<bool>,
) -> Result<(), Diagnostic> {
    let snapshot = capabilities
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

fn append_dynamic_record(
    output: &mut CompactResponseWriter,
    operation: &str,
    fields: &[(String, String)],
) -> Result<(), Diagnostic> {
    let borrowed = fields
        .iter()
        .map(|(name, value)| (name.as_str(), value.as_str()))
        .collect::<Vec<_>>();
    output.append_record(operation, &borrowed)
}

fn parse_usize_option(
    value: Option<String>,
    option: &str,
    default: usize,
) -> Result<usize, Diagnostic> {
    value
        .map(|value| {
            value.parse::<usize>().map_err(|_| {
                usage_error(format!(
                    "{option} requires a canonical nonnegative decimal integer"
                ))
            })
        })
        .transpose()
        .map(|value| value.unwrap_or(default))
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
            "capabilities_output_budget",
            format!(
                "compact capabilities output exceeds the hard {MAXIMUM_CLI_RESPONSE_BYTES}-byte bound"
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
        let section = RegistrySection::parse_public(name)
            .ok_or_else(|| usage_error(format!("unknown capability section '{name}'")))?;
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

fn capabilities_projection_error(message: String) -> Diagnostic {
    Diagnostic::new(
        DiagnosticClass::Corrupt,
        "capabilities_projection_invalid",
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
        let registry = crate::platform::contract::registry_snapshot().expect("compact registry");
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
    fn maintained_affine_worker_definition_pages_are_complete_stateless_and_read_only() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let head_path = project.join("HEAD");
        let catalog_path = project.join("catalog/current.lkjc");
        let generated_path = project.join("generated/lkjournal.lkja");
        let before_head = std::fs::read(&head_path).expect("lkjournal HEAD before projection");
        let before_catalog = std::fs::read(&catalog_path).expect("catalog before projection");
        let before_generated =
            std::fs::read(&generated_path).expect("generated application before projection");
        let function = "decl_a914bb78de075ff44a857ac028d704f3";
        let base = vec![
            "--project".to_owned(),
            project.display().to_string(),
            "inspect".to_owned(),
            "owner".to_owned(),
            "task_function".to_owned(),
            function.to_owned(),
            "--detail".to_owned(),
            "definition".to_owned(),
        ];
        let mut complete_arguments = base.clone();
        complete_arguments.extend([
            "--limit".to_owned(),
            "10000".to_owned(),
            "--bytes".to_owned(),
            MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES.to_string(),
        ]);
        let complete =
            execute_inspect_owner(complete_arguments).expect("complete worker projection");
        let complete_records = parse_records("complete-definition", &complete)
            .expect("complete definition compact records");
        assert_eq!(
            response_field(&complete_records, "page", "complete"),
            "true"
        );
        let digest = response_field(&complete_records, "projection", "digest").to_owned();
        assert!(digest.starts_with("definition_"));
        assert_eq!(
            response_field(&complete_records, "projection", "body-records"),
            "48"
        );
        assert_eq!(
            response_field(&complete_records, "projection", "fact-records"),
            "51"
        );
        let project_record = complete_records
            .iter()
            .find(|record| record.operation == "project")
            .expect("project envelope");
        assert!(
            project_record
                .fields
                .iter()
                .all(|field| field.name != "path")
        );
        assert!(complete_records.iter().any(|record| {
            record.operation == "definition.binding"
                && record
                    .fields
                    .iter()
                    .any(|field| field.name == "name" && field.value == "lease-info")
        }));
        assert!(complete_records.iter().any(|record| {
            record.operation == "definition.binding"
                && record
                    .fields
                    .iter()
                    .any(|field| field.name == "name" && field.value == "renewed-lease")
        }));
        for operation in [
            "op_23bc0c498113c09a2ff0a4cf9c0a37ab",
            "op_1a5491eb1c3ef3d15ec28268b6f04afc",
            "op_f593ba236055aa1afa6c02eaf0db6a64",
            "op_679b43bb7dc0b298a7706d4e8a7bef23",
            "op_242e065f9738b454e2328ed0e558e6a0",
        ] {
            assert!(complete_records.iter().any(|record| {
                record.operation == "definition.reference"
                    && record
                        .fields
                        .iter()
                        .any(|field| field.name == "target" && field.value.ends_with(operation))
            }));
        }
        let complete_items = complete_records
            .iter()
            .filter(|record| record.operation.starts_with("definition."))
            .map(compact_record_identity)
            .collect::<Vec<_>>();

        let mut paged_items = Vec::new();
        let mut continuation: Option<String> = None;
        for page in 0..100_usize {
            let mut arguments = base.clone();
            arguments.extend([
                "--limit".to_owned(),
                if page % 2 == 0 { "7" } else { "11" }.to_owned(),
                "--bytes".to_owned(),
                if page % 3 == 0 { "8192" } else { "16384" }.to_owned(),
            ]);
            if let Some(token) = &continuation {
                arguments.extend(["--continuation".to_owned(), token.clone()]);
            }
            let bytes = execute_inspect_owner(arguments).expect("worker definition page");
            let records =
                parse_records("definition-page", &bytes).expect("definition page records");
            assert_eq!(response_field(&records, "projection", "digest"), digest);
            paged_items.extend(
                records
                    .iter()
                    .filter(|record| record.operation.starts_with("definition."))
                    .map(compact_record_identity),
            );
            if response_field(&records, "page", "complete") == "true" {
                continuation = None;
                break;
            }
            continuation = Some(response_field(&records, "continuation", "token").to_owned());
        }
        assert!(
            continuation.is_none(),
            "definition pagination did not finish"
        );
        assert_eq!(paged_items, complete_items);
        assert_eq!(
            std::fs::read(&head_path).expect("HEAD after projection"),
            before_head
        );
        assert_eq!(
            std::fs::read(&catalog_path).expect("catalog after projection"),
            before_catalog
        );
        assert_eq!(
            std::fs::read(&generated_path).expect("generated app after projection"),
            before_generated
        );
    }

    #[test]
    fn definition_detail_rejects_aliases_projection_input_and_cancellation_without_writes() {
        for (arguments, code) in [
            (
                vec![
                    "inspect".to_owned(),
                    "owner".to_owned(),
                    "task_function".to_owned(),
                    "decl_a914bb78de075ff44a857ac028d704f3".to_owned(),
                    "--detail".to_owned(),
                    "body".to_owned(),
                ],
                "definition_detail_value",
            ),
            (
                vec![
                    "inspect".to_owned(),
                    "owner".to_owned(),
                    "task_function".to_owned(),
                    "decl_a914bb78de075ff44a857ac028d704f3".to_owned(),
                    "--limit".to_owned(),
                    "1".to_owned(),
                ],
                "definition_detail_required",
            ),
            (
                vec![
                    "inspect".to_owned(),
                    "owner".to_owned(),
                    "task_function".to_owned(),
                    "decl_a914bb78de075ff44a857ac028d704f3".to_owned(),
                    "--raw".to_owned(),
                ],
                "cli_usage",
            ),
        ] {
            let error = execute_inspect_owner(arguments).expect_err("definition alias rejection");
            assert_eq!(error.code, code);
        }
        let projection_input = b"request base=rev_0000000000000000000000000000000000000000000000000000000000000001\ndefinition.expression id=expr_00000000000000000000000000000001\n";
        assert!(decode_compact_change("projection-input", projection_input).is_err());

        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let before = std::fs::read(project.join("HEAD")).expect("HEAD before cancellation");
        let repository = GraphRepository::open(&project).expect("lkjournal repository");
        let view = repository.view_current().expect("lkjournal view");
        let owner = "decl_a914bb78de075ff44a857ac028d704f3"
            .parse()
            .expect("worker identity");
        let mut checks = 0_u64;
        let mut cancellation = || {
            checks = checks.saturating_add(1);
            if checks == 6 {
                Err(owner_inspection_error(
                    DiagnosticClass::Cancelled,
                    "definition_cancelled",
                    "injected definition cancellation",
                ))
            } else {
                Ok(())
            }
        };
        let error = materialize_function_definition(
            &view,
            KernelOwnerKind::TaskFunction,
            owner,
            &mut cancellation,
        )
        .expect_err("injected definition cancellation");
        assert_eq!(error.code, "definition_cancelled");
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("HEAD after cancellation"),
            before
        );

        let mut materialization_control = || Ok(());
        let projection = materialize_function_definition(
            &view,
            KernelOwnerKind::TaskFunction,
            owner,
            &mut materialization_control,
        )
        .expect("definition before render cancellation");
        let binding = DefinitionBinding {
            repository: view.current().head.repository_id,
            package: view.package(),
            revision: view.revision(),
            function: owner,
        };
        let capabilities = capabilities_snapshot().expect("capabilities");
        let mut render_checks = 0_u64;
        let mut render_cancellation = || {
            render_checks = render_checks.saturating_add(1);
            if render_checks == 2 {
                Err(owner_inspection_error(
                    DiagnosticClass::Cancelled,
                    "definition_cancelled",
                    "injected definition render cancellation",
                ))
            } else {
                Ok(())
            }
        };
        let error = render_definition_page(
            &repository,
            &view,
            binding,
            KernelOwnerKind::TaskFunction,
            &projection,
            0,
            1,
            MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES,
            MAXIMUM_CLI_RESPONSE_RECORDS,
            &capabilities.digest,
            &mut render_cancellation,
        )
        .expect_err("injected definition render cancellation");
        assert_eq!(error.code, "definition_cancelled");
        assert_eq!(
            std::fs::read(project.join("HEAD")).expect("HEAD after render cancellation"),
            before
        );
    }

    #[test]
    fn definition_continuation_codec_rejects_mutation_and_predecessors() {
        let record = definition_record(
            DefinitionSection::Body,
            "definition.literal",
            &[
                ("owner", "expr_00000000000000000000000000000001".to_owned()),
                ("index", "0".to_owned()),
                ("bytes", "1".to_owned()),
                ("value", "x".to_owned()),
            ],
        )
        .expect("definition record");
        let binding = DefinitionBinding {
            repository: RepositoryId::from_bytes([1; 16]).expect("repository"),
            package: PackageId::from_bytes([2; 16]).expect("package"),
            revision: RevisionId::from_digest([3; 32]),
            function: "decl_04040404040404040404040404040404"
                .parse()
                .expect("function"),
        };
        let digest = DefinitionDigest([5; 32]);
        let token = encode_definition_continuation(binding, digest, 7, &record)
            .expect("definition continuation");
        let decoded = decode_definition_continuation(&token).expect("decode continuation");
        assert_eq!(decoded.binding, binding);
        assert_eq!(decoded.projection, digest);
        assert_eq!(decoded.index, 7);
        assert_eq!(decoded.section, DefinitionSection::Body);
        assert_eq!(decoded.resume_key, record.key);
        let mut mutated = token.into_bytes();
        let last = mutated.last_mut().expect("token byte");
        *last = if *last == b'A' { b'B' } else { b'A' };
        let mutated = String::from_utf8(mutated).expect("mutated token UTF-8");
        let error = decode_definition_continuation(&mutated).expect_err("mutated token");
        assert!(matches!(
            error.code.as_str(),
            "definition_continuation_integrity"
                | "definition_continuation_noncanonical"
                | "definition_continuation_malformed"
        ));
        assert_eq!(
            decode_definition_continuation("qcont_AA")
                .expect_err("query continuation")
                .code,
            "predecessor_contract"
        );
        assert_eq!(definition_text_fragments("").len(), 1);
        let text = "x".repeat(MAXIMUM_FUNCTION_DEFINITION_LITERAL_FRAGMENT_BYTES + 1);
        let fragments = definition_text_fragments(&text);
        assert_eq!(fragments.len(), 2);
        assert_eq!(fragments.concat(), text);
    }

    #[test]
    fn definition_logical_admissions_accept_exact_fit_and_reject_one_over() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let repository = GraphRepository::open(&project).expect("lkjournal repository");
        let view = repository.view_current().expect("lkjournal view");
        let mut reader = view.definition_reader();
        let mut cancellation = || Ok(());
        let mut materializer =
            DefinitionMaterializer::new(&mut reader, view.package(), &mut cancellation);

        materializer.body_records = MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS - 1;
        materializer
            .admit_body_record(MAXIMUM_FUNCTION_DEFINITION_DEPTH)
            .expect("exact body/depth admission");
        assert_eq!(
            materializer.body_records,
            MAXIMUM_FUNCTION_DEFINITION_BODY_RECORDS
        );
        assert_eq!(
            materializer
                .admit_body_record(MAXIMUM_FUNCTION_DEFINITION_DEPTH)
                .expect_err("one-over body records")
                .code,
            "definition_body_record_limit"
        );
        materializer.body_records = 0;
        assert_eq!(
            materializer
                .admit_body_record(MAXIMUM_FUNCTION_DEFINITION_DEPTH + 1)
                .expect_err("one-over body depth")
                .code,
            "definition_depth_limit"
        );
        materializer.structural_edges = MAXIMUM_FUNCTION_DEFINITION_EDGES;
        materializer
            .require_edge_total(MAXIMUM_FUNCTION_DEFINITION_EDGES, 0)
            .expect("exact edge admission");
        assert_eq!(
            materializer
                .admit_structural_edge()
                .expect_err("one-over edge admission")
                .code,
            "definition_edge_limit"
        );
        materializer
            .require_fact_count(MAXIMUM_FUNCTION_DEFINITION_FACT_READS)
            .expect("exact fact admission");
        assert_eq!(
            materializer
                .require_fact_count(MAXIMUM_FUNCTION_DEFINITION_FACT_READS + 1)
                .expect_err("one-over fact admission")
                .code,
            "definition_fact_limit"
        );

        let record_bytes = 64 * 1_024;
        let record_count = MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES / record_bytes;
        let mut exact = Vec::with_capacity(record_count);
        for _ in 0..record_count {
            exact.push(DefinitionLogicalRecord {
                section: DefinitionSection::Body,
                bytes: vec![b'x'; record_bytes],
                key: [0; 32],
            });
        }
        let (_, bytes) = definition_logical_digest(&exact).expect("exact logical byte admission");
        assert_eq!(bytes, MAXIMUM_FUNCTION_DEFINITION_LOGICAL_BYTES);
        exact.push(DefinitionLogicalRecord {
            section: DefinitionSection::Body,
            bytes: vec![b'x'],
            key: [0; 32],
        });
        assert_eq!(
            definition_logical_digest(&exact)
                .expect_err("one-over logical byte admission")
                .code,
            "definition_logical_byte_limit"
        );
        assert_eq!(
            parse_definition_item_limit(&MAXIMUM_FUNCTION_DEFINITION_ITEMS.to_string())
                .expect("exact page item admission"),
            MAXIMUM_FUNCTION_DEFINITION_ITEMS
        );
        assert_eq!(
            parse_definition_item_limit(&(MAXIMUM_FUNCTION_DEFINITION_ITEMS + 1).to_string())
                .expect_err("one-over page item admission")
                .code,
            "definition_invalid_limit"
        );
        assert_eq!(
            parse_definition_byte_limit(&MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES.to_string())
                .expect("exact page byte admission"),
            MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES
        );
        assert_eq!(
            parse_definition_byte_limit(
                &(MAXIMUM_FUNCTION_DEFINITION_OUTPUT_BYTES + 1).to_string()
            )
            .expect_err("one-over page byte admission")
            .code,
            "definition_invalid_byte_limit"
        );
    }

    #[test]
    fn definition_materializer_rejects_missing_shared_and_wrong_ownership() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let repository = GraphRepository::open(&project).expect("lkjournal repository");
        let view = repository.view_current().expect("lkjournal view");
        let function: KernelOwnerKey = "decl_a914bb78de075ff44a857ac028d704f3"
            .parse()
            .expect("worker function");

        let mut reader = view.definition_reader();
        let mut cancellation = || Ok(());
        let mut shared =
            DefinitionMaterializer::new(&mut reader, view.package(), &mut cancellation);
        shared
            .load_function_root(function, KernelOwnerKind::TaskFunction)
            .expect("first function root");
        assert_eq!(
            shared
                .load_function_root(function, KernelOwnerKind::TaskFunction)
                .expect_err("shared function root")
                .code,
            "definition_shared_or_cyclic"
        );

        let mut reader = view.definition_reader();
        let mut cancellation = || Ok(());
        let mut missing =
            DefinitionMaterializer::new(&mut reader, view.package(), &mut cancellation);
        let absent = KernelOwnerKey::Expression(
            crate::platform::semantic_id::ExpressionId::migrate(b"missing-definition-owner", 1),
        );
        assert_eq!(
            missing
                .load_structural_owner(
                    absent,
                    OwnershipEntry::new(
                        OwnershipParent::Owner(function),
                        OwnershipRole::ExpressionRoot(ExpressionRootRole::FunctionBody),
                    ),
                    Some(0),
                )
                .expect_err("missing structural owner")
                .code,
            "definition_owner_missing"
        );

        let root = view
            .owner(function)
            .expect("function read")
            .value
            .expect("function owner");
        let OwnerRecord::Declaration(declaration) = root else {
            panic!("worker root is not a declaration");
        };
        let DeclarationPayload::Function(worker) = declaration.payload else {
            panic!("worker root has no body");
        };
        let body = KernelOwnerKey::Expression(worker.body);
        let mut reader = view.definition_reader();
        let mut cancellation = || Ok(());
        let mut wrong = DefinitionMaterializer::new(&mut reader, view.package(), &mut cancellation);
        assert_eq!(
            wrong
                .load_structural_owner(
                    body,
                    OwnershipEntry::new(
                        OwnershipParent::Owner(function),
                        OwnershipRole::ExpressionRoot(ExpressionRootRole::ConstantValue),
                    ),
                    Some(0),
                )
                .expect_err("wrong structural role")
                .code,
            "definition_ownership_mismatch"
        );
    }

    #[test]
    fn definition_public_boundary_rejects_foreign_missing_and_nonfunction_owners() {
        let project = Path::new(env!("CARGO_MANIFEST_DIR")).join("applications/lkjournal");
        let base = vec![
            "--project".to_owned(),
            project.display().to_string(),
            "inspect".to_owned(),
            "owner".to_owned(),
        ];
        for (suffix, code) in [
            (
                vec![
                    "task_function".to_owned(),
                    "decl_a914bb78de075ff44a857ac028d704f3".to_owned(),
                    "--detail".to_owned(),
                    "definition".to_owned(),
                    "--package".to_owned(),
                    "pkg_10000000000000000000000000000001".to_owned(),
                ],
                "definition_dependency_body",
            ),
            (
                vec![
                    "module".to_owned(),
                    "mod_0510586a801c429b7a4a49a217de7fab".to_owned(),
                    "--detail".to_owned(),
                    "definition".to_owned(),
                ],
                "definition_owner_kind",
            ),
            (
                vec![
                    "pure_function".to_owned(),
                    "decl_01010101010101010101010101010101".to_owned(),
                    "--detail".to_owned(),
                    "definition".to_owned(),
                ],
                "definition_owner_not_found",
            ),
        ] {
            let mut arguments = base.clone();
            arguments.extend(suffix);
            assert_eq!(
                execute_inspect_owner(arguments)
                    .expect_err("definition boundary rejection")
                    .code,
                code
            );
        }
    }

    fn response_field<'a>(
        records: &'a [crate::platform::control::CompactRecord],
        operation: &str,
        name: &str,
    ) -> &'a str {
        records
            .iter()
            .find(|record| record.operation == operation)
            .and_then(|record| record.fields.iter().find(|field| field.name == name))
            .map(|field| field.value.as_str())
            .unwrap_or_else(|| panic!("missing response field {operation}.{name}"))
    }

    fn compact_record_identity(
        record: &crate::platform::control::CompactRecord,
    ) -> (String, Vec<(String, String)>) {
        (
            record.operation.clone(),
            record
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.value.clone()))
                .collect(),
        )
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
