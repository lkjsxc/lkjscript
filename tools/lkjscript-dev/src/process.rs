#[cfg(target_os = "linux")]
mod descendants;
mod launch;
#[cfg(target_os = "linux")]
mod output;
#[cfg(target_os = "linux")]
mod owned;
#[cfg(target_os = "linux")]
mod supervisor;

use crate::error::DevError;
use crate::evidence::{self, FileProof};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

const READ_CHUNK_BYTES: usize = 64 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
const CONTROL_NONE: u8 = 0;
const CONTROL_INTERRUPT: u8 = 1;
const CONTROL_TERMINATE: u8 = 2;
const CONTROL_KILL: u8 = 3;
const APPROVED_ENVIRONMENT: &[&str] = &[
    "AR",
    "CARGO_BUILD_JOBS",
    "CARGO_BUILD_TARGET",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_HOME",
    "CARGO_TARGET_DIR",
    "CC",
    "CFLAGS",
    "CXX",
    "CXXFLAGS",
    "DOCKER_CONFIG",
    "DOCKER_CONTEXT",
    "DOCKER_HOST",
    "HOME",
    "LANG",
    "LC_ALL",
    "PATH",
    "PKG_CONFIG_PATH",
    "RANLIB",
    "RUSTC",
    "RUSTC_WRAPPER",
    "RUSTFLAGS",
    "RUSTUP_HOME",
    "RUSTUP_TOOLCHAIN",
    "SOURCE_DATE_EPOCH",
    "TMPDIR",
    "TZ",
];

pub(crate) fn environment() -> BTreeMap<String, String> {
    let mut environment = BTreeMap::new();
    for name in APPROVED_ENVIRONMENT {
        if let Ok(value) = env::var(name) {
            environment.insert((*name).to_owned(), value);
        }
    }
    environment.insert("CARGO_NET_OFFLINE".to_owned(), "true".to_owned());
    environment
}

#[derive(Clone, Debug)]
pub(crate) struct ProcessSpec {
    pub(crate) command: Vec<String>,
    pub(crate) cwd: PathBuf,
    pub(crate) environment: BTreeMap<String, String>,
    pub(crate) timeout: Duration,
    pub(crate) maximum_stdout_bytes: u64,
    pub(crate) maximum_stderr_bytes: u64,
    pub(crate) stdout_path: PathBuf,
    pub(crate) stderr_path: PathBuf,
    pub(crate) unavailable_exit_code: Option<i32>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ProcessControl {
    requested: Arc<AtomicU8>,
}

impl ProcessControl {
    pub(crate) fn interrupt(&self) {
        self.requested
            .fetch_max(CONTROL_INTERRUPT, Ordering::AcqRel);
    }

    pub(crate) fn kill(&self) {
        self.requested.fetch_max(CONTROL_KILL, Ordering::AcqRel);
    }

    pub(crate) fn terminate(&self) {
        self.requested
            .fetch_max(CONTROL_TERMINATE, Ordering::AcqRel);
    }

    fn requested(&self) -> u8 {
        self.requested.load(Ordering::Acquire)
    }

