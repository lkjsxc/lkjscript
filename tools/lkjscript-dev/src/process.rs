use crate::error::DevError;
use crate::evidence::{self, FileProof};
#[cfg(target_os = "linux")]
use rustix::process::{Pid, Signal, kill_process_group};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
#[cfg(target_os = "linux")]
use std::os::unix::process::{CommandExt, ExitStatusExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::{Duration, Instant};

const READ_CHUNK_BYTES: usize = 64 * 1024;
const POLL_INTERVAL: Duration = Duration::from_millis(10);
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
    pub(crate) stdout_limit_bytes: u64,
    pub(crate) stderr_limit_bytes: u64,
    pub(crate) stdout_limit_exhausted: bool,
    pub(crate) stderr_limit_exhausted: bool,
    pub(crate) stdout: FileProof,
    pub(crate) stderr: FileProof,
}

pub(crate) fn run(specification: &ProcessSpec, repository: &Path) -> ProcessObservation {
    let started = Instant::now();
    match run_inner(specification, repository, started) {
        Ok(observation) => observation,
        Err(error) => infrastructure_observation(specification, repository, started, error),
    }
}

#[cfg(target_os = "linux")]
fn run_inner(
    specification: &ProcessSpec,
    repository: &Path,
    started: Instant,
) -> Result<ProcessObservation, DevError> {
    if specification.command.is_empty() {
        return Err(DevError::infrastructure("child command is empty"));
    }
    let stdout_output = prepare_log(&specification.stdout_path)?;
    let stderr_output = prepare_log(&specification.stderr_path)?;
    let mut command = Command::new(&specification.command[0]);
    command
        .args(&specification.command[1..])
        .current_dir(&specification.cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env_clear()
        .process_group(0);
    for (name, value) in &specification.environment {
        command.env(name, value);
    }
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return observation(
                specification,
                repository,
                started,
                ProcessStatus::Unavailable,
                None,
                None,
                Some("command_not_found".to_owned()),
            );
        }
        Err(error) => {
            return Err(DevError::infrastructure(format!(
                "spawn '{}': {error}",
                specification.command[0]
            )));
        }
    };
    let process_group = Pid::from_child(&child);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| DevError::infrastructure("child stdout pipe is unavailable"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| DevError::infrastructure("child stderr pipe is unavailable"))?;
    let stdout_total = Arc::new(AtomicU64::new(0));
    let stderr_total = Arc::new(AtomicU64::new(0));
    let stdout_exhausted = Arc::new(AtomicBool::new(false));
    let stderr_exhausted = Arc::new(AtomicBool::new(false));
    let stdout_reader = spawn_reader(
        stdout,
        stdout_output,
        specification.stdout_path.clone(),
        Arc::clone(&stdout_total),
        Arc::clone(&stdout_exhausted),
        specification.maximum_stdout_bytes,
    );
    let stderr_reader = spawn_reader(
        stderr,
        stderr_output,
        specification.stderr_path.clone(),
        Arc::clone(&stderr_total),
        Arc::clone(&stderr_exhausted),
        specification.maximum_stderr_bytes,
    );

    let mut terminal_reason = None;
    let exit_status = loop {
        if let Some(status) = child.try_wait().map_err(|error| {
            DevError::infrastructure(format!(
                "poll child '{}': {error}",
                specification.command[0]
            ))
        })? {
            break status;
        }
        if stdout_exhausted.load(Ordering::Acquire) || stderr_exhausted.load(Ordering::Acquire) {
            terminal_reason = Some(ProcessStatus::OutputExhausted);
            let _ = kill_process_group(process_group, Signal::KILL);
            break child.wait().map_err(|error| {
                DevError::infrastructure(format!(
                    "wait for output-exhausted child '{}': {error}",
                    specification.command[0]
                ))
            })?;
        }
        if started.elapsed() >= specification.timeout {
            terminal_reason = Some(ProcessStatus::Timeout);
            let _ = kill_process_group(process_group, Signal::KILL);
            break child.wait().map_err(|error| {
                DevError::infrastructure(format!(
                    "wait for timed-out child '{}': {error}",
                    specification.command[0]
                ))
            })?;
        }
        thread::sleep(POLL_INTERVAL);
    };

    join_reader(stdout_reader, "stdout")?;
    join_reader(stderr_reader, "stderr")?;
    let stdout_limit_exhausted = stdout_exhausted.load(Ordering::Acquire);
    let stderr_limit_exhausted = stderr_exhausted.load(Ordering::Acquire);
    let (status, reason) = match terminal_reason {
        Some(ProcessStatus::OutputExhausted) => (
            ProcessStatus::OutputExhausted,
            Some(exhausted_reason(
                stdout_limit_exhausted,
                stderr_limit_exhausted,
            )),
        ),
        Some(ProcessStatus::Timeout) => (ProcessStatus::Timeout, Some("timeout".to_owned())),
        Some(_) => {
            return Err(DevError::infrastructure(
                "invalid terminal child-process state",
            ));
        }
        None if stdout_limit_exhausted || stderr_limit_exhausted => (
            ProcessStatus::OutputExhausted,
            Some(exhausted_reason(
                stdout_limit_exhausted,
                stderr_limit_exhausted,
            )),
        ),
        None if exit_status.success() => (ProcessStatus::Passed, None),
        None if exit_status.signal().is_some() => {
            (ProcessStatus::Signaled, Some("signal".to_owned()))
        }
        None => (ProcessStatus::Failed, Some("nonzero_exit".to_owned())),
    };
    observation(
        specification,
        repository,
        started,
        status,
        exit_status.code(),
        exit_status.signal(),
        reason,
    )
}

