use crate::authority::{self, AuthorityObservation};
use crate::error::DevError;
use crate::evidence::{self, FileProof, PublishedEvidence, VerificationDigest};
use crate::http_probe;
use crate::process::{self, ProcessControl, ProcessObservation, ProcessSpec, ProcessStatus};
use lkjscript::platform::control::{CompactRecord, parse_records};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const ACCEPTANCE_SCHEMA: &str = "lkjscript-distributed-http-acceptance";
const ACCEPTANCE_SCHEMA_VERSION: u32 = 2;
const ACCEPTANCE_WORKFLOW: &str = "distributed-http-application";
const MAXIMUM_COMMAND_OUTPUT_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_CANDIDATE_EXECUTABLE_BYTES: u64 = 256 * 1024 * 1024;
// Local contributor checks copy a debug verifier with symbols. Keep that operational input
// separate from the tighter distributed product-candidate bound.
const MAXIMUM_VERIFIER_EXECUTABLE_BYTES: u64 = 384 * 1024 * 1024;
const MAXIMUM_RECEIPT_BYTES: u64 = 64 * 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const RUNNER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const READY_TIMEOUT: Duration = Duration::from_secs(30);
const STOP_TIMEOUT: Duration = Duration::from_secs(35);
const KILL_TIMEOUT: Duration = Duration::from_secs(5);
const POLL_INTERVAL: Duration = Duration::from_millis(25);
const CHANGED_RESPONSE: &[u8] = b"changed through the public CLI";
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct Options {
    binary: PathBuf,
    evidence_root: Option<PathBuf>,
    machine: bool,
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

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum AcceptanceStatus {
    Passed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct AcceptanceReceipt {
    schema: SchemaIdentity,
    workflow: String,
    status: AcceptanceStatus,
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
    environment_names: Vec<String>,
    verifier: ExecutableObservation,
    candidate: ExecutableObservation,
    copied_candidate: ExecutableObservation,
    commands: Vec<CommandEvidence>,
    runners: Vec<RunnerEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<WorkflowResult>,
    cleanup: CleanupObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlatformObservation {
    operating_system: String,
    architecture: String,
    process_control: String,
    client: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    expected: String,
    process: ProcessObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    diagnostic_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunnerEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready: Option<ReadyObservation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stopped: Option<StoppedObservation>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ReadyObservation {
    artifact_digest: String,
    target: String,
    runner: String,
    configured_listener: String,
    local_address: String,
    grants: BTreeMap<String, String>,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct StoppedObservation {
    local_address: String,
    admission_stopped: bool,
    remaining_tasks: u64,
    cleanup_failures: u64,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpObservation {
    status: u16,
    body_bytes: u64,
    body_sha256: String,
    body_digest: VerificationDigest,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CompilerObservation {
    cache: String,
    compiled: u64,
    reused: u64,
    removed: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct WorkflowResult {
    package_version: String,
    registry_contract: String,
    registry_digest: String,
    cli_contract_version: u64,
    project_creation_contract: String,
    project: String,
    descriptor_path: String,
    artifact_path: String,
    request_path: String,
    logical_plan_path: String,
    repository: String,
    package: String,
    initial_revision: String,
    accepted_revision: String,
    semantic_state: String,
    semantic_root: String,
    owners: u64,
    dependencies: u64,
    targets: u64,
    tests: u64,
    plan_token: String,
    request_commitment: String,
    prepared_commitment: String,
    change_compiler_units: u64,
    check: CompilerObservation,
    incremental_build: CompilerObservation,
    clean_build: CompilerObservation,
    artifact_manifest: String,
    artifact_bundle: String,
    artifact_bytes: u64,
    artifact_sha256: String,
    clean_artifact_sha256: String,
    clean_incremental_equal: bool,
    descriptor: FileProof,
    artifact: FileProof,
    change_request: FileProof,
    logical_plan: FileProof,
    authority_before: AuthorityObservation,
    authority_after: AuthorityObservation,
    authority_unchanged: bool,
    responses: Vec<HttpObservation>,
    restart_equal: bool,
    startup_failures_without_ready: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupObservation {
    runner_cleanup_attempted: bool,
    runner_cleanup_complete: bool,
    isolated_root_removed: bool,
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
pub(crate) struct TransferredReceiptBinding {
    pub(crate) receipt_bytes: u64,
    pub(crate) receipt_sha256: String,
    pub(crate) verifier_sha256: String,
    pub(crate) candidate_sha256: String,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) commands: u64,
    pub(crate) runners: u64,
    pub(crate) responses: u64,
    pub(crate) cleanup_complete: bool,
}

#[derive(Debug)]
struct AcceptanceFailure {
    class: &'static str,
    code: &'static str,
    message: String,
}

impl AcceptanceFailure {
    fn acceptance(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            class: "acceptance",
            code,
            message: message.into(),
        }
    }

    fn infrastructure(code: &'static str, error: impl std::fmt::Display) -> Self {
        Self {
            class: "infrastructure",
            code,
            message: error.to_string(),
        }
    }

    fn receipt(&self) -> Failure {
        Failure {
            class: self.class.to_owned(),
            code: self.code.to_owned(),
            message: self.message.clone(),
        }
    }
}

struct CommandOutput {
    stdout: Vec<u8>,
}

struct AcceptanceContext {
    observation_root: PathBuf,
    evidence_directory: PathBuf,
    copied_binary: PathBuf,
    command_ordinal: u64,
    commands: Vec<CommandEvidence>,
    runners: Vec<RunnerEvidence>,
    active_runner: Option<ActiveRunner>,
}

struct ActiveRunner {
    name: String,
    command: Vec<String>,
    control: ProcessControl,
    receiver: Receiver<ProcessObservation>,
    thread: Option<thread::JoinHandle<()>>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    ready: Option<ReadyObservation>,
    terminal: Option<ProcessObservation>,
    started: Instant,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse_options(arguments)?;
    let verifier_path = current_verifier()?;
    let verifier = executable_observation(
        &verifier_path,
        "distributed HTTP verifier",
        MAXIMUM_VERIFIER_EXECUTABLE_BYTES,
    )?;
    let (execution_context, checkout_root, candidate_path, evidence_directory, observation_root) =
        if let Some(requested_evidence_root) = &options.evidence_root {
            let candidate = resolve_candidate(None, &options.binary)?;
            let evidence = create_explicit_evidence_root(requested_evidence_root)?;
            (
                "transferred".to_owned(),
                None,
                candidate,
                evidence.clone(),
                evidence,
            )
        } else {
            let repository = repository_root()?;
            let candidate = resolve_candidate(Some(&repository), &options.binary)?;
            let evidence = new_evidence_directory(&repository)?;
            (
                "contributor".to_owned(),
                Some(repository.clone()),
                candidate,
                evidence,
                repository,
            )
        };
    let receipt_path = evidence_directory.join("receipt.json");
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let candidate = executable_observation(
        &candidate_path,
        "distributed HTTP candidate",
        MAXIMUM_CANDIDATE_EXECUTABLE_BYTES,
    )?;
    let temporary = tempfile::Builder::new()
        .prefix("lkjscript-distributed-http-")
        .tempdir()
        .map_err(|error| DevError::infrastructure(format!("create isolated workspace: {error}")))?;
    let isolated_root = temporary.path().canonicalize().map_err(|error| {
        DevError::infrastructure(format!("canonicalize isolated workspace: {error}"))
    })?;
    let outside_checkout = checkout_root
        .as_ref()
        .is_none_or(|repository| !isolated_root.starts_with(repository));
    let copied_binary = isolated_root.join("lkjscript");
    copy_binary(&candidate_path, &copied_binary)?;
    let copied_candidate = executable_observation(
        &copied_binary,
        "copied HTTP candidate",
        MAXIMUM_CANDIDATE_EXECUTABLE_BYTES,
    )?;
    if candidate.byte_length != copied_candidate.byte_length
        || candidate.sha256 != copied_candidate.sha256
        || candidate.verification_digest != copied_candidate.verification_digest
    {
        return Err(DevError::infrastructure(
            "copied HTTP candidate does not match the selected candidate bytes",
        ));
    }
    let mut context = AcceptanceContext {
        observation_root,
        evidence_directory: evidence_directory.clone(),
        copied_binary,
        command_ordinal: 0,
        commands: Vec::new(),
        runners: Vec::new(),
        active_runner: None,
    };

    let workflow = if outside_checkout {
        run_workflow(&mut context, &isolated_root)
    } else {
        Err(AcceptanceFailure::acceptance(
            "workspace_isolation",
            "temporary product workspace is inside the repository checkout",
        ))
    };
    let runner_cleanup_attempted = context.active_runner.is_some();
    let runner_cleanup = context.cleanup_runner();
    let mut failure = workflow.as_ref().err().map(AcceptanceFailure::receipt);
    if let Err(error) = &runner_cleanup
        && failure.is_none()
    {
        failure = Some(error.receipt());
    }
    let result = workflow.ok();
    let commands = std::mem::take(&mut context.commands);
    let runners = std::mem::take(&mut context.runners);
    drop(context);
    let isolated_root_text = isolated_root.display().to_string();
    let removal = temporary.close();
    let isolated_root_removed = removal.is_ok() && !isolated_root.exists();
    if let Err(error) = removal
        && failure.is_none()
    {
        failure = Some(Failure {
            class: "infrastructure".to_owned(),
            code: "workspace_cleanup".to_owned(),
            message: error.to_string(),
        });
    }
    let cleanup = CleanupObservation {
        runner_cleanup_attempted,
        runner_cleanup_complete: runner_cleanup.is_ok(),
        isolated_root_removed,
    };
    let status = if result.is_some()
        && failure.is_none()
        && cleanup.runner_cleanup_complete
        && cleanup.isolated_root_removed
    {
        AcceptanceStatus::Passed
    } else {
        AcceptanceStatus::Failed
    };
    let receipt = AcceptanceReceipt {
        schema: SchemaIdentity {
            identity: ACCEPTANCE_SCHEMA.to_owned(),
            version: ACCEPTANCE_SCHEMA_VERSION,
        },
        workflow: ACCEPTANCE_WORKFLOW.to_owned(),
        status,
        platform: PlatformObservation {
            operating_system: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            process_control: "linux-process-group-sigint-sigkill".to_owned(),
            client: "first-party-bounded-raw-http1".to_owned(),
        },
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        execution_context,
        checkout_root: checkout_root.map(|path| path.display().to_string()),
        evidence_root: evidence_directory.display().to_string(),
        isolated_root: isolated_root_text,
        isolated_root_outside_checkout: outside_checkout,
        environment_names: vec!["LANG".to_owned()],
        verifier,
        candidate,
        copied_candidate,
        commands,
        runners,
        result,
        cleanup,
        failure,
    };
    let published = evidence::publish_json(&receipt_path, &receipt)?;
    let receipt_sha256 = sha256_file(&receipt_path, MAXIMUM_RECEIPT_BYTES)?;
    print_summary(&options, &receipt, &published, &receipt_sha256)?;
    Ok(if status == AcceptanceStatus::Passed {
        0
    } else {
        1
    })
}

pub(crate) fn read_transferred_receipt(
    path: &Path,
    candidate_path: &Path,
    verifier_path: &Path,
) -> Result<TransferredReceiptBinding, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect transferred distributed HTTP receipt '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > MAXIMUM_RECEIPT_BYTES
    {
        return Err(DevError::corrupt(
            "transferred distributed HTTP receipt is unsafe or oversized",
        ));
    }
    let bytes = fs::read(path).map_err(|error| {
        DevError::infrastructure(format!(
            "read transferred distributed HTTP receipt '{}': {error}",
            path.display()
        ))
    })?;
    let receipt: AcceptanceReceipt = serde_json::from_slice(&bytes).map_err(|error| {
        DevError::corrupt(format!(
            "decode transferred distributed HTTP receipt: {error}"
        ))
    })?;
    if evidence::encode_json(&receipt)? != bytes {
        return Err(DevError::corrupt(
            "transferred distributed HTTP receipt is not in canonical evidence encoding",
        ));
    }
    let evidence_root = path
        .parent()
        .ok_or_else(|| DevError::corrupt("transferred distributed HTTP receipt has no parent"))?
        .canonicalize()
        .map_err(|error| {
            DevError::infrastructure(format!(
                "resolve transferred distributed HTTP evidence root: {error}"
            ))
        })?;
    let canonical_path = path.canonicalize().map_err(|error| {
        DevError::infrastructure(format!(
            "resolve transferred distributed HTTP receipt: {error}"
        ))
    })?;
    let candidate = executable_observation(
        candidate_path,
        "target-admission distributed HTTP candidate",
        MAXIMUM_CANDIDATE_EXECUTABLE_BYTES,
    )?;
    let verifier = executable_observation(
        verifier_path,
        "target-admission distributed HTTP verifier",
        MAXIMUM_VERIFIER_EXECUTABLE_BYTES,
    )?;
    let result = receipt
        .result
        .as_ref()
        .ok_or_else(|| DevError::corrupt("passed distributed HTTP receipt omitted its result"))?;
    let cleanup_complete =
        receipt.cleanup.runner_cleanup_complete && receipt.cleanup.isolated_root_removed;
    if canonical_path != evidence_root.join("receipt.json")
        || receipt.schema.identity != ACCEPTANCE_SCHEMA
        || receipt.schema.version != ACCEPTANCE_SCHEMA_VERSION
        || receipt.status != AcceptanceStatus::Passed
        || receipt.workflow != ACCEPTANCE_WORKFLOW
        || receipt.execution_context != "transferred"
        || receipt.checkout_root.is_some()
        || receipt.evidence_root != evidence_root.display().to_string()
        || !receipt.isolated_root_outside_checkout
        || Path::new(&receipt.isolated_root).exists()
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
        || !Path::new(&receipt.copied_candidate.file.path)
            .starts_with(Path::new(&receipt.isolated_root))
        || receipt.failure.is_some()
        || !cleanup_complete
        || !result.clean_incremental_equal
        || result.artifact_sha256 != result.clean_artifact_sha256
        || !result.authority_unchanged
        || result.authority_before != result.authority_after
        || !result.restart_equal
        || result.startup_failures_without_ready == 0
        || result.responses.is_empty()
    {
        return Err(DevError::corrupt(
            "transferred distributed HTTP receipt binding or acceptance mismatch",
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
            "transferred distributed HTTP receipt invoked checkout build tooling",
        ));
    }
    Ok(TransferredReceiptBinding {
        receipt_bytes: metadata.len(),
        receipt_sha256: sha256_file(path, MAXIMUM_RECEIPT_BYTES)?,
        verifier_sha256: verifier.sha256,
        candidate_sha256: candidate.sha256,
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        commands: receipt.commands.len() as u64,
        runners: receipt.runners.len() as u64,
        responses: result.responses.len() as u64,
        cleanup_complete,
    })
}

fn run_workflow(
    context: &mut AcceptanceContext,
    isolated_root: &Path,
) -> Result<WorkflowResult, AcceptanceFailure> {
    let project = isolated_root.join("application");
    let descriptor = project.join("service.deployment.json");
    let artifact = project.join("generated/application.lkja");
    let clean_artifact = project.join("generated/application-clean.lkja");
    let request_path = isolated_root.join("response-change.lkjc");
    let altered_request_path = isolated_root.join("altered-response-change.lkjc");
    let logical_plan_path = isolated_root.join("response-change.logical-plan");

    let capabilities = context.invoke_success(
        "capabilities",
        vec!["capabilities".to_owned()],
        isolated_root,
    )?;
    let capability_records = compact_records("capabilities", &capabilities.stdout)?;
    let registry = required_record(&capability_records, "registry")?;
    let registry_contract = required_field(registry, "contract")?.to_owned();
    let registry_digest = required_field(registry, "digest")?.to_owned();
    let cli_contract_version = parse_u64(required_field(registry, "cli")?, "CLI contract")?;

    let created = context.invoke_success(
        "new-http",
        vec![
            "new".to_owned(),
            project.display().to_string(),
            "--template".to_owned(),
            "http".to_owned(),
            "--name".to_owned(),
            "application".to_owned(),
        ],
        isolated_root,
    )?;
    let created_records = compact_records("new HTTP", &created.stdout)?;
    require_field(&created_records, "project", "template", "http")?;
    let repository =
        required_field(required_record(&created_records, "repository")?, "id")?.to_owned();
    let package = required_field(required_record(&created_records, "package")?, "id")?.to_owned();
    let initial_revision =
        required_field(required_record(&created_records, "revision")?, "id")?.to_owned();
    let summary = required_record(&created_records, "summary")?;
    let owners = parse_u64(required_field(summary, "owners")?, "owner count")?;
    let dependencies = parse_u64(required_field(summary, "dependencies")?, "dependency count")?;
    let targets = parse_u64(required_field(summary, "targets")?, "target count")?;
    let tests = parse_u64(required_field(summary, "tests")?, "test count")?;
    if owners != 20 || dependencies != 1 || targets != 1 || tests != 1 {
        return Err(AcceptanceFailure::acceptance(
            "created_topology",
            "HTTP recipe semantic counts disagree with the maintained closed topology",
        ));
    }
    let deployment_record = required_record(&created_records, "deployment")?;
    require_exact(
        required_field(deployment_record, "descriptor")?,
        &descriptor.display().to_string(),
        "deployment descriptor path",
    )?;
    require_exact(
        required_field(deployment_record, "artifact-output")?,
        &artifact.display().to_string(),
        "recommended artifact path",
    )?;
    require_field(&created_records, "deployment", "target", "serve")?;
    require_field(&created_records, "deployment", "runner", "http")?;
    require_field(&created_records, "deployment", "listener", "127.0.0.1:0")?;
    validate_next_actions(&created_records)?;
    validate_starter_files(&project, &descriptor, &artifact)?;

    context.invoke_runtime_failure(
        "serve-absent-artifact",
        vec![
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor.display().to_string(),
        ],
        isolated_root,
    )?;
    context.invoke_compact_failure(
        "run-http-target",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "run".to_owned(),
            "serve".to_owned(),
        ],
        isolated_root,
        "normalized_runner_kind",
    )?;

    let request = format!(
        "request base={initial_revision}\n\
         expression.static-text as=$response value=\"changed through the public CLI\"\n\
         replace.body function=application/response-text body=$response\n"
    );
    publish_product_input(&request_path, request.as_bytes())?;
    let planned = context.invoke_success(
        "change-plan",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "change".to_owned(),
            "plan".to_owned(),
            "--input-file".to_owned(),
            request_path.display().to_string(),
            "--output".to_owned(),
            logical_plan_path.display().to_string(),
        ],
        isolated_root,
    )?;
    let plan_records = compact_records("change plan", &planned.stdout)?;
    let plan = required_record(&plan_records, "plan")?;
    let plan_token = required_field(plan, "token")?.to_owned();
    let request_commitment = required_field(plan, "request-commitment")?.to_owned();
    let prepared_commitment = required_field(plan, "prepared-commitment")?.to_owned();
    let validation = required_record(&plan_records, "validation")?;
    let change_compiler_units = parse_u64(
        required_field(validation, "compiler-units")?,
        "change compiler units",
    )?;

    let altered_request = format!(
        "request base={initial_revision}\n\
         expression.static-text as=$response value=\"altered after review\"\n\
         replace.body function=application/response-text body=$response\n"
    );
    publish_product_input(&altered_request_path, altered_request.as_bytes())?;
    context.invoke_compact_failure(
        "change-apply-mismatch",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "change".to_owned(),
            "apply".to_owned(),
            "--input-file".to_owned(),
            altered_request_path.display().to_string(),
            "--plan".to_owned(),
            plan_token.clone(),
        ],
        isolated_root,
        "change_request_commitment_mismatch",
    )?;
    require_revision(
        context,
        isolated_root,
        &project,
        &initial_revision,
        "after mismatch",
    )?;

    let applied = context.invoke_success(
        "change-apply",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "change".to_owned(),
            "apply".to_owned(),
            "--input-file".to_owned(),
            request_path.display().to_string(),
            "--plan".to_owned(),
            plan_token.clone(),
        ],
        isolated_root,
    )?;
    let apply_records = compact_records("change apply", &applied.stdout)?;
    let accepted_revision =
        required_field(required_record(&apply_records, "revision")?, "result")?.to_owned();
    if accepted_revision == initial_revision {
        return Err(AcceptanceFailure::acceptance(
            "change_revision",
            "accepted response edit did not advance the semantic revision",
        ));
    }
    context.invoke_compact_failure(
        "change-stale-base",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "change".to_owned(),
            "plan".to_owned(),
            "--input-file".to_owned(),
            request_path.display().to_string(),
        ],
        isolated_root,
        "change_authored_stale_base",
    )?;
    require_revision(
        context,
        isolated_root,
        &project,
        &accepted_revision,
        "after stale plan",
    )?;

    let status = context.invoke_success(
        "status-accepted",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "status".to_owned(),
        ],
        isolated_root,
    )?;
    let status_records = compact_records("accepted status", &status.stdout)?;
    let semantic_state =
        required_field(required_record(&status_records, "state")?, "digest")?.to_owned();
    let semantic_root =
        required_field(required_record(&status_records, "root")?, "digest")?.to_owned();
    let authority_before = authority::observe_graph_authority(&project)
        .map_err(|error| AcceptanceFailure::infrastructure("authority_before", error))?;

    let checked = context.invoke_success(
        "check",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "check".to_owned(),
        ],
        isolated_root,
    )?;
    let check_records = compact_records("check", &checked.stdout)?;
    require_field(&check_records, "tests", "passed", "8")?;
    require_field(&check_records, "tests", "failed", "0")?;
    require_field(&check_records, "tests", "differential", "equal")?;
    let check_compilation = compiler_observation(&check_records)?;

    let built = context.invoke_success(
        "build-recommended",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "build".to_owned(),
            "--output".to_owned(),
            artifact.display().to_string(),
        ],
        isolated_root,
    )?;
    let build_records = compact_records("incremental build", &built.stdout)?;
    let incremental_build = compiler_observation(&build_records)?;
    let artifact_record = required_record(&build_records, "artifact")?;
    let artifact_manifest = required_field(artifact_record, "manifest")?.to_owned();
    let artifact_bundle = required_field(artifact_record, "bundle")?.to_owned();
    let artifact_bytes = parse_u64(required_field(artifact_record, "bytes")?, "artifact bytes")?;
    let artifact_data = process::read_bounded(&artifact, MAXIMUM_ARTIFACT_BYTES)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_read", error))?;
    if artifact_data.len() as u64 != artifact_bytes {
        return Err(AcceptanceFailure::acceptance(
            "artifact_length",
            "artifact output length disagrees with the build receipt",
        ));
    }
    let artifact_sha256 = sha256_hex(&artifact_data);
    context.invoke_compact_failure(
        "build-output-conflict",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "build".to_owned(),
            "--output".to_owned(),
            artifact.display().to_string(),
        ],
        isolated_root,
        "output_conflict",
    )?;
    let after_conflict = process::read_bounded(&artifact, MAXIMUM_ARTIFACT_BYTES)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_reread", error))?;
    if after_conflict != artifact_data {
        return Err(AcceptanceFailure::acceptance(
            "artifact_overwrite",
            "failed create-new build changed the existing artifact output",
        ));
    }

    let derived = project.join("derived");
    if derived.exists() {
        fs::remove_dir_all(&derived)
            .map_err(|error| AcceptanceFailure::infrastructure("derived_cache_reset", error))?;
    }
    let clean = context.invoke_success(
        "build-clean",
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "build".to_owned(),
            "--output".to_owned(),
            clean_artifact.display().to_string(),
        ],
        isolated_root,
    )?;
    let clean_records = compact_records("clean build", &clean.stdout)?;
    let clean_build = compiler_observation(&clean_records)?;
    let clean_data = process::read_bounded(&clean_artifact, MAXIMUM_ARTIFACT_BYTES)
        .map_err(|error| AcceptanceFailure::infrastructure("clean_artifact_read", error))?;
    let clean_artifact_sha256 = sha256_hex(&clean_data);
    let clean_incremental_equal = clean_data == artifact_data;
    if !clean_incremental_equal {
        return Err(AcceptanceFailure::acceptance(
            "artifact_determinism",
            "clean and incremental Artifact 10 bytes disagree",
        ));
    }

    let first = run_http_once(
        context,
        isolated_root,
        &descriptor,
        &artifact_bundle,
        "first",
    )?;
    let second = run_http_once(
        context,
        isolated_root,
        &descriptor,
        &artifact_bundle,
        "restart",
    )?;
    let restart_equal = first.status == second.status
        && first.body_sha256 == second.body_sha256
        && first.body_bytes == second.body_bytes;
    if !restart_equal {
        return Err(AcceptanceFailure::acceptance(
            "restart_response",
            "restarted service response disagrees with the first live response",
        ));
    }
    let artifact_startup_failures = exercise_artifact_startup_failures(
        context,
        isolated_root,
        &descriptor,
        &artifact,
        &artifact_data,
    )?;

    let authority_after = authority::observe_graph_authority(&project)
        .map_err(|error| AcceptanceFailure::infrastructure("authority_after", error))?;
    let authority_unchanged = authority_before == authority_after;
    if !authority_unchanged {
        return Err(AcceptanceFailure::acceptance(
            "authority_changed",
            "check, build, serving, request handling, or restart changed accepted authority",
        ));
    }
    require_revision(
        context,
        isolated_root,
        &project,
        &accepted_revision,
        "after restart",
    )?;

    let descriptor_proof = evidence::proof(&descriptor, descriptor.display().to_string())
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_proof", error))?;
    let artifact_proof = evidence::proof(&artifact, artifact.display().to_string())
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_proof", error))?;
    let request_proof = evidence::proof(&request_path, request_path.display().to_string())
        .map_err(|error| AcceptanceFailure::infrastructure("request_proof", error))?;
    let logical_plan_proof =
        evidence::proof(&logical_plan_path, logical_plan_path.display().to_string())
            .map_err(|error| AcceptanceFailure::infrastructure("logical_plan_proof", error))?;

    Ok(WorkflowResult {
        package_version: lkjscript::PRODUCT_VERSION.to_owned(),
        registry_contract,
        registry_digest,
        cli_contract_version,
        project_creation_contract: "lkjscript-project-creation-2".to_owned(),
        project: project.display().to_string(),
        descriptor_path: descriptor.display().to_string(),
        artifact_path: artifact.display().to_string(),
        request_path: request_path.display().to_string(),
        logical_plan_path: logical_plan_path.display().to_string(),
        repository,
        package,
        initial_revision,
        accepted_revision,
        semantic_state,
        semantic_root,
        owners,
        dependencies,
        targets,
        tests,
        plan_token,
        request_commitment,
        prepared_commitment,
        change_compiler_units,
        check: check_compilation,
        incremental_build,
        clean_build,
        artifact_manifest,
        artifact_bundle,
        artifact_bytes,
        artifact_sha256,
        clean_artifact_sha256,
        clean_incremental_equal,
        descriptor: descriptor_proof,
        artifact: artifact_proof,
        change_request: request_proof,
        logical_plan: logical_plan_proof,
        authority_before,
        authority_after,
        authority_unchanged,
        responses: vec![first, second],
        restart_equal,
        startup_failures_without_ready: 1_u64.saturating_add(artifact_startup_failures),
    })
}

