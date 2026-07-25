use crate::model::{Audit, Edge, Node, Policy};

use super::{declared_node, edge};
use crate::structure::graph::Budget;

pub fn add(
    audit: &Audit,
    policy: &Policy,
    nodes: &mut Vec<Node>,
    edges: &mut Vec<Edge>,
    budget: &mut Budget,
) {
    commands(policy, nodes, edges, budget);
    capsules(audit, nodes, edges, budget);
}

fn commands(policy: &Policy, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>, budget: &mut Budget) {
    let commands = [
        (
            "check-docs",
            "cargo run --locked -p lkjscript-xtask -- check-docs",
        ),
        (
            "check-tree",
            "cargo run --locked -p lkjscript-xtask -- check-tree",
        ),
        (
            "check-sources",
            "cargo run --locked -p lkjscript-xtask -- check-sources",
        ),
        (
            "structure-audit",
            "cargo run --locked -p lkjscript-xtask -- structure audit",
        ),
        (
            "structure-check",
            "cargo run --locked -p lkjscript-xtask -- structure check",
        ),
        (
            "structure-graph",
            "cargo run --locked -p lkjscript-xtask -- structure graph",
        ),
        (
            "structure-context",
            "cargo run --locked -p lkjscript-xtask -- structure context",
        ),
        (
            "structure-impact",
            "cargo run --locked -p lkjscript-xtask -- structure impact",
        ),
        (
            "structure-tests",
            "cargo run --locked -p lkjscript-xtask -- structure tests",
        ),
        ("cargo-test-workspace", "cargo test --workspace --locked"),
    ];
    for (name, command) in commands {
        if !budget.charge(1, command.len() as u64) {
            return;
        }
        declared_node(
            nodes,
            &format!("command:{name}"),
            "command",
            command,
            "Cargo.toml",
        );
    }
    for rule in &policy.rules {
        let id = format!("rule:{}", rule.id);
        declared_node(nodes, &id, "rule", &rule.id, "meta/structure/policy.json");
        edge(
            edges,
            "command:structure-check",
            &id,
            "covers",
            "meta/structure/policy.json",
            "declared",
        );
    }
    declared_node(
        nodes,
        "artifact:repository-graph",
        "artifact",
        "target/lkjscript/structure/graph.json",
        "crates/lkjscript-xtask/src/structure/commands.rs",
    );
    edge(
        edges,
        "command:structure-graph",
        "artifact:repository-graph",
        "produces-artifact",
        "structure graph command output contract",
        "declared",
    );
}

fn capsules(audit: &Audit, nodes: &mut Vec<Node>, edges: &mut Vec<Edge>, budget: &mut Budget) {
    for capsule in &audit.capsules {
        let manifest = manifest(&capsule.root);
        let id = format!("capsule:{}", capsule.id);
        super::boundaries::add(capsule, &id, &manifest, nodes, edges);
        for path in &capsule.facade {
            edge(
                edges,
                &id,
                &format!("file:{path}"),
                "exports",
                &manifest,
                "declared",
            );
        }
        for path in &capsule.tests {
            edge(
                edges,
                &format!("file:{path}"),
                &id,
                "tests",
                &manifest,
                "declared",
            );
        }
        for path in &capsule.decisions {
            edge(
                edges,
                &format!("file:{path}"),
                &id,
                "documents",
                &manifest,
                "declared",
            );
        }
        for name in &capsule.capability.names {
            if !budget.charge(1, name.len() as u64) {
                return;
            }
            let target = format!("capability:{}", stable(name));
            declared_node(nodes, &target, "capability", name, &manifest);
            edge(
                edges,
                &id,
                &target,
                "uses-capability",
                &manifest,
                "declared",
            );
        }
        for name in &capsule.unsafe_boundary.boundaries {
            let target = format!("unsafe-boundary:{}", stable(name));
            declared_node(nodes, &target, "unsafe-boundary", name, &manifest);
            edge(
                edges,
                &id,
                &target,
                "crosses-unsafe-boundary",
                &manifest,
                "declared",
            );
        }
        for command in &capsule.verification {
            let target = format!(
                "command:manifest-{}",
                &crate::sha256::digest(command.as_bytes())[..16]
            );
            declared_node(nodes, &target, "command", command, &manifest);
            edge(edges, &target, &id, "validated-by", &manifest, "declared");
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
fn stable(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() {
                value
            } else {
                '-'
            }
        })
        .collect()
}
