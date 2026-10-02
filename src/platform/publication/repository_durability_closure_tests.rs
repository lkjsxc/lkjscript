//! Publication admission and durable acknowledgement must not trust a mutable stage inventory.

use super::*;
use crate::platform::storage::object::{ImmutableObjectStore, StoreWork};

fn fixture() -> (tempfile::TempDir, CreatedRepository) {
    let temporary = tempfile::tempdir().unwrap();
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&temporary.path().join("meaning"), &logical, None).unwrap();
    (temporary, created)
}

fn assert_replay(
    outcome: PublicationOutcome,
    prepared: &PreparedPublication,
    expected_observed: HeadRecord,
) {
    let PublicationOutcome::AlreadyAccepted { accepted, observed } = outcome else {
        panic!("retry must retain the original immutable acceptance")
    };
    assert_eq!(accepted.head, prepared.head);
    assert_eq!(accepted.receipt, prepared.receipt);
    assert_eq!(observed, expected_observed);
}

fn assert_missing_from_store(created: &CreatedRepository, key: ObjectKey) {
    assert!(
        !created
            .repository
            .object_store()
            .unwrap()
            .contains(key, &mut StoreWork::default())
            .unwrap()
    );
}

fn assert_owner_name(created: &CreatedRepository, expected: &str) {
    let owner = owner_named(&created.initial.snapshot, "callee");
    let Some(OwnerRecord::Declaration(record)) = created
        .repository
        .view_current()
        .unwrap()
        .owner(owner)
        .unwrap()
        .value
    else {
        panic!("accepted owner must be readable through its canonical binding")
    };
    assert_eq!(record.name.as_str(), expected);
}

#[test]
fn accepted_retry_requires_directory_sync_after_uncertain_head_rename() {
    for point in [
        PublicationPoint::AfterHeadRenamed,
        PublicationPoint::HeadDirectorySyncFailed,
    ] {
        let (_temporary, created) = fixture();
        let prepared = prepare_rename_publication(&created, "durable_retry", Some("sync-retry"));
        let error = created
            .repository
            .publish_with_fault(&prepared, point)
            .unwrap_err();
        assert_eq!(
            error.code,
            if point == PublicationPoint::AfterHeadRenamed {
                "publication_repository_injected_interruption"
            } else {
                "publication_repository_visibility_indeterminate"
            }
        );
        let head_bytes = std::fs::read(created.repository.root().join("HEAD")).unwrap();
        assert_eq!(head_bytes, prepared.head_bytes);
        let reopened = GraphRepository::open(created.repository.root()).unwrap();
        for _ in 0..2 {
            let error = reopened
                .publish_with_fault(&prepared, PublicationPoint::HeadDirectorySyncFailed)
                .expect_err("visible acceptance cannot acknowledge an unsuccessful sync");
            assert_eq!(error.class, DiagnosticClass::Infrastructure);
            assert_eq!(
                error.code,
                "publication_repository_visibility_indeterminate"
            );
            assert_eq!(
                std::fs::read(reopened.root().join("HEAD")).unwrap(),
                head_bytes
            );
            assert_eq!(reopened.current().unwrap().receipt, prepared.receipt);
        }
        // This interruption is reachable on replay only after its directory sync completed.
        assert_eq!(
            reopened
                .publish_with_fault(&prepared, PublicationPoint::AfterHeadDirectorySynced)
                .unwrap_err()
                .code,
            "publication_repository_injected_interruption"
        );
        assert_replay(
            reopened.publish(&prepared).unwrap(),
            &prepared,
            prepared.head,
        );
        assert!(reopened.head_staging_leftovers().unwrap().is_empty());
    }
}

