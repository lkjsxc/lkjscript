use super::*;
use crate::platform::kernel::{
    DeclarationPayload, DocumentContent, Name, NamespaceClass, OwnerKey, OwnerKind, OwnerRecord,
    RelationEndpoint, RelationKind,
};
use crate::platform::persistent_map::{MapWork, PersistentMap};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};
use std::collections::{BTreeMap, BTreeSet};

#[test]
fn witness_contract_domains_are_closed_and_unique() {
    assert_eq!(
        OwnerKind::ALL
            .into_iter()
            .map(OwnerKind::tag)
            .collect::<BTreeSet<_>>()
            .len(),
        OwnerKind::ALL.len()
    );
    assert_eq!(
        NamespaceClass::ALL
            .into_iter()
            .map(NamespaceClass::tag)
            .collect::<BTreeSet<_>>()
            .len(),
        NamespaceClass::ALL.len()
    );
    assert_eq!(
        RelationKind::ALL
            .into_iter()
            .map(RelationKind::tag)
            .collect::<BTreeSet<_>>()
            .len(),
        RelationKind::ALL.len()
    );
    assert_ne!(contract::validator_contract_digest().bytes(), [0_u8; 32]);
    assert_eq!(
        contract::validator_contract_digest().to_string(),
        "validator_contract_5b693e91393843a7538098d2e9c09e8c7d463fdd1116383921220e4a341f5cf3"
    );
}

#[test]
fn full_witness_rebuilds_the_normalized_kernel_fixture() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let witness = rebuild_full_witness(&snapshot).expect("full witness must rebuild");
    assert_eq!(witness.report.owners_summarized, 43);
    assert_eq!(witness.report.summary_objects, 43);
    assert_eq!(witness.report.namespace_entries, 20);
    assert_eq!(witness.report.ownership_entries, 43);
    assert_eq!(witness.report.relation_edges, 61);
    assert_eq!(witness.report.test_dependency_entries, 1);
    assert_eq!(witness.manifest.roots.owner_summaries.entries(), 43);
    assert_eq!(witness.manifest.roots.ownership.entries(), 43);
    assert_eq!(witness.summaries.len(), 43);
    assert!(!witness.entries.test_dependencies.is_empty());

    let rebuilt = rebuild_full_witness(&snapshot).expect("second full witness must rebuild");
    assert_eq!(witness.manifest_digest, rebuilt.manifest_digest);
    assert_eq!(witness.manifest, rebuilt.manifest);
    assert_eq!(witness.summary_objects, rebuilt.summary_objects);
    assert_eq!(witness.pages.object_count(), rebuilt.pages.object_count());

    for root in witness_roots(&witness) {
        let mut work = MapWork::default();
        let report = PersistentMap::from_root(root)
            .verify(&witness.pages, &mut work)
            .expect("every derived map must verify independently");
        assert_eq!(report.entries, root.entries());
    }

    let decoded = decode_witness_manifest(&witness.manifest_bytes, witness.manifest_digest)
        .expect("witness manifest must round-trip");
    assert_eq!(decoded, witness.manifest);
    assert_eq!(
        ObjectKey::for_bytes(ObjectDomain::ValidationWitness, &witness.manifest_bytes)
            .digest
            .bytes(),
        witness.manifest_digest.bytes()
    );
    for (digest, bytes) in &witness.summary_objects {
        let summary = decode_owner_summary(bytes, *digest).expect("summary must round-trip");
        assert_eq!(witness.summaries.get(&summary.owner), Some(&summary));
        assert_eq!(
            ObjectKey::for_bytes(ObjectDomain::OwnerSummary, bytes)
                .digest
                .bytes(),
            digest.bytes()
        );
    }
}

