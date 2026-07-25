use std::collections::BTreeSet;

use crate::graph_edges::{add_project_edges, edge, node};
use crate::model::{Audit, Graph, Policy};

pub fn build(audit: &Audit, policy: &Policy) -> Graph {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    node(
        &mut nodes,
        &format!("revision:{}", audit.revision),
        "revision",
        &audit.revision,
    );
    for dir in &audit.directories {
        node(
            &mut nodes,
            &format!("dir:{}", dir.path),
            "directory",
            &dir.path,
        );
    }
    for file in &audit.files {
        let kind = kind(&file.path);
        node(&mut nodes, &format!("file:{}", file.path), kind, &file.path);
        edge(
            &mut edges,
            &format!("dir:{}", parent(&file.path)),
            &format!("file:{}", file.path),
            "contains",
            "git ls-files",
        );
    }
    for capsule in &audit.capsules {
        node(
            &mut nodes,
            &format!("capsule:{}", capsule.id),
            "capsule",
            &capsule.id,
        );
        edge(
            &mut edges,
            &format!("capsule:{}", capsule.id),
            &format!("dir:{}", capsule.root),
            "contains",
            &format!("{}/lkjscript.capsule", capsule.root),
        );
        for dependency in &capsule.dependencies {
            edge(
                &mut edges,
                &format!("capsule:{}", capsule.id),
                &format!("capsule:{dependency}"),
                "depends-on",
                &format!("{}/lkjscript.capsule", capsule.root),
            );
        }
    }
    for command in [
        "check-docs",
        "check-tree",
        "check-sources",
        "structure-audit",
        "structure-check",
    ] {
        node(
            &mut nodes,
            &format!("command:{command}"),
            "command",
            command,
        );
    }
    add_project_edges(audit, &mut nodes, &mut edges);
    nodes.sort_by(|left, right| left.id.cmp(&right.id));
    nodes.dedup_by(|left, right| left.id == right.id);
    edges.sort_by(|left, right| {
        (&left.from, &left.to, &left.kind, &left.evidence).cmp(&(
            &right.from,
            &right.to,
            &right.kind,
            &right.evidence,
        ))
    });
    edges.dedup_by(|left, right| {
        left.from == right.from
            && left.to == right.to
            && left.kind == right.kind
            && left.evidence == right.evidence
    });
    let node_limit = usize::try_from(policy.limits.graph_nodes).unwrap_or(usize::MAX);
    let edge_limit = usize::try_from(policy.limits.graph_edges).unwrap_or(usize::MAX);
    let truncated = nodes.len() > node_limit || edges.len() > edge_limit;
    nodes.truncate(node_limit);
    let retained: BTreeSet<_> = nodes.iter().map(|item| item.id.as_str()).collect();
    edges.retain(|item| {
        retained.contains(item.from.as_str()) && retained.contains(item.to.as_str())
    });
    edges.truncate(edge_limit);
    for item in &mut nodes {
        item.revision_id = format!("{}@{}", item.id, audit.revision);
    }
    Graph {
        schema: "lkjscript.structure.graph.v1".into(),
        revision: audit.revision.clone(),
        nodes,
        edges,
        unsupported: vec![
            "dynamic dispatch call graph".into(),
            "macro-expanded Rust imports".into(),
            "runtime-loaded source dependencies".into(),
        ],
        truncated,
    }
}

fn kind(path: &str) -> &'static str {
    if path.ends_with(".md") {
        if path.starts_with("docs/decisions/") {
            "decision"
        } else {
            "document"
        }
    } else if path.contains("/tests/") || path.ends_with("_test.rs") {
        "test"
    } else if path.ends_with(".lkjscript") {
        "source-declaration"
    } else {
        "file"
    }
}
fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| ".".into(), |(parent, _)| parent.into())
}

pub fn dot(graph: &Graph) -> String {
    let mut text = String::from("digraph lkjscript_structure {\n");
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
