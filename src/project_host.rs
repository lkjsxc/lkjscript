//! Explicit in-process semantic-project grant.
//!
//! This adapter calls the same [`crate::project::Project`] owner as the public CLI. It adds only a
//! closed request vocabulary, workspace/operation authorization, correlation, and request/response
//! bounds. It does not expose an engine, repository path, lock, snapshot mutation, or retry loop.

use crate::application::{ApplicationInvocation, ApplicationRunReceipt};
use crate::error::{ErrorCode, LkError, Result};
use crate::history::{HistoryPage, RevisionRecordInspection};
use crate::ids::{Revision, WorkspaceId};
use crate::project::{
    Project, ProjectChangeReceipt, ProjectChangeRequest, ProjectDiffPage, ProjectFunctionProposal,
    ProjectOrientationResult, ProjectStatus, ProjectTargetReceipt,
};
use crate::target::TargetSummary;
use crate::transaction::TransactionMode;
use crate::workbench::{SemanticProjection, SemanticQueryResult};
use serde::Serialize;
use std::path::{Path, PathBuf};

pub const PROJECT_HOST_CONTRACT_VERSION: u16 = 2;
pub const MAXIMUM_PROJECT_HOST_REQUEST_BYTES: usize = 4 * 1024 * 1024;
pub const MAXIMUM_PROJECT_HOST_RESPONSE_BYTES: usize = 32 * 1024 * 1024;
pub const MAXIMUM_PROJECT_HOST_SELECTORS: usize = 32;
pub const MAXIMUM_PROJECT_HOST_SELECTOR_BYTES: usize = 512;
pub const MAXIMUM_PROJECT_HOST_ACTION_ID_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectGrantOperations {
    pub read: bool,
    pub mutate: bool,
    pub targets: bool,
}

