mod boundaries;
mod cargo;
mod markdown_links;
mod metadata;
mod provenance;
mod public_fact_evidence;
mod public_facts;
mod rust;

use std::path::Path;

use crate::model::{Audit, Edge, Node, Policy};
use crate::public_facts::Registry;

use super::graph::Budget;

pub fn add_project_edges(
    root: &Path,
    audit: &Audit,
    policy: &Policy,
    registry: Option<&Registry>,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    metadata::add(audit, policy, nodes, edges, budget);
    public_facts::add(registry, nodes, edges, budget);
    provenance::add(audit, nodes, edges, budget);
    cargo::add(root, audit, nodes, edges, budget);
    markdown_links::add(root, audit, edges, budget);
    rust::add(root, audit, nodes, edges, budget);
}

pub fn read_text(root: &Path, path: &str, bytes: u64, budget: &mut Budget) -> Option<String> {
    if !budget.charge(1, bytes) {
        return None;
    }
    let input = super::repository_support::read_bounded(&root.join(path), 4 * 1024 * 1024).ok()?;
    String::from_utf8(input).ok()
}

#[allow(clippy::too_many_arguments)]
pub fn node(
    nodes: &mut Vec<Node>,
    id: &str,
    kind: &str,
    label: &str,
    provenance: &str,
    authority: &str,
    span: Option<String>,
    confidence: &str,
) {
    nodes.push(Node {
        id: id.into(),
        revision_id: String::new(),
        kind: kind.into(),
        label: label.into(),
        provenance: provenance.into(),
        authority: authority.into(),
        span,
        confidence: confidence.into(),
    });
}

pub fn declared_node(nodes: &mut Vec<Node>, id: &str, kind: &str, label: &str, authority: &str) {
    node(
        nodes, id, kind, label, "authored", authority, None, "declared",
    );
}

pub fn edge(
    edges: &mut Vec<Edge>,
    from: &str,
    to: &str,
    kind: &str,
    evidence: &str,
    confidence: &str,
) {
    edges.push(Edge {
        from: from.into(),
        to: to.into(),
        kind: kind.into(),
        evidence: evidence.into(),
        confidence: confidence.into(),
    });
}
