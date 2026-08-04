use std::collections::BTreeSet;

use crate::model::{Edge, Graph, Node, Policy};

use super::GraphBuildError;

pub fn canonicalize(
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    policy: &Policy,
) -> Result<(), GraphBuildError> {
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    if let Some(pair) = nodes
        .windows(2)
        .find(|pair| pair[0].id == pair[1].id && pair[0] != pair[1])
    {
        return Err(GraphBuildError::conflicting_node(&pair[0].id));
    }
    nodes.dedup();
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
    edges.dedup();
    let node_count = u64::try_from(nodes.len()).unwrap_or(u64::MAX);
    if node_count > policy.limits.graph_nodes {
        return Err(GraphBuildError::exhausted(
            "nodes",
            policy.limits.graph_nodes,
            0,
            node_count,
        ));
    }
    let edge_count = u64::try_from(edges.len()).unwrap_or(u64::MAX);
    if edge_count > policy.limits.graph_edges {
        return Err(GraphBuildError::exhausted(
            "edges",
            policy.limits.graph_edges,
            0,
            edge_count,
        ));
    }
    let retained: BTreeSet<_> = nodes.iter().map(|item| item.id.as_str()).collect();
    if let Some(edge) = edges
        .iter()
        .find(|item| !retained.contains(item.from.as_str()) || !retained.contains(item.to.as_str()))
    {
        let mut error = GraphBuildError::exhausted("dangling-edge-endpoint", 0, 0, 1);
        error.subject = format!("{} -> {}", edge.from, edge.to);
        return Err(error);
    }
    Ok(())
}

pub fn retained_field_bytes(nodes: &[Node], edges: &[Edge]) -> Option<u64> {
    let mut bytes = 0;
    for item in nodes {
        add_bytes(&mut bytes, item.id.len())?;
        if item.revision_id.is_empty() {
            add_bytes(&mut bytes, item.id.len().checked_add(65)?)?;
        } else {
            add_bytes(&mut bytes, item.revision_id.len())?;
        }
        for value in [
            &item.kind,
            &item.label,
            &item.provenance,
            &item.authority,
            &item.confidence,
        ] {
            add_bytes(&mut bytes, value.len())?;
        }
        if let Some(span) = &item.span {
            add_bytes(&mut bytes, span.len())?;
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
            add_bytes(&mut bytes, value.len())?;
        }
    }
    Some(bytes)
}

fn add_bytes(total: &mut u64, bytes: usize) -> Option<()> {
    *total = total.checked_add(u64::try_from(bytes).ok()?)?;
    Some(())
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
