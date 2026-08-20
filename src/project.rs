//! Discoverable semantic development projects over the single-workspace durable authority.

use crate::application::{ApplicationBuildReceipt, ApplicationInvocation, ApplicationRunReceipt};
use crate::diff::{Change, SemanticDiff};
use crate::engine::Engine;
use crate::error::{ErrorCode, LkError, Result};
use crate::history::{HistoryPage, RevisionRecordInspection};
use crate::ids::{
    IdempotencyKey, NodeId, RequestId, Revision, RevisionRecordDigest, SnapshotHash, WorkspaceId,
};
use crate::machine::MachineSchemaDigest;
use crate::protocol::{Request, Response};
use crate::query::{NodeView, WorkspaceSummary};
use crate::release::ReleaseBuildReceipt;
use crate::schema::Node;
use crate::target::{BuildTargetDefinition, BuildTargetKind, PreparedTarget, TargetSummary};
use crate::transaction::{
    ApplyTransactionRequest, Transaction, TransactionMode, TransactionOp, TransactionReceipt,
    TransactionResponseSpec,
};
use crate::workbench::{
    ContextBuildRequest, ContextPacket, ContextPacketDigest, ContextPurpose, SemanticProjection,
    SemanticQueryRequest, SemanticQueryResult,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

pub const PROJECT_CONTRACT_VERSION: u16 = 1;
pub const PROJECT_CHANGE_VERSION: u16 = 2;
pub const PROJECT_CHANGE_CONTINUATION_VERSION: u16 = 1;
pub const PROJECT_MARKER_NAME: &str = "project";
pub const PROJECT_DIRECTORY_NAME: &str = ".lkjscript";
pub const PROJECT_REPOSITORY_NAME: &str = "repository";
pub const MAXIMUM_PROJECT_MARKER_BYTES: usize = 128;
pub const MAXIMUM_PROJECT_CHANGE_BYTES: usize = crate::machine::MAX_JSON_INPUT_BYTES;
pub const MAXIMUM_PROJECT_CHANGE_CONTINUATION_BYTES: usize = 64 * 1024;
pub const MAXIMUM_DISCOVERY_DEPTH: usize = 64;
pub const MAXIMUM_BACKUP_FILES: u64 = 1_000_000;
pub const MAXIMUM_BACKUP_BYTES: u64 = 4 * 1024 * 1024 * 1024;
const PROJECT_MARKER_MAGIC: [u8; 8] = *b"LKJPROJ1";
const PROJECT_MARKER_CHECKSUM_DOMAIN: &str = "lkjscript.project-marker.v1";
const PROJECT_ORIENTATION_DIGEST_DOMAIN: &str = "lkjscript.project-orientation.v1";
static PROJECT_TEMP_SERIAL: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectLocatorFacts {
    pub root: String,
    pub project_directory: String,
    pub repository_directory: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectInitReceipt {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub revision_record: RevisionRecordDigest,
    pub locator: ProjectLocatorFacts,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupReceipt {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub revision_record: RevisionRecordDigest,
    pub files: u64,
    pub bytes: u64,
    pub destination: ProjectLocatorFacts,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectStatus {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub revision_record: RevisionRecordDigest,
    pub health: ProjectHealth,
    pub summary: WorkspaceSummary,
    pub target_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectHealth {
    Healthy,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectOrientation {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub revision_record: RevisionRecordDigest,
    pub machine_schema: MachineSchemaDigest,
    pub orientation_digest: String,
    pub targets: Vec<TargetSummary>,
    pub commands: Vec<&'static str>,
    pub omissions: Vec<&'static str>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProjectOrientationResult {
    Changed(ProjectOrientation),
    Unchanged {
        contract_version: u16,
        workspace: WorkspaceId,
        revision: Revision,
        orientation_digest: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProjectContextResult {
    Changed(Box<ContextPacket>),
    Unchanged {
        contract_version: u16,
        workspace: WorkspaceId,
        revision: Revision,
        digest: ContextPacketDigest,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectChangeRequest {
    pub version: u16,
    pub workspace: WorkspaceId,
    pub base_revision: Revision,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub idempotency_key: Option<IdempotencyKey>,
    pub operations: Vec<TransactionOp>,
    #[serde(default)]
    pub response: TransactionResponseSpec,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectChangeReceipt {
    pub contract_version: u16,
    pub transaction: TransactionReceipt,
    pub semantic_diff: SemanticDiff,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision_record: Option<RevisionRecordInspection>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<ProjectChangeContinuation>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectChangeContinuation {
    pub version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub revision_record: RevisionRecordDigest,
    pub accepted_change_set: crate::ChangeDigest,
    pub semantic_diff: crate::ChangeDigest,
    pub returned_bindings: Vec<(crate::DraftSymbol, NodeId)>,
    pub changed_functions: Vec<NodeId>,
    pub affected_targets: Vec<NodeId>,
    pub session_local_aliases_invalidated: bool,
    pub digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectDiffPage {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub from: Revision,
    pub to: Revision,
    pub before_snapshot: SnapshotHash,
    pub after_snapshot: SnapshotHash,
    pub digest: crate::ids::ChangeDigest,
    pub total: u64,
    pub offset: u64,
    pub changes: Vec<Change>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectNodeInspection {
    pub contract_version: u16,
    pub selector: String,
    pub resolved: NodeId,
    pub qualified_name: String,
    pub view: NodeView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFunctionProposal {
    pub contract_version: u16,
    pub document_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub function: NodeId,
    pub qualified_name: String,
    pub document_digest: String,
    pub document: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetInspection {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub target: NodeId,
    pub name: String,
    pub kind: BuildTargetKind,
    pub definition_digest: String,
    pub definition: BuildTargetDefinition,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetArtifactReceipt {
    Release(Box<ReleaseBuildReceipt>),
    Application(Box<ApplicationBuildReceipt>),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTargetReceipt {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub target: NodeId,
    pub target_name: String,
    pub artifact_bytes: u64,
    pub artifact_digest: String,
    pub published: bool,
    pub artifact: TargetArtifactReceipt,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TargetTestCountSummary {
    pub total: u64,
    pub passed: u64,
    pub failed: u64,
    pub release_total: u64,
    pub application_total: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TargetArtifactSummaryReceipt {
    Release {
        contract_version: u16,
        format_version: u16,
        release: crate::release::ReleaseId,
        content_digest: crate::release::ReleaseContentDigest,
        tests: TargetTestCountSummary,
    },
    Application {
        contract_version: u16,
        format_version: u16,
        digest: crate::application::ApplicationDigest,
        graph_digest: crate::application::ApplicationGraphDigest,
        root_release: crate::release::ReleaseId,
        tests: TargetTestCountSummary,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectTargetSummaryReceipt {
    pub contract_version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub target: NodeId,
    pub target_name: String,
    pub artifact_bytes: u64,
    pub artifact_digest: String,
    pub published: bool,
    pub artifact: TargetArtifactSummaryReceipt,
}

impl ProjectTargetReceipt {
    /// Projects a successful target preparation into the bounded public command receipt.
    ///
    /// Complete artifact inspection and per-case passing detail remain available from their
    /// artifact owners. Target preparation rejects before returning this receipt when any case
    /// does not pass, so an all-pass count is sufficient continuation state here.
    pub fn into_summary(self) -> ProjectTargetSummaryReceipt {
        let artifact = match self.artifact {
            TargetArtifactReceipt::Release(release) => {
                let total = release.tests.total;
                let passed = release.tests.passed;
                TargetArtifactSummaryReceipt::Release {
                    contract_version: release.contract_version,
                    format_version: release.inspection.format_version,
                    release: release.inspection.release,
                    content_digest: release.inspection.content_digest,
                    tests: TargetTestCountSummary {
                        total,
                        passed,
                        failed: total.saturating_sub(passed),
                        release_total: total,
                        application_total: 0,
                    },
                }
            }
            TargetArtifactReceipt::Application(application) => {
                let total = application.tests.total;
                let passed = application.tests.passed;
                TargetArtifactSummaryReceipt::Application {
                    contract_version: application.contract_version,
                    format_version: application.inspection.format_version,
                    digest: application.inspection.digest,
                    graph_digest: application.inspection.graph_digest,
                    root_release: application.inspection.root_release,
                    tests: TargetTestCountSummary {
                        total,
                        passed,
                        failed: total.saturating_sub(passed),
                        release_total: application.tests.release_total,
                        application_total: application.tests.application_total,
                    },
                }
            }
        };
        ProjectTargetSummaryReceipt {
            contract_version: self.contract_version,
            workspace: self.workspace,
            revision: self.revision,
            snapshot: self.snapshot,
            target: self.target,
            target_name: self.target_name,
            artifact_bytes: self.artifact_bytes,
            artifact_digest: self.artifact_digest,
            published: self.published,
            artifact,
        }
    }
}

impl From<ProjectTargetReceipt> for ProjectTargetSummaryReceipt {
    fn from(receipt: ProjectTargetReceipt) -> Self {
        receipt.into_summary()
    }
}

/// An opened project holds the single-writer engine lock for its repository until it is dropped.
pub struct Project {
    root: PathBuf,
    repository: PathBuf,
    workspace: WorkspaceId,
    engine: Engine,
}

impl Project {
    /// Creates revision zero and a strict project marker as one directory-level publication.
    pub fn initialize(path: &Path) -> Result<ProjectInitReceipt> {
        let (root, created_root) = prepare_initialization_root(path)?;
        let project_directory = root.join(PROJECT_DIRECTORY_NAME);
        match fs::symlink_metadata(&project_directory) {
            Ok(_) => {
                return Err(LkError::new(
                    ErrorCode::WorkspaceExists,
                    "semantic project marker directory already exists",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let serial = PROJECT_TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
        let staging = root.join(format!(
            ".lkjscript.creating-{}-{serial}",
            std::process::id()
        ));
        create_private_directory(&staging)?;
        let result = (|| -> Result<ProjectInitReceipt> {
            let repository = staging.join(PROJECT_REPOSITORY_NAME);
            create_private_directory(&repository)?;
            let mut engine = Engine::open(&repository)?;
            let response = engine.request(RequestId::new(1), Request::CreateWorkspace)?;
            let Response::WorkspaceCreated(summary) = response else {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "project genesis returned an unexpected engine response",
                ));
            };
            let workspace = summary.workspace;
            let record_digest = engine
                .workspace(workspace)?
                .record(Revision::INITIAL)?
                .digest;
            drop(engine);
            write_marker(&staging.join(PROJECT_MARKER_NAME), workspace)?;
            sync_directory(&staging)?;
            fs::rename(&staging, &project_directory)?;
            sync_directory(&root).map_err(|error| {
                LkError::new(
                    ErrorCode::CommitOutcomeUnknown,
                    format!(
                        "project directory became visible but parent synchronization failed: {error}"
                    ),
                )
                .for_workspace(workspace)
            })?;
            Ok(ProjectInitReceipt {
                contract_version: PROJECT_CONTRACT_VERSION,
                workspace,
                revision: summary.revision,
                snapshot: summary.hash,
                revision_record: record_digest,
                locator: locator_facts(&root)?,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
            if created_root {
                let _ = fs::remove_dir(&root);
            }
        }
        result
    }

    /// Opens an explicit project root or discovers exactly one marker above the current directory.
    pub fn open(explicit: Option<&Path>) -> Result<Self> {
        let root = match explicit {
            Some(path) => explicit_project_root(path)?,
            None => discover_project_root(&std::env::current_dir()?)?,
        };
        let project_directory = root.join(PROJECT_DIRECTORY_NAME);
        reject_regular_directory(&project_directory, "semantic project directory")?;
        let marker_path = project_directory.join(PROJECT_MARKER_NAME);
        let workspace = read_marker(&marker_path)?;
        let repository = project_directory.join(PROJECT_REPOSITORY_NAME);
        reject_regular_directory(&repository, "semantic project repository")?;
        let ids = crate::persistence::list_workspace_ids(&repository)?;
        if ids.as_slice() != [workspace] {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "project marker and repository workspace identity disagree",
            )
            .for_workspace(workspace));
        }
        let engine = Engine::open(&repository)?;
        let selected = engine.workspace(workspace)?;
        if selected.id() != workspace {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "opened project authority has a foreign workspace identity",
            )
            .for_workspace(workspace));
        }
        Ok(Self {
            root,
            repository,
            workspace,
            engine,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn repository_path(&self) -> &Path {
        &self.repository
    }

    pub const fn workspace(&self) -> WorkspaceId {
        self.workspace
    }

    pub fn current_revision(&self) -> Result<Revision> {
        Ok(self.engine.workspace(self.workspace)?.head()?.revision())
    }

    pub fn status(&self) -> Result<ProjectStatus> {
        let workspace = self.engine.workspace(self.workspace)?;
        let snapshot = workspace.head()?;
        let record = workspace.record(snapshot.revision())?;
        Ok(ProjectStatus {
            contract_version: PROJECT_CONTRACT_VERSION,
            workspace: self.workspace,
            revision: snapshot.revision(),
            snapshot: snapshot.hash(),
            revision_record: record.digest,
            health: ProjectHealth::Healthy,
            summary: crate::query::workspace_summary(snapshot),
            target_count: u64::try_from(crate::target::summaries(snapshot).len())
                .unwrap_or(u64::MAX),
        })
    }

    pub fn orient(&self, known_digest: Option<&str>) -> Result<ProjectOrientationResult> {
        let workspace = self.engine.workspace(self.workspace)?;
        let snapshot = workspace.head()?;
        let record = workspace.record(snapshot.revision())?;
        let machine_schema = crate::machine::active_machine_schema_digest()?;
        let targets = crate::target::summaries(snapshot);
        let commands = vec![
            "orient",
            "status",
            "inspect",
            "query",
            "proposal",
            "context",
            "change.validate",
            "change.apply",
            "log",
            "show",
            "diff",
            "restore",
            "target.list",
            "target.show",
            "target.build",
            "target.test",
            "target.run",
            "doctor",
            "backup",
        ];
        let omissions = vec![
            "full_graph",
            "full_machine_schema",
            "function_bodies",
            "revision_history",
            "derived_artifact_bytes",
        ];
        let digest = orientation_digest(&OrientationDigestInput {
            workspace: self.workspace,
            revision: snapshot.revision(),
            snapshot: snapshot.hash(),
            record: record.digest,
            schema: machine_schema,
            targets: &targets,
            commands: &commands,
            omissions: &omissions,
        })?;
        if known_digest == Some(digest.as_str()) {
            return Ok(ProjectOrientationResult::Unchanged {
                contract_version: PROJECT_CONTRACT_VERSION,
                workspace: self.workspace,
                revision: snapshot.revision(),
                orientation_digest: digest,
            });
        }
        Ok(ProjectOrientationResult::Changed(ProjectOrientation {
            contract_version: PROJECT_CONTRACT_VERSION,
            workspace: self.workspace,
            revision: snapshot.revision(),
            snapshot: snapshot.hash(),
            revision_record: record.digest,
            machine_schema,
            orientation_digest: digest,
            targets,
            commands,
            omissions,
        }))
    }

    pub fn history(&self, before: Option<Revision>, limit: u32) -> Result<HistoryPage> {
        self.engine
            .workspace(self.workspace)?
            .history_page(before, limit)
    }

    pub fn revision_record(&self, revision: Revision) -> Result<RevisionRecordInspection> {
        Ok(self
            .engine
            .workspace(self.workspace)?
            .record(revision)?
            .clone())
    }

    pub fn restore(
        &mut self,
        source_revision: Revision,
        validate_only: bool,
    ) -> Result<crate::history::RestorationReceipt> {
        self.engine
            .workspace_mut(self.workspace)?
            .restore(source_revision, validate_only)
    }

    pub fn diff(
        &self,
        from: Revision,
        to: Revision,
        offset: u64,
        limit: u32,
    ) -> Result<ProjectDiffPage> {
        if limit == 0 || limit > crate::query::MAX_PAGE_ITEMS {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "project diff limit must be within the public page policy",
            )
            .for_workspace(self.workspace));
        }
        let workspace = self.engine.workspace(self.workspace)?;
        let before = workspace.snapshot(from)?;
        let after = workspace.snapshot(to)?;
        let semantic = crate::history::semantic_diff(before, after);
        let start = usize::try_from(offset).map_err(|_| {
            LkError::new(
                ErrorCode::InvalidCursor,
                "project diff offset is not representable",
            )
        })?;
        if start > semantic.changes.len() {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "project diff offset is beyond the exact result",
            ));
        }
        let end = start
            .saturating_add(limit as usize)
            .min(semantic.changes.len());
        let next_offset = (end < semantic.changes.len()).then_some(end as u64);
        Ok(ProjectDiffPage {
            contract_version: PROJECT_CONTRACT_VERSION,
            workspace: self.workspace,
            from,
            to,
            before_snapshot: before.hash(),
            after_snapshot: after.hash(),
            digest: semantic.digest,
            total: semantic.change_count(),
            offset,
            changes: semantic.changes[start..end].to_vec(),
            next_offset,
        })
    }

    pub fn inspect(
        &self,
        selector: &str,
        revision: Option<Revision>,
        expand: bool,
    ) -> Result<ProjectNodeInspection> {
        let workspace = self.engine.workspace(self.workspace)?;
        let selected_revision = revision.unwrap_or(workspace.head()?.revision());
        let snapshot = workspace.snapshot(selected_revision)?;
        let resolved = resolve_node(snapshot, selector, None)?;
        Ok(ProjectNodeInspection {
            contract_version: PROJECT_CONTRACT_VERSION,
            selector: selector.to_owned(),
            resolved,
            qualified_name: qualified_name(snapshot, resolved)?,
            view: crate::query::node_view(snapshot, resolved, expand)?,
        })
    }

    pub fn semantic_query(
        &self,
        projection: SemanticProjection,
        selectors: &[String],
        revision: Option<Revision>,
        limit: u32,
        continuation: Option<String>,
        known_digest: Option<&str>,
    ) -> Result<SemanticQueryResult> {
        let workspace = self.engine.workspace(self.workspace)?;
        let selected_revision = revision.unwrap_or(workspace.head()?.revision());
        let snapshot = workspace.snapshot(selected_revision)?;
        let roots = selectors
            .iter()
            .map(|selector| resolve_node(snapshot, selector, None))
            .collect::<Result<Vec<_>>>()?;
        let request = SemanticQueryRequest {
            workspace: self.workspace,
            revision: selected_revision,
            projection,
            roots,
            limit,
            continuation,
        };
        crate::workbench::build_semantic_query(snapshot, &request, known_digest)
    }

    pub fn function_proposal(
        &self,
        selector: &str,
        revision: Option<Revision>,
    ) -> Result<ProjectFunctionProposal> {
        let workspace = self.engine.workspace(self.workspace)?;
        let selected_revision = revision.unwrap_or(workspace.head()?.revision());
        let snapshot = workspace.snapshot(selected_revision)?;
        let function = resolve_node(snapshot, selector, None)?;
        if !matches!(snapshot.node(function)?, Node::Function { .. }) {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "function proposal selector does not name a function",
            )
            .for_node(function));
        }
        let bytes = crate::workbench::render_function_document(snapshot, function)?;
        let document = String::from_utf8(bytes).map_err(|_| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "function proposal renderer produced invalid UTF-8",
            )
            .for_node(function)
        })?;
        let mut hasher = blake3::Hasher::new_derive_key("lkjscript.function-proposal.v1");
        hasher.update(document.as_bytes());
        let receipt = ProjectFunctionProposal {
            contract_version: PROJECT_CONTRACT_VERSION,
            document_version: crate::workbench::EDIT_DOCUMENT_VERSION,
            workspace: self.workspace,
            revision: selected_revision,
            snapshot: snapshot.hash(),
            function,
            qualified_name: qualified_name(snapshot, function)?,
            document_digest: crate::release::hex(hasher.finalize().as_bytes()),
            document,
        };
        preflight_project_response(&receipt, "project function proposal")?;
        Ok(receipt)
    }

    pub fn context(
        &mut self,
        purpose: ContextPurpose,
        selectors: &[String],
        revision: Option<Revision>,
        from_revision: Option<Revision>,
        maximum_nodes: Option<u32>,
        known_digest: Option<ContextPacketDigest>,
    ) -> Result<ProjectContextResult> {
        let selected_revision = revision.unwrap_or(self.current_revision()?);
        let targets = {
            let snapshot = self
                .engine
                .workspace(self.workspace)?
                .snapshot(selected_revision)?;
            selectors
                .iter()
                .map(|selector| resolve_node(snapshot, selector, None))
                .collect::<Result<Vec<_>>>()?
        };
        let mut request = ContextBuildRequest::new(self.workspace, selected_revision, purpose);
        request.targets = targets;
        request.from_revision = from_revision;
        if let Some(maximum_nodes) = maximum_nodes {
            request.maximum_nodes = maximum_nodes;
        }
        let packet = crate::workbench::build_context_packet(&mut self.engine, &request)?;
        if known_digest == Some(packet.digest) {
            Ok(ProjectContextResult::Unchanged {
                contract_version: PROJECT_CONTRACT_VERSION,
                workspace: self.workspace,
                revision: selected_revision,
                digest: packet.digest,
            })
        } else {
            Ok(ProjectContextResult::Changed(Box::new(packet)))
        }
    }

    pub fn apply_change(
        &mut self,
        request: &ProjectChangeRequest,
        mode: TransactionMode,
    ) -> Result<ProjectChangeReceipt> {
        validate_change_request(request, mode)?;
        if request.workspace != self.workspace {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "project change workspace does not match the selected semantic project",
            ));
        }
        let transaction = Transaction {
            workspace: self.workspace,
            base_revision: request.base_revision,
            idempotency_key: request.idempotency_key,
            mode,
            operations: request.operations.clone(),
        };
        let engine_request = ApplyTransactionRequest {
            transaction,
            response: request.response.clone(),
        };
        let head = self.current_revision()?;
        let preview = if request.base_revision == head {
            Some(
                self.engine
                    .workspace(self.workspace)?
                    .prepare_transaction(&engine_request)?,
            )
        } else {
            None
        };
        if let Some(preview) = &preview {
            let workspace = self.engine.workspace(self.workspace)?;
            let before = workspace.snapshot(request.base_revision)?;
            let semantic_diff = crate::history::semantic_diff(before, &preview.snapshot);
            let revision_record = if mode == TransactionMode::Commit {
                let parent_record = workspace.record(request.base_revision)?.digest;
                let fingerprint = crate::machine::transaction_fingerprint(&engine_request)?;
                let record = crate::history::RevisionRecord::transition(
                    before,
                    &preview.snapshot,
                    parent_record,
                    crate::ChangeDigest::from_bytes(fingerprint),
                    crate::history::RevisionPublicationOutcome::Accepted,
                )?;
                Some(RevisionRecordInspection {
                    digest: crate::history::digest(&record)?,
                    record,
                })
            } else {
                None
            };
            let continuation =
                project_change_continuation(&preview.receipt, revision_record.as_ref())?;
            let predicted = ProjectChangeReceipt {
                contract_version: PROJECT_CONTRACT_VERSION,
                transaction: preview.receipt.clone(),
                semantic_diff,
                revision_record,
                continuation,
            };
            preflight_project_response(&predicted, "project change receipt")?;
        }
        let response = self
            .engine
            .request(RequestId::new(1), Request::ApplyTransaction(engine_request))?;
        let receipt = match response {
            Response::TransactionReceipt(receipt) => receipt,
            Response::Error(error) => return Err(error),
            _ => {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "project change returned an unexpected engine response",
                ));
            }
        };
        let workspace = self.engine.workspace(self.workspace)?;
        let before = workspace.snapshot(receipt.base_revision)?;
        let semantic_diff = if let Some(preview) = preview {
            crate::history::semantic_diff(before, &preview.snapshot)
        } else {
            let after = workspace.snapshot(receipt.revision)?;
            crate::history::semantic_diff(before, after)
        };
        let revision_record = receipt
            .published
            .then(|| workspace.record(receipt.revision).cloned())
            .transpose()?;
        let continuation = project_change_continuation(&receipt, revision_record.as_ref())?;
        Ok(ProjectChangeReceipt {
            contract_version: PROJECT_CONTRACT_VERSION,
            transaction: receipt,
            semantic_diff,
            revision_record,
            continuation,
        })
    }

    pub fn list_targets(&self, revision: Option<Revision>) -> Result<Vec<TargetSummary>> {
        let workspace = self.engine.workspace(self.workspace)?;
        let revision = revision.unwrap_or(workspace.head()?.revision());
        Ok(crate::target::summaries(workspace.snapshot(revision)?))
    }

    pub fn show_target(
        &self,
        selector: &str,
        revision: Option<Revision>,
    ) -> Result<TargetInspection> {
        let workspace = self.engine.workspace(self.workspace)?;
        let revision = revision.unwrap_or(workspace.head()?.revision());
        let snapshot = workspace.snapshot(revision)?;
        let target = resolve_node(snapshot, selector, Some(BuildTargetKind::ALL.as_slice()))?;
        let Node::BuildTarget {
            name, definition, ..
        } = snapshot.node(target)?
        else {
            return Err(
                LkError::new(ErrorCode::WrongKind, "selected node is not a build target")
                    .for_node(target),
            );
        };
        Ok(TargetInspection {
            contract_version: PROJECT_CONTRACT_VERSION,
            workspace: self.workspace,
            revision,
            target,
            name: name.clone(),
            kind: definition.kind(),
            definition_digest: crate::target::definition_digest(definition)?,
            definition: definition.clone(),
        })
    }

    pub fn build_target(
        &self,
        selector: &str,
        revision: Option<Revision>,
        output: &Path,
    ) -> Result<ProjectTargetReceipt> {
        self.prepare_target_receipt(selector, revision, Some(output))
    }

    pub fn test_target(
        &self,
        selector: &str,
        revision: Option<Revision>,
    ) -> Result<ProjectTargetReceipt> {
        self.prepare_target_receipt(selector, revision, None)
    }

    pub fn run_target(
        &self,
        selector: &str,
        revision: Option<Revision>,
        invocation: &ApplicationInvocation,
    ) -> Result<ApplicationRunReceipt> {
        let workspace = self.engine.workspace(self.workspace)?;
        let revision = revision.unwrap_or(workspace.head()?.revision());
        let snapshot = workspace.snapshot(revision)?;
        let target = resolve_node(snapshot, selector, Some(BuildTargetKind::ALL.as_slice()))?;
        match crate::target::prepare(snapshot, target)? {
            PreparedTarget::Application(application) => {
                crate::application::run(application.bytes(), invocation)
            }
            PreparedTarget::Release(_) => Err(LkError::new(
                ErrorCode::WrongKind,
                "target run requires an application or product target",
            )
            .for_node(target)),
        }
    }

    pub fn doctor(&self, deep: bool) -> Result<ProjectStatus> {
        if deep {
            self.engine.workspace(self.workspace)?.deep_verify()?;
        }
        self.status()
    }

    pub fn backup(&self, destination: &Path) -> Result<ProjectBackupReceipt> {
        reject_lexical_parent(destination)?;
        let destination = absolute_path(destination)?;
        if destination.starts_with(&self.root) || self.root.starts_with(&destination) {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "project backup destination cannot contain or be contained by the source project",
            ));
        }
        match fs::symlink_metadata(&destination) {
            Ok(_) => {
                return Err(LkError::new(
                    ErrorCode::WorkspaceExists,
                    "project backup destination already exists",
                ));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let parent = destination.parent().ok_or_else(|| {
            LkError::new(ErrorCode::Io, "project backup destination has no parent")
        })?;
        reject_directory_chain(parent)?;
        let serial = PROJECT_TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
        let staging = parent.join(format!(".lkjscript.backup-{}-{serial}", std::process::id()));
        create_private_directory(&staging)?;
        let result = (|| -> Result<ProjectBackupReceipt> {
            let (files, bytes) = copy_authority_tree(
                &self.root.join(PROJECT_DIRECTORY_NAME),
                &staging.join(PROJECT_DIRECTORY_NAME),
            )?;
            let verified = Project::open(Some(&staging))?;
            let status = verified.status()?;
            let source = self.status()?;
            if status.workspace != source.workspace
                || status.revision != source.revision
                || status.snapshot != source.snapshot
                || status.revision_record != source.revision_record
            {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "independently opened backup disagrees with source authority",
                )
                .for_workspace(self.workspace));
            }
            drop(verified);
            sync_directory(&staging)?;
            fs::rename(&staging, &destination)?;
            sync_directory(parent).map_err(|error| {
                LkError::new(
                    ErrorCode::ArtifactPublicationOutcomeUnknown,
                    format!(
                        "project backup became visible but parent synchronization failed: {error}"
                    ),
                )
                .for_workspace(self.workspace)
            })?;
            Ok(ProjectBackupReceipt {
                contract_version: PROJECT_CONTRACT_VERSION,
                workspace: source.workspace,
                revision: source.revision,
                snapshot: source.snapshot,
                revision_record: source.revision_record,
                files,
                bytes,
                destination: locator_facts(&destination)?,
            })
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(&staging);
        }
        result
    }

    fn prepare_target_receipt(
        &self,
        selector: &str,
        revision: Option<Revision>,
        output: Option<&Path>,
    ) -> Result<ProjectTargetReceipt> {
        let output = output.map(absolute_path).transpose()?;
        let workspace = self.engine.workspace(self.workspace)?;
        let revision = revision.unwrap_or(workspace.head()?.revision());
        let snapshot = workspace.snapshot(revision)?;
        let target = resolve_node(snapshot, selector, Some(BuildTargetKind::ALL.as_slice()))?;
        let Node::BuildTarget { name, .. } = snapshot.node(target)? else {
            return Err(
                LkError::new(ErrorCode::WrongKind, "selected node is not a build target")
                    .for_node(target),
            );
        };
        let prepared = crate::target::prepare(snapshot, target)?;
        let artifact_bytes = u64::try_from(prepared.bytes().len()).unwrap_or(u64::MAX);
        let artifact_digest = artifact_digest(prepared.bytes());
        let artifact = match &prepared {
            PreparedTarget::Release(release) => {
                TargetArtifactReceipt::Release(Box::new(release.receipt(output.is_some())))
            }
            PreparedTarget::Application(application) => {
                TargetArtifactReceipt::Application(Box::new(application.receipt(output.is_some())))
            }
        };
        let receipt = ProjectTargetReceipt {
            contract_version: PROJECT_CONTRACT_VERSION,
            workspace: self.workspace,
            revision,
            snapshot: snapshot.hash(),
            target,
            target_name: name.clone(),
            artifact_bytes,
            artifact_digest,
            published: output.is_some(),
            artifact,
        };
        preflight_project_response(&receipt, "project target receipt")?;
        if let Some(path) = output.as_deref() {
            prepared.publish(path)?;
        }
        Ok(receipt)
    }
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ProjectChangeContinuationDigestInput<'a> {
    version: u16,
    workspace: WorkspaceId,
    revision: Revision,
    snapshot: SnapshotHash,
    revision_record: RevisionRecordDigest,
    accepted_change_set: crate::ChangeDigest,
    semantic_diff: crate::ChangeDigest,
    returned_bindings: &'a [(crate::DraftSymbol, NodeId)],
    changed_functions: &'a [NodeId],
    affected_targets: &'a [NodeId],
    session_local_aliases_invalidated: bool,
}

fn project_change_continuation(
    receipt: &TransactionReceipt,
    record: Option<&RevisionRecordInspection>,
) -> Result<Option<ProjectChangeContinuation>> {
    if !receipt.published {
        if record.is_some() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "unpublished project change unexpectedly has a revision record",
            ));
        }
        return Ok(None);
    }
    let record = record.ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "published project change is missing its revision record",
        )
    })?;
    if record.record.workspace != receipt.workspace
        || record.record.revision != receipt.revision
        || record.record.result_snapshot != receipt.hash
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "project change receipt and revision record disagree",
        )
        .for_workspace(receipt.workspace)
        .at_revision(receipt.revision));
    }
    let input = ProjectChangeContinuationDigestInput {
        version: PROJECT_CHANGE_CONTINUATION_VERSION,
        workspace: receipt.workspace,
        revision: receipt.revision,
        snapshot: receipt.hash,
        revision_record: record.digest,
        accepted_change_set: record.record.accepted_change_set,
        semantic_diff: record.record.semantic_diff,
        returned_bindings: &receipt.returned_bindings,
        changed_functions: &record.record.function_bodies_changed,
        affected_targets: &record.record.affected_targets,
        session_local_aliases_invalidated: true,
    };
    let encoded = serde_json::to_vec(&input).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("cannot encode project change continuation: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.project-change-continuation.v1");
    hasher.update(&encoded);
    let continuation = ProjectChangeContinuation {
        version: input.version,
        workspace: input.workspace,
        revision: input.revision,
        snapshot: input.snapshot,
        revision_record: input.revision_record,
        accepted_change_set: input.accepted_change_set,
        semantic_diff: input.semantic_diff,
        returned_bindings: input.returned_bindings.to_vec(),
        changed_functions: input.changed_functions.to_vec(),
        affected_targets: input.affected_targets.to_vec(),
        session_local_aliases_invalidated: input.session_local_aliases_invalidated,
        digest: crate::release::hex(hasher.finalize().as_bytes()),
    };
    let encoded = serde_json::to_vec(&continuation).map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("cannot encode project change continuation: {error}"),
        )
    })?;
    if encoded.len() > MAXIMUM_PROJECT_CHANGE_CONTINUATION_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "project change continuation exceeds the response byte policy",
        )
        .for_workspace(receipt.workspace)
        .at_revision(receipt.revision));
    }
    Ok(Some(continuation))
}

