use crate::diff::{self, Change};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{
    ChangeDigest, NodeId, NodeIdentityClass, QueryId, Revision, SnapshotHash, WorkspaceId,
};
use crate::schema::{
    BlockArgumentRole, ByteString, DirectReference, LiteralField, Node, NodeKind, OperandUse,
    OperationCode, OperationKind, RegionRole, SemanticType, TypeReferenceSlot, TypeRule, ValueRef,
};
use crate::transaction::TransactionOpCode;
use crate::type_layout::{DerivedLayout, LayoutShape};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const MAX_PAGE_ITEMS: u32 = 256;
pub const MAX_BATCH_QUERIES: usize = 32;
pub const MAX_BATCH_ITEMS: u32 = 2048;
pub const MAX_CONTEXT_ITEMS: u32 = 64;
const NAME_PREVIEW_BYTES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedCategory {
    EntryFunction,
    FunctionBody,
    Expression,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CompletenessBlocker {
    pub owner: NodeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target: Option<NodeId>,
    pub category: ExpectedCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_type: Option<SemanticType>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceSummary {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub hash: SnapshotHash,
    pub root: NodeId,
    pub node_count: u64,
    pub durable_identity_count: u64,
    pub function_local_reference_count: u64,
    pub anchor_count: u64,
    pub tombstone_count: u64,
    pub complete: bool,
    pub blocker_count: u64,
    pub entry_count: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FunctionSignatureSummary {
    pub parameter_count: u64,
    pub result: SemanticType,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NamePreview {
    pub value: String,
    pub truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeSummary {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub node: NodeId,
    pub identity_class: NodeIdentityClass,
    pub kind: NodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<NamePreview>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature: Option<FunctionSignatureSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_type: Option<SemanticType>,
    pub complete: bool,
    pub blocker_count: u64,
    pub child_count: u64,
    pub outgoing_reference_count: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NodeView {
    pub summary: NodeSummary,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub record: Option<Node>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RepairTarget {
    Hole(NodeId),
    Operand { operation: NodeId, index: u64 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PageRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<PageCursor>,
    pub limit: u32,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VisibleCursorPurpose {
    VisibleValues,
    LegalConstructors,
    RepairContext,
}
impl VisibleCursorPurpose {
    pub const ALL: [Self; 3] = [
        Self::VisibleValues,
        Self::LegalConstructors,
        Self::RepairContext,
    ];
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum PageCursor {
    Blockers {
        workspace: WorkspaceId,
        revision: Revision,
        next: u64,
    },
    OwnerChain {
        workspace: WorkspaceId,
        revision: Revision,
        node: NodeId,
        next: u64,
    },
    Body {
        workspace: WorkspaceId,
        revision: Revision,
        block: NodeId,
        next: u64,
    },
    IncomingUses {
        workspace: WorkspaceId,
        revision: Revision,
        value: ValueRef,
        next: u64,
    },
    DefinitionReferences {
        workspace: WorkspaceId,
        revision: Revision,
        target: NodeId,
        next: u64,
    },
    Dependencies {
        workspace: WorkspaceId,
        revision: Revision,
        node: NodeId,
        next: u64,
    },
    VisibleValues {
        workspace: WorkspaceId,
        revision: Revision,
        purpose: VisibleCursorPurpose,
        target: RepairTarget,
        expected: SemanticType,
        include_incompatible: bool,
        next: u64,
    },
    LegalConstructors {
        workspace: WorkspaceId,
        revision: Revision,
        target: RepairTarget,
        expected: SemanticType,
        next: u64,
    },
    Diff {
        workspace: WorkspaceId,
        from: Revision,
        to: Revision,
        next: u64,
    },
    NominalType {
        workspace: WorkspaceId,
        revision: Revision,
        declaration: NodeId,
        next: u64,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Page<T> {
    pub items: Vec<T>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next: Option<PageCursor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum LiteralValue {
    I64(i64),
    Bool(bool),
    ExpectedType(SemanticType),
    Bytes(ByteString),
    Text(crate::schema::TextString),
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BodyItem {
    pub operation: NodeId,
    pub ordinal: u64,
    pub code: OperationCode,
    pub result_types: Vec<SemanticType>,
    pub operands: Vec<ValueRef>,
    pub definitions: Vec<DefinitionReferenceSite>,
    pub complete: bool,
    pub terminator: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub literal: Option<LiteralValue>,
    pub owned_regions: Vec<OwnedRegionSummary>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnedRegionSummary {
    pub region: NodeId,
    pub role: RegionRole,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BlockArgumentFact {
    pub argument: NodeId,
    pub block: NodeId,
    pub region: NodeId,
    pub ordinal: u32,
    pub role: BlockArgumentRole,
    pub ty: SemanticType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EnclosingRegionFact {
    pub region: NodeId,
    pub owner_operation: NodeId,
    pub role: RegionRole,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerFact {
    pub node: NodeId,
    pub kind: NodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<NamePreview>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct UseSite {
    pub source: NodeId,
    pub operand_index: u64,
    pub target: ValueRef,
    pub owner_block: NodeId,
    pub owner_function: NodeId,
    pub expected_type: SemanticType,
    pub use_mode: OperandUse,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DefinitionSlot {
    PackageEntry,
    CallTarget,
    FunctionResultType,
    ParameterType,
    ProductFieldType,
    SumVariantPayloadType,
    SequenceElementType,
    BlockArgumentType,
    OperationType,
    ProductDeclaration,
    ProductField,
    SumVariant,
    MatchVariant,
    SequenceDeclaration,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DefinitionReferenceSite {
    pub source: NodeId,
    pub slot: DefinitionSlot,
    pub target: NodeId,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum DependencyFact {
    ValueOperand {
        index: u64,
        value: ValueRef,
    },
    Definition {
        slot: DefinitionSlot,
        target: NodeId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct VisibleValue {
    pub value: ValueRef,
    pub ty: SemanticType,
    pub compatible: bool,
    pub producer: NodeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub producer_code: Option<OperationCode>,
    pub owner_function: NodeId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ordinal: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<NamePreview>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ConstructorDescriptor {
    pub code: OperationCode,
    pub result_type: SemanticType,
    pub operand_count: u64,
    pub operand_types: Vec<SemanticType>,
    pub operand_uses: Vec<OperandUse>,
    pub literal_fields: Vec<LiteralField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_target: Option<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declaration: Option<NodeId>,
    pub member_count: u64,
    pub members: Vec<NodeId>,
    pub requirements_complete: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_type_continuation: Option<NominalTypeContinuation>,
    pub direct_refinement: bool,
    pub complete: bool,
    pub terminator: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NominalTypeContinuation {
    pub declaration: NodeId,
    pub page: PageRequest,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NominalLayoutSummary {
    pub representable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failure: Option<crate::type_layout::LayoutFailure>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub align: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub discriminant_bytes: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_offset: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum NominalMemberFact {
    ProductField {
        field: NodeId,
        name: String,
        ordinal: u32,
        ty: SemanticType,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        offset: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cells: Option<u64>,
    },
    SumVariant {
        variant: NodeId,
        name: String,
        ordinal: u32,
        payload: Option<SemanticType>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        discriminant: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_size: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_align: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        payload_cells: Option<u64>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct NominalTypeResult {
    pub declaration: NodeId,
    pub name: String,
    pub kind: NodeKind,
    pub owner: NodeId,
    pub layout: NominalLayoutSummary,
    pub members: Page<NominalMemberFact>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LegalConstructorsResult {
    pub target: RepairTarget,
    pub expected_type: SemanticType,
    pub constructors: Page<ConstructorDescriptor>,
    pub visible_values: Page<VisibleValue>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextBudget {
    pub body_before: u32,
    pub body_after: u32,
    pub visible_values: u32,
    pub incoming_uses: u32,
    pub include_incompatible: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RepairContext {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub target: RepairTarget,
    pub operation: NodeId,
    pub operation_code: OperationCode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operand_index: Option<u64>,
    pub expected_type: SemanticType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub use_mode: Option<OperandUse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_value: Option<ValueRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_actual_type: Option<SemanticType>,
    pub owner_block: NodeId,
    pub owner_function: NodeId,
    pub ordinal: u64,
    pub function_signature: FunctionSignatureSummary,
    pub owner_chain: Vec<OwnerFact>,
    pub enclosing_regions: Vec<EnclosingRegionFact>,
    pub visible_block_arguments: Vec<BlockArgumentFact>,
    pub body_window: Vec<BodyItem>,
    pub visible_values: Page<VisibleValue>,
    pub incoming_uses: Page<UseSite>,
    pub legal_constructor_count: u64,
    pub legal_constructors: Vec<ConstructorDescriptor>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_type: Option<NominalTypeResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nominal_type_continuation: Option<NominalTypeContinuation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub blocker: Option<CompletenessBlocker>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub refinement_operation: Option<TransactionOpCode>,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticDiffPage {
    pub from: Revision,
    pub to: Revision,
    pub change_count: u64,
    pub change_digest: ChangeDigest,
    pub page: Page<Change>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryCode {
    WorkspaceSummary,
    Node,
    Blockers,
    OwnerChain,
    Body,
    IncomingUses,
    DefinitionReferences,
    Dependencies,
    VisibleValues,
    LegalConstructors,
    SemanticDiff,
    RepairContext,
    NominalType,
}
impl QueryCode {
    pub const ALL: [Self; 13] = [
        Self::WorkspaceSummary,
        Self::Node,
        Self::Blockers,
        Self::OwnerChain,
        Self::Body,
        Self::IncomingUses,
        Self::DefinitionReferences,
        Self::Dependencies,
        Self::VisibleValues,
        Self::LegalConstructors,
        Self::SemanticDiff,
        Self::RepairContext,
        Self::NominalType,
    ];
    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::WorkspaceSummary => "workspace_summary",
            Self::Node => "node",
            Self::Blockers => "blockers",
            Self::OwnerChain => "owner_chain",
            Self::Body => "body",
            Self::IncomingUses => "incoming_uses",
            Self::DefinitionReferences => "definition_references",
            Self::Dependencies => "dependencies",
            Self::VisibleValues => "visible_values",
            Self::LegalConstructors => "legal_constructors",
            Self::SemanticDiff => "semantic_diff",
            Self::RepairContext => "repair_context",
            Self::NominalType => "nominal_type",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Query {
    WorkspaceSummary,
    Node {
        node: NodeId,
        expand: bool,
    },
    Blockers {
        page: PageRequest,
    },
    OwnerChain {
        node: NodeId,
        page: PageRequest,
    },
    Body {
        block: NodeId,
        page: PageRequest,
    },
    IncomingUses {
        value: ValueRef,
        page: PageRequest,
    },
    DefinitionReferences {
        target: NodeId,
        page: PageRequest,
    },
    Dependencies {
        node: NodeId,
        page: PageRequest,
    },
    VisibleValues {
        purpose: VisibleCursorPurpose,
        target: RepairTarget,
        include_incompatible: bool,
        page: PageRequest,
    },
    LegalConstructors {
        target: RepairTarget,
        include_incompatible: bool,
        constructors: PageRequest,
        values: PageRequest,
    },
    SemanticDiff {
        from: Revision,
        page: PageRequest,
    },
    RepairContext {
        target: RepairTarget,
        budget: ContextBudget,
    },
    NominalType {
        declaration: NodeId,
        page: PageRequest,
    },
}
impl Query {
    pub const fn code(&self) -> QueryCode {
        match self {
            Self::WorkspaceSummary => QueryCode::WorkspaceSummary,
            Self::Node { .. } => QueryCode::Node,
            Self::Blockers { .. } => QueryCode::Blockers,
            Self::OwnerChain { .. } => QueryCode::OwnerChain,
            Self::Body { .. } => QueryCode::Body,
            Self::IncomingUses { .. } => QueryCode::IncomingUses,
            Self::DefinitionReferences { .. } => QueryCode::DefinitionReferences,
            Self::Dependencies { .. } => QueryCode::Dependencies,
            Self::VisibleValues { .. } => QueryCode::VisibleValues,
            Self::LegalConstructors { .. } => QueryCode::LegalConstructors,
            Self::SemanticDiff { .. } => QueryCode::SemanticDiff,
            Self::RepairContext { .. } => QueryCode::RepairContext,
            Self::NominalType { .. } => QueryCode::NominalType,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryItem {
    pub id: QueryId,
    pub query: Query,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryBatchRequest {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub queries: Vec<QueryItem>,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum QueryResult {
    WorkspaceSummary(WorkspaceSummary),
    Node(NodeView),
    Blockers(Page<CompletenessBlocker>),
    OwnerChain(Page<OwnerFact>),
    Body(Page<BodyItem>),
    IncomingUses(Page<UseSite>),
    DefinitionReferences(Page<DefinitionReferenceSite>),
    Dependencies(Page<DependencyFact>),
    VisibleValues(Page<VisibleValue>),
    LegalConstructors(LegalConstructorsResult),
    SemanticDiff(SemanticDiffPage),
    RepairContext(Box<RepairContext>),
    NominalType(NominalTypeResult),
}
// Errors retain their direct public DTO shape; query batches are independently bounded.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum QueryOutcome {
    Success(Box<QueryResult>),
    Error(LkError),
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryItemResult {
    pub id: QueryId,
    pub outcome: QueryOutcome,
}
#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryBatchResult {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub results: Vec<QueryItemResult>,
}

pub fn validate_batch(request: &QueryBatchRequest) -> Result<()> {
    if request.queries.len() > MAX_BATCH_QUERIES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "query batch exceeds query-count policy",
        ));
    }
    let mut ids = BTreeSet::new();
    let mut total = 0u32;
    for item in &request.queries {
        if !ids.insert(item.id) {
            return Err(LkError::new(
                ErrorCode::InvalidQuery,
                "query IDs must be unique",
            ));
        }
        total = total
            .checked_add(query_budget(&item.query)?)
            .ok_or_else(|| LkError::new(ErrorCode::PolicyExceeded, "query item budget overflow"))?;
    }
    if total > MAX_BATCH_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "query batch exceeds aggregate item policy",
        ));
    }
    Ok(())
}
fn query_budget(query: &Query) -> Result<u32> {
    Ok(match query {
        Query::WorkspaceSummary | Query::Node { .. } => 1,
        Query::RepairContext { budget, .. } => {
            validate_context_budget(*budget)?;
            budget
                .body_before
                .checked_add(budget.body_after)
                .and_then(|v| v.checked_add(budget.visible_values))
                .and_then(|v| v.checked_add(budget.incoming_uses))
                .and_then(|v| v.checked_add(1))
                .ok_or_else(|| LkError::new(ErrorCode::PolicyExceeded, "context budget overflow"))?
        }
        Query::LegalConstructors {
            constructors,
            values,
            ..
        } => validate_page(*constructors)?
            .limit
            .checked_add(validate_page(*values)?.limit)
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    "legal constructor budget overflow",
                )
            })?,
        Query::Blockers { page }
        | Query::OwnerChain { page, .. }
        | Query::Body { page, .. }
        | Query::IncomingUses { page, .. }
        | Query::DefinitionReferences { page, .. }
        | Query::Dependencies { page, .. }
        | Query::VisibleValues { page, .. }
        | Query::SemanticDiff { page, .. }
        | Query::NominalType { page, .. } => validate_page(*page)?.limit,
    })
}
fn validate_context_budget(b: ContextBudget) -> Result<()> {
    if [
        b.body_before,
        b.body_after,
        b.visible_values,
        b.incoming_uses,
    ]
    .into_iter()
    .any(|v| v > MAX_CONTEXT_ITEMS)
    {
        Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "repair context exceeds item policy",
        ))
    } else {
        Ok(())
    }
}
fn validate_page(page: PageRequest) -> Result<PageRequest> {
    if page.limit == 0 {
        Err(LkError::new(
            ErrorCode::InvalidQuery,
            "page limit must be nonzero",
        ))
    } else if page.limit > MAX_PAGE_ITEMS {
        Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "page limit exceeds policy",
        ))
    } else {
        Ok(page)
    }
}

pub fn workspace_summary(snapshot: &Snapshot) -> WorkspaceSummary {
    let blockers = workspace_blockers(snapshot);
    let entries = snapshot
        .nodes()
        .filter(|(_, n)| matches!(n, Node::Package { entry: Some(_), .. }))
        .count();
    WorkspaceSummary {
        workspace: snapshot.workspace(),
        revision: snapshot.revision(),
        hash: snapshot.hash(),
        root: snapshot.root(),
        node_count: snapshot.node_count() as u64,
        durable_identity_count: snapshot.durable_identity_count() as u64,
        function_local_reference_count: snapshot.function_local_reference_count() as u64,
        anchor_count: snapshot.anchor_count() as u64,
        tombstone_count: snapshot.tombstones().count() as u64,
        complete: blockers.is_empty(),
        blocker_count: blockers.len() as u64,
        entry_count: entries as u64,
    }
}
pub fn node_view(snapshot: &Snapshot, id: NodeId, expand: bool) -> Result<NodeView> {
    let node = snapshot.node(id)?;
    let blockers = blockers_for_node(snapshot, id);
    let signature = match node {
        Node::Function {
            parameters, result, ..
        } => Some(FunctionSignatureSummary {
            parameter_count: parameters.len() as u64,
            result: *result,
        }),
        _ => None,
    };
    Ok(NodeView {
        summary: NodeSummary {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            node: id,
            identity_class: id.identity_class(),
            kind: node.kind(),
            owner: node.owner(),
            display_name: node.name().map(name_preview),
            signature,
            value_type: node_value_type(snapshot, id, node, 0),
            complete: blockers.is_empty(),
            blocker_count: blockers.len() as u64,
            child_count: node.owned_child_count() as u64,
            outgoing_reference_count: node.direct_reference_count() as u64,
        },
        record: expand.then(|| node.clone()),
    })
}
fn name_preview(value: &str) -> NamePreview {
    if value.len() <= NAME_PREVIEW_BYTES {
        return NamePreview {
            value: value.to_owned(),
            truncated: false,
        };
    }
    let mut end = NAME_PREVIEW_BYTES;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    NamePreview {
        value: value[..end].to_owned(),
        truncated: true,
    }
}

pub fn execute(
    snapshot: &Snapshot,
    query: &Query,
    diff_before: Option<&Snapshot>,
) -> Result<QueryResult> {
    match query {
        Query::WorkspaceSummary => Ok(QueryResult::WorkspaceSummary(workspace_summary(snapshot))),
        Query::Node { node, expand } => Ok(QueryResult::Node(node_view(snapshot, *node, *expand)?)),
        Query::Blockers { page } => Ok(QueryResult::Blockers(page_items(
            workspace_blockers(snapshot),
            *page,
            |next| PageCursor::Blockers {
                workspace: snapshot.workspace(),
                revision: snapshot.revision(),
                next,
            },
            |c| match c {
                PageCursor::Blockers {
                    workspace,
                    revision,
                    next,
                } if workspace == snapshot.workspace() && revision == snapshot.revision() => {
                    Some(next)
                }
                _ => None,
            },
        )?)),
        Query::OwnerChain { node, page } => Ok(QueryResult::OwnerChain(owner_chain_page(
            snapshot, *node, *page,
        )?)),
        Query::Body { block, page } => Ok(QueryResult::Body(body_page(snapshot, *block, *page)?)),
        Query::IncomingUses { value, page } => Ok(QueryResult::IncomingUses(uses_page(
            snapshot, *value, *page,
        )?)),
        Query::DefinitionReferences { target, page } => Ok(QueryResult::DefinitionReferences(
            definition_page(snapshot, *target, *page)?,
        )),
        Query::Dependencies { node, page } => Ok(QueryResult::Dependencies(dependency_page(
            snapshot, *node, *page,
        )?)),
        Query::VisibleValues {
            purpose,
            target,
            include_incompatible,
            page,
        } => {
            let (expected, loc) = target_contract(snapshot, *target)?;
            Ok(QueryResult::VisibleValues(visible_page(
                snapshot,
                *purpose,
                *target,
                expected,
                loc,
                *include_incompatible,
                *page,
            )?))
        }
        Query::LegalConstructors {
            target,
            include_incompatible,
            constructors,
            values,
        } => {
            let (expected, loc) = target_contract(snapshot, *target)?;
            Ok(QueryResult::LegalConstructors(LegalConstructorsResult {
                target: *target,
                expected_type: expected,
                constructors: legal_constructor_page(snapshot, *target, expected, *constructors)?,
                visible_values: visible_page(
                    snapshot,
                    VisibleCursorPurpose::LegalConstructors,
                    *target,
                    expected,
                    loc,
                    *include_incompatible,
                    *values,
                )?,
            }))
        }
        Query::SemanticDiff { from, page } => {
            let before = diff_before.ok_or_else(|| {
                LkError::new(
                    ErrorCode::RevisionNotFound,
                    "diff base revision is unavailable",
                )
                .at_revision(*from)
            })?;
            Ok(QueryResult::SemanticDiff(diff_page(
                before, snapshot, *page,
            )?))
        }
        Query::RepairContext { target, budget } => Ok(QueryResult::RepairContext(Box::new(
            repair_context(snapshot, *target, *budget)?,
        ))),
        Query::NominalType { declaration, page } => Ok(QueryResult::NominalType(
            nominal_type_result(snapshot, *declaration, *page)?,
        )),
    }
}

fn nominal_type_result(
    snapshot: &Snapshot,
    declaration: NodeId,
    page: PageRequest,
) -> Result<NominalTypeResult> {
    let (name, kind, owner, member_ids) = match snapshot.node(declaration)? {
        Node::ProductType {
            owner,
            name,
            fields,
            ..
        } => (
            name.clone(),
            NodeKind::ProductType,
            *owner,
            fields.as_slice(),
        ),
        Node::SumType {
            owner,
            name,
            variants,
            ..
        } => (name.clone(), NodeKind::SumType, *owner, variants.as_slice()),
        Node::SequenceType { owner, name, .. } => (
            name.clone(),
            NodeKind::SequenceType,
            *owner,
            &[] as &[NodeId],
        ),
        node => {
            return Err(LkError::new(
                ErrorCode::WrongKind,
                "nominal type query requires a product, sum, or sequence declaration",
            )
            .for_node(declaration)
            .with_kinds(NodeKind::ProductType, node.kind()));
        }
    };
    let layouts = crate::type_layout::derive_layouts(snapshot)?;
    let layout = layouts.get(&declaration).ok_or_else(|| {
        LkError::new(
            ErrorCode::WrongKind,
            "nominal declaration has no derived layout",
        )
        .for_node(declaration)
    })?;
    let summary = match layout {
        DerivedLayout::Unrepresentable(failure) => NominalLayoutSummary {
            representable: false,
            failure: Some(*failure),
            size: None,
            align: None,
            cells: None,
            discriminant_bytes: None,
            payload_offset: None,
        },
        DerivedLayout::Representable(value) => {
            let (discriminant_bytes, payload_offset) = match &value.shape {
                LayoutShape::Sum {
                    discriminant_bytes,
                    payload_offset,
                    ..
                } => (Some(*discriminant_bytes), Some(*payload_offset)),
                _ => (None, None),
            };
            NominalLayoutSummary {
                representable: true,
                failure: None,
                size: Some(value.size),
                align: Some(value.align),
                cells: Some(value.cells),
                discriminant_bytes,
                payload_offset,
            }
        }
    };
    let mut facts = Vec::with_capacity(member_ids.len());
    match layout {
        DerivedLayout::Representable(value) => match &value.shape {
            LayoutShape::Product { fields: derived } => {
                for (member, layout) in member_ids.iter().zip(derived) {
                    let Node::ProductField {
                        name, ordinal, ty, ..
                    } = snapshot.node(*member)?
                    else {
                        unreachable!()
                    };
                    facts.push(NominalMemberFact::ProductField {
                        field: *member,
                        name: name.clone(),
                        ordinal: *ordinal,
                        ty: *ty,
                        offset: Some(layout.offset),
                        cells: Some(layout.cells),
                    });
                }
            }
            LayoutShape::Sum {
                variants: derived, ..
            } => {
                for (member, layout) in member_ids.iter().zip(derived) {
                    let Node::SumVariant {
                        name,
                        ordinal,
                        payload,
                        ..
                    } = snapshot.node(*member)?
                    else {
                        unreachable!()
                    };
                    facts.push(NominalMemberFact::SumVariant {
                        variant: *member,
                        name: name.clone(),
                        ordinal: *ordinal,
                        payload: *payload,
                        discriminant: Some(layout.discriminant),
                        payload_size: Some(layout.payload_size),
                        payload_align: Some(layout.payload_align),
                        payload_cells: Some(layout.payload_cells),
                    });
                }
            }
            LayoutShape::Primitive => {}
        },
        DerivedLayout::Unrepresentable(_) => {
            for member in member_ids {
                match snapshot.node(*member)? {
                    Node::ProductField {
                        name, ordinal, ty, ..
                    } => facts.push(NominalMemberFact::ProductField {
                        field: *member,
                        name: name.clone(),
                        ordinal: *ordinal,
                        ty: *ty,
                        offset: None,
                        cells: None,
                    }),
                    Node::SumVariant {
                        name,
                        ordinal,
                        payload,
                        ..
                    } => facts.push(NominalMemberFact::SumVariant {
                        variant: *member,
                        name: name.clone(),
                        ordinal: *ordinal,
                        payload: *payload,
                        discriminant: None,
                        payload_size: None,
                        payload_align: None,
                        payload_cells: None,
                    }),
                    _ => unreachable!(),
                }
            }
        }
    }
    let members = page_items(
        facts,
        page,
        |next| PageCursor::NominalType {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            declaration,
            next,
        },
        |cursor| match cursor {
            PageCursor::NominalType {
                workspace,
                revision,
                declaration: cursor_declaration,
                next,
            } if workspace == snapshot.workspace()
                && revision == snapshot.revision()
                && cursor_declaration == declaration =>
            {
                Some(next)
            }
            _ => None,
        },
    )?;
    Ok(NominalTypeResult {
        declaration,
        name,
        kind,
        owner,
        layout: summary,
        members,
    })
}

fn page_items<T, F, G>(items: Vec<T>, page: PageRequest, make: F, read: G) -> Result<Page<T>>
where
    F: Fn(u64) -> PageCursor,
    G: Fn(PageCursor) -> Option<u64>,
{
    let page = validate_page(page)?;
    let start = match page.after {
        None => 0,
        Some(c) => read(c).ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this query",
            )
        })?,
    };
    let len = items.len() as u64;
    if start > len {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "cursor is beyond query result",
        ));
    }
    let end = start.saturating_add(page.limit as u64).min(len);
    let page_items = items
        .into_iter()
        .skip(start as usize)
        .take((end - start) as usize)
        .collect();
    Ok(Page {
        items: page_items,
        next: (end < len).then(|| make(end)),
        total: Some(len),
    })
}

fn body_item(snapshot: &Snapshot, id: NodeId, ordinal: u64, terminator: bool) -> Result<BodyItem> {
    let Node::Operation { operation, .. } = snapshot.node(id)? else {
        return Err(
            LkError::new(ErrorCode::WrongKind, "body item is not an operation").for_node(id),
        );
    };
    let results = (0..operation.result_count())
        .filter_map(|index| crate::graph::operation_result_type(snapshot, id, operation, index))
        .collect();
    let operands = (0..operation.operand_count())
        .filter_map(|i| operation.operand(i))
        .collect();
    let owned_regions = (0..operation.owned_region_count())
        .filter_map(|index| {
            Some(OwnedRegionSummary {
                region: operation.owned_region(index)?,
                role: operation.region_role(operation.owned_region(index)?)?,
            })
        })
        .collect();
    let definitions = (0..operation.definition_target_count())
        .filter_map(|index| operation.definition_target(index))
        .filter_map(|target| {
            operation_definition_slot(operation, target).map(|slot| DefinitionReferenceSite {
                source: id,
                slot,
                target,
            })
        })
        .collect();
    let literal = match operation {
        OperationKind::ConstI64(v) => Some(LiteralValue::I64(*v)),
        OperationKind::ConstBool(v) => Some(LiteralValue::Bool(*v)),
        OperationKind::ConstBytes(v) => Some(LiteralValue::Bytes(v.clone())),
        OperationKind::ConstText(v) => Some(LiteralValue::Text(v.clone())),
        OperationKind::Hole { expected } => Some(LiteralValue::ExpectedType(*expected)),
        _ => None,
    };
    Ok(BodyItem {
        operation: id,
        ordinal,
        code: operation.code(),
        result_types: results,
        operands,
        definitions,
        complete: operation.is_complete(),
        terminator,
        literal,
        owned_regions,
    })
}
fn body_range(snapshot: &Snapshot, block: NodeId, start: u64, end: u64) -> Result<Vec<BodyItem>> {
    let node = snapshot.node(block)?;
    let Node::Block {
        operations,
        terminator,
        ..
    } = node
    else {
        return Err(wrong(block, NodeKind::Block, node.kind()));
    };
    let len = operations.len() as u64 + u64::from(terminator.is_some());
    let mut items = Vec::with_capacity((end.min(len).saturating_sub(start)) as usize);
    for ordinal in start..end.min(len) {
        let (id, is_terminator) = if ordinal < operations.len() as u64 {
            (operations[ordinal as usize], false)
        } else if let Some(terminator) = terminator {
            (*terminator, true)
        } else {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "body ordinal has no operation",
            ));
        };
        items.push(body_item(snapshot, id, ordinal, is_terminator)?);
    }
    Ok(items)
}

fn body_page(snapshot: &Snapshot, block: NodeId, page: PageRequest) -> Result<Page<BodyItem>> {
    let page = validate_page(page)?;
    let node = snapshot.node(block)?;
    let Node::Block {
        operations,
        terminator,
        ..
    } = node
    else {
        return Err(wrong(block, NodeKind::Block, node.kind()));
    };
    let len = operations.len() as u64 + u64::from(terminator.is_some());
    let start = match page.after {
        None => 0,
        Some(PageCursor::Body {
            workspace,
            revision,
            block: b,
            next,
        }) if workspace == snapshot.workspace()
            && revision == snapshot.revision()
            && b == block =>
        {
            next
        }
        Some(_) => {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this query",
            ));
        }
    };
    if start > len {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "cursor is beyond query result",
        ));
    }
    let end = start.saturating_add(page.limit as u64).min(len);
    Ok(Page {
        items: body_range(snapshot, block, start, end)?,
        next: (end < len).then_some(PageCursor::Body {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            block,
            next: end,
        }),
        total: Some(len),
    })
}

#[derive(Clone, Copy)]
struct Location {
    block: NodeId,
    function: NodeId,
    ordinal: u64,
    result: SemanticType,
}
fn operation_location(snapshot: &Snapshot, operation: NodeId) -> Result<Location> {
    let Node::Operation { owner: block, .. } = snapshot.node(operation)? else {
        return Err(wrong(
            operation,
            NodeKind::Operation,
            snapshot.node(operation)?.kind(),
        ));
    };
    let Node::Block {
        operations,
        terminator,
        ..
    } = snapshot.node(*block)?
    else {
        return Err(LkError::new(
            ErrorCode::InvalidContainment,
            "operation owner is not a block",
        ));
    };
    let ordinal = operations
        .iter()
        .position(|id| *id == operation)
        .or_else(|| (*terminator == Some(operation)).then_some(operations.len()))
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::InvalidContainment,
                "operation is absent from owner body",
            )
        })? as u64;
    let function = crate::validate::owner_function_for_block(snapshot, *block)?;
    let Node::Function { result, .. } = snapshot.node(function)? else {
        unreachable!()
    };
    Ok(Location {
        block: *block,
        function,
        ordinal,
        result: *result,
    })
}
fn node_value_type(
    snapshot: &Snapshot,
    id: NodeId,
    node: &Node,
    output: u8,
) -> Option<SemanticType> {
    match node {
        Node::Parameter { ty, .. } | Node::BlockArgument { ty, .. } => (output == 0).then_some(*ty),
        Node::Operation { operation, .. } => {
            crate::graph::operation_result_type(snapshot, id, operation, output as usize)
        }
        _ => None,
    }
}
fn value_type(snapshot: &Snapshot, value: ValueRef) -> Result<SemanticType> {
    match value {
        ValueRef::FunctionParameter(id) => match snapshot.node(id)? {
            Node::Parameter { ty, .. } => Ok(*ty),
            node => Err(wrong(id, NodeKind::Parameter, node.kind())),
        },
        ValueRef::BlockArgument(id) => match snapshot.node(id)? {
            Node::BlockArgument { ty, .. } => Ok(*ty),
            node => Err(wrong(id, NodeKind::BlockArgument, node.kind())),
        },
        ValueRef::OperationResult { operation, output } => match snapshot.node(operation)? {
            Node::Operation {
                operation: kind, ..
            } => crate::graph::operation_result_type(snapshot, operation, kind, output as usize)
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "value output is outside its dynamic contract",
                    )
                    .for_node(operation)
                }),
            node => Err(wrong(operation, NodeKind::Operation, node.kind())),
        },
    }
}

fn operation_use_context(snapshot: &Snapshot, block: NodeId) -> Result<(NodeId, SemanticType)> {
    let function = crate::validate::owner_function_for_block(snapshot, block)?;
    let Node::Function { result, .. } = snapshot.node(function)? else {
        unreachable!()
    };
    Ok((function, *result))
}

fn expected_operand_type(
    snapshot: &Snapshot,
    operation_id: NodeId,
    operation: &OperationKind,
    index: usize,
    function_result: SemanticType,
) -> Result<SemanticType> {
    match operation {
        OperationKind::Call { function, .. } => match snapshot.node(*function)? {
            Node::Function { parameters, .. } => parameters
                .get(index)
                .ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidOperand,
                        "call argument index is outside target signature",
                    )
                    .for_node(operation_id)
                })
                .and_then(|parameter| match snapshot.node(*parameter)? {
                    Node::Parameter { ty, .. } => Ok(*ty),
                    node => Err(wrong(*parameter, NodeKind::Parameter, node.kind())),
                }),
            node => Err(wrong(*function, NodeKind::Function, node.kind())),
        },
        OperationKind::Yield { .. } => {
            let Node::Operation { owner: block, .. } = snapshot.node(operation_id)? else {
                unreachable!()
            };
            let Node::Block { owner: region, .. } = snapshot.node(*block)? else {
                unreachable!()
            };
            let Node::Region { owner, .. } = snapshot.node(*region)? else {
                unreachable!()
            };
            match snapshot.node(*owner)? {
                Node::Operation {
                    operation: OperationKind::If { result, .. },
                    ..
                } => Ok(*result),
                Node::Operation {
                    operation: OperationKind::ForI64 { carried, .. },
                    ..
                } => Ok(*carried),
                Node::Operation {
                    operation: OperationKind::MatchSum { result, .. },
                    ..
                } => Ok(*result),
                _ => Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "yield has no structured owner contract",
                )
                .for_node(operation_id)),
            }
        }
        OperationKind::ConstructProduct { fields, .. } => fields
            .get(index)
            .and_then(|binding| match snapshot.node(binding.field).ok()? {
                Node::ProductField { ty, .. } => Some(*ty),
                _ => None,
            })
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "product operand index is outside field contract",
                )
                .for_node(operation_id)
            }),
        OperationKind::ProjectField { field, .. } => match snapshot.node(*field)? {
            Node::ProductField { owner, .. } if index == 0 => Ok(SemanticType::Nominal(*owner)),
            _ => Err(LkError::new(
                ErrorCode::InvalidOperand,
                "projection operand index is outside field contract",
            )
            .for_node(operation_id)),
        },
        OperationKind::ConstructVariant { variant, .. } => match snapshot.node(*variant)? {
            Node::SumVariant {
                payload: Some(ty), ..
            } if index == 0 => Ok(*ty),
            _ => Err(LkError::new(
                ErrorCode::InvalidOperand,
                "variant operand index is outside payload contract",
            )
            .for_node(operation_id)),
        },
        OperationKind::MatchSum { arms, .. } => {
            let variant = arms
                .first()
                .ok_or_else(|| {
                    LkError::new(ErrorCode::InvalidOperand, "match has no arms")
                        .for_node(operation_id)
                })?
                .variant;
            match snapshot.node(variant)? {
                Node::SumVariant { owner, .. } if index == 0 => Ok(SemanticType::Nominal(*owner)),
                _ => Err(LkError::new(
                    ErrorCode::InvalidOperand,
                    "match operand index is outside scrutinee contract",
                )
                .for_node(operation_id)),
            }
        }
        _ => operation
            .operand_type(index, Some(function_result))
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "operand index is outside operation contract",
                )
                .for_node(operation_id)
            }),
    }
}

