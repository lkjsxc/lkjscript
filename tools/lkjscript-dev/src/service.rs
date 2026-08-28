use crate::authority::{self, AuthorityObservation};
use crate::error::DevError;
use crate::evidence::{self, FileProof, PublishedEvidence, VerificationDigest};
use crate::http_probe::{self, HttpResponse};
use crate::process::{self, ProcessControl, ProcessObservation, ProcessSpec, ProcessStatus};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::{self, File};
use std::io::Read;
#[cfg(test)]
use std::io::Write;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const SERVICE_CONTRACT_VERSION: u32 = 2;
pub(crate) const POSTGRES_IMAGE: &str =
    "postgres@sha256:075f7ba66bc9b3ce7d6b8b635208ff61cd7cf1a67d71ec530eec5d7ae0cbe571";
const SERVICE_ARTIFACT_RELATIVE: &str = "generated/lkjournal.lkja";
const SERVICE_ARTIFACT_SHA256: &str =
    "80c69d69aec80e49cc0c023ec65eef3106f4a876eff1dc347defb461f3037ccb";
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
const POSTGRES_READY_TIMEOUT: Duration = Duration::from_secs(30);
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
    postgres_image: String,
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
    cleanup: ContainerCleanup,
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
struct ContainerCleanup {
    exact_name: String,
    attempted: bool,
    completed: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ServiceResult {
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
    database_backup: FileProof,
    restored_read_equal: bool,
    shutdown_cleanup_failures: u64,
    initialization_transport: InitializationTransport,
    initialization_observation: InitializationObservation,
    request_elapsed_nanoseconds: BTreeMap<String, u64>,
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
    stdin: Option<(&'a Path, u64)>,
    unavailable_on_failure: bool,
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
            stdin: None,
            unavailable_on_failure: false,
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

    fn output_limits(mut self, stdout: u64, stderr: u64) -> Self {
        self.maximum_stdout_bytes = stdout;
        self.maximum_stderr_bytes = stderr;
        self
    }

    fn stdin(mut self, path: &'a Path, maximum: u64) -> Self {
        self.stdin = Some((path, maximum));
        self
    }

    fn unavailable_on_failure(mut self) -> Self {
        self.unavailable_on_failure = true;
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
    container_name: String,
    container_started: bool,
    cleanup: ContainerCleanup,
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

fn execute(
    repository: &Path,
    options: &Options,
) -> Result<(ServiceReceipt, PublishedEvidence), DevError> {
    let run_directory = new_run_directory(repository)?;
    let receipt_path = run_directory.join("receipt.json");
    let started_wall = unix_nanoseconds()?;
    let started = Instant::now();
    let container_name = unique_container_name()?;
    let mut context = ServiceContext::new(repository, &run_directory, container_name);
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
            "maintained artifact-10 service bundle",
            SERVICE_ARTIFACT_SHA256,
        )?);
        run_acceptance(&mut context, &binary)
    })();

