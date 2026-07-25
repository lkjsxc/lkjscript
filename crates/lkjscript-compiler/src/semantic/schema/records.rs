use serde::{Deserialize, Serialize};

use super::{FactSchema, NodeFacts, NodeRecord, SemanticSubtreeRecord, TriviaRecord};

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
    pub edition: u32,
    pub bytes: u64,
    pub sha256: String,
    pub trailing_trivia: Vec<TriviaRecord>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SemanticDeclarationKind {
    Main,
    Function,
    Product,
    MarkerTrait,
    TraitImplementation,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DeclarationRecord {
    pub key: String,
    pub identity: String,
    pub kind: SemanticDeclarationKind,
    pub name: String,
    pub source: String,
    pub span: SpanRecord,
    pub node: u32,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EntityRecord {
    pub declaration: DeclarationRecord,
    pub source: String,
    pub fingerprint: String,
    pub canonical_subtree: String,
    pub subtree: SemanticSubtreeRecord,
    pub descendants: Vec<NodeRecord>,
    pub available_fact_schemas: Vec<FactSchema>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct NodeQueryRecord {
    pub node: NodeRecord,
    pub facts: NodeFacts,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum IdentityRelationKind {
    RenamedDeclaration,
    ReplacedExpression,
    ChangedReferenceOwner,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct IdentityRelation {
    pub relation: IdentityRelationKind,
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
