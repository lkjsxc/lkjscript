mod base;
mod error;
mod identity;
mod kinds;
mod output;

pub use error::GraphBuildError;

use std::path::Path;

use crate::model::{Audit, Graph, Policy};
use crate::public_facts::Registry;

pub(crate) struct Budget {
    pub work: u64,
    pub bytes: u64,
    work_limit: u64,
    byte_limit: u64,
    pub error: Option<GraphBuildError>,
}

impl Budget {
    fn new(policy: &Policy) -> Self {
        Self {
            work: 0,
            bytes: 0,
            work_limit: policy.limits.graph_work,
            byte_limit: policy.limits.graph_bytes,
            error: None,
        }
    }

    pub fn charge(&mut self, work: u64, bytes: u64) -> bool {
        if self.error.is_some() {
            return false;
        }
        let Some(next_work) = self.work.checked_add(work) else {
            self.error = Some(GraphBuildError::exhausted(
                "work",
                self.work_limit,
                self.work,
                u64::MAX,
            ));
            return false;
        };
        let Some(next_bytes) = self.bytes.checked_add(bytes) else {
            self.error = Some(GraphBuildError::exhausted(
                "retained-bytes",
                self.byte_limit,
                self.bytes,
                u64::MAX,
            ));
            return false;
        };
        if next_work > self.work_limit {
            self.error = Some(GraphBuildError::exhausted(
                "work",
                self.work_limit,
                self.work,
                next_work,
            ));
            return false;
        }
        if next_bytes > self.byte_limit {
            self.error = Some(GraphBuildError::exhausted(
                "retained-bytes",
                self.byte_limit,
                self.bytes,
                next_bytes,
            ));
            return false;
        }
        self.work = next_work;
        self.bytes = next_bytes;
        true
    }

    pub fn reject(&mut self, dimension: &str) {
        self.reject_subject(dimension, "");
    }

    pub fn reject_subject(&mut self, dimension: &str, subject: &str) {
        if self.error.is_none() {
            let mut error = GraphBuildError::exhausted(dimension, 0, 0, 1);
            error.subject = subject.into();
            self.error = Some(error);
        }
    }
}

#[cfg(test)]
pub fn build(root: &Path, audit: &Audit, policy: &Policy) -> Result<Graph, GraphBuildError> {
    let registry = crate::public_facts::load(root).ok();
    build_internal(root, audit, policy, registry.as_ref())
}

pub fn build_with_facts(
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: &Registry,
) -> Result<Graph, GraphBuildError> {
    build_internal(root, audit, policy, Some(registry))
}

fn build_internal(
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: Option<&Registry>,
) -> Result<Graph, GraphBuildError> {
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
    if let Some(error) = budget.error.take() {
        return Err(error);
    }
    output::canonicalize(&mut nodes, &mut edges, policy)?;
    let Some(retained_bytes) = output::retained_field_bytes(&nodes, &edges) else {
        return Err(GraphBuildError::exhausted(
            "retained-byte-arithmetic",
            policy.limits.graph_bytes,
            0,
            u64::MAX,
        ));
    };
    if !budget.charge(0, retained_bytes) {
        return Err(match budget.error.take() {
            Some(error) => error,
            None => GraphBuildError::exhausted(
                "retained-bytes",
                policy.limits.graph_bytes,
                0,
                retained_bytes,
            ),
        });
    }
    let input_identity =
        identity::graph_identity(&audit.revision, &nodes, &edges, budget.work, budget.bytes);
    for item in &mut nodes {
        item.revision_id = format!("{}@{input_identity}", item.id);
    }
    Ok(Graph {
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
    })
}

pub use output::dot;
