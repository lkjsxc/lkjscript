use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::model::{Graph, Policy, QueryResult};

pub fn run(
    command: &str,
    target: &str,
    profile: Option<&str>,
    graph: &Graph,
    policy: &Policy,
) -> QueryResult {
    let start = resolve(target, graph);
    let mut selected = BTreeSet::new();
    let mut edge_indexes = BTreeSet::new();
    let mut queue: VecDeque<_> = start.iter().cloned().collect();
    let mut work = 0u64;
    let mut bytes = 0u64;
    let mut truncated = false;
    let node_map: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    while let Some(id) = queue.pop_front() {
        if !selected.insert(id.clone()) {
            continue;
        }
        work = match work.checked_add(1) {
            Some(value) => value,
            None => {
                truncated = true;
                break;
            }
        };
        let add = node_map
            .get(id.as_str())
            .map_or(0, |node| node.id.len() + node.label.len());
        bytes = match bytes.checked_add(u64::try_from(add).unwrap_or(u64::MAX)) {
            Some(value) => value,
            None => {
                truncated = true;
                break;
            }
        };
        if work > policy.limits.query_work || bytes > policy.limits.query_bytes {
            selected.remove(&id);
            truncated = true;
            break;
        }
        for (index, edge) in graph.edges.iter().enumerate() {
            let Some(next_work) = work.checked_add(1) else {
                truncated = true;
                break;
            };
            if next_work > policy.limits.query_work {
                truncated = true;
                break;
            }
            work = next_work;
            let next = match command {
                "impact" if edge.to == id => Some(edge.from.as_str()),
                "tests" if edge.from == id && edge.kind == "tests" => Some(edge.to.as_str()),
                "tests" if edge.to == id && edge.kind == "tests" => Some(edge.from.as_str()),
                "context" if edge.from == id => Some(edge.to.as_str()),
                "context" if edge.to == id => Some(edge.from.as_str()),
                _ => None,
            };
            if let Some(next) = next {
                let edge_size =
                    edge.from.len() + edge.to.len() + edge.kind.len() + edge.evidence.len();
                let Some(next_bytes) =
                    bytes.checked_add(u64::try_from(edge_size).unwrap_or(u64::MAX))
                else {
                    truncated = true;
                    break;
                };
                if next_bytes > policy.limits.query_bytes {
                    truncated = true;
                    break;
                }
                bytes = next_bytes;
                edge_indexes.insert(index);
                if !selected.contains(next) {
                    queue.push_back(next.into());
                }
            }
        }
        if truncated {
            break;
        }
    }
    let nodes = graph
        .nodes
        .iter()
        .filter(|node| selected.contains(&node.id))
        .cloned()
        .collect();
    let edges = graph
        .edges
        .iter()
        .enumerate()
        .filter(|(index, edge)| {
            edge_indexes.contains(index)
                && selected.contains(&edge.from)
                && selected.contains(&edge.to)
        })
        .map(|(_, edge)| edge.clone())
        .collect();
    let mut unsupported = graph.unsupported.clone();
    if start.is_empty() {
        unsupported.push("target did not resolve to an evidence-backed node".into());
    }
    QueryResult {
        schema: "lkjscript.structure.query.v1".into(),
        command: command.into(),
        target: target.into(),
        profile: profile.map(str::to_owned),
        nodes,
        edges,
        work_used: work,
        bytes_used: bytes,
        truncated,
        unsupported,
    }
}

fn resolve(target: &str, graph: &Graph) -> Vec<String> {
    let candidates = [
        target.to_owned(),
        format!("file:{target}"),
        format!("dir:{target}"),
        format!("capsule:{target}"),
        format!("crate:{target}"),
        format!("command:{target}"),
    ];
    let mut result: Vec<_> = graph
        .nodes
        .iter()
        .filter(|node| candidates.contains(&node.id) || node.label == target)
        .map(|node| node.id.clone())
        .collect();
    result.sort();
    result.dedup();
    result
}

#[cfg(test)]
mod tests {
    use crate::model::{Edge, Graph, Limits, Node, Policy};
    #[test]
    fn truncation_is_deterministic() {
        let graph = Graph {
            schema: "x".into(),
            revision: "r".into(),
            nodes: vec![node("a"), node("b")],
            edges: vec![Edge {
                from: "a".into(),
                to: "b".into(),
                kind: "contains".into(),
                evidence: "e".into(),
            }],
            unsupported: vec![],
            truncated: false,
        };
        let policy = Policy {
            schema: "x".into(),
            version: "1".into(),
            limits: Limits {
                authored_lines: 200,
                authored_bytes: 32768,
                ordinary_line_scalars: 120,
                directory_entries: 16,
                warning_depth: 8,
                error_depth: 12,
                graph_nodes: 10,
                graph_edges: 10,
                query_work: 1,
                query_bytes: 100,
            },
            rules: vec![],
        };
        let first = super::run("context", "a", None, &graph, &policy);
        let second = super::run("context", "a", None, &graph, &policy);
        assert!(first.truncated);
        assert_eq!(first.nodes.len(), second.nodes.len());
    }
    fn node(id: &str) -> Node {
        Node {
            id: id.into(),
            revision_id: format!("{id}@r"),
            kind: "file".into(),
            label: id.into(),
        }
    }
}
