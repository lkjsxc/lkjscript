//! Independent full-oracle, bounded-admission and failure proofs for reviewed refresh.
use super::*;
use crate::platform::change::{
    AuthoredChange, AuthoredChangeSet, AuthoredDeletePolicy, AuthoredReadFootprint, AuthoredType,
    ChangeBudget, OwnerSelector, WitnessBaseRead,
};
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::execution::ExecutionControl;
use crate::platform::kernel::{KernelSnapshot, Name, OwnerKey, OwnerRecord, RelationKind};
use crate::platform::semantic_id::RevisionId;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

fn fixture() -> (tempfile::TempDir, CreatedRepository) {
    let directory = tempfile::tempdir().unwrap();
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let created =
        GraphRepository::create(&directory.path().join("meaning"), &snapshot, None).unwrap();
    (directory, created)
}

fn named(snapshot: &KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| {
            (record
                .name()
                .is_some_and(|observed| observed.as_str() == name))
            .then_some(*owner)
        })
        .unwrap()
}

fn rename(base: RevisionId, owner: OwnerKey, name: &str) -> AuthoredChangeSet {
    AuthoredChangeSet {
        base,
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::RenameOwner {
            owner: OwnerSelector::Exact { owner },
            name: Name::new(name).unwrap(),
        }],
    }
}

fn prepare_text(repository: &GraphRepository, body: &str) -> PreparedAuthoredPublication {
    let text = format!(
        "request base={}\n{body}",
        repository.current().unwrap().head.revision
    );
    let request =
        crate::platform::control::decode_compact_change("refresh-core-fixture", text.as_bytes())
            .unwrap();
    repository
        .prepare_authored_change(&request.semantic, request.options)
        .unwrap()
}

