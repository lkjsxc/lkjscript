mod base;
mod kinds;
mod output;

use std::path::Path;

use crate::model::{Audit, Graph, Policy};
use crate::public_facts::Registry;

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

#[cfg(test)]
pub fn build(root: &Path, audit: &Audit, policy: &Policy) -> Graph {
    let registry = crate::public_facts::load(root).ok();
    build_internal(root, audit, policy, registry.as_ref())
}

pub fn build_with_facts(root: &Path, audit: &Audit, policy: &Policy, registry: &Registry) -> Graph {
    build_internal(root, audit, policy, Some(registry))
}

fn build_internal(
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: Option<&Registry>,
) -> Graph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    let mut budget = Budget::new(policy);
    base::identity(&audit.revision, &mut nodes, &mut edges);
    base::directories(audit, &mut nodes, &mut edges, &mut budget);
    base::files(audit, &mut nodes, &mut edges, &mut budget);
    base::capsules(audit, &mut nodes, &mut edges, &mut budget);
    super::graph_edges::add_project_edges(
        root,
        audit,
        policy,
        registry,
        &mut nodes,
        &mut edges,
        &mut budget,
    );
    super::source_facts::add(root, audit, &mut nodes, &mut edges, &mut budget);
    output::canonicalize(&mut nodes, &mut edges, policy, &mut budget);
    let input_identity = graph_identity(
        &audit.revision,
        &nodes,
        &edges,
        budget.work,
        budget.bytes,
        budget.truncated,
    );
    for item in &mut nodes {
        item.revision_id = format!("{}@{input_identity}", item.id);
    }
    Graph {
        schema: "lkjscript.repository-graph".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        revision: audit.revision.clone(),
        input_identity,
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

fn graph_identity(
    revision: &str,
    nodes: &[crate::model::Node],
    edges: &[crate::model::Edge],
    work: u64,
    charged_bytes: u64,
    truncated: bool,
) -> String {
    let mut bytes = Vec::new();
    append_identity(&mut bytes, revision);
    append_identity(
        &mut bytes,
        &lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
    );
    bytes.extend_from_slice(&work.to_be_bytes());
    bytes.extend_from_slice(&charged_bytes.to_be_bytes());
    bytes.push(u8::from(truncated));
    for item in nodes {
        for value in [
            &item.id,
            &item.kind,
            &item.label,
            &item.provenance,
            &item.authority,
            item.span.as_deref().unwrap_or(""),
            &item.confidence,
        ] {
            append_identity(&mut bytes, value);
        }
    }
    for item in edges {
        for value in [
            &item.from,
            &item.to,
            &item.kind,
            &item.evidence,
            &item.confidence,
        ] {
            append_identity(&mut bytes, value);
        }
    }
    crate::sha256::digest(&bytes)
}

fn append_identity(bytes: &mut Vec<u8>, value: &str) {
    bytes.extend_from_slice(&(value.len() as u128).to_be_bytes());
    bytes.extend_from_slice(value.as_bytes());
}

pub use output::dot;