#[test]
fn rename_changes_namespace_and_presentation_without_rewriting_dependents() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&snapshot, "callee");
    let caller = declaration_named(&snapshot, "caller");
    let before = rebuild_full_witness(&snapshot).expect("base witness");

    let OwnerRecord::Declaration(record) = snapshot
        .owners
        .get_mut(&callee)
        .expect("callee declaration")
    else {
        panic!("callee must be a declaration");
    };
    record.name = Name::new("renamed_callee").expect("valid name");
    let after = rebuild_full_witness(&snapshot).expect("renamed witness");

    assert_ne!(
        before.manifest.roots.owner_summaries,
        after.manifest.roots.owner_summaries
    );
    assert_ne!(
        before.manifest.roots.namespaces,
        after.manifest.roots.namespaces
    );
    assert_eq!(
        before.manifest.roots.ownership,
        after.manifest.roots.ownership
    );
    assert_eq!(
        before.manifest.roots.forward_relations,
        after.manifest.roots.forward_relations
    );
    assert_eq!(
        before.manifest.roots.reverse_relations,
        after.manifest.roots.reverse_relations
    );
    assert_eq!(before.summaries.get(&caller), after.summaries.get(&caller));
    assert_executable_dimensions_equal(
        before.summaries.get(&callee).expect("base callee summary"),
        after
            .summaries
            .get(&callee)
            .expect("renamed callee summary"),
    );
    assert_ne!(
        before.summaries[&callee].presentation,
        after.summaries[&callee].presentation
    );
    assert_ne!(
        before.summaries[&callee].record,
        after.summaries[&callee].record
    );
}

#[test]
fn move_changes_ownership_and_namespace_without_rewriting_exact_callers() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&snapshot, "callee");
    let caller = declaration_named(&snapshot, "caller");
    let destination = module_named(&snapshot, "second");
    let before = rebuild_full_witness(&snapshot).expect("base witness");

    let OwnerRecord::Declaration(record) = snapshot
        .owners
        .get_mut(&callee)
        .expect("callee declaration")
    else {
        panic!("callee must be a declaration");
    };
    let OwnerKey::Module(destination) = destination else {
        panic!("destination must be a module");
    };
    record.module = destination;
    let after = rebuild_full_witness(&snapshot).expect("moved witness");

    assert_ne!(
        before.manifest.roots.namespaces,
        after.manifest.roots.namespaces
    );
    assert_ne!(
        before.manifest.roots.ownership,
        after.manifest.roots.ownership
    );
    assert_ne!(
        before.manifest.roots.forward_relations,
        after.manifest.roots.forward_relations
    );
    assert_ne!(
        before.manifest.roots.reverse_relations,
        after.manifest.roots.reverse_relations
    );
    assert_eq!(before.summaries.get(&caller), after.summaries.get(&caller));
    assert_executable_dimensions_equal(
        before.summaries.get(&callee).expect("base callee summary"),
        after.summaries.get(&callee).expect("moved callee summary"),
    );
    assert_eq!(
        before.summaries[&callee].presentation,
        after.summaries[&callee].presentation
    );
    assert_ne!(
        before.summaries[&callee].record,
        after.summaries[&callee].record
    );
}

#[test]
fn binding_rename_does_not_change_enclosing_function_semantics() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let binding = binding_named(&snapshot, "local");
    let function = declaration_named(&snapshot, "with_binding");
    let before = rebuild_full_witness(&snapshot).expect("base witness");

    let OwnerRecord::Binding(record) = snapshot.owners.get_mut(&binding).expect("binding record")
    else {
        panic!("selected owner must be a binding");
    };
    record.name = Name::new("renamed_local").expect("valid name");
    let after = rebuild_full_witness(&snapshot).expect("renamed binding witness");

    assert_eq!(
        before.summaries.get(&function),
        after.summaries.get(&function)
    );
    assert_executable_dimensions_equal(
        before
            .summaries
            .get(&binding)
            .expect("base binding summary"),
        after
            .summaries
            .get(&binding)
            .expect("renamed binding summary"),
    );
    assert_ne!(
        before.summaries[&binding].presentation,
        after.summaries[&binding].presentation
    );
    assert_eq!(
        before.manifest.roots.namespaces,
        after.manifest.roots.namespaces
    );
    assert_eq!(
        before.manifest.roots.ownership,
        after.manifest.roots.ownership
    );
}