fn inventory(repository: &GraphRepository) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, directory: &Path, result: &mut BTreeMap<PathBuf, Vec<u8>>) {
        for entry in std::fs::read_dir(directory).unwrap() {
            let entry = entry.unwrap();
            if entry.file_type().unwrap().is_dir() {
                visit(root, &entry.path(), result);
            } else {
                assert!(entry.file_type().unwrap().is_file());
                result.insert(
                    entry.path().strip_prefix(root).unwrap().to_owned(),
                    std::fs::read(entry.path()).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(repository.root(), repository.root(), &mut result);
    result
}

#[test]
fn refresh_disjoint_owner_edits_match_independently_composed_full_snapshot_and_witness() {
    let (_directory, created) = fixture();
    let origin = created.repository.view_current().unwrap();
    let before = origin.reconstruct_full_oracle().unwrap().value;
    let first = named(&before, "callee");
    let second = named(&before, "caller");
    let request = rename(origin.revision(), first, "reviewed_callee");
    let original = origin
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    let concurrent = origin
        .prepare_authored_change(
            &rename(origin.revision(), second, "concurrent_caller"),
            PublicationOptions::default(),
        )
        .unwrap();
    created.repository.publish(&concurrent.publication).unwrap();
    let target = created.repository.view_current().unwrap();
    let durable_before = inventory(&created.repository);
    let renewed = target
        .prepare_refreshed_authored_change(
            &request,
            PublicationOptions::default(),
            &original,
            &AuthoredReadFootprint::default(),
        )
        .unwrap();
    assert_eq!(
        inventory(&created.repository),
        durable_before,
        "refresh preparation must remain isolated"
    );
    assert_eq!(
        renewed.publication.expected_base,
        Some(target.current().head)
    );
    assert_eq!(renewed.publication.receipt.bases, vec![target.revision()]);
    assert_eq!(renewed.allocated, original.allocated);
    created.repository.publish(&renewed.publication).unwrap();
    let accepted = created.repository.current().unwrap();
    let actual = created
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    // Independently compose the two requested record changes without replaying refresh/lowering.
    let mut expected = before.owners.clone();
    for (owner, name) in [(first, "reviewed_callee"), (second, "concurrent_caller")] {
        let OwnerRecord::Declaration(record) = expected.get_mut(&owner).unwrap() else {
            panic!("declaration fixture")
        };
        record.name = Name::new(name).unwrap();
    }
    assert_eq!(actual.owners, expected);
    assert_eq!(actual.dependencies, before.dependencies);
    assert_eq!(actual.retirements, before.retirements);
    assert_eq!(actual.types, before.types);
    crate::platform::kernel::validate_full(&actual).unwrap();
    let independent = crate::platform::witness::rebuild_full_witness(&actual).unwrap();
    assert_eq!(
        independent.manifest_digest,
        accepted.accepted.validation_witness
    );
    assert_eq!(independent.manifest, accepted.witness);
    // Pinned origin reads survive both publications, including their original name answers.
    assert_eq!(
        origin.reconstruct_full_oracle().unwrap().value.owners,
        before.owners
    );
    let reopened = created
        .repository
        .view_ancestor_from(
            &created.repository.view_current().unwrap(),
            request.base,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(reopened.current().head, origin.current().head);
}

#[test]
fn refresh_deletion_preserves_original_retirement_provenance_after_unrelated_publication() {
    let (_directory, created) = fixture();
    let disposable = prepare_text(
        &created.repository,
        "create.module as=$module name=refresh_disposable\n\
         expression.unit as=$body\n\
         create.function as=$function module=$module name=disposable visibility=private result=unit effect=pure body=$body\n",
    );
    created.repository.publish(&disposable.publication).unwrap();
    let origin = created.repository.view_current().unwrap();
    let request = AuthoredChangeSet {
        base: origin.revision(),
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::DeleteOwner {
            owner: OwnerSelector::Exact {
                owner: disposable.allocated["$module"],
            },
            policy: AuthoredDeletePolicy::OwnedClosure,
        }],
    };
    let original = origin
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    let expected: BTreeMap<_, _> = original
        .logical_plan
        .retirements
        .iter()
        .map(|(owner, values)| (*owner, values.after.clone().unwrap()))
        .collect();
    assert_eq!(expected.len(), 3);
    for owner in disposable.allocated.values() {
        assert_eq!(expected[owner].last_live_revision, request.base);
    }
    let concurrent = origin
        .prepare_authored_change(
            &rename(
                origin.revision(),
                named(&created.initial.snapshot, "caller"),
                "retirement_concurrent_caller",
            ),
            PublicationOptions::default(),
        )
        .unwrap();
    created.repository.publish(&concurrent.publication).unwrap();
    let target = created.repository.view_current().unwrap();
    assert_ne!(target.revision(), request.base);
    let renewed = target
        .prepare_refreshed_authored_change(
            &request,
            PublicationOptions::default(),
            &original,
            &AuthoredReadFootprint::default(),
        )
        .unwrap();
    for (owner, retirement) in &expected {
        assert_eq!(
            renewed.logical_plan.retirements[owner].after.as_ref(),
            Some(retirement)
        );
        assert_ne!(retirement.last_live_revision, target.revision());
    }
    created.repository.publish(&renewed.publication).unwrap();
    let accepted = created.repository.current().unwrap();
    let actual = created
        .repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    assert_eq!(actual.retirements, expected);
    for owner in disposable.allocated.values() {
        assert!(!actual.owners.contains_key(owner));
    }
    crate::platform::kernel::validate_full(&actual).unwrap();
    let independent = crate::platform::witness::rebuild_full_witness(&actual).unwrap();
    assert_eq!(independent.manifest, accepted.witness);
    assert_eq!(
        independent.manifest_digest,
        accepted.accepted.validation_witness
    );
}

#[test]
fn refresh_guard_and_staging_exhaustion_and_cancellation_publish_no_objects() {
    let (_directory, created) = fixture();
    let origin = created.repository.view_current().unwrap();
    let first = named(&created.initial.snapshot, "callee");
    let request = rename(origin.revision(), first, "bounded_callee");
    let original = origin
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    let concurrent = origin
        .prepare_authored_change(
            &rename(
                origin.revision(),
                named(&created.initial.snapshot, "caller"),
                "bounded_caller",
            ),
            PublicationOptions::default(),
        )
        .unwrap();
    created.repository.publish(&concurrent.publication).unwrap();
    let target = created.repository.view_current().unwrap();
    let before = inventory(&created.repository);
    for staging in [false, true] {
        let mut bounded = request.clone();
        if staging {
            bounded.budget.staging.maximum_objects = 0;
        } else {
            bounded.budget.canonical_reads.maximum_point_reads = 0;
        }
        let errors = target
            .prepare_refreshed_authored_change(
                &bounded,
                PublicationOptions::default(),
                &original,
                &AuthoredReadFootprint::default(),
            )
            .unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.class == DiagnosticClass::Resource
                    && error.code.starts_with("change_budget_")),
            "{errors:?}"
        );
        assert_eq!(inventory(&created.repository), before);
    }
    let control = ExecutionControl::uncancelled();
    let cancelled_target = created
        .repository
        .view_current_with_control(&control)
        .unwrap();
    control.cancel();
    let errors = cancelled_target
        .prepare_refreshed_authored_change(
            &request,
            PublicationOptions::default(),
            &original,
            &AuthoredReadFootprint::default(),
        )
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.class == DiagnosticClass::Cancelled),
        "{errors:?}"
    );
    assert_eq!(inventory(&created.repository), before);
    let error = match created
        .repository
        .view_ancestor_from(&target, request.base, &control)
    {
        Err(error) => error,
        Ok(_) => panic!("cancelled origin traversal must reject"),
    };
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(inventory(&created.repository), before);
}

