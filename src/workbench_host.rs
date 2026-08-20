//! Product-neutral adaptation between interactive workbench actions and exact host authorities.
//!
//! This adapter owns no editor state, key policy, graph mutation, or filesystem policy. It maps
//! closed application action values to the existing semantic-project and selected-filesystem
//! owners, then returns one closed outcome for the application's resume function.

use crate::application::{
    ApplicationInvocation, InteractiveAction, InteractiveActionOutcome,
    InteractiveActionOutcomeClass, MAXIMUM_INTERACTIVE_FRAME_SCALARS,
};
use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{IdempotencyKey, Revision, WorkspaceId};
use crate::project::{PROJECT_CHANGE_VERSION, Project, ProjectChangeRequest};
use crate::project_host::{
    ProjectGrantOperations, ProjectHostReceipt, ProjectHostRequest, ProjectHostResponse,
    SemanticProjectGrant,
};
use crate::selected_filesystem::{
    DirectoryEntryKind, DirectoryPage, FileReadOutcome, FileSaveOutcome, FileSaveRequest,
    FileVersion, FilesystemLimits, FilesystemOperations, ReconciliationOutcome,
    ReconciliationToken, RelativePath, SELECTED_FILESYSTEM_CONTRACT_VERSION, SaveMode,
    SearchCancellation, SearchRequest, SearchResult, SelectedFilesystem,
};
use crate::transaction::{TransactionMode, TransactionOp};
use crate::workbench::{SemanticProjection, parse_edit_document};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::path::{Path, PathBuf};

