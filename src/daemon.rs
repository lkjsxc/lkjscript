use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{RequestId, WorkspaceId};
use crate::interpret;
use crate::persistence::{self, DurableWorkspace};
use crate::protocol::{Request, Response};
use crate::query;
use crate::{machine, transport};
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
        match transport::read_request_body(&mut stream) {
            Ok(Some(body)) => match machine::decode_request(&body) {
                Ok(envelope) => {
                    let request_id = envelope.request_id;
                    let shutdown = matches!(envelope.request, Request::Shutdown);
                    let (handled, fatal_error) = match daemon.symbol(request_id, envelope.request) {
                        Ok(handled) => (handled, None),
                        Err(error) if error.code == ErrorCode::CommitOutcomeUnknown => {
                            let handled = encode_error_handled(request_id, error.clone())?;
                            (handled, Some(error))
                        }
                        Err(error) => (encode_error_handled(request_id, error)?, None),
                    };
                    let response_written =
                        transport::write_response_body(&mut stream, &handled.bytes).is_ok();
                    if let Some(error) = fatal_error {
                        return Err(error);
                    }
                    if shutdown
                        && response_written
                        && matches!(handled.response, Response::Acknowledged)
                    {
                        break;
                    }
                }
                Err(error) => {
                    let request_id = machine::request_id_hint(&body);
                    let boundary =
                        machine::encode_boundary_error(request_id, error.kind, error.to_string());
                    let _ = transport::write_response_body(&mut stream, &boundary);
                }
            },
            Ok(None) => {}
            Err(error) => {
                let kind = if error.code == ErrorCode::PolicyExceeded {
                    machine::BoundaryErrorKind::InputTooLarge
                } else {
                    machine::BoundaryErrorKind::Transport
                };
                let boundary = machine::encode_boundary_error(None, kind, error.to_string());
                let _ = transport::write_response_body(&mut stream, &boundary);
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

struct HandledResponse {
    response: Response,
    bytes: Vec<u8>,
}

fn encode_handled(request_id: RequestId, response: Response) -> Result<HandledResponse> {
    let bytes = machine::encode_response(request_id, &response, false).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("response could not satisfy JSON boundary policy: {error}"),
        )
    })?;
    Ok(HandledResponse { response, bytes })
}

