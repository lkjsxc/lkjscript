use crate::diff::{self, Change};
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{ChangeDigest, NodeId, QueryId, Revision, SnapshotHash, WorkspaceId};
use crate::schema::{
    BlockArgumentRole, LiteralField, Node, NodeKind, OperandUse, OperationCode, OperationKind,
    RegionRole, SemanticType, TypeRule, ValueRef,
};
use crate::transaction::TransactionOpCode;
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
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::VisibleValues => 1,
            Self::LegalConstructors => 2,
            Self::RepairContext => 3,
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::VisibleValues),
            2 => Some(Self::LegalConstructors),
            3 => Some(Self::RepairContext),
            _ => None,
        }
    }
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
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BodyItem {
    pub operation: NodeId,
    pub ordinal: u64,
    pub code: OperationCode,
    pub result_types: Vec<SemanticType>,
    pub operands: Vec<ValueRef>,
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
    pub operand_types: Vec<SemanticType>,
    pub operand_uses: Vec<OperandUse>,
    pub literal_fields: Vec<LiteralField>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub call_target: Option<NodeId>,
    pub direct_refinement: bool,
    pub complete: bool,
    pub terminator: bool,
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
}
impl QueryCode {
    pub const ALL: [Self; 12] = [
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
    ];
    pub const fn stable_tag(self) -> u8 {
        match self {
            Self::WorkspaceSummary => 1,
            Self::Node => 2,
            Self::Blockers => 3,
            Self::OwnerChain => 4,
            Self::Body => 5,
            Self::IncomingUses => 6,
            Self::DefinitionReferences => 7,
            Self::Dependencies => 8,
            Self::VisibleValues => 9,
            Self::LegalConstructors => 10,
            Self::SemanticDiff => 11,
            Self::RepairContext => 12,
        }
    }
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
        }
    }
    pub const fn from_stable_tag(tag: u8) -> Option<Self> {
        match tag {
            1 => Some(Self::WorkspaceSummary),
            2 => Some(Self::Node),
            3 => Some(Self::Blockers),
            4 => Some(Self::OwnerChain),
            5 => Some(Self::Body),
            6 => Some(Self::IncomingUses),
            7 => Some(Self::DefinitionReferences),
            8 => Some(Self::Dependencies),
            9 => Some(Self::VisibleValues),
            10 => Some(Self::LegalConstructors),
            11 => Some(Self::SemanticDiff),
            12 => Some(Self::RepairContext),
            _ => None,
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
}
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
        | Query::SemanticDiff { page, .. } => validate_page(*page)?.limit,
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
    }
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
                role: operation.descriptor().regions.get(index)?.role,
            })
        })
        .collect();
    let literal = match operation {
        OperationKind::ConstI64(v) => Some(LiteralValue::I64(*v)),
        OperationKind::ConstBool(v) => Some(LiteralValue::Bool(*v)),
        OperationKind::Hole { expected } => Some(LiteralValue::ExpectedType(*expected)),
        _ => None,
    };
    Ok(BodyItem {
        operation: id,
        ordinal,
        code: operation.code(),
        result_types: results,
        operands,
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
                _ => Err(LkError::new(
                    ErrorCode::InvalidContainment,
                    "yield has no structured owner contract",
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
                    use_mode: operation.operand_use(i).unwrap_or(OperandUse::Copy),
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
        let slot = match node {
            Node::Package {
                entry: Some(entry), ..
            } if *entry == target => Some(DefinitionSlot::PackageEntry),
            Node::Operation {
                operation: OperationKind::Call { function, .. },
                ..
            } if *function == target => Some(DefinitionSlot::CallTarget),
            _ => None,
        };
        if let Some(slot) = slot {
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
    let mut v = Vec::new();
    match node {
        Node::Package {
            entry: Some(target),
            ..
        } => v.push(DependencyFact::Definition {
            slot: DefinitionSlot::PackageEntry,
            target: *target,
        }),
        Node::Operation { operation, .. } => {
            if let OperationKind::Call { function, .. } = operation {
                v.push(DependencyFact::Definition {
                    slot: DefinitionSlot::CallTarget,
                    target: *function,
                });
            }
            for i in 0..operation.operand_count() {
                if let Some(value) = operation.operand(i) {
                    v.push(DependencyFact::ValueOperand {
                        index: i as u64,
                        value,
                    });
                }
            }
        }
        _ => {}
    }
    Ok(v)
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
                        let operand_types = parameters
                            .iter()
                            .map(|parameter| match snapshot.node(*parameter) {
                                Ok(Node::Parameter { ty, .. }) => *ty,
                                _ => unreachable!("validated parameter checked above"),
                            })
                            .collect::<Vec<_>>();
                        items.push(ConstructorDescriptor {
                            code,
                            result_type: expected,
                            operand_uses: vec![OperandUse::Copy; operand_types.len()],
                            operand_types,
                            literal_fields: Vec::new(),
                            call_target: Some(id),
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
                        operand_uses: vec![OperandUse::Copy; operand_types.len()],
                        operand_types,
                        literal_fields: descriptor.literal_fields.to_vec(),
                        call_target: None,
                        direct_refinement: false,
                        complete: true,
                        terminator: false,
                    });
                }
            }
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
                    items.push(ConstructorDescriptor {
                        code,
                        result_type: result,
                        operand_types: descriptor
                            .operands
                            .iter()
                            .filter_map(|operand| match operand.ty {
                                TypeRule::Fixed(ty) => Some(ty),
                                _ => None,
                            })
                            .collect(),
                        operand_uses: descriptor
                            .operands
                            .iter()
                            .map(|operand| operand.use_mode)
                            .collect(),
                        literal_fields: descriptor.literal_fields.to_vec(),
                        call_target: None,
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
    Ok((0..operation.owned_region_count()).find_map(|index| {
        (operation.owned_region(index) == Some(region))
            .then(|| {
                operation
                    .descriptor()
                    .regions
                    .get(index)
                    .map(|descriptor| EnclosingRegionFact {
                        region,
                        owner_operation: *owner,
                        role: descriptor.role,
                    })
            })
            .flatten()
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
        if fact.is_some_and(|fact| fact.role == RegionRole::ForBody) {
            let roles = [BlockArgumentRole::LoopIndex, BlockArgumentRole::LoopCarried];
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
mod tests {
    use super::*;
    use crate::graph::Workspace;
    use crate::ids::{LocalHandle, WorkspaceId};
    use crate::schema::{OperationDraft, ValueDraft};
    use crate::transaction::{
        ApplyTransactionRequest, ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft,
        NodeTarget, Transaction, TransactionMode, TransactionOp, TransactionResponseSpec,
        YieldingBodyDraft,
    };

    fn fixture() -> (Workspace, Vec<NodeId>) {
        let id = WorkspaceId::from_bytes([0x66; 16]);
        let mut workspace = Workspace::new(id).expect("workspace");
        let local = |v| NodeTarget::Local(LocalHandle::new(v));
        let value = |v| ValueDraft::OperationResult {
            operation: local(v),
            output: 0,
        };
        let tx = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    handle: LocalHandle::new(1),
                    name: "app".into(),
                },
                TransactionOp::CreateModule {
                    handle: LocalHandle::new(2),
                    package: local(1),
                    name: "root".into(),
                },
                TransactionOp::CreateFunction {
                    handle: LocalHandle::new(3),
                    module: local(2),
                    name: "main".into(),
                    parameters: Vec::new(),
                    result: SemanticType::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                handle: LocalHandle::new(6),
                                operation: ExpressionKindDraft::ConstI64(40),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(7),
                                operation: ExpressionKindDraft::ConstI64(2),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(8),
                                operation: ExpressionKindDraft::ConstBool(true),
                            },
                            ExpressionDraft {
                                handle: LocalHandle::new(9),
                                operation: ExpressionKindDraft::Hole {
                                    expected: SemanticType::I64,
                                },
                            },
                        ],
                        return_value: value(9),
                    }),
                },
                TransactionOp::SetEntryFunction {
                    package: local(1),
                    function: local(3),
                },
            ],
        };
        let request = ApplyTransactionRequest {
            transaction: tx,
            response: TransactionResponseSpec {
                return_handles: [1, 2, 3, 6, 7, 8, 9]
                    .into_iter()
                    .map(LocalHandle::new)
                    .collect(),
            },
        };
        let prepared = workspace.prepare_transaction(&request).expect("prepare");
        let binding = |handle| {
            prepared
                .receipt
                .returned_bindings
                .iter()
                .find_map(|(candidate, id)| (*candidate == LocalHandle::new(handle)).then_some(*id))
                .expect("binding")
        };
        let function = binding(3);
        let Node::Function {
            body: Some(region), ..
        } = prepared.snapshot.node(function).expect("function")
        else {
            panic!("function body")
        };
        let Node::Region { blocks, .. } = prepared.snapshot.node(*region).expect("region") else {
            panic!("region")
        };
        let block = blocks[0];
        let Node::Block {
            terminator: Some(terminator),
            ..
        } = prepared.snapshot.node(block).expect("block")
        else {
            panic!("terminator")
        };
        let ids = vec![
            binding(1),
            binding(2),
            function,
            *region,
            block,
            binding(6),
            binding(7),
            binding(8),
            binding(9),
            *terminator,
        ];
        workspace.publish(prepared.snapshot).expect("publish");
        (workspace, ids)
    }

    #[test]
    fn pages_uses_visibility_constructors_and_context_are_exact() {
        let (workspace, ids) = fixture();
        let snapshot = workspace.head().expect("head");
        let block = ids[4];
        let forty = ids[5];
        let two = ids[6];
        let boolean = ids[7];
        let hole = ids[8];
        let ret = ids[9];

        let owner_first = owner_chain_page(
            snapshot,
            hole,
            PageRequest {
                after: None,
                limit: 2,
            },
        )
        .expect("owner first");
        assert_eq!(owner_first.items.len(), 2);
        assert_eq!(owner_first.items[0].node, hole);
        assert!(owner_first.total.expect("owner total") > 2);
        let owner_second = owner_chain_page(
            snapshot,
            hole,
            PageRequest {
                after: owner_first.next,
                limit: 2,
            },
        )
        .expect("owner second");
        assert_eq!(owner_second.items.len(), 2);
        let mut wrong_owner_cursor = owner_first.next.expect("owner cursor");
        if let PageCursor::OwnerChain { node, .. } = &mut wrong_owner_cursor {
            *node = boolean;
        }
        assert_eq!(
            owner_chain_page(
                snapshot,
                hole,
                PageRequest {
                    after: Some(wrong_owner_cursor),
                    limit: 2,
                },
            )
            .expect_err("bound owner cursor")
            .code,
            ErrorCode::InvalidCursor
        );

        let first = body_page(
            snapshot,
            block,
            PageRequest {
                after: None,
                limit: 2,
            },
        )
        .expect("body page");
        assert_eq!(first.items.len(), 2);
        let next = first.next.expect("next");
        let rest = body_page(
            snapshot,
            block,
            PageRequest {
                after: Some(next),
                limit: MAX_PAGE_ITEMS,
            },
        )
        .expect("body rest");
        assert_eq!(rest.items.last().map(|x| x.operation), Some(ret));
        assert!(rest.items.last().expect("return").terminator);
        assert!(rest.next.is_none());
        let terminal = body_page(
            snapshot,
            block,
            PageRequest {
                after: Some(PageCursor::Body {
                    workspace: snapshot.workspace(),
                    revision: snapshot.revision(),
                    block,
                    next: 5,
                }),
                limit: 1,
            },
        )
        .expect("terminal cursor");
        assert!(terminal.items.is_empty());
        assert!(terminal.next.is_none());
        assert_eq!(
            body_page(
                snapshot,
                block,
                PageRequest {
                    after: Some(PageCursor::Body {
                        workspace: snapshot.workspace(),
                        revision: snapshot.revision(),
                        block,
                        next: 99
                    }),
                    limit: 1
                }
            )
            .expect_err("beyond")
            .code,
            ErrorCode::InvalidCursor
        );
        assert_eq!(
            body_page(
                snapshot,
                block,
                PageRequest {
                    after: None,
                    limit: 0
                }
            )
            .expect_err("zero")
            .code,
            ErrorCode::InvalidQuery
        );
        let uses = uses_page(
            snapshot,
            ValueRef::OperationResult {
                operation: hole,
                output: 0,
            },
            PageRequest {
                after: None,
                limit: 8,
            },
        )
        .expect("uses");
        assert_eq!(uses.items.len(), 1);
        assert_eq!(uses.items[0].source, ret);
        assert_eq!(uses.items[0].operand_index, 0);
        let crossed_parameter = value_type(snapshot, ValueRef::FunctionParameter(hole))
            .expect_err("operation as parameter");
        assert_eq!(
            (
                crossed_parameter.code,
                crossed_parameter.expected_kind,
                crossed_parameter.actual_kind
            ),
            (
                ErrorCode::WrongKind,
                Some(NodeKind::Parameter),
                Some(NodeKind::Operation)
            )
        );
        let crossed_operation = value_type(
            snapshot,
            ValueRef::OperationResult {
                operation: ids[2],
                output: 0,
            },
        )
        .expect_err("function as operation");
        assert_eq!(
            (
                crossed_operation.code,
                crossed_operation.expected_kind,
                crossed_operation.actual_kind
            ),
            (
                ErrorCode::WrongKind,
                Some(NodeKind::Operation),
                Some(NodeKind::Function)
            )
        );
        assert_eq!(
            value_type(
                snapshot,
                ValueRef::OperationResult {
                    operation: hole,
                    output: 1
                }
            )
            .expect_err("invalid output")
            .code,
            ErrorCode::InvalidOperand
        );
        let (expected, loc) =
            target_contract(snapshot, RepairTarget::Hole(hole)).expect("contract");
        let visible = visible_page(
            snapshot,
            VisibleCursorPurpose::VisibleValues,
            RepairTarget::Hole(hole),
            expected,
            loc,
            true,
            PageRequest {
                after: None,
                limit: 8,
            },
        )
        .expect("visible");
        assert_eq!(
            visible.items.iter().map(|v| v.producer).collect::<Vec<_>>(),
            vec![forty, two, boolean]
        );
        assert!(!visible.items[2].compatible);
        assert_eq!(
            legal_constructor_slice(snapshot, SemanticType::I64, 0, MAX_CONTEXT_ITEMS as usize)
                .0
                .iter()
                .map(|c| c.code)
                .collect::<Vec<_>>(),
            vec![
                OperationCode::ConstI64,
                OperationCode::AddI64,
                OperationCode::Call,
                OperationCode::If,
                OperationCode::ForI64,
            ]
        );
        assert_eq!(
            legal_constructor_slice(snapshot, SemanticType::Bool, 0, MAX_CONTEXT_ITEMS as usize)
                .0
                .iter()
                .map(|c| c.code)
                .collect::<Vec<_>>(),
            vec![
                OperationCode::ConstBool,
                OperationCode::LtI64,
                OperationCode::If,
                OperationCode::ForI64,
            ]
        );
        assert_eq!(
            legal_constructor_slice(snapshot, SemanticType::Unit, 0, MAX_CONTEXT_ITEMS as usize)
                .0
                .iter()
                .map(|c| c.code)
                .collect::<Vec<_>>(),
            vec![
                OperationCode::ConstUnit,
                OperationCode::If,
                OperationCode::ForI64,
            ]
        );
        let context = repair_context(
            snapshot,
            RepairTarget::Hole(hole),
            ContextBudget {
                body_before: 2,
                body_after: 1,
                visible_values: 8,
                incoming_uses: 8,
                include_incompatible: true,
            },
        )
        .expect("context");
        assert_eq!(context.expected_type, SemanticType::I64);
        assert_eq!(context.incoming_uses.items[0].source, ret);
        assert_eq!(
            context.refinement_operation,
            Some(TransactionOpCode::RefineHole)
        );
        assert!(
            context
                .visible_values
                .items
                .iter()
                .any(|v| v.producer == boolean && !v.compatible)
        );
        let zero_context = repair_context(
            snapshot,
            RepairTarget::Hole(hole),
            ContextBudget {
                body_before: 0,
                body_after: 0,
                visible_values: 0,
                incoming_uses: 0,
                include_incompatible: true,
            },
        )
        .expect("zero context");
        assert!(zero_context.visible_values.items.is_empty());
        let visible_cursor = zero_context
            .visible_values
            .next
            .expect("zero visible continuation");
        assert!(matches!(
            visible_cursor,
            PageCursor::VisibleValues {
                purpose: VisibleCursorPurpose::RepairContext,
                next: 0,
                ..
            }
        ));
        let continued_visible = visible_page(
            snapshot,
            VisibleCursorPurpose::RepairContext,
            RepairTarget::Hole(hole),
            expected,
            loc,
            true,
            PageRequest {
                after: Some(visible_cursor),
                limit: 1,
            },
        )
        .expect("context visible continuation");
        assert_eq!(continued_visible.items[0].producer, forty);
        for purpose in [
            VisibleCursorPurpose::VisibleValues,
            VisibleCursorPurpose::LegalConstructors,
        ] {
            assert_eq!(
                visible_page(
                    snapshot,
                    purpose,
                    RepairTarget::Hole(hole),
                    expected,
                    loc,
                    true,
                    PageRequest {
                        after: Some(visible_cursor),
                        limit: 1
                    }
                )
                .expect_err("cross-purpose visible cursor")
                .code,
                ErrorCode::InvalidCursor
            );
        }
        assert_eq!(
            visible_page(
                snapshot,
                VisibleCursorPurpose::RepairContext,
                RepairTarget::Hole(hole),
                expected,
                loc,
                false,
                PageRequest {
                    after: Some(visible_cursor),
                    limit: 1
                }
            )
            .expect_err("cross-option visible cursor")
            .code,
            ErrorCode::InvalidCursor
        );
        assert_eq!(
            visible_page(
                snapshot,
                VisibleCursorPurpose::RepairContext,
                RepairTarget::Operand {
                    operation: ret,
                    index: 0
                },
                expected,
                operation_location(snapshot, ret).expect("return location"),
                true,
                PageRequest {
                    after: Some(visible_cursor),
                    limit: 1
                }
            )
            .expect_err("cross-target visible cursor")
            .code,
            ErrorCode::InvalidCursor
        );
        let wrong_revision = match visible_cursor {
            PageCursor::VisibleValues {
                workspace,
                purpose,
                target,
                expected,
                include_incompatible,
                next,
                ..
            } => PageCursor::VisibleValues {
                workspace,
                revision: Revision::new(2),
                purpose,
                target,
                expected,
                include_incompatible,
                next,
            },
            _ => unreachable!(),
        };
        assert_eq!(
            visible_page(
                snapshot,
                VisibleCursorPurpose::RepairContext,
                RepairTarget::Hole(hole),
                expected,
                loc,
                true,
                PageRequest {
                    after: Some(wrong_revision),
                    limit: 1
                }
            )
            .expect_err("cross-revision visible cursor")
            .code,
            ErrorCode::InvalidCursor
        );
        assert_eq!(
            uses_page(
                snapshot,
                ValueRef::OperationResult {
                    operation: hole,
                    output: 0
                },
                PageRequest {
                    after: Some(visible_cursor),
                    limit: 1
                }
            )
            .expect_err("cross-family cursor")
            .code,
            ErrorCode::InvalidCursor
        );
        assert!(zero_context.incoming_uses.items.is_empty());
        let incoming_cursor = zero_context
            .incoming_uses
            .next
            .expect("zero incoming continuation");
        assert!(matches!(
            incoming_cursor,
            PageCursor::IncomingUses { next: 0, .. }
        ));
        // Context incoming-use cursors intentionally continue through the exact public IncomingUses query.
        assert_eq!(
            uses_page(
                snapshot,
                ValueRef::OperationResult {
                    operation: hole,
                    output: 0
                },
                PageRequest {
                    after: Some(incoming_cursor),
                    limit: 1
                }
            )
            .expect("context incoming continuation")
            .items[0]
                .source,
            ret
        );
        assert_eq!(
            uses_page(
                snapshot,
                ValueRef::OperationResult {
                    operation: forty,
                    output: 0
                },
                PageRequest {
                    after: Some(incoming_cursor),
                    limit: 1
                }
            )
            .expect_err("cross-target incoming cursor")
            .code,
            ErrorCode::InvalidCursor
        );
        let definitions = definition_page(
            snapshot,
            ids[2],
            PageRequest {
                after: None,
                limit: 8,
            },
        )
        .expect("definition refs");
        assert_eq!(definitions.items.len(), 1);
        assert_eq!(definitions.items[0].source, ids[0]);
        assert_eq!(
            dependencies(snapshot, ret).expect("return deps"),
            vec![DependencyFact::ValueOperand {
                index: 0,
                value: ValueRef::OperationResult {
                    operation: hole,
                    output: 0
                }
            }]
        );
        let operand = repair_context(
            snapshot,
            RepairTarget::Operand {
                operation: ret,
                index: 0,
            },
            ContextBudget {
                body_before: 2,
                body_after: 0,
                visible_values: 8,
                incoming_uses: 8,
                include_incompatible: true,
            },
        )
        .expect("operand context");
        assert_eq!(
            operand.current_value,
            Some(ValueRef::OperationResult {
                operation: hole,
                output: 0
            })
        );
        assert!(
            operand
                .visible_values
                .items
                .iter()
                .any(|v| v.producer == boolean && !v.compatible)
        );
        let cross = PageCursor::Body {
            workspace: WorkspaceId::from_bytes([1; 16]),
            revision: snapshot.revision(),
            block,
            next: 0,
        };
        assert_eq!(
            body_page(
                snapshot,
                block,
                PageRequest {
                    after: Some(cross),
                    limit: 1
                }
            )
            .expect_err("cross workspace")
            .code,
            ErrorCode::InvalidCursor
        );
    }

    #[test]
    fn legal_call_candidates_are_exact_and_paginated() {
        let (mut workspace, ids) = fixture();
        let module = ids[1];
        let hole = ids[8];
        let transaction = Transaction {
            workspace: workspace.id(),
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: (0..70_u32)
                .map(|index| TransactionOp::CreateFunction {
                    handle: LocalHandle::new(100 + index),
                    module: NodeTarget::Existing(module),
                    name: format!("callee-{index:02}"),
                    parameters: Vec::new(),
                    result: SemanticType::I64,
                    body: None,
                })
                .collect(),
        };
        let prepared = workspace
            .prepare_transaction(&ApplyTransactionRequest {
                transaction,
                response: TransactionResponseSpec::default(),
            })
            .expect("callee functions");
        workspace
            .publish(prepared.snapshot)
            .expect("publish callees");
        let snapshot = workspace.head().expect("head");
        let target = RepairTarget::Hole(hole);
        let first = legal_constructor_page(
            snapshot,
            target,
            SemanticType::I64,
            PageRequest {
                after: None,
                limit: 64,
            },
        )
        .expect("first constructor page");
        assert_eq!(first.total, Some(75));
        assert_eq!(first.items.len(), 64);
        let second = legal_constructor_page(
            snapshot,
            target,
            SemanticType::I64,
            PageRequest {
                after: first.next,
                limit: 64,
            },
        )
        .expect("second constructor page");
        assert_eq!(second.items.len(), 11);
        assert!(second.next.is_none());
        let mut all = first.items;
        all.extend(second.items);
        assert_eq!(
            all.iter()
                .filter(|constructor| constructor.code == OperationCode::Call)
                .count(),
            71
        );
        let call_targets = all
            .iter()
            .filter_map(|constructor| constructor.call_target)
            .collect::<Vec<_>>();
        assert!(call_targets.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn structured_repair_context_exposes_region_and_loop_argument_roles() {
        let workspace_id = WorkspaceId::from_bytes([0x68; 16]);
        let workspace = Workspace::new(workspace_id).expect("workspace");
        let local = |handle| NodeTarget::Local(LocalHandle::new(handle));
        let result = |handle| ValueDraft::OperationResult {
            operation: local(handle),
            output: 0,
        };
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: workspace_id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations: vec![
                    TransactionOp::CreatePackage {
                        handle: LocalHandle::new(1),
                        name: "app".into(),
                    },
                    TransactionOp::CreateModule {
                        handle: LocalHandle::new(2),
                        package: local(1),
                        name: "root".into(),
                    },
                    TransactionOp::CreateFunction {
                        handle: LocalHandle::new(3),
                        module: local(2),
                        name: "main".into(),
                        parameters: Vec::new(),
                        result: SemanticType::I64,
                        body: Some(FunctionBodyDraft {
                            operations: vec![
                                ExpressionDraft {
                                    handle: LocalHandle::new(6),
                                    operation: ExpressionKindDraft::ConstI64(0),
                                },
                                ExpressionDraft {
                                    handle: LocalHandle::new(7),
                                    operation: ExpressionKindDraft::ConstI64(10),
                                },
                                ExpressionDraft {
                                    handle: LocalHandle::new(8),
                                    operation: ExpressionKindDraft::ConstBool(true),
                                },
                                ExpressionDraft {
                                    handle: LocalHandle::new(9),
                                    operation: ExpressionKindDraft::ForI64 {
                                        start: result(6),
                                        end_exclusive: result(7),
                                        step: 1,
                                        initial: result(6),
                                        carried: SemanticType::I64,
                                        index_handle: LocalHandle::new(10),
                                        carried_handle: LocalHandle::new(11),
                                        body: YieldingBodyDraft {
                                            operations: vec![ExpressionDraft {
                                                handle: LocalHandle::new(12),
                                                operation: ExpressionKindDraft::If {
                                                    condition: result(8),
                                                    result: SemanticType::I64,
                                                    then_body: YieldingBodyDraft {
                                                        operations: vec![ExpressionDraft {
                                                            handle: LocalHandle::new(13),
                                                            operation: ExpressionKindDraft::Hole {
                                                                expected: SemanticType::I64,
                                                            },
                                                        }],
                                                        yield_value: result(13),
                                                    },
                                                    else_body: YieldingBodyDraft {
                                                        operations: vec![ExpressionDraft {
                                                            handle: LocalHandle::new(14),
                                                            operation:
                                                                ExpressionKindDraft::ConstI64(0),
                                                        }],
                                                        yield_value: result(14),
                                                    },
                                                },
                                            }],
                                            yield_value: result(12),
                                        },
                                    },
                                },
                            ],
                            return_value: result(9),
                        }),
                    },
                    TransactionOp::SetEntryFunction {
                        package: local(1),
                        function: local(3),
                    },
                ],
            },
            response: TransactionResponseSpec {
                return_handles: [3, 9, 10, 11, 12, 13, 14]
                    .into_iter()
                    .map(LocalHandle::new)
                    .collect(),
            },
        };
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("structured query fixture");
        let binding = |handle| {
            prepared
                .receipt
                .returned_bindings
                .iter()
                .find_map(|(candidate, id)| (*candidate == LocalHandle::new(handle)).then_some(*id))
                .expect("binding")
        };
        let context = repair_context(
            &prepared.snapshot,
            RepairTarget::Hole(binding(13)),
            ContextBudget {
                body_before: 2,
                body_after: 2,
                visible_values: 8,
                incoming_uses: 8,
                include_incompatible: true,
            },
        )
        .expect("structured context");
        assert_eq!(
            context
                .enclosing_regions
                .iter()
                .map(|fact| fact.role)
                .collect::<Vec<_>>(),
            vec![RegionRole::IfThen, RegionRole::ForBody]
        );
        assert_eq!(
            context
                .visible_block_arguments
                .iter()
                .map(|fact| fact.role)
                .collect::<Vec<_>>(),
            vec![BlockArgumentRole::LoopIndex, BlockArgumentRole::LoopCarried]
        );
        assert_eq!(
            context
                .visible_block_arguments
                .iter()
                .map(|fact| fact.argument)
                .collect::<Vec<_>>(),
            vec![binding(10), binding(11)]
        );
        assert!(context.visible_block_arguments.iter().all(|fact| {
            fact.region == context.enclosing_regions[1].region && fact.block != context.owner_block
        }));
        let for_item = body_item(&prepared.snapshot, binding(9), 2, false).expect("for body item");
        assert_eq!(for_item.owned_regions.len(), 1);
        assert_eq!(for_item.owned_regions[0].role, RegionRole::ForBody);
        let call = context
            .legal_constructors
            .iter()
            .find(|constructor| constructor.code == OperationCode::Call)
            .expect("call candidate");
        assert_eq!(call.call_target, Some(binding(3)));
        assert!(call.direct_refinement);
        assert!(
            !context
                .legal_constructors
                .iter()
                .find(|constructor| constructor.code == OperationCode::If)
                .expect("if candidate")
                .direct_refinement
        );
        assert!(
            !context
                .legal_constructors
                .iter()
                .find(|constructor| constructor.code == OperationCode::ForI64)
                .expect("for candidate")
                .direct_refinement
        );
    }

    #[test]
    fn thousands_of_incoming_uses_are_paged_deterministically_without_body_rescans() {
        let workspace_id = WorkspaceId::from_bytes([0x67; 16]);
        let mut workspace = Workspace::new(workspace_id).expect("workspace");
        let local = |value| NodeTarget::Local(LocalHandle::new(value));
        let value = ValueDraft::OperationResult {
            operation: local(6),
            output: 0,
        };
        let mut body_operations = vec![ExpressionDraft {
            handle: LocalHandle::new(6),
            operation: ExpressionKindDraft::ConstI64(1),
        }];
        for handle in 100..2100 {
            body_operations.push(ExpressionDraft {
                handle: LocalHandle::new(handle),
                operation: ExpressionKindDraft::AddI64 {
                    lhs: value,
                    rhs: value,
                },
            });
        }
        let operations = vec![
            TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "app".into(),
            },
            TransactionOp::CreateModule {
                handle: LocalHandle::new(2),
                package: local(1),
                name: "root".into(),
            },
            TransactionOp::CreateFunction {
                handle: LocalHandle::new(3),
                module: local(2),
                name: "main".into(),
                parameters: Vec::new(),
                result: SemanticType::I64,
                body: Some(FunctionBodyDraft {
                    operations: body_operations,
                    return_value: value,
                }),
            },
            TransactionOp::SetEntryFunction {
                package: local(1),
                function: local(3),
            },
        ];
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: workspace_id,
                base_revision: Revision::INITIAL,
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations,
            },
            response: TransactionResponseSpec {
                return_handles: vec![LocalHandle::new(6)],
            },
        };
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("large deterministic graph");
        let constant = prepared.receipt.returned_bindings[0].1;
        workspace.publish(prepared.snapshot).expect("publish");
        let snapshot = workspace.head().expect("head");
        let value = ValueRef::OperationResult {
            operation: constant,
            output: 0,
        };
        let mut page = uses_page(
            snapshot,
            value,
            PageRequest {
                after: None,
                limit: 64,
            },
        )
        .expect("first uses");
        assert_eq!(page.total, Some(4001));
        let mut seen = page.items.len();
        assert_eq!(
            (page.items[0].operand_index, page.items[1].operand_index),
            (0, 1)
        );
        while let Some(cursor) = page.next {
            page = uses_page(
                snapshot,
                value,
                PageRequest {
                    after: Some(cursor),
                    limit: 64,
                },
            )
            .expect("continued uses");
            seen += page.items.len();
        }
        assert_eq!(seen, 4001);
    }

    #[test]
    fn batch_policies_partial_outcomes_and_diff_pages_are_deterministic() {
        let (mut workspace, ids) = fixture();
        let id = workspace.id();
        let hole = ids[8];
        let forty = ids[5];
        let two = ids[6];
        let tx = Transaction {
            workspace: id,
            base_revision: Revision::new(1),
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::RefineHole {
                hole: NodeTarget::Existing(hole),
                replacement: OperationDraft::AddI64 {
                    lhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(forty),
                        output: 0,
                    },
                    rhs: ValueDraft::OperationResult {
                        operation: NodeTarget::Existing(two),
                        output: 0,
                    },
                },
            }],
        };
        let req = ApplyTransactionRequest {
            transaction: tx,
            response: TransactionResponseSpec::default(),
        };
        let prepared = workspace.prepare_transaction(&req).expect("refine");
        let receipt = prepared.receipt.clone();
        workspace.publish(prepared.snapshot).expect("publish");
        let before = workspace.snapshot(Revision::new(1)).expect("before");
        let after = workspace.snapshot(Revision::new(2)).expect("after");
        let first_dependency = dependency_page(
            after,
            hole,
            PageRequest {
                after: None,
                limit: 1,
            },
        )
        .expect("first dependency");
        assert_eq!(first_dependency.items.len(), 1);
        let second_dependency = dependency_page(
            after,
            hole,
            PageRequest {
                after: first_dependency.next,
                limit: 1,
            },
        )
        .expect("continued dependency");
        assert_eq!(second_dependency.items.len(), 1);
        assert!(second_dependency.next.is_none());
        let first = diff_page(
            before,
            after,
            PageRequest {
                after: None,
                limit: 1,
            },
        )
        .expect("diff");
        assert_eq!(first.change_count, receipt.change_count);
        assert_eq!(first.change_digest, receipt.change_digest);
        let mut count = first.page.items.len();
        let mut cursor = first.page.next;
        while let Some(c) = cursor {
            let p = diff_page(
                before,
                after,
                PageRequest {
                    after: Some(c),
                    limit: 1,
                },
            )
            .expect("next");
            count += p.page.items.len();
            cursor = p.page.next;
        }
        assert_eq!(count as u64, receipt.change_count);
        let duplicate = QueryBatchRequest {
            workspace: id,
            revision: Revision::new(2),
            queries: vec![
                QueryItem {
                    id: QueryId::new(1),
                    query: Query::WorkspaceSummary,
                },
                QueryItem {
                    id: QueryId::new(1),
                    query: Query::WorkspaceSummary,
                },
            ],
        };
        assert_eq!(
            validate_batch(&duplicate).expect_err("duplicate").code,
            ErrorCode::InvalidQuery
        );
        let summaries = |count: usize| QueryBatchRequest {
            workspace: id,
            revision: Revision::new(2),
            queries: (0..count)
                .map(|i| QueryItem {
                    id: QueryId::new(i as u64),
                    query: Query::WorkspaceSummary,
                })
                .collect(),
        };
        assert_eq!(
            validate_batch(&summaries(33)).expect_err("33 queries").code,
            ErrorCode::PolicyExceeded
        );
        let legal_edge = QueryBatchRequest {
            workspace: id,
            revision: Revision::new(2),
            queries: (0..32)
                .map(|i| QueryItem {
                    id: QueryId::new(i),
                    query: Query::Body {
                        block: ids[4],
                        page: PageRequest {
                            after: None,
                            limit: 64,
                        },
                    },
                })
                .collect(),
        };
        validate_batch(&legal_edge).expect("32 by 64 aggregate edge");
        let aggregate = QueryBatchRequest {
            workspace: id,
            revision: Revision::new(2),
            queries: (0..8)
                .map(|i| QueryItem {
                    id: QueryId::new(i),
                    query: Query::Body {
                        block: ids[4],
                        page: PageRequest {
                            after: None,
                            limit: 256,
                        },
                    },
                })
                .chain(std::iter::once(QueryItem {
                    id: QueryId::new(8),
                    query: Query::WorkspaceSummary,
                }))
                .collect(),
        };
        assert_eq!(
            validate_batch(&aggregate).expect_err("aggregate 2049").code,
            ErrorCode::PolicyExceeded
        );
        let oversized_page = QueryBatchRequest {
            workspace: id,
            revision: Revision::new(2),
            queries: vec![QueryItem {
                id: QueryId::new(1),
                query: Query::Body {
                    block: ids[4],
                    page: PageRequest {
                        after: None,
                        limit: 257,
                    },
                },
            }],
        };
        assert_eq!(
            validate_batch(&oversized_page).expect_err("page 257").code,
            ErrorCode::PolicyExceeded
        );
        let oversized_context = QueryBatchRequest {
            workspace: id,
            revision: Revision::new(2),
            queries: vec![QueryItem {
                id: QueryId::new(1),
                query: Query::RepairContext {
                    target: RepairTarget::Operand {
                        operation: ids[9],
                        index: 0,
                    },
                    budget: ContextBudget {
                        body_before: 65,
                        body_after: 0,
                        visible_values: 0,
                        incoming_uses: 0,
                        include_incompatible: false,
                    },
                },
            }],
        };
        assert_eq!(
            validate_batch(&oversized_context)
                .expect_err("context 65")
                .code,
            ErrorCode::PolicyExceeded
        );
        let ok = execute(after, &Query::WorkspaceSummary, None).expect("success");
        assert!(matches!(ok, QueryResult::WorkspaceSummary(_)));
        let bad = execute(
            after,
            &Query::Body {
                block: hole,
                page: PageRequest {
                    after: None,
                    limit: 1,
                },
            },
            None,
        )
        .expect_err("item error");
        assert_eq!(bad.code, ErrorCode::WrongKind);
    }

    fn publish_operations(workspace: &mut Workspace, operations: Vec<TransactionOp>) {
        let request = ApplyTransactionRequest {
            transaction: Transaction {
                workspace: workspace.id(),
                base_revision: workspace.head_revision(),
                idempotency_key: None,
                mode: TransactionMode::Commit,
                operations,
            },
            response: TransactionResponseSpec::default(),
        };
        let prepared = workspace
            .prepare_transaction(&request)
            .expect("prepare workload");
        workspace
            .publish(prepared.snapshot)
            .expect("publish workload");
    }

    fn sample_query<F>(mut query: F) -> (u128, u128, usize)
    where
        F: FnMut() -> QueryResult,
    {
        let warmup = query();
        std::hint::black_box(&warmup);
        let mut samples = Vec::with_capacity(31);
        let mut last = warmup;
        for _ in 0..31 {
            let started = std::time::Instant::now();
            last = query();
            samples.push(started.elapsed().as_nanos());
            std::hint::black_box(&last);
        }
        samples.sort_unstable();
        let bytes = serde_json::to_vec(&last)
            .expect("measure result bytes")
            .len();
        (samples[15], samples[29], bytes)
    }

    fn sample_batch<F>(mut query: F) -> (u128, u128, usize)
    where
        F: FnMut() -> Vec<QueryResult>,
    {
        let warmup = query();
        std::hint::black_box(&warmup);
        let mut samples = Vec::with_capacity(31);
        let mut last = warmup;
        for _ in 0..31 {
            let started = std::time::Instant::now();
            last = query();
            samples.push(started.elapsed().as_nanos());
            std::hint::black_box(&last);
        }
        samples.sort_unstable();
        let bytes = serde_json::to_vec(&last)
            .expect("measure batch bytes")
            .len();
        (samples[15], samples[29], bytes)
    }

    #[test]
    #[ignore = "manual scan-based query performance measurement"]
    fn query_performance_measurement() {
        let (mut scalar_workspace, scalar_ids) = fixture();
        let scalar_initial = scalar_workspace
            .snapshot(Revision::INITIAL)
            .expect("scalar initial")
            .clone();
        let scalar_before = scalar_workspace
            .snapshot(Revision::new(1))
            .expect("scalar before")
            .clone();
        publish_operations(
            &mut scalar_workspace,
            vec![TransactionOp::RenameNode {
                node: NodeTarget::Existing(scalar_ids[1]),
                name: "renamed".to_owned(),
            }],
        );
        let scalar_after = scalar_workspace.head().expect("scalar after");
        let scalar_block = scalar_ids[4];
        let scalar_hole = scalar_ids[8];
        let context_budget = ContextBudget {
            body_before: 8,
            body_after: 8,
            visible_values: 16,
            incoming_uses: 16,
            include_incompatible: true,
        };
        let summary = sample_query(|| {
            execute(scalar_after, &Query::WorkspaceSummary, None).expect("summary")
        });
        let body = sample_query(|| {
            execute(
                scalar_after,
                &Query::Body {
                    block: scalar_block,
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
                None,
            )
            .expect("body")
        });
        let uses = sample_query(|| {
            execute(
                scalar_after,
                &Query::IncomingUses {
                    value: ValueRef::OperationResult {
                        operation: scalar_hole,
                        output: 0,
                    },
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
                None,
            )
            .expect("uses")
        });
        let context = sample_query(|| {
            execute(
                scalar_after,
                &Query::RepairContext {
                    target: RepairTarget::Hole(scalar_hole),
                    budget: context_budget,
                },
                None,
            )
            .expect("context")
        });
        let adjacent_diff = sample_query(|| {
            execute(
                scalar_after,
                &Query::SemanticDiff {
                    from: Revision::new(1),
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
                Some(&scalar_before),
            )
            .expect("adjacent diff")
        });
        let non_adjacent_diff = sample_query(|| {
            execute(
                scalar_after,
                &Query::SemanticDiff {
                    from: Revision::INITIAL,
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
                Some(&scalar_initial),
            )
            .expect("non-adjacent diff")
        });
        let batch_request = QueryBatchRequest {
            workspace: scalar_after.workspace(),
            revision: scalar_after.revision(),
            queries: vec![
                QueryItem {
                    id: QueryId::new(1),
                    query: Query::WorkspaceSummary,
                },
                QueryItem {
                    id: QueryId::new(2),
                    query: Query::Body {
                        block: scalar_block,
                        page: PageRequest {
                            after: None,
                            limit: 32,
                        },
                    },
                },
                QueryItem {
                    id: QueryId::new(3),
                    query: Query::IncomingUses {
                        value: ValueRef::OperationResult {
                            operation: scalar_hole,
                            output: 0,
                        },
                        page: PageRequest {
                            after: None,
                            limit: 32,
                        },
                    },
                },
                QueryItem {
                    id: QueryId::new(4),
                    query: Query::RepairContext {
                        target: RepairTarget::Hole(scalar_hole),
                        budget: context_budget,
                    },
                },
            ],
        };
        let batch = sample_batch(|| {
            validate_batch(&batch_request).expect("valid measured batch");
            batch_request
                .queries
                .iter()
                .map(|item| execute(scalar_after, &item.query, None).expect("batch item"))
                .collect()
        });

        let (mut body_workspace, body_ids) = fixture();
        let body_block = body_ids[4];
        let body_hole = body_ids[8];
        let body_operations = (0..3_000_u32)
            .map(|index| TransactionOp::InsertExpression {
                block: body_block,
                before: Some(body_hole),
                expression: ExpressionDraft {
                    handle: LocalHandle::new(10_000 + index),
                    operation: ExpressionKindDraft::ConstI64(i64::from(index)),
                },
            })
            .collect();
        publish_operations(&mut body_workspace, body_operations);
        let body_snapshot = body_workspace.head().expect("large body");
        let large_body = sample_query(|| {
            execute(
                body_snapshot,
                &Query::Body {
                    block: body_block,
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
                None,
            )
            .expect("large body page")
        });
        let large_body_context = sample_query(|| {
            execute(
                body_snapshot,
                &Query::RepairContext {
                    target: RepairTarget::Hole(body_hole),
                    budget: context_budget,
                },
                None,
            )
            .expect("large body context")
        });

        let (mut unrelated_workspace, unrelated_ids) = fixture();
        let unrelated_hole = unrelated_ids[8];
        let unrelated_operations = (0..3_000_u32)
            .map(|index| TransactionOp::CreatePackage {
                handle: LocalHandle::new(20_000 + index),
                name: format!("unrelated-{index:04}"),
            })
            .collect();
        publish_operations(&mut unrelated_workspace, unrelated_operations);
        let unrelated_snapshot = unrelated_workspace.head().expect("unrelated graph");
        let unrelated_context = sample_query(|| {
            execute(
                unrelated_snapshot,
                &Query::RepairContext {
                    target: RepairTarget::Hole(unrelated_hole),
                    budget: context_budget,
                },
                None,
            )
            .expect("unrelated context")
        });
        let unrelated_uses = sample_query(|| {
            execute(
                unrelated_snapshot,
                &Query::IncomingUses {
                    value: ValueRef::OperationResult {
                        operation: unrelated_hole,
                        output: 0,
                    },
                    page: PageRequest {
                        after: None,
                        limit: 256,
                    },
                },
                None,
            )
            .expect("unrelated uses")
        });

        let measurement = |value: (u128, u128, usize)| {
            serde_json::json!({
                "median_ns": value.0,
                "p95_ns": value.1,
                "json_result_bytes": value.2,
                "samples": 31,
            })
        };
        println!(
            "QUERY_PERFORMANCE {}",
            serde_json::json!({
                "implementation": "full_scans_no_index_or_cache",
                "scalar": {
                    "nodes": scalar_after.node_count(),
                    "workspace_summary": measurement(summary),
                    "body": measurement(body),
                    "incoming_uses": measurement(uses),
                    "repair_context": measurement(context),
                    "adjacent_diff": measurement(adjacent_diff),
                    "non_adjacent_diff_0_to_2": measurement(non_adjacent_diff),
                    "four_item_batch": measurement(batch),
                },
                "large_body": {
                    "nodes": body_snapshot.node_count(),
                    "operations_added": 3000,
                    "body_first_256": measurement(large_body),
                    "repair_context": measurement(large_body_context),
                },
                "unrelated_graph": {
                    "nodes": unrelated_snapshot.node_count(),
                    "unrelated_packages_added": 3000,
                    "repair_context": measurement(unrelated_context),
                    "incoming_uses": measurement(unrelated_uses),
                },
            })
        );
    }
}