    pub(crate) fn cancelled(&self) -> bool {
        self.requested() != CONTROL_NONE
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProcessStatus {
    Passed,
    Failed,
    Unavailable,
    Timeout,
    OutputExhausted,
    Signaled,
    InfrastructureFailure,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ProcessObservation {
    pub(crate) status: ProcessStatus,
    pub(crate) exit_code: Option<i32>,
    pub(crate) signal: Option<i32>,
    pub(crate) reason: Option<String>,
    pub(crate) elapsed_nanoseconds: u64,
    #[serde(default)]
    pub(crate) cpu_nanoseconds: Option<u64>,
    #[serde(default)]
    pub(crate) peak_rss_kib: Option<u64>,
    pub(crate) stdout_limit_bytes: u64,
    pub(crate) stderr_limit_bytes: u64,
    pub(crate) stdout_limit_exhausted: bool,
    pub(crate) stderr_limit_exhausted: bool,
    pub(crate) stdout: FileProof,
    pub(crate) stderr: FileProof,
}

#[derive(Clone, Copy, Debug, Default)]
struct ProcessResources {
    cpu_nanoseconds: Option<u64>,
    peak_rss_kib: Option<u64>,
}

#[derive(Debug)]
struct ProcessCompletion {
    status: ProcessStatus,
    exit_code: Option<i32>,
    signal: Option<i32>,
    reason: Option<String>,
}

#[derive(Default)]
struct ProcessOptions<'a> {
    stdin_file: Option<(&'a Path, u64)>,
    close_stdout: bool,
    program: launch::Program<'a>,
}

pub(crate) fn run(specification: &ProcessSpec, repository: &Path) -> ProcessObservation {
    run_configured(specification, repository, None, None)
}

pub(crate) fn run_selected(
    specification: &ProcessSpec,
    repository: &Path,
    program: Option<&Path>,
) -> ProcessObservation {
    run_configured_output(
        specification,
        repository,
        None,
        ProcessOptions {
            program: launch::Program::Selected(program),
            ..ProcessOptions::default()
        },
    )
}

pub(crate) fn run_closed_stdout(
    specification: &ProcessSpec,
    repository: &Path,
) -> ProcessObservation {
    run_configured_output(
        specification,
        repository,
        None,
        ProcessOptions {
            close_stdout: true,
            ..ProcessOptions::default()
        },
    )
}

pub(crate) fn run_controlled(
    specification: &ProcessSpec,
    repository: &Path,
    control: &ProcessControl,
) -> ProcessObservation {
    run_configured(specification, repository, Some(control), None)
}

#[cfg(test)]
pub(crate) fn run_with_stdin_file(
    specification: &ProcessSpec,
    repository: &Path,
    stdin_path: &Path,
    maximum_stdin_bytes: u64,
) -> ProcessObservation {
    run_configured(
        specification,
        repository,
        None,
        Some((stdin_path, maximum_stdin_bytes)),
    )
}

pub(crate) fn run_supervised(
    specification: &ProcessSpec,
    repository: &Path,
    control: Option<&ProcessControl>,
) -> ProcessObservation {
    run_configured(specification, repository, control, None)
}

fn run_configured(
    specification: &ProcessSpec,
    repository: &Path,
    control: Option<&ProcessControl>,
    stdin_file: Option<(&Path, u64)>,
) -> ProcessObservation {
    run_configured_output(
        specification,
        repository,
        control,
        ProcessOptions {
            stdin_file,
            ..ProcessOptions::default()
        },
    )
}

fn run_configured_output(
    specification: &ProcessSpec,
    repository: &Path,
    control: Option<&ProcessControl>,
    options: ProcessOptions<'_>,
) -> ProcessObservation {
    let started = Instant::now();
    match run_inner(specification, repository, started, control, options) {
        Ok(observation) => observation,
        Err(error) => infrastructure_observation(specification, repository, started, error),
    }
}

#[cfg(target_os = "linux")]
fn run_inner(
    specification: &ProcessSpec,
    repository: &Path,
    started: Instant,
    control: Option<&ProcessControl>,
    options: ProcessOptions<'_>,
) -> Result<ProcessObservation, DevError> {
    if specification.command.is_empty() {
        return Err(DevError::infrastructure("child command is empty"));
    }
    let (stdout, stdout_pipe) = output::Output::new(
        &specification.stdout_path,
        specification.maximum_stdout_bytes,
        options.close_stdout,
    )?;
    let (stderr, stderr_pipe) = output::Output::new(
        &specification.stderr_path,
        specification.maximum_stderr_bytes,
        false,
    )?;
    let stdin = match options.stdin_file {
        Some((path, maximum)) => bounded_stdin(path, maximum)?,
        None => Stdio::null(),
    };
    let child = match launch::spawn(
        specification,
        stdin,
        stdout_pipe,
        stderr_pipe,
        options.program,
    ) {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return observation(
                specification,
                repository,
                started,
                ProcessCompletion {
                    status: ProcessStatus::Unavailable,
                    exit_code: None,
                    signal: None,
                    reason: Some("command_not_found".to_owned()),
                },
                ProcessResources::default(),
            );
        }
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "spawn '{}': {error}",
                specification.command[0]
            )));
        }
    };
    let finished = supervisor::run(child, stdout, stderr, specification, started, control);
    let mut result = observation(
        specification,
        repository,
        started,
        finished.completion,
        finished.resources,
    )?;
    // Exhaustion remains true even when a later cleanup/I/O failure owns the reason.
    result.stdout_limit_exhausted = finished.stdout_exhausted;
    result.stderr_limit_exhausted = finished.stderr_exhausted;
    Ok(result)
}

