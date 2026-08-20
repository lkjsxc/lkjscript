//! Closed project-level semantic query plans for bounded navigation.
//!
//! This module is a derived observation owner over one immutable snapshot. It deliberately reuses
//! the low-level query vocabulary and never owns accepted meaning, an index, or a cache.

use crate::error::{ErrorCode, LkError, Result};
use crate::graph::Snapshot;
use crate::ids::{NodeId, Revision, SnapshotHash, WorkspaceId};
use crate::query::{
    CompletenessBlocker, DefinitionReferenceSite, DependencyFact, NodeView, OwnerFact, PageCursor,
    PageRequest, Query, QueryResult, UseSite,
};
use crate::schema::{Node, NodeKind, OperationKind, ValueRef};
use crate::target::TargetSummary;
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const SEMANTIC_QUERY_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_SEMANTIC_QUERY_ROOTS: usize = 8;
pub const MAXIMUM_SEMANTIC_QUERY_PAGE_ITEMS: u32 = 256;
pub const MAXIMUM_SEMANTIC_QUERY_WORK_ITEMS: usize = 4_096;
pub const MAXIMUM_SEMANTIC_QUERY_CONTINUATION_BYTES: usize = 2_048;
pub const MAXIMUM_SEMANTIC_QUERY_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

const PLAN_DIGEST_DOMAIN: &str = "lkjscript.semantic-query-plan.v1";
const RESULT_DIGEST_DOMAIN: &str = "lkjscript.semantic-query-result.v1";
const CONTINUATION_DIGEST_DOMAIN: &str = "lkjscript.semantic-query-continuation.v1";

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SemanticProjection {
    Summary,
    Exact,
    Children,
    Function,
    OwnerChain,
    Dependencies,
    IncomingUses,
    Callers,
    Callees,
    Targets,
    Blockers,
}

impl SemanticProjection {
    pub const ALL: [Self; 11] = [
        Self::Summary,
        Self::Exact,
        Self::Children,
        Self::Function,
        Self::OwnerChain,
        Self::Dependencies,
        Self::IncomingUses,
        Self::Callers,
        Self::Callees,
        Self::Targets,
        Self::Blockers,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Summary => "summary",
            Self::Exact => "exact",
            Self::Children => "children",
            Self::Function => "function",
            Self::OwnerChain => "owner_chain",
            Self::Dependencies => "dependencies",
            Self::IncomingUses => "incoming_uses",
            Self::Callers => "callers",
            Self::Callees => "callees",
            Self::Targets => "targets",
            Self::Blockers => "blockers",
        }
    }
}

