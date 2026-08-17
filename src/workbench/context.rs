use super::{MAX_CONTEXT_PACKET_BYTES, WORKBENCH_VERSION};
use crate::error::{ErrorCode, LkError, Result};
use crate::ids::{NodeId, QueryId, RequestId, Revision, WorkspaceId};
use crate::machine::{MachineSchemaDigest, active_machine_schema_digest};
use crate::protocol::{Request, Response};
use crate::query::{
    CompletenessBlocker, ContextBudget, NamePreview, Page, PageRequest, Query, QueryBatchRequest,
    QueryItem, QueryOutcome, QueryResult, RepairTarget, SemanticDiffPage, WorkspaceSummary,
};
use crate::schema::{Node, NodeKind, ValueRef};
use crate::transaction::{ExpressionDraftCode, TransactionOpCode};
use crate::transport::Client;
use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;
use std::path::Path;
use std::str::FromStr;

const CONTEXT_PACKET_DIGEST_DOMAIN: &[u8] = b"lkjscript.context-packet.v1\0";
const DEFAULT_MAX_CONTEXT_NODES: u32 = 64;
const MAX_CONTEXT_NODES: u32 = 256;
const MAX_CONTEXT_TARGETS: usize = 8;
const CONTEXT_PAGE_ITEMS: u32 = 64;

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ContextPacketDigest([u8; 32]);

impl ContextPacketDigest {
    pub const BYTE_LEN: usize = 32;

    pub const fn from_bytes(bytes: [u8; Self::BYTE_LEN]) -> Self {
        Self(bytes)
    }

    pub const fn as_bytes(self) -> [u8; Self::BYTE_LEN] {
        self.0
    }
}

impl fmt::Display for ContextPacketDigest {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut output = [0_u8; 64];
        for (index, byte) in self.0.iter().copied().enumerate() {
            output[index * 2] = HEX[usize::from(byte >> 4)];
            output[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
        }
        formatter.write_str(std::str::from_utf8(&output).map_err(|_| fmt::Error)?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ContextPacketDigestParseError;

impl fmt::Display for ContextPacketDigestParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .write_str("context packet digest must be exactly 64 lowercase hexadecimal characters")
    }
}

impl std::error::Error for ContextPacketDigestParseError {}

impl FromStr for ContextPacketDigest {
    type Err = ContextPacketDigestParseError;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        if value.len() != 64 {
            return Err(ContextPacketDigestParseError);
        }
        let mut bytes = [0_u8; 32];
        for (index, pair) in value.as_bytes().chunks_exact(2).enumerate() {
            let digit = |byte| match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            };
            let high = digit(pair[0]).ok_or(ContextPacketDigestParseError)?;
            let low = digit(pair[1]).ok_or(ContextPacketDigestParseError)?;
            bytes[index] = (high << 4) | low;
        }
        Ok(Self(bytes))
    }
}

impl Serialize for ContextPacketDigest {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ContextPacketDigest {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct DigestVisitor;
        impl Visitor<'_> for DigestVisitor {
            type Value = ContextPacketDigest;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a canonical lowercase context packet digest")
            }

            fn visit_str<E>(self, value: &str) -> std::result::Result<Self::Value, E>
            where
                E: de::Error,
            {
                value.parse().map_err(E::custom)
            }
        }
        deserializer.deserialize_str(DigestVisitor)
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextPurpose {
    Orient,
    Create,
    Repair,
    Refactor,
    Debug,
    Extend,
    Delete,
    Review,
}

impl ContextPurpose {
    pub const ALL: [Self; 8] = [
        Self::Orient,
        Self::Create,
        Self::Repair,
        Self::Refactor,
        Self::Debug,
        Self::Extend,
        Self::Delete,
        Self::Review,
    ];

    pub const fn machine_name(self) -> &'static str {
        match self {
            Self::Orient => "orient",
            Self::Create => "create",
            Self::Repair => "repair",
            Self::Refactor => "refactor",
            Self::Debug => "debug",
            Self::Extend => "extend",
            Self::Delete => "delete",
            Self::Review => "review",
        }
    }
}

impl FromStr for ContextPurpose {
    type Err = &'static str;