#[cfg(not(target_os = "linux"))]
fn run_inner(
    _specification: &ProcessSpec,
    _repository: &Path,
    _started: Instant,
    _control: Option<&ProcessControl>,
    _options: ProcessOptions<'_>,
) -> Result<ProcessObservation, DevError> {
    Err(DevError::infrastructure(
        "bounded process execution requires Linux process-group signaling",
    ))
}

fn bounded_stdin(path: &Path, maximum: u64) -> Result<Stdio, DevError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect child stdin '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum {
        return Err(DevError::infrastructure(format!(
            "child stdin '{}' is unsafe or exceeds {maximum} bytes",
            path.display()
        )));
    }
    File::open(path).map(Stdio::from).map_err(|error| {
        DevError::infrastructure(format!("open child stdin '{}': {error}", path.display()))
    })
}

fn exhausted_reason(stdout: bool, stderr: bool) -> String {
    match (stdout, stderr) {
        (true, true) => "stdout_and_stderr_limit".to_owned(),
        (true, false) => "stdout_limit".to_owned(),
        (false, true) => "stderr_limit".to_owned(),
        (false, false) => "output_limit".to_owned(),
    }
}

fn observation(
    specification: &ProcessSpec,
    repository: &Path,
    started: Instant,
    completion: ProcessCompletion,
    resources: ProcessResources,
) -> Result<ProcessObservation, DevError> {
    let stdout_limit_exhausted = completion
        .reason
        .as_deref()
        .is_some_and(|value| value == "stdout_limit" || value == "stdout_and_stderr_limit");
    let stderr_limit_exhausted = completion
        .reason
        .as_deref()
        .is_some_and(|value| value == "stderr_limit" || value == "stdout_and_stderr_limit");
    Ok(ProcessObservation {
        status: completion.status,
        exit_code: completion.exit_code,
        signal: completion.signal,
        reason: completion.reason,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        cpu_nanoseconds: resources.cpu_nanoseconds,
        peak_rss_kib: resources.peak_rss_kib,
        stdout_limit_bytes: specification.maximum_stdout_bytes,
        stderr_limit_bytes: specification.maximum_stderr_bytes,
        stdout_limit_exhausted,
        stderr_limit_exhausted,
        stdout: evidence::proof(
            &specification.stdout_path,
            evidence::relative(repository, &specification.stdout_path),
        )?,
        stderr: evidence::proof(
            &specification.stderr_path,
            evidence::relative(repository, &specification.stderr_path),
        )?,
    })
}

fn infrastructure_observation(
    specification: &ProcessSpec,
    repository: &Path,
    started: Instant,
    error: DevError,
) -> ProcessObservation {
    let _ = ensure_log(&specification.stdout_path);
    let _ = ensure_log(&specification.stderr_path);
    let stdout = evidence::proof(
        &specification.stdout_path,
        evidence::relative(repository, &specification.stdout_path),
    )
    .unwrap_or_else(|_| missing_proof(repository, &specification.stdout_path));
    let stderr = evidence::proof(
        &specification.stderr_path,
        evidence::relative(repository, &specification.stderr_path),
    )
    .unwrap_or_else(|_| missing_proof(repository, &specification.stderr_path));
    ProcessObservation {
        status: ProcessStatus::InfrastructureFailure,
        exit_code: None,
        signal: None,
        reason: Some(format!("{}:{}", error.kind(), error.message())),
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
        cpu_nanoseconds: None,
        peak_rss_kib: None,
        stdout_limit_bytes: specification.maximum_stdout_bytes,
        stderr_limit_bytes: specification.maximum_stderr_bytes,
        stdout_limit_exhausted: false,
        stderr_limit_exhausted: false,
        stdout,
        stderr,
    }
}

