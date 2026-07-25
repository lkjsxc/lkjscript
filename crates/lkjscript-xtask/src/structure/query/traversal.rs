use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::model::{Edge, Graph, Node};

pub struct Selection {
    pub nodes: BTreeSet<String>,
    pub edges: BTreeSet<usize>,
    pub work: u64,
    pub bytes: u64,
    pub truncated: bool,
    work_limit: u64,
    byte_limit: u64,
}

impl Selection {
    fn new(work_limit: u64, byte_limit: u64, truncated: bool) -> Self {
        Self {
            nodes: BTreeSet::new(),
            edges: BTreeSet::new(),
            work: 0,
            bytes: 0,
            truncated,
            work_limit,
            byte_limit,
        }
    }

    fn charge(&mut self, work: u64, bytes: u64) -> bool {
        let Some(next_work) = self.work.checked_add(work) else {
            return false;
        };
        let Some(next_bytes) = self.bytes.checked_add(bytes) else {
            return false;
        };
        if next_work > self.work_limit || next_bytes > self.byte_limit {
            return false;
        }
        self.work = next_work;
        self.bytes = next_bytes;
        true
    }

    pub fn include_required(&mut self, node: &Node) {
        if self.nodes.contains(&node.id) {
            return;
        }
        let bytes = (node.id.len() + node.label.len() + node.authority.len()) as u64;
        if self.charge(1, bytes) {
            self.nodes.insert(node.id.clone());
        } else {
            self.truncated = true;
        }
    }
}

pub fn select(
    command: &str,
    start: &[String],
    graph: &Graph,
    work_limit: u64,
    byte_limit: u64,
    depth_limit: u64,
) -> Selection {
    let mut result = Selection::new(work_limit, byte_limit, graph.truncated);
    let mut incoming = BTreeMap::<&str, Vec<usize>>::new();
    let mut outgoing = BTreeMap::<&str, Vec<usize>>::new();
    for (index, edge) in graph.edges.iter().enumerate() {
        if !result.charge(1, 0) {
            result.truncated = true;
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
            result.truncated = true;
            break;
        }
        result.nodes.insert(id.clone());
        if depth >= depth_limit {
            result.truncated |=
                incoming.contains_key(id.as_str()) || outgoing.contains_key(id.as_str());
            continue;
        }
        let indexes = indexes(command, &id, &incoming, &outgoing);
        for index in indexes {
            if !result.charge(1, 0) {
                result.truncated = true;
                break;
            }
            let edge = &graph.edges[index];
            let Some(next) = next(command, &id, edge) else {
                continue;
            };
            let size =
                (edge.from.len() + edge.to.len() + edge.kind.len() + edge.evidence.len()) as u64;
            if !result.charge(0, size) {
                result.truncated = true;
                break;
            }
            result.edges.insert(index);
            if !result.nodes.contains(next) {
                queue.push_back((next.into(), depth + 1));
            }
        }
        if result.truncated {
            break;
        }
    }
    result
}

fn indexes(
    command: &str,
    id: &str,
    incoming: &BTreeMap<&str, Vec<usize>>,
    outgoing: &BTreeMap<&str, Vec<usize>>,
) -> Vec<usize> {
    let mut result = Vec::new();
    if matches!(command, "impact" | "tests" | "context") {
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