pub const WORKBENCH_HOST_CONTRACT_VERSION: u16 = 2;
pub const MAXIMUM_WORKBENCH_HOST_CONTENT_BYTES: usize = 1024 * 1024;
pub const MAXIMUM_WORKBENCH_HOST_CONTENT_SCALARS: usize = MAXIMUM_INTERACTIVE_FRAME_SCALARS - 1_024;
pub const MAXIMUM_WORKBENCH_FILE_TOKEN_BYTES: usize = 4_096;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
enum FileSessionToken {
    Origin { path: RelativePath, mode: SaveMode },
    Reconcile(ReconciliationToken),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ProjectDiffAction {
    from: Revision,
    to: Revision,
    offset: u64,
    limit: u32,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ProjectTargetBuildAction {
    selector: String,
    output: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(deny_unknown_fields)]
struct ProjectTargetRunAction {
    selector: String,
    invocation: ApplicationInvocation,
}

pub struct WorkbenchHost {
    project: Option<SemanticProjectGrant>,
    workspace: Option<WorkspaceId>,
    filesystem: Option<SelectedFilesystem>,
    next_action: u64,
}

impl WorkbenchHost {
    pub fn open(project: Option<&Path>, filesystem_root: Option<&Path>) -> Result<Self> {
        let (project, workspace) = if let Some(project) = project {
            let selected = Project::open(Some(project))?;
            let workspace = selected.workspace();
            let project_root = selected.root().to_owned();
            drop(selected);
            let grant = SemanticProjectGrant::open(
                "lkjedit-project".to_owned(),
                &project_root,
                workspace,
                ProjectGrantOperations::DEVELOPMENT,
            )?;
            (Some(grant), Some(workspace))
        } else {
            (None, None)
        };
        let filesystem = filesystem_root
            .map(|root| {
                SelectedFilesystem::select(
                    "lkjedit-files".to_owned(),
                    root,
                    FilesystemOperations::READ_WRITE,
                    FilesystemLimits::default(),
                )
            })
            .transpose()?;
        Ok(Self {
            project,
            workspace,
            filesystem,
            next_action: 1,
        })
    }

    pub const fn workspace(&self) -> Option<WorkspaceId> {
        self.workspace
    }

    pub fn project_root(&self) -> Option<&Path> {
        self.project.as_ref().map(SemanticProjectGrant::root)
    }

    pub fn filesystem_root(&self) -> Option<&Path> {
        self.filesystem.as_ref().map(SelectedFilesystem::root_path)
    }

    pub fn handle(&mut self, action: InteractiveAction) -> Result<InteractiveActionOutcome> {
        let result = self.handle_inner(action);
        Ok(match result {
            Ok(outcome) => outcome,
            Err(error) => error_outcome(error),
        })
    }

    fn handle_inner(&mut self, action: InteractiveAction) -> Result<InteractiveActionOutcome> {
        match action {
            InteractiveAction::ProjectOrient => {
                self.project_text(ProjectHostRequest::Orient { known_digest: None })
            }
            InteractiveAction::ProjectSummary(selector) => {
                self.query_text(SemanticProjection::Summary, selector)
            }
            InteractiveAction::ProjectChildren(selector) => {
                self.query_text(SemanticProjection::Children, selector)
            }
            InteractiveAction::ProjectFunction(selector) => {
                self.query_text(SemanticProjection::Function, selector)
            }
            InteractiveAction::ProjectCallers(selector) => {
                self.query_text(SemanticProjection::Callers, selector)
            }
            InteractiveAction::ProjectCallees(selector) => {
                self.query_text(SemanticProjection::Callees, selector)
            }
            InteractiveAction::ProjectTargets(selector) => {
                self.query_text(SemanticProjection::Targets, selector)
            }
            InteractiveAction::ProjectBlockers => {
                self.query_text(SemanticProjection::Blockers, String::new())
            }
            InteractiveAction::ProjectProposal(selector) => self.project_proposal(selector),
            InteractiveAction::ProjectHistory => self.project_text(ProjectHostRequest::History {
                before: None,
                limit: 64,
            }),
            InteractiveAction::ProjectRecord(revision) => self.project_record(revision),
            InteractiveAction::ProjectDiff(request) => self.project_diff(request),
            InteractiveAction::ProjectValidate(document) => {
                self.project_change(document, TransactionMode::ValidateOnly)
            }
            InteractiveAction::ProjectApply(document) => {
                self.project_change(document, TransactionMode::Commit)
            }
            InteractiveAction::ProjectTargetList => {
                self.project_text(ProjectHostRequest::TargetList { revision: None })
            }
            InteractiveAction::ProjectTargetTest(selector) => {
                let selector = required_text(selector, "target selector")?;
                self.project_text(ProjectHostRequest::TargetTest {
                    selector,
                    revision: None,
                })
            }
            InteractiveAction::ProjectTargetBuild(request) => self.project_target_build(request),
            InteractiveAction::ProjectTargetRun(request) => self.project_target_run(request),
            InteractiveAction::FilesystemList(path) => self.filesystem_list(path),
            InteractiveAction::FilesystemSearch { start, query } => {
                self.filesystem_search(start, query)
            }
            InteractiveAction::FilesystemRead(path) => self.filesystem_read(path),
            InteractiveAction::FilesystemSave {
                origin,
                content,
                create,
            } => self.filesystem_save(origin, content, create),
            InteractiveAction::FilesystemReconcile(token) => self.filesystem_reconcile(token),
        }
    }

    fn query_text(
        &mut self,
        projection: SemanticProjection,
        selector: String,
    ) -> Result<InteractiveActionOutcome> {
        let selectors = optional_selector(selector);
        self.project_text(ProjectHostRequest::Query {
            projection,
            selectors,
            revision: None,
            limit: 64,
            continuation: None,
            known_digest: None,
        })
    }

    fn project_record(&mut self, revision: String) -> Result<InteractiveActionOutcome> {
        let revision = required_text(revision, "revision")?;
        let value = revision.parse::<u64>().map_err(|_| {
            LkError::new(
                ErrorCode::InvalidQuery,
                "revision must be one canonical unsigned decimal integer",
            )
        })?;
        if value.to_string() != revision {
            return Err(LkError::new(
                ErrorCode::InvalidQuery,
                "revision must use canonical unsigned decimal spelling",
            ));
        }
        self.project_text(ProjectHostRequest::Record {
            revision: Revision::new(value),
        })
    }

    fn project_diff(&mut self, request: String) -> Result<InteractiveActionOutcome> {
        let request: ProjectDiffAction = decode_action_payload(&request, "project diff")?;
        self.project_text(ProjectHostRequest::Diff {
            from: request.from,
            to: request.to,
            offset: request.offset,
            limit: request.limit,
        })
    }

    fn project_target_build(&mut self, request: String) -> Result<InteractiveActionOutcome> {
        let request: ProjectTargetBuildAction = decode_action_payload(&request, "target build")?;
        let selector = required_text(request.selector, "target selector")?;
        let output = required_text(request.output, "target output path")?;
        self.project_text(ProjectHostRequest::TargetBuild {
            selector,
            revision: None,
            output: PathBuf::from(output),
        })
    }

    fn project_target_run(&mut self, request: String) -> Result<InteractiveActionOutcome> {
        let request: ProjectTargetRunAction = decode_action_payload(&request, "target run")?;
        let selector = required_text(request.selector, "target selector")?;
        self.project_text(ProjectHostRequest::TargetRun {
            selector,
            revision: None,
            invocation: request.invocation,
        })
    }

    fn project_proposal(&mut self, selector: String) -> Result<InteractiveActionOutcome> {
        let selector = required_text(selector, "function selector")?;
        let receipt = self.execute_project(ProjectHostRequest::Proposal {
            selector,
            revision: None,
        })?;
        let ProjectHostResponse::Proposal(proposal) = receipt.response else {
            return Err(internal_response("function proposal"));
        };
        bounded_success(
            format!(
                "proposal {} at revision {}",
                proposal.qualified_name, proposal.revision
            ),
            proposal.document,
            String::new(),
        )
    }

    fn project_change(
        &mut self,
        document: String,
        mode: TransactionMode,
    ) -> Result<InteractiveActionOutcome> {
        if document.len() > MAXIMUM_WORKBENCH_HOST_CONTENT_BYTES {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "semantic proposal exceeds the workbench host byte policy",
            ));
        }
        let parsed = parse_edit_document(document.as_bytes(), mode, None).map_err(|error| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                format!("semantic proposal is invalid: {error}"),
            )
        })?;
        let refresh_function = parsed
            .request
            .transaction
            .operations
            .iter()
            .find_map(|operation| match operation {
                TransactionOp::ReplaceFunctionBody { function, .. }
                | TransactionOp::DefineFunctionBody { function, .. } => Some(*function),
                _ => None,
            });
        let mut request = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: parsed.request.transaction.workspace,
            base_revision: parsed.request.transaction.base_revision,
            idempotency_key: parsed.request.transaction.idempotency_key,
            operations: parsed.request.transaction.operations,
            response: parsed.request.response,
        };
        if mode == TransactionMode::Commit && request.idempotency_key.is_none() {
            request.idempotency_key = Some(document_idempotency(&document));
        }
        let host_request = match mode {
            TransactionMode::ValidateOnly => ProjectHostRequest::Validate(Box::new(request)),
            TransactionMode::Commit => ProjectHostRequest::Apply(Box::new(request)),
        };
        let receipt = self.execute_project(host_request)?;
        let ProjectHostResponse::Change(change) = &receipt.response else {
            return Err(internal_response("semantic change"));
        };
        if mode == TransactionMode::ValidateOnly {
            return bounded_success(
                format!(
                    "validated {} semantic changes at revision {} without publication",
                    change.semantic_diff.changes.len(),
                    receipt.revision
                ),
                String::new(),
                String::new(),
            );
        }
        if let Some(function) = refresh_function {
            let refreshed = self.execute_project(ProjectHostRequest::Proposal {
                selector: function.to_string(),
                revision: None,
            })?;
            let ProjectHostResponse::Proposal(proposal) = refreshed.response else {
                return Err(internal_response("post-apply function proposal"));
            };
            return bounded_success(
                format!(
                    "applied semantic revision {} and refreshed draft",
                    receipt.revision
                ),
                proposal.document,
                String::new(),
            );
        }
        bounded_success(
            format!("applied semantic revision {}", receipt.revision),
            pretty(&receipt.response)?,
            String::new(),
        )
    }

    fn project_text(&mut self, request: ProjectHostRequest) -> Result<InteractiveActionOutcome> {
        let receipt = self.execute_project(request)?;
        bounded_success(
            format!("project revision {}", receipt.revision),
            pretty(&receipt.response)?,
            String::new(),
        )
    }

    fn execute_project(&mut self, request: ProjectHostRequest) -> Result<ProjectHostReceipt> {
        let action_id = self.action_id("project");
        let workspace = self.workspace.ok_or_else(|| {
            LkError::new(
                ErrorCode::CapabilityDenied,
                "lkjedit has no explicitly selected semantic project grant",
            )
        })?;
        let project = self.project.as_mut().ok_or_else(|| {
            LkError::new(
                ErrorCode::CapabilityDenied,
                "lkjedit has no explicitly selected semantic project grant",
            )
        })?;
        project.execute(action_id, workspace, request)
    }

    fn filesystem_list(&mut self, path: String) -> Result<InteractiveActionOutcome> {
        let path = relative_path(&path)?;
        let filesystem = self.filesystem()?;
        let page = filesystem.list(&path, 256, None)?;
        bounded_success(
            format!(
                "listed {}: {} entries{}",
                path.display(),
                page.entries.len(),
                if page.continuation.is_some() {
                    ", truncated"
                } else {
                    ""
                }
            ),
            directory_page_text(&page),
            String::new(),
        )
    }

    fn filesystem_read(&mut self, path: String) -> Result<InteractiveActionOutcome> {
        let path = relative_path(&path)?;
        let outcome = self.filesystem()?.read(&path)?;
        match outcome {
            FileReadOutcome::Opened(file) => {
                let content = String::from_utf8(file.content).map_err(|_| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "selected file is not valid UTF-8 text",
                    )
                })?;
                let token = encode_file_token(&FileSessionToken::Origin {
                    path: file.path.clone(),
                    mode: SaveMode::Replace {
                        expected: file.version.digest,
                    },
                })?;
                bounded_success(format!("opened {}", file.path.display()), content, token)
            }
            FileReadOutcome::NotFound => {
                let token = encode_file_token(&FileSessionToken::Origin {
                    path: path.clone(),
                    mode: SaveMode::Create,
                })?;
                bounded_success(format!("new file {}", path.display()), String::new(), token)
            }
            FileReadOutcome::WrongType => Err(LkError::new(
                ErrorCode::FilesystemWrongType,
                "selected filesystem entry is not a regular file",
            )),
            FileReadOutcome::Changed => Err(LkError::new(
                ErrorCode::FilesystemConflict,
                "selected file changed while it was being read",
            )),
        }
    }

    fn filesystem_search(
        &mut self,
        start: String,
        query: String,
    ) -> Result<InteractiveActionOutcome> {
        let start = relative_path(&start)?;
        let request = SearchRequest {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            start: start.clone(),
            query,
        };
        let cancellation = SearchCancellation::default();
        let result = self.filesystem()?.search(&request, &cancellation)?;
        bounded_success(
            format!(
                "searched {}: {} matches, {} files, {} bytes{}{}",
                start.display(),
                result.matches.len(),
                result.files,
                result.bytes,
                if result.truncated { ", truncated" } else { "" },
                if result.cancelled { ", cancelled" } else { "" }
            ),
            search_result_text(&result),
            String::new(),
        )
    }

    fn filesystem_save(
        &mut self,
        origin: String,
        content: String,
        create: bool,
    ) -> Result<InteractiveActionOutcome> {
        let (path, mode) = if create {
            (relative_path(&origin)?, SaveMode::Create)
        } else {
            match decode_file_token(&origin)? {
                FileSessionToken::Origin { path, mode } => (path, mode),
                FileSessionToken::Reconcile(_) => {
                    return Err(LkError::new(
                        ErrorCode::FilesystemUnknownVisibility,
                        "file has an unresolved possibly visible save",
                    ));
                }
            }
        };
        let action_id = self.action_id("file");
        let request = FileSaveRequest {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            action_id,
            path: path.clone(),
            content: content.into_bytes(),
            mode,
        };
        match self.filesystem()?.save(&request)? {
            FileSaveOutcome::Published {
                version,
                cleanup_pending,
            } => file_save_success(path, version, cleanup_pending, false),
            FileSaveOutcome::Unchanged { version } => file_save_success(path, version, false, true),
            FileSaveOutcome::Conflict { observed } => {
                let (message, mode) = observed.map_or_else(
                    || {
                        (
                            "file save conflict: target is absent".to_owned(),
                            SaveMode::Create,
                        )
                    },
                    |version| {
                        (
                            format!("file save conflict: observed {}", version.digest),
                            SaveMode::Replace {
                                expected: version.digest,
                            },
                        )
                    },
                );
                let token = encode_file_token(&FileSessionToken::Origin { path, mode })?;
                Ok(InteractiveActionOutcome {
                    job_id: 0,
                    class: InteractiveActionOutcomeClass::Conflict,
                    message,
                    content: String::new(),
                    token,
                })
            }
            FileSaveOutcome::NotFound => Err(LkError::new(
                ErrorCode::FilesystemNotFound,
                "file save expected an existing target",
            )),
            FileSaveOutcome::WrongType => Err(LkError::new(
                ErrorCode::FilesystemWrongType,
                "file save target is not a regular file",
            )),
            FileSaveOutcome::KnownFailure { message } => {
                Err(LkError::new(ErrorCode::FilesystemKnownFailure, message))
            }
            FileSaveOutcome::UnknownVisibility { token, message } => Ok(InteractiveActionOutcome {
                job_id: 0,
                class: InteractiveActionOutcomeClass::Unknown,
                message,
                content: String::new(),
                token: encode_file_token(&FileSessionToken::Reconcile(token))?,
            }),
        }
    }

    fn filesystem_reconcile(&mut self, token: String) -> Result<InteractiveActionOutcome> {
        let FileSessionToken::Reconcile(reconciliation) = decode_file_token(&token)? else {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "filesystem reconciliation requires an unknown-save token",
            ));
        };
        match self.filesystem()?.reconcile(&reconciliation)? {
            ReconciliationOutcome::Present {
                version,
                cleanup_pending,
            } => file_save_success(reconciliation.path, version, cleanup_pending, false),
            ReconciliationOutcome::Absent { observed } => {
                let mode = observed.map_or(SaveMode::Create, |version| SaveMode::Replace {
                    expected: version.digest,
                });
                Ok(InteractiveActionOutcome {
                    job_id: 0,
                    class: InteractiveActionOutcomeClass::Unchanged,
                    message: "unknown file save reconciled as absent".to_owned(),
                    content: String::new(),
                    token: encode_file_token(&FileSessionToken::Origin {
                        path: reconciliation.path,
                        mode,
                    })?,
                })
            }
            ReconciliationOutcome::Conflicting { observed } => Ok(InteractiveActionOutcome {
                job_id: 0,
                class: InteractiveActionOutcomeClass::Conflict,
                message: observed.map_or_else(
                    || "unknown file save reconciled against an absent target".to_owned(),
                    |version| {
                        format!(
                            "unknown file save reconciled against conflicting {}",
                            version.digest
                        )
                    },
                ),
                content: String::new(),
                token: String::new(),
            }),
            ReconciliationOutcome::Indeterminate { message } => Ok(InteractiveActionOutcome {
                job_id: 0,
                class: InteractiveActionOutcomeClass::Unknown,
                message,
                content: String::new(),
                token,
            }),
        }
    }

    fn filesystem(&self) -> Result<&SelectedFilesystem> {
        self.filesystem.as_ref().ok_or_else(|| {
            LkError::new(
                ErrorCode::CapabilityDenied,
                "workbench has no explicitly selected filesystem root",
            )
        })
    }

    fn action_id(&mut self, prefix: &str) -> String {
        let id = format!("{prefix}-{}", self.next_action);
        self.next_action = self.next_action.saturating_add(1);
        id
    }
}