    fn from_str(value: &str) -> std::result::Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|purpose| purpose.machine_name() == value)
            .ok_or("unknown context purpose")
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContextBuildRequest {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub purpose: ContextPurpose,
    pub targets: Vec<NodeId>,
    pub from_revision: Option<Revision>,
    pub maximum_nodes: u32,
}

impl ContextBuildRequest {
    pub fn new(workspace: WorkspaceId, revision: Revision, purpose: ContextPurpose) -> Self {
        Self {
            workspace,
            revision,
            purpose,
            targets: Vec::new(),
            from_revision: None,
            maximum_nodes: DEFAULT_MAX_CONTEXT_NODES,
        }
    }

    fn validate(&self) -> Result<()> {
        if self.targets.len() > MAX_CONTEXT_TARGETS {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "context request exceeds the target-count policy",
            ));
        }
        if self.maximum_nodes == 0 || self.maximum_nodes > MAX_CONTEXT_NODES {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "context maximum_nodes must be in 1..=256",
            ));
        }
        if self
            .targets
            .iter()
            .any(|target| target.workspace() != self.workspace)
        {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "context target belongs to another workspace",
            ));
        }
        let target_required = matches!(
            self.purpose,
            ContextPurpose::Repair
                | ContextPurpose::Refactor
                | ContextPurpose::Debug
                | ContextPurpose::Extend
                | ContextPurpose::Delete
        );
        if target_required && self.targets.is_empty() {
            return Err(LkError::new(
                ErrorCode::InvalidQuery,
                "the selected context purpose requires at least one target",
            ));
        }
        if self.purpose == ContextPurpose::Repair && self.targets.len() != 1 {
            return Err(LkError::new(
                ErrorCode::InvalidQuery,
                "repair context requires exactly one hole target",
            ));
        }
        if self.from_revision.is_some() && self.purpose != ContextPurpose::Review {
            return Err(LkError::new(
                ErrorCode::InvalidQuery,
                "from_revision is accepted only for review context",
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextAlias {
    pub alias: String,
    pub node: NodeId,
    pub kind: NodeKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<NamePreview>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContextObservationRole {
    OwnerChain,
    Dependencies,
    DefinitionReferences,
    IncomingUses,
    RepairContext,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextObservation {
    pub target: NodeId,
    pub role: ContextObservationRole,
    pub outcome: QueryOutcome,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextOmissions {
    pub requested_maximum_nodes: u32,
    pub node_scope_truncated: bool,
    pub discovered_frontier_omitted_nodes: u64,
    pub blockers_truncated: bool,
    pub semantic_diff_truncated: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacketPayload {
    pub workspace: WorkspaceId,
    pub revision: Revision,
    pub schema_digest: MachineSchemaDigest,
    pub purpose: ContextPurpose,
    pub targets: Vec<NodeId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_revision: Option<Revision>,
    pub summary: WorkspaceSummary,
    pub aliases: Vec<ContextAlias>,
    pub nodes: Vec<crate::query::NodeView>,
    pub blockers: Page<CompletenessBlocker>,
    pub observations: Vec<ContextObservation>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub semantic_diff: Option<SemanticDiffPage>,
    pub transaction_operations: Vec<String>,
    pub expression_forms: Vec<String>,
    pub omissions: ContextOmissions,
}

#[derive(Clone, Debug, Eq, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ContextPacket {
    pub version: u16,
    pub digest: ContextPacketDigest,
    pub payload: ContextPacketPayload,
}

pub fn build_context_packet(
    endpoint: &Path,
    request: &ContextBuildRequest,
) -> Result<ContextPacket> {
    request.validate()?;
    let schema_digest = active_machine_schema_digest().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot derive active machine schema digest: {error}"),
        )
    })?;
    let mut remote = Remote::new(endpoint);
    let mut primary_queries = vec![
        Query::WorkspaceSummary,
        Query::Blockers {
            page: first_page(CONTEXT_PAGE_ITEMS),
        },
    ];
    if let Some(from) = request.from_revision {
        primary_queries.push(Query::SemanticDiff {
            from,
            page: first_page(crate::query::MAX_PAGE_ITEMS),
        });
    }
    let mut primary = remote.query(request.workspace, request.revision, primary_queries)?;
    let summary = take_success(&mut primary, "workspace_summary", |result| match result {
        QueryResult::WorkspaceSummary(summary) => Some(summary),
        _ => None,
    })?;
    let blockers = take_success(&mut primary, "blockers", |result| match result {
        QueryResult::Blockers(page) => Some(page),
        _ => None,
    })?;
    let semantic_diff = if request.from_revision.is_some() {
        Some(take_success(
            &mut primary,
            "semantic_diff",
            |result| match result {
                QueryResult::SemanticDiff(page) => Some(page),
                _ => None,
            },
        )?)
    } else {
        None
    };

    let roots = if request.targets.is_empty() {
        vec![summary.root]
    } else {
        request.targets.clone()
    };
    let (nodes, omitted_frontier) = collect_nodes(&mut remote, request, &roots)?;
    let aliases = nodes
        .iter()
        .enumerate()
        .map(|(index, view)| ContextAlias {
            alias: format!("n{}", index + 1),
            node: view.summary.node,
            kind: view.summary.kind,
            display_name: view.summary.display_name.clone(),
        })
        .collect();
    let observations = collect_observations(&mut remote, request, &nodes)?;
    let blockers_truncated = blockers.next.is_some();
    let semantic_diff_truncated = semantic_diff
        .as_ref()
        .is_some_and(|diff| diff.page.next.is_some());
    let payload = ContextPacketPayload {
        workspace: request.workspace,
        revision: request.revision,
        schema_digest,
        purpose: request.purpose,
        targets: request.targets.clone(),
        from_revision: request.from_revision,
        summary,
        aliases,
        nodes,
        blockers,
        observations,
        semantic_diff,
        transaction_operations: TransactionOpCode::ALL
            .into_iter()
            .map(|code| code.machine_name().to_owned())
            .collect(),
        expression_forms: ExpressionDraftCode::ALL
            .into_iter()
            .map(|code| code.machine_name().to_owned())
            .collect(),
        omissions: ContextOmissions {
            requested_maximum_nodes: request.maximum_nodes,
            node_scope_truncated: !omitted_frontier.is_empty(),
            discovered_frontier_omitted_nodes: u64::try_from(omitted_frontier.len())
                .unwrap_or(u64::MAX),
            blockers_truncated,
            semantic_diff_truncated,
        },
    };
    let digest = digest_payload(&payload)?;
    let packet = ContextPacket {
        version: WORKBENCH_VERSION,
        digest,
        payload,
    };
    let bytes = serde_json::to_vec(&packet).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode context packet: {error}"),
        )
    })?;
    if bytes.len() > MAX_CONTEXT_PACKET_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "context packet exceeds the output byte policy",
        ));
    }
    Ok(packet)
}

