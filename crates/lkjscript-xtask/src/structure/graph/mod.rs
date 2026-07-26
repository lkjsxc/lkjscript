mod base;
mod kinds;
mod output;

use std::path::Path;

use crate::model::{Audit, Graph, Policy};

pub(crate) struct Budget {
    pub work: u64,
    pub bytes: u64,
    work_limit: u64,
    byte_limit: u64,
    pub truncated: bool,
}

impl Budget {
    fn new(policy: &Policy) -> Self {
        Self {
            work: 0,
            bytes: 0,
            work_limit: policy.limits.graph_work,
            byte_limit: policy.limits.graph_bytes,
            truncated: false,
        }
    }

    pub fn charge(&mut self, work: u64, bytes: u64) -> bool {
        let Some(next_work) = self.work.checked_add(work) else {
            self.truncated = true;
            return false;
        };
        let Some(next_bytes) = self.bytes.checked_add(bytes) else {
            self.truncated = true;
            return false;
        };
        if next_work > self.work_limit || next_bytes > self.byte_limit {
            self.truncated = true;
            return false;
        }
        self.work = next_work;
        self.bytes = next_bytes;
        true
    }
}

pub fn build(root: &Path, audit: &Audit, policy: &Policy) -> Graph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut budget = Budget::new(policy);
    base::identity(&audit.revision, &mut nodes, &mut edges);
    base::directories(audit, &mut nodes, &mut edges, &mut budget);
    base::files(audit, &mut nodes, &mut edges, &mut budget);
    base::capsules(audit, &mut nodes, &mut edges, &mut budget);
    super::graph_edges::add_project_edges(root, audit, policy, &mut nodes, &mut edges, &mut budget);
    super::source_facts::add(root, audit, &mut nodes, &mut edges, &mut budget);
    output::canonicalize(&mut nodes, &mut edges, audit, policy, &mut budget);
    Graph {
        schema: "lkjscript.repository-graph".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        revision: audit.revision.clone(),
        nodes,
        edges,
        work_used: budget.work,
        bytes_used: budget.bytes,
        unsupported: vec![
            "dynamic calls and type-use edges are unsupported".into(),
            "macro-expanded Rust symbols, imports, implementations, and tests are unsupported"
                .into(),
            "validated lkjscript implementation edges emit only when declarations exist".into(),
            "lkjscript imports use an exact bounded structural parser and are inferred".into(),
            "compiler source facts expose declarations but not import edges".into(),
            "artifact consumption beyond exact provenance and command contracts is unsupported"
                .into(),
        ],
        truncated: budget.truncated,
    }
}

pub use output::dot;