fn exercise_artifact_startup_failures(
    context: &mut AcceptanceContext,
    isolated_root: &Path,
    descriptor: &Path,
    artifact: &Path,
    valid_bytes: &[u8],
) -> Result<u64, AcceptanceFailure> {
    let backup = artifact.with_extension("valid.lkja");
    fs::rename(artifact, &backup)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_backup", error))?;
    let serve_arguments = || {
        vec![
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor.display().to_string(),
        ]
    };
    let mut failures = 0_u64;

    fs::create_dir(artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_directory", error))?;
    context.invoke_runtime_failure(
        "serve-nonregular-artifact",
        serve_arguments(),
        isolated_root,
    )?;
    failures = failures.saturating_add(1);
    fs::remove_dir(artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_directory_cleanup", error))?;

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&backup, artifact)
            .map_err(|error| AcceptanceFailure::infrastructure("artifact_symlink", error))?;
        context.invoke_runtime_failure(
            "serve-symlink-artifact",
            serve_arguments(),
            isolated_root,
        )?;
        failures = failures.saturating_add(1);
        fs::remove_file(artifact).map_err(|error| {
            AcceptanceFailure::infrastructure("artifact_symlink_cleanup", error)
        })?;
    }

    let truncated_length = valid_bytes.len().min(16);
    publish_product_input(artifact, &valid_bytes[..truncated_length])?;
    context.invoke_runtime_failure("serve-truncated-artifact", serve_arguments(), isolated_root)?;
    failures = failures.saturating_add(1);
    fs::remove_file(artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("truncated_cleanup", error))?;

    let mut corrupt = valid_bytes.to_vec();
    let last = corrupt.last_mut().ok_or_else(|| {
        AcceptanceFailure::acceptance("artifact_empty", "valid Artifact 10 bytes are empty")
    })?;
    *last ^= 0x01;
    publish_product_input(artifact, &corrupt)?;
    context.invoke_runtime_failure("serve-corrupt-artifact", serve_arguments(), isolated_root)?;
    failures = failures.saturating_add(1);
    fs::remove_file(artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("corrupt_cleanup", error))?;

    let foreign_project = isolated_root.join("foreign-command");
    let foreign_artifact = isolated_root.join("foreign-command.lkja");
    context.invoke_success(
        "new-foreign-command",
        vec![
            "new".to_owned(),
            foreign_project.display().to_string(),
            "--template".to_owned(),
            "command".to_owned(),
            "--name".to_owned(),
            "foreign".to_owned(),
        ],
        isolated_root,
    )?;
    context.invoke_success(
        "build-foreign-command",
        vec![
            "--project".to_owned(),
            foreign_project.display().to_string(),
            "build".to_owned(),
            "--output".to_owned(),
            foreign_artifact.display().to_string(),
        ],
        isolated_root,
    )?;
    fs::rename(&foreign_artifact, artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("foreign_artifact_stage", error))?;
    context.invoke_runtime_failure("serve-foreign-artifact", serve_arguments(), isolated_root)?;
    failures = failures.saturating_add(1);
    fs::remove_file(artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("foreign_cleanup", error))?;

    fs::rename(&backup, artifact)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_restore", error))?;
    let restored = process::read_bounded(artifact, MAXIMUM_ARTIFACT_BYTES)
        .map_err(|error| AcceptanceFailure::infrastructure("artifact_restore_read", error))?;
    if restored != valid_bytes {
        return Err(AcceptanceFailure::acceptance(
            "artifact_restore",
            "negative startup checks did not restore the exact product-built artifact",
        ));
    }
    Ok(failures)
}

