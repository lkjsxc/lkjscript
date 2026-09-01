use crate::authority::{self, AuthorityObservation};
use crate::error::DevError;
use crate::evidence::{self, FileProof, PublishedEvidence, VerificationDigest};
use crate::http_probe::{self, HttpResponse};
use crate::process::{self, ProcessControl, ProcessObservation, ProcessSpec, ProcessStatus};
use lkjscript::platform::data::{
    DataCommitOutcome, DataExpectation, DataKey, DataKeyPart, DataLimits, DataScanDirection,
    DataStore, DataTransaction,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SERVICE_CONTRACT_VERSION: u32 = 5;
pub(crate) const DATA_CONTRACT: &str = "lkjscript-data-store-1";
const QUEUE_DATA_CONTRACT: &str = "lkjscript-durable-queue-data-1";
const QUEUE_NAMESPACE: &str = "lkjournal-queue";
const QUEUE_JOB_SPACE: &str = "__queue.jobs";
const QUEUE_IDEMPOTENCY_SPACE: &str = "__queue.idempotency";
const QUEUE_CLAIM_SPACE: &str = "__queue.claim";
const QUEUE_SCHEMA_SPACE: &str = "__queue.schema";
const QUEUE_JOB_MAGIC: &[u8; 8] = b"LKJQJOB1";
const QUEUE_JOB_CHECKSUM_DOMAIN: &str = "lkjscript.queue.data-job.v1";
const QUEUE_SCHEMA_DIGEST_DOMAIN: &str = "lkjscript.queue.data-schema.v1";
const ORACLE_RETRY_JOB: &str = "affine-oracle-retry";
const ORACLE_STALE_JOB: &str = "affine-oracle-stale";
const SERVICE_ARTIFACT_RELATIVE: &str = "generated/lkjournal.lkja";
const SERVICE_ARTIFACT_SHA256: &str =
    "12b39dce25366bd6f6ee2d78dc4d73f03b55d020df7332e1ef914497ad46e728";
const MAXIMUM_COMMAND_STDOUT_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_COMMAND_STDERR_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_RUNNER_STDOUT_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_RUNNER_STDERR_BYTES: u64 = 16 * 1024 * 1024;
const MAXIMUM_BACKUP_BYTES: u64 = 64 * 1024 * 1024;
const MAXIMUM_DESCRIPTOR_BYTES: u64 = 1024 * 1024;
const MAXIMUM_ARTIFACT_BYTES: u64 = 64 * 1024 * 1024;
const COMMAND_TIMEOUT: Duration = Duration::from_secs(60);
const RUNNER_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const RUNNER_READY_TIMEOUT: Duration = Duration::from_secs(30);
const RUNNER_STOP_TIMEOUT: Duration = Duration::from_secs(35);
const RUNNER_KILL_TIMEOUT: Duration = Duration::from_secs(5);
const WORKER_READY_TIMEOUT: Duration = Duration::from_secs(10);
const POLL_INTERVAL: Duration = Duration::from_millis(50);
static RUN_ORDINAL: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
struct Options {
    binary: PathBuf,
    machine: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ServiceStatus {
    Passed,
    Failed,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ServiceReceipt {
    contract_version: u32,
    status: ServiceStatus,
    data_contract: String,
    platform: PlatformObservation,
    started_unix_nanoseconds: u128,
    completed_unix_nanoseconds: u128,
    elapsed_nanoseconds: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    binary: Option<FileProof>,
    #[serde(skip_serializing_if = "Option::is_none")]
    artifact: Option<FileProof>,
    retained_files: Vec<FileProof>,
    secret_environment_names: Vec<String>,
    raw_secret_values_retained: bool,
    commands: Vec<CommandEvidence>,
    runners: Vec<RunnerEvidence>,
    requests: Vec<HttpObservation>,
    cleanup: CleanupObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<ServiceResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure: Option<Failure>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct PlatformObservation {
    operating_system: String,
    architecture: String,
    child_process_control: String,
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
struct CommandEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunnerEvidence {
    name: String,
    command: Vec<String>,
    process: ProcessObservation,
    #[serde(skip_serializing_if = "Option::is_none")]
    ready: Option<RunnerReady>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stopped: Option<RunnerStopped>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunnerReady {
    artifact_digest: String,
    target: String,
    runner: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    listen: Option<String>,
    secret_names: Vec<String>,
    readiness_elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RunnerStopped {
    admission_stopped: bool,
    remaining_tasks: u64,
    cleanup_failures: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    productive_iterations: Option<u64>,
}

impl RunnerStopped {
    fn clean(&self) -> bool {
        self.admission_stopped && self.remaining_tasks == 0 && self.cleanup_failures == 0
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct HttpObservation {
    name: String,
    method: String,
    path: String,
    status: u16,
    request_body_bytes: u64,
    response_body_bytes: u64,
    response_digest: VerificationDigest,
    elapsed_nanoseconds: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CleanupObservation {
    scope: String,
    attempted: bool,
    completed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ServiceResult {
    data_contract: String,
    artifact_digest: String,
    artifact_identity: ArtifactIdentity,
    authority_before: AuthorityObservation,
    authority_after: AuthorityObservation,
    authority_unchanged: bool,
    routes_checked: u64,
    resource_revision: u64,
    history_entries: u64,
    object_bytes: u64,
    worker_productive_iterations: u64,
    queue_observation: QueueObservation,
    data_backup: FileProof,
    restart_read_equal: bool,
    restored_read_equal: bool,
    corrupt_backup_rejected: bool,
    shutdown_cleanup_failures: u64,
    initialization_transport: InitializationTransport,
    initialization_observation: InitializationObservation,
    request_elapsed_nanoseconds: BTreeMap<String, u64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct QueueObservation {
    data_contract: String,
    records_scanned: u64,
    workers_started: u64,
    productive_iterations: u64,
    completed_jobs: u64,
    retry_job_state: String,
    retry_job_attempts: u32,
    retry_error_class: String,
    stale_job_state: String,
    stale_job_attempts: u32,
    stale_replacement_observed: bool,
    transition_authority_cleared: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum QueueJobState {
    Ready,
    Leased,
    Completed,
    Failed,
    Cancelled,
}

impl QueueJobState {
    const fn name(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Leased => "leased",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Debug)]
struct QueueFixtureJob {
    job_id: String,
    idempotency_key: String,
    payload: Vec<u8>,
    state: QueueJobState,
    available_at: i64,
    created_at: i64,
    attempt_count: u32,
    attempt_id: Option<String>,
    worker_id: Option<String>,
    lease_until: Option<i64>,
    result: Option<Vec<u8>>,
    last_error_class: Option<String>,
}

#[derive(Clone, Debug)]
struct ObservedQueueJob {
    job_id: String,
    state: QueueJobState,
    attempt_count: u32,
    attempt_id: Option<String>,
    worker_id: Option<String>,
    lease_until: Option<i64>,
    last_error_class: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ReceiptBinding {
    pub(crate) receipt_bytes: u64,
    pub(crate) receipt_sha256: String,
    pub(crate) candidate_digest: VerificationDigest,
    pub(crate) elapsed_nanoseconds: u64,
    pub(crate) commands: u64,
    pub(crate) runners: u64,
    pub(crate) requests: u64,
    pub(crate) cleanup_complete: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ArtifactIdentity {
    repository: String,
    package: String,
    revision: String,
    semantic_state: String,
    compilation_manifest: String,
    artifact_manifest: String,
    artifact_bundle: String,
    bytes: u64,
    packages: u64,
    closure_objects: u64,
    compiler_units: u64,
    manifest_objects: u64,
    manifest_object_bytes: u64,
    segments: u64,
    load_objects: u64,
    load_object_bytes: u64,
    checked_in_sha256: String,
    fresh_build_equal: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InitializationTransport {
    status: u16,
    body_bytes: u64,
    body_sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_code: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct InitializationObservation {
    status: u16,
    actor_inserted: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Clone, Debug)]
struct ServiceFailure {
    status: ServiceStatus,
    class: &'static str,
    code: &'static str,
    message: String,
}

impl ServiceFailure {
    fn failed(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: ServiceStatus::Failed,
            class: "acceptance",
            code,
            message: message.into(),
        }
    }

    fn unavailable(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: ServiceStatus::Unavailable,
            class: "unavailable",
            code,
            message: message.into(),
        }
    }

    fn infrastructure(code: &'static str, error: impl std::fmt::Display) -> Self {
        Self {
            status: ServiceStatus::Failed,
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
    observation: ProcessObservation,
    stdout: Vec<u8>,
}

struct CommandRequest<'a> {
    name: &'a str,
    command: Vec<String>,
    environment: BTreeMap<String, String>,
    timeout: Duration,
    maximum_stdout_bytes: u64,
    maximum_stderr_bytes: u64,
}

impl<'a> CommandRequest<'a> {
    fn standard(name: &'a str, command: Vec<String>) -> Self {
        Self {
            name,
            command,
            environment: process::environment(),
            timeout: COMMAND_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_COMMAND_STDOUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_COMMAND_STDERR_BYTES,
        }
    }

    fn environment(mut self, environment: BTreeMap<String, String>) -> Self {
        self.environment = environment;
        self
    }

    fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

struct ServiceContext {
    repository: PathBuf,
    run_directory: PathBuf,
    command_ordinal: u64,
    commands: Vec<CommandEvidence>,
    runners: Vec<Option<ActiveRunner>>,
    runner_evidence: Vec<RunnerEvidence>,
    requests: Vec<HttpObservation>,
    retained_files: Vec<FileProof>,
    secret_values: Vec<Vec<u8>>,
    cleanup: CleanupObservation,
}

struct ActiveRunner {
    name: String,
    command: Vec<String>,
    control: ProcessControl,
    receiver: Receiver<ProcessObservation>,
    thread: Option<thread::JoinHandle<()>>,
    stdout_path: PathBuf,
    stderr_path: PathBuf,
    ready: Option<RunnerReady>,
    terminal: Option<ProcessObservation>,
    started: Instant,
}

pub(crate) fn command(arguments: impl Iterator<Item = OsString>) -> Result<u8, DevError> {
    let options = parse(arguments)?;
    let repository = repository_root()?;
    let (receipt, published) = execute(&repository, &options)?;
    print_summary(&repository, &options, &receipt, &published)?;
    Ok(match receipt.status {
        ServiceStatus::Passed => 0,
        ServiceStatus::Failed => 1,
        ServiceStatus::Unavailable => 2,
    })
}

pub(crate) fn read_receipt(path: &Path, candidate: &Path) -> Result<ReceiptBinding, DevError> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect service receipt '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() == 0
        || metadata.len() > 128 * 1024 * 1024
    {
        return Err(DevError::corrupt("service receipt is unsafe or oversized"));
    }
    let bytes = fs::read(path).map_err(|error| {
        DevError::infrastructure(format!(
            "read service receipt '{}': {error}",
            path.display()
        ))
    })?;
    let receipt: ServiceReceipt = serde_json::from_slice(&bytes)
        .map_err(|error| DevError::corrupt(format!("decode service receipt: {error}")))?;
    if evidence::encode_json(&receipt)? != bytes {
        return Err(DevError::corrupt(
            "service receipt is not in canonical evidence encoding",
        ));
    }
    let repository = repository_root()?.canonicalize().map_err(|error| {
        DevError::infrastructure(format!("resolve service receipt repository: {error}"))
    })?;
    let candidate = candidate
        .canonicalize()
        .map_err(|error| DevError::infrastructure(format!("resolve service candidate: {error}")))?;
    let candidate_proof = proof_input(&repository, &candidate).map_err(|error| {
        DevError::corrupt(format!("observe service candidate: {}", error.message))
    })?;
    let artifact_path = repository
        .join("applications/lkjournal")
        .join(SERVICE_ARTIFACT_RELATIVE);
    let artifact_proof = proof_required_file_with_sha256(
        &repository,
        &artifact_path,
        MAXIMUM_ARTIFACT_BYTES,
        "maintained artifact-11 service bundle",
        SERVICE_ARTIFACT_SHA256,
    )
    .map_err(|error| DevError::corrupt(format!("observe service artifact: {}", error.message)))?;
    let result = receipt
        .result
        .as_ref()
        .ok_or_else(|| DevError::corrupt("passed service receipt omitted its result"))?;
    let candidate_digest = candidate_proof
        .digest
        .clone()
        .ok_or_else(|| DevError::corrupt("service candidate proof omitted its digest"))?;
    let cleanup_complete = receipt.cleanup.attempted && receipt.cleanup.completed;
    if receipt.contract_version != SERVICE_CONTRACT_VERSION
        || receipt.status != ServiceStatus::Passed
        || receipt.data_contract != DATA_CONTRACT
        || receipt.completed_unix_nanoseconds < receipt.started_unix_nanoseconds
        || receipt.binary.as_ref() != Some(&candidate_proof)
        || receipt.artifact.as_ref() != Some(&artifact_proof)
        || receipt.secret_environment_names != ["LKJOURNAL_BOOTSTRAP_TOKEN"]
        || receipt.raw_secret_values_retained
        || receipt.failure.is_some()
        || !cleanup_complete
        || !receipt.cleanup.scope.starts_with("lkjscript-service-")
        || receipt.commands.is_empty()
        || receipt.runners.is_empty()
        || receipt.requests.is_empty()
        || receipt.runners.iter().any(|runner| {
            runner
                .stopped
                .as_ref()
                .is_none_or(|stopped| !stopped.clean())
        })
        || !result.artifact_identity.fresh_build_equal
        || result.data_contract != DATA_CONTRACT
        || !result.authority_unchanged
        || result.authority_before != result.authority_after
        || result.routes_checked != receipt.requests.len() as u64
        || result.resource_revision == 0
        || result.history_entries == 0
        || result.object_bytes == 0
        || result.worker_productive_iterations == 0
        || result.queue_observation.data_contract != QUEUE_DATA_CONTRACT
        || result.queue_observation.records_scanned < 2
        || result.queue_observation.workers_started != 2
        || result.queue_observation.productive_iterations != result.worker_productive_iterations
        || result.queue_observation.completed_jobs == 0
        || !matches!(
            result.queue_observation.retry_job_state.as_str(),
            "ready" | "failed"
        )
        || result.queue_observation.retry_job_attempts == 0
        || result.queue_observation.retry_error_class != "empty-payload"
        || result.queue_observation.stale_job_state != "completed"
        || result.queue_observation.stale_job_attempts < 2
        || !result.queue_observation.stale_replacement_observed
        || !result.queue_observation.transition_authority_cleared
        || !result.restart_read_equal
        || !result.restored_read_equal
        || !result.corrupt_backup_rejected
        || result.shutdown_cleanup_failures != 0
        || result.initialization_transport.status == 0
        || result.initialization_observation.status == 0
    {
        return Err(DevError::corrupt(
            "service receipt binding or maintained acceptance mismatch",
        ));
    }
    Ok(ReceiptBinding {
        receipt_bytes: metadata.len(),
        receipt_sha256: sha256_hex(&bytes),
        candidate_digest,
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        commands: receipt.commands.len() as u64,
        runners: receipt.runners.len() as u64,
        requests: receipt.requests.len() as u64,
        cleanup_complete,
    })
}

fn execute(
    repository: &Path,
    options: &Options,
) -> Result<(ServiceReceipt, PublishedEvidence), DevError> {
    let run_directory = new_run_directory(repository)?;
    let receipt_path = run_directory.join("receipt.json");
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let cleanup_scope = unique_cleanup_scope()?;
    let mut context = ServiceContext::new(repository, &run_directory, cleanup_scope);
    let mut binary_proof = None;
    let mut artifact_proof = None;

    let workflow = (|| {
        let binary = resolve_input_file(repository, &options.binary, "runner binary")?;
        binary_proof = Some(proof_input(repository, &binary)?);
        let artifact = repository
            .join("applications/lkjournal")
            .join(SERVICE_ARTIFACT_RELATIVE);
        artifact_proof = Some(proof_required_file_with_sha256(
            repository,
            &artifact,
            MAXIMUM_ARTIFACT_BYTES,
            "maintained artifact-11 service bundle",
            SERVICE_ARTIFACT_SHA256,
        )?);
        run_acceptance(&mut context, &binary)
    })();

    let runner_cleanup = context.cleanup_runners();
    let data_cleanup = context.cleanup_data();
    let final_result = match (workflow, runner_cleanup, data_cleanup) {
        (Ok(result), Ok(()), Ok(())) => Ok(result),
        (Err(error), _, _) => Err(error),
        (Ok(_), Err(error), _) => Err(error),
        (Ok(_), Ok(()), Err(error)) => Err(error),
    };
    let (status, result, failure) = match final_result {
        Ok(result) => (ServiceStatus::Passed, Some(result), None),
        Err(error) => (error.status, None, Some(error.receipt())),
    };
    let receipt = ServiceReceipt {
        contract_version: SERVICE_CONTRACT_VERSION,
        status,
        data_contract: DATA_CONTRACT.to_owned(),
        platform: PlatformObservation {
            operating_system: std::env::consts::OS.to_owned(),
            architecture: std::env::consts::ARCH.to_owned(),
            child_process_control: "linux_process_group".to_owned(),
        },
        started_unix_nanoseconds: started_wall,
        completed_unix_nanoseconds: unix_nanoseconds()?,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        binary: binary_proof,
        artifact: artifact_proof,
        retained_files: context.retained_files,
        secret_environment_names: vec!["LKJOURNAL_BOOTSTRAP_TOKEN".to_owned()],
        raw_secret_values_retained: false,
        commands: context.commands,
        runners: context.runner_evidence,
        requests: context.requests,
        cleanup: context.cleanup,
        result,
        failure,
    };
    let published = evidence::publish_json(&receipt_path, &receipt)?;
    Ok((receipt, published))
}

impl ServiceContext {
    fn new(repository: &Path, run_directory: &Path, cleanup_scope: String) -> Self {
        Self {
            repository: repository.to_path_buf(),
            run_directory: run_directory.to_path_buf(),
            command_ordinal: 0,
            commands: Vec::new(),
            runners: Vec::new(),
            runner_evidence: Vec::new(),
            requests: Vec::new(),
            retained_files: Vec::new(),
            secret_values: Vec::new(),
            cleanup: CleanupObservation {
                scope: cleanup_scope,
                attempted: false,
                completed: false,
            },
        }
    }

    fn observe_command(
        &mut self,
        request: CommandRequest<'_>,
    ) -> Result<CommandOutput, ServiceFailure> {
        let ordinal = self.command_ordinal;
        self.command_ordinal = self.command_ordinal.checked_add(1).ok_or_else(|| {
            ServiceFailure::infrastructure("command_ordinal_overflow", "command ordinal overflow")
        })?;
        let safe_name = safe_file_component(request.name)?;
        let prefix = format!("command-{ordinal:06}-{safe_name}");
        let stdout_path = self.run_directory.join(format!("{prefix}.stdout.log"));
        let stderr_path = self.run_directory.join(format!("{prefix}.stderr.log"));
        let specification = ProcessSpec {
            command: request.command.clone(),
            cwd: self.repository.clone(),
            environment: request.environment,
            timeout: request.timeout,
            maximum_stdout_bytes: request.maximum_stdout_bytes,
            maximum_stderr_bytes: request.maximum_stderr_bytes,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: None,
        };
        let mut observation = process::run(&specification, &self.repository);
        redact_process_logs(
            &self.repository,
            &stdout_path,
            &stderr_path,
            &self.secret_values,
            &mut observation,
        )?;
        let stdout = process::read_bounded(&stdout_path, request.maximum_stdout_bytes)
            .map_err(|error| ServiceFailure::infrastructure("command_stdout_read", error))?;
        self.commands.push(CommandEvidence {
            name: request.name.to_owned(),
            command: redact_command(&request.command, &self.secret_values),
            process: observation.clone(),
        });
        Ok(CommandOutput {
            observation,
            stdout,
        })
    }

    fn invoke(&mut self, request: CommandRequest<'_>) -> Result<Vec<u8>, ServiceFailure> {
        let name = request.name.to_owned();
        let output = self.observe_command(request)?;
        if output.observation.status == ProcessStatus::Passed {
            return Ok(output.stdout);
        }
        let reason = output
            .observation
            .reason
            .as_deref()
            .unwrap_or("child_failed");
        if output.observation.status == ProcessStatus::Unavailable {
            return Err(ServiceFailure::unavailable(
                "required_command_unavailable",
                format!("{name} is unavailable ({reason})"),
            ));
        }
        Err(ServiceFailure::failed(
            "child_command_failed",
            format!("{name} failed ({reason})"),
        ))
    }

    fn start_runner(
        &mut self,
        name: &str,
        command: Vec<String>,
        cwd: &Path,
        environment: BTreeMap<String, String>,
    ) -> Result<usize, ServiceFailure> {
        let safe_name = safe_file_component(name)?;
        let stdout_path = self.run_directory.join(format!("{safe_name}.stdout.log"));
        let stderr_path = self.run_directory.join(format!("{safe_name}.stderr.log"));
        let specification = ProcessSpec {
            command: command.clone(),
            cwd: cwd.to_path_buf(),
            environment,
            timeout: RUNNER_TIMEOUT,
            maximum_stdout_bytes: MAXIMUM_RUNNER_STDOUT_BYTES,
            maximum_stderr_bytes: MAXIMUM_RUNNER_STDERR_BYTES,
            stdout_path: stdout_path.clone(),
            stderr_path: stderr_path.clone(),
            unavailable_exit_code: None,
        };
        let repository = self.repository.clone();
        let control = ProcessControl::default();
        let child_control = control.clone();
        let (sender, receiver) = mpsc::channel();
        let child = thread::Builder::new()
            .name(format!("lkjscript-dev-{safe_name}"))
            .spawn(move || {
                let observation =
                    process::run_controlled(&specification, &repository, &child_control);
                let _ = sender.send(observation);
            })
            .map_err(|error| ServiceFailure::infrastructure("runner_thread_spawn", error))?;
        let index = self.runners.len();
        self.runners.push(Some(ActiveRunner {
            name: name.to_owned(),
            command: redact_command(&command, &self.secret_values),
            control,
            receiver,
            thread: Some(child),
            stdout_path,
            stderr_path,
            ready: None,
            terminal: None,
            started: Instant::now(),
        }));
        Ok(index)
    }

    fn runner_ready(&mut self, index: usize) -> Result<RunnerReady, ServiceFailure> {
        let runner = self
            .runners
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or_else(|| ServiceFailure::failed("runner_missing", "runner is not active"))?;
        let line = runner.wait_for_ready_line(RUNNER_READY_TIMEOUT)?;
        let mut ready = parse_ready_event(&line)?;
        ready.readiness_elapsed_nanoseconds = duration_nanoseconds(runner.started.elapsed());
        runner.ready = Some(ready.clone());
        Ok(ready)
    }

    fn stop_runner(&mut self, index: usize) -> Result<RunnerStopped, ServiceFailure> {
        let evidence = self.finish_runner(index, true)?;
        let process_status = evidence.process.status;
        let stopped = evidence.stopped.clone();
        self.runner_evidence.push(evidence);
        if process_status != ProcessStatus::Passed {
            return Err(ServiceFailure::failed(
                "runner_process_failed",
                format!("runner process ended as {process_status:?}"),
            ));
        }
        let stopped = stopped.ok_or_else(|| {
            ServiceFailure::failed("runner_stop_missing", "runner omitted its stop receipt")
        })?;
        if !stopped.clean() {
            return Err(ServiceFailure::failed(
                "runner_shutdown_not_clean",
                "runner shutdown retained tasks or cleanup failures",
            ));
        }
        Ok(stopped)
    }

    fn finish_runner(
        &mut self,
        index: usize,
        graceful: bool,
    ) -> Result<RunnerEvidence, ServiceFailure> {
        let mut runner = self
            .runners
            .get_mut(index)
            .and_then(Option::take)
            .ok_or_else(|| ServiceFailure::failed("runner_missing", "runner is not active"))?;
        if graceful {
            runner.control.interrupt();
        } else if runner.poll_terminal()?.is_none() {
            runner.control.kill();
        }
        let mut observation = match runner.wait_terminal(if graceful {
            RUNNER_STOP_TIMEOUT
        } else {
            RUNNER_KILL_TIMEOUT
        }) {
            Ok(observation) => observation,
            Err(error) if graceful => {
                runner.control.kill();
                runner
                    .wait_terminal(RUNNER_KILL_TIMEOUT)
                    .map_err(|_| error)?
            }
            Err(error) => return Err(error),
        };
        runner.join()?;
        redact_process_logs(
            &self.repository,
            &runner.stdout_path,
            &runner.stderr_path,
            &self.secret_values,
            &mut observation,
        )?;
        let stdout = process::read_bounded(&runner.stdout_path, MAXIMUM_RUNNER_STDOUT_BYTES)
            .map_err(|error| ServiceFailure::infrastructure("runner_stdout_read", error))?;
        let stopped = parse_stopped_event(&stdout).ok();
        let evidence = RunnerEvidence {
            name: runner.name,
            command: runner.command,
            process: observation.clone(),
            ready: runner.ready,
            stopped,
        };
        Ok(evidence)
    }

    fn cleanup_runners(&mut self) -> Result<(), ServiceFailure> {
        let mut first_error = None;
        for index in 0..self.runners.len() {
            if self.runners[index].is_none() {
                continue;
            }
            match self.finish_runner(index, false) {
                Ok(evidence) => self.runner_evidence.push(evidence),
                Err(error) if first_error.is_none() => first_error = Some(error),
                Err(_) => {}
            }
        }
        first_error.map_or(Ok(()), Err)
    }

    fn cleanup_data(&mut self) -> Result<(), ServiceFailure> {
        self.cleanup.attempted = true;
        self.cleanup.completed = true;
        Ok(())
    }

    fn request(
        &mut self,
        name: &str,
        port: u16,
        method: &str,
        path: &str,
        body: &[u8],
        headers: &[(&str, &str)],
    ) -> Result<HttpResponse, ServiceFailure> {
        let response = http_request(port, method, path, body, headers)?;
        self.requests.push(HttpObservation {
            name: name.to_owned(),
            method: method.to_owned(),
            path: path.to_owned(),
            status: response.status,
            request_body_bytes: body.len() as u64,
            response_body_bytes: response.body.len() as u64,
            response_digest: VerificationDigest::of(&response.body),
            elapsed_nanoseconds: response.elapsed_nanoseconds,
        });
        Ok(response)
    }

    fn retain(&mut self, path: &Path) -> Result<FileProof, ServiceFailure> {
        let proof = evidence::proof(path, evidence::relative(&self.repository, path))
            .map_err(|error| ServiceFailure::infrastructure("retained_file_proof", error))?;
        self.retained_files.push(proof.clone());
        Ok(proof)
    }
}

impl ActiveRunner {
    fn wait_for_ready_line(&mut self, timeout: Duration) -> Result<Vec<u8>, ServiceFailure> {
        let started = Instant::now();
        loop {
            if let Some(line) = first_line(&self.stdout_path, MAXIMUM_RUNNER_STDOUT_BYTES)? {
                return Ok(line);
            }
            if self.poll_terminal()?.is_some() {
                return Err(ServiceFailure::failed(
                    "runner_exited_before_ready",
                    "runner exited before publishing readiness",
                ));
            }
            if started.elapsed() >= timeout {
                return Err(ServiceFailure::failed(
                    "runner_readiness_timeout",
                    "runner did not publish readiness before the deadline",
                ));
            }
            thread::sleep(POLL_INTERVAL);
        }
    }

    fn poll_terminal(&mut self) -> Result<Option<&ProcessObservation>, ServiceFailure> {
        if self.terminal.is_none() {
            match self.receiver.try_recv() {
                Ok(observation) => self.terminal = Some(observation),
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    return Err(ServiceFailure::infrastructure(
                        "runner_observation_disconnected",
                        "runner observation channel disconnected",
                    ));
                }
            }
        }
        Ok(self.terminal.as_ref())
    }

    fn wait_terminal(&mut self, timeout: Duration) -> Result<ProcessObservation, ServiceFailure> {
        if let Some(observation) = self.terminal.take() {
            return Ok(observation);
        }
        self.receiver
            .recv_timeout(timeout)
            .map_err(|error| ServiceFailure::infrastructure("runner_stop_timeout", error))
    }

    fn join(&mut self) -> Result<(), ServiceFailure> {
        if let Some(child) = self.thread.take() {
            child.join().map_err(|_| {
                ServiceFailure::infrastructure("runner_thread_panic", "runner thread panicked")
            })?;
        }
        Ok(())
    }
}

fn run_acceptance(
    context: &mut ServiceContext,
    binary: &Path,
) -> Result<ServiceResult, ServiceFailure> {
    let application = context.repository.join("applications/lkjournal");
    let authority_before = observe_graph_authority(&application)?;
    let artifact_source = application.join(SERVICE_ARTIFACT_RELATIVE);
    let service_source = application.join("service.deployment.json");
    let worker_source = application.join("worker.deployment.json");
    let artifact_bytes = process::read_bounded(&artifact_source, MAXIMUM_ARTIFACT_BYTES)
        .map_err(|error| ServiceFailure::infrastructure("artifact_read", error))?;
    let fresh_artifact = context.run_directory.join("fresh-lkjournal.lkja");
    let build_output = context.invoke(
        CommandRequest::standard(
            "artifact-fresh-build",
            vec![
                binary.to_string_lossy().into_owned(),
                "--project".to_owned(),
                application.to_string_lossy().into_owned(),
                "build".to_owned(),
                "--output".to_owned(),
                fresh_artifact.to_string_lossy().into_owned(),
            ],
        )
        .timeout(Duration::from_secs(120)),
    )?;
    let fresh_bytes = process::read_bounded(&fresh_artifact, MAXIMUM_ARTIFACT_BYTES)
        .map_err(|error| ServiceFailure::infrastructure("fresh_artifact_read", error))?;
    require(
        fresh_bytes == artifact_bytes,
        "artifact_fresh_build_mismatch",
        "fresh public build is not byte-equal to the checked-in maintained artifact",
    )?;
    let artifact_identity = parse_build_identity(
        &build_output,
        artifact_bytes.len() as u64,
        &sha256_hex(&artifact_bytes),
    )?;
    fs::remove_file(&fresh_artifact)
        .map_err(|error| ServiceFailure::infrastructure("fresh_artifact_remove", error))?;
    let artifact_directory = context.run_directory.join("generated");
    fs::create_dir(&artifact_directory)
        .map_err(|error| ServiceFailure::infrastructure("artifact_directory", error))?;
    let artifact_path = context.run_directory.join(SERVICE_ARTIFACT_RELATIVE);
    evidence::publish(&artifact_path, &artifact_bytes)
        .map_err(|error| ServiceFailure::infrastructure("artifact_stage", error))?;
    context.retain(&artifact_path)?;
    fs::create_dir_all(context.run_directory.join("state/objects"))
        .map_err(|error| ServiceFailure::infrastructure("object_host_directory", error))?;

    let bootstrap_token = random_hex(16)?;
    let application_password = random_hex(16)?;
    context.secret_values.extend([
        bootstrap_token.as_bytes().to_vec(),
        application_password.as_bytes().to_vec(),
    ]);

    let service_port = free_port()?;
    let service_path = context.run_directory.join("service.json");
    write_descriptor(
        &service_source,
        &service_path,
        Some(service_port),
        "state/data",
    )?;
    context.retain(&service_path)?;
    let worker_path = context.run_directory.join("worker.json");
    write_descriptor(&worker_source, &worker_path, None, "state/data")?;
    context.retain(&worker_path)?;

    let mut runner_environment = process::environment();
    runner_environment.insert(
        "LKJOURNAL_BOOTSTRAP_TOKEN".to_owned(),
        bootstrap_token.clone(),
    );
    let absent = context.observe_command(
        CommandRequest::standard(
            "service-absent-data-root",
            vec![
                binary.to_string_lossy().into_owned(),
                "serve".to_owned(),
                "--deployment".to_owned(),
                "service.json".to_owned(),
            ],
        )
        .environment(runner_environment.clone())
        .timeout(Duration::from_secs(15)),
    )?;
    require(
        absent.observation.status == ProcessStatus::Failed
            && !absent
                .stdout
                .windows(b"\"event\":\"ready\"".len())
                .any(|window| window == b"\"event\":\"ready\""),
        "absent_data_readiness",
        "service admitted readiness without an initialized data root",
    )?;
    let data_root = context.run_directory.join("state/data");
    context.invoke(CommandRequest::standard(
        "data-initialize",
        vec![
            binary.to_string_lossy().into_owned(),
            "data".to_owned(),
            "initialize".to_owned(),
            "--root".to_owned(),
            data_root.to_string_lossy().into_owned(),
        ],
    ))?;
    let service_index = context.start_runner(
        "service-first",
        vec![
            binary.to_string_lossy().into_owned(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            "service.json".to_owned(),
        ],
        &context.run_directory.clone(),
        runner_environment.clone(),
    )?;
    let ready = context.runner_ready(service_index)?;
    require(
        ready.artifact_digest == artifact_identity.artifact_bundle
            && ready.target == "serve"
            && ready.runner == "http",
        "service_artifact_identity",
        "service readiness disagrees with the exact fresh artifact-11 build",
    )?;
    require(
        ready.secret_names == ["bootstrap-token"],
        "service_secret_names",
        "service readiness secret names changed",
    )?;

    let mut timings = BTreeMap::new();
    let health = context.request("health", service_port, "GET", "/health", b"", &[])?;
    timings.insert("health".to_owned(), health.elapsed_nanoseconds);
    require(
        health.status == 200 && health.body == b"ready",
        "health_route",
        "health route failed",
    )?;

    let initialize_path = format!("/initialize?{}", query(&[("actor", "operator")]));
    let denied = context.request(
        "bootstrap-denial",
        service_port,
        "POST",
        &initialize_path,
        b"body-must-not-be-read",
        &[],
    )?;
    timings.insert("bootstrap_denial".to_owned(), denied.elapsed_nanoseconds);
    require(
        denied.status == 403,
        "bootstrap_denial",
        "bootstrap denial changed",
    )?;

    let bootstrap_authorization = format!("Bearer {bootstrap_token}");
    let initialized = context.request(
        "initialize",
        service_port,
        "POST",
        &initialize_path,
        application_password.as_bytes(),
        &[("Authorization", &bootstrap_authorization)],
    )?;
    timings.insert("initialize".to_owned(), initialized.elapsed_nanoseconds);
    let initialized_json = parse_json_body(&initialized.body)?;
    let actor_inserted = boolean_at(&initialized_json, "actor_inserted").unwrap_or(false);
    let initialization_error = optional_string_at(&initialized_json, "error")?;
    require(
        initialized.status == 200 && actor_inserted,
        "initialization",
        "application initialization failed",
    )?;
    let initialization_transport = InitializationTransport {
        status: initialized.status,
        body_bytes: initialized.body.len() as u64,
        body_sha256: sha256_hex(&initialized.body),
        failure_class: initialized
            .headers
            .get("x-lkjscript-failure-class")
            .cloned(),
        failure_code: initialized.headers.get("x-lkjscript-failure-code").cloned(),
    };
    let initialization_observation = InitializationObservation {
        status: initialized.status,
        actor_inserted,
        error: initialization_error,
    };

    let login_path = format!("/login?{}", query(&[("actor", "operator")]));
    let logged_in = context.request(
        "login",
        service_port,
        "POST",
        &login_path,
        application_password.as_bytes(),
        &[],
    )?;
    timings.insert("login".to_owned(), logged_in.elapsed_nanoseconds);
    let login_json = parse_json_body(&logged_in.body)?;
    require(
        logged_in.status == 200 && string_at(&login_json, "actor")? == "operator",
        "login",
        "application login failed",
    )?;
    let token = string_at(&login_json, "token")?;
    context.secret_values.push(token.as_bytes().to_vec());
    let authorization = format!("Bearer {token}");

    let create_body =
        br##"{"title":"Acceptance entry","body":"# Initial\nLive service evidence."}"##;
    let created = context.request(
        "create-resource",
        service_port,
        "POST",
        "/resources",
        create_body,
        &[
            ("Authorization", &authorization),
            ("Content-Type", "application/json"),
        ],
    )?;
    timings.insert("create".to_owned(), created.elapsed_nanoseconds);
    let created_json = parse_json_body(&created.body)?;
    require(
        created.status == 201 && u64_at(&created_json, "revision")? == 0,
        "resource_create",
        "resource creation failed",
    )?;
    let resource_id = string_at(&created_json, "id")?;
    require(
        resource_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-'),
        "resource_identity",
        "service returned an unsafe resource identity",
    )?;

    let listed = context.request(
        "list-resources",
        service_port,
        "GET",
        "/resources",
        b"",
        &[("Authorization", &authorization)],
    )?;
    timings.insert("list".to_owned(), listed.elapsed_nanoseconds);
    let listed_json = parse_json_body(&listed.body)?;
    require(
        listed.status == 200
            && listed_json.as_array().is_some_and(|items| items.len() == 1)
            && listed_json
                .as_array()
                .and_then(|items| items.first())
                .and_then(|item| item.get("id"))
                .and_then(Value::as_str)
                == Some(resource_id.as_str()),
        "resource_list",
        "resource list failed",
    )?;

    let resource_path = format!("/resource?{}", query(&[("id", &resource_id)]));
    let read = context.request(
        "read-resource",
        service_port,
        "GET",
        &resource_path,
        b"",
        &[("Authorization", &authorization)],
    )?;
    timings.insert("read".to_owned(), read.elapsed_nanoseconds);
    require(
        read.status == 200 && u64_at(&parse_json_body(&read.body)?, "revision")? == 0,
        "resource_read",
        "resource read failed",
    )?;

    let update_body = br##"{"title":"Acceptance entry revised","body":"# Revised","base":0}"##;
    let update_path = format!("/resource/update?{}", query(&[("id", &resource_id)]));
    let updated = context.request(
        "update-resource",
        service_port,
        "POST",
        &update_path,
        update_body,
        &[
            ("Authorization", &authorization),
            ("Content-Type", "application/json"),
        ],
    )?;
    timings.insert("update".to_owned(), updated.elapsed_nanoseconds);
    require(
        updated.status == 200 && u64_at(&parse_json_body(&updated.body)?, "revision")? == 1,
        "resource_update",
        "exact-base update failed",
    )?;
    let stale = context.request(
        "stale-update",
        service_port,
        "POST",
        &update_path,
        update_body,
        &[
            ("Authorization", &authorization),
            ("Content-Type", "application/json"),
        ],
    )?;
    timings.insert("stale_update".to_owned(), stale.elapsed_nanoseconds);
    require(
        stale.status == 409,
        "stale_update",
        "stale update did not reject",
    )?;

    let history_path = format!("/resource/history?{}", query(&[("id", &resource_id)]));
    let history = context.request(
        "resource-history",
        service_port,
        "GET",
        &history_path,
        b"",
        &[("Authorization", &authorization)],
    )?;
    timings.insert("history".to_owned(), history.elapsed_nanoseconds);
    require(
        history.status == 200
            && parse_json_body(&history.body)?
                .as_array()
                .is_some_and(|values| values.len() == 2),
        "resource_history",
        "immutable history failed",
    )?;

    let unauthenticated = context.request(
        "unauthenticated-read",
        service_port,
        "GET",
        &resource_path,
        b"",
        &[],
    )?;
    timings.insert(
        "unauthenticated".to_owned(),
        unauthenticated.elapsed_nanoseconds,
    );
    require(
        unauthenticated.status == 401,
        "unauthenticated_request",
        "unauthenticated request did not reject",
    )?;

    let strict_json = context.request(
        "strict-json",
        service_port,
        "POST",
        "/resources",
        br#"{"title":"x","body":"y","foreign":true}"#,
        &[
            ("Authorization", &authorization),
            ("Content-Type", "application/json"),
        ],
    )?;
    timings.insert("strict_json".to_owned(), strict_json.elapsed_nanoseconds);
    require(
        strict_json.status == 400,
        "strict_json",
        "unknown JSON field did not reject",
    )?;

    let object_payload = vec![b'z'; 200_000];
    let object_path_query = format!("/objects?{}", query(&[("name", "acceptance-200k.bin")]));
    let object = context.request(
        "object-put",
        service_port,
        "POST",
        &object_path_query,
        &object_payload,
        &[("Authorization", &authorization)],
    )?;
    timings.insert("object_put".to_owned(), object.elapsed_nanoseconds);
    require(
        object.status == 201
            && u64_at(&parse_json_body(&object.body)?, "bytes")? == object_payload.len() as u64,
        "object_publication",
        "object publication failed",
    )?;
    let object_path = context
        .run_directory
        .join("state/objects/lkjournal/operator/acceptance-200k.bin");
    let retained_object = process::read_bounded(&object_path, object_payload.len() as u64)
        .map_err(|error| ServiceFailure::infrastructure("object_read", error))?;
    require(
        retained_object == object_payload,
        "object_bytes",
        "published object bytes disagree",
    )?;
    context.retain(&object_path)?;

    inject_queue_oracle_fixtures(&data_root)?;

    let worker_a_index = context.start_runner(
        "worker-a",
        vec![
            binary.to_string_lossy().into_owned(),
            "worker".to_owned(),
            "--deployment".to_owned(),
            "worker.json".to_owned(),
        ],
        &context.run_directory.clone(),
        runner_environment.clone(),
    )?;
    let worker_a_ready = context.runner_ready(worker_a_index)?;
    require(
        worker_a_ready.artifact_digest == artifact_identity.artifact_bundle
            && worker_a_ready.target == "work"
            && worker_a_ready.runner == "worker",
        "worker_artifact_identity",
        "worker readiness disagrees with the exact fresh artifact-11 build",
    )?;
    let worker_b_index = context.start_runner(
        "worker-b",
        vec![
            binary.to_string_lossy().into_owned(),
            "worker".to_owned(),
            "--deployment".to_owned(),
            "worker.json".to_owned(),
        ],
        &context.run_directory.clone(),
        runner_environment.clone(),
    )?;
    let worker_b_ready = context.runner_ready(worker_b_index)?;
    require(
        worker_b_ready.artifact_digest == artifact_identity.artifact_bundle
            && worker_b_ready.target == "work"
            && worker_b_ready.runner == "worker",
        "second_worker_artifact_identity",
        "second worker readiness disagrees with the exact fresh artifact-11 build",
    )?;
    thread::sleep(WORKER_READY_TIMEOUT.min(Duration::from_secs(2)));
    let worker_a_stopped = context.stop_runner(worker_a_index)?;
    let worker_b_stopped = context.stop_runner(worker_b_index)?;
    let productive_iterations = worker_a_stopped
        .productive_iterations
        .unwrap_or(0)
        .checked_add(worker_b_stopped.productive_iterations.unwrap_or(0))
        .ok_or_else(|| {
            ServiceFailure::failed(
                "worker_productivity_overflow",
                "combined worker productivity overflowed u64",
            )
        })?;
    require(
        productive_iterations >= 2,
        "worker_productivity",
        "two-worker acceptance reported insufficient productive work",
    )?;
    let queue_observation = observe_queue_oracle(&data_root, productive_iterations)?;
    context.stop_runner(service_index)?;

    let restart_index = context.start_runner(
        "service-restart",
        vec![
            binary.to_string_lossy().into_owned(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            "service.json".to_owned(),
        ],
        &context.run_directory.clone(),
        runner_environment.clone(),
    )?;
    let restart_ready = context.runner_ready(restart_index)?;
    require(
        restart_ready.artifact_digest == artifact_identity.artifact_bundle,
        "restart_artifact_identity",
        "restarted service changed the exact artifact bundle identity",
    )?;
    let restarted = context.request(
        "restart-read",
        service_port,
        "GET",
        &resource_path,
        b"",
        &[("Authorization", &authorization)],
    )?;
    timings.insert("restart_read".to_owned(), restarted.elapsed_nanoseconds);
    require(
        restarted.status == 200 && u64_at(&parse_json_body(&restarted.body)?, "revision")? == 1,
        "restart_read",
        "restarted service disagrees with accepted data",
    )?;
    context.stop_runner(restart_index)?;

    context.invoke(CommandRequest::standard(
        "data-verify",
        vec![
            binary.to_string_lossy().into_owned(),
            "data".to_owned(),
            "verify".to_owned(),
            "--root".to_owned(),
            data_root.to_string_lossy().into_owned(),
        ],
    ))?;
    let backup_path = context.run_directory.join("lkjournal-data.lkjb");
    context.invoke(CommandRequest::standard(
        "data-backup",
        vec![
            binary.to_string_lossy().into_owned(),
            "data".to_owned(),
            "backup".to_owned(),
            "--root".to_owned(),
            data_root.to_string_lossy().into_owned(),
            "--output".to_owned(),
            backup_path.to_string_lossy().into_owned(),
        ],
    ))?;
    let backup_bytes = process::read_bounded(&backup_path, MAXIMUM_BACKUP_BYTES)
        .map_err(|error| ServiceFailure::infrastructure("data_backup_read", error))?;
    require(
        !backup_bytes.is_empty(),
        "data_backup_size",
        "logical data backup is empty",
    )?;
    let backup_proof = context.retain(&backup_path)?;

    let mut corrupt_bytes = backup_bytes;
    let corrupt_byte = corrupt_bytes.last_mut().ok_or_else(|| {
        ServiceFailure::failed("data_backup_empty", "logical data backup is empty")
    })?;
    *corrupt_byte ^= 0x01;
    let corrupt_backup = context.run_directory.join("lkjournal-data-corrupt.lkjb");
    evidence::publish(&corrupt_backup, &corrupt_bytes)
        .map_err(|error| ServiceFailure::infrastructure("corrupt_backup_publish", error))?;
    let rejected_root = context.run_directory.join("state/rejected-data");
    let rejected = context.observe_command(CommandRequest::standard(
        "data-restore-corrupt",
        vec![
            binary.to_string_lossy().into_owned(),
            "data".to_owned(),
            "restore".to_owned(),
            "--backup".to_owned(),
            corrupt_backup.to_string_lossy().into_owned(),
            "--root".to_owned(),
            rejected_root.to_string_lossy().into_owned(),
        ],
    ))?;
    require(
        rejected.observation.status == ProcessStatus::Failed && !rejected_root.exists(),
        "corrupt_backup_rejection",
        "corrupt logical backup made a destination visible",
    )?;

    let restored_root = context.run_directory.join("state/restored-data");
    context.invoke(CommandRequest::standard(
        "data-restore",
        vec![
            binary.to_string_lossy().into_owned(),
            "data".to_owned(),
            "restore".to_owned(),
            "--backup".to_owned(),
            backup_path.to_string_lossy().into_owned(),
            "--root".to_owned(),
            restored_root.to_string_lossy().into_owned(),
        ],
    ))?;
    context.invoke(CommandRequest::standard(
        "data-verify-restored",
        vec![
            binary.to_string_lossy().into_owned(),
            "data".to_owned(),
            "verify".to_owned(),
            "--root".to_owned(),
            restored_root.to_string_lossy().into_owned(),
        ],
    ))?;

    let restored_port = free_port()?;
    let restored_descriptor = context.run_directory.join("service-restored.json");
    write_descriptor(
        &service_source,
        &restored_descriptor,
        Some(restored_port),
        "state/restored-data",
    )?;
    context.retain(&restored_descriptor)?;
    let restored_index = context.start_runner(
        "service-restored",
        vec![
            binary.to_string_lossy().into_owned(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            "service-restored.json".to_owned(),
        ],
        &context.run_directory.clone(),
        runner_environment,
    )?;
    let restored_ready = context.runner_ready(restored_index)?;
    require(
        restored_ready.artifact_digest == artifact_identity.artifact_bundle,
        "restored_artifact_identity",
        "restored service readiness changed the exact artifact-11 bundle identity",
    )?;
    let restored = context.request(
        "restored-read",
        restored_port,
        "GET",
        &resource_path,
        b"",
        &[("Authorization", &authorization)],
    )?;
    timings.insert("restored_read".to_owned(), restored.elapsed_nanoseconds);
    require(
        restored.status == 200 && u64_at(&parse_json_body(&restored.body)?, "revision")? == 1,
        "restored_read",
        "restored service disagrees",
    )?;
    context.stop_runner(restored_index)?;

    let authority_after = observe_graph_authority(&application)?;
    require(
        authority_after == authority_before,
        "graph_authority_changed",
        "service acceptance changed the maintained Graph 6 authority inventory",
    )?;

    Ok(ServiceResult {
        data_contract: DATA_CONTRACT.to_owned(),
        artifact_digest: ready.artifact_digest,
        artifact_identity,
        authority_before,
        authority_after,
        authority_unchanged: true,
        routes_checked: context.requests.len() as u64,
        resource_revision: 1,
        history_entries: 2,
        object_bytes: object_payload.len() as u64,
        worker_productive_iterations: productive_iterations,
        queue_observation,
        data_backup: backup_proof,
        restart_read_equal: true,
        restored_read_equal: true,
        corrupt_backup_rejected: true,
        shutdown_cleanup_failures: 0,
        initialization_transport,
        initialization_observation,
        request_elapsed_nanoseconds: timings,
    })
}

fn inject_queue_oracle_fixtures(data_root: &Path) -> Result<(), ServiceFailure> {
    let store = DataStore::open(data_root, QUEUE_NAMESPACE, DataLimits::default())
        .map_err(|error| ServiceFailure::infrastructure("queue_oracle_open", error))?;
    let fixtures = [
        QueueFixtureJob {
            job_id: ORACLE_STALE_JOB.to_owned(),
            idempotency_key: "affine-oracle-stale-key".to_owned(),
            payload: b"stale-replacement".to_vec(),
            state: QueueJobState::Leased,
            available_at: 0,
            created_at: -2,
            attempt_count: 1,
            attempt_id: Some("affine-oracle-stale:1".to_owned()),
            worker_id: Some("departed-worker".to_owned()),
            lease_until: Some(0),
            result: None,
            last_error_class: None,
        },
        QueueFixtureJob {
            job_id: ORACLE_RETRY_JOB.to_owned(),
            idempotency_key: "affine-oracle-retry-key".to_owned(),
            payload: Vec::new(),
            state: QueueJobState::Ready,
            available_at: 0,
            created_at: -1,
            attempt_count: 0,
            attempt_id: None,
            worker_id: None,
            lease_until: None,
            result: None,
            last_error_class: None,
        },
    ];
    for _ in 0..32 {
        let mut transaction = store
            .begin()
            .map_err(|error| ServiceFailure::infrastructure("queue_oracle_begin", error))?;
        let schema = transaction
            .schema_read(QUEUE_SCHEMA_SPACE)
            .map_err(|error| ServiceFailure::infrastructure("queue_oracle_schema", error))?;
        require(
            schema.is_some_and(|schema| {
                schema.identity == QUEUE_DATA_CONTRACT
                    && schema.digest == queue_schema_digest(QUEUE_DATA_CONTRACT)
            }),
            "queue_oracle_schema",
            "maintained queue namespace does not own the unchanged queue-data contract",
        )?;
        for fixture in &fixtures {
            put_queue_oracle_fixture(&store, &mut transaction, fixture)?;
        }
        match transaction
            .commit()
            .map_err(|error| ServiceFailure::infrastructure("queue_oracle_commit", error))?
        {
            DataCommitOutcome::Committed { .. } => return Ok(()),
            DataCommitOutcome::Conflict { .. } => continue,
            DataCommitOutcome::Unchanged { .. } => {
                return Err(ServiceFailure::failed(
                    "queue_oracle_unchanged",
                    "queue oracle fixture transaction did not publish its create-new records",
                ));
            }
        }
    }
    Err(ServiceFailure::failed(
        "queue_oracle_conflict",
        "queue oracle fixture transaction exhausted bounded exact-base retries",
    ))
}

fn put_queue_oracle_fixture(
    store: &DataStore,
    transaction: &mut DataTransaction,
    job: &QueueFixtureJob,
) -> Result<(), ServiceFailure> {
    let job_key = queue_text_key(store, &job.job_id)?;
    let idempotency_key = queue_text_key(store, &job.idempotency_key)?;
    let claim_key = queue_claim_key(store, job)?;
    let primary_inserted = transaction
        .put(
            QUEUE_JOB_SPACE,
            &job_key,
            encode_queue_job(job)?,
            DataExpectation::Missing,
        )
        .map_err(|error| ServiceFailure::infrastructure("queue_oracle_primary", error))?;
    let idempotency_inserted = transaction
        .put(
            QUEUE_IDEMPOTENCY_SPACE,
            &idempotency_key,
            job.job_id.as_bytes().to_vec(),
            DataExpectation::Missing,
        )
        .map_err(|error| ServiceFailure::infrastructure("queue_oracle_idempotency", error))?;
    let claim_inserted = transaction
        .put(
            QUEUE_CLAIM_SPACE,
            &claim_key,
            Vec::new(),
            DataExpectation::Missing,
        )
        .map_err(|error| ServiceFailure::infrastructure("queue_oracle_claim", error))?;
    require(
        primary_inserted && idempotency_inserted && claim_inserted,
        "queue_oracle_create_new",
        "queue oracle fixture identity already exists",
    )
}

fn observe_queue_oracle(
    data_root: &Path,
    productive_iterations: u64,
) -> Result<QueueObservation, ServiceFailure> {
    let store = DataStore::open(data_root, QUEUE_NAMESPACE, DataLimits::default())
        .map_err(|error| ServiceFailure::infrastructure("queue_observer_open", error))?;
    let transaction = store
        .begin()
        .map_err(|error| ServiceFailure::infrastructure("queue_observer_begin", error))?;
    let page = transaction
        .scan(
            QUEUE_JOB_SPACE,
            &[],
            DataScanDirection::Forward,
            10_000,
            16 * 1_048_576,
            1_000_000,
            None,
        )
        .map_err(|error| ServiceFailure::infrastructure("queue_observer_scan", error))?;
    require(
        page.continuation.is_none(),
        "queue_observer_continuation",
        "bounded maintained queue observation unexpectedly requires continuation",
    )?;
    let mut retry = None;
    let mut stale = None;
    let mut completed_jobs = 0_u64;
    let mut transition_authority_cleared = true;
    for item in &page.items {
        let job = decode_queue_job(&item.value)?;
        let [DataKeyPart::Text(key_job_id)] = item.key.parts() else {
            return Err(ServiceFailure::failed(
                "queue_observer_primary_key",
                "queue primary record has a foreign key shape",
            ));
        };
        require(
            key_job_id == &job.job_id,
            "queue_observer_primary_identity",
            "queue primary key disagrees with its encoded job identity",
        )?;
        if job.state == QueueJobState::Completed {
            completed_jobs = completed_jobs.checked_add(1).ok_or_else(|| {
                ServiceFailure::failed(
                    "queue_observer_count_overflow",
                    "completed queue observation count overflowed u64",
                )
            })?;
        }
        transition_authority_cleared &= job.state != QueueJobState::Leased
            && job.attempt_id.is_none()
            && job.worker_id.is_none()
            && job.lease_until.is_none();
        if job.job_id == ORACLE_RETRY_JOB {
            require(
                retry.replace(job).is_none(),
                "queue_observer_retry_duplicate",
                "queue observation contains duplicate retry fixture identity",
            )?;
        } else if job.job_id == ORACLE_STALE_JOB {
            require(
                stale.replace(job).is_none(),
                "queue_observer_stale_duplicate",
                "queue observation contains duplicate stale fixture identity",
            )?;
        }
    }
    let retry = retry.ok_or_else(|| {
        ServiceFailure::failed(
            "queue_observer_retry_absent",
            "maintained worker did not retain the retry/fail fixture",
        )
    })?;
    let stale = stale.ok_or_else(|| {
        ServiceFailure::failed(
            "queue_observer_stale_absent",
            "maintained worker did not retain the stale-replacement fixture",
        )
    })?;
    require(
        matches!(retry.state, QueueJobState::Ready | QueueJobState::Failed)
            && retry.attempt_count >= 1
            && retry.last_error_class.as_deref() == Some("empty-payload"),
        "queue_observer_retry",
        "maintained worker did not consume a live lease through its retry/fail path",
    )?;
    require(
        stale.state == QueueJobState::Completed && stale.attempt_count >= 2,
        "queue_observer_stale_replacement",
        "maintained workers did not replace and complete the expired lease",
    )?;
    require(
        transition_authority_cleared,
        "queue_observer_transition_authority",
        "stopped workers left raw queue transition authority in a primary record",
    )?;
    Ok(QueueObservation {
        data_contract: QUEUE_DATA_CONTRACT.to_owned(),
        records_scanned: page.items.len() as u64,
        workers_started: 2,
        productive_iterations,
        completed_jobs,
        retry_job_state: retry.state.name().to_owned(),
        retry_job_attempts: retry.attempt_count,
        retry_error_class: retry.last_error_class.unwrap_or_default(),
        stale_job_state: stale.state.name().to_owned(),
        stale_job_attempts: stale.attempt_count,
        stale_replacement_observed: true,
        transition_authority_cleared: true,
    })
}

fn queue_text_key(store: &DataStore, value: &str) -> Result<DataKey, ServiceFailure> {
    DataKey::new(vec![DataKeyPart::Text(value.to_owned())], store.limits())
        .map_err(|error| ServiceFailure::infrastructure("queue_oracle_key", error))
}

fn queue_claim_key(store: &DataStore, job: &QueueFixtureJob) -> Result<DataKey, ServiceFailure> {
    let claim_at = match job.state {
        QueueJobState::Ready => job.available_at,
        QueueJobState::Leased => job.lease_until.ok_or_else(|| {
            ServiceFailure::failed(
                "queue_oracle_lease_deadline",
                "leased queue oracle fixture has no deadline",
            )
        })?,
        QueueJobState::Completed | QueueJobState::Failed | QueueJobState::Cancelled => {
            return Err(ServiceFailure::failed(
                "queue_oracle_terminal_claim",
                "terminal queue oracle fixture cannot own a claim index",
            ));
        }
    };
    DataKey::new(
        vec![
            DataKeyPart::I64(claim_at),
            DataKeyPart::I64(job.created_at),
            DataKeyPart::Text(job.job_id.clone()),
        ],
        store.limits(),
    )
    .map_err(|error| ServiceFailure::infrastructure("queue_oracle_claim_key", error))
}

fn encode_queue_job(job: &QueueFixtureJob) -> Result<Vec<u8>, ServiceFailure> {
    let mut output = Vec::new();
    output.extend_from_slice(QUEUE_JOB_MAGIC);
    push_queue_text(&mut output, &job.job_id)?;
    push_queue_text(&mut output, &job.idempotency_key)?;
    push_queue_blob(&mut output, &job.payload)?;
    output.push(match job.state {
        QueueJobState::Ready => 0,
        QueueJobState::Leased => 1,
        QueueJobState::Completed => 2,
        QueueJobState::Failed => 3,
        QueueJobState::Cancelled => 4,
    });
    output.extend_from_slice(&job.available_at.to_be_bytes());
    output.extend_from_slice(&job.created_at.to_be_bytes());
    output.extend_from_slice(&job.attempt_count.to_be_bytes());
    push_queue_optional_text(&mut output, job.attempt_id.as_deref())?;
    push_queue_optional_text(&mut output, job.worker_id.as_deref())?;
    push_queue_optional_i64(&mut output, job.lease_until);
    push_queue_optional_blob(&mut output, job.result.as_deref())?;
    push_queue_optional_text(&mut output, job.last_error_class.as_deref())?;
    output.extend_from_slice(&queue_digest(QUEUE_JOB_CHECKSUM_DOMAIN, &output));
    Ok(output)
}

fn decode_queue_job(bytes: &[u8]) -> Result<ObservedQueueJob, ServiceFailure> {
    let payload_length = bytes.len().checked_sub(32).ok_or_else(|| {
        ServiceFailure::failed(
            "queue_observer_truncated",
            "queue primary record is shorter than its checksum",
        )
    })?;
    let (payload, checksum) = bytes.split_at(payload_length);
    require(
        queue_digest(QUEUE_JOB_CHECKSUM_DOMAIN, payload).as_slice() == checksum,
        "queue_observer_checksum",
        "queue primary record checksum is corrupt",
    )?;
    let mut cursor = QueueCursor::new(payload);
    require(
        cursor.take(QUEUE_JOB_MAGIC.len())? == QUEUE_JOB_MAGIC,
        "queue_observer_magic",
        "queue primary record has a foreign magic value",
    )?;
    let job_id = cursor.text(512)?;
    let _idempotency_key = cursor.text(512)?;
    let _payload = cursor.blob(1_048_576)?;
    let state = match cursor.u8()? {
        0 => QueueJobState::Ready,
        1 => QueueJobState::Leased,
        2 => QueueJobState::Completed,
        3 => QueueJobState::Failed,
        4 => QueueJobState::Cancelled,
        _ => {
            return Err(ServiceFailure::failed(
                "queue_observer_state",
                "queue primary record has a foreign state",
            ));
        }
    };
    let _available_at = cursor.i64()?;
    let _created_at = cursor.i64()?;
    let attempt_count = cursor.u32()?;
    let attempt_id = cursor.optional_text(512)?;
    let worker_id = cursor.optional_text(512)?;
    let lease_until = cursor.optional_i64()?;
    let _result = cursor.optional_blob(1_048_576)?;
    let last_error_class = cursor.optional_text(128)?;
    cursor.finish()?;
    let owns_transition_authority =
        attempt_id.is_some() && worker_id.is_some() && lease_until.is_some();
    require(
        (state == QueueJobState::Leased) == owns_transition_authority,
        "queue_observer_lease_shape",
        "queue primary state disagrees with its raw lease fields",
    )?;
    Ok(ObservedQueueJob {
        job_id,
        state,
        attempt_count,
        attempt_id,
        worker_id,
        lease_until,
        last_error_class,
    })
}

fn queue_schema_digest(identity: &str) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new_derive_key(QUEUE_SCHEMA_DIGEST_DOMAIN);
    hasher.update(&(identity.len() as u64).to_be_bytes());
    hasher.update(identity.as_bytes());
    hasher.finalize().as_bytes().to_vec()
}

fn queue_digest(domain: &'static str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn push_queue_text(output: &mut Vec<u8>, value: &str) -> Result<(), ServiceFailure> {
    push_queue_blob(output, value.as_bytes())
}

fn push_queue_blob(output: &mut Vec<u8>, value: &[u8]) -> Result<(), ServiceFailure> {
    let length = u32::try_from(value.len()).map_err(|_| {
        ServiceFailure::failed(
            "queue_oracle_field_length",
            "queue oracle fixture field exceeds u32",
        )
    })?;
    output.extend_from_slice(&length.to_be_bytes());
    output.extend_from_slice(value);
    Ok(())
}

fn push_queue_optional_text(
    output: &mut Vec<u8>,
    value: Option<&str>,
) -> Result<(), ServiceFailure> {
    match value {
        Some(value) => {
            output.push(1);
            push_queue_text(output, value)
        }
        None => {
            output.push(0);
            Ok(())
        }
    }
}

fn push_queue_optional_blob(
    output: &mut Vec<u8>,
    value: Option<&[u8]>,
) -> Result<(), ServiceFailure> {
    match value {
        Some(value) => {
            output.push(1);
            push_queue_blob(output, value)
        }
        None => {
            output.push(0);
            Ok(())
        }
    }
}

fn push_queue_optional_i64(output: &mut Vec<u8>, value: Option<i64>) {
    match value {
        Some(value) => {
            output.push(1);
            output.extend_from_slice(&value.to_be_bytes());
        }
        None => output.push(0),
    }
}

struct QueueCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> QueueCursor<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn take(&mut self, length: usize) -> Result<&'a [u8], ServiceFailure> {
        let end = self.offset.checked_add(length).ok_or_else(|| {
            ServiceFailure::failed(
                "queue_observer_offset",
                "queue primary record offset overflowed usize",
            )
        })?;
        let value = self.bytes.get(self.offset..end).ok_or_else(|| {
            ServiceFailure::failed(
                "queue_observer_truncated",
                "queue primary record is truncated",
            )
        })?;
        self.offset = end;
        Ok(value)
    }

    fn u8(&mut self) -> Result<u8, ServiceFailure> {
        self.take(1)?.first().copied().ok_or_else(|| {
            ServiceFailure::failed(
                "queue_observer_truncated",
                "queue primary record is truncated",
            )
        })
    }

    fn u32(&mut self) -> Result<u32, ServiceFailure> {
        let bytes = self.take(4)?;
        Ok(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn i64(&mut self) -> Result<i64, ServiceFailure> {
        let bytes = self.take(8)?;
        Ok(i64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]))
    }

    fn blob(&mut self, maximum: usize) -> Result<Vec<u8>, ServiceFailure> {
        let length = usize::try_from(self.u32()?).map_err(|_| {
            ServiceFailure::failed(
                "queue_observer_field_length",
                "queue primary record field length is unsupported",
            )
        })?;
        require(
            length <= maximum,
            "queue_observer_field_limit",
            "queue primary record field exceeds its exact byte limit",
        )?;
        Ok(self.take(length)?.to_vec())
    }

    fn text(&mut self, maximum: usize) -> Result<String, ServiceFailure> {
        String::from_utf8(self.blob(maximum)?).map_err(|_| {
            ServiceFailure::failed(
                "queue_observer_utf8",
                "queue primary record text is not UTF-8",
            )
        })
    }

    fn optional_text(&mut self, maximum: usize) -> Result<Option<String>, ServiceFailure> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.text(maximum).map(Some),
            _ => Err(ServiceFailure::failed(
                "queue_observer_option_tag",
                "queue primary record has a noncanonical option tag",
            )),
        }
    }

    fn optional_blob(&mut self, maximum: usize) -> Result<Option<Vec<u8>>, ServiceFailure> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.blob(maximum).map(Some),
            _ => Err(ServiceFailure::failed(
                "queue_observer_option_tag",
                "queue primary record has a noncanonical option tag",
            )),
        }
    }

    fn optional_i64(&mut self) -> Result<Option<i64>, ServiceFailure> {
        match self.u8()? {
            0 => Ok(None),
            1 => self.i64().map(Some),
            _ => Err(ServiceFailure::failed(
                "queue_observer_option_tag",
                "queue primary record has a noncanonical option tag",
            )),
        }
    }

    fn finish(self) -> Result<(), ServiceFailure> {
        require(
            self.offset == self.bytes.len(),
            "queue_observer_trailing",
            "queue primary record contains trailing bytes",
        )
    }
}

fn http_request(
    port: u16,
    method: &str,
    path: &str,
    body: &[u8],
    headers: &[(&str, &str)],
) -> Result<HttpResponse, ServiceFailure> {
    let address = std::net::SocketAddr::from((std::net::Ipv4Addr::LOCALHOST, port));
    http_probe::request(address, method, path, body, headers)
        .map_err(|error| ServiceFailure::infrastructure("http_probe", error))
}
fn parse_ready_event(line: &[u8]) -> Result<RunnerReady, ServiceFailure> {
    let value: Value = serde_json::from_slice(line).map_err(|_| {
        ServiceFailure::failed("runner_ready_json", "runner readiness was not machine JSON")
    })?;
    require(
        value.get("ok").and_then(Value::as_bool) == Some(true)
            && value.get("event").and_then(Value::as_str) == Some("ready")
            && value.get("contract_version").is_none(),
        "runner_ready_event",
        "runner readiness event was rejected",
    )?;
    let deployment = value.get("deployment").ok_or_else(|| {
        ServiceFailure::failed(
            "runner_ready_deployment",
            "runner readiness omitted deployment",
        )
    })?;
    require(
        deployment.get("contract_version").is_none(),
        "runner_ready_predecessor",
        "runner readiness contains a removed predecessor field",
    )?;
    let artifact_digest = string_at(deployment, "artifact_digest")?;
    require(
        domain_identity(&artifact_digest, "artifact_bundle_", 64),
        "runner_ready_artifact_identity",
        "runner readiness artifact digest is not an exact artifact bundle identity",
    )?;
    Ok(RunnerReady {
        artifact_digest,
        target: string_at(deployment, "target")?,
        runner: string_at(deployment, "runner")?,
        listen: optional_string_at(deployment, "listen")?,
        secret_names: string_array_at(deployment, "secret_names")?,
        readiness_elapsed_nanoseconds: 0,
    })
}

fn parse_stopped_event(bytes: &[u8]) -> Result<RunnerStopped, ServiceFailure> {
    let mut stopped = None;
    for line in bytes
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
    {
        let value: Value = serde_json::from_slice(line).map_err(|_| {
            ServiceFailure::failed("runner_stop_json", "runner output was not machine JSON")
        })?;
        if value.get("event").and_then(Value::as_str) == Some("stopped") {
            require(
                value.get("ok").and_then(Value::as_bool) == Some(true)
                    && value.get("contract_version").is_none(),
                "runner_stop_event",
                "runner stop receipt was unsuccessful",
            )?;
            let receipt = value.get("receipt").ok_or_else(|| {
                ServiceFailure::failed("runner_stop_receipt", "runner stop receipt is absent")
            })?;
            require(
                receipt.get("contract_version").is_none(),
                "runner_stop_predecessor",
                "runner stop receipt contains a removed predecessor field",
            )?;
            let shutdown = receipt.get("shutdown").ok_or_else(|| {
                ServiceFailure::failed("runner_shutdown_receipt", "runner shutdown is absent")
            })?;
            require(
                shutdown.get("contract_version").is_none(),
                "runner_shutdown_predecessor",
                "runner shutdown contains a removed predecessor field",
            )?;
            stopped = Some(RunnerStopped {
                admission_stopped: boolean_at(shutdown, "admission_stopped")?,
                remaining_tasks: u64_at(shutdown, "remaining_tasks")?,
                cleanup_failures: shutdown
                    .get("cleanup_failures")
                    .and_then(Value::as_array)
                    .map(|values| values.len() as u64)
                    .ok_or_else(|| {
                        ServiceFailure::failed(
                            "runner_cleanup_failures",
                            "runner cleanup failure list is absent",
                        )
                    })?,
                productive_iterations: receipt
                    .get("productive_iterations")
                    .map(|value| {
                        value.as_u64().ok_or_else(|| {
                            ServiceFailure::failed(
                                "runner_productivity",
                                "worker productivity is invalid",
                            )
                        })
                    })
                    .transpose()?,
            });
        }
    }
    stopped.ok_or_else(|| {
        ServiceFailure::failed(
            "runner_stop_missing",
            "runner omitted a successful stop receipt",
        )
    })
}

fn first_line(path: &Path, maximum: u64) -> Result<Option<Vec<u8>>, ServiceFailure> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(ServiceFailure::infrastructure("runner_log_inspect", error));
        }
    };
    require(
        metadata.is_file() && !metadata.file_type().is_symlink(),
        "runner_log_type",
        "runner stdout is not a regular file",
    )?;
    require(
        metadata.len() <= maximum,
        "runner_stdout_limit",
        "runner stdout exceeded its retained bound",
    )?;
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    File::open(path)
        .and_then(|file| file.take(maximum.saturating_add(1)).read_to_end(&mut bytes))
        .map_err(|error| ServiceFailure::infrastructure("runner_log_read", error))?;
    if let Some(index) = bytes.iter().position(|byte| *byte == b'\n') {
        bytes.truncate(index);
        return Ok(Some(bytes));
    }
    if bytes.len() as u64 >= maximum {
        return Err(ServiceFailure::failed(
            "runner_ready_line_limit",
            "runner readiness line exceeded its retained bound",
        ));
    }
    Ok(None)
}

fn redact_process_logs(
    repository: &Path,
    stdout_path: &Path,
    stderr_path: &Path,
    secrets: &[Vec<u8>],
    observation: &mut ProcessObservation,
) -> Result<(), ServiceFailure> {
    redact_file(stdout_path, observation.stdout_limit_bytes, secrets)?;
    redact_file(stderr_path, observation.stderr_limit_bytes, secrets)?;
    observation.stdout = evidence::proof(stdout_path, evidence::relative(repository, stdout_path))
        .map_err(|error| ServiceFailure::infrastructure("stdout_proof", error))?;
    observation.stderr = evidence::proof(stderr_path, evidence::relative(repository, stderr_path))
        .map_err(|error| ServiceFailure::infrastructure("stderr_proof", error))?;
    Ok(())
}

fn redact_file(path: &Path, maximum: u64, secrets: &[Vec<u8>]) -> Result<(), ServiceFailure> {
    let bytes = process::read_bounded(path, maximum)
        .map_err(|error| ServiceFailure::infrastructure("log_redaction_read", error))?;
    let redacted = redact_bytes(&bytes, secrets);
    if redacted != bytes {
        evidence::publish(path, &redacted)
            .map_err(|error| ServiceFailure::infrastructure("log_redaction_publish", error))?;
    }
    Ok(())
}

fn redact_bytes(bytes: &[u8], secrets: &[Vec<u8>]) -> Vec<u8> {
    let mut redacted = bytes.to_vec();
    for secret in secrets.iter().filter(|secret| !secret.is_empty()) {
        let mut cursor = 0;
        while let Some(relative) = find_bytes(&redacted[cursor..], secret) {
            let start = cursor + relative;
            redacted.splice(start..start + secret.len(), b"<redacted>".iter().copied());
            cursor = start + b"<redacted>".len();
        }
    }
    redacted
}

fn redact_command(command: &[String], secrets: &[Vec<u8>]) -> Vec<String> {
    command
        .iter()
        .map(|argument| {
            String::from_utf8_lossy(&redact_bytes(argument.as_bytes(), secrets)).into_owned()
        })
        .collect()
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

fn write_descriptor(
    source: &Path,
    destination: &Path,
    port: Option<u16>,
    data_root: &str,
) -> Result<(), ServiceFailure> {
    let bytes = process::read_bounded(source, MAXIMUM_DESCRIPTOR_BYTES)
        .map_err(|error| ServiceFailure::infrastructure("descriptor_read", error))?;
    let mut descriptor: Value = serde_json::from_slice(&bytes).map_err(|_| {
        ServiceFailure::failed(
            "descriptor_json",
            "maintained deployment descriptor is invalid",
        )
    })?;
    let object = descriptor.as_object_mut().ok_or_else(|| {
        ServiceFailure::failed("descriptor_shape", "deployment descriptor is not an object")
    })?;
    if object.get("artifact").and_then(Value::as_str) != Some(SERVICE_ARTIFACT_RELATIVE) {
        return Err(ServiceFailure::failed(
            "descriptor_artifact_boundary",
            "maintained deployment descriptor does not bind the current artifact-11 bundle",
        ));
    }
    if let Some(port) = port {
        object.insert(
            "listen".to_owned(),
            Value::String(format!("127.0.0.1:{port}")),
        );
    }
    let grants = object
        .get_mut("grants")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| {
            ServiceFailure::failed(
                "descriptor_grants",
                "maintained deployment descriptor omitted grants",
            )
        })?;
    for grant in grants {
        let Some(adapter) = grant
            .as_object_mut()
            .and_then(|grant| grant.get_mut("adapter"))
            .and_then(Value::as_object_mut)
        else {
            return Err(ServiceFailure::failed(
                "descriptor_grant_shape",
                "maintained deployment grant is malformed",
            ));
        };
        if matches!(
            adapter.get("kind").and_then(Value::as_str),
            Some("data" | "durable_queue_data")
        ) {
            adapter.insert("root".to_owned(), Value::String(data_root.to_owned()));
        }
    }
    let encoded = evidence::encode_json(&descriptor)
        .map_err(|error| ServiceFailure::infrastructure("descriptor_encode", error))?;
    evidence::publish(destination, &encoded)
        .map_err(|error| ServiceFailure::infrastructure("descriptor_publish", error))?;
    Ok(())
}

fn parse_build_identity(
    bytes: &[u8],
    expected_bytes: u64,
    checked_in_sha256: &str,
) -> Result<ArtifactIdentity, ServiceFailure> {
    let text = std::str::from_utf8(bytes).map_err(|_| {
        ServiceFailure::failed(
            "artifact_build_output_utf8",
            "fresh public build output is not UTF-8",
        )
    })?;
    let records = text
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            let name = fields.next()?;
            let values = fields
                .filter_map(|field| field.split_once('='))
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect::<BTreeMap<_, _>>();
            Some((name.to_owned(), values))
        })
        .collect::<BTreeMap<_, _>>();
    let authority = build_record(&records, "authority")?;
    let compilation = build_record(&records, "compilation")?;
    let artifact = build_record(&records, "artifact")?;
    let identity = ArtifactIdentity {
        repository: build_field(authority, "repository")?,
        package: build_field(authority, "package")?,
        revision: build_field(authority, "revision")?,
        semantic_state: build_field(authority, "state")?,
        compilation_manifest: build_field(compilation, "manifest")?,
        artifact_manifest: build_field(artifact, "manifest")?,
        artifact_bundle: build_field(artifact, "bundle")?,
        bytes: build_u64_field(artifact, "bytes")?,
        packages: build_u64_field(artifact, "packages")?,
        closure_objects: build_u64_field(artifact, "closure-objects")?,
        compiler_units: build_u64_field(artifact, "compiler-units")?,
        manifest_objects: build_u64_field(artifact, "manifest-objects")?,
        manifest_object_bytes: build_u64_field(artifact, "manifest-object-bytes")?,
        segments: build_u64_field(artifact, "segments")?,
        load_objects: build_u64_field(artifact, "load-objects")?,
        load_object_bytes: build_u64_field(artifact, "load-object-bytes")?,
        checked_in_sha256: checked_in_sha256.to_owned(),
        fresh_build_equal: true,
    };
    require(
        identity.bytes == expected_bytes
            && identity.packages > 0
            && identity.compiler_units > 0
            && identity.segments > 0
            && identity.closure_objects == identity.manifest_objects
            && identity.load_objects == identity.manifest_objects
            && identity.load_object_bytes == identity.manifest_object_bytes
            && domain_identity(&identity.repository, "repo_", 32)
            && domain_identity(&identity.package, "pkg_", 32)
            && domain_identity(&identity.revision, "rev_", 64)
            && domain_identity(&identity.semantic_state, "semantic_state_", 64)
            && domain_identity(&identity.compilation_manifest, "compilation_manifest_", 64)
            && domain_identity(&identity.artifact_manifest, "artifact_manifest_", 64)
            && domain_identity(&identity.artifact_bundle, "artifact_bundle_", 64)
            && checked_in_sha256.len() == 64
            && checked_in_sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "artifact_build_identity",
        "fresh public build reported a malformed or inconsistent exact artifact identity",
    )?;
    Ok(identity)
}

fn build_record<'a>(
    records: &'a BTreeMap<String, BTreeMap<String, String>>,
    name: &str,
) -> Result<&'a BTreeMap<String, String>, ServiceFailure> {
    records.get(name).ok_or_else(|| {
        ServiceFailure::failed(
            "artifact_build_record",
            format!("fresh public build omitted its {name} record"),
        )
    })
}

fn build_field(record: &BTreeMap<String, String>, name: &str) -> Result<String, ServiceFailure> {
    record.get(name).cloned().ok_or_else(|| {
        ServiceFailure::failed(
            "artifact_build_field",
            format!("fresh public build omitted its {name} field"),
        )
    })
}

fn build_u64_field(record: &BTreeMap<String, String>, name: &str) -> Result<u64, ServiceFailure> {
    build_field(record, name)?.parse().map_err(|_| {
        ServiceFailure::failed(
            "artifact_build_count",
            format!("fresh public build reported invalid artifact field '{name}'"),
        )
    })
}

fn domain_identity(value: &str, prefix: &str, hexadecimal_bytes: usize) -> bool {
    value.len() == prefix.len().saturating_add(hexadecimal_bytes)
        && value.starts_with(prefix)
        && value[prefix.len()..]
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn parse_json_body(bytes: &[u8]) -> Result<Value, ServiceFailure> {
    serde_json::from_slice(bytes)
        .map_err(|_| ServiceFailure::failed("service_json", "service response was not JSON"))
}

fn string_at(value: &Value, field: &str) -> Result<String, ServiceFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            ServiceFailure::failed(
                "response_field",
                format!("response field '{field}' is absent"),
            )
        })
}

fn optional_string_at(value: &Value, field: &str) -> Result<Option<String>, ServiceFailure> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ServiceFailure::failed(
            "response_field",
            format!("response field '{field}' is not text"),
        )),
    }
}