pub fn decode_change(bytes: &[u8]) -> Result<ProjectChangeRequest> {
    if bytes.len() > MAXIMUM_PROJECT_CHANGE_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "project change JSON exceeds input byte policy",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let request = ProjectChangeRequest::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("project change JSON is malformed: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("project change JSON has trailing input: {error}"),
        )
    })?;
    if request.version != PROJECT_CHANGE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "project change version is unsupported",
        ));
    }
    Ok(request)
}

fn validate_change_request(request: &ProjectChangeRequest, mode: TransactionMode) -> Result<()> {
    if request.version != PROJECT_CHANGE_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "project change version is unsupported",
        ));
    }
    if mode == TransactionMode::ValidateOnly && request.idempotency_key.is_some() {
        return Err(LkError::new(
            ErrorCode::InvalidOperand,
            "validate-only project changes cannot carry an idempotency key",
        ));
    }
    Ok(())
}

fn artifact_digest(bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.project-target-output.v1");
    hasher.update(bytes);
    crate::release::hex(hasher.finalize().as_bytes())
}

fn preflight_project_response(value: &impl Serialize, label: &str) -> Result<()> {
    let maximum = crate::machine::MAX_JSON_OUTPUT_BYTES.saturating_sub(1024);
    for encoded in [serde_json::to_vec(value), serde_json::to_vec_pretty(value)] {
        let encoded = encoded.map_err(|error| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                format!("{label} cannot be encoded: {error}"),
            )
        })?;
        if encoded.len() > maximum {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                format!("{label} exceeds response byte policy"),
            ));
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct OrientationDigestInput<'a> {
    workspace: WorkspaceId,
    revision: Revision,
    snapshot: SnapshotHash,
    record: RevisionRecordDigest,
    schema: MachineSchemaDigest,
    targets: &'a [TargetSummary],
    commands: &'a [&'a str],
    omissions: &'a [&'a str],
}