fn run_http_once(
    context: &mut AcceptanceContext,
    isolated_root: &Path,
    descriptor: &Path,
    artifact_bundle: &str,
    name: &str,
) -> Result<HttpObservation, AcceptanceFailure> {
    let ready = context.start_runner(
        name,
        vec![
            "serve".to_owned(),
            "--deployment".to_owned(),
            descriptor.display().to_string(),
        ],
        isolated_root,
    )?;
    if ready.artifact_digest != artifact_bundle
        || ready.target != "serve"
        || ready.runner != "http"
        || ready.configured_listener != "127.0.0.1:0"
        || ready.grants.get("streams").map(String::as_str) != Some("byte-stream")
    {
        return Err(AcceptanceFailure::acceptance(
            "ready_identity",
            "ready event disagrees with the exact starter deployment and artifact",
        ));
    }
    let address = ready
        .local_address
        .parse()
        .map_err(|error| AcceptanceFailure::infrastructure("ready_address", error))?;
    let response = http_probe::request(address, "GET", "/", &[], &[])
        .map_err(|error| AcceptanceFailure::infrastructure("http_request", error))?;
    if response.status != 200 || response.body != CHANGED_RESPONSE {
        return Err(AcceptanceFailure::acceptance(
            "http_response",
            "live HTTP response disagrees with accepted response-text graph meaning",
        ));
    }
    let expected_content_length = CHANGED_RESPONSE.len().to_string();
    if response.headers.get("content-length").map(String::as_str)
        != Some(expected_content_length.as_str())
    {
        return Err(AcceptanceFailure::acceptance(
            "http_content_length",
            "live HTTP response content length is absent or incorrect",
        ));
    }
    context.stop_runner()?;
    Ok(HttpObservation {
        status: response.status,
        body_bytes: response.body.len() as u64,
        body_sha256: sha256_hex(&response.body),
        body_digest: VerificationDigest::of(&response.body),
        elapsed_nanoseconds: response.elapsed_nanoseconds,
    })
}