pub fn encode_context_packet(packet: &ContextPacket, pretty: bool) -> Result<Vec<u8>> {
    validate_packet(packet)?;
    let bytes = if pretty {
        serde_json::to_vec_pretty(packet)
    } else {
        serde_json::to_vec(packet)
    }
    .map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode context packet: {error}"),
        )
    })?;
    if bytes.len() > MAX_CONTEXT_PACKET_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "context packet exceeds the output byte policy",
        ));
    }
    Ok(bytes)
}

pub fn decode_context_packet(bytes: &[u8]) -> Result<ContextPacket> {
    if bytes.len() > MAX_CONTEXT_PACKET_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "context packet exceeds the input byte policy",
        ));
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let packet = ContextPacket::deserialize(&mut deserializer).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("context packet JSON is invalid: {error}"),
        )
    })?;
    deserializer.end().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("context packet has trailing input: {error}"),
        )
    })?;
    validate_packet(&packet)?;
    Ok(packet)
}

fn validate_packet(packet: &ContextPacket) -> Result<()> {
    if packet.version != WORKBENCH_VERSION {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "context packet version is unsupported",
        ));
    }
    let active = active_machine_schema_digest().map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot derive active machine schema digest: {error}"),
        )
    })?;
    if packet.payload.schema_digest != active {
        return Err(LkError::new(
            ErrorCode::ProtocolVersion,
            "context packet machine schema is stale",
        ));
    }
    if packet.digest != digest_payload(&packet.payload)? {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "context packet digest does not match its payload",
        ));
    }
    if packet.payload.summary.workspace != packet.payload.workspace
        || packet.payload.summary.revision != packet.payload.revision
    {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "context packet summary domain does not match its payload",
        ));
    }
    let mut nodes = BTreeMap::new();
    for view in &packet.payload.nodes {
        if view.summary.workspace != packet.payload.workspace
            || view.summary.revision != packet.payload.revision
            || view.summary.node.workspace() != packet.payload.workspace
        {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "context packet node belongs to another domain",
            ));
        }
        if nodes.insert(view.summary.node, view.summary.kind).is_some() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "context packet repeats a node",
            ));
        }
    }
    let mut aliases = BTreeSet::new();
    for (index, alias) in packet.payload.aliases.iter().enumerate() {
        if alias.alias != format!("n{}", index + 1) || !aliases.insert(alias.alias.as_str()) {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "context packet aliases are not canonical and unique",
            ));
        }
        if nodes.get(&alias.node) != Some(&alias.kind) {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "context packet alias does not match its node fact",
            ));
        }
    }
    if packet.payload.aliases.len() != packet.payload.nodes.len() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            "context packet must assign exactly one alias to every included node",
        ));
    }
    Ok(())
}

