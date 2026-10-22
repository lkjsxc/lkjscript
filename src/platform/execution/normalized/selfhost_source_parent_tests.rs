//! Admitted containment parents must equal the independent namespace projection.
use super::*;

#[test]
fn selfhost_source_namespace_reuses_exact_admitted_parents_for_all_supported_classes() {
    let source = byte_buffer_tests::author_only(concat!(
        include_str!("../../../../tests/fixtures/selfhost-source-namespace.lkjc"),
        "\n",
        include_str!("../../../../tests/fixtures/selfhost-source-effect-owners.lkjc"),
    ))
    .unwrap();
    kernel::validate_full(&source).unwrap();
    let bytes = graph::packet(&source);
    let (code, image) = result("read-namespace-parents", &bytes);
    assert_eq!(code, 0);
    let expected = source
        .owners
        .values()
        .filter_map(kernel::owner_namespace)
        .collect::<Vec<_>>();
    let (rows, remainder) = image.as_chunks::<16>();
    assert!(remainder.is_empty());
    assert_eq!(rows.len(), expected.len());
    let mut classes = std::collections::BTreeSet::new();
    for (row, entry) in rows.iter().zip(expected) {
        let word = |at| u32::from_le_bytes(row[at..at + 4].try_into().unwrap()) as usize;
        let parent = entry.parent.map_or(source.owners.len(), |parent| {
            source
                .owners
                .keys()
                .position(|owner| *owner == parent)
                .unwrap()
        });
        assert_eq!(word(0), parent, "{:?}: {}", entry.class, entry.name);
        assert_eq!(word(4), usize::from(entry.class.tag()));
        let at = word(8);
        let end = at.checked_add(word(12)).unwrap();
        assert_eq!(&bytes[at..end], entry.name.as_str().as_bytes());
        classes.insert(entry.class);
    }
    use kernel::NamespaceClass;
    assert_eq!(
        classes,
        std::collections::BTreeSet::from([
            NamespaceClass::Module,
            NamespaceClass::Declaration,
            NamespaceClass::TypeParameter,
            NamespaceClass::Field,
            NamespaceClass::Case,
            NamespaceClass::Parameter,
            NamespaceClass::Port,
            NamespaceClass::Target,
            NamespaceClass::Operation,
            NamespaceClass::Requirement,
            NamespaceClass::EffectParameter,
            NamespaceClass::RequirementParameter,
        ])
    );
    index::assert_probe("read-namespace-parents", &bytes, &image);
}