impl AcceptanceContext {
    fn invoke_success(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
    ) -> Result<CommandOutput, AcceptanceFailure> {
        let (stdout, observation, command) = self.observe(name, arguments, cwd)?;
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            expected: "success".to_owned(),
            process: observation.clone(),
            diagnostic_code: None,
        });
        if observation.status != ProcessStatus::Passed {
            return Err(AcceptanceFailure::acceptance(
                "command_failed",
                format!("{name} ended as {:?}", observation.status),
            ));
        }
        Ok(CommandOutput { stdout })
    }

    fn invoke_compact_failure(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
        expected_code: &str,
    ) -> Result<(), AcceptanceFailure> {
        let (stdout, observation, command) = self.observe(name, arguments, cwd)?;
        let records = compact_records(name, &stdout)?;
        let code = required_field(required_record(&records, "diagnostic")?, "code")?.to_owned();
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            expected: "classified-failure".to_owned(),
            process: observation.clone(),
            diagnostic_code: Some(code.clone()),
        });
        if observation.status != ProcessStatus::Failed || code != expected_code {
            return Err(AcceptanceFailure::acceptance(
                "failure_classification",
                format!(
                    "{name} produced status {:?} and code {code}",
                    observation.status
                ),
            ));
        }
        Ok(())
    }

    fn invoke_runtime_failure(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
    ) -> Result<(), AcceptanceFailure> {
        let (stdout, observation, command) = self.observe(name, arguments, cwd)?;
        if stdout
            .windows(b"\"event\":\"ready\"".len())
            .any(|window| window == b"\"event\":\"ready\"")
        {
            return Err(AcceptanceFailure::acceptance(
                "failure_ready",
                format!("{name} emitted readiness for invalid startup input"),
            ));
        }
        let value: Value = serde_json::from_slice(&stdout)
            .map_err(|error| AcceptanceFailure::infrastructure("runtime_failure_json", error))?;
        let code = value
            .pointer("/error/code")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                AcceptanceFailure::acceptance(
                    "runtime_failure_shape",
                    format!("{name} omitted its typed error code"),
                )
            })?
            .to_owned();
        self.commands.push(CommandEvidence {
            name: name.to_owned(),
            command,
            expected: "startup-failure-without-ready".to_owned(),
            process: observation.clone(),
            diagnostic_code: Some(code),
        });
        if observation.status == ProcessStatus::Passed {
            return Err(AcceptanceFailure::acceptance(
                "runtime_failure_status",
                format!("{name} unexpectedly succeeded"),
            ));
        }
        Ok(())
    }

    fn observe(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
    ) -> Result<(Vec<u8>, ProcessObservation, Vec<String>), AcceptanceFailure> {
        let safe_name = safe_name(name)?;
        let ordinal = self.command_ordinal;
        self.command_ordinal = self.command_ordinal.saturating_add(1);
        let stdout_path = self
            .evidence_directory
            .join(format!("{ordinal:03}-{safe_name}.stdout.log"));
        let stderr_path = self
            .evidence_directory
            .join(format!("{ordinal:03}-{safe_name}.stderr.log"));
        let mut command = vec![self.copied_binary.display().to_string()];
        command.extend(arguments);
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment: isolated_environment(),
            timeout: COMMAND_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path,
            unavailable_exit_code: None,
        };
        let observation = process::run(&specification, &self.observation_root);
        let stdout = process::read_bounded(&stdout_path, MAXIMUM_COMMAND_OUTPUT_BYTES)
            .map_err(|error| AcceptanceFailure::infrastructure("command_stdout", error))?;
        Ok((stdout, observation, command))
    }

    fn start_runner(
        &mut self,
        name: &str,
        arguments: Vec<String>,
        cwd: &Path,
    ) -> Result<ReadyObservation, AcceptanceFailure> {
        if self.active_runner.is_some() {
            return Err(AcceptanceFailure::acceptance(
                "runner_overlap",
                "acceptance attempted to start two resident processes",
            ));
        }
        let safe_name = safe_name(name)?;
        let stdout_path = self
            .evidence_directory
            .join(format!("runner-{safe_name}.stdout.log"));
        let stderr_path = self
            .evidence_directory
            .join(format!("runner-{safe_name}.stderr.log"));
        let mut command = vec![self.copied_binary.display().to_string()];
        command.extend(arguments);
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment: isolated_environment(),
            timeout: RUNNER_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_COMMAND_OUTPUT_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: None,
        };
        let observation_root = self.observation_root.clone();
        let control = ProcessControl::default();
        let child_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        let thread = thread::Builder::new()
            .name(format!("distributed-http-{safe_name}"))
            .spawn(move || {
                let observation =
                    process::run_controlled(&specification, &observation_root, &child_control);
                let _ = sender.send(observation);
            })
            .map_err(|error| AcceptanceFailure::infrastructure("runner_thread", error))?;
        self.active_runner = Some(ActiveRunner {
            name: name.to_owned(),
            command,
            control,
            receiver,
            thread: Some(thread),
            stdout_path,
            stderr_path,
            ready: None,
            terminal: None,
            started: Instant::now(),
        });
        let ready = self
            .active_runner
            .as_mut()
            .ok_or_else(|| AcceptanceFailure::acceptance("runner_missing", "runner vanished"))?
            .wait_ready()?;
        self.active_runner
            .as_mut()
            .ok_or_else(|| AcceptanceFailure::acceptance("runner_missing", "runner vanished"))?
            .ready = Some(ready.clone());
        Ok(ready)
    }

    fn stop_runner(&mut self) -> Result<(), AcceptanceFailure> {
        let mut runner = self.active_runner.take().ok_or_else(|| {
            AcceptanceFailure::acceptance("runner_missing", "no active runner to stop")
        })?;
        runner.control.interrupt();
        let observation = match runner.wait_terminal(STOP_TIMEOUT) {
            Ok(observation) => observation,
            Err(error) => {
                runner.control.kill();
                let _ = runner.wait_terminal(KILL_TIMEOUT);
                runner.join()?;
                return Err(error);
            }
        };
        runner.join()?;
        let stdout = process::read_bounded(&runner.stdout_path, MAXIMUM_COMMAND_OUTPUT_BYTES)
            .map_err(|error| AcceptanceFailure::infrastructure("runner_stdout", error))?;
        let stopped = parse_stopped(&stdout)?;
        self.runners.push(RunnerEvidence {
            name: runner.name,
            command: runner.command,
            process: observation.clone(),
            ready: runner.ready,
            stopped: Some(stopped.clone()),
        });
        if observation.status != ProcessStatus::Passed
            || !stopped.admission_stopped
            || stopped.remaining_tasks != 0
            || stopped.cleanup_failures != 0
        {
            return Err(AcceptanceFailure::acceptance(
                "runner_shutdown",
                "resident process did not stop cleanly",
            ));
        }
        Ok(())
    }

    fn cleanup_runner(&mut self) -> Result<(), AcceptanceFailure> {
        let Some(mut runner) = self.active_runner.take() else {
            return Ok(());
        };
        runner.control.kill();
        let observation = runner.wait_terminal(KILL_TIMEOUT)?;
        runner.join()?;
        self.runners.push(RunnerEvidence {
            name: runner.name,
            command: runner.command,
            process: observation,
            ready: runner.ready,
            stopped: None,
        });
        Ok(())
    }
}