#[test]
fn refresh_rejects_truncated_negative_range_and_new_referrers_without_partial_publication() {
    let (_directory, created) = fixture();
    let module = created
        .initial
        .snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| matches!(record, OwnerRecord::Module(_)).then_some(*owner))
        .unwrap();
    let added = prepare_text(
        &created.repository,
        &format!(
            "expression.unit as=$body\ncreate.function as=$unused module={module} name=unused_refresh visibility=private result=unit effect=pure body=$body\n"
        ),
    );
    let unused = added.allocated["$unused"];
    created.repository.publish(&added.publication).unwrap();
    let origin = created.repository.view_current().unwrap();
    let read = origin
        .read_incoming_relations_of_kind(unused, RelationKind::FunctionCall, 1)
        .unwrap();
    assert!(read.value.edges.is_empty());
    assert!(!read.value.truncated);
    let mut native_reads = AuthoredReadFootprint::default();
    native_reads
        .record_relations(
            unused,
            Some(RelationKind::FunctionCall),
            true,
            1,
            &read.value.edges,
            false,
        )
        .unwrap();
    let mut incomplete = AuthoredReadFootprint::default();
    let error = incomplete
        .record_relations(unused, Some(RelationKind::FunctionCall), true, 1, &[], true)
        .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(error.code, "change_refresh_incomplete_footprint");
    assert!(
        incomplete.is_empty(),
        "incomplete observations must not retain absence evidence"
    );
    let request = rename(
        origin.revision(),
        named(&created.initial.snapshot, "callee"),
        "range_callee",
    );
    let original = origin
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    let incoming = prepare_text(
        &created.repository,
        &format!(
            "expression.call as=$first_call function={unused}\ncreate.function as=$first module={module} name=first_refresh_referrer visibility=private result=unit effect=pure body=$first_call\n\
         expression.call as=$second_call function={unused}\ncreate.function as=$second module={module} name=second_refresh_referrer visibility=private result=unit effect=pure body=$second_call\n"
        ),
    );
    created.repository.publish(&incoming.publication).unwrap();
    let target = created.repository.view_current().unwrap();
    let full = target
        .read_incoming_relations_of_kind(unused, RelationKind::FunctionCall, 10)
        .unwrap();
    assert_eq!(full.value.edges.len(), 2);
    assert!(!full.value.truncated);
    assert!(
        target
            .read_incoming_relations_of_kind(unused, RelationKind::FunctionCall, 1)
            .unwrap()
            .value
            .truncated
    );
    let before = inventory(&created.repository);
    let errors = target
        .prepare_refreshed_authored_change(
            &request,
            PublicationOptions::default(),
            &original,
            &native_reads,
        )
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "change_refresh_conflict"),
        "{errors:?}"
    );
    assert_eq!(inventory(&created.repository), before);
    crate::platform::kernel::validate_full(&target.reconstruct_full_oracle().unwrap().value)
        .unwrap();
}