impl std::str::FromStr for SemanticProjection {
    type Err = &'static str;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|projection| projection.machine_name() == value)
            .ok_or("unknown semantic query projection")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticQueryRequest {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub projection: SemanticProjection,
    pub roots: Vec<NodeId>,
    pub limit: u32,
    pub continuation: Option<String>,
}

impl SemanticQueryRequest {
    pub fn new(workspace: WorkspaceId, revision: Revision, projection: SemanticProjection) -> Self {
        Self {
            workspace,
            revision,
            projection,
            roots: Vec::new(),
            limit: 64,
            continuation: None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticQueryPlanFacts {
    pub projection: SemanticProjection,
    pub roots: Vec<NodeId>,
    pub limit: u32,
    pub plan_digest: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticQueryOmissions {
    pub work_items: u64,
    pub total_items: u64,
    pub returned_items: u64,
    pub truncated: bool,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CallFact {
    pub operation: NodeId,
    pub function: NodeId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
#[allow(clippy::large_enum_variant)] // Query pages are strictly bounded; direct values avoid a second projection shape.
pub enum SemanticQueryItem {
    Node(NodeView),
    Owner(OwnerFact),
    Dependency(DependencyFact),
    Use(UseSite),
    DefinitionReference(DefinitionReferenceSite),
    Call(CallFact),
    Target(TargetSummary),
    Blocker(CompletenessBlocker),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct SemanticQueryPage {
    pub version: u16,
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub snapshot: SnapshotHash,
    pub plan: SemanticQueryPlanFacts,
    pub result_digest: String,
    pub items: Vec<SemanticQueryItem>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub continuation: Option<String>,
    pub omissions: SemanticQueryOmissions,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SemanticQueryResult {
    Changed(Box<SemanticQueryPage>),
    Unchanged {
        version: u16,
        workspace: WorkspaceId,
        revision: Revision,
        snapshot: SnapshotHash,
        plan_digest: String,
        result_digest: String,
    },
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct PlanDigestInput<'a> {
    version: u16,
    workspace: WorkspaceId,
    revision: Revision,
    snapshot: SnapshotHash,
    projection: SemanticProjection,
    roots: &'a [NodeId],
    limit: u32,
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ContinuationPayload {
    version: u16,
    workspace: WorkspaceId,
    revision: Revision,
    snapshot: SnapshotHash,
    projection: SemanticProjection,
    roots: Vec<NodeId>,
    limit: u32,
    plan_digest: String,
    next_offset: u64,
}

#[derive(Serialize)]
#[serde(deny_unknown_fields)]
struct ResultDigestInput<'a> {
    version: u16,
    workspace: WorkspaceId,
    revision: Revision,
    snapshot: SnapshotHash,
    plan: &'a SemanticQueryPlanFacts,
    items: &'a [SemanticQueryItem],
    continuation: &'a Option<String>,
    omissions: SemanticQueryOmissions,
}

pub fn build_semantic_query(
    snapshot: &Snapshot,
    request: &SemanticQueryRequest,
    known_digest: Option<&str>,
) -> Result<SemanticQueryResult> {
    validate_request(snapshot, request)?;
    let plan_digest = digest_json(
        PLAN_DIGEST_DOMAIN,
        &PlanDigestInput {
            version: SEMANTIC_QUERY_CONTRACT_VERSION,
            workspace: request.workspace,
            revision: request.revision,
            snapshot: snapshot.hash(),
            projection: request.projection,
            roots: &request.roots,
            limit: request.limit,
        },
    )?;
    let offset = match request.continuation.as_deref() {
        Some(token) => {
            let continuation = decode_continuation(token)?;
            if continuation.version != SEMANTIC_QUERY_CONTRACT_VERSION
                || continuation.workspace != request.workspace
                || continuation.revision != request.revision
                || continuation.snapshot != snapshot.hash()
                || continuation.projection != request.projection
                || continuation.roots != request.roots
                || continuation.limit != request.limit
                || continuation.plan_digest != plan_digest
            {
                return Err(LkError::new(
                    ErrorCode::InvalidCursor,
                    "semantic query continuation does not match the exact query plan",
                )
                .for_workspace(request.workspace)
                .at_revision(request.revision));
            }
            continuation.next_offset
        }
        None => 0,
    };

    let (candidates, work_items) = collect_candidates(snapshot, request)?;
    let total = u64::try_from(candidates.len()).unwrap_or(u64::MAX);
    let start = usize::try_from(offset).map_err(|_| {
        LkError::new(
            ErrorCode::InvalidCursor,
            "semantic query continuation offset is not representable",
        )
    })?;
    if start > candidates.len() {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "semantic query continuation is beyond the exact result",
        ));
    }
    let end = start
        .saturating_add(request.limit as usize)
        .min(candidates.len());
    let items = candidates[start..end].to_vec();
    let continuation = if end < candidates.len() {
        Some(encode_continuation(&ContinuationPayload {
            version: SEMANTIC_QUERY_CONTRACT_VERSION,
            workspace: request.workspace,
            revision: request.revision,
            snapshot: snapshot.hash(),
            projection: request.projection,
            roots: request.roots.clone(),
            limit: request.limit,
            plan_digest: plan_digest.clone(),
            next_offset: u64::try_from(end).unwrap_or(u64::MAX),
        })?)
    } else {
        None
    };
    let omissions = SemanticQueryOmissions {
        work_items: u64::try_from(work_items).unwrap_or(u64::MAX),
        total_items: total,
        returned_items: u64::try_from(items.len()).unwrap_or(u64::MAX),
        truncated: continuation.is_some(),
    };
    let plan = SemanticQueryPlanFacts {
        projection: request.projection,
        roots: request.roots.clone(),
        limit: request.limit,
        plan_digest,
    };
    let result_digest = digest_json(
        RESULT_DIGEST_DOMAIN,
        &ResultDigestInput {
            version: SEMANTIC_QUERY_CONTRACT_VERSION,
            workspace: request.workspace,
            revision: request.revision,
            snapshot: snapshot.hash(),
            plan: &plan,
            items: &items,
            continuation: &continuation,
            omissions,
        },
    )?;
    if let Some(known) = known_digest {
        validate_digest_spelling(known)?;
        if known == result_digest {
            return Ok(SemanticQueryResult::Unchanged {
                version: SEMANTIC_QUERY_CONTRACT_VERSION,
                workspace: request.workspace,
                revision: request.revision,
                snapshot: snapshot.hash(),
                plan_digest: plan.plan_digest,
                result_digest,
            });
        }
    }
    let page = SemanticQueryPage {
        version: SEMANTIC_QUERY_CONTRACT_VERSION,
        workspace: request.workspace,
        revision: request.revision,
        snapshot: snapshot.hash(),
        plan,
        result_digest,
        items,
        continuation,
        omissions,
    };
    let encoded = serde_json::to_vec(&page).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode semantic query result: {error}"),
        )
    })?;
    if encoded.len() > MAXIMUM_SEMANTIC_QUERY_RESPONSE_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic query response exceeds the byte policy",
        ));
    }
    Ok(SemanticQueryResult::Changed(Box::new(page)))
}

fn validate_request(snapshot: &Snapshot, request: &SemanticQueryRequest) -> Result<()> {
    if request.workspace != snapshot.workspace() || request.revision != snapshot.revision() {
        return Err(LkError::new(
            ErrorCode::RevisionConflict,
            "semantic query domain does not match the selected snapshot",
        )
        .for_workspace(request.workspace)
        .at_revision(request.revision));
    }
    if request.roots.len() > MAXIMUM_SEMANTIC_QUERY_ROOTS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic query exceeds the root-count policy",
        ));
    }
    if request.limit == 0 || request.limit > MAXIMUM_SEMANTIC_QUERY_PAGE_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic query page limit must be in 1..=256",
        ));
    }
    if request
        .roots
        .iter()
        .any(|root| root.workspace() != request.workspace)
    {
        return Err(LkError::new(
            ErrorCode::WrongWorkspace,
            "semantic query root belongs to another workspace",
        ));
    }
    if request.roots.iter().copied().collect::<BTreeSet<_>>().len() != request.roots.len() {
        return Err(LkError::new(
            ErrorCode::InvalidQuery,
            "semantic query roots must be unique",
        ));
    }
    let needs_roots = !matches!(
        request.projection,
        SemanticProjection::Targets | SemanticProjection::Blockers
    );
    if needs_roots && request.roots.is_empty() {
        return Err(LkError::new(
            ErrorCode::InvalidQuery,
            "selected semantic query projection requires at least one root",
        ));
    }
    if !needs_roots && !request.roots.is_empty() {
        return Err(LkError::new(
            ErrorCode::InvalidQuery,
            "selected semantic query projection does not accept roots",
        ));
    }
    for root in &request.roots {
        snapshot.node(*root)?;
    }
    if matches!(request.projection, SemanticProjection::Function)
        && (request.roots.len() != 1
            || snapshot.node(request.roots[0])?.kind() != NodeKind::Function)
    {
        return Err(LkError::new(
            ErrorCode::WrongKind,
            "function projection requires exactly one function root",
        ));
    }
    Ok(())
}

