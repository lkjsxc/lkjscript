use serde::Serialize;

use super::{FileRecord, Finding, Rule};

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Graph {
    pub schema: String,
    pub contract: String,
    pub revision: String,
    pub input_identity: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub work_used: u64,
    pub bytes_used: u64,
    pub unsupported: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Node {
    pub id: String,
    pub revision_id: String,
    pub kind: String,
    pub label: String,
    pub provenance: String,
    pub authority: String,
    pub span: Option<String>,
    pub confidence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub evidence: String,
    pub confidence: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct ContextSection {
    pub name: String,
    pub node_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct QueryCompletion {
    pub status: String,
    pub stop_reasons: Vec<String>,
    pub ordering: String,
    pub continuation_supported: bool,
    pub omitted_frontier: Vec<String>,
    pub work_limit: u64,
    pub retained_byte_limit: u64,
    pub output_byte_limit: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct QueryResult {
    pub schema: String,
    pub contract: String,
    pub graph_identity: String,
    pub command: String,
    pub target: String,
    pub profile: Option<String>,
    pub sections: Vec<ContextSection>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub work_used: u64,
    pub bytes_used: u64,
    pub completion: QueryCompletion,
    pub unsupported: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExplainResult {
    pub schema: String,
    pub contract: String,
    pub graph_identity: String,
    pub query: String,
    pub rules: Vec<Rule>,
    pub files: Vec<FileRecord>,
    pub facts: Vec<FactExplanation>,
    pub findings: Vec<Finding>,
    pub unsupported: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FactExplanation {
    pub id: String,
    pub status: String,
    pub digest: String,
    pub interface: String,
    pub exclusions: Vec<String>,
    pub authority: String,
    pub evidence: Vec<String>,
    pub projections: Vec<String>,
}
