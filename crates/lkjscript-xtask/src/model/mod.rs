mod capsule;
mod graph;
pub use capsule::*;
pub use graph::*;

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
    pub graph_work: u64,
    pub graph_bytes: u64,
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

#[derive(Clone, Debug, Serialize)]
pub struct Audit {
    pub schema: String,
    pub version: u32,
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
    pub max_physical_line_scalars: u64,
    pub max_ordinary_line_scalars: u64,
    pub exact_data_lines: u64,
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