fn orientation_digest(input: &OrientationDigestInput<'_>) -> Result<String> {
    let value = serde_json::to_vec(&(PROJECT_CONTRACT_VERSION, input)).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("cannot encode project orientation: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(PROJECT_ORIENTATION_DIGEST_DOMAIN);
    hasher.update(&value);
    Ok(crate::release::hex(hasher.finalize().as_bytes()))
}

fn resolve_node(
    snapshot: &crate::graph::Snapshot,
    selector: &str,
    target_kinds: Option<&[BuildTargetKind]>,
) -> Result<NodeId> {
    if let Ok(id) = selector.parse::<NodeId>() {
        let node = snapshot.node(id)?;
        if let Some(kinds) = target_kinds {
            let Node::BuildTarget { definition, .. } = node else {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "exact selector does not name a build target",
                )
                .for_node(id));
            };
            if !kinds.contains(&definition.kind()) {
                return Err(LkError::new(
                    ErrorCode::WrongKind,
                    "exact selector names the wrong build-target kind",
                )
                .for_node(id));
            }
        }
        return Ok(id);
    }
    let mut matches = Vec::new();
    for (id, node) in snapshot.nodes().filter(|(id, _)| id.is_durable()) {
        if let Some(kinds) = target_kinds {
            let Node::BuildTarget { definition, .. } = node else {
                continue;
            };
            if !kinds.contains(&definition.kind()) {
                continue;
            }
        }
        let simple = node.name();
        let qualified = qualified_name(snapshot, id)?;
        if simple == Some(selector) || qualified == selector {
            matches.push(id);
        }
    }
    match matches.as_slice() {
        [id] => Ok(*id),
        [] => Err(LkError::new(
            ErrorCode::NodeNotFound,
            "selector does not resolve at the selected revision",
        )
        .for_workspace(snapshot.workspace())
        .at_revision(snapshot.revision())),
        _ => Err(LkError::new(
            ErrorCode::InvalidQuery,
            "selector is ambiguous at the selected revision",
        )
        .for_workspace(snapshot.workspace())
        .at_revision(snapshot.revision())
        .with_related(matches)),
    }
}

