use super::*;
use crate::platform::change::{CanonicalDelta, PrimitiveEdit, prepare_change_analysis};
use crate::platform::kernel::{
    DeclarationPayload, ExpressionOperation, Name, OwnerKey, OwnerRecord, encode_owner,
};
use crate::platform::storage::memory::MemoryPackedStore;
use crate::platform::storage::object::{ImmutableObjectStore, ObjectDomain, ObjectKey, StoreWork};

#[test]
fn initial_history_binds_every_layer_and_reopens_from_packs() {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let mut store = MemoryPackedStore::default();
    let initial =
        prepare_initial_publication(&logical, &store, Some("fixture bootstrap".to_owned()))
            .expect("initial publication must prepare");
    let publication = &initial.publication;

    assert!(publication.expected_base.is_none());
    assert_eq!(publication.revision.core.parents, Vec::new());
    assert_eq!(
        publication.receipt.status,
        PublicationStatus::ProjectCreated
    );
    assert_eq!(
        publication.receipt.validation.profile,
        ValidationProfile::FullRebuild
    );
    assert_eq!(
        publication.receipt.validation.full_oracle,
        FullOracleStatus::NotApplicable
    );
    assert_eq!(
        publication.receipt.work.validation_work,
        initial.witness.report.full_validation.work_consumed
    );
    assert_eq!(publication.receipt.work.expression_work, 0);
    assert_eq!(
        publication.revision.core.semantic_root,
        publication.authority.semantic.digest
    );
    assert_eq!(
        publication.revision.core.validation_witness,
        publication.authority.witness.digest
    );
    assert_eq!(publication.accepted.head, publication.head);
    assert_history_objects_present(publication);
    round_trip_history(publication);

    install(&mut store, publication);
    let mut work = StoreWork::default();
    let revision_bytes = store
        .read(
            ObjectKey::from_digest(ObjectDomain::Revision, publication.revision_digest.bytes()),
            ObjectDomain::Revision.maximum_bytes(),
            &mut work,
        )
        .expect("packed revision read")
        .expect("packed revision exists");
    assert_eq!(revision_bytes, publication.revision_bytes);
    assert_eq!(
        RevisionRecord::decode(&revision_bytes, publication.revision_digest)
            .expect("packed revision decode"),
        publication.revision
    );

    let repeated = prepare_initial_publication(&logical, &MemoryPackedStore::default(), None)
        .expect("repeated initial preparation");
    assert_eq!(
        repeated.publication.head.revision,
        publication.head.revision
    );
    assert_eq!(
        repeated.publication.authority.semantic.bytes,
        publication.authority.semantic.bytes
    );
    assert_eq!(
        repeated.publication.authority.witness.bytes,
        publication.authority.witness.bytes
    );
}