fn encode_error_handled(request_id: RequestId, error: LkError) -> Result<HandledResponse> {
    encode_handled(request_id, Response::Error(error)).or_else(|_| {
        encode_handled(
            request_id,
            Response::Error(LkError::new(
                ErrorCode::PolicyExceeded,
                "response could not satisfy JSON boundary policy",
            )),
        )
    })
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

    fn symbol(&mut self, request_id: RequestId, request: Request) -> Result<HandledResponse> {
        let response = match request {
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
                let (workspace, handled) =
                    DurableWorkspace::create_preflighted(&self.state_directory, id, |snapshot| {
                        encode_handled(
                            request_id,
                            Response::WorkspaceCreated(query::workspace_summary(snapshot)),
                        )
                    })?;
                self.workspaces.insert(id, workspace);
                return Ok(handled);
            }
            Request::ApplyTransaction(request) => {
                let fingerprint = machine::transaction_fingerprint(&request)?;
                let workspace = self.workspace_mut(request.transaction.workspace)?;
                let (receipt, bytes) =
                    workspace.apply_with_response(&request, fingerprint, request_id)?;
                return Ok(HandledResponse {
                    response: Response::TransactionReceipt(receipt),
                    bytes,
                });
            }
            Request::QueryBatch(batch) => {
                query::validate_batch(&batch)?;
                let workspace = self.workspace(batch.workspace)?;
                let snapshot = workspace.snapshot(batch.revision)?;
                let mut results = Vec::with_capacity(batch.queries.len());
                for item in &batch.queries {
                    let before = match &item.query {
                        query::Query::SemanticDiff { from, .. } => {
                            workspace.snapshot(*from).ok().map(AsRef::as_ref)
                        }
                        _ => None,
                    };
                    let outcome = match query::execute(snapshot, &item.query, before) {
                        Ok(result) => query::QueryOutcome::Success(Box::new(result)),
                        Err(error) => query::QueryOutcome::Error(error),
                    };
                    results.push(query::QueryItemResult {
                        id: item.id,
                        outcome,
                    });
                }
                let result = query::QueryBatchResult {
                    workspace: batch.workspace,
                    revision: batch.revision,
                    results,
                };
                Response::QueryBatchResult(result)
            }
            Request::Run {
                workspace,
                revision,
                entry,
                arguments,
                policy,
            } => {
                let snapshot = self.workspace(workspace)?.snapshot(revision)?;
                Response::Run(interpret::compile_and_run(
                    snapshot, entry, &arguments, policy,
                )?)
            }
            Request::Shutdown => Response::Acknowledged,
            Request::DescribeSchema(request) => {
                let result = crate::machine::describe_schema(&request)
                    .map_err(|message| LkError::new(ErrorCode::ProtocolMalformed, message))?;
                Response::DescribeSchema(Box::new(result))
            }
        };
        encode_handled(request_id, response)
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
    use crate::ids::{DraftSymbol, QueryId, RequestId, Revision};
    use crate::query::{PageRequest, Query, QueryBatchRequest, QueryItem};
    use crate::transaction::{
        ApplyTransactionRequest, NodeTarget, Transaction, TransactionMode, TransactionOp,
        TransactionResponseSpec,
    };
    use std::thread;

    #[test]
    fn trailing_request_bytes_reject_before_daemon_dispatch() {
        let temporary = tempfile::tempdir().expect("temporary state directory");
        persistence::ensure_state_directory(temporary.path()).expect("state directory");
        let daemon = Daemon::open(temporary.path()).expect("daemon");
        let body = machine::encode_request(RequestId::new(1), &Request::CreateWorkspace)
            .expect("request JSON");
        let mut bytes = Vec::new();
        transport::write_request_body(&mut bytes, &body).expect("request frame");
        bytes.push(0xff);
        assert_eq!(
            transport::read_request_body(&mut bytes.as_slice())
                .expect_err("trailing connection byte")
                .code,
            ErrorCode::ProtocolMalformed
        );
        assert!(daemon.workspaces.is_empty());
    }

    #[test]
    fn workspace_creation_response_preflight_precedes_publication() {
        let temporary = tempfile::tempdir().expect("temporary state directory");
        persistence::ensure_state_directory(temporary.path()).expect("state directory");
        let mut daemon = Daemon::open(temporary.path()).expect("daemon");
        let Err(error) = daemon.symbol(RequestId::new(0), Request::CreateWorkspace) else {
            panic!("invalid response correlation must fail before creation");
        };
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert!(daemon.workspaces.is_empty());
        assert!(
            persistence::list_workspace_ids(temporary.path())
                .expect("workspace listing")
                .is_empty()
        );
    }

    #[test]
    fn response_between_request_and_response_bounds_is_accepted_without_mutation() {
        let temporary = tempfile::tempdir().expect("temporary state directory");
        persistence::ensure_state_directory(temporary.path()).expect("state directory");
        let mut daemon = Daemon::open(temporary.path()).expect("daemon");
        let Response::WorkspaceCreated(initial) = daemon
            .symbol(RequestId::new(1), Request::CreateWorkspace)
            .expect("create")
            .response
        else {
            panic!("create response")
        };
        let workspace = initial.workspace;
        let create_operations = (1..=9)
            .map(|symbol| TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(symbol),
                name: format!("p{symbol}"),
            })
            .collect();
        let create = ApplyTransactionRequest {
            transaction: Transaction {
                workspace,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: create_operations,
            },
            response: TransactionResponseSpec {
                return_symbols: (1..=9).map(DraftSymbol::generated).collect(),
            },
        };
        let Response::TransactionReceipt(created) = daemon
            .symbol(RequestId::new(2), Request::ApplyTransaction(create))
            .expect("create packages")
            .response
        else {
            panic!("create response")
        };
        let rename_operations = created
            .returned_bindings
            .iter()
            .enumerate()
            .map(|(index, (_, node))| TransactionOp::RenameNode {
                node: NodeTarget::Existing(*node),
                name: format!("{}-{index}", "x".repeat(1024 * 1024 - 2)),
            })
            .collect();
        let rename = ApplyTransactionRequest {
            transaction: Transaction {
                workspace,
                base_revision: Revision::new(1),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: rename_operations,
            },
            response: TransactionResponseSpec::default(),
        };
        daemon
            .symbol(RequestId::new(3), Request::ApplyTransaction(rename))
            .expect("publish large renames");
        let head_path = persistence::workspace_directory(temporary.path(), workspace).join("HEAD");
        let head_before = fs::read(&head_path).expect("head before query");
        let hash_before = daemon
            .workspace(workspace)
            .expect("workspace")
            .head()
            .expect("head")
            .hash();
        let query = Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision: Revision::new(2),
            queries: vec![QueryItem {
                id: QueryId::new(1),
                query: Query::SemanticDiff {
                    from: Revision::new(1),
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
            }],
        });
        let handled = daemon
            .symbol(RequestId::new(4), query)
            .expect("response under 32 MiB must be accepted");
        assert!(handled.bytes.len() > transport::MAXIMUM_REQUEST_FRAME_BYTES);
        assert!(handled.bytes.len() <= transport::MAXIMUM_RESPONSE_FRAME_BYTES);
        assert_eq!(fs::read(&head_path).expect("head after query"), head_before);
        let head = daemon
            .workspace(workspace)
            .expect("workspace")
            .head()
            .expect("head");
        assert_eq!(head.revision(), Revision::new(2));
        assert_eq!(head.hash(), hash_before);
    }

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
