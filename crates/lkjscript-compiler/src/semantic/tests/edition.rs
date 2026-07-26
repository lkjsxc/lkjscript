use crate::semantic::schema::{SemanticNodeKind, SemanticNodeValue};
use crate::source::{validate, SourceEdition};
use lkjscript_core::Limits;

#[test]
fn schema_v2_projects_exact_edition_identity_and_marker() {
    let source = concat!(
        ";; leading\n",
        "edition/\n2\n/edition\n",
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n"
    );
    let tree = validate(source, "src/main.lkjscript", &Limits::default()).expect("Edition 2");
    assert_eq!(tree.edition(), SourceEdition::Edition2);
    let units = crate::semantic::tree::source_units(&tree);
    assert_eq!(units.len(), 1);
    assert_eq!(units[0].edition, 2);
    assert_eq!(
        units[0].identity,
        tree.source_identity("src/main.lkjscript")
            .expect("source identity")
            .to_hex()
    );

    let records = crate::semantic::tree::node_records(&tree);
    assert_eq!(records[0].kind, SemanticNodeKind::EditionMarker);
    assert!(matches!(
        records[0].value,
        Some(SemanticNodeValue::EditionIdentity { edition: 2 })
    ));
    assert_eq!(records[1].kind, SemanticNodeKind::EditionNumber);
    assert!(matches!(
        records[1].value,
        Some(SemanticNodeValue::EditionIdentity { edition: 2 })
    ));
    assert_eq!(records[0].children, vec![1]);

    let subtree = crate::semantic::tree::subtree_record(&tree, 0).expect("marker subtree");
    let rebuilt = subtree.to_source().expect("closed marker projection");
    let mut malformed = subtree.clone();
    malformed.children.clear();
    malformed.node.children.clear();
    assert!(malformed.to_source().is_err());
    assert_eq!(
        crate::source::format_node_source(&rebuilt),
        ";; leading\nedition/\n2\n/edition\n"
    );
}

#[test]
fn schema_identity_stays_version_two_for_both_source_editions() {
    assert_eq!(crate::semantic::SCHEMA, "lkjscript.semantic-source");
    assert_eq!(crate::semantic::VERSION, 2);
    for source in [
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
        "edition/\n2\n/edition\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
    ] {
        validate(source, "src/main.lkjscript", &Limits::default()).expect("schema source");
    }
}
