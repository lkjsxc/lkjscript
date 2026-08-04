use std::collections::BTreeSet;

use crate::model::{Edge, Graph, Node, Policy};

use super::Budget;

pub fn canonicalize(
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    policy: &Policy,
    budget: &mut Budget,
) {
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    nodes.dedup_by(|left, right| left.id == right.id);
    edges.sort_by(|left, right| {
        (
            &left.from,
            &left.to,
            &left.kind,
            &left.evidence,
            &left.confidence,
        )
            .cmp(&(
                &right.from,
                &right.to,
                &right.kind,
                &right.evidence,
                &right.confidence,
            ))
    });
    edges.dedup_by(|left, right| {
        left.from == right.from
            && left.to == right.to
            && left.kind == right.kind
            && left.evidence == right.evidence
            && left.confidence == right.confidence
    });
    let node_limit = usize::try_from(policy.limits.graph_nodes).unwrap_or(usize::MAX);
    let edge_limit = usize::try_from(policy.limits.graph_edges).unwrap_or(usize::MAX);
    budget.truncated |= nodes.len() > node_limit || edges.len() > edge_limit;
    nodes.truncate(node_limit);
    let retained: BTreeSet<_> = nodes.iter().map(|item| item.id.as_str()).collect();
    edges.retain(|item| {
        retained.contains(item.from.as_str()) && retained.contains(item.to.as_str())
    });
    if edges.len() > edge_limit {
        budget.truncated = true;
        edges.truncate(edge_limit);
    }
}

pub fn dot(graph: &Graph) -> String {
    let mut text = String::from("digraph lkjscript_repository {\n");
    for item in &graph.nodes {
        text.push_str(&format!(
            "  \"{}\" [label=\"{}\\n{}\"];\n",
            escape(&item.id),
            escape(&item.label),
            escape(&item.kind)
        ));
    }
    for item in &graph.edges {
        text.push_str(&format!(
            "  \"{}\" -> \"{}\" [label=\"{}\"];\n",
            escape(&item.from),
            escape(&item.to),
            escape(&item.kind)
        ));
    }
    text.push_str("}\n");
    text
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}