fn use_sites_page(
    snapshot: &Snapshot,
    target: ValueRef,
    page: PageRequest,
    allow_zero: bool,
) -> Result<Page<UseSite>> {
    value_type(snapshot, target)?;
    if !allow_zero || page.limit != 0 {
        validate_page(page)?;
    }
    let start = match page.after {
        None => 0,
        Some(PageCursor::IncomingUses {
            workspace,
            revision,
            value,
            next,
        }) if workspace == snapshot.workspace()
            && revision == snapshot.revision()
            && value == target =>
        {
            next
        }
        Some(_) => {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this query",
            ));
        }
    };
    let end = start.saturating_add(page.limit as u64);
    let mut total = 0u64;
    let mut items = Vec::with_capacity(page.limit as usize);
    for (id, node) in snapshot.nodes() {
        let Node::Operation {
            owner: block,
            operation,
        } = node
        else {
            continue;
        };
        let (function, function_result) = operation_use_context(snapshot, *block)?;
        for i in 0..operation.operand_count() {
            if operation.operand(i) != Some(target) {
                continue;
            }
            if total >= start && total < end {
                let index = u64::try_from(i).map_err(|_| {
                    LkError::new(ErrorCode::PolicyExceeded, "operand index overflow")
                })?;
                let expected = expected_operand_type(snapshot, id, operation, i, function_result)?;
                items.push(UseSite {
                    source: id,
                    operand_index: index,
                    target,
                    owner_block: *block,
                    owner_function: function,
                    expected_type: expected,
                    use_mode: operation.operand_use(i).unwrap_or(OperandUse::Read),
                });
            }
            total += 1;
        }
    }
    if start > total {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "cursor is beyond query result",
        ));
    }
    let consumed = start.saturating_add(items.len() as u64);
    Ok(Page {
        items,
        next: (consumed < total).then_some(PageCursor::IncomingUses {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            value: target,
            next: consumed,
        }),
        total: Some(total),
    })
}

