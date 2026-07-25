mod context;
mod traversal;

use crate::model::{Graph, Policy, QueryResult};

pub fn run(
    command: &str,
    target: &str,
    profile: Option<&str>,
    graph: &Graph,
    policy: &Policy,
) -> QueryResult {
    let start = context::resolve(target, graph);
    let weak = profile == Some("weak");
    let work_limit = if weak {
        policy.limits.query_work.min(16_384)
    } else {
        policy.limits.query_work
    };
    let byte_limit = if weak {
        policy.limits.query_bytes.min(32_768)
    } else {
        policy.limits.query_bytes
    };
    let depth_limit = match command {
        "context" => 1,
        "tests" => 2,
        _ => 3,
    };
    let mut selected =
        traversal::select(command, &start, graph, work_limit, byte_limit, depth_limit);
    if command == "context" {
        for node in graph
            .nodes
            .iter()
            .filter(|node| matches!(node.kind.as_str(), "repository-revision" | "rule"))
        {
            selected.include_required(node);
        }
    }
    let nodes: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| selected.nodes.contains(&node.id))
        .cloned()
        .collect();
    let edges: Vec<_> = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(index, edge)| {
            selected.edges.contains(index)
                && selected.nodes.contains(&edge.from)
                && selected.nodes.contains(&edge.to)
        })
        .map(|(_, edge)| edge.clone())
        .collect();
    let sections = if command == "context" {
        context::sections(&start, &nodes, &edges)
    } else {
        Vec::new()
    };
    let mut unsupported = graph.unsupported.clone();
    if start.is_empty() {
        unsupported.push("target did not resolve to an evidence-backed node".into());
    }
    if selected.truncated {
        unsupported.push(
            "query traversal was explicitly truncated by a work, byte, or depth limit".into(),
        );
    }
    QueryResult {
        schema: "lkjscript.repository-query".into(),
        version: 1,
        command: command.into(),
        target: target.into(),
        profile: profile.map(str::to_owned),
        sections,
        nodes,
        edges,
        work_used: selected.work,
        bytes_used: selected.bytes,
        truncated: selected.truncated,
        unsupported,
    }
}
