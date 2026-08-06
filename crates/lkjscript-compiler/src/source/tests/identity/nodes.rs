use super::super::*;
use crate::source::NodeId;

#[test]
fn node_ids_are_dense_preorder_deterministic_and_revision_scoped() {
    let source = unit_main("do/\nunit\ntrue\n/do");
    let first = validate(&source, "src/nodes.lkjscript").expect("first");
    let again = validate(&source, "src/nodes.lkjscript").expect("again");
    assert_eq!(first.revision(), again.revision());
    for (index, node) in first.nodes().iter().enumerate() {
        assert_eq!(usize::try_from(node.id().index()).ok(), Some(index));
        assert_eq!(node.id().revision(), first.revision());
    }
    let root = first.nodes().first().expect("root");
    let high = NodeId {
        revision: first.revision(),
        index: u64::from(u32::MAX) + 1,
    };
    assert_ne!(high, root.id());
    assert!(first.node(high).expect("same revision").is_none());
    assert_eq!(root.kind(), NodeKind::Call);
    assert_eq!(root.label(), Some("main"));
    assert!(root.parent().is_none());
    assert!(!root.children().is_empty());

    let changed =
        validate(&unit_main("do/\nunit\nfalse\n/do"), "src/nodes.lkjscript").expect("changed");
    let stale = changed.node(root.id()).expect_err("cross-revision lookup");
    assert_eq!(stale.actual_revision(), first.revision());
    assert_eq!(stale.expected_revision(), changed.revision());
}

#[test]
fn public_validate_requires_canonical_relative_lkjscript_paths() {
    let source = unit_main("unit");
    for rejected in [
        "legacy.lkjml",
        "../escape.lkjscript",
        "/absolute.lkjscript",
        "./aliased.lkjscript",
        "src//aliased.lkjscript",
        ".hidden.lkjscript",
    ] {
        let error =
            validate(&source, rejected).expect_err("noncanonical logical path must be rejected");
        assert_eq!(error.code(), "LKJ-SRC-LOAD", "{rejected}");
    }
    let accepted =
        validate(&source, "src/canonical.lkjscript").expect("canonical relative logical path");
    assert_eq!(
        accepted.root_origin().logical_path(),
        "src/canonical.lkjscript"
    );
    assert!(accepted.format_source("src/canonical.lkjscript").is_some());
    assert!(accepted.format_source("../canonical.lkjscript").is_none());
    assert!(accepted.format_source("/src/canonical.lkjscript").is_none());
}
