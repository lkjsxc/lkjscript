mod completion;
mod support;

use std::fs;

use support::{fixture, policy, root};

#[test]
fn graph_is_deterministic_and_stable_ids_ignore_unrelated_revision_edits(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = root();
    let audit = fixture(&root, &"a".repeat(40));
    let first = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000))?;
    let second = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000))?;
    assert_eq!(
        serde_json::to_string(&first).ok(),
        serde_json::to_string(&second).ok()
    );
    let changed = fixture(&root, &"b".repeat(40));
    let third = super::graph::build(&root, &changed, &policy(1000, 4000, 100_000, 1_000_000))?;
    let id = "cargo-package:x";
    assert_ne!(first.input_identity, third.input_identity);
    assert!(first.nodes.iter().any(|node| node.id == id));
    assert!(third.nodes.iter().any(|node| node.id == id));
    assert_ne!(
        first
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| &node.revision_id),
        third
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| &node.revision_id)
    );
    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn exact_import_markdown_cargo_capsule_and_test_edges_have_evidence(
) -> Result<(), Box<dyn std::error::Error>> {
    let root = root();
    support::public_facts(&root);
    let audit = fixture(&root, &"c".repeat(40));
    let graph = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000))?;
    let expected = [
        ("imports", "source-unit:src/part.lkjscript"),
        ("documents", "file:src/main.lkjscript"),
        ("depends-on", "cargo-package:lkjscript-core"),
        ("owns", "file:crates/x/src/lib.rs"),
        ("tests", "capsule:x"),
    ];
    for (kind, target) in expected {
        assert!(
            graph
                .edges
                .iter()
                .any(|edge| edge.kind == kind && edge.to == target && !edge.evidence.is_empty()),
            "missing {kind} -> {target}"
        );
    }
    assert!(graph.nodes.iter().any(|node| {
        node.kind == "lkjscript-declaration"
            && node.confidence == "compiler-derived"
            && node.span.is_some()
    }));
    assert!(graph.nodes.iter().any(|node| node.id == "fact:test-fact"));
    assert!(graph.edges.iter().any(|edge| {
        edge.from == "file:docs/decision.md"
            && edge.to == "fact:test-fact"
            && edge.kind == "projects"
    }));
    let limits = policy(1000, 4000, 100_000, 1_000_000);
    let context = super::query::run("context", "fact:test-fact", Some("strong"), &graph, &limits);
    assert!(context
        .sections
        .iter()
        .any(|section| { section.name == "exclusions" && !section.node_ids.is_empty() }));
    let impact = super::query::run("impact", "fact:test-fact", None, &graph, &limits);
    assert!(impact
        .nodes
        .iter()
        .any(|node| node.id == "file:docs/decision.md"));
    assert!(!impact.nodes.iter().any(|node| node.id == "capsule:x"));
    let tests = super::query::run("tests", "fact:test-fact", None, &graph, &limits);
    assert!(tests
        .nodes
        .iter()
        .any(|node| node.id == "file:docs/decision.md"));
    assert!(tests.edges.iter().any(|edge| edge.kind == "tests"));
    fs::remove_dir_all(root)?;
    Ok(())
}