fn uses_page(snapshot: &Snapshot, value: ValueRef, page: PageRequest) -> Result<Page<UseSite>> {
    use_sites_page(snapshot, value, page, false)
}
fn operation_definition_slot(operation: &OperationKind, target: NodeId) -> Option<DefinitionSlot> {
    Some(match operation {
        OperationKind::Call { .. } => DefinitionSlot::CallTarget,
        OperationKind::ConstructProduct { product, .. } if *product == target => {
            DefinitionSlot::ProductDeclaration
        }
        OperationKind::ConstructProduct { .. } | OperationKind::ProjectField { .. } => {
            DefinitionSlot::ProductField
        }
        OperationKind::ConstructVariant { .. } => DefinitionSlot::SumVariant,
        OperationKind::MatchSum { .. } => DefinitionSlot::MatchVariant,
        OperationKind::SequenceEmpty { .. }
        | OperationKind::SequenceLen { .. }
        | OperationKind::SequenceGet { .. }
        | OperationKind::SequenceAppend { .. }
        | OperationKind::SequenceReplace { .. }
        | OperationKind::SequenceSlice { .. }
        | OperationKind::SequenceConcat { .. } => DefinitionSlot::SequenceDeclaration,
        _ => return None,
    })
}

fn definition_page(
    snapshot: &Snapshot,
    target: NodeId,
    page: PageRequest,
) -> Result<Page<DefinitionReferenceSite>> {
    snapshot.node(target)?;
    let page = validate_page(page)?;
    let start = match page.after {
        None => 0,
        Some(PageCursor::DefinitionReferences {
            workspace,
            revision,
            target: t,
            next,
        }) if workspace == snapshot.workspace()
            && revision == snapshot.revision()
            && t == target =>
        {
            next
        }
        Some(_) => {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this query",
            ));
        }
    };
    let end = start.saturating_add(page.limit as u64);
    let mut total = 0u64;
    let mut items = Vec::with_capacity(page.limit as usize);
    for (id, node) in snapshot.nodes() {
        for index in 0..node.direct_reference_count() {
            let Some(reference) = node.direct_reference(index) else {
                continue;
            };
            if reference.target() != target {
                continue;
            }
            let slot = match reference {
                DirectReference::Definition { .. } => match node {
                    Node::Package { .. } => DefinitionSlot::PackageEntry,
                    Node::Operation { operation, .. } => {
                        operation_definition_slot(operation, target)
                            .unwrap_or(DefinitionSlot::CallTarget)
                    }
                    _ => continue,
                },
                DirectReference::Type { slot, .. } => match slot {
                    TypeReferenceSlot::FunctionResult => DefinitionSlot::FunctionResultType,
                    TypeReferenceSlot::ParameterType => DefinitionSlot::ParameterType,
                    TypeReferenceSlot::ProductFieldType => DefinitionSlot::ProductFieldType,
                    TypeReferenceSlot::SumVariantPayload => DefinitionSlot::SumVariantPayloadType,
                    TypeReferenceSlot::SequenceElementType => DefinitionSlot::SequenceElementType,
                    TypeReferenceSlot::BlockArgumentType => DefinitionSlot::BlockArgumentType,
                    TypeReferenceSlot::OperationType => DefinitionSlot::OperationType,
                },
                DirectReference::ValueOperand { .. } => continue,
            };
            if total >= start && total < end {
                items.push(DefinitionReferenceSite {
                    source: id,
                    slot,
                    target,
                });
            }
            total += 1;
        }
    }
    if start > total {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "cursor is beyond query result",
        ));
    }
    let consumed = start + items.len() as u64;
    Ok(Page {
        items,
        next: (consumed < total).then_some(PageCursor::DefinitionReferences {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            target,
            next: consumed,
        }),
        total: Some(total),
    })
}
fn dependencies(snapshot: &Snapshot, id: NodeId) -> Result<Vec<DependencyFact>> {
    let node = snapshot.node(id)?;
    let mut values = Vec::new();
    for index in 0..node.direct_reference_count() {
        let Some(reference) = node.direct_reference(index) else {
            continue;
        };
        match reference {
            DirectReference::Definition { target } => {
                let slot = match node {
                    Node::Package { .. } => DefinitionSlot::PackageEntry,
                    Node::Operation { operation, .. } => {
                        operation_definition_slot(operation, target)
                            .unwrap_or(DefinitionSlot::CallTarget)
                    }
                    _ => continue,
                };
                values.push(DependencyFact::Definition { slot, target });
            }
            DirectReference::Type { slot, target } => {
                let slot = match slot {
                    TypeReferenceSlot::FunctionResult => DefinitionSlot::FunctionResultType,
                    TypeReferenceSlot::ParameterType => DefinitionSlot::ParameterType,
                    TypeReferenceSlot::ProductFieldType => DefinitionSlot::ProductFieldType,
                    TypeReferenceSlot::SumVariantPayload => DefinitionSlot::SumVariantPayloadType,
                    TypeReferenceSlot::SequenceElementType => DefinitionSlot::SequenceElementType,
                    TypeReferenceSlot::BlockArgumentType => DefinitionSlot::BlockArgumentType,
                    TypeReferenceSlot::OperationType => DefinitionSlot::OperationType,
                };
                values.push(DependencyFact::Definition { slot, target });
            }
            DirectReference::ValueOperand { index, value } => {
                values.push(DependencyFact::ValueOperand { index, value });
            }
        }
    }
    Ok(values)
}
fn dependency_page(
    snapshot: &Snapshot,
    id: NodeId,
    page: PageRequest,
) -> Result<Page<DependencyFact>> {
    page_items(
        dependencies(snapshot, id)?,
        page,
        |next| PageCursor::Dependencies {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            node: id,
            next,
        },
        |c| match c {
            PageCursor::Dependencies {
                workspace,
                revision,
                node,
                next,
            } if workspace == snapshot.workspace()
                && revision == snapshot.revision()
                && node == id =>
            {
                Some(next)
            }
            _ => None,
        },
    )
}

