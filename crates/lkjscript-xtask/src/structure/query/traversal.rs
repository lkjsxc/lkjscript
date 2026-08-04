use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::model::{Edge, Graph, Node};

include!("traversal/selection.rs");

pub fn select(
    command: &str,
    start: &[String],
    graph: &Graph,
    work_limit: u64,
    byte_limit: u64,
    depth_limit: u64,
) -> Selection {
    let mut result = Selection::new(work_limit, byte_limit);
    if start.is_empty() {
        return result;
    }
    let mut incoming = BTreeMap::<&str, Vec<usize>>::new();
    let mut outgoing = BTreeMap::<&str, Vec<usize>>::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        if !result.charge(1, 0) {
            result.omitted_frontier.extend(start.iter().cloned());
            return result;
        }
        incoming.entry(&edge.to).or_default().push(index);
        outgoing.entry(&edge.from).or_default().push(index);
    }
    let nodes: BTreeMap<_, _> = graph
        .nodes
        .iter()
        .map(|node| (node.id.as_str(), node))
        .collect();
    let mut queue: VecDeque<_> = start.iter().cloned().map(|id| (id, 0_u64)).collect();
    while let Some((id, depth)) = queue.pop_front() {
        if result.nodes.contains(&id) {
            continue;
        }
        let Some(node) = nodes.get(id.as_str()) else {
            continue;
        };
        let size = (node.id.len() + node.label.len() + node.authority.len()) as u64;
        if !result.charge(1, size) {
            result.omitted_frontier.insert(id);
            result
                .omitted_frontier
                .extend(queue.iter().map(|(id, _)| id.clone()));
            break;
        }
        result.nodes.insert(id.clone());
        let indexes = indexes(command, &id, &incoming, &outgoing);
        if depth >= depth_limit {
            let mut stopped = false;
            for index in indexes {
                if let Some(next) = next(command, &id, &graph.edges[index]) {
                    result.omitted_frontier.insert(next.into());
                    stopped = true;
                }
            }
            if stopped {
                result.stop_reasons.insert("depth-budget".into());
            }
            continue;
        }
        for (position, index) in indexes.iter().copied().enumerate() {
            let edge = &graph.edges[index];
            let Some(next) = next(command, &id, edge) else {
                continue;
            };
            if !result.charge(1, 0) {
                omit_neighbors(&mut result, command, &id, &indexes[position..], graph);
                break;
            }
            let size =
                (edge.from.len() + edge.to.len() + edge.kind.len() + edge.evidence.len()) as u64;
            if !result.charge_edge(size) {
                omit_neighbors(&mut result, command, &id, &indexes[position..], graph);
                break;
            }
            result.edges.insert(index);
            if !result.nodes.contains(next) {
                queue.push_back((next.into(), depth + 1));
            }
        }
    }
    result
}

fn omit_neighbors(
    result: &mut Selection,
    command: &str,
    id: &str,
    indexes: &[usize],
    graph: &Graph,
) {
    for index in indexes {
        if let Some(next) = next(command, id, &graph.edges[*index]) {
            result.omitted_frontier.insert(next.into());
        }
    }
}

fn indexes(
    command: &str,
    id: &str,
    incoming: &BTreeMap<&str, Vec<usize>>,
    outgoing: &BTreeMap<&str, Vec<usize>>,
) -> Vec<usize> {
    let mut result = Vec::new();
    if matches!(command, "impact" | "fact-impact" | "tests" | "context") {
        result.extend(incoming.get(id).into_iter().flatten().copied());
    }
    if matches!(command, "tests" | "context") {
        result.extend(outgoing.get(id).into_iter().flatten().copied());
    }
    result.sort_unstable();
    result.dedup();
    result
}

fn next<'a>(command: &str, id: &str, edge: &'a Edge) -> Option<&'a str> {
    match command {
        "impact" if edge.to == id => Some(edge.from.as_str()),
        "fact-impact"
            if edge.to == id
                && id.starts_with("fact:")
                && matches!(
                    edge.kind.as_str(),
                    "projects" | "depends-on" | "invalidated-by"
                ) =>
        {
            Some(edge.from.as_str())
        }
        "tests" if edge.to == id && matches!(edge.kind.as_str(), "tests" | "owns") => {
            Some(edge.from.as_str())
        }
        "tests" if edge.from == id && matches!(edge.kind.as_str(), "tests" | "defines") => {
            Some(edge.to.as_str())
        }
        "context" if edge.from == id => Some(edge.to.as_str()),
        "context" if edge.to == id => Some(edge.from.as_str()),
        _ => None,
    }
}