#[test]
fn incremental_publication_is_deterministic_parent_bound_and_isolated() {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let mut store = MemoryPackedStore::default();
    let initial =
        prepare_initial_publication(&logical, &store, None).expect("initial publication prepares");
    install(&mut store, &initial.publication);
    let base = initial.publication.accepted;
    let body = function_body(&initial.snapshot, "callee");
    let caller = owner_named(&initial.snapshot, "caller");
    let mut replacement = initial.snapshot.owners[&body].clone();
    let OwnerRecord::Expression(expression) = &mut replacement else {
        panic!("callee body must be an expression");
    };
    expression.operation = ExpressionOperation::Unit;
    let expected = encode_owner(&initial.snapshot.owners[&body])
        .expect("base expression encoding")
        .0;
    let delta = CanonicalDelta::normalize(
        &initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
    )
    .expect("canonical body delta");
    let analysis =
        prepare_change_analysis(&initial.snapshot, &initial.witness, delta).expect("analysis");
    let prepared = prepare_change_publication(
        base,
        &initial.snapshot,
        &initial.witness,
        &analysis,
        &store,
        PublicationOptions::default(),
    )
    .expect("incremental publication prepares");

    assert_eq!(prepared.expected_base, Some(base.head));
    assert_eq!(prepared.revision.core.parents, vec![base.parent()]);
    assert_eq!(prepared.receipt.bases, vec![base.head.revision]);
    assert_eq!(
        prepared.receipt.validation.profile,
        ValidationProfile::IncrementalOwnerFrontier
    );
    assert_eq!(
        prepared.receipt.validation.full_oracle,
        FullOracleStatus::NotRun
    );
    assert_eq!(prepared.receipt.counts.owners_updated, 1);
    assert_eq!(
        prepared.receipt.work.validation_work,
        prepared
            .receipt
            .work
            .owner_records_checked
            .saturating_add(prepared.receipt.work.ownership_entries_checked)
            .saturating_add(prepared.receipt.work.type_objects_checked)
            .saturating_add(prepared.receipt.work.expression_work)
    );
    assert_ne!(prepared.head.revision, base.head.revision);
    assert_history_objects_present(&prepared);
    round_trip_history(&prepared);

    let SemanticDiffBody::Change { owners, .. } = &prepared.semantic_diff.body else {
        panic!("body edit must produce a change diff");
    };
    let body_diff = owners
        .iter()
        .find(|entry| entry.owner == body)
        .expect("body diff entry");
    assert!(
        body_diff
            .classes
            .contains(&OwnerChangeClass::SemanticPayloadChanged)
    );
    let caller_digest = encode_owner(&initial.snapshot.owners[&caller])
        .expect("caller encoding")
        .0;
    assert!(!prepared.objects.contains_key(&ObjectKey::from_digest(
        ObjectDomain::Owner,
        caller_digest.bytes(),
    )));

    let candidate_root = ObjectKey::from_digest(
        ObjectDomain::SemanticRoot,
        prepared.authority.semantic.digest.bytes(),
    );
    assert!(
        store
            .read(
                candidate_root,
                ObjectDomain::SemanticRoot.maximum_bytes(),
                &mut StoreWork::default(),
            )
            .expect("accepted base lookup")
            .is_none(),
        "preparation must not mutate accepted packs"
    );

    let repeated = prepare_change_publication(
        base,
        &initial.snapshot,
        &initial.witness,
        &analysis,
        &store,
        PublicationOptions::default(),
    )
    .expect("repeated preparation");
    assert_eq!(repeated.head, prepared.head);
    assert_eq!(repeated.revision_bytes, prepared.revision_bytes);
    assert_eq!(repeated.receipt_bytes, prepared.receipt_bytes);
    assert_eq!(repeated.objects, prepared.objects);
}

#[test]
fn history_codecs_reject_predecessor_magic_and_inconsistent_evidence() {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let store = MemoryPackedStore::default();
    let initial = prepare_initial_publication(&logical, &store, None).expect("initial publication");

    let mut predecessor_head = initial.publication.head_bytes.clone();
    predecessor_head[..8].copy_from_slice(b"LKJHEAD4");
    let error = HeadRecord::decode(&predecessor_head).expect_err("predecessor HEAD must reject");
    assert_eq!(error.code, "packed_contract");

    let mut foreign_head = initial.publication.head;
    foreign_head.contract_version -= 1;
    assert_eq!(
        AcceptedBinding::verify(
            foreign_head,
            &initial.publication.revision,
            initial.publication.authority.witness.digest,
            &initial.publication.authority.witness.manifest,
        )
        .expect_err("programmatic predecessor HEAD must reject")
        .code,
        "publication_head_contract"
    );

    let mut receipt = initial.publication.receipt.clone();
    receipt.validation.tests_selected = 1;
    receipt.validation.tests_executed = 2;
    assert_eq!(
        receipt
            .encode()
            .expect_err("inconsistent tests must reject")
            .code,
        "publication_receipt_tests"
    );

    let mut false_incremental_bootstrap = initial.publication.receipt.clone();
    false_incremental_bootstrap.validation.profile = ValidationProfile::IncrementalOwnerFrontier;
    false_incremental_bootstrap.validation.full_oracle = FullOracleStatus::NotRun;
    assert_eq!(
        false_incremental_bootstrap
            .encode()
            .expect_err("project creation cannot claim incremental-only evidence")
            .code,
        "publication_receipt_validation_profile"
    );

    let mut wrong_digest = initial.publication.transaction_digest.bytes();
    wrong_digest[0] ^= 1;
    assert_eq!(
        NormalizedTransaction::decode(
            &initial.publication.transaction_bytes,
            TransactionDigest::from_bytes(wrong_digest),
        )
        .expect_err("foreign transaction digest must reject")
        .code,
        "publication_transaction_digest"
    );

    let mut duplicate_parent_core = initial.publication.revision.core.clone();
    let parent = ParentRevision {
        revision: initial.publication.revision.revision,
        record: initial.publication.revision_digest,
    };
    let mut foreign_record = initial.publication.revision_digest.bytes();
    foreign_record[0] ^= 1;
    duplicate_parent_core.parents = vec![
        parent,
        ParentRevision {
            revision: parent.revision,
            record: RevisionObjectDigest::from_bytes(foreign_record),
        },
    ];
    assert_eq!(
        duplicate_parent_core
            .revision_id()
            .expect_err("one logical parent cannot appear through two receipt records")
            .code,
        "publication_revision_parents"
    );
}

