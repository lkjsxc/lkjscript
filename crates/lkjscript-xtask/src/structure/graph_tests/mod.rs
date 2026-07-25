mod support;

use std::fs;

use support::{fixture, policy, root};

#[test]
fn graph_is_deterministic_and_stable_ids_ignore_unrelated_revision_edits() {
    let root = root();
    let audit = fixture(&root, &"a".repeat(40));
    let first = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000));
    let second = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000));
    assert_eq!(
        serde_json::to_string(&first).ok(),
        serde_json::to_string(&second).ok()
    );
    let changed = fixture(&root, &"b".repeat(40));
    let third = super::graph::build(&root, &changed, &policy(1000, 4000, 100_000, 1_000_000));
    let id = "cargo-package:lkjscript-x";
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
    assert!(fs::remove_dir_all(root).is_ok());
}

#[test]
fn exact_import_markdown_cargo_capsule_and_test_edges_have_evidence() {
    let root = root();
    let audit = fixture(&root, &"c".repeat(40));
    let graph = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000));
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
    assert!(fs::remove_dir_all(root).is_ok());
}

#[test]
fn truncation_context_order_impact_and_tests_are_explicit() {
    let root = root();
    let audit = fixture(&root, &"d".repeat(40));
    let graph = super::graph::build(&root, &audit, &policy(4, 4, 10, 100));
    assert!(graph.truncated);
    let full = super::graph::build(&root, &audit, &policy(1000, 4000, 100_000, 1_000_000));
    let limits = policy(1000, 4000, 100_000, 1_000_000);
    let context = super::query::run("context", "x", Some("strong"), &full, &limits);
    assert_eq!(
        context
            .sections
            .first()
            .map(|section| section.name.as_str()),
        Some("goal")
    );
    assert_eq!(
        context.sections.last().map(|section| section.name.as_str()),
        Some("omissions")
    );
    let impact = super::query::run("impact", "core", None, &full, &limits);
    assert!(impact.nodes.iter().any(|node| node.id == "capsule:x"));
    let tests = super::query::run("tests", "x", None, &full, &limits);
    assert!(tests.nodes.iter().any(|node| node.kind == "test"));
    assert!(fs::remove_dir_all(root).is_ok());
}