fn qualified_name(snapshot: &crate::graph::Snapshot, id: NodeId) -> Result<String> {
    let mut names = Vec::new();
    let mut current = Some(id);
    let mut seen = BTreeSet::new();
    while let Some(selected) = current {
        if !seen.insert(selected) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "semantic owner chain contains a cycle",
            )
            .for_node(selected));
        }
        let node = snapshot.node(selected)?;
        if let Some(name) = node.name() {
            names.push(name.to_owned());
        }
        current = node.owner();
    }
    names.reverse();
    if names.is_empty() {
        Ok(id.to_string())
    } else {
        Ok(names.join("::"))
    }
}

fn prepare_initialization_root(path: &Path) -> Result<(PathBuf, bool)> {
    reject_lexical_parent(path)?;
    let absolute = absolute_path(path)?;
    match fs::symlink_metadata(&absolute) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "semantic project root must be a non-symlink directory",
                ));
            }
            reject_directory_chain(&absolute)?;
            Ok((absolute, false))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let parent = absolute.parent().ok_or_else(|| {
                LkError::new(ErrorCode::Io, "semantic project root has no parent")
            })?;
            reject_regular_directory(parent, "semantic project parent")?;
            fs::create_dir(&absolute)?;
            fs::set_permissions(&absolute, fs::Permissions::from_mode(0o700))?;
            sync_directory(parent)?;
            Ok((absolute, true))
        }
        Err(error) => Err(error.into()),
    }
}

