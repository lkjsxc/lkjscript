use std::collections::BTreeSet;

use crate::model::{ContextSection, Edge, Graph, Node};

pub fn resolve(target: &str, graph: &Graph) -> Vec<String> {
    let prefixes = [
        "file",
        "directory",
        "capsule",
        "cargo-package",
        "cargo-crate",
        "command",
        "source-unit",
        "rule",
    ];
    let mut candidates = vec![target.to_owned()];
    candidates.extend(prefixes.iter().map(|prefix| format!("{prefix}:{target}")));
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

pub fn sections(start: &[String], nodes: &[Node], edges: &[Edge]) -> Vec<ContextSection> {
    let order = [
        "goal",
        "revision/profile",
        "capsule-card",
        "interfaces",
        "rules",
        "implementations",
        "source-facts",
        "dependencies",
        "dependents",
        "tests",
        "decisions/status",
        "provenance",
        "omissions",
    ];
    let dependency_kinds = [
        "depends-on",
        "imports",
        "uses-capability",
        "consumes-artifact",
    ];
    let inbound: BTreeSet<_> = edges
        .iter()
        .filter(|edge| start.contains(&edge.to) && dependency_kinds.contains(&edge.kind.as_str()))
        .map(|edge| edge.from.as_str())
        .collect();
    let outbound: BTreeSet<_> = edges
        .iter()
        .filter(|edge| start.contains(&edge.from) && dependency_kinds.contains(&edge.kind.as_str()))
        .map(|edge| edge.to.as_str())
        .collect();
    let interfaces: BTreeSet<_> = edges
        .iter()
        .filter(|edge| start.contains(&edge.from) && edge.kind == "exports")
        .map(|edge| edge.to.as_str())
        .collect();
    let tests: BTreeSet<_> = edges
        .iter()
        .filter(|edge| edge.kind == "tests")
        .flat_map(|edge| [edge.from.as_str(), edge.to.as_str()])
        .collect();
    order
        .iter()
        .map(|name| {
            let mut ids: Vec<_> = nodes
                .iter()
                .filter(|node| section(name, node, start, &inbound, &outbound, &interfaces, &tests))
                .map(|node| node.id.clone())
                .collect();
            ids.sort();
            ContextSection {
                name: (*name).into(),
                node_ids: ids,
            }
        })
        .collect()
}

fn section(
    name: &str,
    node: &Node,
    start: &[String],
    inbound: &BTreeSet<&str>,
    outbound: &BTreeSet<&str>,
    interfaces: &BTreeSet<&str>,
    tests: &BTreeSet<&str>,
) -> bool {
    match name {
        "goal" => start.contains(&node.id),
        "revision/profile" => node.kind == "repository-revision",
        "capsule-card" => node.kind == "capsule",
        "interfaces" => interfaces.contains(node.id.as_str()),
        "rules" => node.kind == "rule",
        "implementations" => {
            matches!(node.kind.as_str(), "authored-file" | "rust-symbol")
        }
        "source-facts" => {
            matches!(node.kind.as_str(), "source-unit" | "lkjscript-declaration")
        }
        "dependencies" => outbound.contains(node.id.as_str()),
        "dependents" => inbound.contains(node.id.as_str()),
        "tests" => node.kind == "test" || tests.contains(node.id.as_str()),
        "decisions/status" => matches!(node.kind.as_str(), "decision" | "current-state"),
        "provenance" => matches!(node.provenance.as_str(), "generated" | "immutable-evidence"),
        "omissions" => false,
        _ => false,
    }
}