fn directory_page_text(page: &DirectoryPage) -> String {
    let parent = page.path.display();
    let mut output = String::new();
    for entry in &page.entries {
        let kind = match entry.kind {
            DirectoryEntryKind::Directory => 'D',
            DirectoryEntryKind::File => 'F',
            DirectoryEntryKind::Symlink => 'L',
            DirectoryEntryKind::Other => 'O',
        };
        output.push(kind);
        output.push(' ');
        if parent != "." {
            output.push_str(&parent);
            output.push('/');
        }
        output.push_str(&entry.name);
        output.push('\n');
    }
    output
}

fn search_result_text(result: &SearchResult) -> String {
    let mut output = String::new();
    let mut previous = None;
    for found in &result.matches {
        let path = found.path.display();
        if previous.as_deref() == Some(path.as_str()) {
            continue;
        }
        output.push_str(&path);
        output.push('\n');
        previous = Some(path);
    }
    output
}

fn file_save_success(
    path: RelativePath,
    version: FileVersion,
    cleanup_pending: bool,
    unchanged: bool,
) -> Result<InteractiveActionOutcome> {
    let token = encode_file_token(&FileSessionToken::Origin {
        path: path.clone(),
        mode: SaveMode::Replace {
            expected: version.digest,
        },
    })?;
    Ok(InteractiveActionOutcome {
        job_id: 0,
        class: if unchanged {
            InteractiveActionOutcomeClass::Unchanged
        } else {
            InteractiveActionOutcomeClass::Succeeded
        },
        message: format!(
            "{} {}{}",
            if unchanged { "unchanged" } else { "saved" },
            path.display(),
            if cleanup_pending {
                "; temporary cleanup remains pending"
            } else {
                ""
            }
        ),
        content: String::new(),
        token,
    })
}