fn target_contract(snapshot: &Snapshot, target: RepairTarget) -> Result<(SemanticType, Location)> {
    match target {
        RepairTarget::Hole(id) => {
            let loc = operation_location(snapshot, id)?;
            let Node::Operation {
                operation: OperationKind::Hole { expected },
                ..
            } = snapshot.node(id)?
            else {
                return Err(
                    LkError::new(ErrorCode::WrongKind, "repair target is not a hole").for_node(id),
                );
            };
            Ok((*expected, loc))
        }
        RepairTarget::Operand { operation, index } => {
            let loc = operation_location(snapshot, operation)?;
            let Node::Operation {
                operation: kind, ..
            } = snapshot.node(operation)?
            else {
                unreachable!()
            };
            let index = usize::try_from(index).map_err(|_| {
                LkError::new(
                    ErrorCode::InvalidOperand,
                    "operand index overflows host indexes",
                )
                .for_node(operation)
            })?;
            let expected = expected_operand_type(snapshot, operation, kind, index, loc.result)?;
            Ok((expected, loc))
        }
    }
}
fn visible_block_limits(snapshot: &Snapshot, loc: Location) -> Result<Vec<(NodeId, usize)>> {
    let mut path = vec![(loc.block, loc.ordinal as usize)];
    let mut current = loc.block;
    loop {
        let Node::Block { owner: region, .. } = snapshot.node(current)? else {
            unreachable!()
        };
        let Node::Region { owner, .. } = snapshot.node(*region)? else {
            unreachable!()
        };
        match snapshot.node(*owner)? {
            Node::Function { .. } => break,
            Node::Operation {
                owner: parent_block,
                ..
            } => {
                let Node::Block { operations, .. } = snapshot.node(*parent_block)? else {
                    unreachable!()
                };
                let position = operations
                    .iter()
                    .position(|operation| *operation == *owner)
                    .ok_or_else(|| {
                        LkError::new(
                            ErrorCode::InvalidContainment,
                            "structured owner is absent from parent block",
                        )
                        .for_node(*owner)
                    })?;
                path.push((*parent_block, position));
                current = *parent_block;
            }
            _ => {
                return Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "region owner cannot form a lexical path",
                )
                .for_node(*region));
            }
        }
    }
    path.reverse();
    Ok(path)
}

