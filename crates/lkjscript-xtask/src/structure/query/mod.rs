mod context;
mod explain;
mod traversal;

use crate::model::{Audit, ExplainResult, Graph, Policy, QueryCompletion, QueryResult};
use crate::public_facts::Registry;

pub fn explain(
    audit: &Audit,
    policy: &Policy,
    registry: &Registry,
    graph_identity: &str,
    query: &str,
) -> ExplainResult {
    explain::run(audit, policy, registry, graph_identity, query)
}

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
    let output_byte_limit = if weak {
        policy.limits.query_bytes.min(32_768)
    } else {
        policy.limits.query_bytes
    };
    let byte_limit = (output_byte_limit / 8).max(1_024);
    let depth_limit = match command {
        "context" => 1,
        "tests" => 2,
        _ => 3,
    };
    let traversal_command = if command == "impact" && start.iter().any(|id| id.starts_with("fact:"))
    {
        "fact-impact"
    } else {
        command
    };
    let mut selected = traversal::select(
        traversal_command,
        &start,
        graph,
        work_limit,
        byte_limit,
        depth_limit,
    );
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
        selected.stop_reasons.insert("target-unresolved".into());
        selected.omitted_frontier.insert(target.into());
    }
    let complete = selected.stop_reasons.is_empty();
    let completion = QueryCompletion {
        status: if complete { "complete" } else { "bounded" }.into(),
        stop_reasons: selected.stop_reasons.into_iter().collect(),
        ordering: "canonical-node-id-then-canonical-edge-order".into(),
        continuation_supported: false,
        omitted_frontier: selected.omitted_frontier.into_iter().collect(),
        work_limit,
        retained_byte_limit: byte_limit,
        output_byte_limit,
    };
    QueryResult {
        schema: "lkjscript.repository-query".into(),
        contract: lkjscript_contracts::REPOSITORY_GRAPH_DIGEST.to_hex(),
        graph_identity: graph.input_identity.clone(),
        command: command.into(),
        target: target.into(),
        profile: profile.map(str::to_owned),
        sections,
        nodes,
        edges,
        work_used: selected.work,
        bytes_used: selected.bytes,
        completion,
        unsupported,
    }
}
