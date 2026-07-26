use crate::semantic::schema::{SemanticDeclarationKind, SemanticNodeKind};
use crate::source::validate;
use lkjscript_core::Limits;

#[test]
fn canonical_snapshot_has_no_language_generation_fields() {
    let source = ";; leading\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    let tree =
        validate(source, "src/main.lkjscript", &Limits::default()).expect("canonical source");
    let units = crate::semantic::tree::source_units(&tree);
    assert_eq!(units.len(), 1);
    assert_eq!(
        units[0].identity,
        tree.source_identity("src/main.lkjscript")
            .expect("source identity")
            .to_hex()
    );
    let snapshot = crate::semantic::operations::snapshot::build(&tree);
    let encoded = serde_json::to_string(&snapshot).expect("snapshot JSON");
    assert!(!encoded.contains("edition"));
    assert!(crate::semantic::tree::node_records(&tree)
        .iter()
        .all(|record| record.kind != SemanticNodeKind::Import));
}

#[test]
fn generic_enum_nodes_identities_and_subtree_roundtrip_are_closed() {
    let source = concat!(
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

    let subtree =
        crate::semantic::tree::subtree_record(&tree, enum_node.index).expect("enum subtree");
    let rebuilt = subtree.to_source().expect("closed enum subtree");
    let enum_source = crate::source::format_node_source(&rebuilt);
    let complete = format!("{enum_source}main/\nsig/\n->\nUnit\n/sig\nunit\n/main\n");
    validate(&complete, "src/main.lkjscript", &Limits::default())
        .expect("roundtripped enum validates");
}

#[test]
fn semantic_source_identity_is_the_full_current_contract_digest() {
    assert_eq!(
        crate::semantic::SCHEMA,
        lkjscript_contracts::SEMANTIC_SOURCE
    );
    assert_eq!(
        crate::semantic::CONTRACT,
        lkjscript_contracts::SEMANTIC_SOURCE_DIGEST
    );
    assert_eq!(crate::semantic::CONTRACT.to_hex().len(), 64);
}
