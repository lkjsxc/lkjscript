use crate::semantic::schema::{SemanticDeclarationKind, SemanticNodeKind, SemanticNodeValue};
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
fn generic_enum_nodes_identities_and_subtree_roundtrip_are_closed_v2() {
    let source = concat!(
        "edition/\n2\n/edition\n",
        "enum/\nname/\nMaybe\n/name\nforall/\nT\n/forall\nvariants/\n",
        "variant/\nname/\nNone\n/name\nfields/\n/fields\n/variant\n",
        "variant/\nname/\nNext\n/name\nfields/\nvariant-field/\n",
        "name/\nvalue\n/name\ntype/\nMaybe/\nT\n/Maybe\n/type\n",
        "/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n",
    );
    let tree = validate(source, "src/main.lkjscript", &Limits::default()).expect("enum tree");
    let declaration = tree
        .declarations()
        .iter()
        .find(|item| item.kind() == crate::source::DeclarationKind::Enum)
        .expect("enum declaration");
    let projected = crate::semantic::tree::declaration_record(&tree, declaration);
    assert_eq!(projected.kind, SemanticDeclarationKind::Enum);
    assert_eq!(projected.key, declaration.key().to_hex());

    let records = crate::semantic::tree::node_records(&tree);
    let enum_node = records
        .iter()
        .find(|node| node.kind == SemanticNodeKind::EnumDeclaration)
        .expect("enum node");
    assert_eq!(
        enum_node.semantic_identity.as_deref(),
        Some(projected.key.as_str())
    );
    let variants = records
        .iter()
        .filter(|node| node.kind == SemanticNodeKind::EnumVariant)
        .collect::<Vec<_>>();
    assert_eq!(variants.len(), 2);
    assert!(variants.iter().all(|node| node.semantic_identity.is_some()));
    assert_ne!(variants[0].semantic_identity, variants[1].semantic_identity);
    let field = records
        .iter()
        .find(|node| node.kind == SemanticNodeKind::EnumVariantField)
        .expect("variant field");
    assert!(field.semantic_identity.is_some());
    assert!(records
        .iter()
        .any(|node| node.kind == SemanticNodeKind::ContextVariants));
    assert!(records
        .iter()
        .any(|node| node.kind == SemanticNodeKind::TypeEnum));

    let subtree =
        crate::semantic::tree::subtree_record(&tree, enum_node.index).expect("enum subtree");
    let rebuilt = subtree.to_source().expect("closed enum subtree");
    let enum_source = crate::source::format_node_source(&rebuilt);
    let complete =
        format!("edition/\n2\n/edition\n{enum_source}main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n");
    validate(&complete, "src/main.lkjscript", &Limits::default())
        .expect("roundtripped enum validates");
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