fn error_outcome(error: LkError) -> InteractiveActionOutcome {
    let class = match error.code {
        ErrorCode::NoChange => InteractiveActionOutcomeClass::Unchanged,
        ErrorCode::RevisionConflict
        | ErrorCode::IdempotencyConflict
        | ErrorCode::FilesystemConflict => InteractiveActionOutcomeClass::Conflict,
        ErrorCode::CommitOutcomeUnknown
        | ErrorCode::ArtifactPublicationOutcomeUnknown
        | ErrorCode::HostOutcomeUnknown
        | ErrorCode::FilesystemUnknownVisibility
        | ErrorCode::FilesystemReconciliationIndeterminate => {
            InteractiveActionOutcomeClass::Unknown
        }
        _ => InteractiveActionOutcomeClass::Rejected,
    };
    InteractiveActionOutcome {
        job_id: 0,
        class,
        message: format!("{}: {}", error.code.machine_name(), error.message),
        content: String::new(),
        token: String::new(),
    }
}

fn document_idempotency(document: &str) -> IdempotencyKey {
    let mut digest = blake3::Hasher::new_derive_key("lkjscript.workbench-document-action.v1");
    digest.update(document.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest.finalize().as_bytes()[..16]);
    IdempotencyKey::from_bytes(bytes)
}

