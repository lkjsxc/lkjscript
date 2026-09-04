//! Copied-binary stateful HTTP authoring acceptance.

use crate::authority::{self, AuthorityObservation};
use crate::error::DevError;
use crate::evidence::{self, FileProof, VerificationDigest};
use crate::http_probe;
use crate::process::{self, ProcessControl, ProcessObservation, ProcessSpec, ProcessStatus};
use crate::stateful_http_program::{ProjectReferences, StandardReferences, build_program_request};
use lkjscript::platform::control::{CompactRecord, decode_logical_change_plan, parse_records};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, Read};
use std::net::SocketAddr;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const MAXIMUM_OUTPUT_BYTES: u64 = 32 * 1024 * 1024;
const MAXIMUM_CANDIDATE_BINARY_BYTES: u64 = 256 * 1024 * 1024;
const MAXIMUM_VERIFIER_BINARY_BYTES: u64 = 384 * 1024 * 1024;
const MAXIMUM_RECEIPT_BYTES: u64 = 128 * 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(120);
const RUNNER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_TIMEOUT: Duration = Duration::from_secs(35);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
const MAXIMUM_CONCURRENT_TASKS: u64 = 8;
const MAXIMUM_QUEUED_TASKS: u64 = 32;
pub(crate) const STATEFUL_SCHEMA: &str = "lkjscript-stateful-http-acceptance";
pub(crate) const STATEFUL_SCHEMA_VERSION: u32 = 6;
const STATEFUL_WORKFLOW: &str = "stateful-http-application";
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct Options {
    binary: PathBuf,
    evidence_root: Option<PathBuf>,
    machine: bool,
}

#[derive(Debug)]
struct Context {
    binary: PathBuf,
    evidence: PathBuf,
    observation_root: PathBuf,
    ordinal: u64,
    commands: Vec<CommandEvidence>,
}

