use serde::{Deserialize, Serialize};

use super::Expression;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Charges {
    pub request_bytes: u64,
    pub source_bytes: u64,
    pub source_units: u64,
    pub source_nodes: u64,
    pub operations: u64,
    pub work_units: u64,
    pub output_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct PositionRecord {
    pub byte: u32,
    pub line: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SpanRecord {
    pub start: PositionRecord,
    pub end: PositionRecord,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SourceUnitRecord {
    pub path: String,
    pub bytes: u64,
    pub sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclarationRecord {
    pub key: String,
    pub identity: String,
    pub kind: String,
    pub name: String,
    pub source: String,
    pub span: SpanRecord,
    pub node: u32,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeRecord {
    pub index: u32,
    pub kind: String,
    pub label: Option<String>,
    pub source: String,
    pub span: SpanRecord,
    pub parent: Option<u32>,
    pub children: Vec<u32>,
    pub declaration: Option<String>,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntityRecord {
    pub declaration: DeclarationRecord,
    pub source: String,
    pub fingerprint: String,
    pub canonical_subtree: String,
    pub subtree: Expression,
    pub descendants: Vec<NodeRecord>,
    pub derived_summaries: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "availability", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum FactRecord {
    Available {
        producer: String,
        version: u32,
        certainty: String,
        value: String,
    },
    Unavailable {
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeFacts {
    pub binding: FactRecord,
    pub static_type: FactRecord,
    pub effects: FactRecord,
    pub ownership: FactRecord,
    pub control_flow: FactRecord,
    pub layout: FactRecord,
    pub proof: FactRecord,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeQueryRecord {
    pub node: NodeRecord,
    pub facts: NodeFacts,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityRelation {
    pub relation: String,
    pub old_key: Option<String>,
    pub new_key: Option<String>,
    pub old_node: Option<u32>,
    pub new_node: Option<u32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ChangedSource {
    pub path: String,
    pub old_sha256: String,
    pub new_sha256: String,
    pub bytes: u64,
}