fn optional_selector(value: String) -> Vec<String> {
    let value = value.trim();
    if value.is_empty() {
        Vec::new()
    } else {
        vec![value.to_owned()]
    }
}

fn required_text(value: String, label: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() {
        Err(LkError::new(
            ErrorCode::InvalidQuery,
            format!("{label} is empty"),
        ))
    } else {
        Ok(value.to_owned())
    }
}

fn decode_action_payload<T: DeserializeOwned>(value: &str, label: &str) -> Result<T> {
    if value.len() > MAXIMUM_WORKBENCH_HOST_CONTENT_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("{label} request exceeds the workbench host byte policy"),
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_str(value);
    let request = T::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} request is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("{label} request has trailing input: {error}"),
        )
    })?;
    Ok(request)
}

fn relative_path(value: &str) -> Result<RelativePath> {
    let value = value.trim();
    if value.is_empty() || value == "." {
        return Ok(RelativePath::root());
    }
    if value.starts_with('/') || value.ends_with('/') {
        return Err(LkError::new(
            ErrorCode::FilesystemDenied,
            "filesystem path must be a relative component sequence",
        ));
    }
    RelativePath::new(value.split('/').map(str::to_owned).collect())
}

fn encode_file_token(token: &FileSessionToken) -> Result<String> {
    let token = serde_json::to_string(token).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode file-origin token: {error}"),
        )
    })?;
    if token.len() > MAXIMUM_WORKBENCH_FILE_TOKEN_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "file-origin token exceeds workbench policy",
        ));
    }
    Ok(token)
}