fn collect_candidates(
    snapshot: &Snapshot,
    request: &SemanticQueryRequest,
) -> Result<(Vec<SemanticQueryItem>, usize)> {
    let mut work = 0_usize;
    let mut items = Vec::new();
    match request.projection {
        SemanticProjection::Summary | SemanticProjection::Exact => {
            let expand = request.projection == SemanticProjection::Exact;
            for root in &request.roots {
                charge_work(&mut work, 1)?;
                items.push(SemanticQueryItem::Node(crate::query::node_view(
                    snapshot, *root, expand,
                )?));
            }
        }
        SemanticProjection::Children => {
            for root in &request.roots {
                let node = snapshot.node(*root)?;
                charge_work(&mut work, node.owned_child_count().saturating_add(1))?;
                for index in 0..node.owned_child_count() {
                    let child = node.owned_child(index).ok_or_else(|| {
                        LkError::new(
                            ErrorCode::ArtifactCorrupt,
                            "semantic node omitted a counted child",
                        )
                    })?;
                    items.push(SemanticQueryItem::Node(crate::query::node_view(
                        snapshot, child, false,
                    )?));
                }
            }
        }
        SemanticProjection::Function => {
            let mut stack = vec![request.roots[0]];
            let mut seen = BTreeSet::new();
            while let Some(node_id) = stack.pop() {
                if !seen.insert(node_id) {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "function ownership tree contains a duplicate or cycle",
                    ));
                }
                charge_work(&mut work, 1)?;
                let node = snapshot.node(node_id)?;
                let mut children = Vec::with_capacity(node.owned_child_count());
                for index in 0..node.owned_child_count() {
                    children.push(node.owned_child(index).ok_or_else(|| {
                        LkError::new(
                            ErrorCode::ArtifactCorrupt,
                            "function ownership tree omitted a counted child",
                        )
                    })?);
                }
                for child in children.into_iter().rev() {
                    stack.push(child);
                }
                items.push(SemanticQueryItem::Node(crate::query::node_view(
                    snapshot, node_id, true,
                )?));
            }
        }
        SemanticProjection::OwnerChain => {
            for root in &request.roots {
                collect_query_pages(
                    snapshot,
                    Query::OwnerChain {
                        node: *root,
                        page: first_query_page(),
                    },
                    &mut work,
                    &mut items,
                    |result| match result {
                        QueryResult::OwnerChain(page) => {
                            let observed = page.items.len();
                            Some((
                                page.items
                                    .into_iter()
                                    .map(SemanticQueryItem::Owner)
                                    .collect(),
                                page.next,
                                observed,
                            ))
                        }
                        _ => None,
                    },
                )?;
            }
        }
        SemanticProjection::Dependencies => {
            for root in &request.roots {
                collect_query_pages(
                    snapshot,
                    Query::Dependencies {
                        node: *root,
                        page: first_query_page(),
                    },
                    &mut work,
                    &mut items,
                    |result| match result {
                        QueryResult::Dependencies(page) => {
                            let observed = page.items.len();
                            Some((
                                page.items
                                    .into_iter()
                                    .map(SemanticQueryItem::Dependency)
                                    .collect(),
                                page.next,
                                observed,
                            ))
                        }
                        _ => None,
                    },
                )?;
            }
        }
        SemanticProjection::IncomingUses => {
            for root in &request.roots {
                let value = value_ref_for_node(snapshot, *root)?;
                collect_query_pages(
                    snapshot,
                    Query::IncomingUses {
                        value,
                        page: first_query_page(),
                    },
                    &mut work,
                    &mut items,
                    |result| match result {
                        QueryResult::IncomingUses(page) => {
                            let observed = page.items.len();
                            Some((
                                page.items.into_iter().map(SemanticQueryItem::Use).collect(),
                                page.next,
                                observed,
                            ))
                        }
                        _ => None,
                    },
                )?;
            }
        }
        SemanticProjection::Callers => {
            for root in &request.roots {
                if snapshot.node(*root)?.kind() != NodeKind::Function {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "callers projection requires function roots",
                    )
                    .for_node(*root));
                }
                collect_query_pages(
                    snapshot,
                    Query::DefinitionReferences {
                        target: *root,
                        page: first_query_page(),
                    },
                    &mut work,
                    &mut items,
                    |result| match result {
                        QueryResult::DefinitionReferences(page) => {
                            let observed = page.items.len();
                            Some((
                                page.items
                                    .into_iter()
                                    .filter(|site| {
                                        site.slot == crate::query::DefinitionSlot::CallTarget
                                    })
                                    .map(SemanticQueryItem::DefinitionReference)
                                    .collect(),
                                page.next,
                                observed,
                            ))
                        }
                        _ => None,
                    },
                )?;
            }
        }
        SemanticProjection::Callees => {
            for root in &request.roots {
                if snapshot.node(*root)?.kind() != NodeKind::Function {
                    return Err(LkError::new(
                        ErrorCode::WrongKind,
                        "callees projection requires function roots",
                    )
                    .for_node(*root));
                }
                let mut stack = vec![*root];
                let mut seen = BTreeSet::new();
                while let Some(node_id) = stack.pop() {
                    if !seen.insert(node_id) {
                        return Err(LkError::new(
                            ErrorCode::ArtifactCorrupt,
                            "function call traversal contains a duplicate or cycle",
                        )
                        .for_node(node_id));
                    }
                    charge_work(&mut work, 1)?;
                    let node = snapshot.node(node_id)?;
                    if let Node::Operation {
                        operation: OperationKind::Call { function, .. },
                        ..
                    } = node
                    {
                        items.push(SemanticQueryItem::Call(CallFact {
                            operation: node_id,
                            function: *function,
                        }));
                    }
                    for index in (0..node.owned_child_count()).rev() {
                        stack.push(node.owned_child(index).ok_or_else(|| {
                            LkError::new(
                                ErrorCode::ArtifactCorrupt,
                                "function call traversal omitted a counted child",
                            )
                        })?);
                    }
                }
            }
            items.sort_by(|left, right| match (left, right) {
                (SemanticQueryItem::Call(left), SemanticQueryItem::Call(right)) => left.cmp(right),
                _ => std::cmp::Ordering::Equal,
            });
        }
        SemanticProjection::Targets => {
            let targets = crate::target::summaries(snapshot);
            charge_work(&mut work, targets.len())?;
            items.extend(targets.into_iter().map(SemanticQueryItem::Target));
        }
        SemanticProjection::Blockers => {
            let blockers = crate::query::workspace_blockers(snapshot);
            charge_work(&mut work, blockers.len())?;
            items.extend(blockers.into_iter().map(SemanticQueryItem::Blocker));
        }
    }
    Ok((items, work))
}