impl ActiveRunner {
    fn wait_ready(&mut self) -> Result<ReadyObservation, AcceptanceFailure> {
        let started = Instant::now();
        loop {
            if let Some(line) = first_line(&self.stdout_path)? {
                let mut ready = parse_ready(&line)?;
                ready.elapsed_nanoseconds = duration_nanoseconds(self.started.elapsed());
                return Ok(ready);
            }
            if self.poll_terminal()?.is_some() {
                return Err(AcceptanceFailure::acceptance(
                    "runner_before_ready",
                    "resident process exited before readiness",
                ));
            }
            if started.elapsed() >= READY_TIMEOUT {
                return Err(AcceptanceFailure::acceptance(
                    "runner_ready_timeout",
                    "resident process omitted readiness before the deadline",
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn poll_terminal(&mut self) -> Result<Option<&ProcessObservation>, AcceptanceFailure> {
        if self.terminal.is_none() {
            match self.receiver.try_recv() {
                Ok(observation) => self.terminal = Some(observation),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Err(AcceptanceFailure::infrastructure(
                        "runner_channel",
                        "resident process observation channel disconnected",
                    ));
                }
            }
        }
        Ok(self.terminal.as_ref())
    }

    fn wait_terminal(
        &mut self,
        timeout: Duration,
    ) -> Result<ProcessObservation, AcceptanceFailure> {
        if let Some(observation) = self.terminal.take() {
            return Ok(observation);
        }
        self.receiver
            .recv_timeout(timeout)
            .map_err(|error| AcceptanceFailure::infrastructure("runner_stop_timeout", error))
    }

    fn join(&mut self) -> Result<(), AcceptanceFailure> {
        if let Some(thread) = self.thread.take() {
            thread.join().map_err(|_| {
                AcceptanceFailure::infrastructure("runner_thread", "runner thread panicked")
            })?;
        }
        let stderr = process::read_bounded(&self.stderr_path, MAXIMUM_COMMAND_OUTPUT_BYTES)
            .map_err(|error| AcceptanceFailure::infrastructure("runner_stderr", error))?;
        if !stderr.is_empty() {
            return Err(AcceptanceFailure::acceptance(
                "runner_stderr",
                "classified resident workflow wrote to stderr",
            ));
        }
        Ok(())
    }
}

fn parse_ready(line: &[u8]) -> Result<ReadyObservation, AcceptanceFailure> {
    let value: Value = serde_json::from_slice(line)
        .map_err(|error| AcceptanceFailure::infrastructure("ready_json", error))?;
    if value.get("event").and_then(Value::as_str) != Some("ready")
        || value.get("ok").and_then(Value::as_bool) != Some(true)
    {
        return Err(AcceptanceFailure::acceptance(
            "ready_event",
            "first resident event is not successful readiness",
        ));
    }
    let deployment = value.get("deployment").ok_or_else(|| {
        AcceptanceFailure::acceptance("ready_deployment", "ready event omitted deployment")
    })?;
    let grants = deployment
        .get("grants")
        .and_then(Value::as_object)
        .ok_or_else(|| AcceptanceFailure::acceptance("ready_grants", "ready event omitted grants"))?
        .iter()
        .map(|(name, value)| {
            value
                .as_str()
                .map(|kind| (name.clone(), kind.to_owned()))
                .ok_or_else(|| {
                    AcceptanceFailure::acceptance("ready_grants", "ready grant is not text")
                })
        })
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    Ok(ReadyObservation {
        artifact_digest: json_text(deployment, "artifact_digest", "ready artifact")?,
        target: json_text(deployment, "target", "ready target")?,
        runner: json_text(deployment, "runner", "ready runner")?,
        configured_listener: json_text(deployment, "listen", "ready listener")?,
        local_address: json_text(&value, "local_address", "bound listener")?,
        grants,
        elapsed_nanoseconds: 0,
    })
}

fn parse_stopped(bytes: &[u8]) -> Result<StoppedObservation, AcceptanceFailure> {
    let line = bytes
        .split(|byte| *byte == b'\n')
        .find(|line| {
            serde_json::from_slice::<Value>(line)
                .ok()
                .and_then(|value| {
                    value
                        .get("event")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                })
                .as_deref()
                == Some("stopped")
        })
        .ok_or_else(|| {
            AcceptanceFailure::acceptance("stopped_event", "runner omitted stopped event")
        })?;
    let value: Value = serde_json::from_slice(line)
        .map_err(|error| AcceptanceFailure::infrastructure("stopped_json", error))?;
    let receipt = value.get("receipt").ok_or_else(|| {
        AcceptanceFailure::acceptance("stopped_receipt", "stopped event omitted receipt")
    })?;
    let shutdown = receipt.get("shutdown").ok_or_else(|| {
        AcceptanceFailure::acceptance("stopped_shutdown", "stopped receipt omitted shutdown")
    })?;
    Ok(StoppedObservation {
        local_address: json_text(receipt, "local_address", "stopped listener")?,
        admission_stopped: shutdown
            .get("admission_stopped")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        remaining_tasks: shutdown
            .get("remaining_tasks")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX),
        cleanup_failures: shutdown
            .get("cleanup_failures")
            .and_then(Value::as_array)
            .map(|values| values.len() as u64)
            .unwrap_or(u64::MAX),
        elapsed_nanoseconds: shutdown
            .get("elapsed_nanoseconds")
            .and_then(Value::as_u64)
            .unwrap_or(u64::MAX),
    })
}

fn validate_starter_files(
    project: &Path,
    descriptor: &Path,
    artifact: &Path,
) -> Result<(), AcceptanceFailure> {
    let descriptor_metadata = fs::symlink_metadata(descriptor)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_metadata", error))?;
    if descriptor_metadata.file_type().is_symlink() || !descriptor_metadata.is_file() {
        return Err(AcceptanceFailure::acceptance(
            "descriptor_kind",
            "starter descriptor is not a regular file",
        ));
    }
    let descriptor_bytes = process::read_bounded(descriptor, 1024 * 1024)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_read", error))?;
    if descriptor_bytes.last() != Some(&b'\n') {
        return Err(AcceptanceFailure::acceptance(
            "descriptor_newline",
            "starter descriptor lacks its canonical final newline",
        ));
    }
    let value: Value = serde_json::from_slice(&descriptor_bytes)
        .map_err(|error| AcceptanceFailure::infrastructure("descriptor_json", error))?;
    if value.get("contract_version").and_then(Value::as_u64) != Some(1)
        || value.get("artifact").and_then(Value::as_str) != Some("generated/application.lkja")
        || value.get("target").and_then(Value::as_str) != Some("serve")
        || value.get("listen").and_then(Value::as_str) != Some("127.0.0.1:0")
        || value
            .get("configuration")
            .and_then(Value::as_object)
            .map(|map| map.is_empty())
            != Some(true)
        || value
            .get("secrets")
            .and_then(Value::as_array)
            .map(Vec::is_empty)
            != Some(true)
    {
        return Err(AcceptanceFailure::acceptance(
            "descriptor_defaults",
            "starter descriptor disagrees with deployment-contract-1 loopback defaults",
        ));
    }
    let authority = value
        .pointer("/grants/0/authority_revision")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "descriptor_authority",
                "starter stream grant omitted authority revision",
            )
        })?;
    if authority.len() != 64 || authority.bytes().all(|byte| byte == b'0') {
        return Err(AcceptanceFailure::acceptance(
            "descriptor_authority",
            "starter stream authority is not a fresh nonzero canonical digest",
        ));
    }
    let generated = project.join("generated");
    let generated_metadata = fs::symlink_metadata(&generated)
        .map_err(|error| AcceptanceFailure::infrastructure("generated_metadata", error))?;
    if generated_metadata.file_type().is_symlink()
        || !generated_metadata.is_dir()
        || fs::read_dir(&generated)
            .map_err(|error| AcceptanceFailure::infrastructure("generated_read", error))?
            .next()
            .is_some()
        || artifact.exists()
    {
        return Err(AcceptanceFailure::acceptance(
            "generated_initial_state",
            "HTTP recipe emitted a prebuilt artifact or a foreign generated entry",
        ));
    }
    Ok(())
}