    let runner_cleanup = context.cleanup_runners();
    let container_cleanup = context.cleanup_container();
    let final_result = match (workflow, runner_cleanup, container_cleanup) {
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
        postgres_image: POSTGRES_IMAGE.to_owned(),
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
        secret_environment_names: vec![
            "LKJOURNAL_BOOTSTRAP_TOKEN".to_owned(),
            "LKJOURNAL_DATABASE_URL".to_owned(),
            "POSTGRES_PASSWORD".to_owned(),
        ],
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
    fn new(repository: &Path, run_directory: &Path, container_name: String) -> Self {
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
            cleanup: ContainerCleanup {
                exact_name: container_name.clone(),
                attempted: false,
                completed: false,
            },
            container_name,
            container_started: false,
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
        let mut observation = match request.stdin {
            Some((path, maximum)) => {
                process::run_with_stdin_file(&specification, &self.repository, path, maximum)
            }
            None => process::run(&specification, &self.repository),
        };
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
        let unavailable_on_failure = request.unavailable_on_failure;
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
        if output.observation.status == ProcessStatus::Unavailable || unavailable_on_failure {
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

    fn cleanup_container(&mut self) -> Result<(), ServiceFailure> {
        if !self.container_started {
            self.cleanup.completed = true;
            return Ok(());
        }
        self.cleanup.attempted = true;
        let name = self.container_name.clone();
        let output = self.observe_command(
            CommandRequest::standard("postgres-stop", container_stop_command(&name))
                .timeout(Duration::from_secs(15)),
        )?;
        if output.observation.status != ProcessStatus::Passed {
            return Err(ServiceFailure::failed(
                "postgres_cleanup_failed",
                "owned PostgreSQL container did not stop cleanly",
            ));
        }
        self.cleanup.completed = true;
        self.container_started = false;
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

fn container_stop_command(name: &str) -> Vec<String> {
    vec![
        "docker".to_owned(),
        "stop".to_owned(),
        "--time".to_owned(),
        "5".to_owned(),
        name.to_owned(),
    ]
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

    context.invoke(
        CommandRequest::standard(
            "docker-image-inspect",
            vec![
                "docker".to_owned(),
                "image".to_owned(),
                "inspect".to_owned(),
                POSTGRES_IMAGE.to_owned(),
            ],
        )
        .unavailable_on_failure(),
    )?;

    let database_password = random_hex(16)?;
    let bootstrap_token = random_hex(16)?;
    let application_password = random_hex(16)?;
    context.secret_values.extend([
        database_password.as_bytes().to_vec(),
        bootstrap_token.as_bytes().to_vec(),
        application_password.as_bytes().to_vec(),
    ]);

    let mut docker_environment = process::environment();
    docker_environment.insert("POSTGRES_PASSWORD".to_owned(), database_password.clone());
    context.cleanup.attempted = true;
    // Once creation is attempted, cleanup must target the exact generated name even when
    // the Docker client fails after the daemon has accepted the container.
    context.container_started = true;
    context.invoke(
        CommandRequest::standard(
            "postgres-start",
            vec![
                "docker".to_owned(),
                "run".to_owned(),
                "--rm".to_owned(),
                "--name".to_owned(),
                context.container_name.clone(),
                "-e".to_owned(),
                "POSTGRES_PASSWORD".to_owned(),
                "-e".to_owned(),
                "POSTGRES_DB=lkjournal".to_owned(),
                "-p".to_owned(),
                "127.0.0.1::5432".to_owned(),
                "-d".to_owned(),
                POSTGRES_IMAGE.to_owned(),
            ],
        )
        .environment(docker_environment),
    )?;

    let container_name = context.container_name.clone();
    let port_output = context.invoke(
        CommandRequest::standard(
            "postgres-port",
            vec![
                "docker".to_owned(),
                "port".to_owned(),
                container_name.clone(),
                "5432/tcp".to_owned(),
            ],
        )
        .output_limits(64 * 1024, MAXIMUM_COMMAND_STDERR_BYTES),
    )?;
    let postgres_port = parse_host_port(&port_output)?;
    wait_for_postgres(context, postgres_port)?;
    let database_url =
        format!("postgresql://postgres:{database_password}@127.0.0.1:{postgres_port}/lkjournal");
    context.secret_values.push(database_url.as_bytes().to_vec());

    let service_port = free_port()?;
    let service_path = context.run_directory.join("service.json");
    write_descriptor(&service_source, &service_path, Some(service_port))?;
    context.retain(&service_path)?;
    let worker_path = context.run_directory.join("worker.json");
    write_descriptor(&worker_source, &worker_path, None)?;
    context.retain(&worker_path)?;

    let mut runner_environment = process::environment();
    runner_environment.insert(
        "LKJOURNAL_BOOTSTRAP_TOKEN".to_owned(),
        bootstrap_token.clone(),
    );
    runner_environment.insert("LKJOURNAL_DATABASE_URL".to_owned(), database_url.clone());
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
        "service readiness disagrees with the exact fresh artifact-10 build",
    )?;
    require(
        ready.secret_names == ["bootstrap-token", "database-url"],
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

    let worker_index = context.start_runner(
        "worker",
        vec![
            binary.to_string_lossy().into_owned(),
            "worker".to_owned(),
            "--deployment".to_owned(),
            "worker.json".to_owned(),
        ],
        &context.run_directory.clone(),
        runner_environment.clone(),
    )?;
    let worker_ready = context.runner_ready(worker_index)?;
    require(
        worker_ready.artifact_digest == artifact_identity.artifact_bundle
            && worker_ready.target == "work"
            && worker_ready.runner == "worker",
        "worker_artifact_identity",
        "worker readiness disagrees with the exact fresh artifact-10 build",
    )?;
    let worker_started = Instant::now();
    let mut job_state = String::new();
    while worker_started.elapsed() < WORKER_READY_TIMEOUT {
        job_state = postgres_scalar(
            context,
            "lkjournal",
            &format!("SELECT state FROM lkjscript_durable_jobs WHERE job_id = '{resource_id}';"),
        )?;
        if job_state == "completed" {
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    require(
        job_state == "completed",
        "worker_completion",
        "worker did not complete the durable job",
    )?;
    let worker_stopped = context.stop_runner(worker_index)?;
    let productive_iterations = worker_stopped.productive_iterations.unwrap_or(0);
    require(
        productive_iterations >= 1,
        "worker_productivity",
        "worker reported no productive work",
    )?;
    context.stop_runner(service_index)?;

    let backup_output = context.invoke(
        CommandRequest::standard(
            "postgres-backup",
            vec![
                "docker".to_owned(),
                "exec".to_owned(),
                container_name.clone(),
                "pg_dump".to_owned(),
                "-U".to_owned(),
                "postgres".to_owned(),
                "-d".to_owned(),
                "lkjournal".to_owned(),
                "--format=custom".to_owned(),
            ],
        )
        .output_limits(MAXIMUM_BACKUP_BYTES, MAXIMUM_COMMAND_STDERR_BYTES),
    )?;
    require(
        !backup_output.is_empty() && backup_output.len() as u64 <= MAXIMUM_BACKUP_BYTES,
        "database_backup_size",
        "database backup size is invalid",
    )?;
    let backup_path = context.run_directory.join("lkjournal.pgcustom");
    evidence::publish(&backup_path, &backup_output)
        .map_err(|error| ServiceFailure::infrastructure("database_backup_publish", error))?;
    let backup_proof = context.retain(&backup_path)?;

    context.invoke(CommandRequest::standard(
        "postgres-create-restore-database",
        vec![
            "docker".to_owned(),
            "exec".to_owned(),
            container_name.clone(),
            "createdb".to_owned(),
            "-U".to_owned(),
            "postgres".to_owned(),
            "lkjournal_restore".to_owned(),
        ],
    ))?;
    context.invoke(
        CommandRequest::standard(
            "postgres-restore",
            vec![
                "docker".to_owned(),
                "exec".to_owned(),
                "-i".to_owned(),
                container_name,
                "pg_restore".to_owned(),
                "-U".to_owned(),
                "postgres".to_owned(),
                "-d".to_owned(),
                "lkjournal_restore".to_owned(),
                "--exit-on-error".to_owned(),
            ],
        )
        .stdin(&backup_path, MAXIMUM_BACKUP_BYTES),
    )?;

    let restored_port = free_port()?;
    let restored_descriptor = context.run_directory.join("service-restored.json");
    write_descriptor(&service_source, &restored_descriptor, Some(restored_port))?;
    context.retain(&restored_descriptor)?;
    let restored_database_url = format!(
        "postgresql://postgres:{database_password}@127.0.0.1:{postgres_port}/lkjournal_restore"
    );
    context
        .secret_values
        .push(restored_database_url.as_bytes().to_vec());
    let mut restored_environment = runner_environment;
    restored_environment.insert("LKJOURNAL_DATABASE_URL".to_owned(), restored_database_url);
    let restored_index = context.start_runner(
        "service-restored",
        vec![
            binary.to_string_lossy().into_owned(),
            "serve".to_owned(),
            "--deployment".to_owned(),
            "service-restored.json".to_owned(),
        ],
        &context.run_directory.clone(),
        restored_environment,
    )?;
    let restored_ready = context.runner_ready(restored_index)?;
    require(
        restored_ready.artifact_digest == artifact_identity.artifact_bundle,
        "restored_artifact_identity",
        "restored service readiness changed the exact artifact-10 bundle identity",
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
        "service acceptance changed the maintained Graph 5 authority inventory",
    )?;

    Ok(ServiceResult {
        artifact_digest: ready.artifact_digest,
        artifact_identity,
        authority_before,
        authority_after,
        authority_unchanged: true,
        routes_checked: 13,
        resource_revision: 1,
        history_entries: 2,
        object_bytes: object_payload.len() as u64,
        worker_productive_iterations: productive_iterations,
        database_backup: backup_proof,
        restored_read_equal: true,
        shutdown_cleanup_failures: 0,
        initialization_transport,
        initialization_observation,
        request_elapsed_nanoseconds: timings,
    })
}

fn wait_for_postgres(context: &mut ServiceContext, host_port: u16) -> Result<(), ServiceFailure> {
    let started = Instant::now();
    let mut attempt = 0_u64;
    while started.elapsed() < POSTGRES_READY_TIMEOUT {
        let name = format!("postgres-ready-{attempt:03}");
        let output = context.observe_command(
            CommandRequest::standard(
                &name,
                vec![
                    "docker".to_owned(),
                    "exec".to_owned(),
                    context.container_name.clone(),
                    "pg_isready".to_owned(),
                    "-U".to_owned(),
                    "postgres".to_owned(),
                    "-d".to_owned(),
                    "lkjournal".to_owned(),
                ],
            )
            .timeout(Duration::from_secs(5))
            .output_limits(64 * 1024, 64 * 1024),
        )?;
        if output.observation.status == ProcessStatus::Passed
            && TcpStream::connect_timeout(
                &format!("127.0.0.1:{host_port}")
                    .parse()
                    .map_err(|error| ServiceFailure::infrastructure("postgres_address", error))?,
                Duration::from_millis(250),
            )
            .is_ok()
        {
            return Ok(());
        }
        attempt = attempt.saturating_add(1);
        thread::sleep(Duration::from_millis(250));
    }
    Err(ServiceFailure::failed(
        "postgres_readiness_timeout",
        "PostgreSQL did not become ready",
    ))
}

fn postgres_scalar(
    context: &mut ServiceContext,
    database: &str,
    statement: &str,
) -> Result<String, ServiceFailure> {
    let output = context.invoke(
        CommandRequest::standard(
            "postgres-scalar",
            vec![
                "docker".to_owned(),
                "exec".to_owned(),
                context.container_name.clone(),
                "psql".to_owned(),
                "-U".to_owned(),
                "postgres".to_owned(),
                "-d".to_owned(),
                database.to_owned(),
                "-Atc".to_owned(),
                statement.to_owned(),
            ],
        )
        .output_limits(64 * 1024, 64 * 1024),
    )?;
    String::from_utf8(output)
        .map(|value| value.trim().to_owned())
        .map_err(|_| {
            ServiceFailure::failed("postgres_output_utf8", "PostgreSQL output was not UTF-8")
        })
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
            && value.get("event").and_then(Value::as_str) == Some("ready"),
        "runner_ready_event",
        "runner readiness event was rejected",
    )?;
    let deployment = value.get("deployment").ok_or_else(|| {
        ServiceFailure::failed(
            "runner_ready_deployment",
            "runner readiness omitted deployment",
        )
    })?;
    let artifact_digest = string_at(deployment, "artifact_digest")?;
    require(
        domain_identity(&artifact_digest, "artifact_bundle_", 64),
        "runner_ready_artifact_identity",
        "runner readiness artifact digest is not an exact artifact-10 bundle identity",
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
                value.get("ok").and_then(Value::as_bool) == Some(true),
                "runner_stop_event",
                "runner stop receipt was unsuccessful",
            )?;
            let receipt = value.get("receipt").ok_or_else(|| {
                ServiceFailure::failed("runner_stop_receipt", "runner stop receipt is absent")
            })?;
            let shutdown = receipt.get("shutdown").ok_or_else(|| {
                ServiceFailure::failed("runner_shutdown_receipt", "runner shutdown is absent")
            })?;
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
            "maintained deployment descriptor does not bind the current artifact-10 bundle",
        ));
    }
    if let Some(port) = port {
        object.insert(
            "listen".to_owned(),
            Value::String(format!("127.0.0.1:{port}")),
        );
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

fn parse_host_port(bytes: &[u8]) -> Result<u16, ServiceFailure> {
    let output = std::str::from_utf8(bytes)
        .map_err(|_| ServiceFailure::failed("postgres_port", "Docker port output was not UTF-8"))?;
    output
        .trim()
        .rsplit_once(':')
        .and_then(|(_, port)| port.parse::<u16>().ok())
        .filter(|port| *port != 0)
        .ok_or_else(|| {
            ServiceFailure::failed("postgres_port", "Docker did not report a valid host port")
        })
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

fn unique_container_name() -> Result<String, DevError> {
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
    fn postgres_acceptance_image_is_immutable() {
        let digest = POSTGRES_IMAGE
            .strip_prefix("postgres@sha256:")
            .expect("PostgreSQL image uses a digest reference");
        assert_eq!(digest.len(), 64);
        assert!(
            digest
                .bytes()
                .all(|byte| { byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte) })
        );
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
            br#"{"contract_version":1,"ok":true,"event":"ready","deployment":{"artifact_digest":"artifact_bundle_0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","target":"serve","runner":"http","listen":"127.0.0.1:1","secret_names":["bootstrap-token","database-url"]}}"#,
        )
        .expect("parse ready event");
        assert_eq!(ready.runner, "http");
        assert_eq!(ready.secret_names.len(), 2);
        let stopped = parse_stopped_event(
            br#"{"contract_version":1,"ok":true,"event":"ready"}
{"contract_version":1,"ok":true,"event":"stopped","receipt":{"productive_iterations":3,"shutdown":{"admission_stopped":true,"remaining_tasks":0,"cleanup_failures":[]}}}
"#,
        )
        .expect("parse stopped event");
        assert!(stopped.clean());
        assert_eq!(stopped.productive_iterations, Some(3));
    }

    #[test]
    fn secret_values_are_removed_from_commands_and_logs() {
        let secret = b"database-password".to_vec();
        let bytes = redact_bytes(
            b"before database-password after",
            std::slice::from_ref(&secret),
        );
        assert_eq!(bytes, b"before <redacted> after");
        let command = redact_command(
            &["--url=postgresql://database-password@host".to_owned()],
            &[secret],
        );
        assert_eq!(command, ["--url=postgresql://<redacted>@host"]);
    }

    #[test]
    fn container_cleanup_targets_only_the_exact_owned_name() {
        assert_eq!(
            container_stop_command("lkjscript-service-123"),
            ["docker", "stop", "--time", "5", "lkjscript-service-123"]
        );
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
        assert_eq!(before.files, 4);
        assert_eq!(before.bytes, 8 + 7 + 4 + 9);

        std::fs::create_dir(application.join("derived")).expect("derived directory");
        std::fs::write(application.join("derived/cache"), b"disposable").expect("derived fixture");
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