fn explicit_project_root(path: &Path) -> Result<PathBuf> {
    reject_lexical_parent(path)?;
    let absolute = absolute_path(path)?;
    reject_directory_chain(&absolute)?;
    reject_regular_directory(&absolute, "explicit semantic project root")?;
    let marker = absolute
        .join(PROJECT_DIRECTORY_NAME)
        .join(PROJECT_MARKER_NAME);
    match fs::symlink_metadata(&marker) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "project marker is not a regular file",
            ))
        }
        Ok(_) => Ok(absolute),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(LkError::new(
            ErrorCode::WorkspaceNotFound,
            "explicit directory is not a semantic project root",
        )),
        Err(error) => Err(error.into()),
    }
}

fn discover_project_root(start: &Path) -> Result<PathBuf> {
    let start = absolute_path(start)?;
    reject_directory_chain(&start)?;
    let mut current = Some(start.as_path());
    let mut found = Vec::new();
    let mut depth = 0_usize;
    while let Some(directory) = current {
        if depth > MAXIMUM_DISCOVERY_DEPTH {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "project discovery exceeded parent traversal policy",
            ));
        }
        let marker = directory
            .join(PROJECT_DIRECTORY_NAME)
            .join(PROJECT_MARKER_NAME);
        match fs::symlink_metadata(&marker) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "project discovery found a nonregular marker",
                ));
            }
            Ok(_) => found.push(directory.to_owned()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        current = directory.parent();
        depth = depth.saturating_add(1);
    }
    match found.as_slice() {
        [root] => Ok(root.clone()),
        [] => Err(LkError::new(
            ErrorCode::WorkspaceNotFound,
            "no semantic project marker exists within the discovery boundary",
        )),
        _ => Err(LkError::new(
            ErrorCode::InvalidContainment,
            "project discovery found ambiguous nested semantic authorities",
        )),
    }
}