fn visible_page(
    snapshot: &Snapshot,
    purpose: VisibleCursorPurpose,
    target: RepairTarget,
    expected: SemanticType,
    loc: Location,
    include: bool,
    page: PageRequest,
) -> Result<Page<VisibleValue>> {
    if page.limit == 0 {
        if purpose != VisibleCursorPurpose::RepairContext {
            return Err(LkError::new(
                ErrorCode::InvalidQuery,
                "page limit must be nonzero",
            ));
        }
    } else {
        validate_page(page)?;
    }
    let start = match page.after {
        None => 0,
        Some(PageCursor::VisibleValues {
            workspace,
            revision,
            purpose: p,
            target: t,
            expected: e,
            include_incompatible: i,
            next,
        }) if workspace == snapshot.workspace()
            && revision == snapshot.revision()
            && p == purpose
            && t == target
            && e == expected
            && i == include =>
        {
            next
        }
        Some(_) => {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this query",
            ));
        }
    };
    let end = start.saturating_add(page.limit as u64);
    let mut total = 0u64;
    let mut items = Vec::with_capacity(page.limit as usize);
    let mut retain = |value: VisibleValue| {
        if value.compatible || include {
            if total >= start && total < end {
                items.push(value);
            }
            total += 1;
        }
    };
    let Node::Function { parameters, .. } = snapshot.node(loc.function)? else {
        unreachable!()
    };
    for parameter in parameters {
        let Node::Parameter { name, ty, .. } = snapshot.node(*parameter)? else {
            return Err(LkError::new(
                ErrorCode::InvalidContainment,
                "function parameter slot is invalid",
            ));
        };
        retain(VisibleValue {
            value: ValueRef::FunctionParameter(*parameter),
            ty: *ty,
            compatible: *ty == expected,
            producer: *parameter,
            producer_code: None,
            owner_function: loc.function,
            ordinal: None,
            name: Some(name_preview(name)),
        });
    }
    for (visible_block, limit) in visible_block_limits(snapshot, loc)? {
        let Node::Block {
            arguments,
            operations,
            ..
        } = snapshot.node(visible_block)?
        else {
            unreachable!()
        };
        for argument in arguments {
            let Node::BlockArgument { ordinal, ty, .. } = snapshot.node(*argument)? else {
                unreachable!()
            };
            retain(VisibleValue {
                value: ValueRef::BlockArgument(*argument),
                ty: *ty,
                compatible: *ty == expected,
                producer: *argument,
                producer_code: None,
                owner_function: loc.function,
                ordinal: Some(u64::from(*ordinal)),
                name: None,
            });
        }
        for (ordinal, id) in operations.iter().copied().take(limit).enumerate() {
            let Node::Operation { operation, .. } = snapshot.node(id)? else {
                unreachable!()
            };
            for output in 0..operation.result_count() {
                if let Some(ty) =
                    crate::graph::operation_result_type(snapshot, id, operation, output)
                {
                    retain(VisibleValue {
                        value: ValueRef::OperationResult {
                            operation: id,
                            output: output as u8,
                        },
                        ty,
                        compatible: ty == expected,
                        producer: id,
                        producer_code: Some(operation.code()),
                        owner_function: loc.function,
                        ordinal: Some(ordinal as u64),
                        name: None,
                    });
                }
            }
        }
    }
    if start > total {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "cursor is beyond query result",
        ));
    }
    let consumed = start + items.len() as u64;
    Ok(Page {
        items,
        next: (consumed < total).then_some(PageCursor::VisibleValues {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            purpose,
            target,
            expected,
            include_incompatible: include,
            next: consumed,
        }),
        total: Some(total),
    })
}
fn legal_constructor_slice(
    snapshot: &Snapshot,
    expected: SemanticType,
    start: u64,
    limit: usize,
) -> (Vec<ConstructorDescriptor>, u64) {
    let mut items = Vec::with_capacity(limit.min(MAX_CONTEXT_ITEMS as usize));
    let mut total = 0_u64;
    let end = start.saturating_add(u64::try_from(limit).unwrap_or(u64::MAX));
    for code in OperationCode::ALL {
        let descriptor = code.descriptor();
        if !descriptor.complete || descriptor.terminator || descriptor.results.len() != 1 {
            continue;
        }
        match code {
            OperationCode::Call => {
                for (id, node) in snapshot.nodes() {
                    let Node::Function {
                        parameters, result, ..
                    } = node
                    else {
                        continue;
                    };
                    if *result != expected
                        || parameters.iter().any(|parameter| {
                            !matches!(snapshot.node(*parameter), Ok(Node::Parameter { .. }))
                        })
                    {
                        continue;
                    }
                    let retain = total >= start && total < end;
                    total = total.saturating_add(1);
                    if retain {
                        let operand_count = parameters.len() as u64;
                        let operand_types = parameters
                            .iter()
                            .take(MAX_CONTEXT_ITEMS as usize)
                            .map(|parameter| match snapshot.node(*parameter) {
                                Ok(Node::Parameter { ty, .. }) => *ty,
                                _ => unreachable!("validated parameter checked above"),
                            })
                            .collect::<Vec<_>>();
                        let requirements_complete = operand_types.len() == parameters.len();
                        items.push(ConstructorDescriptor {
                            code,
                            result_type: expected,
                            operand_count,
                            operand_uses: vec![OperandUse::Read; operand_types.len()],
                            operand_types,
                            literal_fields: Vec::new(),
                            call_target: Some(id),
                            declaration: None,
                            member_count: 0,
                            members: Vec::new(),
                            requirements_complete,
                            nominal_type_continuation: None,
                            direct_refinement: true,
                            complete: true,
                            terminator: false,
                        });
                    }
                }
            }
            OperationCode::If | OperationCode::ForI64 => {
                let retain = total >= start && total < end;
                total = total.saturating_add(1);
                if retain {
                    let operand_types = if code == OperationCode::If {
                        vec![SemanticType::Bool]
                    } else {
                        vec![SemanticType::I64, SemanticType::I64, expected]
                    };
                    items.push(ConstructorDescriptor {
                        code,
                        result_type: expected,
                        operand_count: operand_types.len() as u64,
                        operand_uses: vec![OperandUse::Read; operand_types.len()],
                        operand_types,
                        literal_fields: descriptor.literal_fields.to_vec(),
                        call_target: None,
                        declaration: None,
                        member_count: 0,
                        members: Vec::new(),
                        requirements_complete: true,
                        nominal_type_continuation: None,
                        direct_refinement: false,
                        complete: true,
                        terminator: false,
                    });
                }
            }
            OperationCode::ConstructProduct => {
                let SemanticType::Nominal(declaration) = expected else {
                    continue;
                };
                let Ok(Node::ProductType { fields, .. }) = snapshot.node(declaration) else {
                    continue;
                };
                let retain = total >= start && total < end;
                total = total.saturating_add(1);
                if retain {
                    let operand_types = fields
                        .iter()
                        .take(MAX_CONTEXT_ITEMS as usize)
                        .map(|field| match snapshot.node(*field) {
                            Ok(Node::ProductField { ty, .. }) => *ty,
                            _ => unreachable!("validated product declaration"),
                        })
                        .collect::<Vec<_>>();
                    let members = fields
                        .iter()
                        .take(MAX_CONTEXT_ITEMS as usize)
                        .copied()
                        .collect::<Vec<_>>();
                    let requirements_complete = members.len() == fields.len();
                    items.push(ConstructorDescriptor {
                        code,
                        result_type: expected,
                        operand_count: fields.len() as u64,
                        operand_uses: vec![OperandUse::Read; operand_types.len()],
                        operand_types,
                        literal_fields: Vec::new(),
                        call_target: None,
                        declaration: Some(declaration),
                        member_count: fields.len() as u64,
                        members,
                        requirements_complete,
                        nominal_type_continuation: (!requirements_complete).then_some(
                            NominalTypeContinuation {
                                declaration,
                                page: PageRequest {
                                    after: None,
                                    limit: MAX_CONTEXT_ITEMS,
                                },
                            },
                        ),
                        direct_refinement: true,
                        complete: true,
                        terminator: false,
                    });
                }
            }
            OperationCode::ConstructVariant => {
                let SemanticType::Nominal(declaration) = expected else {
                    continue;
                };
                let Ok(Node::SumType { variants, .. }) = snapshot.node(declaration) else {
                    continue;
                };
                for variant in variants {
                    let Ok(Node::SumVariant { payload, .. }) = snapshot.node(*variant) else {
                        continue;
                    };
                    let retain = total >= start && total < end;
                    total = total.saturating_add(1);
                    if retain {
                        let operand_types = payload.iter().copied().collect::<Vec<_>>();
                        items.push(ConstructorDescriptor {
                            code,
                            result_type: expected,
                            operand_count: operand_types.len() as u64,
                            operand_uses: vec![OperandUse::Read; operand_types.len()],
                            operand_types,
                            literal_fields: Vec::new(),
                            call_target: None,
                            declaration: Some(declaration),
                            member_count: 1,
                            members: vec![*variant],
                            requirements_complete: true,
                            nominal_type_continuation: None,
                            direct_refinement: true,
                            complete: true,
                            terminator: false,
                        });
                    }
                }
            }
            OperationCode::ProjectField => {
                for (field, node) in snapshot.nodes() {
                    let Node::ProductField { owner, ty, .. } = node else {
                        continue;
                    };
                    if *ty != expected {
                        continue;
                    }
                    let retain = total >= start && total < end;
                    total = total.saturating_add(1);
                    if retain {
                        items.push(ConstructorDescriptor {
                            code,
                            result_type: expected,
                            operand_count: 1,
                            operand_types: vec![SemanticType::Nominal(*owner)],
                            operand_uses: vec![OperandUse::Read],
                            literal_fields: Vec::new(),
                            call_target: None,
                            declaration: Some(*owner),
                            member_count: 1,
                            members: vec![field],
                            requirements_complete: true,
                            nominal_type_continuation: None,
                            direct_refinement: true,
                            complete: true,
                            terminator: false,
                        });
                    }
                }
            }
            OperationCode::MatchSum => continue,
            _ => {
                let Some(result) = (match descriptor.results[0] {
                    TypeRule::Fixed(ty) => Some(ty),
                    _ => None,
                }) else {
                    continue;
                };
                if result != expected {
                    continue;
                }
                let retain = total >= start && total < end;
                total = total.saturating_add(1);
                if retain {
                    let operand_types = descriptor
                        .operands
                        .iter()
                        .filter_map(|operand| match operand.ty {
                            TypeRule::Fixed(ty) => Some(ty),
                            _ => None,
                        })
                        .collect::<Vec<_>>();
                    items.push(ConstructorDescriptor {
                        code,
                        result_type: result,
                        operand_count: operand_types.len() as u64,
                        operand_types,
                        operand_uses: descriptor
                            .operands
                            .iter()
                            .map(|operand| operand.use_mode)
                            .collect(),
                        literal_fields: descriptor.literal_fields.to_vec(),
                        call_target: None,
                        declaration: None,
                        member_count: 0,
                        members: Vec::new(),
                        requirements_complete: true,
                        nominal_type_continuation: None,
                        direct_refinement: true,
                        complete: descriptor.complete,
                        terminator: descriptor.terminator,
                    });
                }
            }
        }
    }
    (items, total)
}