#[test]
fn body_edit_changes_its_declaration_but_not_callers_validation_key() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let callee = declaration_named(&snapshot, "callee");
    let caller = declaration_named(&snapshot, "caller");
    let body = match &snapshot.owners[&callee] {
        OwnerRecord::Declaration(record) => match &record.payload {
            DeclarationPayload::Function(function) => OwnerKey::Expression(function.body),
            _ => panic!("callee must be a function"),
        },
        _ => panic!("callee must be a declaration"),
    };
    let before = rebuild_full_witness(&snapshot).expect("base witness");
    let OwnerRecord::Expression(record) = snapshot.owners.get_mut(&body).expect("body expression")
    else {
        panic!("body must be an expression");
    };
    record.operation = crate::platform::kernel::ExpressionOperation::Unit;
    let after = rebuild_full_witness(&snapshot).expect("edited witness");

    assert_ne!(
        before.summaries[&body].implementation,
        after.summaries[&body].implementation
    );
    assert_ne!(
        before.summaries[&callee].implementation,
        after.summaries[&callee].implementation
    );
    assert_eq!(
        before.summaries[&callee].semantic_interface,
        after.summaries[&callee].semantic_interface
    );
    assert_eq!(before.summaries.get(&caller), after.summaries.get(&caller));
}

#[test]
fn nonsemantic_documentation_changes_only_its_own_presentation_summary() {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let caller = declaration_named(&snapshot, "caller");
    let documentation = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| {
            matches!(record, OwnerRecord::Documentation(_)).then_some(*owner)
        })
        .expect("documentation owner");
    let before = rebuild_full_witness(&snapshot).expect("base witness");
    let OwnerRecord::Documentation(record) = snapshot
        .owners
        .get_mut(&documentation)
        .expect("documentation record")
    else {
        panic!("selected owner must be documentation");
    };
    record.content = DocumentContent::Inline("changed review note".to_owned());
    let after = rebuild_full_witness(&snapshot).expect("documentation witness");

    assert_eq!(before.summaries.get(&caller), after.summaries.get(&caller));
    assert_executable_dimensions_equal(
        before
            .summaries
            .get(&documentation)
            .expect("base documentation summary"),
        after
            .summaries
            .get(&documentation)
            .expect("changed documentation summary"),
    );
    assert_ne!(
        before.summaries[&documentation].presentation,
        after.summaries[&documentation].presentation
    );
    assert_eq!(
        before.manifest.roots.forward_relations,
        after.manifest.roots.forward_relations
    );
    assert_eq!(
        before.manifest.roots.ownership,
        after.manifest.roots.ownership
    );
}

#[test]
fn test_dependencies_are_exact_bidirectional_witness_entries() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let test = declaration_named(&snapshot, "caller_test");
    let dependency = declaration_named(&snapshot, "with_binding");
    let witness = rebuild_full_witness(&snapshot).expect("full witness");
    assert!(witness.entries.test_dependencies.iter().any(|entry| {
        entry.test == test
            && entry.kind == RelationKind::FunctionCall
            && matches!(
                entry.target,
                RelationEndpoint::Owner(exact) if exact.owner == dependency
            )
    }));
    assert_eq!(
        witness.manifest.roots.test_dependencies.entries(),
        witness.entries.test_dependencies.len() as u64 * 2
    );
    let exact = witness
        .entries
        .test_dependencies
        .iter()
        .find(|entry| entry.test == test)
        .copied()
        .expect("test dependency");
    let [forward, reverse] = test_dependency_keys(exact);
    assert_eq!(
        decode_test_dependency_forward_key(&forward).expect("forward dependency key must decode"),
        exact
    );
    assert!(forward.starts_with(&test_dependency_forward_prefix(test)));
    assert_eq!(
        decode_test_dependency_forward_key(&reverse)
            .expect_err("reverse dependency key must reject as forward")
            .code,
        "witness_test_dependency_direction"
    );
    let mut trailing = forward;
    trailing.push(0);
    assert_eq!(
        decode_test_dependency_forward_key(&trailing)
            .expect_err("trailing dependency-key bytes must reject")
            .code,
        "witness_test_dependency_trailing"
    );
}