fn locator_facts(root: &Path) -> Result<ProjectLocatorFacts> {
    let root = path_string(root)?;
    Ok(ProjectLocatorFacts {
        project_directory: format!("{root}/{PROJECT_DIRECTORY_NAME}"),
        repository_directory: format!("{root}/{PROJECT_DIRECTORY_NAME}/{PROJECT_REPOSITORY_NAME}"),
        root,
    })
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    let base = std::env::current_dir()?;
    let mut result = if path.is_absolute() {
        PathBuf::from("/")
    } else {
        base
    };
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::CurDir => {}
            Component::Normal(value) => result.push(value),
            Component::ParentDir => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "project paths cannot contain parent traversal",
                ));
            }
            Component::Prefix(_) => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "project path uses an unsupported prefix",
                ));
            }
        }
    }
    if result.as_os_str().as_bytes().len() > crate::artifact_io::MAXIMUM_ARTIFACT_PATH_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "project path exceeds byte policy",
        ));
    }
    Ok(result)
}

fn reject_lexical_parent(path: &Path) -> Result<()> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(LkError::new(
            ErrorCode::Io,
            "project paths cannot contain parent traversal",
        ));
    }
    Ok(())
}

fn reject_directory_chain(path: &Path) -> Result<()> {
    let mut current = PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir | Component::CurDir => continue,
            Component::Normal(part) => current.push(part),
            Component::ParentDir | Component::Prefix(_) => {
                return Err(LkError::new(ErrorCode::Io, "project path is not canonical"));
            }
        }
        let metadata = fs::symlink_metadata(&current)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(LkError::new(
                ErrorCode::Io,
                "project path components must be non-symlink directories",
            ));
        }
    }
    Ok(())
}

fn reject_regular_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::Io,
            format!("cannot inspect {label} {}: {error}", path.display()),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LkError::new(
            ErrorCode::Io,
            format!("{label} must be a non-symlink directory"),
        ));
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn write_marker(path: &Path, workspace: WorkspaceId) -> Result<()> {
    let bytes = encode_marker(workspace);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(&bytes)?;
    file.sync_all()?;
    Ok(())
}

fn read_marker(path: &Path) -> Result<WorkspaceId> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("cannot inspect semantic project marker: {error}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "semantic project marker must be a regular non-symlink file",
        ));
    }
    if metadata.len() > MAXIMUM_PROJECT_MARKER_BYTES as u64 {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic project marker exceeds byte policy",
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)?
        .take(MAXIMUM_PROJECT_MARKER_BYTES as u64 + 1)
        .read_to_end(&mut bytes)?;
    decode_marker(&bytes)
}

fn encode_marker(workspace: WorkspaceId) -> Vec<u8> {
    let mut body = Vec::with_capacity(8 + 2 + WorkspaceId::BYTE_LEN);
    body.extend_from_slice(&PROJECT_MARKER_MAGIC);
    body.extend_from_slice(&PROJECT_CONTRACT_VERSION.to_le_bytes());
    body.extend_from_slice(&workspace.as_bytes());
    let mut hasher = blake3::Hasher::new_derive_key(PROJECT_MARKER_CHECKSUM_DOMAIN);
    hasher.update(&body);
    body.extend_from_slice(hasher.finalize().as_bytes());
    body
}

