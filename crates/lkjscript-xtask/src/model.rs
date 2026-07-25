use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Policy {
    pub schema: String,
    pub version: String,
    pub limits: Limits,
    pub rules: Vec<Rule>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub authored_lines: u64,
    pub authored_bytes: u64,
    pub ordinary_line_scalars: u64,
    pub directory_entries: u64,
    pub warning_depth: u64,
    pub error_depth: u64,
    pub graph_nodes: u64,
    pub graph_edges: u64,
    pub query_work: u64,
    pub query_bytes: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub id: String,
    pub level: String,
    pub summary: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProvenanceFile {
    pub schema: String,
    pub version: u32,
    pub entries: Vec<Provenance>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Provenance {
    pub path: String,
    pub class: String,
    pub sha256: String,
    pub generator: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RatchetFile {
    pub schema: String,
    pub version: u32,
    pub records: Vec<Ratchet>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Ratchet {
    pub rule: String,
    pub path: String,
    pub observed: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CapsuleKind {
    Workspace,
    Collection,
    Crate,
    Source,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub schema: String,
    pub version: u32,
    pub id: String,
    pub root: String,
    pub kind: CapsuleKind,
    pub dependencies: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Audit {
    pub schema: String,
    pub revision: String,
    pub policy_version: String,
    pub files: Vec<FileRecord>,
    pub directories: Vec<DirectoryRecord>,
    pub classifications: Vec<ClassCount>,
    pub capsules: Vec<Capsule>,
    pub findings: Vec<Finding>,
    pub provenance: Vec<Provenance>,
    pub unsupported: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct FileRecord {
    pub path: String,
    pub bytes: u64,
    pub lines: u64,
    pub max_line_scalars: u64,
    pub class: String,
    pub capsule: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DirectoryRecord {
    pub path: String,
    pub entries: u64,
    pub depth: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct ClassCount {
    pub class: String,
    pub files: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub severity: String,
    pub rule: String,
    pub path: String,
    pub observed: Option<u64>,
    pub limit: Option<u64>,
    pub message: String,
    pub provenance: Option<String>,
    pub sort_key: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Graph {
    pub schema: String,
    pub revision: String,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub unsupported: Vec<String>,
    pub truncated: bool,
}

#[derive(Clone, Debug, Serialize)]
pub struct Node {
    pub id: String,
    pub revision_id: String,
    pub kind: String,
    pub label: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: String,
    pub evidence: String,
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

#[derive(Clone, Debug, Serialize)]
pub struct QueryResult {
    pub schema: String,
    pub command: String,
    pub target: String,
    pub profile: Option<String>,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub work_used: u64,
    pub bytes_used: u64,
    pub truncated: bool,
    pub unsupported: Vec<String>,
}