#[test]
fn refresh_renews_untaken_nominal_constraint_validation_after_referenced_field_changes() {
    let (_directory, created) = fixture();
    let setup = prepare_text(
        &created.repository,
        "create.module as=$module name=refresh_contracts\n\
         create.record as=$record module=$module name=Env visibility=private\n\
         add.field as=$field record=$record name=value type=i64\n\
         expression.unit as=$body\n\
         create.function as=$generic module=$module name=generic visibility=private result=unit effect=pure body=$body\n\
         add.type-parameter as=$T declaration=$generic name=T constraint=capture-safe\n",
    );
    let module = setup.allocated["$module"];
    let record = setup.allocated["$record"];
    let field = setup.allocated["$field"];
    let generic = setup.allocated["$generic"];
    created.repository.publish(&setup.publication).unwrap();
    let origin = created.repository.view_current().unwrap();
    let text = format!(
        "request base={}\ntype.named as=@Env declaration={record}\n\
         expression.call as=$call function={generic}\n\
         type.argument parent=$call index=0 type=@Env\n\
         expression.bool as=$condition value=true\n\
         expression.unit as=$unit\n\
         expression.if as=$body condition=$condition when-true=$unit when-false=$call\n\
         create.function as=$caller module={module} name=reviewed_caller visibility=private result=unit effect=pure body=$body\n",
        origin.revision()
    );
    let decoded = crate::platform::control::decode_compact_change(
        "refresh-untaken-constraint",
        text.as_bytes(),
    )
    .unwrap();
    let original = origin
        .prepare_authored_change(&decoded.semantic, decoded.options.clone())
        .unwrap();
    let changed = origin
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: origin.revision(),
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::SetFieldType {
                    field: OwnerSelector::Exact { owner: field },
                    ty: AuthoredType::Secret {},
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap();
    created.repository.publish(&changed.publication).unwrap();
    let target = created.repository.view_current().unwrap();
    crate::platform::kernel::validate_full(&target.reconstruct_full_oracle().unwrap().value)
        .unwrap();
    // The named type and generic owner have the same identities; their complete instantiated
    // contract must be admitted again, including the untaken branch of the proposed caller.
    original
        .intent_reads
        .check_against(&target, &target, decoded.semantic.budget)
        .unwrap();
    let before = inventory(&created.repository);
    let errors = target
        .prepare_refreshed_authored_change(
            &decoded.semantic,
            decoded.options,
            &original,
            &AuthoredReadFootprint::default(),
        )
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "kernel_type_constraint"),
        "{errors:?}"
    );
    assert_eq!(inventory(&created.repository), before);
    assert!(
        target
            .owner(original.allocated["$caller"])
            .unwrap()
            .value
            .is_none()
    );
}