fn collect_query_pages(
    snapshot: &Snapshot,
    mut query: Query,
    work: &mut usize,
    output: &mut Vec<SemanticQueryItem>,
    mut extract: impl FnMut(QueryResult) -> Option<(Vec<SemanticQueryItem>, Option<PageCursor>, usize)>,
) -> Result<()> {
    loop {
        let result = crate::query::execute(snapshot, &query, None)?;
        let (items, next, observed) = extract(result).ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                "semantic query owner returned an unexpected result family",
            )
        })?;
        charge_work(work, observed)?;
        output.extend(items);
        let Some(cursor) = next else {
            return Ok(());
        };
        query = query_with_cursor(query, cursor)?;
    }
}

fn query_with_cursor(query: Query, cursor: PageCursor) -> Result<Query> {
    let page = PageRequest {
        after: Some(cursor),
        limit: crate::query::MAX_PAGE_ITEMS,
    };
    match query {
        Query::OwnerChain { node, .. } => Ok(Query::OwnerChain { node, page }),
        Query::Dependencies { node, .. } => Ok(Query::Dependencies { node, page }),
        Query::IncomingUses { value, .. } => Ok(Query::IncomingUses { value, page }),
        Query::DefinitionReferences { target, .. } => {
            Ok(Query::DefinitionReferences { target, page })
        }
        _ => Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "semantic query continuation cannot be applied to this projection",
        )),
    }
}