fn legal_constructor_page(
    snapshot: &Snapshot,
    target: RepairTarget,
    expected: SemanticType,
    page: PageRequest,
) -> Result<Page<ConstructorDescriptor>> {
    let page = validate_page(page)?;
    let start = match page.after {
        None => 0,
        Some(PageCursor::LegalConstructors {
            workspace,
            revision,
            target: cursor_target,
            expected: cursor_expected,
            next,
        }) if workspace == snapshot.workspace()
            && revision == snapshot.revision()
            && cursor_target == target
            && cursor_expected == expected =>
        {
            next
        }
        Some(_) => {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this legal-constructor query",
            ));
        }
    };
    let (items, total) = legal_constructor_slice(snapshot, expected, start, page.limit as usize);
    if start > total {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "legal-constructor cursor is beyond the result",
        ));
    }
    let consumed = start.saturating_add(items.len() as u64);
    Ok(Page {
        items,
        next: (consumed < total).then_some(PageCursor::LegalConstructors {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            target,
            expected,
            next: consumed,
        }),
        total: Some(total),
    })
}

fn owner_chain_page(
    snapshot: &Snapshot,
    target: NodeId,
    page: PageRequest,
) -> Result<Page<OwnerFact>> {
    let page = validate_page(page)?;
    let start = match page.after {
        None => 0,
        Some(PageCursor::OwnerChain {
            workspace,
            revision,
            node,
            next,
        }) if workspace == snapshot.workspace()
            && revision == snapshot.revision()
            && node == target =>
        {
            next
        }
        Some(_) => {
            return Err(LkError::new(
                ErrorCode::InvalidCursor,
                "cursor does not belong to this owner-chain query",
            ));
        }
    };
    let end = start.saturating_add(page.limit as u64);
    let mut items = Vec::with_capacity(page.limit as usize);
    let mut total = 0_u64;
    let mut current = Some(target);
    while let Some(id) = current {
        let node = snapshot.node(id)?;
        if total >= start && total < end {
            items.push(OwnerFact {
                node: id,
                kind: node.kind(),
                name: node.name().map(name_preview),
            });
        }
        total = total.saturating_add(1);
        current = node.owner();
    }
    if start > total {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "owner-chain cursor is beyond the result",
        ));
    }
    let consumed = start.saturating_add(items.len() as u64);
    Ok(Page {
        items,
        next: (consumed < total).then_some(PageCursor::OwnerChain {
            workspace: snapshot.workspace(),
            revision: snapshot.revision(),
            node: target,
            next: consumed,
        }),
        total: Some(total),
    })
}

