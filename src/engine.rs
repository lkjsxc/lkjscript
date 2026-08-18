//! Topology-neutral semantic engine and single-writer authority.

use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{RequestId, WorkspaceId};
use crate::interpret;
use crate::persistence::{self, DurableWorkspace};
use crate::protocol::{Request, Response};
use crate::query;
use crate::release::{PreparedRelease, ReleaseBuildRequest};
use crate::{machine, transport};
use fs2::FileExt;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

const AUTHORITY_LOCK_FILE: &str = "lkjscript.engine.lock";

pub struct Engine {
    state_directory: PathBuf,
    workspaces: BTreeMap<WorkspaceId, DurableWorkspace>,
    _authority_lock: File,
    outcome_unknown: bool,
}

#[derive(Debug)]
pub(crate) struct HandledResponse {
    pub(crate) response: Response,
    pub(crate) bytes: Vec<u8>,
}

impl Engine {
    pub fn open(state_directory: &Path) -> Result<Self> {
        persistence::ensure_state_directory(state_directory)?;
        let authority_lock = acquire_authority_lock(state_directory)?;
        let mut workspaces = BTreeMap::new();
        for id in persistence::list_workspace_ids(state_directory)? {
            let workspace = DurableWorkspace::open(state_directory, id)?;
            workspaces.insert(id, workspace);
        }
        Ok(Self {
            state_directory: state_directory.to_owned(),
            workspaces,
            _authority_lock: authority_lock,
            outcome_unknown: false,
        })
    }

    pub fn request(&mut self, request_id: RequestId, request: Request) -> Result<Response> {
        match self.handle(request_id, request) {
            Ok(handled) => Ok(handled.response),
            Err(error) if error.code == ErrorCode::CommitOutcomeUnknown => Err(error),
            Err(error) => Ok(Response::Error(error)),
        }
    }

    /// Prepares one canonical reusable release from an exact immutable workspace revision and an
    /// explicitly supplied exact dependency closure. No mutable resolver or workspace HEAD is
    /// consulted and the returned bytes have not yet been published.
    pub fn prepare_release(
        &self,
        request: &ReleaseBuildRequest,
        supplied_dependency_bytes: &[Vec<u8>],
    ) -> Result<PreparedRelease> {
        let snapshot = self
            .workspace(request.workspace)?
            .snapshot(request.revision)?;
        crate::release::prepare(snapshot, request, supplied_dependency_bytes)
    }

    pub(crate) fn handle(
        &mut self,
        request_id: RequestId,
        request: Request,
    ) -> Result<HandledResponse> {
        if self.outcome_unknown {
            return Err(LkError::new(
                ErrorCode::CommitOutcomeUnknown,
                "semantic engine stopped after an unknown publication outcome",
            ));
        }
        let handled = self.handle_usable(request_id, request);
        if handled
            .as_ref()
            .is_err_and(|error| error.code == ErrorCode::CommitOutcomeUnknown)
        {
            self.outcome_unknown = true;
        }
        handled
    }

    fn handle_usable(
        &mut self,
        request_id: RequestId,
        request: Request,
    ) -> Result<HandledResponse> {
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
                Response::QueryBatchResult(query::QueryBatchResult {
                    workspace: batch.workspace,
                    revision: batch.revision,
                    results,
                })
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
                let result = machine::describe_schema(&request)
                    .map_err(|message| LkError::new(ErrorCode::ProtocolMalformed, message))?;
                Response::DescribeSchema(Box::new(result))
            }
        };
        encode_handled(request_id, response)
    }

    pub(crate) fn workspace(&self, id: WorkspaceId) -> Result<&DurableWorkspace> {
        self.workspaces.get(&id).ok_or_else(|| {
            LkError::new(
                ErrorCode::WorkspaceNotFound,
                "workspace is not open in this semantic engine",
            )
            .for_workspace(id)
        })
    }

    fn workspace_mut(&mut self, id: WorkspaceId) -> Result<&mut DurableWorkspace> {
        self.workspaces.get_mut(&id).ok_or_else(|| {
            LkError::new(
                ErrorCode::WorkspaceNotFound,
                "workspace is not open in this semantic engine",
            )
            .for_workspace(id)
        })
    }
}

pub(crate) fn encode_handled(request_id: RequestId, response: Response) -> Result<HandledResponse> {
    let bytes = machine::encode_response(request_id, &response, false).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("response could not satisfy JSON boundary policy: {error}"),
        )
    })?;
    if bytes.len() > transport::MAXIMUM_RESPONSE_FRAME_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "response exceeds the transport response policy",
        ));
    }
    Ok(HandledResponse { response, bytes })
}

pub(crate) fn encode_error_handled(
    request_id: RequestId,
    error: LkError,
) -> Result<HandledResponse> {
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

fn acquire_authority_lock(state_directory: &Path) -> Result<File> {
    let path = state_directory.join(AUTHORITY_LOCK_FILE);
    if let Ok(metadata) = fs::symlink_metadata(&path)
        && (metadata.file_type().is_symlink() || !metadata.is_file())
    {
        return Err(LkError::new(
            ErrorCode::Io,
            "semantic engine lock path must be a regular file",
        ));
    }
    let lock = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .mode(0o600)
        .open(path)?;
    lock.set_permissions(fs::Permissions::from_mode(0o600))?;
    FileExt::try_lock_exclusive(&lock).map_err(|error| {
        LkError::new(
            ErrorCode::AuthorityBusy,
            format!("another engine owns this state directory: {error}"),
        )
    })?;
    Ok(lock)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_engine_lock_owns_the_state_directory() {
        let temporary = tempfile::tempdir().expect("state directory");
        let engine = Engine::open(temporary.path()).expect("first engine");
        let Err(error) = Engine::open(temporary.path()) else {
            panic!("competing engine must reject");
        };
        assert_eq!(error.code, ErrorCode::AuthorityBusy);
        drop(engine);
        Engine::open(temporary.path()).expect("reopened engine");
    }

    #[test]
    fn workspace_creation_response_preflight_precedes_publication() {
        let temporary = tempfile::tempdir().expect("state directory");
        let mut engine = Engine::open(temporary.path()).expect("engine");
        let Err(error) = engine.handle(RequestId::new(0), Request::CreateWorkspace) else {
            panic!("invalid correlation must reject");
        };
        assert_eq!(error.code, ErrorCode::PolicyExceeded);
        assert!(engine.workspaces.is_empty());
        assert!(
            persistence::list_workspace_ids(temporary.path())
                .expect("workspace listing")
                .is_empty()
        );
    }
}