fn digest_payload(payload: &ContextPacketPayload) -> Result<ContextPacketDigest> {
    let encoded = serde_json::to_vec(payload).map_err(|error| {
        LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("cannot encode context packet facts: {error}"),
        )
    })?;
    let mut hasher = blake3::Hasher::new();
    hasher.update(CONTEXT_PACKET_DIGEST_DOMAIN);
    hasher.update(&encoded);
    Ok(ContextPacketDigest::from_bytes(
        *hasher.finalize().as_bytes(),
    ))
}

struct Remote {
    client: Client,
    next_request_id: u64,
}

impl Remote {
    fn new(endpoint: &Path) -> Self {
        Self {
            client: Client::new(endpoint),
            next_request_id: 1,
        }
    }

    fn request(&mut self, request: Request) -> Result<Response> {
        let id = RequestId::new(self.next_request_id);
        self.next_request_id = self
            .next_request_id
            .checked_add(1)
            .ok_or_else(|| LkError::new(ErrorCode::PolicyExceeded, "request ID overflow"))?;
        self.client.request(id, &request)
    }

    fn query(
        &mut self,
        workspace: WorkspaceId,
        revision: Revision,
        queries: Vec<Query>,
    ) -> Result<Vec<QueryOutcome>> {
        let items = queries
            .into_iter()
            .enumerate()
            .map(|(index, query)| QueryItem {
                id: QueryId::new(u64::try_from(index + 1).unwrap_or(u64::MAX)),
                query,
            })
            .collect();
        match self.request(Request::QueryBatch(QueryBatchRequest {
            workspace,
            revision,
            queries: items,
        }))? {
            Response::QueryBatchResult(result) => Ok(result
                .results
                .into_iter()
                .map(|item| item.outcome)
                .collect()),
            Response::Error(error) => Err(error),
            _ => Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "daemon returned the wrong response family for a context query",
            )),
        }
    }
}

fn first_page(limit: u32) -> PageRequest {
    PageRequest { after: None, limit }
}

fn take_success<T>(
    outcomes: &mut Vec<QueryOutcome>,
    name: &str,
    extract: impl FnOnce(QueryResult) -> Option<T>,
) -> Result<T> {
    if outcomes.is_empty() {
        return Err(LkError::new(
            ErrorCode::ProtocolMalformed,
            format!("context query omitted {name} result"),
        ));
    }
    match outcomes.remove(0) {
        QueryOutcome::Success(result) => extract(*result).ok_or_else(|| {
            LkError::new(
                ErrorCode::ProtocolMalformed,
                format!("context query returned the wrong {name} result family"),
            )
        }),
        QueryOutcome::Error(error) => Err(error),
    }
}

