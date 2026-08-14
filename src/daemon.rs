use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{RequestId, WorkspaceId};
use crate::interpret;
use crate::persistence::{self, DurableWorkspace};
use crate::protocol::{self, Request, Response};
use crate::query;
use fs2::FileExt;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub const SOCKET_FILE: &str = "lkjscript.sock";
const LOCK_FILE: &str = "lkjscriptd.lock";
const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);

pub fn endpoint_path(state_directory: &Path) -> PathBuf {
    state_directory.join(SOCKET_FILE)
}

pub fn run_foreground(state_directory: &Path) -> Result<()> {
    persistence::ensure_state_directory(state_directory)?;
    let guard = DaemonGuard::acquire(state_directory)?;
    let mut daemon = Daemon::open(state_directory)?;
    let listener = UnixListener::bind(&guard.socket)?;
    fs::set_permissions(&guard.socket, fs::Permissions::from_mode(0o600))?;
    loop {
        let (stream, _) = listener.accept()?;
        let mut stream = DeadlineStream::new(stream, CONNECTION_TIMEOUT);
        match protocol::read_request(&mut stream) {
            Ok(Some((request_id, request))) => {
                let shutdown = matches!(request, Request::Shutdown);
                let (response, fatal_error) = match daemon.handle(request) {
                    Ok(response) => (response, None),
                    Err(error) if error.code == ErrorCode::CommitOutcomeUnknown => {
                        (Response::Error(error.clone()), Some(error))
                    }
                    Err(error) => (Response::Error(error), None),
                };
                let response_written =
                    match protocol::write_response(&mut stream, request_id, &response) {
                        Ok(()) => true,
                        Err(error) if error.code == ErrorCode::PolicyExceeded => {
                            protocol::write_response(
                                &mut stream,
                                request_id,
                                &Response::Error(LkError::new(
                                    ErrorCode::PolicyExceeded,
                                    format!("response could not satisfy IPC policy: {error}"),
                                )),
                            )
                            .is_ok()
                        }
                        Err(_) => false,
                    };
                if let Some(error) = fatal_error {
                    return Err(error);
                }
                if shutdown && response_written && matches!(response, Response::Acknowledged) {
                    break;
                }
            }
            Ok(None) => {}
            Err(error) => {
                let _ = protocol::write_response(
                    &mut stream,
                    RequestId::new(0),
                    &Response::Error(error),
                );
            }
        }
    }
    drop(listener);
    drop(guard);
    Ok(())
}

struct DeadlineStream {
    stream: UnixStream,
    deadline: Instant,
}

impl DeadlineStream {
    fn new(stream: UnixStream, timeout: Duration) -> Self {
        Self {
            stream,
            deadline: Instant::now() + timeout,
        }
    }

    fn remaining(&self) -> std::io::Result<Duration> {
        let remaining = self.deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            Err(std::io::Error::new(
                std::io::ErrorKind::TimedOut,
                "absolute connection deadline expired",
            ))
        } else {
            Ok(remaining)
        }
    }
}

impl Read for DeadlineStream {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        self.stream.set_read_timeout(Some(self.remaining()?))?;
        self.stream.read(buffer)
    }
}

impl Write for DeadlineStream {
    fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.write(buffer)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.stream.set_write_timeout(Some(self.remaining()?))?;
        self.stream.flush()
    }
}

struct Daemon {
    state_directory: PathBuf,
    workspaces: BTreeMap<WorkspaceId, DurableWorkspace>,
}

impl Daemon {
    fn open(state_directory: &Path) -> Result<Self> {
        let mut workspaces = BTreeMap::new();
        for id in persistence::list_workspace_ids(state_directory)? {
            let workspace = DurableWorkspace::open(state_directory, id)?;
            workspaces.insert(id, workspace);
        }
        Ok(Self {
            state_directory: state_directory.to_owned(),
            workspaces,
        })
    }

