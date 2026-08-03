use super::*;

fn descriptor(items: Vec<ContractItem>) -> ContractDescriptor {
    ContractDescriptor {
        name: ContractName::registered("lkjscript.test"),
        items,
        dependencies: Vec::new(),
    }
}

fn digest(value: &ContractDescriptor) -> ContractDigest {
    let result = ContractDigest::of(value);
    assert!(result.is_ok(), "test descriptor must be valid: {result:?}");
    result.unwrap_or(ContractDigest::from_bytes([0; 32]))
}

fn fact(id: &str, name: &str, value: &str) -> ContractFact {
    ContractFact::required(id, name, value)
}

#[test]
fn canonical_encoding_is_independent_of_item_and_stable_fact_order() {
    let first = ContractItem::new("a", ContractItemKind::Type)
        .fact(fact("field-b", "b", "bool"))
        .fact(fact("field-a", "a", "i64"));
    let second =
        ContractItem::new("b", ContractItemKind::Operation).fact(fact("result", "result", "unit"));
    let left = descriptor(vec![first.clone(), second.clone()]);
    let right = descriptor(vec![
        second,
        ContractItem {
            facts: first.facts.into_iter().rev().collect(),
            ..first
        },
    ]);
    assert_eq!(digest(&left), digest(&right));
    assert_eq!(canonical_bytes(&left), canonical_bytes(&right));
}

#[test]
fn semantic_order_changes_identity() {
    let left = descriptor(vec![ContractItem::new(
        "slots",
        ContractItemKind::Operation,
    )
    .semantic_order()
    .fact(fact("first", "first", "i64"))
    .fact(fact("second", "second", "bool"))]);
    let right = descriptor(vec![ContractItem::new(
        "slots",
        ContractItemKind::Operation,
    )
    .semantic_order()
    .fact(fact("second", "second", "bool"))
    .fact(fact("first", "first", "i64"))]);
    assert_ne!(digest(&left), digest(&right));
}

#[test]
fn length_framing_prevents_ambiguous_concatenation() {
    let left = descriptor(vec![
        ContractItem::new("item", ContractItemKind::Field).fact(fact("field", "field", "ab:c"))
    ]);
    let right = descriptor(vec![
        ContractItem::new("item", ContractItemKind::Field).fact(fact("field", "field", "a:bc"))
    ]);
    assert_ne!(canonical_bytes(&left), canonical_bytes(&right));
    assert_ne!(digest(&left), digest(&right));
}

#[test]
fn every_contract_fact_changes_identity() {
    let base = descriptor(vec![
        ContractItem::new("item", ContractItemKind::Field).fact(fact("field", "field", "i64"))
    ]);
    let changed = descriptor(vec![
        ContractItem::new("item", ContractItemKind::Field).fact(fact("field", "field", "bool"))
    ]);
    assert_ne!(digest(&base), digest(&changed));
}

#[test]
fn names_are_metadata_only_when_explicitly_declared() {
    let metadata_left = descriptor(vec![ContractItem::new("item", ContractItemKind::Field)
        .fact(fact("field", "old-name", "i64").presentation_name())]);
    let metadata_right = descriptor(vec![ContractItem::new("item", ContractItemKind::Field)
        .fact(fact("field", "new-name", "i64").presentation_name())]);
    let included =
        descriptor(vec![ContractItem::new("item", ContractItemKind::Field)
            .fact(fact("field", "new-name", "i64"))]);
    assert_eq!(digest(&metadata_left), digest(&metadata_right));
    assert_ne!(digest(&metadata_left), digest(&included));
}

#[test]
fn full_digest_is_required_even_when_display_prefix_matches() {
    let expected = ContractDigest::from_bytes([7; 32]);
    let mut actual = [7; 32];
    actual[31] = 8;
    let mismatch = require_exact(
        ContractName::registered("lkjscript.test"),
        expected,
        ContractDigest::from_bytes(actual),
        "producer",
        "consumer",
    );
    assert!(mismatch.is_err());
}

#[test]
fn current_registry_is_closed_deterministic_and_dependency_checked() {
    let first_result = current_contracts();
    assert!(
        first_result.is_ok(),
        "current registry must be valid: {first_result:?}"
    );
    let first = first_result.unwrap_or_default();
    let second_result = current_contracts();
    assert!(
        second_result.is_ok(),
        "current registry must repeat: {second_result:?}"
    );
    let second = second_result.unwrap_or_default();
    assert_eq!(first, second);
    assert_eq!(first.len(), 26);
    assert!(first.get(LANGUAGE).is_some());
    assert!(first.get(SEMANTIC_RESOURCE_PLANE).is_some());
    assert!(first.get(MEMORY_OBLIGATIONS).is_some());
    assert!(first.get(STRUCTURAL_OWNERSHIP_DOMAINS).is_some());
    assert!(first.get(CAPABILITY_STATUS).is_some());
    assert!(first.get(RUNTIME_CONTROL).is_some());
    assert!(first.get(PREPARED_PROGRAM).is_some());
}

#[test]
fn compiled_source_digests_match_descriptors() {
    let result = current_contracts();
    assert!(result.is_ok());
    let contracts = result.unwrap_or_default();
    for (name, digest) in [
        (AGENT_PROTOCOL, AGENT_PROTOCOL_DIGEST),
        (AGENT_WORK_STATE, AGENT_WORK_STATE_DIGEST),
        (CAPSULE_MANIFEST, CAPSULE_MANIFEST_DIGEST),
        (DIAGNOSTICS, DIAGNOSTICS_DIGEST),
        (LANGUAGE, LANGUAGE_DIGEST),
        (MEMORY_OBLIGATIONS, MEMORY_OBLIGATIONS_DIGEST),
        (METRICS, METRICS_DIGEST),
        (NATIVE_LAYOUT, NATIVE_LAYOUT_DIGEST),
        (REPOSITORY_GRAPH, REPOSITORY_GRAPH_DIGEST),
        (RESOURCE_CATEGORIES, RESOURCE_CATEGORIES_DIGEST),
        (RESOURCE_PROFILES, RESOURCE_PROFILES_DIGEST),
        (RUNTIME_CALLS, RUNTIME_CALLS_DIGEST),
        (SEMANTIC_RESOURCE_PLANE, SEMANTIC_RESOURCE_PLANE_DIGEST),
        (SEMANTIC_SOURCE, SEMANTIC_SOURCE_DIGEST),
        (SOURCE, SOURCE_DIGEST),
        (
            STRUCTURAL_OWNERSHIP_DOMAINS,
            STRUCTURAL_OWNERSHIP_DOMAINS_DIGEST,
        ),
        (VERIFIED_SSA, VERIFIED_SSA_DIGEST),
    ] {
        assert_eq!(
            contracts.get(name).map(RegisteredContract::digest),
            Some(digest)
        );
    }
}

#[test]
fn digest_text_is_full_lowercase_sha256() {
    let item = ContractItem::new("item", ContractItemKind::Type);
    let value = digest(&descriptor(vec![item]));
    let text = value.to_hex();
    assert_eq!(text.len(), 64);
    assert_eq!(ContractDigest::from_hex(&text), Some(value));
    assert_eq!(ContractDigest::from_hex(&text.to_uppercase()), None);
}