fn owner_chain_with_limit(
    snapshot: &Snapshot,
    node: NodeId,
    limit: usize,
) -> Result<Vec<OwnerFact>> {
    let mut facts = Vec::new();
    let mut current = Some(node);
    while let Some(id) = current {
        if facts.len() == limit {
            break;
        }
        let node = snapshot.node(id)?;
        facts.push(OwnerFact {
            node: id,
            kind: node.kind(),
            name: node.name().map(name_preview),
        });
        current = node.owner();
    }
    Ok(facts)
}

fn derived_region_fact(snapshot: &Snapshot, region: NodeId) -> Result<Option<EnclosingRegionFact>> {
    let Node::Region { owner, .. } = snapshot.node(region)? else {
        return Err(wrong(
            region,
            NodeKind::Region,
            snapshot.node(region)?.kind(),
        ));
    };
    let Node::Operation { operation, .. } = snapshot.node(*owner)? else {
        return Ok(None);
    };
    Ok(operation
        .region_role(region)
        .map(|role| EnclosingRegionFact {
            region,
            owner_operation: *owner,
            role,
        }))
}

fn structured_context_facts(
    snapshot: &Snapshot,
    start_block: NodeId,
) -> Result<(Vec<EnclosingRegionFact>, Vec<BlockArgumentFact>)> {
    let mut enclosing_regions = Vec::new();
    let mut visible_arguments = Vec::new();
    let mut block = start_block;
    loop {
        let Node::Block {
            owner: region,
            arguments,
            ..
        } = snapshot.node(block)?
        else {
            return Err(wrong(block, NodeKind::Block, snapshot.node(block)?.kind()));
        };
        let fact = derived_region_fact(snapshot, *region)?;
        if fact
            .is_some_and(|fact| matches!(fact.role, RegionRole::ForBody | RegionRole::MatchArm(_)))
        {
            let roles: &[BlockArgumentRole] = match fact.map(|fact| fact.role) {
                Some(RegionRole::ForBody) => {
                    &[BlockArgumentRole::LoopIndex, BlockArgumentRole::LoopCarried]
                }
                Some(RegionRole::MatchArm(_)) => &[BlockArgumentRole::MatchPayload],
                _ => &[],
            };
            for (index, argument) in arguments.iter().enumerate() {
                if visible_arguments.len() == MAX_CONTEXT_ITEMS as usize {
                    break;
                }
                let Node::BlockArgument { ordinal, ty, .. } = snapshot.node(*argument)? else {
                    return Err(wrong(
                        *argument,
                        NodeKind::BlockArgument,
                        snapshot.node(*argument)?.kind(),
                    ));
                };
                let role = roles.get(index).copied().ok_or_else(|| {
                    LkError::new(
                        ErrorCode::InvalidContainment,
                        "loop body has an unexpected block argument",
                    )
                    .for_node(*argument)
                })?;
                visible_arguments.push(BlockArgumentFact {
                    argument: *argument,
                    block,
                    region: *region,
                    ordinal: *ordinal,
                    role,
                    ty: *ty,
                });
            }
        }
        let Some(fact) = fact else { break };
        if enclosing_regions.len() < MAX_CONTEXT_ITEMS as usize {
            enclosing_regions.push(fact);
        }
        let Node::Operation {
            owner: parent_block,
            ..
        } = snapshot.node(fact.owner_operation)?
        else {
            unreachable!()
        };
        block = *parent_block;
    }
    Ok((enclosing_regions, visible_arguments))
}
fn repair_context(
    snapshot: &Snapshot,
    target: RepairTarget,
    budget: ContextBudget,
) -> Result<RepairContext> {
    validate_context_budget(budget)?;
    let (expected, loc) = target_contract(snapshot, target)?;
    let (operation, index, current, use_mode, code) = match target {
        RepairTarget::Hole(id) => {
            let Node::Operation { operation, .. } = snapshot.node(id)? else {
                unreachable!()
            };
            (id, None, None, None, operation.code())
        }
        RepairTarget::Operand { operation, index } => {
            let Node::Operation { operation: k, .. } = snapshot.node(operation)? else {
                unreachable!()
            };
            (
                operation,
                Some(index),
                usize::try_from(index)
                    .ok()
                    .and_then(|index| k.operand(index)),
                usize::try_from(index)
                    .ok()
                    .and_then(|index| k.operand_use(index)),
                k.code(),
            )
        }
    };
    let current_actual = current.map(|v| value_type(snapshot, v)).transpose()?;
    let Node::Block {
        operations,
        terminator,
        ..
    } = snapshot.node(loc.block)?
    else {
        unreachable!()
    };
    let body_len = operations.len() as u64 + u64::from(terminator.is_some());
    let start = loc.ordinal.saturating_sub(budget.body_before as u64);
    let end = loc
        .ordinal
        .saturating_add(budget.body_after as u64)
        .saturating_add(1)
        .min(body_len);
    let visible = visible_page(
        snapshot,
        VisibleCursorPurpose::RepairContext,
        target,
        expected,
        loc,
        budget.include_incompatible,
        PageRequest {
            after: None,
            limit: budget.visible_values,
        },
    )?;
    let incoming = if node_value_type(snapshot, operation, snapshot.node(operation)?, 0).is_some() {
        // Repair-context use continuations intentionally reuse IncomingUses: clients continue
        // through that public query with the exact value embedded in the bound cursor.
        use_sites_page(
            snapshot,
            ValueRef::OperationResult {
                operation,
                output: 0,
            },
            PageRequest {
                after: None,
                limit: budget.incoming_uses,
            },
            true,
        )?
    } else {
        Page {
            items: Vec::new(),
            next: None,
            total: Some(0),
        }
    };
    let Node::Function {
        parameters, result, ..
    } = snapshot.node(loc.function)?
    else {
        unreachable!()
    };
    let blocker = workspace_blockers(snapshot)
        .into_iter()
        .find(|b| b.target == Some(operation));
    let (legal_constructors, legal_constructor_count) =
        legal_constructor_slice(snapshot, expected, 0, MAX_CONTEXT_ITEMS as usize);
    let (enclosing_regions, visible_block_arguments) =
        structured_context_facts(snapshot, loc.block)?;
    let (nominal_type, nominal_type_continuation) =
        if let SemanticType::Nominal(declaration) = expected {
            let count = snapshot.node(declaration)?.owned_child_count();
            if count <= MAX_CONTEXT_ITEMS as usize {
                (
                    Some(nominal_type_result(
                        snapshot,
                        declaration,
                        PageRequest {
                            after: None,
                            limit: u32::try_from(count.max(1)).unwrap_or(MAX_CONTEXT_ITEMS),
                        },
                    )?),
                    None,
                )
            } else {
                (
                    None,
                    Some(NominalTypeContinuation {
                        declaration,
                        page: PageRequest {
                            after: None,
                            limit: MAX_CONTEXT_ITEMS,
                        },
                    }),
                )
            }
        } else {
            (None, None)
        };
    Ok(RepairContext {
        workspace: snapshot.workspace(),
        revision: snapshot.revision(),
        target,
        operation,
        operation_code: code,
        operand_index: index,
        expected_type: expected,
        use_mode,
        current_value: current,
        current_actual_type: current_actual,
        owner_block: loc.block,
        owner_function: loc.function,
        ordinal: loc.ordinal,
        function_signature: FunctionSignatureSummary {
            parameter_count: parameters.len() as u64,
            result: *result,
        },
        owner_chain: owner_chain_with_limit(snapshot, operation, MAX_CONTEXT_ITEMS as usize)?,
        enclosing_regions,
        visible_block_arguments,
        body_window: body_range(snapshot, loc.block, start, end)?,
        visible_values: visible,
        incoming_uses: incoming,
        legal_constructor_count,
        legal_constructors,
        nominal_type,
        nominal_type_continuation,
        blocker,
        refinement_operation: matches!(target, RepairTarget::Hole(_))
            .then_some(TransactionOpCode::RefineHole),
    })
}
fn diff_page(before: &Snapshot, after: &Snapshot, page: PageRequest) -> Result<SemanticDiffPage> {
    if before.workspace() != after.workspace() || before.revision() > after.revision() {
        return Err(LkError::new(
            ErrorCode::InvalidQuery,
            "diff revisions are invalid",
        ));
    }
    let d = diff::between(before, after);
    let count = d.change_count();
    let digest = d.digest;
    let p = page_items(
        d.changes,
        page,
        |next| PageCursor::Diff {
            workspace: after.workspace(),
            from: before.revision(),
            to: after.revision(),
            next,
        },
        |c| match c {
            PageCursor::Diff {
                workspace,
                from,
                to,
                next,
            } if workspace == after.workspace()
                && from == before.revision()
                && to == after.revision() =>
            {
                Some(next)
            }
            _ => None,
        },
    )?;
    Ok(SemanticDiffPage {
        from: before.revision(),
        to: after.revision(),
        change_count: count,
        change_digest: digest,
        page: p,
    })
}
fn wrong(node: NodeId, expected: NodeKind, actual: NodeKind) -> LkError {
    LkError::new(ErrorCode::WrongKind, "query target has the wrong kind")
        .for_node(node)
        .with_kinds(expected, actual)
}