    fn handle(&mut self, request: Request) -> Result<Response> {
        match request {
            Request::CreateWorkspace => {
                let id = loop {
                    let candidate = WorkspaceId::generate().map_err(|error| {
                        LkError::new(
                            ErrorCode::Io,
                            format!("cannot generate workspace identity: {error}"),
                        )
                    })?;
                    if !self.workspaces.contains_key(&candidate) {
                        break candidate;
                    }
                };
                let workspace = DurableWorkspace::create(&self.state_directory, id)?;
                let summary = query::workspace_summary(workspace.head()?);
                self.workspaces.insert(id, workspace);
                Ok(Response::WorkspaceCreated(summary))
            }
            Request::ApplyTransaction(transaction) => {
                let fingerprint = protocol::transaction_fingerprint(&transaction)?;
                let workspace = self.workspace_mut(transaction.workspace)?;
                let result = workspace.apply(&transaction, fingerprint)?;
                Ok(Response::TransactionApplied(result))
            }
            Request::WorkspaceSummary {
                workspace,
                revision,
            } => {
                let snapshot = self.workspace(workspace)?.snapshot(revision)?;
                Ok(Response::WorkspaceSummary(query::workspace_summary(
                    snapshot,
                )))
            }
            Request::Node {
                workspace,
                revision,
                node,
                expand,
            } => {
                let snapshot = self.workspace(workspace)?.snapshot(revision)?;
                Ok(Response::Node(query::node_view(snapshot, node, expand)?))
            }
            Request::Blockers {
                workspace,
                revision,
            } => {
                let snapshot = self.workspace(workspace)?.snapshot(revision)?;
                Ok(Response::Blockers {
                    workspace,
                    revision,
                    blockers: query::workspace_blockers(snapshot),
                })
            }
            Request::Run {
                workspace,
                revision,
                entry,
            } => {
                let snapshot = self.workspace(workspace)?.snapshot(revision)?;
                Ok(Response::Run(interpret::compile_and_run(snapshot, entry)?))
            }
            Request::Shutdown => Ok(Response::Acknowledged),
        }
    }

    fn workspace(&self, id: WorkspaceId) -> Result<&DurableWorkspace> {
        self.workspaces.get(&id).ok_or_else(|| {
            LkError::new(
                ErrorCode::WorkspaceNotFound,
                "workspace is not open in this daemon",
            )
            .for_workspace(id)
        })
    }

    fn workspace_mut(&mut self, id: WorkspaceId) -> Result<&mut DurableWorkspace> {
        self.workspaces.get_mut(&id).ok_or_else(|| {
            LkError::new(
                ErrorCode::WorkspaceNotFound,
                "workspace is not open in this daemon",
            )
            .for_workspace(id)
        })
    }
}

struct DaemonGuard {
    _lock: File,
    socket: PathBuf,
}

impl DaemonGuard {
    fn acquire(state_directory: &Path) -> Result<Self> {
        let lock_path = state_directory.join(LOCK_FILE);
        if let Ok(metadata) = fs::symlink_metadata(&lock_path)
            && metadata.file_type().is_symlink()
        {
            return Err(LkError::new(
                ErrorCode::Io,
                "daemon lock path must not be a symlink",
            ));
        }
        let mut lock = OpenOptions::new()
            .create(true)
            .read(true)
            .write(true)
            .truncate(false)
            .mode(0o600)
            .open(lock_path)?;
        lock.set_permissions(fs::Permissions::from_mode(0o600))?;
        FileExt::try_lock_exclusive(&lock).map_err(|error| {
            LkError::new(
                ErrorCode::WorkspaceExists,
                format!("another daemon owns this state directory: {error}"),
            )
        })?;
        lock.set_len(0)?;
        use std::io::Write;
        writeln!(lock, "{}", std::process::id())?;
        lock.sync_all()?;
        let socket = endpoint_path(state_directory);
        match fs::symlink_metadata(&socket) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    return Err(LkError::new(
                        ErrorCode::Io,
                        "daemon socket path must not be a symlink",
                    ));
                }
                fs::remove_file(&socket)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        Ok(Self {
            _lock: lock,
            socket,
        })
    }
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.socket);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn connection_deadline_is_absolute_despite_slow_progress() {
        let (reader, mut writer) = UnixStream::pair().expect("Unix stream pair");
        let sender = thread::spawn(move || {
            for byte in 0..20_u8 {
                if writer.write_all(&[byte]).is_err() {
                    break;
                }
                thread::sleep(Duration::from_millis(10));
            }
        });
        let started = Instant::now();
        let mut reader = DeadlineStream::new(reader, Duration::from_millis(35));
        let mut bytes = [0_u8; 20];
        let error = reader
            .read_exact(&mut bytes)
            .expect_err("absolute deadline must expire");
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
        ));
        assert!(started.elapsed() < Duration::from_secs(2));
        drop(reader);
        sender.join().expect("slow sender");
    }
}