#[test]
fn refresh_origin_authentication_and_locked_target_reject_without_staging() {
    let (_directory, created) = fixture();
    let origin = created.repository.view_current().unwrap();
    let request = rename(
        origin.revision(),
        named(&created.initial.snapshot, "callee"),
        "stale_callee",
    );
    let original = origin
        .prepare_authored_change(&request, PublicationOptions::default())
        .unwrap();
    let concurrent = origin
        .prepare_authored_change(
            &rename(
                origin.revision(),
                named(&created.initial.snapshot, "caller"),
                "stale_caller",
            ),
            PublicationOptions::default(),
        )
        .unwrap();
    created.repository.publish(&concurrent.publication).unwrap();
    let target = created.repository.view_current().unwrap();
    let before = inventory(&created.repository);
    let foreign_origin = RevisionId::from_digest([91; 32]);
    let error = match created.repository.view_ancestor_from(
        &target,
        foreign_origin,
        &ExecutionControl::uncancelled(),
    ) {
        Err(error) => error,
        Ok(_) => panic!("unaccepted origin must reject"),
    };
    assert_eq!(error.class, DiagnosticClass::Semantic);
    assert_eq!(error.code, "change_refresh_origin_unreachable");
    let mut invalid = request.clone();
    invalid.base = foreign_origin;
    let errors = target
        .prepare_refreshed_authored_change(
            &invalid,
            PublicationOptions::default(),
            &original,
            &AuthoredReadFootprint::default(),
        )
        .unwrap_err();
    assert!(
        errors
            .iter()
            .any(|error| error.code == "change_refresh_origin_binding"),
        "{errors:?}"
    );
    assert_eq!(inventory(&created.repository), before);
    let renewed = target
        .prepare_refreshed_authored_change(
            &request,
            PublicationOptions::default(),
            &original,
            &AuthoredReadFootprint::default(),
        )
        .unwrap();
    let intervening = target
        .prepare_authored_change(
            &rename(
                target.revision(),
                named(&created.initial.snapshot, "caller"),
                "later_caller",
            ),
            PublicationOptions::default(),
        )
        .unwrap();
    created
        .repository
        .publish(&intervening.publication)
        .unwrap();
    let advanced = inventory(&created.repository);
    assert!(matches!(
        created.repository.publish(&renewed.publication).unwrap(),
        PublicationOutcome::Stale { .. }
    ));
    assert_eq!(
        inventory(&created.repository),
        advanced,
        "locked stale decision must precede every durable object stage"
    );
    assert_eq!(
        created.repository.current().unwrap().head,
        intervening.publication.head
    );
}

#[test]
fn authenticated_target_ancestry_admission_excludes_later_descendants() {
    let (_directory, created) = fixture();
    let origin = created.repository.view_current().unwrap();
    let owner = named(&created.initial.snapshot, "callee");
    let target_change = origin
        .prepare_authored_change(
            &rename(origin.revision(), owner, "ancestry_target"),
            PublicationOptions::default(),
        )
        .unwrap();
    created
        .repository
        .publish(&target_change.publication)
        .unwrap();
    let target = created.repository.view_current().unwrap();
    for name in ["ancestry_descendant_one", "ancestry_descendant_two"] {
        let current = created.repository.view_current().unwrap();
        let change = current
            .prepare_authored_change(
                &rename(current.revision(), owner, name),
                PublicationOptions::default(),
            )
            .unwrap();
        created.repository.publish(&change.publication).unwrap();
    }
    let before = inventory(&created.repository);
    let control = ExecutionControl::uncancelled();
    let reopened = created
        .repository
        .view_ancestor_from_with_limit(&target, origin.revision(), &control, 2)
        .unwrap();
    assert_eq!(reopened.current().head, origin.current().head);
    let exhausted = match created.repository.view_ancestor_from_with_limit(
        &target,
        origin.revision(),
        &control,
        1,
    ) {
        Err(error) => error,
        Ok(_) => panic!("both target and origin require an admitted ancestry visit"),
    };
    assert_eq!(exhausted.class, DiagnosticClass::Resource);
    assert_eq!(exhausted.code, "change_refresh_history_capacity");
    assert_eq!(inventory(&created.repository), before);
}