fn collect_nodes(
    remote: &mut Remote,
    request: &ContextBuildRequest,
    roots: &[NodeId],
) -> Result<(Vec<crate::query::NodeView>, BTreeSet<NodeId>)> {
    let maximum = usize::try_from(request.maximum_nodes).unwrap_or(usize::MAX);
    let mut queue = VecDeque::new();
    let mut scheduled = BTreeMap::new();
    let mut processed = BTreeMap::new();
    for root in roots {
        schedule_node(*root, ExpansionScope::Target, &mut scheduled, &mut queue);
    }
    let mut views = BTreeMap::new();
    while !queue.is_empty() && views.len() < maximum {
        let remaining = maximum - views.len();
        let mut pending = Vec::new();
        let mut new_nodes = BTreeSet::new();
        while pending.len() < crate::query::MAX_BATCH_QUERIES {
            let Some((node, scope)) = queue.pop_front() else {
                break;
            };
            if processed.get(&node).is_some_and(|prior| *prior >= scope) {
                continue;
            }
            if !views.contains_key(&node) && new_nodes.insert(node) && new_nodes.len() > remaining {
                queue.push_front((node, scope));
                break;
            }
            pending.push((node, scope));
        }
        if pending.is_empty() {
            continue;
        }
        let queries = pending
            .iter()
            .map(|(node, _)| Query::Node {
                node: *node,
                expand: true,
            })
            .collect();
        let outcomes = remote.query(request.workspace, request.revision, queries)?;
        if outcomes.len() != pending.len() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "context node query result count mismatch",
            ));
        }
        for ((id, scope), outcome) in pending.into_iter().zip(outcomes) {
            let view = match outcome {
                QueryOutcome::Success(result) => match *result {
                    QueryResult::Node(view) => view,
                    _ => {
                        return Err(LkError::new(
                            ErrorCode::ProtocolMalformed,
                            "context node query returned the wrong result family",
                        ));
                    }
                },
                QueryOutcome::Error(error) => return Err(error),
            };
            let record = view.record.as_ref().ok_or_else(|| {
                LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "expanded context node omitted its record",
                )
            })?;
            if view.summary.node != id {
                return Err(LkError::new(
                    ErrorCode::ProtocolMalformed,
                    "context node query returned another identity",
                ));
            }
            processed.insert(id, scope);
            enqueue_related(request.purpose, scope, record, &mut scheduled, &mut queue);
            views.insert(id, view);
        }
    }
    let omitted = scheduled
        .into_iter()
        .filter_map(|(node, scheduled_scope)| {
            processed
                .get(&node)
                .is_none_or(|processed_scope| *processed_scope < scheduled_scope)
                .then_some(node)
        })
        .collect();
    Ok((views.into_values().collect(), omitted))
}

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
enum ExpansionScope {
    Owner,
    Dependency,
    Target,
}

fn schedule_node(
    node: NodeId,
    scope: ExpansionScope,
    scheduled: &mut BTreeMap<NodeId, ExpansionScope>,
    queue: &mut VecDeque<(NodeId, ExpansionScope)>,
) {
    if scheduled.get(&node).is_none_or(|prior| *prior < scope) {
        scheduled.insert(node, scope);
        queue.push_back((node, scope));
    }
}

fn enqueue_related(
    purpose: ContextPurpose,
    scope: ExpansionScope,
    node: &Node,
    scheduled: &mut BTreeMap<NodeId, ExpansionScope>,
    queue: &mut VecDeque<(NodeId, ExpansionScope)>,
) {
    if scope != ExpansionScope::Owner {
        for index in 0..node.owned_child_count() {
            let signature_only = scope == ExpansionScope::Dependency
                && !matches!(node, Node::ProductType { .. } | Node::SumType { .. })
                && !matches!(node, Node::Function { parameters, .. } if index < parameters.len());
            let skip_body = matches!(purpose, ContextPurpose::Orient | ContextPurpose::Create)
                && matches!(node, Node::Function { parameters, .. } if index >= parameters.len());
            if signature_only || skip_body {
                continue;
            }
            if let Some(child) = node.owned_child(index) {
                schedule_node(child, scope, scheduled, queue);
            }
        }
    }
    if scope != ExpansionScope::Owner
        && !matches!(purpose, ContextPurpose::Orient | ContextPurpose::Create)
    {
        for index in 0..node.direct_reference_count() {
            if let Some(reference) = node.direct_reference(index) {
                schedule_node(
                    reference.target(),
                    ExpansionScope::Dependency,
                    scheduled,
                    queue,
                );
            }
        }
    }
    if let Some(owner) = node.owner()
        && !matches!(purpose, ContextPurpose::Orient | ContextPurpose::Create)
    {
        schedule_node(owner, ExpansionScope::Owner, scheduled, queue);
    }
}

