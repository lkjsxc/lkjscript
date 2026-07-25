use std::collections::BTreeSet;
use std::path::Path;

use crate::model::{Audit, Edge, Node};

pub fn add_project_edges(audit: &Audit, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>) {
    for file in &audit.files {
        if file.path.ends_with("Cargo.toml") {
            let crate_root = file.path.strip_suffix("/Cargo.toml").unwrap_or(".");
            let crate_id = format!("crate:{crate_root}");
            node(nodes, &crate_id, "crate", crate_root);
            edge(
                edges,
                &crate_id,
                &format!("file:{}", file.path),
                "contains",
                &file.path,
            );
        }
        if file.path.ends_with(".lkjscript") {
            edge(
                edges,
                &format!("file:{}", file.path),
                "command:check-sources",
                "validated-by",
                "xtask check-sources corpus walk",
            );
        }
        if file.path.ends_with(".md") {
            document_edges(file.path.as_str(), audit, edges);
        }
        if file.path.contains("/tests/") {
            if let Some(prefix) = file.path.split("/tests/").next() {
                edge(
                    edges,
                    &format!("file:{}", file.path),
                    &format!("crate:{prefix}"),
                    "tests",
                    "Cargo integration-test location",
                );
            }
        }
        if file.path.ends_with(".rs") {
            rust_import_edges(&file.path, nodes, edges);
        }
    }
}

fn document_edges(path: &str, audit: &Audit, edges: &mut Vec<Edge>) {
    let stem = path.trim_end_matches(".md");
    for candidate in &audit.files {
        if candidate.path != path && candidate.path.starts_with(stem) {
            edge(
                edges,
                &format!("file:{path}"),
                &format!("file:{}", candidate.path),
                "documents",
                "path-prefix projection",
            );
        }
    }
}

fn rust_import_edges(path: &str, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>) {
    let Ok(bytes) = crate::repository_support::read_bounded(Path::new(path), 4 * 1024 * 1024)
    else {
        return;
    };
    let Ok(text) = std::str::from_utf8(&bytes) else {
        return;
    };
    let mut imported = BTreeSet::new();
    for line in text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("use lkjscript_"))
    {
        if let Some(name) = line.split([':', ';', ' ']).next() {
            imported.insert(name.replace('_', "-"));
        }
    }
    for name in imported {
        let target = format!("crate:crates/{name}");
        node(nodes, &target, "crate", &format!("crates/{name}"));
        edge(edges, &format!("file:{path}"), &target, "imports", path);
    }
}

pub fn node(nodes: &mut Vec<Node>, id: &str, kind: &str, label: &str) {
    nodes.push(Node {
        id: id.into(),
        revision_id: String::new(),
        kind: kind.into(),
        label: label.into(),
    });
}
pub fn edge(edges: &mut Vec<Edge>, from: &str, to: &str, kind: &str, evidence: &str) {
    edges.push(Edge {
        from: from.into(),
        to: to.into(),
        kind: kind.into(),
        evidence: evidence.into(),
    });
}