fn first_query_page() -> PageRequest {
    PageRequest {
        after: None,
        limit: crate::query::MAX_PAGE_ITEMS,
    }
}

fn value_ref_for_node(snapshot: &Snapshot, node: NodeId) -> Result<ValueRef> {
    match snapshot.node(node)? {
        Node::Parameter { .. } => Ok(ValueRef::FunctionParameter(node)),
        Node::BlockArgument { .. } => Ok(ValueRef::BlockArgument(node)),
        Node::Operation { operation, .. } if operation.result_count() != 0 => {
            Ok(ValueRef::OperationResult {
                operation: node,
                output: 0,
            })
        }
        _ => Err(LkError::new(
            ErrorCode::WrongKind,
            "incoming-uses projection requires a parameter, block argument, or value operation",
        )
        .for_node(node)),
    }
}

fn charge_work(work: &mut usize, amount: usize) -> Result<()> {
    *work = work.checked_add(amount).ok_or_else(|| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic query work counter overflowed",
        )
    })?;
    if *work > MAXIMUM_SEMANTIC_QUERY_WORK_ITEMS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic query exceeds the work-item policy",
        ));
    }
    Ok(())
}

fn encode_continuation(payload: &ContinuationPayload) -> Result<String> {
    let body = serde_json::to_vec(payload).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode semantic query continuation: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(CONTINUATION_DIGEST_DOMAIN);
    hasher.update(&body);
    let mut bytes = body;
    bytes.extend_from_slice(hasher.finalize().as_bytes());
    let encoded = URL_SAFE_NO_PAD.encode(bytes);
    if encoded.len() > MAXIMUM_SEMANTIC_QUERY_CONTINUATION_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "semantic query continuation exceeds the byte policy",
        ));
    }
    Ok(encoded)
}