fn decode_marker(bytes: &[u8]) -> Result<WorkspaceId> {
    let body_len = PROJECT_MARKER_MAGIC.len() + 2 + WorkspaceId::BYTE_LEN;
    let expected_len = body_len + 32;
    if bytes.len() != expected_len {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "semantic project marker length is invalid",
        ));
    }
    let (body, checksum) = bytes.split_at(body_len);
    let mut hasher = blake3::Hasher::new_derive_key(PROJECT_MARKER_CHECKSUM_DOMAIN);
    hasher.update(body);
    if hasher.finalize().as_bytes() != checksum {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "semantic project marker checksum is invalid",
        ));
    }
    if body[..PROJECT_MARKER_MAGIC.len()] != PROJECT_MARKER_MAGIC {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "semantic project marker magic is unsupported",
        ));
    }
    let version_start = PROJECT_MARKER_MAGIC.len();
    let version = u16::from_le_bytes([body[version_start], body[version_start + 1]]);
    if version != PROJECT_CONTRACT_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "semantic project marker version is unsupported",
        ));
    }
    let mut workspace = [0_u8; WorkspaceId::BYTE_LEN];
    workspace.copy_from_slice(&body[version_start + 2..body_len]);
    Ok(WorkspaceId::from_bytes(workspace))
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn copy_authority_tree(source: &Path, destination: &Path) -> Result<(u64, u64)> {
    reject_regular_directory(source, "project authority source")?;
    create_private_directory(destination)?;
    let mut pending = VecDeque::from([(source.to_owned(), destination.to_owned(), 0_usize)]);
    let mut directories = vec![destination.to_owned()];
    let mut files = 0_u64;
    let mut bytes = 0_u64;
    while let Some((source_directory, destination_directory, depth)) = pending.pop_front() {
        if depth > 16 {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "project backup authority depth exceeds policy",
            ));
        }
        let mut entries = fs::read_dir(&source_directory)?.collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry.file_name();
            if source_directory.ends_with(PROJECT_REPOSITORY_NAME)
                && name == "lkjscript.engine.lock"
            {
                continue;
            }
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "project backup source contains a symlink",
                ));
            }
            let destination_entry = destination_directory.join(&name);
            if file_type.is_dir() {
                create_private_directory(&destination_entry)?;
                directories.push(destination_entry.clone());
                pending.push_back((entry.path(), destination_entry, depth.saturating_add(1)));
                continue;
            }
            if !file_type.is_file() {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "project backup source contains a nonregular authority entry",
                ));
            }
            let length = entry.metadata()?.len();
            files = files.checked_add(1).ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "project backup file count overflowed",
                )
            })?;
            bytes = bytes.checked_add(length).ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "project backup byte count overflowed",
                )
            })?;
            if files > MAXIMUM_BACKUP_FILES || bytes > MAXIMUM_BACKUP_BYTES {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "project backup exceeds retained file or byte policy",
                ));
            }
            let mut input = File::open(entry.path())?;
            let mut output = OpenOptions::new()
                .create_new(true)
                .write(true)
                .mode(0o600)
                .open(&destination_entry)?;
            let copied = std::io::copy(&mut input, &mut output)?;
            if copied != length {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "project backup source changed during bounded copy",
                ));
            }
            output.sync_all()?;
        }
    }
    for directory in directories.into_iter().rev() {
        sync_directory(&directory)?;
    }
    Ok((files, bytes))
}