pub fn workspace_blockers(snapshot: &Snapshot) -> Vec<CompletenessBlocker> {
    let mut b = Vec::new();
    if !snapshot
        .nodes()
        .any(|(_, n)| matches!(n, Node::Package { .. }))
    {
        b.push(CompletenessBlocker {
            owner: snapshot.root(),
            target: None,
            category: ExpectedCategory::EntryFunction,
            expected_type: None,
        });
    }
    for (id, n) in snapshot.nodes() {
        match n {
            Node::Package { entry: None, .. } => b.push(CompletenessBlocker {
                owner: id,
                target: None,
                category: ExpectedCategory::EntryFunction,
                expected_type: None,
            }),
            Node::Function { body: None, .. } => b.push(CompletenessBlocker {
                owner: id,
                target: None,
                category: ExpectedCategory::FunctionBody,
                expected_type: None,
            }),
            Node::Operation {
                operation: OperationKind::Hole { expected },
                ..
            } => b.push(CompletenessBlocker {
                owner: id,
                target: Some(id),
                category: ExpectedCategory::Expression,
                expected_type: Some(*expected),
            }),
            _ => {}
        }
    }
    b.sort();
    b
}
pub fn entry_blockers(snapshot: &Snapshot, entry: NodeId) -> Result<Vec<CompletenessBlocker>> {
    let entry_node = snapshot.node(entry)?;
    if !matches!(entry_node, Node::Function { .. }) {
        return Err(wrong(entry, NodeKind::Function, entry_node.kind()));
    }
    let mut blockers = Vec::new();
    let mut pending_functions = vec![entry];
    let mut visited_functions = BTreeSet::new();
    while let Some(function) = pending_functions.pop() {
        if !visited_functions.insert(function) {
            continue;
        }
        let Node::Function { body, .. } = snapshot.node(function)? else {
            return Err(wrong(
                function,
                NodeKind::Function,
                snapshot.node(function)?.kind(),
            ));
        };
        let Some(body) = body else {
            blockers.push(CompletenessBlocker {
                owner: function,
                target: None,
                category: ExpectedCategory::FunctionBody,
                expected_type: None,
            });
            continue;
        };
        let mut stack = vec![*body];
        while let Some(id) = stack.pop() {
            let node = snapshot.node(id)?;
            if let Node::Operation { operation, .. } = node {
                match operation {
                    OperationKind::Hole { expected } => blockers.push(CompletenessBlocker {
                        owner: id,
                        target: Some(id),
                        category: ExpectedCategory::Expression,
                        expected_type: Some(*expected),
                    }),
                    OperationKind::Call {
                        function: target, ..
                    } if !visited_functions.contains(target) => pending_functions.push(*target),
                    _ => {}
                }
            }
            for index in (0..node.owned_child_count()).rev() {
                if let Some(child) = node.owned_child(index) {
                    stack.push(child);
                }
            }
        }
        pending_functions.sort_by(|left, right| right.cmp(left));
        pending_functions.dedup();
    }
    blockers.sort();
    Ok(blockers)
}
fn blockers_for_node(snapshot: &Snapshot, id: NodeId) -> Vec<CompletenessBlocker> {
    let mut descendants = BTreeSet::new();
    let mut stack = vec![id];
    while let Some(current) = stack.pop() {
        if !descendants.insert(current) {
            continue;
        }
        if let Ok(n) = snapshot.node(current) {
            for i in (0..n.owned_child_count()).rev() {
                if let Some(c) = n.owned_child(i) {
                    stack.push(c)
                }
            }
        }
    }
    workspace_blockers(snapshot)
        .into_iter()
        .filter(|b| {
            descendants.contains(&b.owner) || b.target.is_some_and(|t| descendants.contains(&t))
        })
        .collect()
}

#[cfg(test)]
mod tests;