#[cfg(target_os = "linux")]
fn sample_linux_process(process_id: u32, resources: &mut ProcessResources) {
    let process_root = PathBuf::from(format!("/proc/{process_id}"));
    if let Ok(status) = std::fs::read_to_string(process_root.join("status")) {
        let resident = status.lines().find_map(|line| {
            line.strip_prefix("VmHWM:")
                .or_else(|| line.strip_prefix("VmRSS:"))
                .and_then(|value| value.split_whitespace().next())
                .and_then(|value| value.parse::<u64>().ok())
        });
        if let Some(resident) = resident {
            resources.peak_rss_kib = Some(
                resources
                    .peak_rss_kib
                    .map_or(resident, |current| current.max(resident)),
            );
        }
    }
    if let Ok(stat) = std::fs::read_to_string(process_root.join("stat"))
        && let Some(command_end) = stat.rfind(')')
    {
        let mut fields = stat[command_end + 1..].split_whitespace();
        let user_ticks = fields.nth(11).and_then(|value| value.parse::<u64>().ok());
        let system_ticks = fields.next().and_then(|value| value.parse::<u64>().ok());
        if let (Some(user_ticks), Some(system_ticks)) = (user_ticks, system_ticks) {
            let ticks = u128::from(user_ticks.saturating_add(system_ticks));
            let frequency = u128::from(rustix::param::clock_ticks_per_second());
            if let Some(nanoseconds) = ticks.saturating_mul(1_000_000_000).checked_div(frequency)
                && let Ok(nanoseconds) = u64::try_from(nanoseconds)
            {
                resources.cpu_nanoseconds = Some(
                    resources
                        .cpu_nanoseconds
                        .map_or(nanoseconds, |current| current.max(nanoseconds)),
                );
            }
        }
    }
}

fn missing_proof(repository: &Path, path: &Path) -> FileProof {
    FileProof {
        path: evidence::relative(repository, path),
        kind: evidence::FileKind::Missing,
        mode: None,
        bytes: None,
        digest: None,
        link_target: None,
    }
}

fn prepare_log(path: &Path) -> Result<File, DevError> {
    let parent = path.parent().ok_or_else(|| {
        DevError::infrastructure(format!("child log '{}' has no parent", path.display()))
    })?;
    std::fs::create_dir_all(parent).map_err(|error| {
        DevError::infrastructure(format!(
            "create child log directory '{}': {error}",
            parent.display()
        ))
    })?;
    let file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| {
            DevError::infrastructure(format!("create child log '{}': {error}", path.display()))
        })?;
    file.sync_all().map_err(|error| {
        DevError::infrastructure(format!(
            "synchronize child log '{}': {error}",
            path.display()
        ))
    })?;
    Ok(file)
}

fn ensure_log(path: &Path) -> Result<(), DevError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.file_type().is_symlink() => Ok(()),
        Ok(_) => Err(DevError::infrastructure(format!(
            "child log '{}' is not a regular file",
            path.display()
        ))),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => prepare_log(path).map(|_| ()),
        Err(error) => Err(DevError::infrastructure(format!(
            "inspect child log '{}': {error}",
            path.display()
        ))),
    }
}