impl ProjectGrantOperations {
    pub const DEVELOPMENT: Self = Self {
        read: true,
        mutate: true,
        targets: true,
    };
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ProjectHostRequest {
    Status,
    Orient {
        known_digest: Option<String>,
    },
    Query {
        projection: SemanticProjection,
        selectors: Vec<String>,
        revision: Option<Revision>,
        limit: u32,
        continuation: Option<String>,
        known_digest: Option<String>,
    },
    Proposal {
        selector: String,
        revision: Option<Revision>,
    },
    History {
        before: Option<Revision>,
        limit: u32,
    },
    Record {
        revision: Revision,
    },
    Diff {
        from: Revision,
        to: Revision,
        offset: u64,
        limit: u32,
    },
    Validate(Box<ProjectChangeRequest>),
    Apply(Box<ProjectChangeRequest>),
    TargetList {
        revision: Option<Revision>,
    },
    TargetTest {
        selector: String,
        revision: Option<Revision>,
    },
    TargetBuild {
        selector: String,
        revision: Option<Revision>,
        output: PathBuf,
    },
    TargetRun {
        selector: String,
        revision: Option<Revision>,
        invocation: ApplicationInvocation,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
#[allow(clippy::large_enum_variant)] // Responses are already byte-bounded and retain their public owner shapes.
pub enum ProjectHostResponse {
    Status(ProjectStatus),
    Orientation(ProjectOrientationResult),
    Query(SemanticQueryResult),
    Proposal(ProjectFunctionProposal),
    History(HistoryPage),
    Record(RevisionRecordInspection),
    Diff(ProjectDiffPage),
    Change(ProjectChangeReceipt),
    Targets(Vec<TargetSummary>),
    TargetTest(ProjectTargetReceipt),
    TargetBuild(ProjectTargetReceipt),
    TargetRun(ApplicationRunReceipt),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectHostReceipt {
    pub contract_version: u16,
    pub action_id: String,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub response: ProjectHostResponse,
}

pub struct SemanticProjectGrant {
    grant_id: String,
    operations: ProjectGrantOperations,
    project: Project,
}

impl SemanticProjectGrant {
    pub fn open(
        grant_id: String,
        path: &Path,
        expected_workspace: WorkspaceId,
        operations: ProjectGrantOperations,
    ) -> Result<Self> {
        validate_identifier(&grant_id, "semantic-project grant identity")?;
        let project = Project::open(Some(path))?;
        if project.workspace() != expected_workspace {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "semantic-project grant selected a foreign workspace",
            )
            .for_workspace(project.workspace()));
        }
        Ok(Self {
            grant_id,
            operations,
            project,
        })
    }

    pub fn grant_id(&self) -> &str {
        &self.grant_id
    }

    pub fn workspace(&self) -> WorkspaceId {
        self.project.workspace()
    }

    pub fn root(&self) -> &Path {
        self.project.root()
    }

    pub fn execute(
        &mut self,
        action_id: String,
        expected_workspace: WorkspaceId,
        request: ProjectHostRequest,
    ) -> Result<ProjectHostReceipt> {
        validate_identifier(&action_id, "semantic-project action identity")?;
        if expected_workspace != self.workspace() {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "semantic-project action names a workspace outside its grant",
            )
            .for_workspace(expected_workspace));
        }
        validate_request(&request)?;
        preflight(&request, MAXIMUM_PROJECT_HOST_REQUEST_BYTES, "request")?;
        let response = match request {
            ProjectHostRequest::Status => {
                self.require(self.operations.read, "project status")?;
                ProjectHostResponse::Status(self.project.status()?)
            }
            ProjectHostRequest::Orient { known_digest } => {
                self.require(self.operations.read, "project orientation")?;
                ProjectHostResponse::Orientation(self.project.orient(known_digest.as_deref())?)
            }
            ProjectHostRequest::Query {
                projection,
                selectors,
                revision,
                limit,
                continuation,
                known_digest,
            } => {
                self.require(self.operations.read, "semantic query")?;
                ProjectHostResponse::Query(self.project.semantic_query(
                    projection,
                    &selectors,
                    revision,
                    limit,
                    continuation,
                    known_digest.as_deref(),
                )?)
            }
            ProjectHostRequest::Proposal { selector, revision } => {
                self.require(self.operations.read, "function proposal")?;
                ProjectHostResponse::Proposal(self.project.function_proposal(&selector, revision)?)
            }
            ProjectHostRequest::History { before, limit } => {
                self.require(self.operations.read, "revision history")?;
                ProjectHostResponse::History(self.project.history(before, limit)?)
            }
            ProjectHostRequest::Record { revision } => {
                self.require(self.operations.read, "revision record")?;
                ProjectHostResponse::Record(self.project.revision_record(revision)?)
            }
            ProjectHostRequest::Diff {
                from,
                to,
                offset,
                limit,
            } => {
                self.require(self.operations.read, "semantic diff")?;
                ProjectHostResponse::Diff(self.project.diff(from, to, offset, limit)?)
            }
            ProjectHostRequest::Validate(request) => {
                self.require(self.operations.mutate, "project change validation")?;
                ProjectHostResponse::Change(
                    self.project
                        .apply_change(&request, TransactionMode::ValidateOnly)?,
                )
            }
            ProjectHostRequest::Apply(request) => {
                self.require(self.operations.mutate, "project change publication")?;
                ProjectHostResponse::Change(
                    self.project
                        .apply_change(&request, TransactionMode::Commit)?,
                )
            }
            ProjectHostRequest::TargetList { revision } => {
                self.require(self.operations.targets, "target listing")?;
                ProjectHostResponse::Targets(self.project.list_targets(revision)?)
            }
            ProjectHostRequest::TargetTest { selector, revision } => {
                self.require(self.operations.targets, "target testing")?;
                ProjectHostResponse::TargetTest(self.project.test_target(&selector, revision)?)
            }
            ProjectHostRequest::TargetBuild {
                selector,
                revision,
                output,
            } => {
                self.require(self.operations.targets, "target building")?;
                ProjectHostResponse::TargetBuild(
                    self.project.build_target(&selector, revision, &output)?,
                )
            }
            ProjectHostRequest::TargetRun {
                selector,
                revision,
                invocation,
            } => {
                self.require(self.operations.targets, "target running")?;
                ProjectHostResponse::TargetRun(self.project.run_target(
                    &selector,
                    revision,
                    &invocation,
                )?)
            }
        };
        let receipt = ProjectHostReceipt {
            contract_version: PROJECT_HOST_CONTRACT_VERSION,
            action_id,
            workspace: self.workspace(),
            revision: self.project.current_revision()?,
            response,
        };
        preflight(&receipt, MAXIMUM_PROJECT_HOST_RESPONSE_BYTES, "response")?;
        Ok(receipt)
    }

    fn require(&self, allowed: bool, operation: &str) -> Result<()> {
        if allowed {
            Ok(())
        } else {
            Err(LkError::new(
                ErrorCode::CapabilityDenied,
                format!("semantic-project grant does not allow {operation}"),
            )
            .for_workspace(self.workspace()))
        }
    }
}

fn validate_request(request: &ProjectHostRequest) -> Result<()> {
    match request {
        ProjectHostRequest::Query { selectors, .. } => validate_selectors(selectors),
        ProjectHostRequest::Proposal { selector, .. }
        | ProjectHostRequest::TargetTest { selector, .. }
        | ProjectHostRequest::TargetBuild { selector, .. }
        | ProjectHostRequest::TargetRun { selector, .. } => {
            validate_selectors(std::slice::from_ref(selector))
        }
        _ => Ok(()),
    }
}

fn validate_selectors(selectors: &[String]) -> Result<()> {
    if selectors.len() > MAXIMUM_PROJECT_HOST_SELECTORS
        || selectors.iter().any(|selector| {
            selector.is_empty() || selector.len() > MAXIMUM_PROJECT_HOST_SELECTOR_BYTES
        })
    {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic-project selectors exceed the host grant policy",
        ));
    }
    Ok(())
}