fn validate_next_actions(records: &[CompactRecord]) -> Result<(), AcceptanceFailure> {
    let actions = records
        .iter()
        .filter(|record| record.operation == "next")
        .map(|record| {
            Ok((
                parse_u64(required_field(record, "order")?, "next order")?,
                required_field(record, "kind")?.to_owned(),
            ))
        })
        .collect::<Result<Vec<_>, AcceptanceFailure>>()?;
    let expected = vec![
        (1, "status".to_owned()),
        (2, "check".to_owned()),
        (3, "response-change-plan".to_owned()),
        (4, "response-change-apply".to_owned()),
        (5, "build".to_owned()),
        (6, "serve".to_owned()),
    ];
    if actions != expected {
        return Err(AcceptanceFailure::acceptance(
            "next_actions",
            "new HTTP output does not expose the exact ordered workflow",
        ));
    }
    Ok(())
}

fn require_revision(
    context: &mut AcceptanceContext,
    cwd: &Path,
    project: &Path,
    expected: &str,
    label: &str,
) -> Result<(), AcceptanceFailure> {
    let name = format!("status-{}", safe_name(label)?);
    let output = context.invoke_success(
        &name,
        vec![
            "--project".to_owned(),
            project.display().to_string(),
            "status".to_owned(),
        ],
        cwd,
    )?;
    let records = compact_records(label, &output.stdout)?;
    require_exact(
        required_field(required_record(&records, "revision")?, "id")?,
        expected,
        label,
    )
}

fn compiler_observation(
    records: &[CompactRecord],
) -> Result<CompilerObservation, AcceptanceFailure> {
    let record = required_record(records, "compilation")?;
    Ok(CompilerObservation {
        cache: required_field(record, "cache")?.to_owned(),
        compiled: parse_u64(required_field(record, "compiled")?, "compiled units")?,
        reused: parse_u64(required_field(record, "reused")?, "reused units")?,
        removed: parse_u64(required_field(record, "removed")?, "removed units")?,
    })
}

