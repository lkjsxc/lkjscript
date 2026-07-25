use serde::Serialize;

use super::{FileRecord, Finding, Rule};

#[derive(Clone, Debug, Serialize)]
pub struct Graph {
    pub schema: String,
    pub version: u32,
    pub revision: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub work_used: u64,
    pub bytes_used: u64,
    pub unsupported: Vec<String>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
pub struct QueryResult {
    pub schema: String,
    pub version: u32,
    pub command: String,
    pub target: String,
    pub profile: Option<String>,
    pub sections: Vec<ContextSection>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub work_used: u64,
    pub bytes_used: u64,
    pub truncated: bool,
    pub unsupported: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExplainResult {
    pub schema: String,
    pub query: String,
    pub rules: Vec<Rule>,
    pub files: Vec<FileRecord>,
    pub findings: Vec<Finding>,
    pub unsupported: Vec<String>,
}