fn validate_identifier(value: &str, label: &str) -> Result<()> {
    if value.is_empty()
        || value.len() > MAXIMUM_PROJECT_HOST_ACTION_ID_BYTES
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!(
                "{label} must be 1..={MAXIMUM_PROJECT_HOST_ACTION_ID_BYTES} ASCII identifier bytes"
            ),
        ));
    }
    Ok(())
}

fn preflight(value: &impl Serialize, maximum: usize, label: &str) -> Result<()> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode semantic-project host {label}: {error}"),
        )
    })?;
    if bytes.len() > maximum {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            format!("semantic-project host {label} exceeds its byte policy"),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::{DraftSymbol, IdempotencyKey};
    use crate::transaction::{TransactionOp, TransactionResponseSpec};

    fn create_request(workspace: WorkspaceId, base: Revision, key: u128) -> ProjectChangeRequest {
        ProjectChangeRequest {
            version: crate::project::PROJECT_CHANGE_VERSION,
            workspace,
            base_revision: base,
            idempotency_key: Some(IdempotencyKey::from_bytes(key.to_le_bytes())),
            operations: vec![TransactionOp::CreatePackage {
                symbol: DraftSymbol::new("package"),
                name: "hosted".to_owned(),
            }],
            response: TransactionResponseSpec::default(),
        }
    }

    #[test]
    fn read_validate_apply_idempotency_and_foreign_authority_are_exact() {
        let root = tempfile::tempdir().expect("root");
        let initialized = Project::initialize(root.path()).expect("initialize");
        let mut grant = SemanticProjectGrant::open(
            "project-grant".to_owned(),
            root.path(),
            initialized.workspace,
            ProjectGrantOperations::DEVELOPMENT,
        )
        .expect("grant");

        let status = grant
            .execute(
                "status-1".to_owned(),
                initialized.workspace,
                ProjectHostRequest::Status,
            )
            .expect("status");
        assert_eq!(status.revision, Revision::INITIAL);

        let request = create_request(initialized.workspace, Revision::INITIAL, 1);
        let mut validation_request = request.clone();
        validation_request.idempotency_key = None;
        let validated = grant
            .execute(
                "validate-1".to_owned(),
                initialized.workspace,
                ProjectHostRequest::Validate(Box::new(validation_request)),
            )
            .expect("validate");
        assert_eq!(validated.revision, Revision::INITIAL);
        let applied = grant
            .execute(
                "apply-1".to_owned(),
                initialized.workspace,
                ProjectHostRequest::Apply(Box::new(request.clone())),
            )
            .expect("apply");
        assert_eq!(applied.revision, Revision::new(1));
        let replayed = grant
            .execute(
                "apply-replay".to_owned(),
                initialized.workspace,
                ProjectHostRequest::Apply(Box::new(request)),
            )
            .expect("idempotent replay");
        assert_eq!(applied.response, replayed.response);
        assert_eq!(
            grant.project.current_revision().expect("head"),
            Revision::new(1)
        );

        let foreign = WorkspaceId::from_bytes([9; 16]);
        assert_eq!(
            grant
                .execute("foreign".to_owned(), foreign, ProjectHostRequest::Status)
                .expect_err("foreign workspace")
                .code,
            ErrorCode::WrongWorkspace
        );
    }

    #[test]
    fn read_only_grant_rejects_mutation_without_publication() {
        let root = tempfile::tempdir().expect("root");
        let initialized = Project::initialize(root.path()).expect("initialize");
        let mut grant = SemanticProjectGrant::open(
            "read-only".to_owned(),
            root.path(),
            initialized.workspace,
            ProjectGrantOperations {
                read: true,
                mutate: false,
                targets: false,
            },
        )
        .expect("grant");
        let error = grant
            .execute(
                "denied".to_owned(),
                initialized.workspace,
                ProjectHostRequest::Apply(Box::new(create_request(
                    initialized.workspace,
                    Revision::INITIAL,
                    2,
                ))),
            )
            .expect_err("mutation denied");
        assert_eq!(error.code, ErrorCode::CapabilityDenied);
        assert_eq!(
            grant.project.current_revision().expect("head"),
            Revision::INITIAL
        );
    }

    #[test]
    fn in_process_reads_equal_the_public_project_owner() {
        let root = tempfile::tempdir().expect("root");
        let initialized = Project::initialize(root.path()).expect("initialize");
        let mut grant = SemanticProjectGrant::open(
            "oracle".to_owned(),
            root.path(),
            initialized.workspace,
            ProjectGrantOperations::DEVELOPMENT,
        )
        .expect("grant");
        let host = grant
            .execute(
                "orient".to_owned(),
                initialized.workspace,
                ProjectHostRequest::Orient { known_digest: None },
            )
            .expect("host orientation");
        let ProjectHostResponse::Orientation(host_orientation) = host.response else {
            panic!("wrong host response");
        };
        drop(grant);
        let direct = Project::open(Some(root.path())).expect("direct project");
        assert_eq!(
            host_orientation,
            direct.orient(None).expect("direct orientation")
        );
    }
}