fn decode_file_token(token: &str) -> Result<FileSessionToken> {
    if token.len() > MAXIMUM_WORKBENCH_FILE_TOKEN_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "file-origin token exceeds workbench policy",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_str(token);
    let token = FileSessionToken::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("file-origin token is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("file-origin token has trailing input: {error}"),
        )
    })?;
    Ok(token)
}

fn pretty(value: &impl Serialize) -> Result<String> {
    let content = serde_json::to_string_pretty(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot render exact host response: {error}"),
        )
    })?;
    if content.len() > MAXIMUM_WORKBENCH_HOST_CONTENT_BYTES
        || content.chars().count() > MAXIMUM_WORKBENCH_HOST_CONTENT_SCALARS
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "host response exceeds workbench editor-buffer policy",
        ));
    }
    Ok(content)
}

fn bounded_success(
    message: String,
    content: String,
    token: String,
) -> Result<InteractiveActionOutcome> {
    if content.len() > MAXIMUM_WORKBENCH_HOST_CONTENT_BYTES
        || content.chars().count() > MAXIMUM_WORKBENCH_HOST_CONTENT_SCALARS
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "host response exceeds workbench editor-buffer policy",
        ));
    }
    Ok(InteractiveActionOutcome {
        job_id: 0,
        class: InteractiveActionOutcomeClass::Succeeded,
        message,
        content,
        token,
    })
}