fn path_string(path: &Path) -> Result<String> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        LkError::new(
            ErrorCode::Io,
            "semantic project paths must be valid UTF-8 in this contract",
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn marker_is_canonical_and_direct_predecessors_reject() {
        let workspace = WorkspaceId::from_bytes([0x41; WorkspaceId::BYTE_LEN]);
        let bytes = encode_marker(workspace);
        assert_eq!(decode_marker(&bytes).expect("marker"), workspace);
        let mut old = bytes.clone();
        old[..8].copy_from_slice(b"LKJPROJ0");
        assert_eq!(
            decode_marker(&old).expect_err("old marker").code,
            ErrorCode::ArtifactCorrupt
        );
        let mut trailing = bytes;
        trailing.push(0);
        assert_eq!(
            decode_marker(&trailing).expect_err("trailing marker").code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn init_discovery_and_nested_ambiguity_are_exact() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let outer = temporary.path().join("outer");
        fs::create_dir(&outer).expect("outer");
        let receipt = Project::initialize(&outer).expect("initialize outer");
        let nested = outer.join("a/b");
        fs::create_dir_all(&nested).expect("nested");
        let project = Project::open(Some(&outer)).expect("explicit project");
        assert_eq!(project.workspace(), receipt.workspace);
        drop(project);

        assert_eq!(
            discover_project_root(&nested).expect("discover outer"),
            outer
        );

        Project::initialize(&outer.join("a")).expect("initialize nested");
        let error = match discover_project_root(&nested) {
            Ok(_) => panic!("nested discovery must be ambiguous"),
            Err(error) => error,
        };
        assert_eq!(error.code, ErrorCode::InvalidContainment);
    }

    #[test]
    fn project_paths_markers_and_current_authority_fail_closed() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary root");
        let root = temporary.path().join("project");
        fs::create_dir(&root).expect("project root");
        let initialized = Project::initialize(&root).expect("initialize");
        assert_eq!(
            Project::initialize(&root)
                .expect_err("repeat initialization")
                .code,
            ErrorCode::WorkspaceExists
        );

        let alias = temporary.path().join("project-alias");
        symlink(&root, &alias).expect("project symlink");
        assert_eq!(
            match Project::open(Some(&alias)) {
                Ok(_) => panic!("symlinked project root must reject"),
                Err(error) => error.code,
            },
            ErrorCode::Io
        );

        let marker = root.join(PROJECT_DIRECTORY_NAME).join(PROJECT_MARKER_NAME);
        let marker_bytes = fs::read(&marker).expect("marker bytes");
        fs::remove_file(&marker).expect("remove marker");
        symlink("project-target", &marker).expect("symlink marker");
        assert_eq!(
            match Project::open(Some(&root)) {
                Ok(_) => panic!("symlinked marker must reject"),
                Err(error) => error.code,
            },
            ErrorCode::ArtifactCorrupt
        );
        fs::remove_file(&marker).expect("remove marker symlink");
        fs::write(&marker, &marker_bytes).expect("restore marker");

        let foreign_root = temporary.path().join("foreign");
        fs::create_dir(&foreign_root).expect("foreign root");
        Project::initialize(&foreign_root).expect("initialize foreign");
        let foreign_marker = foreign_root
            .join(PROJECT_DIRECTORY_NAME)
            .join(PROJECT_MARKER_NAME);
        fs::write(&marker, fs::read(&foreign_marker).expect("foreign marker"))
            .expect("replace marker");
        assert_eq!(
            match Project::open(Some(&root)) {
                Ok(_) => panic!("foreign marker must reject"),
                Err(error) => error.code,
            },
            ErrorCode::WrongWorkspace
        );
        fs::write(&marker, &marker_bytes).expect("restore marker identity");

        let head = root
            .join(PROJECT_DIRECTORY_NAME)
            .join(PROJECT_REPOSITORY_NAME)
            .join("workspaces")
            .join(initialized.workspace.to_string())
            .join("HEAD");
        let mut head_bytes = fs::read(&head).expect("HEAD bytes");
        head_bytes[0] ^= 0xff;
        fs::write(&head, head_bytes).expect("corrupt HEAD");
        assert_eq!(
            match Project::open(Some(&root)) {
                Ok(_) => panic!("corrupt current authority must reject"),
                Err(error) => error.code,
            },
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn project_change_automatically_records_one_revision() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = temporary.path().join("project");
        fs::create_dir(&root).expect("project root");
        let initialized = Project::initialize(&root).expect("initialize");
        let mut project = Project::open(Some(&root)).expect("open");
        let request = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: initialized.workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            operations: vec![TransactionOp::CreatePackage {
                symbol: crate::DraftSymbol::new("package"),
                name: "package".into(),
            }],
            response: TransactionResponseSpec {
                return_symbols: vec![crate::DraftSymbol::new("package")],
            },
        };
        let validated = project
            .apply_change(&request, TransactionMode::ValidateOnly)
            .expect("validate");
        assert!(!validated.transaction.published);
        assert_eq!(
            project.current_revision().expect("revision"),
            Revision::INITIAL
        );
        let applied = project
            .apply_change(&request, TransactionMode::Commit)
            .expect("apply");
        assert!(applied.transaction.published);
        assert_eq!(applied.transaction.revision, Revision::new(1));
        assert!(applied.revision_record.is_some());
        assert_eq!(project.history(None, 10).expect("history").records.len(), 2);
        drop(project);
        let reopened = Project::open(Some(&root)).expect("reopen");
        assert_eq!(reopened.workspace(), initialized.workspace);
        assert_eq!(
            reopened.current_revision().expect("revision"),
            Revision::new(1)
        );
    }

    #[test]
    fn project_change_binds_workspace_and_replays_without_a_second_revision() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = temporary.path().join("project");
        fs::create_dir(&root).expect("project root");
        let initialized = Project::initialize(&root).expect("initialize");
        let mut project = Project::open(Some(&root)).expect("open");
        let mut request = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: WorkspaceId::from_bytes([0x51; WorkspaceId::BYTE_LEN]),
            base_revision: Revision::INITIAL,
            idempotency_key: Some(IdempotencyKey::from_bytes([0x42; 16])),
            operations: vec![TransactionOp::CreatePackage {
                symbol: crate::DraftSymbol::new("package"),
                name: "package".into(),
            }],
            response: TransactionResponseSpec {
                return_symbols: vec![crate::DraftSymbol::new("package")],
            },
        };
        assert_eq!(
            project
                .apply_change(&request, TransactionMode::Commit)
                .expect_err("foreign change")
                .code,
            ErrorCode::WrongWorkspace
        );
        assert_eq!(
            project.current_revision().expect("unchanged revision"),
            Revision::INITIAL
        );

        request.workspace = initialized.workspace;
        let first = project
            .apply_change(&request, TransactionMode::Commit)
            .expect("first apply");
        assert_eq!(first.transaction.revision, Revision::new(1));
        let replay = project
            .apply_change(&request, TransactionMode::Commit)
            .expect("idempotent replay");
        assert_eq!(replay.transaction, first.transaction);
        assert_eq!(replay.semantic_diff, first.semantic_diff);
        assert_eq!(
            project.current_revision().expect("single revision"),
            Revision::new(1)
        );
        drop(project);

        let mut reopened = Project::open(Some(&root)).expect("reopen");
        let restarted_replay = reopened
            .apply_change(&request, TransactionMode::Commit)
            .expect("restart replay");
        assert_eq!(restarted_replay.transaction, first.transaction);
        assert_eq!(
            reopened.history(None, 10).expect("history").records.len(),
            2
        );
    }

    #[test]
    fn context_known_digest_is_exact_and_no_change_publishes_nothing() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = temporary.path().join("project");
        fs::create_dir(&root).expect("project root");
        let initialized = Project::initialize(&root).expect("initialize");
        let mut project = Project::open(Some(&root)).expect("open");
        let first = project
            .context(ContextPurpose::Orient, &[], None, None, None, None)
            .expect("context");
        let ProjectContextResult::Changed(packet) = first else {
            panic!("first context must be complete");
        };
        assert!(matches!(
            project
                .context(
                    ContextPurpose::Orient,
                    &[],
                    None,
                    None,
                    None,
                    Some(packet.digest),
                )
                .expect("unchanged context"),
            ProjectContextResult::Unchanged { digest, .. } if digest == packet.digest
        ));

        let create = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: initialized.workspace,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            operations: vec![TransactionOp::CreatePackage {
                symbol: crate::DraftSymbol::new("package"),
                name: "package".into(),
            }],
            response: TransactionResponseSpec {
                return_symbols: vec![crate::DraftSymbol::new("package")],
            },
        };
        let created = project
            .apply_change(&create, TransactionMode::Commit)
            .expect("create package");
        let package = created.transaction.returned_bindings[0].1;
        let request = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: initialized.workspace,
            base_revision: Revision::new(1),
            idempotency_key: None,
            operations: vec![TransactionOp::RenameNode {
                node: crate::NodeTarget::Existing(package),
                name: "package".into(),
            }],
            response: TransactionResponseSpec::default(),
        };
        let error = project
            .apply_change(&request, TransactionMode::Commit)
            .expect_err("semantic no-change");
        assert_eq!(error.code, ErrorCode::NoChange);
        assert_eq!(
            project.current_revision().expect("revision remains one"),
            Revision::new(1)
        );
        assert_eq!(project.history(None, 10).expect("history").records.len(), 2);
    }

    #[test]
    fn restoration_publishes_forward_and_refuses_identity_resurrection() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = temporary.path().join("project");
        fs::create_dir(&root).expect("project root");
        Project::initialize(&root).expect("initialize");
        let mut project = Project::open(Some(&root)).expect("open");
        let create = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: project.workspace(),
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            operations: vec![TransactionOp::CreatePackage {
                symbol: crate::DraftSymbol::new("package"),
                name: "before".into(),
            }],
            response: TransactionResponseSpec {
                return_symbols: vec![crate::DraftSymbol::new("package")],
            },
        };
        let created = project
            .apply_change(&create, TransactionMode::Commit)
            .expect("create package");
        let package = created.transaction.returned_bindings[0].1;
        let rename = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: project.workspace(),
            base_revision: Revision::new(1),
            idempotency_key: None,
            operations: vec![TransactionOp::RenameNode {
                node: crate::NodeTarget::Existing(package),
                name: "after".into(),
            }],
            response: TransactionResponseSpec::default(),
        };
        project
            .apply_change(&rename, TransactionMode::Commit)
            .expect("rename package");
        let preview = project
            .restore(Revision::new(1), true)
            .expect("preview restore");
        assert!(!preview.published);
        assert_eq!(
            project.current_revision().expect("still revision two"),
            Revision::new(2)
        );
        let restored = project.restore(Revision::new(1), false).expect("restore");
        assert!(restored.published);
        assert_eq!(restored.revision, Revision::new(3));
        assert_eq!(
            project
                .inspect(&package.to_string(), None, true)
                .expect("restored package")
                .view
                .record
                .and_then(|node| node.name().map(str::to_owned)),
            Some("before".into())
        );

        let delete = ProjectChangeRequest {
            version: PROJECT_CHANGE_VERSION,
            workspace: project.workspace(),
            base_revision: Revision::new(3),
            idempotency_key: None,
            operations: vec![TransactionOp::DeleteOwnedSubtree {
                root: crate::NodeTarget::Existing(package),
            }],
            response: TransactionResponseSpec::default(),
        };
        project
            .apply_change(&delete, TransactionMode::Commit)
            .expect("delete package");
        assert_eq!(
            project
                .restore(Revision::new(3), false)
                .expect_err("restoration must not resurrect package")
                .code,
            ErrorCode::DeleteBlocked
        );
        assert_eq!(
            project.current_revision().expect("revision four"),
            Revision::new(4)
        );
    }

    #[test]
    fn backup_opens_as_the_same_path_independent_authority() {
        let temporary = tempfile::tempdir().expect("temporary root");
        let root = temporary.path().join("source");
        fs::create_dir(&root).expect("source root");
        let initialized = Project::initialize(&root).expect("initialize");
        let project = Project::open(Some(&root)).expect("source project");
        let destination = temporary.path().join("backup");
        let receipt = project.backup(&destination).expect("backup");
        assert_eq!(receipt.workspace, initialized.workspace);
        drop(project);
        let backup = Project::open(Some(&destination)).expect("open backup");
        let status = backup.doctor(true).expect("deep backup verification");
        assert_eq!(status.workspace, initialized.workspace);
        assert_eq!(status.revision, Revision::INITIAL);
    }
}