pub(crate) fn excerpt(path: &Path, maximum: usize) -> Result<String, DevError> {
    if maximum == 0 {
        return Ok(String::new());
    }
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!("inspect failure log '{}': {error}", path.display()))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(DevError::infrastructure(format!(
            "failure log '{}' is not a regular file",
            path.display()
        )));
    }
    let mut file = File::open(path).map_err(|error| {
        DevError::infrastructure(format!("open failure log '{}': {error}", path.display()))
    })?;
    let selected = if metadata.len() <= maximum as u64 {
        let mut bytes = Vec::with_capacity(maximum.saturating_add(1));
        Read::by_ref(&mut file)
            .take(maximum.saturating_add(1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|error| {
                DevError::infrastructure(format!("read failure log '{}': {error}", path.display()))
            })?;
        bytes.truncate(maximum);
        bytes
    } else {
        let half = maximum / 2;
        let tail = maximum.saturating_sub(half);
        let mut head_bytes = vec![0_u8; half];
        file.read_exact(&mut head_bytes).map_err(|error| {
            DevError::infrastructure(format!(
                "read failure log head '{}': {error}",
                path.display()
            ))
        })?;
        let tail_offset = i64::try_from(tail)
            .map_err(|_| DevError::infrastructure("failure excerpt bound exceeds i64"))?;
        file.seek(SeekFrom::End(-tail_offset)).map_err(|error| {
            DevError::infrastructure(format!("seek failure log '{}': {error}", path.display()))
        })?;
        let mut tail_bytes = vec![0_u8; tail];
        file.read_exact(&mut tail_bytes).map_err(|error| {
            DevError::infrastructure(format!(
                "read failure log tail '{}': {error}",
                path.display()
            ))
        })?;
        let mut selected = Vec::with_capacity(maximum.saturating_add(32));
        selected.extend_from_slice(&head_bytes);
        selected.extend_from_slice(b"\n... bounded excerpt ...\n");
        selected.extend_from_slice(&tail_bytes);
        selected
    };
    Ok(String::from_utf8_lossy(&selected).into_owned())
}