fn install(store: &mut MemoryPackedStore, publication: &PreparedPublication) {
    let mut work = StoreWork::default();
    for (key, bytes) in &publication.objects {
        store
            .stage(*key, bytes, &mut work)
            .expect("prepared object must stage");
    }
    store
        .seal_staged(64 * 1024, &mut work)
        .expect("prepared objects must seal");
}

fn assert_history_objects_present(publication: &PreparedPublication) {
    for (domain, digest) in [
        (
            ObjectDomain::Transaction,
            publication.transaction_digest.bytes(),
        ),
        (
            ObjectDomain::SemanticDiff,
            publication.semantic_diff_digest.bytes(),
        ),
        (ObjectDomain::Receipt, publication.receipt_digest.bytes()),
        (ObjectDomain::Revision, publication.revision_digest.bytes()),
    ] {
        let bytes = match domain {
            ObjectDomain::Transaction => &publication.transaction_bytes,
            ObjectDomain::SemanticDiff => &publication.semantic_diff_bytes,
            ObjectDomain::Receipt => &publication.receipt_bytes,
            ObjectDomain::Revision => &publication.revision_bytes,
            _ => unreachable!("closed history object domain"),
        };
        assert_eq!(
            ObjectKey::for_bytes(domain, bytes),
            ObjectKey::from_digest(domain, digest),
            "typed and generic object digests disagree for {domain:?}"
        );
        assert!(
            publication
                .objects
                .contains_key(&ObjectKey::from_digest(domain, digest)),
            "prepared closure omits {domain:?}"
        );
    }
}

fn round_trip_history(publication: &PreparedPublication) {
    assert_eq!(
        NormalizedTransaction::decode(
            &publication.transaction_bytes,
            publication.transaction_digest,
        )
        .expect("transaction round trip"),
        publication.transaction
    );
    assert_eq!(
        SemanticDiff::decode(
            &publication.semantic_diff_bytes,
            publication.semantic_diff_digest,
        )
        .expect("diff round trip"),
        publication.semantic_diff
    );
    assert_eq!(
        PublicationReceipt::decode(&publication.receipt_bytes, publication.receipt_digest)
            .expect("receipt round trip"),
        publication.receipt
    );
    assert_eq!(
        RevisionRecord::decode(&publication.revision_bytes, publication.revision_digest)
            .expect("revision round trip"),
        publication.revision
    );
    assert_eq!(
        HeadRecord::decode(&publication.head_bytes).expect("HEAD round trip"),
        publication.head
    );
}

fn owner_named(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Declaration(declaration) if declaration.name.as_str() == name => {
                Some(*owner)
            }
            _ => None,
        })
        .expect("named declaration")
}

fn function_body(snapshot: &crate::platform::kernel::KernelSnapshot, name: &str) -> OwnerKey {
    let declaration = owner_named(snapshot, name);
    let OwnerRecord::Declaration(record) = &snapshot.owners[&declaration] else {
        panic!("named owner must be a declaration");
    };
    let DeclarationPayload::Function(function) = &record.payload else {
        panic!("named declaration must be a function");
    };
    OwnerKey::Expression(function.body)
}