fn internal_response(label: &str) -> LkError {
    LkError::new(
        ErrorCode::ArtifactCorrupt,
        format!("semantic-project adapter returned the wrong {label} response"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selected_filesystem::{DirectoryEntry, SearchMatch};

    #[test]
    fn host_content_reserves_bounded_application_frame_space() {
        assert_eq!(MAXIMUM_WORKBENCH_HOST_CONTENT_SCALARS, 130_048);
        assert!(
            bounded_success(
                "exact".into(),
                "x".repeat(MAXIMUM_WORKBENCH_HOST_CONTENT_SCALARS),
                String::new(),
            )
            .is_ok()
        );
        assert_eq!(
            bounded_success(
                "one over".into(),
                "x".repeat(MAXIMUM_WORKBENCH_HOST_CONTENT_SCALARS + 1),
                String::new(),
            )
            .expect_err("one-over host content")
            .code,
            ErrorCode::PolicyExceeded
        );
    }

    #[test]
    fn filesystem_pages_use_closed_unambiguous_application_lines() {
        let page = DirectoryPage {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            path: RelativePath::new(vec!["nested".into()]).expect("directory path"),
            digest: "observation".into(),
            entries: vec![
                DirectoryEntry {
                    name: "child".into(),
                    kind: DirectoryEntryKind::Directory,
                    size: 0,
                },
                DirectoryEntry {
                    name: "note.txt".into(),
                    kind: DirectoryEntryKind::File,
                    size: 12,
                },
            ],
            continuation: None,
        };
        assert_eq!(
            directory_page_text(&page),
            "D nested/child\nF nested/note.txt\n"
        );

        let result = SearchResult {
            contract_version: SELECTED_FILESYSTEM_CONTRACT_VERSION,
            start: RelativePath::root(),
            query: "needle".into(),
            matches: vec![
                SearchMatch {
                    path: RelativePath::new(vec!["a.txt".into()]).expect("a"),
                    start_byte: 0,
                    end_byte: 6,
                    line: 1,
                    preview: "needle".into(),
                },
                SearchMatch {
                    path: RelativePath::new(vec!["a.txt".into()]).expect("a again"),
                    start_byte: 7,
                    end_byte: 13,
                    line: 2,
                    preview: "needle".into(),
                },
                SearchMatch {
                    path: RelativePath::new(vec!["b.txt".into()]).expect("b"),
                    start_byte: 3,
                    end_byte: 9,
                    line: 1,
                    preview: "needle".into(),
                },
            ],
            directories: 1,
            files: 2,
            bytes: 24,
            unsupported_files: 0,
            failures: vec![],
            truncated: false,
            cancelled: false,
        };
        assert_eq!(search_result_text(&result), "a.txt\nb.txt\n");
    }

    #[test]
    fn project_and_selected_file_actions_use_exact_owners() {
        let root = tempfile::tempdir().expect("root");
        let initialized = Project::initialize(root.path()).expect("project");
        let mut host = WorkbenchHost::open(Some(root.path()), Some(root.path())).expect("host");
        assert_eq!(host.workspace(), Some(initialized.workspace));

        let orientation = host
            .handle(InteractiveAction::ProjectOrient)
            .expect("orientation");
        assert_eq!(orientation.class, InteractiveActionOutcomeClass::Succeeded);
        assert!(
            orientation
                .content
                .contains(&initialized.workspace.to_string())
        );

        let opened = host
            .handle(InteractiveAction::FilesystemRead("note.txt".into()))
            .expect("new file origin");
        assert_eq!(opened.class, InteractiveActionOutcomeClass::Succeeded);
        assert!(!opened.token.is_empty());
        let saved = host
            .handle(InteractiveAction::FilesystemSave {
                origin: opened.token,
                content: "hello".into(),
                create: false,
            })
            .expect("save");
        assert_eq!(saved.class, InteractiveActionOutcomeClass::Succeeded);
        assert_eq!(
            std::fs::read_to_string(root.path().join("note.txt")).expect("saved bytes"),
            "hello"
        );

        let read = host
            .handle(InteractiveAction::FilesystemRead("note.txt".into()))
            .expect("read");
        assert_eq!(read.content, "hello");
        std::fs::write(root.path().join("note.txt"), "external").expect("external change");
        let conflict = host
            .handle(InteractiveAction::FilesystemSave {
                origin: read.token,
                content: "local".into(),
                create: false,
            })
            .expect("conflict outcome");
        assert_eq!(conflict.class, InteractiveActionOutcomeClass::Conflict);
        assert!(!conflict.token.is_empty());
        assert_eq!(
            std::fs::read_to_string(root.path().join("note.txt")).expect("preserved external"),
            "external"
        );

        let overwritten = host
            .handle(InteractiveAction::FilesystemSave {
                origin: conflict.token,
                content: "local".into(),
                create: false,
            })
            .expect("explicit current-base overwrite");
        assert_eq!(overwritten.class, InteractiveActionOutcomeClass::Succeeded);
        assert_eq!(
            std::fs::read_to_string(root.path().join("note.txt")).expect("overwritten bytes"),
            "local"
        );
    }
}
