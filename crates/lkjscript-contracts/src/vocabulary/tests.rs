use std::collections::BTreeSet;

use super::*;
use crate::CapabilityKind;

#[test]
fn canonical_identifiers_follow_exact_ascii_grammar() {
    for accepted in ["a", "i64", "editor-state", "sha256", "a-0"] {
        assert!(is_identifier(accepted), "{accepted}");
    }
    for rejected in [
        "", "I64", "Point", "foo_bar", "foo--bar", "foo-", "-foo", "foo?", "é", "9x",
    ] {
        assert!(!is_identifier(rejected), "{rejected}");
    }
}

#[test]
fn operations_are_total_unique_and_deterministic() {
    let mut stable = BTreeSet::new();
    let mut source = BTreeSet::new();
    for index in 0..OPERATION_COUNT {
        let identity = OperationIdentity::new(index as u16);
        let record = operation_by_id(identity);
        assert!(record.is_some(), "missing operation {index}");
        let Some(record) = record else {
            continue;
        };
        assert_eq!(record.identity, identity);
        assert_eq!(operation_semantics_by_id(identity), Some(record.semantics));
        assert_eq!(record.semantics.identity, identity);
        assert!(stable.insert(record.stable_name), "{}", record.stable_name);
        assert!(source.insert(record.source_name), "{}", record.source_name);
        assert!(is_identifier(record.stable_name));
        assert!(is_identifier(record.source_name));
        assert_eq!(operation_by_source_name(record.source_name), Some(record));
        assert!(!record.summary.is_empty());
    }
    let out_of_range = OperationIdentity::new(OPERATION_COUNT as u16);
    assert!(operation_by_id(out_of_range).is_none());
    assert!(operation_semantics_by_id(out_of_range).is_none());
}

#[test]
fn every_registered_name_is_canonical() {
    for name in SIMPLE_TYPE_NAMES
        .iter()
        .chain(TYPE_CONSTRUCTOR_NAMES)
        .chain(BUILTIN_ERROR_NAMES)
        .chain(COMPILER_TRAIT_NAMES)
        .chain(PRELUDE_TYPE_NAMES)
        .chain(PRELUDE_VARIANT_NAMES)
        .chain(CONTEXTUAL_FORM_NAMES)
        .chain(RESERVED_WORDS)
    {
        assert!(is_identifier(name), "{name}");
    }
    for kind in CapabilityKind::ALL {
        assert!(is_identifier(kind.as_str()));
        assert_eq!(CapabilityKind::parse(kind.as_str()), Some(kind));
    }
}

#[test]
fn removed_spellings_are_diagnostics_not_aliases() {
    for (old, replacement) in [
        ("+", "add"),
        ("div", "divide"),
        ("->", "structured sig inputs/output"),
        ("I64", "i64"),
        ("FileSystem", "file-system"),
    ] {
        let record = removed_spelling(old);
        assert!(record.is_some(), "missing removed spelling {old}");
        let Some(record) = record else {
            continue;
        };
        assert_eq!(record.replacement, replacement);
        assert!(operation_by_source_name(old).is_none());
    }
}