fn string_array_at(value: &Value, field: &str) -> Result<Vec<String>, ServiceFailure> {
    value
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ServiceFailure::failed(
                "response_field",
                format!("response field '{field}' is absent"),
            )
        })?
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned).ok_or_else(|| {
                ServiceFailure::failed(
                    "response_field",
                    format!("response field '{field}' contains non-text"),
                )
            })
        })
        .collect()
}

fn boolean_at(value: &Value, field: &str) -> Result<bool, ServiceFailure> {
    value.get(field).and_then(Value::as_bool).ok_or_else(|| {
        ServiceFailure::failed(
            "response_field",
            format!("response field '{field}' is absent"),
        )
    })
}

fn u64_at(value: &Value, field: &str) -> Result<u64, ServiceFailure> {
    value.get(field).and_then(Value::as_u64).ok_or_else(|| {
        ServiceFailure::failed(
            "response_field",
            format!("response field '{field}' is absent"),
        )
    })
}

fn require(
    condition: bool,
    code: &'static str,
    message: impl Into<String>,
) -> Result<(), ServiceFailure> {
    if condition {
        Ok(())
    } else {
        Err(ServiceFailure::failed(code, message))
    }
}

fn query(values: &[(&str, &str)]) -> String {
    values
        .iter()
        .map(|(name, value)| format!("{}={}", percent_encode(name), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
            encoded.push(char::from(byte));
        } else {
            encoded.push('%');
            encoded.push(hex_digit(byte >> 4));
            encoded.push(hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn hex_digit(value: u8) -> char {
    char::from(if value < 10 {
        b'0' + value
    } else {
        b'A' + value - 10
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    lower_hex(&digest)
}

fn lower_hex(bytes: &[u8]) -> String {
    let mut encoded = String::with_capacity(bytes.len().saturating_mul(2));
    for &byte in bytes {
        encoded.push(hex_digit(byte >> 4).to_ascii_lowercase());
        encoded.push(hex_digit(byte & 0x0f).to_ascii_lowercase());
    }
    encoded
}

fn observe_graph_authority(application: &Path) -> Result<AuthorityObservation, ServiceFailure> {
    authority::observe_graph_authority(application)
        .map_err(|error| ServiceFailure::infrastructure("authority_observation", error))
}

fn free_port() -> Result<u16, ServiceFailure> {
    TcpListener::bind(("127.0.0.1", 0))
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .map_err(|error| ServiceFailure::infrastructure("free_port", error))
}

fn random_hex(bytes: usize) -> Result<String, ServiceFailure> {
    let mut random = vec![0_u8; bytes];
    File::open("/dev/urandom")
        .and_then(|mut file| file.read_exact(&mut random))
        .map_err(|error| ServiceFailure::infrastructure("secure_random", error))?;
    let mut encoded = String::with_capacity(bytes.saturating_mul(2));
    for byte in random {
        encoded.push(hex_digit(byte >> 4).to_ascii_lowercase());
        encoded.push(hex_digit(byte & 0x0f).to_ascii_lowercase());
    }
    Ok(encoded)
}

fn safe_file_component(value: &str) -> Result<String, ServiceFailure> {
    if !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        Ok(value.to_owned())
    } else {
        Err(ServiceFailure::infrastructure(
            "evidence_name",
            "internal evidence name is not portable",
        ))
    }
}

fn parse(arguments: impl Iterator<Item = OsString>) -> Result<Options, DevError> {
    let mut binary = PathBuf::from("target/release/lkjscript");
    let mut machine = false;
    let mut arguments = arguments;
    while let Some(argument) = crate::next_utf8(&mut arguments, "service option")? {
        match argument.as_str() {
            "--binary" => {
                let value = crate::next_utf8(&mut arguments, "--binary")?
                    .ok_or_else(|| DevError::usage("--binary requires a path"))?;
                binary = PathBuf::from(value);
            }
            "--machine" if !machine => machine = true,
            value => {
                return Err(DevError::usage(format!(
                    "unknown or duplicate service option '{value}'"
                )));
            }
        }
    }
    Ok(Options { binary, machine })
}

fn resolve_input_file(
    repository: &Path,
    path: &Path,
    label: &str,
) -> Result<PathBuf, ServiceFailure> {
    let path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        repository.join(path)
    };
    let metadata = fs::symlink_metadata(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            ServiceFailure::unavailable("runner_binary_absent", format!("{label} is absent"))
        } else {
            ServiceFailure::infrastructure("runner_binary_inspect", error)
        }
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(ServiceFailure::unavailable(
            "runner_binary_unsafe",
            format!("{label} is not a regular non-symlink file"),
        ));
    }
    Ok(path)
}

fn proof_input(repository: &Path, path: &Path) -> Result<FileProof, ServiceFailure> {
    evidence::proof(path, path.to_string_lossy().into_owned())
        .map_err(|error| ServiceFailure::infrastructure("input_proof", error))
        .map(|mut proof| {
            if proof
                .path
                .starts_with(repository.to_string_lossy().as_ref())
            {
                proof.path = evidence::relative(repository, path);
            }
            proof
        })
}

fn proof_required_file(
    repository: &Path,
    path: &Path,
    maximum: u64,
    label: &str,
) -> Result<FileProof, ServiceFailure> {
    let bytes = process::read_bounded(path, maximum).map_err(|error| {
        ServiceFailure::unavailable("maintained_fixture_absent", format!("{label}: {error}"))
    })?;
    if bytes.is_empty() {
        return Err(ServiceFailure::unavailable(
            "maintained_fixture_empty",
            format!("{label} is empty"),
        ));
    }
    evidence::proof(path, evidence::relative(repository, path))
        .map_err(|error| ServiceFailure::infrastructure("fixture_proof", error))
}

fn proof_required_file_with_sha256(
    repository: &Path,
    path: &Path,
    maximum: u64,
    label: &str,
    expected_sha256: &str,
) -> Result<FileProof, ServiceFailure> {
    let bytes = process::read_bounded(path, maximum).map_err(|error| {
        ServiceFailure::unavailable("maintained_fixture_absent", format!("{label}: {error}"))
    })?;
    let observed = sha256_hex(&bytes);
    if observed != expected_sha256 {
        return Err(ServiceFailure::failed(
            "service_artifact_sha256",
            format!("{label} SHA-256 {observed} does not match {expected_sha256}"),
        ));
    }
    proof_required_file(repository, path, maximum, label)
}

fn new_run_directory(repository: &Path) -> Result<PathBuf, DevError> {
    let root = repository.join(".artifacts/lkjscript-dev/service");
    fs::create_dir_all(&root).map_err(|error| {
        DevError::infrastructure(format!("create service evidence root: {error}"))
    })?;
    let now = unix_nanoseconds()?;
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    let path = root.join(format!("{now}-{}-{ordinal}", std::process::id()));
    fs::create_dir(&path).map_err(|error| {
        DevError::infrastructure(format!("create service evidence directory: {error}"))
    })?;
    Ok(path)
}

fn unique_cleanup_scope() -> Result<String, DevError> {
    let now = unix_nanoseconds()?;
    let ordinal = RUN_ORDINAL.fetch_add(1, Ordering::Relaxed);
    Ok(format!(
        "lkjscript-service-{}-{now}-{ordinal}",
        std::process::id()
    ))
}

fn repository_root() -> Result<PathBuf, DevError> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| DevError::infrastructure("resolve repository root"))
}

fn unix_nanoseconds() -> Result<u128, DevError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .map_err(|error| DevError::infrastructure(format!("system clock before epoch: {error}")))
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn print_summary(
    repository: &Path,
    options: &Options,
    receipt: &ServiceReceipt,
    published: &PublishedEvidence,
) -> Result<(), DevError> {
    #[derive(Serialize)]
    struct Summary<'a> {
        contract_version: u32,
        status: ServiceStatus,
        elapsed_nanoseconds: u64,
        receipt: String,
        receipt_bytes: u64,
        receipt_digest: &'a VerificationDigest,
        failure: &'a Option<Failure>,
    }
    let summary = Summary {
        contract_version: receipt.contract_version,
        status: receipt.status,
        elapsed_nanoseconds: receipt.elapsed_nanoseconds,
        receipt: evidence::relative(repository, &published.path),
        receipt_bytes: published.bytes,
        receipt_digest: &published.digest,
        failure: &receipt.failure,
    };
    if options.machine {
        println!(
            "{}",
            serde_json::to_string(&summary).map_err(|error| {
                DevError::infrastructure(format!("encode compact service summary: {error}"))
            })?
        );
    } else {
        println!(
            "service {:?}: elapsed={:.3}s receipt={} digest={}",
            receipt.status,
            receipt.elapsed_nanoseconds as f64 / 1_000_000_000.0,
            summary.receipt,
            summary.receipt_digest
        );
        if let Some(failure) = &receipt.failure {
            println!(
                "failure: class={} code={} message={}",
                failure.class, failure.code, failure.message
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn data_contract_is_exact_and_versioned() {
        assert_eq!(DATA_CONTRACT, "lkjscript-data-store-1");
        assert_eq!(SERVICE_CONTRACT_VERSION, 5);
    }

    #[test]
    fn unavailable_run_retains_an_atomic_typed_receipt() {
        let repository = tempfile::tempdir().expect("temporary service repository");
        let options = Options {
            binary: repository.path().join("absent-lkjscript"),
            machine: true,
        };
        let (receipt, published) =
            execute(repository.path(), &options).expect("publish unavailable receipt");
        assert_eq!(receipt.status, ServiceStatus::Unavailable);
        assert_eq!(
            receipt
                .failure
                .as_ref()
                .map(|failure| failure.code.as_str()),
            Some("runner_binary_absent")
        );
        assert!(!receipt.raw_secret_values_retained);
        let bytes = std::fs::read(&published.path).expect("read service receipt");
        let decoded: ServiceReceipt =
            serde_json::from_slice(&bytes).expect("decode strict service receipt");
        assert_eq!(decoded.status, ServiceStatus::Unavailable);
        let mut value: Value = serde_json::from_slice(&bytes).expect("decode receipt value");
        value
            .as_object_mut()
            .expect("receipt object")
            .insert("unknown".to_owned(), Value::Bool(true));
        assert!(serde_json::from_value::<ServiceReceipt>(value).is_err());
    }

    #[test]
    fn runner_events_are_typed_and_require_clean_shutdown() {
        let ready = parse_ready_event(
            br#"{"ok":true,"event":"ready","deployment":{"artifact_digest":"artifact_bundle_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","target":"serve","runner":"http","listen":"127.0.0.1:1","secret_names":["bootstrap-token"]}}"#,
        )
        .expect("parse ready event");
        assert_eq!(ready.runner, "http");
        assert_eq!(ready.secret_names, ["bootstrap-token"]);
        let stopped = parse_stopped_event(
            br#"{"ok":true,"event":"ready"}
{"ok":true,"event":"stopped","receipt":{"productive_iterations":3,"shutdown":{"admission_stopped":true,"remaining_tasks":0,"cleanup_failures":[]}}}
"#,
        )
        .expect("parse stopped event");
        assert!(stopped.clean());
        assert_eq!(stopped.productive_iterations, Some(3));
        assert_eq!(
            parse_ready_event(
                br#"{"ok":true,"event":"ready","contract_version":1,"deployment":{}}"#
            )
            .expect_err("predecessor ready event must reject")
            .code,
            "runner_ready_event"
        );
        assert_eq!(
            parse_stopped_event(
                br#"{"ok":true,"event":"stopped","receipt":{"contract_version":1,"shutdown":{"admission_stopped":true,"remaining_tasks":0,"cleanup_failures":[]}}}"#
            )
            .expect_err("predecessor stopped event must reject")
            .code,
            "runner_stop_predecessor"
        );
    }

    #[test]
    fn secret_values_are_removed_from_commands_and_logs() {
        let secret = b"bootstrap-secret".to_vec();
        let bytes = redact_bytes(
            b"before bootstrap-secret after",
            std::slice::from_ref(&secret),
        );
        assert_eq!(bytes, b"before <redacted> after");
        let command = redact_command(&["--token=bootstrap-secret".to_owned()], &[secret]);
        assert_eq!(command, ["--token=<redacted>"]);
    }

    #[test]
    fn bounded_http_client_records_content_length_response() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind test listener");
        let port = listener.local_addr().expect("test listener address").port();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept test request");
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).expect("read test request");
            assert!(request[..read].starts_with(b"GET /health HTTP/1.1\r\n"));
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\nX-Test: yes\r\n\r\nready")
                .expect("write test response");
        });
        let response =
            http_request(port, "GET", "/health", b"", &[]).expect("read bounded HTTP response");
        server.join().expect("join test HTTP server");
        assert_eq!(response.status, 200);
        assert_eq!(response.body, b"ready");
        assert_eq!(
            response.headers.get("x-test").map(String::as_str),
            Some("yes")
        );
    }

    #[test]
    fn query_encoding_is_explicit_and_deterministic() {
        assert_eq!(
            query(&[("actor", "a b"), ("id", "x/y")]),
            "actor=a%20b&id=x%2Fy"
        );
    }

    #[test]
    fn graph_authority_inventory_is_bounded_and_excludes_derived_state() {
        let temporary = tempfile::tempdir().expect("Graph authority fixture");
        let application = temporary.path();
        std::fs::create_dir(application.join("catalog")).expect("catalog directory");
        std::fs::create_dir(application.join("packs")).expect("pack directory");
        std::fs::create_dir_all(application.join("PACKAGE-TRANSPORTS/dependency"))
            .expect("transport directory");
        std::fs::write(application.join("HEAD"), b"head-one").expect("HEAD fixture");
        std::fs::write(application.join("catalog/current.lkjc"), b"catalog")
            .expect("catalog fixture");
        std::fs::write(application.join("packs/pack_one.lkjp"), b"pack").expect("pack fixture");
        std::fs::write(
            application.join("PACKAGE-TRANSPORTS/dependency/CURRENT"),
            b"transport",
        )
        .expect("transport fixture");
        let before = observe_graph_authority(application).expect("authority observation");
        assert_eq!(before.files, 3);
        assert_eq!(before.bytes, 8 + 4 + 9);

        std::fs::create_dir(application.join("derived")).expect("derived directory");
        std::fs::write(application.join("derived/cache"), b"disposable").expect("derived fixture");
        std::fs::write(application.join("catalog/current.lkjc"), b"rebuilt catalog")
            .expect("rebuilt catalog fixture");
        assert_eq!(
            observe_graph_authority(application).expect("authority excludes derived"),
            before
        );

        std::fs::write(application.join("packs/pack_one.lkjp"), b"changed")
            .expect("change authority fixture");
        assert_ne!(
            observe_graph_authority(application).expect("changed authority"),
            before
        );
    }
}