fn decode_continuation(encoded: &str) -> Result<ContinuationPayload> {
    if encoded.is_empty() || encoded.len() > MAXIMUM_SEMANTIC_QUERY_CONTINUATION_BYTES {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "semantic query continuation length is invalid",
        ));
    }
    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
        LkError::new(
            ErrorCode::InvalidCursor,
            "semantic query continuation encoding is invalid",
        )
    })?;
    if bytes.len() < 32 {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "semantic query continuation is truncated",
        ));
    }
    let split = bytes.len() - 32;
    let (body, claimed) = bytes.split_at(split);
    let mut hasher = blake3::Hasher::new_derive_key(CONTINUATION_DIGEST_DOMAIN);
    hasher.update(body);
    if hasher.finalize().as_bytes() != claimed {
        return Err(LkError::new(
            ErrorCode::InvalidCursor,
            "semantic query continuation digest is invalid",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(body);
    let payload = ContinuationPayload::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::InvalidCursor,
            format!("semantic query continuation payload is invalid: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::InvalidCursor,
            format!("semantic query continuation has trailing input: {error}"),
        )
    })?;
    Ok(payload)
}

fn digest_json(domain: &str, value: &impl Serialize) -> Result<String> {
    let bytes = serde_json::to_vec(value).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode semantic query digest input: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&bytes);
    Ok(hasher.finalize().to_hex().to_string())
}

fn validate_digest_spelling(value: &str) -> Result<()> {
    if value.len() != 64
        || !value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(LkError::new(
            ErrorCode::InvalidQuery,
            "known semantic query digest must be 64 lowercase hexadecimal characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Snapshot;

    #[test]
    fn projection_names_are_unique_and_closed() {
        let names = SemanticProjection::ALL
            .into_iter()
            .map(SemanticProjection::machine_name)
            .collect::<BTreeSet<_>>();
        assert_eq!(names.len(), SemanticProjection::ALL.len());
        for projection in SemanticProjection::ALL {
            assert_eq!(projection.machine_name().parse(), Ok(projection));
        }
    }

    #[test]
    fn root_children_are_paged_and_continuation_is_exact() {
        let snapshot = Snapshot::initial(WorkspaceId::from_bytes([7; 16]))
            .expect("initial snapshot must be valid");
        let mut request = SemanticQueryRequest::new(
            snapshot.workspace(),
            snapshot.revision(),
            SemanticProjection::Children,
        );
        request.roots.push(snapshot.root());
        request.limit = 1;
        let first = build_semantic_query(&snapshot, &request, None).expect("query succeeds");
        let SemanticQueryResult::Changed(first) = first else {
            panic!("first query unexpectedly unchanged");
        };
        assert_eq!(first.items.len(), 0);
        assert!(first.continuation.is_none());

        request.projection = SemanticProjection::Summary;
        let summary = build_semantic_query(&snapshot, &request, None).expect("summary succeeds");
        let SemanticQueryResult::Changed(summary) = summary else {
            panic!("summary unexpectedly unchanged");
        };
        let unchanged = build_semantic_query(&snapshot, &request, Some(&summary.result_digest))
            .expect("known digest succeeds");
        assert!(matches!(unchanged, SemanticQueryResult::Unchanged { .. }));
    }

    #[test]
    fn malformed_continuation_rejects() {
        let snapshot = Snapshot::initial(WorkspaceId::from_bytes([9; 16]))
            .expect("initial snapshot must be valid");
        let mut request = SemanticQueryRequest::new(
            snapshot.workspace(),
            snapshot.revision(),
            SemanticProjection::Summary,
        );
        request.roots.push(snapshot.root());
        request.continuation = Some("not-a-valid-continuation".into());
        let error = build_semantic_query(&snapshot, &request, None).expect_err("must reject");
        assert_eq!(error.code, ErrorCode::InvalidCursor);
    }
}