#[test]
fn codecs_reject_corrupt_certificates_and_foreign_summary_domains() {
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let witness = rebuild_full_witness(&snapshot).expect("full witness");
    let mut corrupt = witness.manifest.clone();
    corrupt.certificate = ValidationCertificateDigest::from_bytes([7_u8; 32]);
    assert_eq!(
        encode_witness_manifest(&corrupt)
            .expect_err("foreign certificate must reject")
            .code,
        "witness_certificate_mismatch"
    );

    let (&digest, bytes) = witness
        .summary_objects
        .first_key_value()
        .expect("summary object");
    let foreign = OwnerSummaryDigest::from_bytes([9_u8; 32]);
    assert_ne!(digest, foreign);
    assert_eq!(
        decode_owner_summary(bytes, foreign)
            .expect_err("foreign summary digest must reject")
            .code,
        "witness_summary_digest"
    );

    let (&owner, &summary_digest) = witness
        .entries
        .summaries
        .first_key_value()
        .expect("summary binding");
    let binding = SummaryBinding {
        kind: witness.summaries[&owner].kind,
        summary: summary_digest,
    };
    let binding_bytes = binding.encode();
    assert_eq!(
        SummaryBinding::decode(&binding_bytes, owner).expect("summary binding must decode"),
        binding
    );
    assert!(SummaryBinding::decode(&binding_bytes[..32], owner).is_err());

    let edge = witness.entries.relations[0];
    let forward = forward_relation_key(edge);
    let reverse = reverse_relation_key(edge);
    assert_eq!(
        decode_forward_relation_key(&forward).expect("forward relation key must decode"),
        edge
    );
    assert_eq!(
        decode_reverse_relation_key(&reverse).expect("reverse relation key must decode"),
        edge
    );
    let mut trailing = forward;
    trailing.push(0);
    assert_eq!(
        decode_forward_relation_key(&trailing)
            .expect_err("trailing relation-key bytes must reject")
            .code,
        "witness_relation_trailing"
    );
}

fn witness_roots(witness: &FullWitness) -> [crate::platform::persistent_map::MapRoot; 6] {
    let roots = witness.manifest.roots;
    [
        roots.owner_summaries,
        roots.namespaces,
        roots.ownership,
        roots.forward_relations,
        roots.reverse_relations,
        roots.test_dependencies,
    ]
}

fn declaration_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    named_owners(snapshot)
        .get(name)
        .copied()
        .expect("named declaration must exist")
}

fn module_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Module(record) if record.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named module must exist")
}

fn binding_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Binding(record) if record.name.as_str() == name => Some(*owner),
            _ => None,
        })
        .expect("named binding must exist")
}

fn named_owners(snapshot: &crate::platform::kernel::KernelSnapshot) -> BTreeMap<String, OwnerKey> {
    snapshot
        .owners
        .iter()
        .filter_map(|(owner, record)| match record {
            OwnerRecord::Declaration(record) => Some((record.name.as_str().to_owned(), *owner)),
            _ => None,
        })
        .collect()
}

fn assert_executable_dimensions_equal(before: &OwnerSummary, after: &OwnerSummary) {
    assert_eq!(before.kind, after.kind);
    assert_eq!(before.semantic_interface, after.semantic_interface);
    assert_eq!(before.implementation, after.implementation);
    assert_eq!(before.type_digest, after.type_digest);
    assert_eq!(before.effect, after.effect);
    assert_eq!(before.capability, after.capability);
    assert_eq!(before.relations, after.relations);
    assert_eq!(before.test, after.test);
    assert_eq!(
        before.validation_dependencies,
        after.validation_dependencies
    );
}
