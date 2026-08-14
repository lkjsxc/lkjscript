use crate::ids::{NodeId, Revision, WorkspaceId};
use crate::schema::{NodeKind, SemanticType};
use serde::{Deserialize, Serialize};
use std::fmt;

pub type Result<T> = std::result::Result<T, LkError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    ArtifactCorrupt,
    CommitOutcomeUnknown,
    CompileIncomplete,
    CoreIrInvalid,
    DeleteBlocked,
    DuplicateHandle,
    DuplicateName,
    IdempotencyConflict,
    InvalidContainment,
    InvalidHandle,
    InvalidOperand,
    InvalidCursor,
    InvalidQuery,
    Io,
    NodeNotFound,
    NoChange,
    OwnerMismatch,
    PolicyExceeded,
    ProtocolMalformed,
    ProtocolVersion,
    RevisionConflict,
    RevisionNotFound,
    RuntimeTrap,
    TypeMismatch,
    WorkspaceExists,
    WorkspaceNotFound,
    WrongKind,
    WrongWorkspace,
}

impl ErrorCode {
    pub const ALL: [Self; 28] = [
        Self::ArtifactCorrupt,
        Self::CompileIncomplete,
        Self::CoreIrInvalid,
        Self::DeleteBlocked,
        Self::DuplicateHandle,
        Self::DuplicateName,
        Self::IdempotencyConflict,
        Self::InvalidContainment,
        Self::InvalidHandle,
        Self::InvalidOperand,
        Self::Io,
        Self::NodeNotFound,
        Self::NoChange,
        Self::OwnerMismatch,
        Self::PolicyExceeded,
        Self::ProtocolMalformed,
        Self::ProtocolVersion,
        Self::RevisionConflict,
        Self::RevisionNotFound,
        Self::RuntimeTrap,
        Self::TypeMismatch,
        Self::WorkspaceExists,
        Self::WorkspaceNotFound,
        Self::WrongKind,
        Self::WrongWorkspace,
        Self::CommitOutcomeUnknown,
        Self::InvalidCursor,
        Self::InvalidQuery,
    ];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::ArtifactCorrupt => 1,
            Self::CompileIncomplete => 2,
            Self::CoreIrInvalid => 3,
            Self::DeleteBlocked => 4,
            Self::DuplicateHandle => 5,
            Self::DuplicateName => 6,
            Self::IdempotencyConflict => 7,
            Self::InvalidContainment => 8,
            Self::InvalidHandle => 9,
            Self::InvalidOperand => 10,
            Self::Io => 11,
            Self::NodeNotFound => 12,
            Self::NoChange => 13,
            Self::OwnerMismatch => 14,
            Self::PolicyExceeded => 15,
            Self::ProtocolMalformed => 16,
            Self::ProtocolVersion => 17,
            Self::RevisionConflict => 18,
            Self::RevisionNotFound => 19,
            Self::RuntimeTrap => 20,
            Self::TypeMismatch => 21,
            Self::WorkspaceExists => 22,
            Self::WorkspaceNotFound => 23,
            Self::WrongKind => 24,
            Self::WrongWorkspace => 25,
            Self::CommitOutcomeUnknown => 26,
            Self::InvalidCursor => 27,
            Self::InvalidQuery => 28,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        if tag >= 1 && tag <= 28 {
            Some(Self::ALL[(tag - 1) as usize])
        } else {
            None
        }
    }
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::ArtifactCorrupt => "artifact_corrupt",
            Self::CommitOutcomeUnknown => "commit_outcome_unknown",
            Self::CompileIncomplete => "compile_incomplete",
            Self::CoreIrInvalid => "core_ir_invalid",
            Self::DeleteBlocked => "delete_blocked",
            Self::DuplicateHandle => "duplicate_handle",
            Self::DuplicateName => "duplicate_name",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::InvalidContainment => "invalid_containment",
            Self::InvalidHandle => "invalid_handle",
            Self::InvalidOperand => "invalid_operand",
            Self::InvalidCursor => "invalid_cursor",
            Self::InvalidQuery => "invalid_query",
            Self::Io => "io",
            Self::NodeNotFound => "node_not_found",
            Self::NoChange => "no_change",
            Self::OwnerMismatch => "owner_mismatch",
            Self::PolicyExceeded => "policy_exceeded",
            Self::ProtocolMalformed => "protocol_malformed",
            Self::ProtocolVersion => "protocol_version",
            Self::RevisionConflict => "revision_conflict",
            Self::RevisionNotFound => "revision_not_found",
            Self::RuntimeTrap => "runtime_trap",
            Self::TypeMismatch => "type_mismatch",
            Self::WorkspaceExists => "workspace_exists",
            Self::WorkspaceNotFound => "workspace_not_found",
            Self::WrongKind => "wrong_kind",
            Self::WrongWorkspace => "wrong_workspace",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LkError {
    pub code: ErrorCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<WorkspaceId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<Revision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation_index: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_kind: Option<NodeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_kind: Option<NodeKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<SemanticType>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actual_type: Option<SemanticType>,
    pub related: Vec<NodeId>,
    pub retryable: bool,
    pub message: String,
}

impl LkError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            workspace: None,
            revision: None,
            operation_index: None,
            target: None,
            expected_kind: None,
            actual_kind: None,
            expected_type: None,
            actual_type: None,
            related: Vec::new(),
            retryable: false,
            message: message.into(),
        }
    }

    pub fn at_operation(mut self, operation_index: usize) -> Self {
        self.operation_index = u32::try_from(operation_index).ok();
        self
    }

    pub fn for_workspace(mut self, workspace: WorkspaceId) -> Self {
        self.workspace = Some(workspace);
        self
    }

    pub fn at_revision(mut self, revision: Revision) -> Self {
        self.revision = Some(revision);
        self
    }

    pub fn for_node(mut self, node: NodeId) -> Self {
        self.target = Some(node);
        self
    }

    pub fn with_kinds(mut self, expected: NodeKind, actual: NodeKind) -> Self {
        self.expected_kind = Some(expected);
        self.actual_kind = Some(actual);
        self
    }

    pub fn with_types(mut self, expected: SemanticType, actual: SemanticType) -> Self {
        self.expected_type = Some(expected);
        self.actual_type = Some(actual);
        self
    }

    pub fn with_related(mut self, related: impl IntoIterator<Item = NodeId>) -> Self {
        self.related = related.into_iter().collect();
        self.related.sort();
        self.related.dedup();
        self
    }
}

impl fmt::Display for LkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for LkError {}

impl From<std::io::Error> for LkError {
    fn from(error: std::io::Error) -> Self {
        Self::new(ErrorCode::Io, error.to_string())
    }
}