fn collect_observations(
    remote: &mut Remote,
    request: &ContextBuildRequest,
    nodes: &[crate::query::NodeView],
) -> Result<Vec<ContextObservation>> {
    let mut specifications = Vec::new();
    for target in &request.targets {
        specifications.push((
            *target,
            ContextObservationRole::OwnerChain,
            Query::OwnerChain {
                node: *target,
                page: first_page(CONTEXT_PAGE_ITEMS),
            },
        ));
        specifications.push((
            *target,
            ContextObservationRole::Dependencies,
            Query::Dependencies {
                node: *target,
                page: first_page(CONTEXT_PAGE_ITEMS),
            },
        ));
        if matches!(
            request.purpose,
            ContextPurpose::Refactor
                | ContextPurpose::Debug
                | ContextPurpose::Extend
                | ContextPurpose::Delete
                | ContextPurpose::Review
        ) {
            specifications.push((
                *target,
                ContextObservationRole::DefinitionReferences,
                Query::DefinitionReferences {
                    target: *target,
                    page: first_page(CONTEXT_PAGE_ITEMS),
                },
            ));
        }
        if nodes
            .iter()
            .any(|view| view.summary.node == *target && view.summary.kind == NodeKind::Operation)
        {
            specifications.push((
                *target,
                ContextObservationRole::IncomingUses,
                Query::IncomingUses {
                    value: ValueRef::OperationResult {
                        operation: *target,
                        output: 0,
                    },
                    page: first_page(CONTEXT_PAGE_ITEMS),
                },
            ));
        }
        if request.purpose == ContextPurpose::Repair {
            specifications.push((
                *target,
                ContextObservationRole::RepairContext,
                Query::RepairContext {
                    target: RepairTarget::Hole(*target),
                    budget: ContextBudget {
                        body_before: 16,
                        body_after: 16,
                        visible_values: 32,
                        incoming_uses: 32,
                        include_incompatible: true,
                    },
                },
            ));
        }
    }
    let mut observations = Vec::new();
    for chunk in specifications.chunks(crate::query::MAX_BATCH_QUERIES) {
        let outcomes = remote.query(
            request.workspace,
            request.revision,
            chunk.iter().map(|(_, _, query)| query.clone()).collect(),
        )?;
        if outcomes.len() != chunk.len() {
            return Err(LkError::new(
                ErrorCode::ProtocolMalformed,
                "context observation result count mismatch",
            ));
        }
        observations.extend(
            chunk
                .iter()
                .zip(outcomes)
                .map(|((target, role, _), outcome)| ContextObservation {
                    target: *target,
                    role: *role,
                    outcome,
                }),
        );
    }
    Ok(observations)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packet_digest_spelling_is_canonical() {
        let digest = ContextPacketDigest::from_bytes([0xab; 32]);
        let spelling = digest.to_string();
        assert_eq!(spelling.len(), 64);
        assert_eq!(spelling.parse::<ContextPacketDigest>(), Ok(digest));
        assert!(
            spelling
                .to_uppercase()
                .parse::<ContextPacketDigest>()
                .is_err()
        );
    }

    #[test]
    fn context_purposes_have_unique_stable_names() {
        for (index, purpose) in ContextPurpose::ALL.into_iter().enumerate() {
            assert!(
                ContextPurpose::ALL[..index]
                    .iter()
                    .all(|prior| prior.machine_name() != purpose.machine_name())
            );
            assert_eq!(purpose.machine_name().parse(), Ok(purpose));
        }
    }
}