#[derive(Debug)]
struct Output {
    bytes: Vec<u8>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct SchemaIdentity {
    identity: String,
    version: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ExecutableObservation {
    file: FileProof,
    byte_length: u64,
    mode: u32,
    executable: bool,
    sha256: String,
    verification_digest: VerificationDigest,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlatformObservation {
    operating_system: String,
    architecture: String,
    process_control: String,
    client: String,
    data_engine: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StatefulStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Failure {
    class: String,
    code: String,
    message: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlanObservation {
    bytes: u64,
    records: u64,
    allocations: u64,
    owners: u64,
    types: u64,
    dependencies: u64,
    retirements: u64,
    relations_removed: u64,
    relations_added: u64,
    structural_owners: u64,
    semantic_owners: u64,
    tests: u64,
    reasons: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpObservation {
    name: String,
    method: String,
    path: String,
    status: u16,
    request_bytes: u64,
    response_bytes: u64,
    response_sha256: String,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ResidentRuntimeObservation {
    runs: u64,
    admitted_tasks: u64,
    completed_tasks: u64,
    failed_tasks: u64,
    cancelled_tasks: u64,
    overloaded_tasks: u64,
    rejected_after_shutdown_tasks: u64,
    maximum_queued_tasks: u64,
    maximum_active_tasks: u64,
    maximum_admission_permits: u64,
    maximum_worker_permits: u64,
}

impl ResidentRuntimeObservation {
    fn bounded_and_closed(&self, expected_runs: u64) -> bool {
        self.runs == expected_runs
            && self.admitted_tasks > 0
            && self.admitted_tasks == self.completed_tasks
            && self.failed_tasks <= self.completed_tasks
            && self.cancelled_tasks <= self.failed_tasks
            && (1..=MAXIMUM_QUEUED_TASKS).contains(&self.maximum_queued_tasks)
            && (1..=MAXIMUM_CONCURRENT_TASKS).contains(&self.maximum_active_tasks)
            && self.maximum_admission_permits >= self.maximum_queued_tasks
            && self.maximum_admission_permits >= self.maximum_worker_permits
            && self.maximum_admission_permits <= MAXIMUM_CONCURRENT_TASKS + MAXIMUM_QUEUED_TASKS
            && self.maximum_worker_permits >= self.maximum_active_tasks
            && self.maximum_worker_permits <= MAXIMUM_CONCURRENT_TASKS
    }
}

#[derive(Clone, Debug)]
struct HttpStopObservation {
    matcher_nodes: u64,
    runtime: ResidentRuntimeObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct LiveObservation {
    data_contract: String,
    data_root: String,
    matcher_nodes: u64,
    matcher_step_bound: u64,
    runtime: ResidentRuntimeObservation,
    routes_checked: u64,
    created_identity: String,
    persistence_after_restart: bool,
    backup_restore_equivalent: bool,
    backup_digest: String,
    startup_failures_without_ready: u64,
    absent_root_no_ready: bool,
    corrupt_root_no_ready: bool,
    malformed_request_contained: bool,
    exact_over_pattern_precedence: bool,
    ordered_two_captures: bool,
    capture_query_ignored: bool,
    runner_starts: u64,
    shutdown_cleanup_failures: u64,
    temporary_data_cleanup_complete: bool,
    authority_before: AuthorityObservation,
    authority_after: AuthorityObservation,
    authority_unchanged: bool,
    requests: Vec<HttpObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AuthoringResult {
    workflow: String,
    project: String,
    initial_revision: String,
    accepted_revision: String,
    request_records: usize,
    request_bytes: usize,
    plan_token: String,
    plan: PlanObservation,
    idempotent_reconciliation: bool,
    discovery_commands: u64,
    capabilities_digest: String,
    initial_template: String,
    initial_owners: u64,
    initial_dependencies: u64,
    builtin_package: String,
    builtin_semantic_revision: String,
    builtin_package_revision: String,
    builtin_transport: String,
    dependency_staged: bool,
    pattern_lifecycle: PatternLifecycleObservation,
    topology: TopologyObservation,
    artifact: String,
    artifact_bytes: u64,
    incremental_sha256: String,
    clean_sha256: String,
    deterministic: bool,
    live: LiveObservation,
    evidence: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PatternLifecycleObservation {
    route: String,
    temporary_route: String,
    set_preserved_identity: bool,
    altered_plan_rejected: bool,
    stale_plan_rejected: bool,
    reviewed_selector_evidence: bool,
    temporary_pattern_inspected: bool,
    temporary_pattern_deleted: bool,
    intermediate_revisions: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct TopologyObservation {
    module: String,
    component: String,
    requirements: BTreeMap<String, String>,
    routes: Vec<HttpRouteObservation>,
    target: String,
    target_name: String,
    runner: String,
    exact_routes: u64,
    pattern_routes: u64,
    pattern_segments: u64,
    maximum_specificity_chain: u64,
    route_set: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpRouteObservation {
    route: String,
    method: String,
    selector: String,
    path: String,
    captures: Vec<String>,
    port: String,
    handler: String,
    signature: String,
    parameters: Vec<HttpRouteParameterObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpRouteParameterObservation {
    id: String,
    index: u64,
    name: String,
    ty: String,
    use_mode: String,
    requirement: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StatefulReceipt {
    schema: SchemaIdentity,
    status: StatefulStatus,
    workflow: String,
    platform: PlatformObservation,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    execution_context: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checkout_root: Option<String>,
    evidence_root: String,
    isolated_root: String,
    isolated_root_outside_checkout: bool,
    verifier: ExecutableObservation,
    candidate: ExecutableObservation,
    copied_candidate: ExecutableObservation,
    environment_names: Vec<String>,
    commands: Vec<CommandEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<AuthoringResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
    cleanup: CleanupObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupObservation {
    temporary_root_removed: bool,
    data_cleanup_complete: bool,
    runner_cleanup_complete: bool,
    raw_secret_values_retained: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct TransferredReceiptBinding {
    pub(crate) receipt_bytes: u64,
    pub(crate) receipt_sha256: String,
    pub(crate) verifier_sha256: String,
    pub(crate) candidate_sha256: String,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) commands: u64,
    pub(crate) requests: u64,
    pub(crate) data_contract: String,
    pub(crate) cleanup_complete: bool,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_options(arguments)?;
    let verifier_path = current_verifier()?;
    let verifier = executable_observation(
        &verifier_path,
        "stateful HTTP verifier",
        MAXIMUM_VERIFIER_BINARY_BYTES,
    )?;
    let (execution_context, checkout_root, binary, evidence_path, observation_root) =
        if let Some(requested_evidence_root) = &options.evidence_root {
            let binary = resolve_binary(None, &options.binary)?;
            let evidence = create_explicit_evidence_root(requested_evidence_root)?;
            (
                "transferred".to_owned(),
                None,
                binary,
                evidence.clone(),
                evidence,
            )
        } else {
            let repository = repository_root()?;
            let binary = resolve_binary(Some(&repository), &options.binary)?;
            let evidence = new_evidence_directory(&repository)?;
            (
                "contributor".to_owned(),
                Some(repository.clone()),
                binary,
                evidence,
                repository,
            )
        };
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let candidate = executable_observation(
        &binary,
        "stateful HTTP candidate",
        MAXIMUM_CANDIDATE_BINARY_BYTES,
    )?;
    let temporary = tempfile::Builder::new()
        .prefix("lkjscript-stateful-http-")
        .tempdir()
        .map_err(|error| DevError::infrastructure(format!("create isolated root: {error}")))?;
    let isolated = temporary.path().canonicalize()?;
    let outside_checkout = checkout_root
        .as_ref()
        .is_none_or(|repository| !isolated.starts_with(repository));
    let copied = isolated.join("lkjscript");
    copy_binary(&binary, &copied)?;
    let copied_candidate = executable_observation(
        &copied,
        "copied stateful HTTP candidate",
        MAXIMUM_CANDIDATE_BINARY_BYTES,
    )?;
    if candidate.byte_length != copied_candidate.byte_length
        || candidate.sha256 != copied_candidate.sha256
        || candidate.verification_digest != copied_candidate.verification_digest
        || candidate.mode != copied_candidate.mode
    {
        return Err(DevError::infrastructure(
            "copied stateful HTTP candidate disagrees with the source candidate",
        ));
    }
    let mut context = Context {
        binary: copied,
        evidence: evidence_path.clone(),
        observation_root,
        ordinal: 0,
        commands: Vec::new(),
    };
    let workflow = if outside_checkout {
        run_authoring(&mut context, &isolated)
    } else {
        Err(DevError::corrupt(
            "stateful HTTP isolated root is inside the checkout",
        ))
    };
    let cleanup = temporary.close().map_err(|error| {
        DevError::infrastructure(format!("remove isolated stateful HTTP root: {error}"))
    });
    let cleanup_complete = cleanup.is_ok();
    let (status, result, failure) = match (workflow, cleanup) {
        (Ok(result), Ok(())) => (StatefulStatus::Passed, Some(result), None),
        (Err(error), _) | (Ok(_), Err(error)) => {
            let status = if error.kind() == "unavailable" {
                StatefulStatus::Unavailable
            } else {
                StatefulStatus::Failed
            };
            let failure = Failure {
                class: error.kind().to_owned(),
                code: format!("stateful_http_{}", error.kind()),
                message: error.message().to_owned(),
            };
            (status, None, Some(failure))
        }
    };
    let elapsed_nanoseconds = u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX);
    let data_cleanup_complete = result
        .as_ref()
        .is_some_and(|result| result.live.temporary_data_cleanup_complete);
    let runner_cleanup_complete = result
        .as_ref()
        .is_some_and(|result| result.live.shutdown_cleanup_failures == 0);
    let isolated_text = isolated.display().to_string();
    let receipt_value = StatefulReceipt {
        schema: SchemaIdentity {
            identity: STATEFUL_SCHEMA.to_owned(),
            version: STATEFUL_SCHEMA_VERSION,
        },
        status,
        workflow: STATEFUL_WORKFLOW.to_owned(),
        platform: PlatformObservation {
            operating_system: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            process_control: "linux-process-group-sigint-sigkill".to_owned(),
            client: "first-party-bounded-raw-http1".to_owned(),
            data_engine: "first-party-ordered-data".to_owned(),
        },
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds,
        execution_context,
        checkout_root: checkout_root.map(|path| path.display().to_string()),
        evidence_root: evidence_path.display().to_string(),
        isolated_root: isolated_text,
        isolated_root_outside_checkout: outside_checkout,
        verifier,
        candidate,
        copied_candidate,
        environment_names: vec!["LANG".to_owned()],
        commands: context.commands,
        result,
        failure,
        cleanup: CleanupObservation {
            temporary_root_removed: cleanup_complete && !isolated.exists(),
            data_cleanup_complete,
            runner_cleanup_complete,
            raw_secret_values_retained: false,
        },
    };
    let receipt = evidence_path.join("receipt.json");
    let published = evidence::publish_json(&receipt, &receipt_value)?;
    let receipt_sha256 = sha256_file(&receipt, MAXIMUM_RECEIPT_BYTES)?;
    if options.machine {
        println!(
            "{}",
            serde_json::to_string(&serde_json::json!({
                "status": status,
                "schema": receipt_value.schema,
                "workflow": receipt_value.workflow,
                "receipt": receipt,
                "receipt_bytes": published.bytes,
                "receipt_digest": published.digest,
                "receipt_sha256": receipt_sha256,
                "verifier_sha256": receipt_value.verifier.sha256,
                "candidate_sha256": receipt_value.candidate.sha256,
                "execution_context": receipt_value.execution_context,
                "cleanup_complete": receipt_value.cleanup.temporary_root_removed
                    && receipt_value.cleanup.data_cleanup_complete
                    && receipt_value.cleanup.runner_cleanup_complete,
            }))?
        );
    } else {
        println!("stateful HTTP application {status:?}");
        println!("receipt: {}", receipt.display());
        println!("receipt digest: {}", published.digest);
    }
    Ok(match status {
        StatefulStatus::Passed => 0,
        StatefulStatus::Failed => 1,
        StatefulStatus::Unavailable => 2,
    })
}

pub(crate) fn read_transferred_receipt(
    path: &Path,
    candidate_path: &Path,
    verifier_path: &Path,
) -> Result<TransferredReceiptBinding, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect transferred stateful receipt '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAXIMUM_RECEIPT_BYTES
    {
        return Err(DevError::corrupt(
            "transferred stateful receipt is unsafe or oversized",
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        DevError::infrastructure(format!(
            "read transferred stateful receipt '{}': {error}",
            path.display()
        ))
    })?;
    let receipt: StatefulReceipt = serde_json::from_slice(&bytes).map_err(|error| {
        DevError::corrupt(format!("decode transferred stateful receipt: {error}"))
    })?;
    if evidence::encode_json(&receipt)? != bytes {
        return Err(DevError::corrupt(
            "transferred stateful receipt is not in canonical evidence encoding",
        ));
    }
    let evidence_root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("transferred stateful receipt has no parent"))?
        .canonicalize()
        .map_err(|error| {
            DevError::infrastructure(format!(
                "resolve transferred stateful evidence root: {error}"
            ))
        })?;
    let expected_path = evidence_root.join("receipt.json");
    let canonical_path = path.canonicalize().map_err(|error| {
        DevError::infrastructure(format!("resolve transferred stateful receipt: {error}"))
    })?;
    let candidate = executable_observation(
        candidate_path,
        "target-admission stateful candidate",
        MAXIMUM_CANDIDATE_BINARY_BYTES,
    )?;
    let verifier = executable_observation(
        verifier_path,
        "target-admission stateful verifier",
        MAXIMUM_VERIFIER_BINARY_BYTES,
    )?;
    let result = receipt
        .result
        .as_ref()
        .ok_or_else(|| DevError::corrupt("passed stateful receipt omitted its result"))?;
    let cleanup_complete = receipt.cleanup.temporary_root_removed
        && receipt.cleanup.data_cleanup_complete
        && receipt.cleanup.runner_cleanup_complete
        && !receipt.cleanup.raw_secret_values_retained;
    let copied_path = Path::new(&receipt.copied_candidate.file.path);
    if canonical_path != expected_path
        || receipt.schema.identity != STATEFUL_SCHEMA
        || receipt.schema.version != STATEFUL_SCHEMA_VERSION
        || receipt.status != StatefulStatus::Passed
        || receipt.workflow != STATEFUL_WORKFLOW
        || receipt.execution_context != "transferred"
        || receipt.checkout_root.is_some()
        || receipt.evidence_root != evidence_root.display().to_string()
        || !receipt.isolated_root_outside_checkout
        || Path::new(&receipt.isolated_root).exists()
        || !copied_path.starts_with(Path::new(&receipt.isolated_root))
        || receipt.environment_names != ["LANG"]
        || receipt.completed_unix_nanoseconds < receipt.started_unix_nanoseconds
        || receipt.verifier.sha256 != verifier.sha256
        || receipt.verifier.byte_length != verifier.byte_length
        || receipt.verifier.mode != verifier.mode
        || receipt.verifier.verification_digest != verifier.verification_digest
        || receipt.candidate.sha256 != candidate.sha256
        || receipt.candidate.byte_length != candidate.byte_length
        || receipt.candidate.mode != candidate.mode
        || receipt.candidate.verification_digest != candidate.verification_digest
        || receipt.copied_candidate.sha256 != candidate.sha256
        || receipt.copied_candidate.byte_length != candidate.byte_length
        || receipt.copied_candidate.mode != candidate.mode
        || receipt.copied_candidate.verification_digest != candidate.verification_digest
        || receipt.failure.is_some()
        || !cleanup_complete
        || result.workflow != STATEFUL_WORKFLOW
        || result.request_records == 0
        || !result.idempotent_reconciliation
        || result.discovery_commands == 0
        || !result.deterministic
        || result.initial_template != "minimal"
        || result.initial_owners != 0
        || result.initial_dependencies != 0
        || !result.dependency_staged
        || !result.pattern_lifecycle.set_preserved_identity
        || !result.pattern_lifecycle.altered_plan_rejected
        || !result.pattern_lifecycle.stale_plan_rejected
        || !result.pattern_lifecycle.reviewed_selector_evidence
        || !result.pattern_lifecycle.temporary_pattern_inspected
        || !result.pattern_lifecycle.temporary_pattern_deleted
        || result.pattern_lifecycle.intermediate_revisions != 4
        || result.topology.target_name != "serve"
        || result.topology.runner != "http"
        || result.topology.requirements.len() != 4
        || result.topology.routes.len() != 6
        || result.topology.exact_routes != 4
        || result.topology.pattern_routes != 2
        || result.topology.pattern_segments != 6
        || result.topology.maximum_specificity_chain != 2
        || result.topology.route_set.is_empty()
        || ["streams", "data", "identifiers", "clock"]
            .iter()
            .any(|name| !result.topology.requirements.contains_key(*name))
        || [
            ("GET", "/"),
            ("GET", "/api/posts"),
            ("POST", "/api/posts"),
            ("PUT", "/api/posts/{id}"),
            ("DELETE", "/api/{space}/{id}"),
            ("DELETE", "/api/posts/featured"),
        ]
        .iter()
        .any(|(method, path)| {
            !result
                .topology
                .routes
                .iter()
                .any(|route| route.method == *method && route.path == *path)
        })
        || result.incremental_sha256 != result.clean_sha256
        || result.evidence != evidence_root.display().to_string()
        || result.live.data_contract != "lkjscript-data-store-1"
        || result.live.matcher_nodes == 0
        || result.live.matcher_step_bound != result.live.matcher_nodes.saturating_add(1)
        || !result.live.runtime.bounded_and_closed(3)
        || !result.live.persistence_after_restart
        || result.live.startup_failures_without_ready != 2
        || !result.live.backup_restore_equivalent
        || !result.live.absent_root_no_ready
        || !result.live.corrupt_root_no_ready
        || !result.live.malformed_request_contained
        || !result.live.exact_over_pattern_precedence
        || !result.live.ordered_two_captures
        || !result.live.capture_query_ignored
        || result.live.runner_starts != 3
        || result.live.shutdown_cleanup_failures != 0
        || !result.live.temporary_data_cleanup_complete
        || !result.live.authority_unchanged
        || result.live.authority_before != result.live.authority_after
        || result.live.routes_checked != result.live.requests.len() as u64
        || result.live.requests.is_empty()
    {
        return Err(DevError::corrupt(
            "transferred stateful receipt binding or acceptance mismatch",
        ));
    }
    if receipt.commands.iter().any(|command| {
        command.command.first().is_some_and(|program| {
            program == "cargo"
                || program == "rustc"
                || program.ends_with("/cargo")
                || program.ends_with("/rustc")
        })
    }) {
        return Err(DevError::corrupt(
            "transferred stateful receipt invoked checkout build tooling",
        ));
    }
    Ok(TransferredReceiptBinding {
        receipt_bytes: metadata.len(),
        receipt_sha256: sha256_file(path, MAXIMUM_RECEIPT_BYTES)?,
        verifier_sha256: verifier.sha256,
        candidate_sha256: candidate.sha256,
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        commands: receipt.commands.len() as u64,
        requests: result.live.requests.len() as u64,
        data_contract: result.live.data_contract.clone(),
        cleanup_complete,
    })
}

fn run_authoring(context: &mut Context, isolated: &Path) -> Result<AuthoringResult, DevError> {
    let capabilities = records(
        &context
            .success("capabilities", &["capabilities"], isolated)?
            .bytes,
    )?;
    require_record_field(&capabilities, "product", "name", "lkjscript")?;
    require_record_field(
        &capabilities,
        "product",
        "version",
        lkjscript::PRODUCT_VERSION,
    )?;
    let capabilities_digest =
        required_field(required_record(&capabilities, "capabilities")?, "digest")?.to_owned();
    for section in ["change", "deployment", "expression", "type"] {
        let present = capabilities
            .iter()
            .any(|record| record.operation == "section" && field(record, "name") == Some(section));
        if !present {
            return Err(DevError::corrupt(format!(
                "capabilities omitted required '{section}' section"
            )));
        }
    }
    verify_discovery(context, isolated)?;
    let builtin = records(
        &context
            .success(
                "builtin-summary",
                &["package", "builtin", "inspect"],
                isolated,
            )?
            .bytes,
    )?;
    let builtin_record = required_record(&builtin, "package")?;
    let builtin_package = required_field(builtin_record, "id")?.to_owned();
    let builtin_semantic_revision = required_field(builtin_record, "revision")?.to_owned();
    let builtin_package_revision = required_field(builtin_record, "package-revision")?.to_owned();
    let builtin_transport = required_field(builtin_record, "transport")?.to_owned();
    let standard = discover_standard(
        context,
        isolated,
        builtin_package.clone(),
        builtin_semantic_revision.clone(),
        builtin_package_revision.clone(),
    )?;
    let discovery_commands = context.ordinal;
    let project = isolated.join("application");
    let created = records(
        &context
            .success(
                "new-minimal",
                &[
                    "new",
                    path_text(&project)?,
                    "--template",
                    "minimal",
                    "--name",
                    "bbs",
                ],
                isolated,
            )?
            .bytes,
    )?;
    require_record_field(&created, "project", "template", "minimal")?;
    require_record_field(&created, "summary", "owners", "0")?;
    require_record_field(&created, "summary", "dependencies", "0")?;
    let initial_revision = required_field(required_record(&created, "revision")?, "id")?.to_owned();
    let transport_path = isolated.join("builtin-standard.transport");
    let exported = records(
        &context
            .success(
                "builtin-export-transport",
                &[
                    "package",
                    "builtin",
                    "export",
                    "--kind",
                    "transport",
                    "--output",
                    path_text(&transport_path)?,
                ],
                isolated,
            )?
            .bytes,
    )?;
    require_record_field(&exported, "output", "digest", &builtin_transport)?;
    let staged = records(
        &context
            .success(
                "dependency-stage",
                &[
                    "--project",
                    path_text(&project)?,
                    "package",
                    "dependency",
                    "stage",
                    "--transport",
                    &builtin_transport,
                    "--input-file",
                    path_text(&transport_path)?,
                ],
                isolated,
            )?
            .bytes,
    )?;
    require_record_field(&staged, "result", "outcome", "inserted")?;
    require_record_field(&staged, "package", "id", &builtin_package)?;
    require_record_field(&staged, "package", "revision", &builtin_semantic_revision)?;
    require_record_field(
        &staged,
        "package",
        "package-revision",
        &builtin_package_revision,
    )?;
    require_record_field(&staged, "package", "transport", &builtin_transport)?;
    require_record_field(&staged, "authority", "current-revision", &initial_revision)?;
    require_record_field(&staged, "authority", "semantic-head-changed", "false")?;
    let project_references = ProjectReferences {
        base_revision: initial_revision.clone(),
    };
    let request = build_program_request(&standard, &project_references)?;
    let request_path = isolated.join("bbs-change.lkjc");
    evidence::publish(&request_path, &request.bytes)?;
    evidence::publish(&context.evidence.join("bbs-change.lkjc"), &request.bytes)?;
    let plan_path = isolated.join("bbs-change.logical-plan");
    let planned = records(
        &context
            .success(
                "change-plan",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "plan",
                    "--input-file",
                    path_text(&request_path)?,
                    "--output",
                    path_text(&plan_path)?,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let plan_token = required_field(required_record(&planned, "plan")?, "token")?.to_owned();
    let plan_file = File::open(&plan_path)?;
    let decoded_plan =
        decode_logical_change_plan(BufReader::new(plan_file)).map_err(|diagnostic| {
            DevError::corrupt(format!(
                "strict logical-plan decode failed: {}",
                diagnostic.code
            ))
        })?;
    if decoded_plan.token != plan_token {
        return Err(DevError::corrupt(
            "strictly decoded logical-plan token disagrees with plan output",
        ));
    }
    let plan_output = required_record(&planned, "plan-output")?;
    let advertised_plan_bytes = required_field(plan_output, "bytes")?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("plan output byte count is not u64"))?;
    let advertised_plan_records = required_field(plan_output, "records")?
        .parse::<u64>()
        .map_err(|_| DevError::corrupt("plan output record count is not u64"))?;
    if decoded_plan.bytes != advertised_plan_bytes
        || decoded_plan.records != advertised_plan_records
    {
        return Err(DevError::corrupt(
            "strict logical-plan decode disagrees with advertised meters",
        ));
    }
    let plan_bytes = fs::read(&plan_path)?;
    evidence::publish(
        &context.evidence.join("bbs-change.logical-plan"),
        &plan_bytes,
    )?;
    let plan = PlanObservation {
        bytes: decoded_plan.bytes,
        records: decoded_plan.records,
        allocations: decoded_plan.counts.allocations,
        owners: decoded_plan.counts.owners,
        types: decoded_plan.counts.types,
        dependencies: decoded_plan.counts.dependencies,
        retirements: decoded_plan.counts.retirements,
        relations_removed: decoded_plan.counts.relations_removed,
        relations_added: decoded_plan.counts.relations_added,
        structural_owners: decoded_plan.counts.structural_owners,
        semantic_owners: decoded_plan.counts.semantic_owners,
        tests: decoded_plan.counts.tests,
        reasons: decoded_plan.counts.reasons,
    };
    let replay_plan_path = isolated.join("bbs-change-replay.logical-plan");
    let replay_planned = records(
        &context
            .success(
                "change-plan-repeat",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "plan",
                    "--input-file",
                    path_text(&request_path)?,
                    "--output",
                    path_text(&replay_plan_path)?,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let replay_token =
        required_field(required_record(&replay_planned, "plan")?, "token")?.to_owned();
    let replay_plan_bytes = fs::read(&replay_plan_path)?;
    evidence::publish(
        &context.evidence.join("bbs-change-replay.logical-plan"),
        &replay_plan_bytes,
    )?;
    if replay_token != plan_token || replay_plan_bytes != plan_bytes {
        let first_difference = plan_bytes
            .split(|byte| *byte == b'\n')
            .zip(replay_plan_bytes.split(|byte| *byte == b'\n'))
            .position(|(left, right)| left != right)
            .unwrap_or(usize::MAX);
        return Err(DevError::corrupt(format!(
            "repeated pre-publication plan changed at record {first_difference}"
        )));
    }
    let applied = records(
        &context
            .success(
                "change-apply",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "apply",
                    "--input-file",
                    path_text(&request_path)?,
                    "--plan",
                    &plan_token,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let initial_accepted_revision =
        required_field(required_record(&applied, "revision")?, "result")?.to_owned();
    let reconciled = records(
        &context
            .success(
                "change-apply-reconcile",
                &[
                    "--project",
                    path_text(&project)?,
                    "change",
                    "apply",
                    "--input-file",
                    path_text(&request_path)?,
                    "--plan",
                    &plan_token,
                ],
                isolated,
            )?
            .bytes,
    )?;
    require_record_field(&reconciled, "result", "status", "already-accepted")?;
    require_record_field(
        &reconciled,
        "revision",
        "result",
        &initial_accepted_revision,
    )?;
    let initial_topology = discover_authored_topology(context, isolated, &project)?;
    let (accepted_revision, pattern_lifecycle) = exercise_pattern_lifecycle(
        context,
        isolated,
        &project,
        &initial_accepted_revision,
        &initial_topology,
    )?;
    let topology = discover_authored_topology(context, isolated, &project)?;
    context.success(
        "check",
        &["--project", path_text(&project)?, "check"],
        isolated,
    )?;
    fs::create_dir(project.join("generated")).map_err(|error| {
        DevError::infrastructure(format!(
            "create stateful derived output directory after semantic acceptance: {error}"
        ))
    })?;
    let artifact = project.join("generated/bbs.lkja");
    context.success(
        "build-incremental",
        &[
            "--project",
            path_text(&project)?,
            "build",
            "--output",
            path_text(&artifact)?,
        ],
        isolated,
    )?;
    let first = fs::read(&artifact)?;
    let derived = project.join("derived");
    if derived.exists() {
        fs::remove_dir_all(&derived)?;
    }
    let clean_artifact = project.join("generated/bbs-clean.lkja");
    context.success(
        "build-clean",
        &[
            "--project",
            path_text(&project)?,
            "build",
            "--output",
            path_text(&clean_artifact)?,
        ],
        isolated,
    )?;
    let clean = fs::read(&clean_artifact)?;
    if first != clean {
        return Err(DevError::corrupt(
            "clean and incremental stateful HTTP artifacts differ",
        ));
    }
    let live = run_live(context, isolated, &project, &artifact)?;
    Ok(AuthoringResult {
        workflow: STATEFUL_WORKFLOW.to_owned(),
        project: project.display().to_string(),
        initial_revision,
        accepted_revision,
        request_records: request.records,
        request_bytes: request.bytes.len(),
        plan_token,
        plan,
        idempotent_reconciliation: true,
        discovery_commands,
        capabilities_digest,
        initial_template: "minimal".to_owned(),
        initial_owners: 0,
        initial_dependencies: 0,
        builtin_package,
        builtin_semantic_revision,
        builtin_package_revision,
        builtin_transport,
        dependency_staged: true,
        pattern_lifecycle,
        topology,
        artifact: artifact.display().to_string(),
        artifact_bytes: first.len() as u64,
        incremental_sha256: sha256(&first),
        clean_sha256: sha256(&clean),
        deterministic: true,
        live,
        evidence: context.evidence.display().to_string(),
    })
}

#[derive(Debug)]
struct PlannedPatternChange {
    request_path: PathBuf,
    token: String,
    output: Vec<CompactRecord>,
    plan: Vec<CompactRecord>,
}

fn plan_pattern_change(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    label: &str,
    request: &str,
) -> Result<PlannedPatternChange, DevError> {
    let request_path = isolated.join(format!("{label}.lkjc"));
    let plan_path = isolated.join(format!("{label}.logical-plan"));
    evidence::publish(&request_path, request.as_bytes())?;
    evidence::publish(
        &context.evidence.join(format!("{label}.lkjc")),
        request.as_bytes(),
    )?;
    let arguments = vec![
        "--project".to_owned(),
        path_text(project)?.to_owned(),
        "change".to_owned(),
        "plan".to_owned(),
        "--input-file".to_owned(),
        path_text(&request_path)?.to_owned(),
        "--output".to_owned(),
        path_text(&plan_path)?.to_owned(),
    ];
    let output = records(
        &context
            .success(&format!("{label}-plan"), &arguments, isolated)?
            .bytes,
    )?;
    let token = required_field(required_record(&output, "plan")?, "token")?.to_owned();
    let plan_bytes = fs::read(&plan_path)?;
    evidence::publish(
        &context.evidence.join(format!("{label}.logical-plan")),
        &plan_bytes,
    )?;
    Ok(PlannedPatternChange {
        request_path,
        token,
        output,
        plan: records(&plan_bytes)?,
    })
}

fn apply_pattern_change(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    label: &str,
    planned: &PlannedPatternChange,
) -> Result<String, DevError> {
    let arguments = vec![
        "--project".to_owned(),
        path_text(project)?.to_owned(),
        "change".to_owned(),
        "apply".to_owned(),
        "--input-file".to_owned(),
        path_text(&planned.request_path)?.to_owned(),
        "--plan".to_owned(),
        planned.token.clone(),
    ];
    let output = records(
        &context
            .success(&format!("{label}-apply"), &arguments, isolated)?
            .bytes,
    )?;
    Ok(required_field(required_record(&output, "revision")?, "result")?.to_owned())
}

fn require_pattern_failure(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    label: &str,
    request_path: &Path,
    token: &str,
    expected_code: &str,
) -> Result<(), DevError> {
    let arguments = vec![
        "--project".to_owned(),
        path_text(project)?.to_owned(),
        "change".to_owned(),
        "apply".to_owned(),
        "--input-file".to_owned(),
        path_text(request_path)?.to_owned(),
        "--plan".to_owned(),
        token.to_owned(),
    ];
    let rejected = records(
        &context
            .failure(
                label,
                &arguments,
                isolated,
                BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
            )?
            .bytes,
    )?;
    require_record_field(&rejected, "diagnostic", "code", expected_code)
}

fn exercise_pattern_lifecycle(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    initial_revision: &str,
    topology: &TopologyObservation,
) -> Result<(String, PatternLifecycleObservation), DevError> {
    let update = topology
        .routes
        .iter()
        .find(|route| route.method == "PUT" && route.path == "/api/posts/{id}")
        .ok_or_else(|| DevError::corrupt("stateful topology omitted the update pattern"))?;
    let changed_request = format!(
        "request base={initial_revision}\n\
         set.http-route route={} method=PUT pattern=\"/api/articles/{{id}}\" port={}\n",
        update.route, update.port
    );
    let changed = plan_pattern_change(context, isolated, project, "pattern-set", &changed_request)?;
    let reviewed_before = changed.plan.iter().any(|record| {
        record.operation == "logical-plan.http-route-before"
            && field(record, "route") == Some(update.route.as_str())
            && field(record, "changed") == Some("true")
            && field(record, "kind") == Some("pattern")
            && field(record, "selector") == Some("/api/posts/{id}")
            && field(record, "captures") == Some("id")
            && field(record, "signature") == Some("(HttpRequest,id:Text)->HttpResponse")
    });
    let reviewed_after = changed.plan.iter().any(|record| {
        record.operation == "logical-plan.http-route-after"
            && field(record, "route") == Some(update.route.as_str())
            && field(record, "changed") == Some("true")
            && field(record, "kind") == Some("pattern")
            && field(record, "selector") == Some("/api/articles/{id}")
            && field(record, "captures") == Some("id")
            && field(record, "signature") == Some("(HttpRequest,id:Text)->HttpResponse")
            && field(record, "target-exact-routes") == Some("4")
            && field(record, "target-pattern-routes") == Some("2")
            && field(record, "target-pattern-segments") == Some("6")
            && field(record, "maximum-specificity-chain") == Some("2")
    });
    let reviewed_selector_evidence = reviewed_before && reviewed_after;
    if !reviewed_selector_evidence {
        return Err(DevError::corrupt(
            "reviewed pattern set plan omitted selector-indexed before/after evidence",
        ));
    }

    let altered_request = format!(
        "request base={initial_revision}\n\
         set.http-route route={} method=PUT pattern=\"/api/altered/{{id}}\" port={}\n",
        update.route, update.port
    );
    let altered_path = isolated.join("pattern-set-altered.lkjc");
    evidence::publish(&altered_path, altered_request.as_bytes())?;
    let head_before_altered = fs::read(project.join("HEAD"))?;
    require_pattern_failure(
        context,
        isolated,
        project,
        "pattern-set-altered-apply",
        &altered_path,
        &changed.token,
        "change_request_commitment_mismatch",
    )?;
    if fs::read(project.join("HEAD"))? != head_before_altered {
        return Err(DevError::corrupt(
            "altered reviewed pattern request advanced accepted authority",
        ));
    }

    let changed_revision =
        apply_pattern_change(context, isolated, project, "pattern-set", &changed)?;
    let changed_detail = records(
        &context
            .success(
                "inspect-pattern-set",
                &[
                    "--project",
                    path_text(project)?,
                    "inspect",
                    "owner",
                    "http_route",
                    &update.route,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let changed_owner = required_record(&changed_detail, "owner")?;
    let set_preserved_identity = field(changed_owner, "id") == Some(update.route.as_str())
        && field(changed_owner, "selector") == Some("pattern")
        && field(changed_owner, "path") == Some("/api/articles/{id}")
        && field(changed_owner, "captures") == Some("id");
    if !set_preserved_identity {
        return Err(DevError::corrupt(
            "set.http-route did not preserve and expose the pattern route identity",
        ));
    }

    let head_before_stale = fs::read(project.join("HEAD"))?;
    require_pattern_failure(
        context,
        isolated,
        project,
        "pattern-set-stale-apply",
        &changed.request_path,
        &changed.token,
        "change_authored_stale_base",
    )?;
    if fs::read(project.join("HEAD"))? != head_before_stale {
        return Err(DevError::corrupt(
            "stale reviewed pattern request advanced accepted authority",
        ));
    }

    let restored_request = format!(
        "request base={changed_revision}\n\
         set.http-route route={} method=PUT pattern=\"/api/posts/{{id}}\" port={}\n",
        update.route, update.port
    );
    let restored = plan_pattern_change(
        context,
        isolated,
        project,
        "pattern-restore",
        &restored_request,
    )?;
    let restored_revision =
        apply_pattern_change(context, isolated, project, "pattern-restore", &restored)?;

    let temporary_request = format!(
        "request base={restored_revision}\n\
         add.http-route as=$temporary target={} method=PATCH pattern=\"/temporary/{{id}}\" port={}\n",
        topology.target, update.port
    );
    let temporary = plan_pattern_change(
        context,
        isolated,
        project,
        "pattern-add-temporary",
        &temporary_request,
    )?;
    let temporary_route = temporary
        .output
        .iter()
        .find(|record| {
            record.operation == "identity" && field(record, "symbol") == Some("$temporary")
        })
        .and_then(|record| field(record, "id"))
        .ok_or_else(|| DevError::corrupt("temporary pattern plan omitted its allocated identity"))?
        .to_owned();
    let temporary_revision = apply_pattern_change(
        context,
        isolated,
        project,
        "pattern-add-temporary",
        &temporary,
    )?;
    let temporary_detail = records(
        &context
            .success(
                "inspect-temporary-pattern",
                &[
                    "--project",
                    path_text(project)?,
                    "inspect",
                    "owner",
                    "http_route",
                    &temporary_route,
                ],
                isolated,
            )?
            .bytes,
    )?;
    let temporary_owner = required_record(&temporary_detail, "owner")?;
    let temporary_pattern_inspected = field(temporary_owner, "selector") == Some("pattern")
        && field(temporary_owner, "path") == Some("/temporary/{id}")
        && field(temporary_owner, "captures") == Some("id")
        && field(temporary_owner, "signature") == Some("(HttpRequest,Text)->HttpResponse");
    if !temporary_pattern_inspected {
        return Err(DevError::corrupt(
            "temporary pattern inspection disagreed with its selector-indexed contract",
        ));
    }

    let deletion_request = format!(
        "request base={temporary_revision}\n\
         delete.owner owner={temporary_route} policy=reject\n"
    );
    let deletion = plan_pattern_change(
        context,
        isolated,
        project,
        "pattern-delete-temporary",
        &deletion_request,
    )?;
    let final_revision = apply_pattern_change(
        context,
        isolated,
        project,
        "pattern-delete-temporary",
        &deletion,
    )?;
    let inventory = records(
        &context
            .success(
                "pattern-final-inventory",
                &[
                    "--project",
                    path_text(project)?,
                    "query",
                    "owners",
                    "--kind",
                    "http_route",
                    "--limit",
                    "4096",
                    "--bytes",
                    "1048576",
                ],
                isolated,
            )?
            .bytes,
    )?;
    let temporary_pattern_deleted = field(required_record(&inventory, "summary")?, "returned")
        == Some("6")
        && !inventory.iter().any(|record| {
            record.operation == "owner" && field(record, "id") == Some(temporary_route.as_str())
        });
    if !temporary_pattern_deleted {
        return Err(DevError::corrupt(
            "deleted temporary pattern remained in the final route inventory",
        ));
    }

    Ok((
        final_revision,
        PatternLifecycleObservation {
            route: update.route.clone(),
            temporary_route,
            set_preserved_identity,
            altered_plan_rejected: true,
            stale_plan_rejected: true,
            reviewed_selector_evidence,
            temporary_pattern_inspected,
            temporary_pattern_deleted,
            intermediate_revisions: 4,
        },
    ))
}

fn run_live(
    context: &mut Context,
    isolated: &Path,
    project: &Path,
    artifact: &Path,
) -> Result<LiveObservation, DevError> {
    let authority_before = authority::observe_graph_authority(project)?;
    let descriptor = project.join("bbs.deployment.json");
    write_descriptor(&descriptor, artifact, "state/data")?;
    context.failure(
        "serve-absent-data-root",
        &["serve", "--deployment", path_text(&descriptor)?],
        isolated,
        BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
    )?;

    let corrupt_root = project.join("state/corrupt-data");
    fs::create_dir_all(&corrupt_root)?;
    evidence::publish(&corrupt_root.join("FORMAT"), b"foreign-data-format\n")?;
    let corrupt_descriptor = project.join("bbs-corrupt.deployment.json");
    write_descriptor(&corrupt_descriptor, artifact, "state/corrupt-data")?;
    context.failure(
        "serve-corrupt-data-root",
        &["serve", "--deployment", path_text(&corrupt_descriptor)?],
        isolated,
        BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
    )?;

    let data_root = project.join("state/data");
    context.success(
        "data-initialize",
        &["data", "initialize", "--root", path_text(&data_root)?],
        isolated,
    )?;
    let runner_environment = BTreeMap::from([("LANG".to_owned(), "C".to_owned())]);
    let (mut runner, address) = ActiveRunner::start(
        context,
        "service-first",
        &descriptor,
        isolated,
        runner_environment.clone(),
    )?;
    let mut requests = Vec::new();
    let first_result = exercise_before_restart(address, &mut requests);
    let first_stop = runner.stop();
    let created_identity = first_result?;
    let first_stop = first_stop?;
    let matcher_nodes = first_stop.matcher_nodes;

    context.success(
        "data-verify-after-first-service",
        &["data", "verify", "--root", path_text(&data_root)?],
        isolated,
    )?;
    let backup = isolated.join("bbs-data.backup");
    let backup_output = context.success(
        "data-backup",
        &[
            "data",
            "backup",
            "--root",
            path_text(&data_root)?,
            "--output",
            path_text(&backup)?,
        ],
        isolated,
    )?;
    let backup_records = records(&backup_output.bytes)?;
    let backup_digest =
        required_field(required_record(&backup_records, "backup")?, "digest")?.to_owned();

    let mut corrupt_backup_bytes = fs::read(&backup)?;
    let corrupt_byte = corrupt_backup_bytes
        .last_mut()
        .ok_or_else(|| DevError::corrupt("data backup is unexpectedly empty"))?;
    *corrupt_byte ^= 0x01;
    let corrupt_backup = isolated.join("bbs-data-corrupt.backup");
    evidence::publish(&corrupt_backup, &corrupt_backup_bytes)?;
    let rejected_root = project.join("state/rejected-data");
    context.failure(
        "data-restore-corrupt-backup",
        &[
            "data",
            "restore",
            "--backup",
            path_text(&corrupt_backup)?,
            "--root",
            path_text(&rejected_root)?,
        ],
        isolated,
        BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
    )?;
    if rejected_root.exists() {
        return Err(DevError::corrupt(
            "corrupt data backup made a destination visible",
        ));
    }

    let (mut restarted, restarted_address) = ActiveRunner::start(
        context,
        "service-restart",
        &descriptor,
        isolated,
        runner_environment.clone(),
    )?;
    exercise_persisted(restarted_address, &created_identity, &mut requests)?;
    let restarted_stop = restarted.stop()?;

    let restored_root = project.join("state/restored-data");
    context.success(
        "data-restore",
        &[
            "data",
            "restore",
            "--backup",
            path_text(&backup)?,
            "--root",
            path_text(&restored_root)?,
        ],
        isolated,
    )?;
    context.success(
        "data-verify-restored",
        &["data", "verify", "--root", path_text(&restored_root)?],
        isolated,
    )?;
    let restored_descriptor = project.join("bbs-restored.deployment.json");
    write_descriptor(&restored_descriptor, artifact, "state/restored-data")?;
    let (mut restored_runner, restored_address) = ActiveRunner::start(
        context,
        "service-restored",
        &restored_descriptor,
        isolated,
        runner_environment,
    )?;
    exercise_after_restart(restored_address, &created_identity, &mut requests)?;
    let restored_stop = restored_runner.stop()?;

    if matcher_nodes == 0
        || matcher_nodes != restarted_stop.matcher_nodes
        || matcher_nodes != restored_stop.matcher_nodes
    {
        return Err(DevError::corrupt(
            "stateful HTTP matcher node count was empty or changed across restart and restore",
        ));
    }
    let matcher_step_bound = matcher_nodes
        .checked_add(1)
        .ok_or_else(|| DevError::corrupt("stateful HTTP matcher step bound overflowed"))?;
    let runtime = aggregate_runtime(&[&first_stop, &restarted_stop, &restored_stop])?;

    let authority_after = authority::observe_graph_authority(project)?;
    let authority_unchanged = authority_before == authority_after;
    if !authority_unchanged {
        return Err(DevError::corrupt(
            "stateful HTTP runtime changed accepted graph authority",
        ));
    }
    Ok(LiveObservation {
        data_contract: "lkjscript-data-store-1".to_owned(),
        data_root: data_root.display().to_string(),
        matcher_nodes,
        matcher_step_bound,
        runtime,
        routes_checked: requests.len() as u64,
        created_identity,
        persistence_after_restart: true,
        backup_restore_equivalent: true,
        backup_digest,
        startup_failures_without_ready: 2,
        absent_root_no_ready: true,
        corrupt_root_no_ready: true,
        malformed_request_contained: true,
        exact_over_pattern_precedence: true,
        ordered_two_captures: true,
        capture_query_ignored: true,
        runner_starts: 3,
        shutdown_cleanup_failures: 0,
        temporary_data_cleanup_complete: true,
        authority_before,
        authority_after,
        authority_unchanged,
        requests,
    })
}

fn aggregate_runtime(
    observations: &[&HttpStopObservation],
) -> Result<ResidentRuntimeObservation, DevError> {
    let mut aggregate = ResidentRuntimeObservation::default();
    for observation in observations {
        let runtime = &observation.runtime;
        aggregate.runs = checked_runtime_sum(aggregate.runs, runtime.runs, "runs")?;
        aggregate.admitted_tasks = checked_runtime_sum(
            aggregate.admitted_tasks,
            runtime.admitted_tasks,
            "admitted tasks",
        )?;
        aggregate.completed_tasks = checked_runtime_sum(
            aggregate.completed_tasks,
            runtime.completed_tasks,
            "completed tasks",
        )?;
        aggregate.failed_tasks =
            checked_runtime_sum(aggregate.failed_tasks, runtime.failed_tasks, "failed tasks")?;
        aggregate.cancelled_tasks = checked_runtime_sum(
            aggregate.cancelled_tasks,
            runtime.cancelled_tasks,
            "cancelled tasks",
        )?;
        aggregate.overloaded_tasks = checked_runtime_sum(
            aggregate.overloaded_tasks,
            runtime.overloaded_tasks,
            "overloaded tasks",
        )?;
        aggregate.rejected_after_shutdown_tasks = checked_runtime_sum(
            aggregate.rejected_after_shutdown_tasks,
            runtime.rejected_after_shutdown_tasks,
            "tasks rejected after shutdown",
        )?;
        aggregate.maximum_queued_tasks = aggregate
            .maximum_queued_tasks
            .max(runtime.maximum_queued_tasks);
        aggregate.maximum_active_tasks = aggregate
            .maximum_active_tasks
            .max(runtime.maximum_active_tasks);
        aggregate.maximum_admission_permits = aggregate
            .maximum_admission_permits
            .max(runtime.maximum_admission_permits);
        aggregate.maximum_worker_permits = aggregate
            .maximum_worker_permits
            .max(runtime.maximum_worker_permits);
    }
    let runs = u64::try_from(observations.len())
        .map_err(|_| DevError::corrupt("stateful HTTP runner count is not representable"))?;
    if !aggregate.bounded_and_closed(runs) {
        return Err(DevError::corrupt(
            "stateful HTTP aggregate task or permit metrics are inconsistent",
        ));
    }
    Ok(aggregate)
}

fn checked_runtime_sum(left: u64, right: u64, label: &str) -> Result<u64, DevError> {
    left.checked_add(right)
        .ok_or_else(|| DevError::corrupt(format!("stateful HTTP {label} accounting overflowed")))
}

fn write_descriptor(path: &Path, artifact: &Path, data_root: &str) -> Result<(), DevError> {
    let artifact = artifact
        .strip_prefix(path.parent().ok_or_else(|| {
            DevError::infrastructure("stateful descriptor has no parent directory")
        })?)
        .map_err(|_| DevError::infrastructure("stateful artifact escaped its project"))?;
    let descriptor = json!({
        "artifact": path_text(artifact)?,
        "target": "serve",
        "listen": "127.0.0.1:0",
        "runtime": {
            "maximum_concurrent_tasks": MAXIMUM_CONCURRENT_TASKS,
            "maximum_queued_tasks": MAXIMUM_QUEUED_TASKS,
            "request_deadline_milliseconds": 30000,
            "shutdown_grace_milliseconds": 30000,
            "cancellation_grace_milliseconds": 5000
        },
        "execution": {
            "instruction_fuel": 10000000,
            "maximum_call_depth": 4096,
            "maximum_value_stack": 1000000
        },
        "http": {
            "maximum_request_body_bytes": 65536,
            "maximum_response_body_bytes": 1048576,
            "maximum_header_bytes": 32768,
            "maximum_headers": 128
        },
        "session": null,
        "worker": null,
        "streams": {
            "maximum_chunk_bytes": 65536,
            "maximum_buffered_chunks": 8,
            "maximum_total_bytes": 1048576,
            "maximum_live_streams": 64
        },
        "configuration": {},
        "secrets": [],
        "grants": [
            {
                "requirement": "streams",
                "sharing_domain": "bbs-streams",
                "authority_revision": "1111111111111111111111111111111111111111111111111111111111111111",
                "adapter": {"kind": "byte_stream"}
            },
            {
                "requirement": "data",
                "sharing_domain": "bbs-data",
                "authority_revision": "2222222222222222222222222222222222222222222222222222222222222222",
                "adapter": {
                    "kind": "data",
                    "root": data_root,
                    "namespace": "bbs",
                    "limits": {
                        "maximum_space_name_bytes": 128,
                        "maximum_key_parts": 16,
                        "maximum_key_bytes": 4096,
                        "maximum_value_bytes": 4194304,
                        "maximum_transaction_mutations": 4096,
                        "maximum_transaction_bytes": 16777216,
                        "maximum_scan_items": 10000,
                        "maximum_scan_bytes": 16777216,
                        "maximum_scan_work": 1000000,
                        "maximum_live_transactions": 1024
                    }
                }
            },
            {
                "requirement": "identifiers",
                "sharing_domain": "bbs-identifiers",
                "authority_revision": "3333333333333333333333333333333333333333333333333333333333333333",
                "adapter": {"kind": "identifier"}
            },
            {
                "requirement": "clock",
                "sharing_domain": "bbs-clock",
                "authority_revision": "4444444444444444444444444444444444444444444444444444444444444444",
                "adapter": {"kind": "wall_clock"}
            }
        ]
    });
    let mut bytes = serde_json::to_vec_pretty(&descriptor)?;
    bytes.push(b'\n');
    evidence::publish(path, &bytes)?;
    Ok(())
}

fn verify_discovery(context: &mut Context, cwd: &Path) -> Result<(), DevError> {
    let change = records(
        &context
            .success(
                "discover-change-grammar",
                &["capabilities", "--section", "change"],
                cwd,
            )?
            .bytes,
    )?;
    for name in [
        "add.dependency",
        "create.component",
        "add.requirement",
        "add.port",
        "create.target",
        "add.http-route",
    ] {
        require_named_record(&change, "change.operation", "name", name)?;
    }
    for name in ["pure", "task"] {
        require_named_record(&change, "change.function-effect", "name", name)?;
    }

    let types = records(
        &context
            .success(
                "discover-type-grammar",
                &["capabilities", "--section", "type"],
                cwd,
            )?
            .bytes,
    )?;
    require_named_record(&types, "type.form", "name", "structural-record")?;

    let expressions = records(
        &context
            .success(
                "discover-expression-grammar",
                &["capabilities", "--section", "expression"],
                cwd,
            )?
            .bytes,
    )?;
    for name in [
        "let",
        "record",
        "field",
        "list",
        "variant",
        "match",
        "capability-call",
        "transaction",
    ] {
        require_named_record(&expressions, "expression.form", "name", name)?;
    }

    let deployment = records(
        &context
            .success(
                "discover-deployment-schema",
                &["capabilities", "--section", "deployment"],
                cwd,
            )?
            .bytes,
    )?;
    require_named_record(&deployment, "deployment.adapter", "kind", "data")?;
    for path in [
        "adapter.data.root",
        "adapter.data.namespace",
        "adapter.data.limits",
    ] {
        require_named_record(&deployment, "deployment.adapter-field", "path", path)?;
    }
    if deployment.iter().any(|record| {
        field(record, "kind") == Some("postgres")
            || field(record, "path").is_some_and(|path| path.contains("postgres"))
    }) {
        return Err(DevError::corrupt(
            "deployment discovery retained a PostgreSQL production adapter",
        ));
    }

    let generated = cwd.join("discovered-guides");
    let result = records(
        &context
            .success(
                "generate-stateful-walkthrough",
                &["capabilities", "--generate-docs", path_text(&generated)?],
                cwd,
            )?
            .bytes,
    )?;
    if result
        .iter()
        .filter(|record| record.operation == "file")
        .count()
        != 8
    {
        return Err(DevError::corrupt(
            "generated guide discovery did not publish eight owned documents",
        ));
    }
    let walkthrough = fs::read(generated.join("stateful-http-authoring.md"))?;
    for required in [
        b"walkthrough.request".as_slice(),
        b"walkthrough.body".as_slice(),
        b"walkthrough.json".as_slice(),
        b"walkthrough.data".as_slice(),
        b"walkthrough.response".as_slice(),
        b"walkthrough.grant".as_slice(),
    ] {
        if !walkthrough
            .windows(required.len())
            .any(|window| window == required)
        {
            return Err(DevError::corrupt(
                "generated stateful HTTP walkthrough is incomplete",
            ));
        }
    }
    Ok(())
}

fn require_named_record(
    records: &[CompactRecord],
    operation: &str,
    field_name: &str,
    expected: &str,
) -> Result<(), DevError> {
    if records
        .iter()
        .any(|record| record.operation == operation && field(record, field_name) == Some(expected))
    {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "discovery omitted {operation}.{field_name}='{expected}'"
        )))
    }
}

fn discover_standard(
    context: &mut Context,
    cwd: &Path,
    package: String,
    semantic_revision: String,
    package_revision: String,
) -> Result<StandardReferences, DevError> {
    let mut references = StandardReferences {
        package,
        semantic_revision,
        package_revision,
        declarations: BTreeMap::new(),
        interfaces: BTreeMap::new(),
        operations: BTreeMap::new(),
        cases: BTreeMap::new(),
        fields: BTreeMap::new(),
    };
    for name in [
        "DataEntry",
        "DataExpectation",
        "DataKeyPart",
        "DataScanDirection",
        "DataScanItem",
        "DataScanPage",
        "DataSchema",
        "DataSchemaExpectation",
        "add",
        "bool-and",
        "bool-not",
        "bool-or",
        "bytes-concat",
        "bytes-equal",
        "bytes-from-text",
        "bytes-length",
        "data-decode-or",
        "data-encode",
        "i64-equal",
        "json-decode-or",
        "json-encode",
        "less",
        "less-equal",
        "list-fold-left",
        "list-get",
        "list-length",
        "text-empty",
        "text-equal",
        "text-length",
    ] {
        let owner = builtin_owner(context, cwd, None, name)?;
        references.declarations.insert(name.to_owned(), owner);
    }
    for name in ["DataEntry", "DataScanItem", "DataScanPage", "DataSchema"] {
        let owner = references
            .declarations
            .get(name)
            .ok_or_else(|| DevError::corrupt("discovered standard record vanished"))?
            .split('/')
            .next_back()
            .ok_or_else(|| DevError::corrupt("standard record reference is invalid"))?
            .to_owned();
        let detail = records(
            &context
                .success(
                    &format!("builtin-inspect-{name}"),
                    &["package", "builtin", "inspect", "owner", "record", &owner],
                    cwd,
                )?
                .bytes,
        )?;
        for record in detail.iter().filter(|record| record.operation == "owner") {
            if field(record, "kind") == Some("field") {
                references.fields.insert(
                    format!("{name}.{}", required_field(record, "name")?),
                    required_field(record, "reference")?.to_owned(),
                );
            }
        }
    }
    for name in ["ByteStream", "DataStore", "Identifier", "WallClock"] {
        let owner = builtin_owner(context, cwd, Some("interface"), name)?;
        references.interfaces.insert(name.to_owned(), owner.clone());
        let identity = owner
            .split('/')
            .next_back()
            .ok_or_else(|| DevError::corrupt("standard interface reference is invalid"))?;
        let detail = records(
            &context
                .success(
                    &format!("builtin-inspect-{name}"),
                    &[
                        "package",
                        "builtin",
                        "inspect",
                        "owner",
                        "interface",
                        identity,
                    ],
                    cwd,
                )?
                .bytes,
        )?;
        for record in detail.iter().filter(|record| record.operation == "owner") {
            if field(record, "kind") == Some("operation") {
                let operation = required_field(record, "name")?;
                let reference = required_field(record, "reference")?;
                references
                    .operations
                    .insert(format!("{name}.{operation}"), reference.to_owned());
            }
        }
    }
    for name in [
        "DataExpectation",
        "DataKeyPart",
        "DataScanDirection",
        "DataSchemaExpectation",
    ] {
        let owner = references
            .declarations
            .get(name)
            .ok_or_else(|| DevError::corrupt("discovered standard declaration vanished"))?
            .split('/')
            .next_back()
            .ok_or_else(|| DevError::corrupt("standard declaration reference is invalid"))?
            .to_owned();
        let detail = records(
            &context
                .success(
                    &format!("builtin-inspect-{name}"),
                    &["package", "builtin", "inspect", "owner", "variant", &owner],
                    cwd,
                )?
                .bytes,
        )?;
        for record in detail.iter().filter(|record| record.operation == "owner") {
            if field(record, "kind") == Some("case") {
                references.cases.insert(
                    format!("{name}.{}", required_field(record, "name")?),
                    required_field(record, "reference")?.to_owned(),
                );
            }
        }
    }
    Ok(references)
}

fn builtin_owner(
    context: &mut Context,
    cwd: &Path,
    kind: Option<&str>,
    name: &str,
) -> Result<String, DevError> {
    let mut arguments = vec!["package", "builtin", "query", "owners"];
    if let Some(kind) = kind {
        arguments.extend(["--kind", kind]);
    }
    arguments.extend(["--name", name]);
    let output = records(
        &context
            .success(&format!("builtin-query-{name}"), &arguments, cwd)?
            .bytes,
    )?;
    let owners = output
        .iter()
        .filter(|record| record.operation == "owner")
        .collect::<Vec<_>>();
    if owners.len() != 1 {
        return Err(DevError::corrupt(format!(
            "built-in discovery found {} owners named '{name}'",
            owners.len()
        )));
    }
    Ok(required_field(owners[0], "reference")?.to_owned())
}

fn discover_authored_topology(
    context: &mut Context,
    cwd: &Path,
    project: &Path,
) -> Result<TopologyObservation, DevError> {
    let module = discover_project_owner(context, cwd, project, "module", "bbs", None)?;
    let component = discover_project_owner(
        context,
        cwd,
        project,
        "component",
        "application",
        Some(&module),
    )?;
    let mut requirements = BTreeMap::new();
    for name in ["streams", "data", "identifiers", "clock"] {
        let owner =
            discover_project_owner(context, cwd, project, "requirement", name, Some(&component))?;
        requirements.insert(name.to_owned(), owner);
    }
    let target = discover_project_owner(context, cwd, project, "target", "serve", Some("package"))?;

    let target_relations = project_relations(context, cwd, project, &target)?;
    require_relation(&target_relations, "target_component", &target, &component)?;
    if target_relations
        .iter()
        .any(|record| field(record, "kind") == Some("target_port"))
    {
        return Err(DevError::corrupt(
            "HTTP target retained predecessor target_port authority",
        ));
    }

    let target_detail = records(
        &context
            .success(
                "inspect-http-target",
                &[
                    "--project",
                    path_text(project)?,
                    "inspect",
                    "owner",
                    "target",
                    &target,
                ],
                cwd,
            )?
            .bytes,
    )?;
    let target_owner = required_record(&target_detail, "owner")?;
    if required_field(target_owner, "route-count")? != "6" {
        return Err(DevError::corrupt(
            "HTTP target inspection did not report six routes",
        ));
    }
    let exact_routes = parse_u64(
        required_field(target_owner, "exact-routes")?,
        "exact route count",
    )?;
    let pattern_routes = parse_u64(
        required_field(target_owner, "pattern-routes")?,
        "pattern route count",
    )?;
    let pattern_segments = parse_u64(
        required_field(target_owner, "pattern-segments")?,
        "pattern segment count",
    )?;
    let maximum_specificity_chain = parse_u64(
        required_field(target_owner, "maximum-specificity-chain")?,
        "maximum specificity chain",
    )?;
    if exact_routes != 4
        || pattern_routes != 2
        || pattern_segments != 6
        || maximum_specificity_chain != 2
    {
        return Err(DevError::corrupt(
            "HTTP target selector counts or specificity chain changed",
        ));
    }
    let route_set = required_field(target_owner, "route-set")?.to_owned();

    let route_inventory = records(
        &context
            .success(
                "project-http-routes",
                &[
                    "--project",
                    path_text(project)?,
                    "query",
                    "owners",
                    "--kind",
                    "http_route",
                    "--limit",
                    "4096",
                    "--bytes",
                    "1048576",
                ],
                cwd,
            )?
            .bytes,
    )?;
    require_record_field(&route_inventory, "summary", "truncated", "false")?;
    let route_ids = route_inventory
        .iter()
        .filter(|record| record.operation == "owner" && field(record, "kind") == Some("http_route"))
        .map(|record| required_field(record, "id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    if route_ids.len() != 6 {
        return Err(DevError::corrupt(format!(
            "authored project has {} HTTP routes instead of six",
            route_ids.len()
        )));
    }

    let expected = [
        (
            "GET",
            "/",
            "exact",
            &[][..],
            "home",
            "handle-home",
            &["request"][..],
        ),
        (
            "GET",
            "/api/posts",
            "exact",
            &[][..],
            "list",
            "handle-list-route",
            &["request"][..],
        ),
        (
            "POST",
            "/api/posts",
            "exact",
            &[][..],
            "create",
            "handle-create-route",
            &["request"][..],
        ),
        (
            "PUT",
            "/api/posts/{id}",
            "pattern",
            &["id"][..],
            "update",
            "handle-update-route",
            &["request", "id"][..],
        ),
        (
            "DELETE",
            "/api/{space}/{id}",
            "pattern",
            &["space", "id"][..],
            "delete",
            "handle-delete-route",
            &["request", "space", "id"][..],
        ),
        (
            "DELETE",
            "/api/posts/featured",
            "exact",
            &[][..],
            "featured",
            "handle-featured-route",
            &["request"][..],
        ),
    ];
    let mut routes = Vec::with_capacity(expected.len());
    for (method, path, selector, captures, port_name, handler_name, parameter_names) in expected {
        let port =
            discover_project_owner(context, cwd, project, "port", port_name, Some(&component))?;
        let handler = discover_project_owner(
            context,
            cwd,
            project,
            "task_function",
            handler_name,
            Some(&module),
        )?;
        let parameters = inspect_http_handler_parameters(context, cwd, project, &handler)?;
        if parameters
            .iter()
            .map(|parameter| parameter.name.as_str())
            .ne(parameter_names.iter().copied())
        {
            return Err(DevError::corrupt(format!(
                "HTTP handler {handler_name} parameter order does not match its route captures"
            )));
        }
        let port_relations = project_relations(context, cwd, project, &port)?;
        require_relation(&port_relations, "member_declaration", &port, &component)?;
        require_relation(&port_relations, "function_value", &port, &handler)?;

        let mut matching = Vec::new();
        for route in &route_ids {
            let detail = records(
                &context
                    .success(
                        &format!("inspect-http-route-{route}"),
                        &[
                            "--project",
                            path_text(project)?,
                            "inspect",
                            "owner",
                            "http_route",
                            route,
                        ],
                        cwd,
                    )?
                    .bytes,
            )?;
            let owner = required_record(&detail, "owner")?;
            if field(owner, "method") == Some(method) && field(owner, "path") == Some(path) {
                if field(owner, "target") != Some(target.as_str())
                    || !required_field(owner, "component")?.ends_with(&format!("/{component}"))
                    || !required_field(owner, "port")?.ends_with(&format!("/{port}"))
                    || field(owner, "selector") != Some(selector)
                    || split_http_captures(required_field(owner, "captures")?) != captures
                    || field(owner, "handler").is_none_or(|value| !value.ends_with(&handler))
                {
                    return Err(DevError::corrupt(format!(
                        "HTTP route {method} {path} inspection disagrees with its selector, captures, handler, target, component, or port"
                    )));
                }
                let expected_signature = if captures.is_empty() {
                    "(HttpRequest)->HttpResponse".to_owned()
                } else {
                    format!(
                        "(HttpRequest,{})->HttpResponse",
                        std::iter::repeat_n("Text", captures.len())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                };
                if field(owner, "signature") != Some(expected_signature.as_str()) {
                    return Err(DevError::corrupt(format!(
                        "HTTP route {method} {path} has signature drift"
                    )));
                }
                matching.push((route.clone(), required_field(owner, "port")?.to_owned()));
            }
        }
        if matching.len() != 1 {
            return Err(DevError::corrupt(format!(
                "authored project has {} routes for selector key {method} {path}",
                matching.len()
            )));
        }
        let (route, route_port) = matching.remove(0);
        let route_relations = project_relations(context, cwd, project, &route)?;
        require_relation(&route_relations, "http_route_target", &route, &target)?;
        require_relation(&route_relations, "http_route_port", &route, &port)?;
        routes.push(HttpRouteObservation {
            route,
            method: method.to_owned(),
            selector: selector.to_owned(),
            path: path.to_owned(),
            captures: captures
                .iter()
                .map(|capture| (*capture).to_owned())
                .collect(),
            port: route_port,
            handler,
            signature: if captures.is_empty() {
                "(HttpRequest)->HttpResponse".to_owned()
            } else {
                format!(
                    "(HttpRequest,{})->HttpResponse",
                    std::iter::repeat_n("Text", captures.len())
                        .collect::<Vec<_>>()
                        .join(",")
                )
            },
            parameters,
        });
    }
    routes.sort_by(|left, right| (&left.method, &left.path).cmp(&(&right.method, &right.path)));

    Ok(TopologyObservation {
        module,
        component,
        requirements,
        routes,
        target,
        target_name: "serve".to_owned(),
        runner: "http".to_owned(),
        exact_routes,
        pattern_routes,
        pattern_segments,
        maximum_specificity_chain,
        route_set,
    })
}

fn split_http_captures(value: &str) -> Vec<&str> {
    if value.is_empty() {
        Vec::new()
    } else {
        value.split(',').collect()
    }
}

fn inspect_http_handler_parameters(
    context: &mut Context,
    cwd: &Path,
    project: &Path,
    handler: &str,
) -> Result<Vec<HttpRouteParameterObservation>, DevError> {
    let output = records(
        &context
            .success(
                &format!("inspect-http-handler-{handler}"),
                &[
                    "--project",
                    path_text(project)?,
                    "inspect",
                    "owner",
                    "task_function",
                    handler,
                    "--detail",
                    "definition",
                    "--limit",
                    "64",
                    "--bytes",
                    "65536",
                ],
                cwd,
            )?
            .bytes,
    )?;
    require_record_field(&output, "result", "status", "success")?;
    require_record_field(&output, "page", "start", "0")?;
    let function = required_record(&output, "definition.function")?;
    if required_field(function, "id")? != handler
        || required_field(function, "kind")? != "task_function"
    {
        return Err(DevError::corrupt(
            "HTTP handler definition projection selected another function",
        ));
    }
    let expected = parse_u64(
        required_field(function, "parameters")?,
        "HTTP handler parameter count",
    )?;
    let mut parameters = output
        .iter()
        .filter(|record| record.operation == "definition.parameter")
        .map(|record| {
            if required_field(record, "parent")? != handler {
                return Err(DevError::corrupt(
                    "HTTP handler definition exposed a foreign parameter",
                ));
            }
            Ok(HttpRouteParameterObservation {
                id: required_field(record, "id")?.to_owned(),
                index: parse_u64(
                    required_field(record, "index")?,
                    "HTTP handler parameter index",
                )?,
                name: required_field(record, "name")?.to_owned(),
                ty: required_field(record, "type")?.to_owned(),
                use_mode: required_field(record, "use")?.to_owned(),
                requirement: required_field(record, "requirement")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>, DevError>>()?;
    parameters.sort_by_key(|parameter| parameter.index);
    if parameters.len() as u64 != expected
        || parameters.iter().enumerate().any(|(index, parameter)| {
            parameter.index != index as u64
                || parameter.use_mode != "unrestricted"
                || parameter.requirement != "none"
        })
    {
        return Err(DevError::corrupt(
            "HTTP handler parameters are missing, reordered, resource-bound, or non-unrestricted",
        ));
    }
    Ok(parameters)
}

fn discover_project_owner(
    context: &mut Context,
    cwd: &Path,
    project: &Path,
    kind: &str,
    name: &str,
    expected_parent: Option<&str>,
) -> Result<String, DevError> {
    let output = records(
        &context
            .success(
                &format!("project-owner-{kind}-{name}"),
                &[
                    "--project",
                    path_text(project)?,
                    "query",
                    "owners",
                    "--kind",
                    kind,
                    "--limit",
                    "4096",
                    "--bytes",
                    "1048576",
                ],
                cwd,
            )?
            .bytes,
    )?;
    require_record_field(&output, "summary", "truncated", "false")?;
    let matches = output
        .iter()
        .filter(|record| {
            record.operation == "owner"
                && field(record, "kind") == Some(kind)
                && field(record, "name") == Some(name)
                && expected_parent.is_none_or(|parent| field(record, "parent") == Some(parent))
        })
        .collect::<Vec<_>>();
    if matches.len() != 1 {
        return Err(DevError::corrupt(format!(
            "authored project has {} {kind} owners named '{name}' with the expected parent",
            matches.len()
        )));
    }
    Ok(required_field(matches[0], "id")?.to_owned())
}

fn project_relations(
    context: &mut Context,
    cwd: &Path,
    project: &Path,
    owner: &str,
) -> Result<Vec<CompactRecord>, DevError> {
    let output = records(
        &context
            .success(
                &format!("project-relations-{owner}"),
                &[
                    "--project",
                    path_text(project)?,
                    "query",
                    "relations",
                    owner,
                    "--direction",
                    "outgoing",
                    "--limit",
                    "4096",
                    "--bytes",
                    "1048576",
                ],
                cwd,
            )?
            .bytes,
    )?;
    require_record_field(&output, "summary", "truncated", "false")?;
    Ok(output)
}

fn require_relation(
    records: &[CompactRecord],
    kind: &str,
    source: &str,
    target: &str,
) -> Result<(), DevError> {
    if records.iter().any(|record| {
        record.operation == "relation"
            && field(record, "kind") == Some(kind)
            && field(record, "source-owner") == Some(source)
            && field(record, "target-owner") == Some(target)
    }) {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "authored topology omitted {kind} relation {source} -> {target}"
        )))
    }
}

struct ActiveRunner {
    name: String,
    control: ProcessControl,
    receiver: Receiver<ProcessObservation>,
    thread: Option<thread::JoinHandle<()>>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
}

impl ActiveRunner {
    fn start(
        context: &mut Context,
        name: &str,
        descriptor: &Path,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(Self, SocketAddr), DevError> {
        let stdout_path = context.evidence.join(format!("runner-{name}.stdout.log"));
        let stderr_path = context.evidence.join(format!("runner-{name}.stderr.log"));
        let command = vec![
            context.binary.display().to_string(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor.display().to_string(),
        ];
        let specification = ProcessSpec {
            command,
            cwd: cwd.to_path_buf(),
            environment,
            timeout: RUNNER_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: None,
        };
        let observation_root = context.observation_root.clone();
        let control = ProcessControl::default();
        let child_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("stateful-http-{name}"))
            .spawn(move || {
                let observation =
                    process::run_controlled(&specification, &observation_root, &child_control);
                let _ = sender.send(observation);
            })
            .map_err(|error| DevError::infrastructure(format!("start runner thread: {error}")))?;
        let mut runner = Self {
            name: name.to_owned(),
            control,
            receiver,
            thread: Some(thread),
            stdout_path,
            stderr_path,
        };
        match runner.wait_ready() {
            Ok(address) => Ok((runner, address)),
            Err(error) => {
                let _ = runner.kill();
                Err(error)
            }
        }
    }

    fn wait_ready(&mut self) -> Result<SocketAddr, DevError> {
        let started = Instant::now();
        loop {
            if let Some(line) = first_line(&self.stdout_path)? {
                let value: Value = serde_json::from_slice(&line)?;
                if value.get("event").and_then(Value::as_str) != Some("ready")
                    || value.get("ok").and_then(Value::as_bool) != Some(true)
                {
                    return Err(DevError::corrupt(format!(
                        "stateful runner '{}' emitted a non-ready first event",
                        self.name
                    )));
                }
                return value
                    .get("local_address")
                    .and_then(Value::as_str)
                    .ok_or_else(|| DevError::corrupt("ready event omitted local_address"))?
                    .parse()
                    .map_err(|error| {
                        DevError::corrupt(format!("ready address is invalid: {error}"))
                    });
            }
            match self.receiver.try_recv() {
                Ok(observation) => {
                    return Err(DevError::corrupt(format!(
                        "stateful runner '{}' exited {:?} before readiness; inspect {}",
                        self.name,
                        observation.status,
                        self.stdout_path.display()
                    )));
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Err(DevError::infrastructure(
                        "stateful runner observation channel disconnected",
                    ));
                }
            }
            if started.elapsed() >= READY_TIMEOUT {
                return Err(DevError::corrupt(format!(
                    "stateful runner '{}' omitted readiness",
                    self.name
                )));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn stop(&mut self) -> Result<HttpStopObservation, DevError> {
        self.control.interrupt();
        let observation = match self.receiver.recv_timeout(STOP_TIMEOUT) {
            Ok(observation) => observation,
            Err(_) => {
                self.control.kill();
                self.receiver
                    .recv_timeout(Duration::from_secs(5))
                    .map_err(|error| {
                        DevError::infrastructure(format!("stop stateful runner: {error}"))
                    })?
            }
        };
        self.join()?;
        if observation.status != ProcessStatus::Passed {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' stopped as {:?}",
                self.name, observation.status
            )));
        }
        let stdout = process::read_bounded(&self.stdout_path, MAXIMUM_OUTPUT_BYTES)?;
        let stopped = stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .find_map(|line| {
                serde_json::from_slice::<Value>(line)
                    .ok()
                    .filter(|value| value.get("event").and_then(Value::as_str) == Some("stopped"))
            })
            .ok_or_else(|| {
                DevError::corrupt(format!(
                    "stateful runner '{}' omitted its stopped receipt",
                    self.name
                ))
            })?;
        let receipt = stopped
            .get("receipt")
            .ok_or_else(|| DevError::corrupt("stateful stopped event omitted its receipt"))?;
        let matcher_nodes = receipt
            .get("matcher_nodes")
            .and_then(Value::as_u64)
            .ok_or_else(|| DevError::corrupt("stateful stopped receipt omitted matcher nodes"))?;
        let runtime = receipt
            .get("runtime")
            .ok_or_else(|| DevError::corrupt("stateful stopped receipt omitted runtime metrics"))?;
        let resident = runtime.get("resident").ok_or_else(|| {
            DevError::corrupt("stateful stopped receipt omitted resident metrics")
        })?;
        let runtime_observation = ResidentRuntimeObservation {
            runs: 1,
            admitted_tasks: runtime_u64(resident, "admitted")?,
            completed_tasks: runtime_u64(resident, "completed")?,
            failed_tasks: runtime_u64(resident, "failed")?,
            cancelled_tasks: runtime_u64(resident, "cancelled")?,
            overloaded_tasks: runtime_u64(resident, "overloaded")?,
            rejected_after_shutdown_tasks: runtime_u64(resident, "rejected_after_shutdown")?,
            maximum_queued_tasks: runtime_u64(resident, "maximum_queued")?,
            maximum_active_tasks: runtime_u64(resident, "maximum_active")?,
            maximum_admission_permits: runtime_u64(runtime, "maximum_admission_permits")?,
            maximum_worker_permits: runtime_u64(runtime, "maximum_worker_permits")?,
        };
        if resident.get("accepting").and_then(Value::as_bool) != Some(false)
            || runtime_u64(resident, "queued")? != 0
            || runtime_u64(resident, "active")? != 0
            || runtime_u64(runtime, "admission_permits")? != 0
            || runtime_u64(runtime, "worker_permits")? != 0
            || runtime_observation.admitted_tasks == 0
            || runtime_observation.admitted_tasks != runtime_observation.completed_tasks
            || runtime_observation.cancelled_tasks > runtime_observation.failed_tasks
            || runtime_observation.maximum_queued_tasks == 0
            || runtime_observation.maximum_queued_tasks > MAXIMUM_QUEUED_TASKS
            || runtime_observation.maximum_active_tasks == 0
            || runtime_observation.maximum_active_tasks > MAXIMUM_CONCURRENT_TASKS
            || runtime_observation.maximum_admission_permits
                < runtime_observation.maximum_queued_tasks
            || runtime_observation.maximum_admission_permits
                < runtime_observation.maximum_worker_permits
            || runtime_observation.maximum_admission_permits
                > MAXIMUM_CONCURRENT_TASKS + MAXIMUM_QUEUED_TASKS
            || runtime_observation.maximum_worker_permits < runtime_observation.maximum_active_tasks
            || runtime_observation.maximum_worker_permits > MAXIMUM_CONCURRENT_TASKS
        {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' emitted inconsistent task or permit metrics",
                self.name
            )));
        }
        let shutdown = receipt
            .get("shutdown")
            .ok_or_else(|| DevError::corrupt("stateful stopped receipt omitted shutdown"))?;
        if shutdown.get("admission_stopped").and_then(Value::as_bool) != Some(true)
            || shutdown.get("remaining_tasks").and_then(Value::as_u64) != Some(0)
            || shutdown
                .get("cleanup_failures")
                .and_then(Value::as_array)
                .is_none_or(|failures| !failures.is_empty())
        {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' retained admission, tasks, or cleanup failures",
                self.name
            )));
        }
        let stderr = process::read_bounded(&self.stderr_path, MAXIMUM_OUTPUT_BYTES)?;
        if !stderr.is_empty() {
            return Err(DevError::corrupt(format!(
                "stateful runner '{}' wrote to stderr",
                self.name
            )));
        }
        Ok(HttpStopObservation {
            matcher_nodes,
            runtime: runtime_observation,
        })
    }

    fn kill(&mut self) -> Result<(), DevError> {
        self.control.kill();
        let _ = self.receiver.recv_timeout(Duration::from_secs(5));
        self.join()
    }

    fn join(&mut self) -> Result<(), DevError> {
        if let Some(thread) = self.thread.take() {
            thread
                .join()
                .map_err(|_| DevError::infrastructure("stateful runner thread panicked"))?;
        }
        Ok(())
    }
}

fn runtime_u64(value: &Value, field: &str) -> Result<u64, DevError> {
    value
        .get(field)
        .and_then(Value::as_u64)
        .ok_or_else(|| DevError::corrupt(format!("stateful runtime metric '{field}' is absent")))
}

impl Drop for ActiveRunner {
    fn drop(&mut self) {
        if self.thread.is_some() {
            self.control.kill();
            let _ = self.receiver.recv_timeout(Duration::from_secs(5));
            if let Some(thread) = self.thread.take() {
                let _ = thread.join();
            }
        }
    }
}

fn exercise_before_restart(
    address: SocketAddr,
    observations: &mut Vec<HttpObservation>,
) -> Result<String, DevError> {
    let root = request(observations, address, "root", "GET", "/", b"", &[])?;
    require_http(&root, 200, Some("text/html; charset=utf-8"))?;
    if !root.body.starts_with(b"<!doctype html>") {
        return Err(DevError::corrupt(
            "BBS root did not return graph-owned HTML",
        ));
    }
    let featured = request(
        observations,
        address,
        "exact-over-pattern",
        "DELETE",
        "/api/posts/featured?query=does-not-select",
        b"",
        &[],
    )?;
    require_http(&featured, 200, Some("text/plain; charset=utf-8"))?;
    if featured.body != b"featured-exact" {
        return Err(DevError::corrupt(
            "exact HTTP route did not win over its two-capture pattern",
        ));
    }
    let ordered_captures = request(
        observations,
        address,
        "ordered-two-captures",
        "DELETE",
        "/api/not-posts/00000000-0000-4000-8000-000000000000",
        b"",
        &[],
    )?;
    require_http(&ordered_captures, 404, Some("application/json"))?;
    let list = request(
        observations,
        address,
        "list-empty",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&list, 200, Some("application/json"))?;
    require_json_array_len(&list.body, 0)?;

    for (name, body) in [
        ("malformed-json", b"{".as_slice()),
        (
            "trailing-json",
            b"{\"author\":\"agent\",\"body\":\"first\"} trailing".as_slice(),
        ),
        (
            "duplicate-json-field",
            b"{\"author\":\"agent\",\"author\":\"other\",\"body\":\"first\"}".as_slice(),
        ),
    ] {
        let response = request(
            observations,
            address,
            name,
            "POST",
            "/api/posts",
            body,
            &[("Content-Type", "application/json")],
        )?;
        require_http(&response, 400, Some("application/json"))?;
    }
    let missing_header = request(
        observations,
        address,
        "missing-content-type",
        "POST",
        "/api/posts",
        b"{\"author\":\"agent\",\"body\":\"first\"}",
        &[],
    )?;
    require_http(&missing_header, 400, Some("application/json"))?;

    let nonmatching_header = request(
        observations,
        address,
        "nonmatching-content-type",
        "POST",
        "/api/posts",
        b"{\"author\":\"agent\",\"body\":\"header-probe\"}",
        &[("Content-Type", "text/plain")],
    )?;
    require_http(&nonmatching_header, 400, Some("application/json"))?;

    for (name, headers) in [
        (
            "matching-content-type-after-nonmatch",
            [
                ("Content-Type", "text/plain"),
                ("X-Header-Order", "first"),
                ("Content-Type", "application/json"),
            ],
        ),
        (
            "matching-content-type-before-nonmatch",
            [
                ("Content-Type", "application/json"),
                ("X-Header-Order", "second"),
                ("Content-Type", "text/plain"),
            ],
        ),
    ] {
        let admitted = request(
            observations,
            address,
            name,
            "POST",
            "/api/posts",
            b"{\"author\":\"agent\",\"body\":\"header-probe\"}",
            &headers,
        )?;
        require_http(&admitted, 201, Some("application/json"))?;
        let value: Value = serde_json::from_slice(&admitted.body)?;
        let identity = value
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| DevError::corrupt("header probe omitted its post identity"))?;
        let deleted = request(
            observations,
            address,
            &format!("delete-{name}"),
            "DELETE",
            &format!("/api/posts/{identity}"),
            b"",
            &[],
        )?;
        require_http(&deleted, 204, None)?;
    }

    let created = request(
        observations,
        address,
        "create",
        "POST",
        "/api/posts",
        b"{\"author\":\"agent\",\"body\":\"first\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&created, 201, Some("application/json"))?;
    let created_value: Value = serde_json::from_slice(&created.body)?;
    let identity = created_value
        .get("id")
        .and_then(Value::as_str)
        .filter(|value| value.len() == 36)
        .ok_or_else(|| DevError::corrupt("created BBS post omitted its semantic identity"))?
        .to_owned();
    if created_value.get("author").and_then(Value::as_str) != Some("agent")
        || created_value.get("body").and_then(Value::as_str) != Some("first")
    {
        return Err(DevError::corrupt(
            "created BBS post disagrees with its request",
        ));
    }
    let list = request(
        observations,
        address,
        "list-created",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_json_array_len(&list.body, 1)?;

    let missing_capture = request(
        observations,
        address,
        "update-missing-capture",
        "PUT",
        "/api/posts",
        b"{\"author\":\"agent\",\"body\":\"updated\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&missing_capture, 404, None)?;
    let malformed_capture = request(
        observations,
        address,
        "update-malformed-capture",
        "PUT",
        "/api/posts/bad",
        b"{\"author\":\"agent\",\"body\":\"updated\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&malformed_capture, 400, Some("application/json"))?;
    let absent = request(
        observations,
        address,
        "update-absent",
        "PUT",
        "/api/posts/00000000-0000-4000-8000-000000000000",
        b"{\"author\":\"agent\",\"body\":\"updated\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&absent, 404, Some("application/json"))?;
    let updated = request(
        observations,
        address,
        "update",
        "PUT",
        &format!("/api/posts/{identity}?id=ignored&id=also-ignored"),
        b"{\"author\":\"agent-two\",\"body\":\"updated\"}",
        &[("Content-Type", "application/json")],
    )?;
    require_http(&updated, 200, Some("application/json"))?;
    let updated_value: Value = serde_json::from_slice(&updated.body)?;
    if updated_value.get("body").and_then(Value::as_str) != Some("updated") {
        return Err(DevError::corrupt(
            "BBS update did not return the updated post",
        ));
    }
    Ok(identity)
}

fn exercise_persisted(
    address: SocketAddr,
    identity: &str,
    observations: &mut Vec<HttpObservation>,
) -> Result<(), DevError> {
    let list = request(
        observations,
        address,
        "list-after-restart",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&list, 200, Some("application/json"))?;
    let value: Value = serde_json::from_slice(&list.body)?;
    let post = value
        .as_array()
        .and_then(|items| items.first())
        .ok_or_else(|| DevError::corrupt("BBS post did not persist across restart"))?;
    if post.get("id").and_then(Value::as_str) != Some(identity)
        || post.get("body").and_then(Value::as_str) != Some("updated")
    {
        return Err(DevError::corrupt(
            "BBS persisted value disagrees after restart",
        ));
    }
    Ok(())
}

fn exercise_after_restart(
    address: SocketAddr,
    identity: &str,
    observations: &mut Vec<HttpObservation>,
) -> Result<(), DevError> {
    exercise_persisted(address, identity, observations)?;
    let unsupported = request(
        observations,
        address,
        "unsupported-method",
        "PATCH",
        "/api/posts",
        b"",
        &[],
    )?;
    require_http(&unsupported, 404, None)?;
    if !unsupported.body.is_empty() {
        return Err(DevError::corrupt(
            "unmatched BBS method returned an application body",
        ));
    }
    let unknown = request(
        observations,
        address,
        "unknown-route",
        "GET",
        "/unknown",
        b"",
        &[],
    )?;
    require_http(&unknown, 404, None)?;
    if !unknown.body.is_empty() {
        return Err(DevError::corrupt(
            "unmatched BBS path returned an application body",
        ));
    }
    let deleted = request(
        observations,
        address,
        "delete",
        "DELETE",
        &format!("/api/posts/{identity}"),
        b"",
        &[],
    )?;
    require_http(&deleted, 204, None)?;
    if !deleted.body.is_empty() {
        return Err(DevError::corrupt("204 BBS response contained a body"));
    }
    let missing = request(
        observations,
        address,
        "delete-absent",
        "DELETE",
        &format!("/api/posts/{identity}"),
        b"",
        &[],
    )?;
    require_http(&missing, 404, Some("application/json"))?;
    let final_list = request(
        observations,
        address,
        "list-after-delete",
        "GET",
        "/api/posts",
        b"",
        &[],
    )?;
    require_json_array_len(&final_list.body, 0)
}

fn request(
    observations: &mut Vec<HttpObservation>,
    address: SocketAddr,
    name: &str,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<http_probe::HttpResponse, DevError> {
    let response = http_probe::request(address, method, path, body, headers)?;
    observations.push(HttpObservation {
        name: name.to_owned(),
        method: method.to_owned(),
        path: path.split('?').next().unwrap_or(path).to_owned(),
        status: response.status,
        request_bytes: body.len() as u64,
        response_bytes: response.body.len() as u64,
        response_sha256: sha256(&response.body),
        elapsed_nanoseconds: response.elapsed_nanoseconds,
    });
    Ok(response)
}

fn require_http(
    response: &http_probe::HttpResponse,
    status: u16,
    content_type: Option<&str>,
) -> Result<(), DevError> {
    if response.status != status {
        return Err(DevError::corrupt(format!(
            "BBS response status was {}, expected {status}; failure code was {:?}",
            response.status,
            response.headers.get("x-lkjscript-failure-code")
        )));
    }
    if let Some(expected) = content_type
        && response.headers.get("content-type").map(String::as_str) != Some(expected)
    {
        return Err(DevError::corrupt(format!(
            "BBS response content type was {:?}, expected '{expected}'",
            response.headers.get("content-type")
        )));
    }
    Ok(())
}

fn require_json_array_len(bytes: &[u8], expected: usize) -> Result<(), DevError> {
    let value: Value = serde_json::from_slice(bytes)?;
    if value.as_array().map(Vec::len) == Some(expected) {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "BBS JSON list does not contain {expected} items"
        )))
    }
}

fn first_line(path: &Path) -> Result<Option<Vec<u8>>, DevError> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes
            .iter()
            .position(|byte| *byte == b'\n')
            .map(|end| bytes[..end].to_vec())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

impl Context {
    fn success<S: AsRef<str>>(
        &mut self,
        name: &str,
        arguments: &[S],
        cwd: &Path,
    ) -> Result<Output, DevError> {
        let mut command = vec![self.binary.display().to_string()];
        command.extend(arguments.iter().map(|value| value.as_ref().to_owned()));
        let (observation, output) = self.observe_command(
            name,
            command,
            cwd,
            BTreeMap::from([("LANG".to_owned(), "C".to_owned())]),
        )?;
        self.require_status(name, observation, ProcessStatus::Passed)?;
        Ok(output)
    }

    fn failure<S: AsRef<str>>(
        &mut self,
        name: &str,
        arguments: &[S],
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<Output, DevError> {
        let mut command = vec![self.binary.display().to_string()];
        command.extend(arguments.iter().map(|value| value.as_ref().to_owned()));
        let (observation, output) = self.observe_command(name, command, cwd, environment)?;
        if observation.status == ProcessStatus::Passed {
            return Err(DevError::corrupt(format!(
                "stateful HTTP negative step '{name}' unexpectedly succeeded"
            )));
        }
        if output
            .bytes
            .windows(b"\"event\":\"ready\"".len())
            .any(|value| value == b"\"event\":\"ready\"")
        {
            return Err(DevError::corrupt(format!(
                "stateful HTTP negative step '{name}' emitted readiness"
            )));
        }
        Ok(output)
    }

    fn observe_command(
        &mut self,
        name: &str,
        command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(ProcessObservation, Output), DevError> {
        let recorded_command = command.clone();
        self.observe_command_with_record(name, command, recorded_command, cwd, environment)
    }

    fn observe_command_with_record(
        &mut self,
        name: &str,
        command: Vec<String>,
        recorded_command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<(ProcessObservation, Output), DevError> {
        let ordinal = self.ordinal;
        self.ordinal = self.ordinal.saturating_add(1);
        let safe = name
            .chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() {
                    character
                } else {
                    '-'
                }
            })
            .collect::<String>();
        let stdout_path = self
            .evidence
            .join(format!("{ordinal:03}-{safe}.stdout.log"));
        let stderr_path = self
            .evidence
            .join(format!("{ordinal:03}-{safe}.stderr.log"));
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment,
            timeout: COMMAND_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path,
            unavailable_exit_code: None,
        };
        let observation = process::run(&specification, &self.observation_root);
        let output = Output {
            bytes: process::read_bounded(&stdout_path, MAXIMUM_OUTPUT_BYTES)?,
        };
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command: recorded_command,
            process: observation.clone(),
        });
        Ok((observation, output))
    }

    fn require_status(
        &self,
        name: &str,
        observation: ProcessObservation,
        expected: ProcessStatus,
    ) -> Result<(), DevError> {
        if observation.status == ProcessStatus::Unavailable {
            return Err(DevError::unavailable(format!(
                "stateful HTTP step '{name}' is unavailable; inspect {}",
                self.evidence.display()
            )));
        }
        if observation.status != expected {
            return Err(DevError::corrupt(format!(
                "stateful HTTP step '{name}' ended as {:?}; inspect {}",
                observation.status,
                self.evidence.display()
            )));
        }
        Ok(())
    }
}

fn records(bytes: &[u8]) -> Result<Vec<CompactRecord>, DevError> {
    parse_records("stateful-http", bytes).map_err(|diagnostics| {
        DevError::corrupt(format!(
            "stateful HTTP command output is not compact records: {}",
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>()
                .join(",")
        ))
    })
}

fn required_record<'a>(
    records: &'a [CompactRecord],
    operation: &str,
) -> Result<&'a CompactRecord, DevError> {
    records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| DevError::corrupt(format!("compact output omitted '{operation}'")))
}

fn field<'a>(record: &'a CompactRecord, name: &str) -> Option<&'a str> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
}

fn required_field<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, DevError> {
    field(record, name).ok_or_else(|| {
        DevError::corrupt(format!(
            "compact '{}' record omitted '{name}'",
            record.operation
        ))
    })
}

fn parse_u64(value: &str, label: &str) -> Result<u64, DevError> {
    value
        .parse::<u64>()
        .map_err(|_| DevError::corrupt(format!("{label} is not an unsigned integer")))
}

fn require_record_field(
    records: &[CompactRecord],
    operation: &str,
    name: &str,
    expected: &str,
) -> Result<(), DevError> {
    let actual = required_field(required_record(records, operation)?, name)?;
    if actual == expected {
        Ok(())
    } else {
        Err(DevError::corrupt(format!(
            "compact {operation}.{name} was '{actual}', expected '{expected}'"
        )))
    }
}

fn path_text(path: &Path) -> Result<&str, DevError> {
    path.to_str()
        .ok_or_else(|| DevError::usage("stateful HTTP path is not UTF-8"))
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut binary = None;
    let mut evidence_root = None;
    let mut machine = false;
    let mut arguments = arguments;
    while let Some(argument) = crate::next_utf8(&mut arguments, "stateful HTTP option")? {
        match argument.as_str() {
            "--binary" => {
                let value = crate::next_utf8(&mut arguments, "stateful HTTP binary")?
                    .ok_or_else(|| DevError::usage("--binary requires a path"))?;
                if binary.replace(PathBuf::from(value)).is_some() {
                    return Err(DevError::usage("duplicate --binary option"));
                }
            }
            "--evidence-root" => {
                let value = crate::next_utf8(&mut arguments, "stateful HTTP evidence root")?
                    .ok_or_else(|| DevError::usage("--evidence-root requires a path"))?;
                if evidence_root.replace(PathBuf::from(value)).is_some() {
                    return Err(DevError::usage("duplicate --evidence-root option"));
                }
            }
            "--machine" if !machine => machine = true,
            "--machine" => return Err(DevError::usage("duplicate --machine option")),
            other => return Err(DevError::usage(format!("unknown option '{other}'"))),
        }
    }
    Ok(Options {
        binary: binary.unwrap_or_else(|| PathBuf::from("target/release/lkjscript")),
        evidence_root,
        machine,
    })
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| DevError::infrastructure("lkjscript-dev escaped its workspace"))?
        .canonicalize()
        .map_err(DevError::from)
}

fn resolve_binary(repository: Option<&Path>, path: &Path) -> Result<PathBuf, DevError> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository
            .ok_or_else(|| {
                DevError::usage(
                    "--binary must be absolute when --evidence-root selects transferred mode",
                )
            })?
            .join(path)
    };
    if !path.is_absolute() || has_noncanonical_component(&path) {
        return Err(DevError::usage(
            "stateful HTTP binary path must be absolute and lexically canonical",
        ));
    }
    let metadata = fs::symlink_metadata(&path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(
            "stateful HTTP binary must be a regular file",
        ));
    }
    if metadata.len() > MAXIMUM_CANDIDATE_BINARY_BYTES {
        return Err(DevError::usage(format!(
            "stateful HTTP binary exceeds {MAXIMUM_CANDIDATE_BINARY_BYTES} bytes"
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage("stateful HTTP binary is not executable"));
    }
    let canonical = path.canonicalize().map_err(DevError::from)?;
    if canonical != path {
        return Err(DevError::usage(
            "stateful HTTP binary contains a symlink or noncanonical component",
        ));
    }
    Ok(canonical)
}

fn copy_binary(source: &Path, destination: &Path) -> Result<(), DevError> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(
            "stateful HTTP candidate must be a regular non-symlink file",
        ));
    }
    let mut input = File::open(source)?;
    let mut output = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(destination)?;
    io::copy(&mut input, &mut output)?;
    output.set_permissions(metadata.permissions())?;
    output.sync_all()?;
    drop(output);
    File::open(
        destination
            .parent()
            .ok_or_else(|| DevError::infrastructure("copied binary has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

fn current_verifier() -> Result<PathBuf, DevError> {
    let path = std::env::current_exe().map_err(|error| {
        DevError::infrastructure(format!("resolve stateful HTTP verifier: {error}"))
    })?;
    resolve_regular_executable(
        &path,
        "stateful HTTP verifier",
        MAXIMUM_VERIFIER_BINARY_BYTES,
    )
}

fn resolve_regular_executable(
    path: &Path,
    label: &str,
    maximum_bytes: u64,
) -> Result<PathBuf, DevError> {
    if !path.is_absolute() || has_noncanonical_component(path) {
        return Err(DevError::usage(format!(
            "{label} path '{}' must be absolute and lexically canonical",
            path.display()
        )));
    }
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect {label} '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(format!(
            "{label} '{}' must be a regular non-symlink file",
            path.display()
        )));
    }
    if metadata.len() > maximum_bytes || metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage(format!(
            "{label} '{}' is oversized or non-executable",
            path.display()
        )));
    }
    let canonical = path.canonicalize().map_err(|error| {
        DevError::infrastructure(format!("resolve {label} '{}': {error}", path.display()))
    })?;
    if canonical != path {
        return Err(DevError::usage(format!(
            "{label} path '{}' contains a symlink or noncanonical component",
            path.display()
        )));
    }
    Ok(canonical)
}

fn executable_observation(
    path: &Path,
    label: &str,
    maximum_bytes: u64,
) -> Result<ExecutableObservation, DevError> {
    let path = resolve_regular_executable(path, label, maximum_bytes)?;
    let file = evidence::proof(&path, path.display().to_string())?;
    let byte_length = file
        .bytes
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted byte length")))?;
    let mode = file
        .mode
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted mode")))?;
    let verification_digest = file
        .digest
        .clone()
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted digest")))?;
    Ok(ExecutableObservation {
        file,
        byte_length,
        mode,
        executable: mode & 0o111 != 0,
        sha256: sha256_file(&path, maximum_bytes)?,
        verification_digest,
    })
}

fn sha256_file(path: &Path, maximum_bytes: u64) -> Result<String, DevError> {
    use sha2::{Digest, Sha256};
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(DevError::usage(
            "stateful HTTP SHA-256 input is unsafe or oversized",
        ));
    }
    let mut input = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        observed = observed
            .checked_add(read as u64)
            .ok_or_else(|| DevError::infrastructure("stateful SHA-256 length overflow"))?;
        hasher.update(&buffer[..read]);
    }
    if observed != metadata.len() {
        return Err(DevError::infrastructure(
            "stateful SHA-256 input changed while reading",
        ));
    }
    Ok(hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

fn create_explicit_evidence_root(requested: &Path) -> Result<PathBuf, DevError> {
    if !requested.is_absolute() || has_noncanonical_component(requested) {
        return Err(DevError::usage(format!(
            "evidence root '{}' must be absolute and lexically canonical",
            requested.display()
        )));
    }
    if fs::symlink_metadata(requested).is_ok() {
        return Err(DevError::usage(format!(
            "evidence root '{}' already exists",
            requested.display()
        )));
    }
    let parent = requested
        .parent()
        .ok_or_else(|| DevError::usage("evidence root has no parent"))?;
    let metadata = fs::symlink_metadata(parent).map_err(|error| {
        DevError::usage(format!(
            "inspect evidence-root parent '{}': {error}",
            parent.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::usage(format!(
            "evidence-root parent '{}' must be a regular non-symlink directory",
            parent.display()
        )));
    }
    let canonical_parent = parent.canonicalize().map_err(|error| {
        DevError::infrastructure(format!(
            "resolve evidence-root parent '{}': {error}",
            parent.display()
        ))
    })?;
    if canonical_parent != parent {
        return Err(DevError::usage(format!(
            "evidence-root parent '{}' contains a symlink or noncanonical component",
            parent.display()
        )));
    }
    let name = requested
        .file_name()
        .ok_or_else(|| DevError::usage("evidence root has no private directory name"))?;
    let root = canonical_parent.join(name);
    fs::create_dir(&root)?;
    let result = (|| {
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700))?;
        File::open(&canonical_parent)?.sync_all()?;
        let canonical_root = root.canonicalize()?;
        if canonical_root != root {
            return Err(DevError::infrastructure(
                "created evidence root escaped its canonical parent",
            ));
        }
        Ok(canonical_root)
    })();
    if result.is_err() {
        let _ = fs::remove_dir(&root);
    }
    result
}

fn new_evidence_directory(repository: &Path) -> Result<PathBuf, DevError> {
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let parent = repository.join(".artifacts/lkjscript-dev/stateful-http");
    fs::create_dir_all(&parent)?;
    let metadata = fs::symlink_metadata(&parent)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::infrastructure(
            "stateful evidence parent is not a regular non-symlink directory",
        ));
    }
    let directory = parent.join(format!(
        "{}-{}-{ordinal}",
        unix_nanoseconds()?,
        std::process::id()
    ));
    fs::create_dir(&directory)?;
    Ok(directory)
}

fn has_noncanonical_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

fn sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(values: &[&str]) -> Result<Options, DevError> {
        parse_options(values.iter().map(OsString::from))
    }

    #[test]
    fn stateful_http_schema_is_stable_and_campaign_independent() {
        let schema = SchemaIdentity {
            identity: STATEFUL_SCHEMA.to_owned(),
            version: STATEFUL_SCHEMA_VERSION,
        };
        assert_eq!(
            serde_json::to_string(&schema).expect("encode stateful schema"),
            r#"{"identity":"lkjscript-stateful-http-acceptance","version":6}"#
        );
        assert_eq!(STATEFUL_WORKFLOW, "stateful-http-application");
    }

    #[test]
    fn options_are_closed() {
        let parsed =
            parse_options([OsString::from("--machine")].into_iter()).expect("stateful options");
        assert!(parsed.machine);
        assert!(parsed.evidence_root.is_none());
        assert!(parse_options([OsString::from("--unknown")].into_iter()).is_err());
        assert!(options(&["--binary"]).is_err());
        assert!(options(&["--postgres-root"]).is_err());
        assert!(options(&["--evidence-root"]).is_err());
        assert!(options(&["--binary", "one", "--binary", "two"]).is_err());
        assert!(options(&["--postgres-root", "one"]).is_err());
        assert!(options(&["--evidence-root", "/tmp/one", "--evidence-root", "/tmp/two"]).is_err());
        assert!(options(&["--machine", "--machine"]).is_err());
        assert!(resolve_binary(None, Path::new("relative-candidate")).is_err());
    }

    #[test]
    fn explicit_evidence_root_is_absolute_private_and_create_new() {
        let temporary = tempfile::tempdir().expect("temporary evidence-root parent");
        assert!(create_explicit_evidence_root(Path::new("relative")).is_err());
        let root = temporary.path().join("stateful");
        let created = create_explicit_evidence_root(&root).expect("create evidence root");
        assert_eq!(created, root);
        let metadata = fs::symlink_metadata(&created).expect("evidence-root metadata");
        assert!(metadata.is_dir());
        #[cfg(unix)]
        assert_eq!(metadata.permissions().mode() & 0o7777, 0o700);
        assert!(create_explicit_evidence_root(&created).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn explicit_evidence_root_rejects_symlinked_boundaries() {
        let temporary = tempfile::tempdir().expect("temporary evidence-root fixtures");
        let real = temporary.path().join("real");
        fs::create_dir(&real).expect("create real parent");
        let link = temporary.path().join("link");
        std::os::unix::fs::symlink(&real, &link).expect("create parent symlink");
        assert!(create_explicit_evidence_root(&link.join("escaped")).is_err());

        let file = temporary.path().join("file");
        fs::write(&file, b"retained").expect("write existing file");
        assert!(create_explicit_evidence_root(&file).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn candidate_binary_is_regular_executable_and_bounded() {
        let temporary = tempfile::tempdir().expect("temporary stateful binary fixtures");
        let candidate = temporary.path().join("candidate");
        let file = File::create(&candidate).expect("create candidate");
        file.set_len(MAXIMUM_CANDIDATE_BINARY_BYTES.saturating_add(1))
            .expect("extend oversized candidate");
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))
            .expect("make candidate executable");
        assert!(resolve_binary(Some(temporary.path()), &candidate).is_err());

        file.set_len(1).expect("shrink bounded candidate");
        assert!(resolve_binary(Some(temporary.path()), &candidate).is_ok());
        let link = temporary.path().join("candidate-link");
        std::os::unix::fs::symlink(&candidate, &link).expect("create candidate link");
        assert!(resolve_binary(Some(temporary.path()), &link).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn copied_candidate_preserves_bytes_and_mode() {
        let temporary = tempfile::tempdir().expect("temporary copy fixtures");
        let source = temporary.path().join("source");
        fs::write(&source, b"exact candidate").expect("write source candidate");
        fs::set_permissions(&source, fs::Permissions::from_mode(0o751)).expect("set source mode");
        let destination = temporary.path().join("destination");
        copy_binary(&source, &destination).expect("copy candidate");
        assert_eq!(
            fs::read(&source).expect("read source"),
            fs::read(&destination).expect("read copy")
        );
        assert_eq!(
            fs::metadata(&destination)
                .expect("copy metadata")
                .permissions()
                .mode()
                & 0o7777,
            0o751
        );
        assert!(copy_binary(&source, &destination).is_err());
    }
}