fn compact_records(label: &str, bytes: &[u8]) -> Result<Vec<CompactRecord>, AcceptanceFailure> {
    parse_records(label, bytes).map_err(|diagnostics| {
        let codes = diagnostics
            .iter()
            .map(|diagnostic| diagnostic.code.as_str())
            .collect::<Vec<_>>()
            .join(",");
        AcceptanceFailure::acceptance(
            "compact_output",
            format!("{label} output is not strict compact records ({codes})"),
        )
    })
}

fn required_record<'a>(
    records: &'a [CompactRecord],
    operation: &str,
) -> Result<&'a CompactRecord, AcceptanceFailure> {
    records
        .iter()
        .find(|record| record.operation == operation)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "compact_record",
                format!("compact output omitted '{operation}'"),
            )
        })
}

fn required_field<'a>(record: &'a CompactRecord, name: &str) -> Result<&'a str, AcceptanceFailure> {
    record
        .fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| field.value.as_str())
        .ok_or_else(|| {
            AcceptanceFailure::acceptance(
                "compact_field",
                format!("compact '{}' record omitted '{name}'", record.operation),
            )
        })
}

fn require_field(
    records: &[CompactRecord],
    operation: &str,
    field: &str,
    expected: &str,
) -> Result<(), AcceptanceFailure> {
    require_exact(
        required_field(required_record(records, operation)?, field)?,
        expected,
        field,
    )
}

fn require_exact(actual: &str, expected: &str, label: &str) -> Result<(), AcceptanceFailure> {
    if actual == expected {
        Ok(())
    } else {
        Err(AcceptanceFailure::acceptance(
            "exact_output",
            format!("{label} is '{actual}', expected '{expected}'"),
        ))
    }
}

fn parse_u64(value: &str, label: &str) -> Result<u64, AcceptanceFailure> {
    value.parse::<u64>().map_err(|error| {
        AcceptanceFailure::infrastructure(
            "numeric_output",
            format!("{label} is not an unsigned integer: {error}"),
        )
    })
}

fn json_text(value: &Value, field: &str, label: &str) -> Result<String, AcceptanceFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            AcceptanceFailure::acceptance("runtime_event_field", format!("{label} is absent"))
        })
}

fn first_line(path: &Path) -> Result<Option<Vec<u8>>, AcceptanceFailure> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(AcceptanceFailure::infrastructure("runner_log", error)),
    };
    if bytes.len() as u64 > MAXIMUM_COMMAND_OUTPUT_BYTES {
        return Err(AcceptanceFailure::acceptance(
            "runner_log_limit",
            "resident stdout exceeded the acceptance bound",
        ));
    }
    Ok(bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|end| bytes[..end].to_vec()))
}

fn publish_product_input(path: &Path, bytes: &[u8]) -> Result<(), AcceptanceFailure> {
    evidence::publish(path, bytes)
        .map(|_| ())
        .map_err(|error| AcceptanceFailure::infrastructure("product_input", error))
}