pub(crate) fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, DevError> {
    let metadata = std::fs::symlink_metadata(path).map_err(|error| {
        DevError::infrastructure(format!(
            "inspect bounded file '{}': {error}",
            path.display()
        ))
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() > maximum {
        return Err(DevError::infrastructure(format!(
            "file '{}' is unsafe or exceeds {maximum} bytes",
            path.display()
        )));
    }
    let capacity = usize::try_from(metadata.len())
        .map_err(|_| DevError::infrastructure("bounded file length does not fit this platform"))?;
    let mut bytes = Vec::with_capacity(capacity);
    File::open(path)
        .and_then(|file| file.take(maximum.saturating_add(1)).read_to_end(&mut bytes))
        .map_err(|error| {
            DevError::infrastructure(format!("read bounded file '{}': {error}", path.display()))
        })?;
    if bytes.len() as u64 != metadata.len() {
        return Err(DevError::infrastructure(format!(
            "file '{}' changed during bounded read",
            path.display()
        )));
    }
    Ok(bytes)
}

fn duration_nanoseconds(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specification(
        temporary: &tempfile::TempDir,
        command: Vec<String>,
        timeout: Duration,
        maximum: u64,
    ) -> ProcessSpec {
        ProcessSpec {
            command,
            cwd: temporary.path().to_path_buf(),
            environment: BTreeMap::new(),
            timeout,
            maximum_stdout_bytes: maximum,
            maximum_stderr_bytes: maximum,
            stdout_path: temporary.path().join("stdout.log"),
            stderr_path: temporary.path().join("stderr.log"),
            unavailable_exit_code: None,
        }
    }

    #[test]
    fn child_failure_is_typed_and_retained() {
        let temporary = tempfile::tempdir().expect("temporary process directory");
        let result = run(
            &specification(
                &temporary,
                vec!["/bin/false".to_owned()],
                Duration::from_secs(2),
                1024,
            ),
            temporary.path(),
        );
        assert_eq!(result.status, ProcessStatus::Failed);
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(result.stdout.kind, evidence::FileKind::File);
    }

    #[test]
    fn configured_exit_is_classified_as_unavailable() {
        let temporary = tempfile::tempdir().expect("temporary process directory");
        let mut child = specification(
            &temporary,
            vec!["/bin/false".to_owned()],
            Duration::from_secs(2),
            1024,
        );
        child.unavailable_exit_code = Some(1);
        let result = run(&child, temporary.path());
        assert_eq!(result.status, ProcessStatus::Unavailable);
        assert_eq!(result.exit_code, Some(1));
        assert_eq!(
            result.reason.as_deref(),
            Some("configured_unavailable_exit")
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn child_observation_samples_cpu_and_peak_resident_memory() {
        let temporary = tempfile::tempdir().expect("temporary process directory");
        let result = run(
            &specification(
                &temporary,
                vec!["/bin/sleep".to_owned(), "0.03".to_owned()],
                Duration::from_secs(2),
                1024,
            ),
            temporary.path(),
        );
        assert_eq!(result.status, ProcessStatus::Passed);
        assert!(result.cpu_nanoseconds.is_some());
        assert!(result.peak_rss_kib.is_some_and(|value| value > 0));
    }

    #[test]
    fn child_timeout_kills_the_process_group() {
        let temporary = tempfile::tempdir().expect("temporary process directory");
        let result = run(
            &specification(
                &temporary,
                vec!["/bin/sleep".to_owned(), "5".to_owned()],
                Duration::from_millis(30),
                1024,
            ),
            temporary.path(),
        );
        assert_eq!(result.status, ProcessStatus::Timeout);
    }

    #[test]
    fn child_output_exhaustion_is_bounded() {
        let temporary = tempfile::tempdir().expect("temporary process directory");
        let result = run(
            &specification(
                &temporary,
                vec!["/usr/bin/yes".to_owned()],
                Duration::from_secs(2),
                1024,
            ),
            temporary.path(),
        );
        assert_eq!(result.status, ProcessStatus::OutputExhausted);
        assert!(result.stdout.bytes.unwrap_or(0) <= 1024);
        assert!(result.stderr.bytes.unwrap_or(0) <= 1024);
        assert!(result.stdout_limit_exhausted);
        assert!(!result.stderr_limit_exhausted);
    }

    #[test]
    fn diagnostic_excerpt_does_not_read_a_large_log_whole() {
        let temporary = tempfile::tempdir().expect("temporary excerpt directory");
        let path = temporary.path().join("large.log");
        std::fs::write(&path, vec![b'x'; 1024 * 1024]).expect("write large log");
        let value = excerpt(&path, 64).expect("bounded excerpt");
        assert!(value.len() <= 96);
        assert!(value.contains("bounded excerpt"));
    }

    #[test]
    fn controlled_child_handles_graceful_interrupt() {
        let temporary = tempfile::tempdir().expect("temporary controlled process directory");
        let specification = specification(
            &temporary,
            vec![
                "/bin/sh".to_owned(),
                "-c".to_owned(),
                "trap 'exit 0' INT; echo ready; while :; do sleep 1; done".to_owned(),
            ],
            Duration::from_secs(5),
            1024,
        );
        let stdout_path = specification.stdout_path.clone();
        let repository = temporary.path().to_path_buf();
        let control = ProcessControl::default();
        let child_control = control.clone();
        let child =
            std::thread::spawn(move || run_controlled(&specification, &repository, &child_control));
        let started = Instant::now();
        while std::fs::metadata(&stdout_path).map_or(true, |metadata| metadata.len() == 0) {
            assert!(started.elapsed() < Duration::from_secs(2));
            std::thread::sleep(Duration::from_millis(10));
        }
        control.interrupt();
        let result = child.join().expect("join controlled process");
        assert_eq!(result.status, ProcessStatus::Passed);
    }

    #[test]
    fn child_stdin_file_is_bounded_and_streamed() {
        let temporary = tempfile::tempdir().expect("temporary stdin process directory");
        let stdin_path = temporary.path().join("stdin.bin");
        std::fs::write(&stdin_path, b"bounded input").expect("write child stdin");
        let result = run_with_stdin_file(
            &specification(
                &temporary,
                vec!["/bin/cat".to_owned()],
                Duration::from_secs(2),
                1024,
            ),
            temporary.path(),
            &stdin_path,
            1024,
        );
        assert_eq!(result.status, ProcessStatus::Passed);
        assert_eq!(
            std::fs::read(temporary.path().join("stdout.log")).expect("read child stdout"),
            b"bounded input"
        );
    }
}

#[cfg(all(test, target_os = "linux"))]
mod lifecycle_tests;