#[cfg(not(target_os = "linux"))]
fn run_inner(
    _specification: &ProcessSpec,
    _repository: &Path,
    _started: Instant,
) -> Result<ProcessObservation, DevError> {
    Err(DevError::infrastructure(
        "bounded process execution requires Linux process-group signaling",
    ))
}

fn exhausted_reason(stdout: bool, stderr: bool) -> String {
    match (stdout, stderr) {
        (true, true) => "stdout_and_stderr_limit".to_owned(),
        (true, false) => "stdout_limit".to_owned(),
        (false, true) => "stderr_limit".to_owned(),
        (false, false) => "output_limit".to_owned(),
    }
}

fn spawn_reader<R: Read + Send + 'static>(
    mut input: R,
    mut output: File,
    path: PathBuf,
    total: Arc<AtomicU64>,
    exhausted: Arc<AtomicBool>,
    maximum: u64,
) -> thread::JoinHandle<Result<(), DevError>> {
    thread::spawn(move || {
        let mut buffer = [0_u8; READ_CHUNK_BYTES];
        loop {
            let read = input.read(&mut buffer).map_err(|error| {
                DevError::infrastructure(format!(
                    "read child pipe for '{}': {error}",
                    path.display()
                ))
            })?;
            if read == 0 {
                break;
            }
            let prior = total.fetch_add(read as u64, Ordering::AcqRel);
            let writable = maximum.saturating_sub(prior).min(read as u64) as usize;
            if writable > 0 {
                output.write_all(&buffer[..writable]).map_err(|error| {
                    DevError::infrastructure(format!(
                        "write child log '{}': {error}",
                        path.display()
                    ))
                })?;
            }
            if writable != read {
                exhausted.store(true, Ordering::Release);
            }
        }
        output.sync_all().map_err(|error| {
            DevError::infrastructure(format!(
                "synchronize child log '{}': {error}",
                path.display()
            ))
        })
    })
}

fn join_reader(
    reader: thread::JoinHandle<Result<(), DevError>>,
    stream: &str,
) -> Result<(), DevError> {
    reader
        .join()
        .map_err(|_| DevError::infrastructure(format!("{stream} reader thread panicked")))?
}

fn observation(
    specification: &ProcessSpec,
    repository: &Path,
    started: Instant,
    status: ProcessStatus,
    exit_code: Option<i32>,
    signal: Option<i32>,
    reason: Option<String>,
) -> Result<ProcessObservation, DevError> {
    let stdout_limit_exhausted = reason
        .as_deref()
        .is_some_and(|value| value == "stdout_limit" || value == "stdout_and_stderr_limit");
    let stderr_limit_exhausted = reason
        .as_deref()
        .is_some_and(|value| value == "stderr_limit" || value == "stdout_and_stderr_limit");
    Ok(ProcessObservation {
        status,
        exit_code,
        signal,
        reason,
        elapsed_nanoseconds: duration_nanoseconds(started.elapsed()),
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
        stdout_limit_bytes: specification.maximum_stdout_bytes,
        stderr_limit_bytes: specification.maximum_stderr_bytes,
        stdout_limit_exhausted: false,
        stderr_limit_exhausted: false,
        stdout,
        stderr,
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
}
