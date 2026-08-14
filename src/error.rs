use crate::ids::{NodeId, Revision, WorkspaceId};
use crate::schema::{NodeKind, SemanticType};
use std::fmt;

pub type Result<T> = std::result::Result<T, LkError>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LkError {
    pub code: ErrorCode,
    pub workspace: Option<WorkspaceId>,
    pub revision: Option<Revision>,
    pub operation_index: Option<u32>,
    pub target: Option<NodeId>,
    pub expected_kind: Option<NodeKind>,
    pub actual_kind: Option<NodeKind>,
    pub expected_type: Option<SemanticType>,
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
