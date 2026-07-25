use crate::model::{Audit, Edge, Node};

use super::super::graph_edges::{edge, node};
use super::Budget;

pub fn identity(revision: &str, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>) {
    node(
        nodes,
        "repository:workspace",
        "repository",
        "lkjscript workspace",
        "authored",
        "capsule.json",
        None,
        "declared",
    );
    node(
        nodes,
        "repository-revision:HEAD",
        "repository-revision",
        revision,
        "generated",
        "git rev-parse HEAD",
        None,
        "declared",
    );
    edge(
        edges,
        "repository:workspace",
        "repository-revision:HEAD",
        "contains",
        "git rev-parse HEAD",
        "declared",
    );
}

pub fn directories(
    audit: &Audit,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    for dir in &audit.directories {
        if !budget.charge(1, dir.path.len() as u64) {
            break;
        }
        node(
            nodes,
            &format!("directory:{}", dir.path),
            "directory",
            &dir.path,
            "authored",
            "git ls-files",
            None,
            "declared",
        );
        let owner = if dir.path == "." {
            "repository:workspace".into()
        } else {
            format!("directory:{}", parent(&dir.path))
        };
        edge(
            edges,
            &owner,
            &format!("directory:{}", dir.path),
            "contains",
            "git ls-files",
            "declared",
        );
    }
}

pub fn files(audit: &Audit, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>, budget: &mut Budget) {
    for file in &audit.files {
        if !budget.charge(1, file.path.len() as u64) {
            break;
        }
        node(
            nodes,
            &format!("file:{}", file.path),
            super::kinds::file(&file.path, &file.class),
            &file.path,
            super::kinds::provenance(&file.class),
            &file.path,
            None,
            "declared",
        );
        edge(
            edges,
            &format!("directory:{}", parent(&file.path)),
            &format!("file:{}", file.path),
            "contains",
            "git ls-files",
            "declared",
        );
    }
}

pub fn capsules(audit: &Audit, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>, budget: &mut Budget) {
    for capsule in &audit.capsules {
        if !budget.charge(1, capsule.id.len() as u64) {
            break;
        }
        let manifest = manifest(&capsule.root);
        let id = format!("capsule:{}", capsule.id);
        node(
            nodes,
            &id,
            "capsule",
            &capsule.context_card.goal,
            capsule.provenance.class.as_str(),
            &manifest,
            None,
            "declared",
        );
        edge(
            edges,
            &id,
            &format!("directory:{}", capsule.root),
            "contains",
            &manifest,
            "declared",
        );
        for dependency in &capsule.allowed_dependencies {
            edge(
                edges,
                &id,
                &format!("capsule:{dependency}"),
                "depends-on",
                &manifest,
                "declared",
            );
            edge(
                edges,
                &id,
                &format!("capsule:{dependency}"),
                "permits",
                &manifest,
                "declared",
            );
        }
        for dependency in &capsule.forbidden_dependencies {
            edge(
                edges,
                &id,
                &format!("capsule:{dependency}"),
                "forbids",
                &manifest,
                "declared",
            );
        }
        for file in audit
            .files
            .iter()
            .filter(|file| file.capsule.as_deref() == Some(&capsule.id))
        {
            edge(
                edges,
                &id,
                &format!("file:{}", file.path),
                "owns",
                &manifest,
                "declared",
            );
        }
    }
}

fn manifest(root: &str) -> String {
    if root == "." {
        "capsule.json".into()
    } else {
        format!("{root}/capsule.json")
    }
}
fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map_or_else(|| ".".into(), |(value, _)| value.into())
}