#[test]
fn historical_idempotent_retry_syncs_current_head_and_retains_original_result() {
    let (_temporary, created) = fixture();
    let owner = owner_named(&created.initial.snapshot, "callee");
    let first = prepare_rename_publication(&created, "first_result", Some("historical-sync"));
    assert!(matches!(
        created.repository.publish(&first).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    let second = prepare_current_rename(&created.repository, owner, "later_result", None);
    created
        .repository
        .publish_with_fault(&second, PublicationPoint::AfterHeadRenamed)
        .unwrap_err();
    let error = created
        .repository
        .publish_with_fault(&first, PublicationPoint::HeadDirectorySyncFailed)
        .expect_err("historical replay must durably acknowledge its observed HEAD too");
    assert_eq!(
        error.code,
        "publication_repository_visibility_indeterminate"
    );
    assert_eq!(created.repository.current().unwrap().head, second.head);
    assert_replay(
        created.repository.publish(&first).unwrap(),
        &first,
        second.head,
    );
}

#[test]
fn omitted_new_owner_rejects_before_head_replacement_and_complete_retry_succeeds() {
    let (_temporary, created) = fixture();
    let prepared = prepare_rename_publication(&created, "complete_owner", None);
    let TransactionBody::Change { owners, .. } = &prepared.transaction.body else {
        panic!("rename must be a canonical change")
    };
    assert_eq!(owners.len(), 1);
    let key = ObjectKey::from_digest(
        ObjectDomain::Owner,
        owners[0].objects.after.unwrap().bytes(),
    );
    assert_missing_from_store(&created, key);
    let mut omitted = prepared.clone();
    assert!(omitted.objects.remove(&key).is_some());
    let before = std::fs::read(created.repository.root().join("HEAD")).unwrap();
    let error = created.repository.publish(&omitted).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Corrupt);
    assert_eq!(error.code, "publication_repository_object_missing");
    assert_eq!(
        std::fs::read(created.repository.root().join("HEAD")).unwrap(),
        before
    );
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
    assert!(
        created
            .repository
            .head_staging_leftovers()
            .unwrap()
            .is_empty()
    );
    assert!(matches!(
        created.repository.publish(&prepared).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_owner_name(&created, "complete_owner");
    // A subsequent mutable proposal cannot invalidate the already accepted immutable result.
    omitted.objects.clear();
    assert_replay(
        created.repository.publish(&omitted).unwrap(),
        &prepared,
        prepared.head,
    );
}

#[test]
fn omitted_owner_inventory_is_valid_when_exact_object_was_already_sealed() {
    let (_temporary, created) = fixture();
    let mut prepared = prepare_rename_publication(&created, "sealed_owner", None);
    assert_eq!(
        created
            .repository
            .publish_with_fault(&prepared, PublicationPoint::AfterPacksSealed)
            .unwrap_err()
            .code,
        "publication_repository_injected_interruption"
    );
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
    prepared
        .objects
        .retain(|key, _| key.domain != ObjectDomain::Owner);
    assert!(matches!(
        created.repository.publish(&prepared).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_owner_name(&created, "sealed_owner");
}

#[test]
fn omitted_semantic_or_witness_closure_rejects_without_exposing_head() {
    for selected in 0..4 {
        let (_temporary, created) = fixture();
        let prepared = prepare_rename_publication(&created, "complete_maps", None);
        let witness = prepared.authority.witness.manifest.roots;
        let key = match selected {
            0 => ObjectKey::from_digest(
                ObjectDomain::MapPage,
                prepared.authority.semantic.root.owners.page().bytes(),
            ),
            1 => ObjectKey::from_digest(
                ObjectDomain::MapPage,
                witness.owner_summaries.page().bytes(),
            ),
            2 => ObjectKey::from_digest(ObjectDomain::MapPage, witness.namespaces.page().bytes()),
            _ => *prepared
                .objects
                .keys()
                .find(|key| key.domain == ObjectDomain::OwnerSummary)
                .unwrap(),
        };
        assert_missing_from_store(&created, key);
        let mut omitted = prepared.clone();
        assert!(omitted.objects.remove(&key).is_some());
        let before = std::fs::read(created.repository.root().join("HEAD")).unwrap();
        let error = created.repository.publish(&omitted).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Corrupt, "omitted {key:?}");
        assert_eq!(
            std::fs::read(created.repository.root().join("HEAD")).unwrap(),
            before
        );
        assert_eq!(
            created.repository.current().unwrap().head,
            created.current.head
        );
        assert!(matches!(
            created.repository.publish(&prepared).unwrap(),
            PublicationOutcome::Accepted { .. }
        ));
    }
}

fn prepare_nested_types(created: &CreatedRepository) -> PreparedPublication {
    let module = created
        .initial
        .snapshot
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Module(module) => Some(*module),
            _ => None,
        })
        .unwrap();
    let structural = AuthoredType::StructuralRecord {
        fields: vec![AuthoredStructuralTypeField {
            name: Name::new("values").unwrap(),
            ty: AuthoredType::List {
                item: Box::new(AuthoredType::Text {}),
            },
        }],
    };
    created
        .repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: created.current.head.revision,
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::CreateFunction {
                    symbol: "$identity".to_owned(),
                    module: ModuleSelector::Id { module },
                    name: Name::new("nested_identity").unwrap(),
                    visibility: DeclarationVisibility::Private,
                    type_parameters: Vec::new(),
                    parameters: vec![AuthoredParameter {
                        symbol: "$value".to_owned(),
                        name: Name::new("value").unwrap(),
                        ty: structural.clone(),
                        use_mode: crate::platform::kernel::ParameterUse::Unrestricted,
                        resource_requirement: None,
                    }],
                    result: structural,
                    effect: AuthoredFunctionEffect::Pure {},
                    body: AuthoredExpression {
                        symbol: Some("$body".to_owned()),
                        operation: AuthoredExpressionOperation::Local {
                            value: AuthoredLocalReference::Symbol {
                                symbol: "$value".to_owned(),
                            },
                        },
                    },
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap()
        .publication
}

#[test]
fn omitted_new_type_objects_reject_and_repaired_nested_type_closure_publishes() {
    let (_temporary, created) = fixture();
    let prepared = prepare_nested_types(&created);
    let types = prepared
        .objects
        .keys()
        .filter(|key| key.domain == ObjectDomain::Type)
        .copied()
        .collect::<Vec<_>>();
    assert!(types.len() >= 2, "fixture must add nested structural types");
    for selected in 0..types.len() {
        let (_temporary, created) = fixture();
        let prepared = prepare_nested_types(&created);
        let keys = prepared
            .objects
            .keys()
            .filter(|key| key.domain == ObjectDomain::Type)
            .copied()
            .collect::<Vec<_>>();
        assert_eq!(keys.len(), types.len());
        let key = keys[selected];
        assert_missing_from_store(&created, key);
        let mut omitted = prepared.clone();
        omitted.objects.remove(&key).unwrap();
        let error = created.repository.publish(&omitted).unwrap_err();
        assert_eq!(error.code, "publication_repository_object_missing");
        assert_eq!(
            created.repository.current().unwrap().head,
            created.current.head
        );
        assert!(matches!(
            created.repository.publish(&prepared).unwrap(),
            PublicationOutcome::Accepted { .. }
        ));
        assert!(
            created
                .repository
                .view_current()
                .unwrap()
                .type_object(TypeObjectDigest::from_bytes(key.digest.bytes()))
                .unwrap()
                .value
                .is_some()
        );
    }
}

#[test]
fn exhausted_candidate_reference_admission_preserves_head_and_sealed_owner_reuse() {
    let (_temporary, created) = fixture();
    let mut prepared = prepare_rename_publication(&created, "bounded_reuse", None);
    let before = std::fs::read(created.repository.root().join("HEAD")).unwrap();
    let limits = crate::platform::change::CanonicalReadAdmission {
        maximum_decoded_records: 0,
        ..Default::default()
    };
    let error = created
        .repository
        .publish_with_fault(&prepared, PublicationPoint::ReferenceAdmission(limits))
        .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(
        error.code,
        "publication_repository_reference_visits_exhausted"
    );
    assert_eq!(
        std::fs::read(created.repository.root().join("HEAD")).unwrap(),
        before
    );
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
    assert!(
        created
            .repository
            .head_staging_leftovers()
            .unwrap()
            .is_empty()
    );
    // Refusal can retain sealed objects; exact immutable reuse still succeeds.
    prepared
        .objects
        .retain(|key, _| key.domain != ObjectDomain::Owner);
    assert!(matches!(
        created.repository.publish(&prepared).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_owner_name(&created, "bounded_reuse");
}