fn copy_binary(source: &Path, destination: &Path) -> Result<(), DevError> {
    let metadata = fs::symlink_metadata(source).map_err(|error| {
        DevError::infrastructure(format!("inspect candidate '{}': {error}", source.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::usage(
            "candidate binary must be a regular non-symlink file",
        ));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage("candidate binary is not executable"));
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
            .ok_or_else(|| DevError::infrastructure("copied binary destination has no parent"))?,
    )?
    .sync_all()?;
    Ok(())
}

fn parse_options(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut binary = None;
    let mut evidence_root = None;
    let mut machine = false;
    let mut arguments = arguments;
    while let Some(argument) = crate::next_utf8(&mut arguments, "distributed HTTP option")? {
        match argument.as_str() {
            "--binary" => {
                if binary.is_some() {
                    return Err(DevError::usage("duplicate --binary option"));
                }
                let value = crate::next_utf8(&mut arguments, "--binary path")?
                    .ok_or_else(|| DevError::usage("--binary requires a path"))?;
                binary = Some(PathBuf::from(value));
            }
            "--evidence-root" => {
                if evidence_root.is_some() {
                    return Err(DevError::usage("duplicate --evidence-root option"));
                }
                let value = crate::next_utf8(&mut arguments, "--evidence-root path")?
                    .ok_or_else(|| DevError::usage("--evidence-root requires a path"))?;
                evidence_root = Some(PathBuf::from(value));
            }
            "--machine" => {
                if machine {
                    return Err(DevError::usage("duplicate --machine option"));
                }
                machine = true;
            }
            other => return Err(DevError::usage(format!("unknown option '{other}'"))),
        }
    }
    Ok(Options {
        binary: binary.unwrap_or_else(|| PathBuf::from("target/release/lkjscript")),
        evidence_root,
        machine,
    })
}

fn resolve_candidate(checkout_root: Option<&Path>, binary: &Path) -> Result<PathBuf, DevError> {
    let path = if binary.is_absolute() {
        binary.to_path_buf()
    } else {
        let repository = checkout_root.ok_or_else(|| {
            DevError::usage(
                "--binary must be absolute when an explicit --evidence-root selects transferred mode",
            )
        })?;
        repository.join(binary)
    };
    resolve_regular_executable(
        &path,
        "distributed HTTP candidate",
        MAXIMUM_CANDIDATE_EXECUTABLE_BYTES,
    )
}

fn current_verifier() -> Result<PathBuf, DevError> {
    let path = std::env::current_exe().map_err(|error| {
        DevError::infrastructure(format!("resolve distributed HTTP verifier: {error}"))
    })?;
    resolve_regular_executable(
        &path,
        "distributed HTTP verifier",
        MAXIMUM_VERIFIER_EXECUTABLE_BYTES,
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
    if metadata.len() > maximum_bytes {
        return Err(DevError::usage(format!(
            "{label} '{}' exceeds {maximum_bytes} bytes",
            path.display()
        )));
    }
    #[cfg(unix)]
    if metadata.permissions().mode() & 0o111 == 0 {
        return Err(DevError::usage(format!(
            "{label} '{}' is not executable",
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
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted file mode")))?;
    let verification_digest = file
        .digest
        .clone()
        .ok_or_else(|| DevError::infrastructure(format!("{label} proof omitted digest")))?;
    let sha256 = sha256_file(&path, maximum_bytes)?;
    Ok(ExecutableObservation {
        file,
        byte_length,
        mode,
        executable: mode & 0o111 != 0,
        sha256,
        verification_digest,
    })
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
    let parent = requested.parent().ok_or_else(|| {
        DevError::usage(format!(
            "evidence root '{}' has no parent directory",
            requested.display()
        ))
    })?;
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
    let name = requested.file_name().ok_or_else(|| {
        DevError::usage(format!(
            "evidence root '{}' has no private directory name",
            requested.display()
        ))
    })?;
    let root = canonical_parent.join(name);
    fs::create_dir(&root).map_err(|error| {
        DevError::infrastructure(format!(
            "create evidence root '{}': {error}",
            root.display()
        ))
    })?;
    let result = (|| {
        #[cfg(unix)]
        fs::set_permissions(&root, fs::Permissions::from_mode(0o700)).map_err(|error| {
            DevError::infrastructure(format!(
                "set evidence-root mode '{}': {error}",
                root.display()
            ))
        })?;
        File::open(&canonical_parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| {
                DevError::infrastructure(format!(
                    "synchronize evidence-root parent '{}': {error}",
                    canonical_parent.display()
                ))
            })?;
        let canonical_root = root.canonicalize().map_err(|error| {
            DevError::infrastructure(format!(
                "resolve evidence root '{}': {error}",
                root.display()
            ))
        })?;
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

fn has_noncanonical_component(path: &Path) -> bool {
    path.components()
        .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
}

fn isolated_environment() -> BTreeMap<String, String> {
    BTreeMap::from([("LANG".to_owned(), "C".to_owned())])
}

fn safe_name(value: &str) -> Result<String, AcceptanceFailure> {
    let normalized = value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || byte == b'-' {
                char::from(byte)
            } else {
                '-'
            }
        })
        .collect::<String>();
    if normalized.is_empty() || normalized.bytes().all(|byte| byte == b'-') {
        Err(AcceptanceFailure::acceptance(
            "evidence_name",
            "evidence name is not a safe file component",
        ))
    } else {
        Ok(normalized)
    }
}

fn new_evidence_directory(repository: &Path) -> Result<PathBuf, DevError> {
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let parent = repository.join(".artifacts/lkjscript-dev/distributed-http");
    fs::create_dir_all(&parent).map_err(|error| {
        DevError::infrastructure(format!(
            "create distributed HTTP evidence parent '{}': {error}",
            parent.display()
        ))
    })?;
    let metadata = fs::symlink_metadata(&parent).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect distributed HTTP evidence parent '{}': {error}",
            parent.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(DevError::infrastructure(
            "distributed HTTP evidence parent is not a regular non-symlink directory",
        ));
    }
    let directory = parent.join(format!(
        "{}-{}-{ordinal}",
        unix_nanoseconds()?,
        std::process::id()
    ));
    fs::create_dir(&directory).map_err(|error| {
        DevError::infrastructure(format!(
            "create distributed HTTP evidence directory '{}': {error}",
            directory.display()
        ))
    })?;
    Ok(directory)
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| DevError::infrastructure("lkjscript-dev package escaped its workspace"))?
        .canonicalize()
        .map_err(|error| DevError::infrastructure(format!("resolve repository root: {error}")))
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    lowercase_hex(&digest)
}

fn lowercase_hex(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len().saturating_mul(2));
    for byte in bytes {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        output.push(char::from(HEX[usize::from(*byte >> 4)]));
        output.push(char::from(HEX[usize::from(*byte & 0x0f)]));
    }
    output
}

fn sha256_file(path: &Path, maximum_bytes: u64) -> Result<String, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect SHA-256 input '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum_bytes {
        return Err(DevError::infrastructure(format!(
            "SHA-256 input '{}' is unsafe or exceeds {maximum_bytes} bytes",
            path.display()
        )));
    }
    let mut input = File::open(path).map_err(|error| {
        DevError::infrastructure(format!("open SHA-256 input '{}': {error}", path.display()))
    })?;
    let mut hasher = Sha256::new();
    let mut observed = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = input.read(&mut buffer).map_err(|error| {
            DevError::infrastructure(format!("read SHA-256 input '{}': {error}", path.display()))
        })?;
        if read == 0 {
            break;
        }
        observed = observed.checked_add(read as u64).ok_or_else(|| {
            DevError::infrastructure("distributed HTTP SHA-256 byte length overflow")
        })?;
        hasher.update(&buffer[..read]);
    }
    if observed != metadata.len() {
        return Err(DevError::infrastructure(format!(
            "SHA-256 input '{}' changed while reading",
            path.display()
        )));
    }
    Ok(lowercase_hex(&hasher.finalize()))
}

fn print_summary(
    options: &Options,
    receipt: &AcceptanceReceipt,
    published: &PublishedEvidence,
    receipt_sha256: &str,
) -> Result<(), DevError> {
    if options.machine {
        let summary = serde_json::json!({
            "status": receipt.status,
            "schema": receipt.schema,
            "workflow": receipt.workflow,
            "receipt": published.path,
            "receipt_bytes": published.bytes,
            "receipt_digest": published.digest,
            "receipt_sha256": receipt_sha256,
            "verifier_sha256": receipt.verifier.sha256,
            "candidate_sha256": receipt.candidate.sha256,
            "elapsed_nanoseconds": receipt.elapsed_nanoseconds,
            "commands": receipt.commands.len(),
            "runners": receipt.runners.len(),
        });
        println!(
            "{}",
            serde_json::to_string(&summary).map_err(|error| {
                DevError::infrastructure(format!("encode distributed HTTP summary: {error}"))
            })?
        );
    } else {
        println!(
            "distributed HTTP application: status={:?} commands={} runners={} receipt={} digest={}",
            receipt.status,
            receipt.commands.len(),
            receipt.runners.len(),
            published.path.display(),
            published.digest
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(values: &[&str]) -> Result<Options, DevError> {
        parse_options(values.iter().map(OsString::from))
    }

    #[test]
    fn distributed_http_acceptance_schema_is_stable_and_campaign_independent() {
        let schema = SchemaIdentity {
            identity: ACCEPTANCE_SCHEMA.to_owned(),
            version: ACCEPTANCE_SCHEMA_VERSION,
        };
        assert_eq!(
            serde_json::to_string(&schema).expect("encode acceptance schema"),
            r#"{"identity":"lkjscript-distributed-http-acceptance","version":2}"#
        );
        assert_eq!(ACCEPTANCE_WORKFLOW, "distributed-http-application");
    }

    #[test]
    fn distributed_http_options_reject_duplicates_and_missing_values() {
        assert!(options(&["--binary"]).is_err());
        assert!(options(&["--evidence-root"]).is_err());
        assert!(options(&["--binary", "one", "--binary", "two"]).is_err());
        assert!(options(&["--evidence-root", "/tmp/one", "--evidence-root", "/tmp/two"]).is_err());
        assert!(options(&["--machine", "--machine"]).is_err());
        assert!(options(&["--unknown"]).is_err());
        assert!(resolve_candidate(None, Path::new("relative-candidate")).is_err());
        assert!(resolve_candidate(None, Path::new("/definitely/missing/candidate")).is_err());
    }

    #[test]
    fn explicit_evidence_root_is_absolute_private_and_create_new() {
        let temporary = tempfile::tempdir().expect("temporary evidence-root parent");
        assert!(create_explicit_evidence_root(Path::new("relative")).is_err());
        let root = temporary.path().join("acceptance");
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
    fn explicit_evidence_root_rejects_existing_and_symlinked_boundaries() {
        let temporary = tempfile::tempdir().expect("temporary evidence-root fixtures");
        let file = temporary.path().join("file");
        fs::write(&file, b"retained").expect("write conflict file");
        let directory = temporary.path().join("directory");
        fs::create_dir(&directory).expect("create conflict directory");
        let link = temporary.path().join("link");
        std::os::unix::fs::symlink(&directory, &link).expect("create conflict link");
        for path in [&file, &directory, &link] {
            assert!(create_explicit_evidence_root(path).is_err());
        }
        assert!(create_explicit_evidence_root(&link.join("escaped")).is_err());
        assert_eq!(fs::read(file).expect("read retained file"), b"retained");
    }

    #[cfg(unix)]
    #[test]
    fn executable_observation_binds_bytes_mode_and_both_digests() {
        let temporary = tempfile::tempdir().expect("temporary executable fixtures");
        let source = current_verifier().expect("current verifier");
        let candidate = temporary.path().join("candidate");
        fs::copy(source, &candidate).expect("copy candidate fixture");
        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o755))
            .expect("set candidate mode");
        let observed =
            executable_observation(&candidate, "fixture", MAXIMUM_CANDIDATE_EXECUTABLE_BYTES)
                .expect("observe fixture");
        assert!(observed.executable);
        assert_eq!(observed.mode & 0o7777, 0o755);
        assert_eq!(observed.sha256.len(), 64);
        assert_eq!(
            observed.byte_length,
            fs::metadata(&candidate).expect("metadata").len()
        );

        fs::set_permissions(&candidate, fs::Permissions::from_mode(0o644))
            .expect("remove executable mode");
        assert!(
            executable_observation(&candidate, "fixture", MAXIMUM_CANDIDATE_EXECUTABLE_BYTES,)
                .is_err()
        );
        let link = temporary.path().join("candidate-link");
        std::os::unix::fs::symlink(&candidate, &link).expect("create candidate link");
        assert!(
            executable_observation(&link, "fixture", MAXIMUM_CANDIDATE_EXECUTABLE_BYTES).is_err()
        );

        let oversized = temporary.path().join("oversized");
        let oversized_file = File::create(&oversized).expect("create oversized fixture");
        oversized_file
            .set_len(MAXIMUM_CANDIDATE_EXECUTABLE_BYTES.saturating_add(1))
            .expect("size oversized fixture");
        fs::set_permissions(&oversized, fs::Permissions::from_mode(0o755))
            .expect("set oversized mode");
        assert!(
            executable_observation(&oversized, "fixture", MAXIMUM_CANDIDATE_EXECUTABLE_BYTES,)
                .is_err()
        );
    }

    #[test]
    fn distributed_http_sha256_file_uses_standard_single_hash_identity() {
        let temporary = tempfile::tempdir().expect("temporary SHA-256 fixture");
        let path = temporary.path().join("bytes");
        fs::write(&path, b"abc").expect("write SHA-256 fixture");
        assert_eq!(
            sha256_file(&path, 3).expect("hash fixture"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert!(sha256_file(&path, 2).is_err());
    }
}