#[test]
fn rename_diff_is_stable_identity_and_presentation_only() {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let mut store = MemoryPackedStore::default();
    let initial = prepare_initial_publication(&logical, &store, None).expect("initial publication");
    install(&mut store, &initial.publication);
    let callee = owner_named(&initial.snapshot, "callee");
    let mut replacement = initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration");
    };
    declaration.name = Name::new("renamed_callee").expect("new name");
    let expected = encode_owner(&initial.snapshot.owners[&callee])
        .expect("callee encoding")
        .0;
    let delta = CanonicalDelta::normalize(
        &initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
    )
    .expect("rename delta");
    let analysis = prepare_change_analysis(&initial.snapshot, &initial.witness, delta)
        .expect("rename analysis");
    let prepared = prepare_change_publication(
        initial.publication.accepted,
        &initial.snapshot,
        &initial.witness,
        &analysis,
        &store,
        PublicationOptions::default(),
    )
    .expect("rename publication");
    let SemanticDiffBody::Change { owners, .. } = &prepared.semantic_diff.body else {
        panic!("rename must produce change diff");
    };
    let entry = owners
        .iter()
        .find(|entry| entry.owner == callee)
        .expect("callee diff");
    assert!(entry.classes.contains(&OwnerChangeClass::Renamed));
    assert!(entry.classes.contains(&OwnerChangeClass::PresentationOnly));
    assert!(!entry.dimensions.executable());

    let mut impossible = prepared.semantic_diff.clone();
    let SemanticDiffBody::Change { owners, .. } = &mut impossible.body else {
        unreachable!("cloned rename diff remains a change")
    };
    let entry = owners
        .iter_mut()
        .find(|entry| entry.owner == callee)
        .expect("callee diff remains present");
    entry.objects.before = None;
    entry.classes.push(OwnerChangeClass::Created);
    entry.classes.sort_unstable();
    entry.classes.dedup();
    assert_eq!(
        impossible
            .encode()
            .expect_err("created owner cannot also claim a rename")
            .code,
        "publication_diff_owner_transition"
    );
}

#[test]
fn move_diff_preserves_declaration_and_caller_identity() {
    let logical = crate::platform::kernel::tests::witness_snapshot();
    let mut store = MemoryPackedStore::default();
    let initial = prepare_initial_publication(&logical, &store, None).expect("initial publication");
    install(&mut store, &initial.publication);
    let callee = owner_named(&initial.snapshot, "callee");
    let caller = owner_named(&initial.snapshot, "caller");
    let caller_before = encode_owner(&initial.snapshot.owners[&caller])
        .expect("caller encoding")
        .0;
    let mut replacement = initial.snapshot.owners[&callee].clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        panic!("callee must be a declaration");
    };
    let original_module = declaration.module;
    declaration.module = initial
        .snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match (owner, record) {
            (OwnerKey::Module(module), OwnerRecord::Module(_)) if *module != original_module => {
                Some(*module)
            }
            _ => None,
        })
        .expect("second module");
    let expected = encode_owner(&initial.snapshot.owners[&callee])
        .expect("callee encoding")
        .0;
    let delta = CanonicalDelta::normalize(
        &initial.snapshot,
        vec![PrimitiveEdit::ReplaceOwner {
            expected,
            record: replacement,
        }],
    )
    .expect("move delta");
    let analysis =
        prepare_change_analysis(&initial.snapshot, &initial.witness, delta).expect("move analysis");
    let prepared = prepare_change_publication(
        initial.publication.accepted,
        &initial.snapshot,
        &initial.witness,
        &analysis,
        &store,
        PublicationOptions::default(),
    )
    .expect("move publication");
    let SemanticDiffBody::Change { owners, .. } = &prepared.semantic_diff.body else {
        panic!("move must produce change diff");
    };
    let entry = owners
        .iter()
        .find(|entry| entry.owner == callee)
        .expect("callee move diff");
    assert!(entry.classes.contains(&OwnerChangeClass::Moved));
    assert_eq!(
        encode_owner(&initial.snapshot.owners[&caller])
            .expect("caller re-encoding")
            .0,
        caller_before
    );
    assert!(!prepared.objects.contains_key(&ObjectKey::from_digest(
        ObjectDomain::Owner,
        caller_before.bytes(),
    )));
}
