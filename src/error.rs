use crate::ids::{DraftSymbol, NodeId, Revision, WorkspaceId};
use crate::schema::{NodeKind, SemanticType};
use serde::{Deserialize, Serialize};
use std::fmt;

pub type Result<T> = std::result::Result<T, LkError>;

/// Maximum semantic identities carried inline by any diagnostic. Exact larger detail belongs in
/// revision-bound paginated queries.
pub const MAX_ERROR_RELATED_IDS: usize = 64;
pub const MAX_DRAFT_PATH_BYTES: usize = 256;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    AuthorityBusy,
    ArtifactCorrupt,
    CommitOutcomeUnknown,
    CompileIncomplete,
    CoreIrInvalid,
    DeleteBlocked,
    DuplicateDraftSymbol,
    DuplicateName,
    IdempotencyConflict,
    InvalidContainment,
    InvalidDraftSymbol,
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
    RunArgumentMismatch,
    ExecutionFuelExhausted,
    ExecutionFrameExhausted,
    TypeMismatch,
    WorkspaceExists,
    WorkspaceNotFound,
    WrongKind,
    WrongWorkspace,
    ByValueTypeCycle,
    TypeLayoutUnrepresentable,
    ByteLiteralTooLarge,
    RuntimeByteInputTooLarge,
    ManagedObjectPolicyExceeded,
    ManagedVisibleBytePolicyExceeded,
    RetainedBytePolicyExceeded,
    ByteIndexOutOfBounds,
    ByteSliceOutOfBounds,
    InvalidManagedHandle,
    ResultBytePolicyExceeded,
    ByteValueTooLarge,
    ExecutionMemoryExhausted,
    OwnershipPlanInvalid,
}

impl ErrorCode {
    pub const ALL: [Self; 46] = [
        Self::AuthorityBusy,
        Self::ArtifactCorrupt,
        Self::CompileIncomplete,
        Self::CoreIrInvalid,
        Self::DeleteBlocked,
        Self::DuplicateDraftSymbol,
        Self::DuplicateName,
        Self::IdempotencyConflict,
        Self::InvalidContainment,
        Self::InvalidDraftSymbol,
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
        Self::RunArgumentMismatch,
        Self::ExecutionFuelExhausted,
        Self::ExecutionFrameExhausted,
        Self::ByValueTypeCycle,
        Self::TypeLayoutUnrepresentable,
        Self::ByteLiteralTooLarge,
        Self::RuntimeByteInputTooLarge,
        Self::ManagedObjectPolicyExceeded,
        Self::ManagedVisibleBytePolicyExceeded,
        Self::RetainedBytePolicyExceeded,
        Self::ByteIndexOutOfBounds,
        Self::ByteSliceOutOfBounds,
        Self::InvalidManagedHandle,
        Self::ResultBytePolicyExceeded,
        Self::ByteValueTooLarge,
        Self::ExecutionMemoryExhausted,
        Self::OwnershipPlanInvalid,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::AuthorityBusy => "authority_busy",
            Self::ArtifactCorrupt => "artifact_corrupt",
            Self::CommitOutcomeUnknown => "commit_outcome_unknown",
            Self::CompileIncomplete => "compile_incomplete",
            Self::CoreIrInvalid => "core_ir_invalid",
            Self::DeleteBlocked => "delete_blocked",
            Self::DuplicateDraftSymbol => "duplicate_draft_symbol",
            Self::DuplicateName => "duplicate_name",
            Self::IdempotencyConflict => "idempotency_conflict",
            Self::InvalidContainment => "invalid_containment",
            Self::InvalidDraftSymbol => "invalid_draft_symbol",
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
            Self::RunArgumentMismatch => "run_argument_mismatch",
            Self::ExecutionFuelExhausted => "execution_fuel_exhausted",
            Self::ExecutionFrameExhausted => "execution_frame_exhausted",
            Self::TypeMismatch => "type_mismatch",
            Self::WorkspaceExists => "workspace_exists",
            Self::WorkspaceNotFound => "workspace_not_found",
            Self::WrongKind => "wrong_kind",
            Self::WrongWorkspace => "wrong_workspace",
            Self::ByValueTypeCycle => "by_value_type_cycle",
            Self::TypeLayoutUnrepresentable => "type_layout_unrepresentable",
            Self::ByteLiteralTooLarge => "byte_literal_too_large",
            Self::RuntimeByteInputTooLarge => "runtime_byte_input_too_large",
            Self::ManagedObjectPolicyExceeded => "managed_object_policy_exceeded",
            Self::ManagedVisibleBytePolicyExceeded => "managed_visible_byte_policy_exceeded",
            Self::RetainedBytePolicyExceeded => "retained_byte_policy_exceeded",
            Self::ByteIndexOutOfBounds => "byte_index_out_of_bounds",
            Self::ByteSliceOutOfBounds => "byte_slice_out_of_bounds",
            Self::InvalidManagedHandle => "invalid_managed_handle",
            Self::ResultBytePolicyExceeded => "result_byte_policy_exceeded",
            Self::ByteValueTooLarge => "byte_value_too_large",
            Self::ExecutionMemoryExhausted => "execution_memory_exhausted",
            Self::OwnershipPlanInvalid => "ownership_plan_invalid",
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
    pub draft_symbol: Option<DraftSymbol>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub draft_path: Option<String>,
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
    pub related: Box<[NodeId]>,
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
            draft_symbol: None,
            draft_path: None,
            target: None,
            expected_kind: None,
            actual_kind: None,
            expected_type: None,
            actual_type: None,
            related: Vec::new().into_boxed_slice(),
            retryable: false,
            message: message.into(),
        }
    }

    pub fn at_operation(mut self, operation_index: usize) -> Self {
        self.operation_index = u32::try_from(operation_index).ok();
        self
    }

    pub fn for_symbol(mut self, symbol: DraftSymbol) -> Self {
        self.draft_symbol = Some(symbol);
        self
    }

    pub fn at_draft_path(mut self, path: impl Into<String>) -> Self {
        let path = path.into();
        debug_assert!(path.len() <= MAX_DRAFT_PATH_BYTES);
        self.draft_path = Some(path);
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
        let mut bounded = Vec::with_capacity(MAX_ERROR_RELATED_IDS);
        for id in related {
            match bounded.binary_search(&id) {
                Ok(_) => {}
                Err(index) if index < MAX_ERROR_RELATED_IDS => {
                    if bounded.len() == MAX_ERROR_RELATED_IDS {
                        bounded.pop();
                    }
                    bounded.insert(index, id);
                }
                Err(_) => {}
            }
        }
        self.related = bounded.into_boxed_slice();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn related_ids_are_sorted_deduplicated_and_allocation_bounded() {
        let workspace = WorkspaceId::from_bytes([7; 16]);
        let error = LkError::new(ErrorCode::CompileIncomplete, "many").with_related(
            (1..=200).rev().flat_map(|serial| {
                let id = NodeId::new(workspace, serial).expect("node");
                [id, id]
            }),
        );
        assert_eq!(error.related.len(), MAX_ERROR_RELATED_IDS);
        assert!(error.related.windows(2).all(|ids| ids[0] < ids[1]));
        assert_eq!(error.related[0].serial(), 1);
        assert_eq!(error.related[MAX_ERROR_RELATED_IDS - 1].serial(), 64);
    }
}
