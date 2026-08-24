//! Binding validated semantic authority into exact Graph 5 transaction, diff, and history objects.

use super::contract::{
    RECEIPT_CONTRACT_VERSION, REVISION_CONTRACT_VERSION, SEMANTIC_DIFF_CONTRACT_VERSION,
    TRANSACTION_CONTRACT_VERSION,
};
use super::idempotency::{
    IdempotencyBinding, advance_idempotency_history, empty_idempotency_history,
};
use super::{
    AcceptedBinding, ChangeCounts, DependencyDiffEntry, DependencyTransactionEdit, DigestEdit,
    FullOracleStatus, HeadRecord, NormalizedTransaction, OwnerChangeClass, OwnerDiffEntry,
    OwnerTransactionEdit, PublicationReceipt, PublicationStatus, RetirementDiffEntry,
    RetirementTransactionEdit, RevisionCore, RevisionRecord, SemanticDiff, SemanticDiffBody,
    SummaryDimensions, TransactionBody, ValidationEvidence, ValidationProfile, WorkObservation,
};
use crate::platform::change::{
    CanonicalBaseRead, CanonicalDelta, CanonicalReadWork, PreparedAuthority,
    PreparedChangeAnalysis, WitnessBaseRead, stage_full_authority, stage_prepared_authority,
    summary_dimension_change,
};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    KernelSnapshot, OwnerKey, OwnerObjectDigest, OwnerRecord, encode_owner, encode_root,
};
use crate::platform::persistent_map::{MapRoot, MapWork};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StoreError, StoreErrorClass,
    StoreWork,
};
use crate::platform::storage::page_store::ObjectPageStore;
use crate::platform::witness::{FullWitness, encode_witness_manifest};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PublicationOptions {
    pub idempotency_key: Option<String>,
    pub intent: Option<String>,
}

#[derive(Clone, Debug)]
pub struct PreparedPublication {
    pub expected_base: Option<HeadRecord>,
    pub authority: PreparedAuthority,
    pub transaction: NormalizedTransaction,
    pub transaction_digest: super::TransactionDigest,
    pub transaction_bytes: Vec<u8>,
    pub semantic_diff: SemanticDiff,
    pub semantic_diff_digest: super::SemanticDiffDigest,
    pub semantic_diff_bytes: Vec<u8>,
    pub receipt: PublicationReceipt,
    pub receipt_digest: super::ReceiptObjectDigest,
    pub receipt_bytes: Vec<u8>,
    pub revision: RevisionRecord,
    pub revision_digest: super::RevisionObjectDigest,
    pub revision_bytes: Vec<u8>,
    pub head: HeadRecord,
    pub head_bytes: Vec<u8>,
    pub accepted: AcceptedBinding,
    pub compiler_units: BTreeSet<OwnerKey>,
    pub objects: BTreeMap<ObjectKey, Vec<u8>>,
    pub store_work: StoreWork,
    pub budget_work: crate::platform::change::ChangeBudgetWork,
}

#[derive(Clone, Debug)]
pub struct PreparedInitialPublication {
    pub publication: PreparedPublication,
    pub snapshot: KernelSnapshot,
    pub witness: FullWitness,
}

pub fn prepare_initial_publication<S: ImmutableObjectStore + ?Sized>(
    logical: &KernelSnapshot,
    store: &S,
    intent: Option<String>,
) -> Result<PreparedInitialPublication, Vec<Diagnostic>> {
    let mut stage = ObjectStage::new(store);
    let full = stage_full_authority(logical, &mut stage)?;
    let repository_id = full.snapshot.root.repository_id;
    let root = full.binding.semantic.digest;
    let transaction = NormalizedTransaction {
        contract_version: TRANSACTION_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        repository_id,
        body: TransactionBody::Bootstrap { result_root: root },
    };
    let semantic_diff = SemanticDiff {
        contract_version: SEMANTIC_DIFF_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        repository_id,
        body: SemanticDiffBody::Bootstrap {
            result_root: root,
            owners: full.snapshot.owners.len() as u64,
            dependencies: full.snapshot.dependencies.len() as u64,
            retirements: full.snapshot.retirements.len() as u64,
        },
    };
    let counts = ChangeCounts {
        owners_created: full.snapshot.owners.len() as u64,
        type_objects_added: full.snapshot.types.len() as u64,
        dependencies_changed: full.snapshot.dependencies.len() as u64,
        retirements_changed: full.snapshot.retirements.len() as u64,
        witness_entries_changed: witness_entry_count(&full.witness),
        ..ChangeCounts::default()
    };
    let validation = ValidationEvidence {
        profile: ValidationProfile::FullRebuild,
        structurally_checked: full.witness.report.full_validation.owners_checked,
        semantically_checked: full.witness.report.full_validation.owners_checked,
        summaries_reused: 0,
        reverse_edges_visited: 0,
        tests_selected: 0,
        tests_executed: 0,
        tests_passed: 0,
        compiler_units_planned: 0,
        full_oracle: FullOracleStatus::NotApplicable,
    };
    let work = full_work(&full);
    let bound = bind_history(
        None,
        PublicationStatus::ProjectCreated,
        full.binding,
        transaction,
        semantic_diff,
        counts,
        validation,
        work,
        PublicationOptions {
            intent,
            ..PublicationOptions::default()
        },
        &mut stage,
    )?;
    let publication = finish_prepared(bound, stage);
    Ok(PreparedInitialPublication {
        publication,
        snapshot: full.snapshot,
        witness: full.witness,
    })
}

pub fn prepare_change_publication<
    B: CanonicalBaseRead + ?Sized,
    W: WitnessBaseRead + ?Sized,
    S: ImmutableObjectStore + ?Sized,
>(
    base: AcceptedBinding,
    base_snapshot: &B,
    base_witness: &W,
    analysis: &PreparedChangeAnalysis,
    store: &S,
    options: PublicationOptions,
) -> Result<PreparedPublication, Vec<Diagnostic>> {
    validate_base(base, base_snapshot, base_witness)?;
    if analysis.canonical.is_empty() {
        return Err(vec![publication_error(
            DiagnosticClass::Semantic,
            "publication_semantic_no_change",
            "empty normalized change publishes no revision",
        )]);
    }
    let mut stage = ObjectStage::new(store);
    let mut authority =
        stage_prepared_authority(base_snapshot, base_witness, analysis, &mut stage)?;
    if authority.semantic.digest == base.semantic_root {
        return Err(vec![publication_error(
            DiagnosticClass::Semantic,
            "publication_semantic_no_change",
            "candidate semantic root equals the exact base and publishes no revision",
        )]);
    }
    let transaction = transaction_for_change(base, &authority, &analysis.canonical);
    let (semantic_diff, diff_read_work) =
        diff_for_change(base, base_snapshot, &authority, analysis)?;
    authority.semantic.canonical_read_work.add(diff_read_work);
    let counts = change_counts(analysis);
    let validation = change_validation(analysis);
    let work = change_work(&authority, analysis);
    let bound = bind_history(
        Some(base),
        PublicationStatus::AcceptedChange,
        authority,
        transaction,
        semantic_diff,
        counts,
        validation,
        work,
        options,
        &mut stage,
    )?;
    let mut prepared = finish_prepared(bound, stage);
    prepared.compiler_units = analysis.summaries.plan.compiler_units.clone();
    prepared.budget_work = analysis.budget_work;
    Ok(prepared)
}

struct BoundHistory {
    expected_base: Option<HeadRecord>,
    authority: PreparedAuthority,
    transaction: NormalizedTransaction,
    transaction_digest: super::TransactionDigest,
    transaction_bytes: Vec<u8>,
    semantic_diff: SemanticDiff,
    semantic_diff_digest: super::SemanticDiffDigest,
    semantic_diff_bytes: Vec<u8>,
    receipt: PublicationReceipt,
    receipt_digest: super::ReceiptObjectDigest,
    receipt_bytes: Vec<u8>,
    revision: RevisionRecord,
    revision_digest: super::RevisionObjectDigest,
    revision_bytes: Vec<u8>,
    head: HeadRecord,
    head_bytes: Vec<u8>,
    accepted: AcceptedBinding,
    store_work: StoreWork,
}

#[allow(
    clippy::too_many_arguments,
    reason = "one exact history binding has these closed inputs"
)]
fn bind_history<S: ImmutableObjectStore + ?Sized>(
    base: Option<AcceptedBinding>,
    status: PublicationStatus,
    authority: PreparedAuthority,
    transaction: NormalizedTransaction,
    semantic_diff: SemanticDiff,
    counts: ChangeCounts,
    validation: ValidationEvidence,
    mut work: WorkObservation,
    options: PublicationOptions,
    stage: &mut ObjectStage<'_, S>,
) -> Result<BoundHistory, Vec<Diagnostic>> {
    let repository_id = authority.semantic.root.repository_id;
    if transaction.repository_id != repository_id
        || semantic_diff.repository_id != repository_id
        || transaction.result_root() != authority.semantic.digest
        || semantic_diff.result_root() != authority.semantic.digest
    {
        return Err(vec![publication_error(
            DiagnosticClass::Corrupt,
            "publication_history_authority",
            "transaction or semantic diff does not bind the candidate semantic authority",
        )]);
    }
    validate_history_base(base, status, &transaction, &semantic_diff).map_err(single)?;
    let (transaction_digest, transaction_bytes) = transaction.encode().map_err(single)?;
    let (semantic_diff_digest, semantic_diff_bytes) = semantic_diff.encode().map_err(single)?;
    let authority_store_work = authority.store_work;
    let mut store_work = authority_store_work;
    stage_object(
        stage,
        ObjectDomain::Transaction,
        transaction_digest.bytes(),
        &transaction_bytes,
        &mut store_work,
    )?;
    stage_object(
        stage,
        ObjectDomain::SemanticDiff,
        semantic_diff_digest.bytes(),
        &semantic_diff_bytes,
        &mut store_work,
    )?;

    let (idempotency_receipts, idempotency_map_work) =
        prepare_idempotency_history(base, stage, &mut store_work)?;
    work.map_pages_read = work
        .map_pages_read
        .saturating_add(idempotency_map_work.pages_read);
    work.map_pages_written = work
        .map_pages_written
        .saturating_add(idempotency_map_work.pages_written);

    let parents = base.into_iter().map(AcceptedBinding::parent).collect();
    let core = RevisionCore {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        witness_contract_version: crate::platform::witness::contract::WITNESS_CONTRACT_VERSION,
        repository_id,
        parents,
        semantic_root: authority.semantic.digest,
        validation_witness: authority.witness.digest,
        validation_certificate: authority.witness.manifest.certificate,
        validator_contract: authority.witness.manifest.validator_contract,
        idempotency_receipts,
        transaction: transaction_digest,
        semantic_diff: semantic_diff_digest,
    };
    let revision_id = core.revision_id().map_err(single)?;
    work.objects_staged = store_work.objects_staged;
    work.objects_read = work.objects_read.saturating_add(
        store_work
            .objects_read
            .saturating_sub(authority_store_work.objects_read),
    );
    work.bytes_staged = store_work.bytes_staged;
    work.bytes_read = work.bytes_read.saturating_add(
        store_work
            .bytes_read
            .saturating_sub(authority_store_work.bytes_read),
    );
    let bases = base
        .into_iter()
        .map(|binding| binding.head.revision)
        .collect();
    let receipt = PublicationReceipt {
        contract_version: RECEIPT_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        repository_id,
        status,
        bases,
        result: revision_id,
        transaction: transaction_digest,
        semantic_diff: semantic_diff_digest,
        idempotency_key: options.idempotency_key,
        counts,
        validation,
        work,
        intent: options.intent,
    };
    let (receipt_digest, receipt_bytes) = receipt.encode().map_err(single)?;
    let revision = RevisionRecord::new(core, receipt_digest).map_err(single)?;
    let (revision_digest, revision_bytes) = revision.encode().map_err(single)?;
    stage_object(
        stage,
        ObjectDomain::Receipt,
        receipt_digest.bytes(),
        &receipt_bytes,
        &mut store_work,
    )?;
    stage_object(
        stage,
        ObjectDomain::Revision,
        revision_digest.bytes(),
        &revision_bytes,
        &mut store_work,
    )?;
    let head = HeadRecord {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        repository_id,
        revision: revision.revision,
        record: revision_digest,
    };
    let head_bytes = head.encode().map_err(single)?;
    let accepted = AcceptedBinding::verify(
        head,
        &revision,
        authority.witness.digest,
        &authority.witness.manifest,
    )
    .map_err(single)?;
    Ok(BoundHistory {
        expected_base: base.map(|binding| binding.head),
        authority,
        transaction,
        transaction_digest,
        transaction_bytes,
        semantic_diff,
        semantic_diff_digest,
        semantic_diff_bytes,
        receipt,
        receipt_digest,
        receipt_bytes,
        revision,
        revision_digest,
        revision_bytes,
        head,
        head_bytes,
        accepted,
        store_work,
    })
}

fn prepare_idempotency_history<S: ImmutableObjectStore + ?Sized>(
    base: Option<AcceptedBinding>,
    stage: &mut ObjectStage<'_, S>,
    store_work: &mut StoreWork,
) -> Result<(MapRoot, MapWork), Vec<Diagnostic>> {
    let binding = match base {
        Some(base) => {
            let bytes = stage
                .read(
                    ObjectKey::from_digest(ObjectDomain::Receipt, base.receipt.bytes()),
                    ObjectDomain::Receipt.maximum_bytes(),
                    store_work,
                )
                .map_err(|error| vec![store_diagnostic(error)])?
                .ok_or_else(|| {
                    vec![publication_error(
                        DiagnosticClass::Corrupt,
                        "publication_idempotency_base_receipt_missing",
                        "accepted base references a missing receipt required for idempotency history",
                    )]
                })?;
            let receipt = PublicationReceipt::decode(&bytes, base.receipt).map_err(single)?;
            IdempotencyBinding::from_accepted(base, &receipt).map_err(single)?
        }
        None => None,
    };
    let mut map_work = MapWork::default();
    let mut page_store = ObjectPageStore::new(&mut *stage);
    let root = match base {
        Some(base) => advance_idempotency_history(
            base.idempotency_receipts,
            binding.as_ref(),
            &mut page_store,
            &mut map_work,
        ),
        None => empty_idempotency_history(&mut page_store, &mut map_work),
    }
    .map_err(single)?;
    store_work.add(page_store.work());
    Ok((root, map_work))
}

pub(super) fn validate_history_base(
    base: Option<AcceptedBinding>,
    status: PublicationStatus,
    transaction: &NormalizedTransaction,
    semantic_diff: &SemanticDiff,
) -> Result<(), Diagnostic> {
    match (base, status, &transaction.body, &semantic_diff.body) {
        (
            None,
            PublicationStatus::ProjectCreated,
            TransactionBody::Bootstrap {
                result_root: transaction_root,
            },
            SemanticDiffBody::Bootstrap {
                result_root: diff_root,
                ..
            },
        ) if transaction_root == diff_root => Ok(()),
        (
            Some(base),
            PublicationStatus::AcceptedChange,
            TransactionBody::Change {
                base: transaction_base,
                base_root: transaction_base_root,
                owners: transaction_owners,
                type_additions: transaction_types,
                dependencies: transaction_dependencies,
                retirements: transaction_retirements,
                ..
            },
            SemanticDiffBody::Change {
                base: diff_base,
                base_root: diff_base_root,
                owners: diff_owners,
                type_additions: diff_types,
                dependencies: diff_dependencies,
                retirements: diff_retirements,
                ..
            },
        ) if *transaction_base == base.head.revision
            && *diff_base == base.head.revision
            && *transaction_base_root == base.semantic_root
            && *diff_base_root == base.semantic_root
            && transaction_types == diff_types
            && transaction_dependencies.len() == diff_dependencies.len()
            && transaction_retirements.len() == diff_retirements.len()
            && transaction_dependencies.iter().zip(diff_dependencies).all(
                |(transaction, diff)| {
                    transaction.package == diff.package && transaction.objects == diff.objects
                },
            )
            && transaction_retirements.iter().zip(diff_retirements).all(
                |(transaction, diff)| {
                    transaction.owner == diff.owner && transaction.objects == diff.objects
                },
            )
            && transaction_owners.iter().all(|transaction| {
                diff_owners
                    .binary_search_by_key(&transaction.owner, |diff| diff.owner)
                    .ok()
                    .and_then(|index| diff_owners.get(index))
                    .is_some_and(|diff| diff.objects == transaction.objects)
            }) =>
        {
            Ok(())
        }
        _ => Err(publication_error(
            DiagnosticClass::Corrupt,
            "publication_history_base",
            "transaction, semantic diff, status, and expected base do not form one normalized publication",
        )),
    }
}

fn finish_prepared<S: ImmutableObjectStore + ?Sized>(
    bound: BoundHistory,
    stage: ObjectStage<'_, S>,
) -> PreparedPublication {
    PreparedPublication {
        expected_base: bound.expected_base,
        authority: bound.authority,
        transaction: bound.transaction,
        transaction_digest: bound.transaction_digest,
        transaction_bytes: bound.transaction_bytes,
        semantic_diff: bound.semantic_diff,
        semantic_diff_digest: bound.semantic_diff_digest,
        semantic_diff_bytes: bound.semantic_diff_bytes,
        receipt: bound.receipt,
        receipt_digest: bound.receipt_digest,
        receipt_bytes: bound.receipt_bytes,
        revision: bound.revision,
        revision_digest: bound.revision_digest,
        revision_bytes: bound.revision_bytes,
        head: bound.head,
        head_bytes: bound.head_bytes,
        accepted: bound.accepted,
        compiler_units: BTreeSet::new(),
        objects: stage.into_objects(),
        store_work: bound.store_work,
        budget_work: crate::platform::change::ChangeBudgetWork::default(),
    }
}

fn validate_base<B: CanonicalBaseRead + ?Sized, W: WitnessBaseRead + ?Sized>(
    accepted: AcceptedBinding,
    snapshot: &B,
    witness: &W,
) -> Result<(), Vec<Diagnostic>> {
    let root = snapshot.semantic_root();
    let manifest = witness.witness_manifest();
    let (semantic_root, _) = encode_root(root).map_err(single)?;
    let (witness_digest, _) = encode_witness_manifest(manifest).map_err(single)?;
    if accepted.head.repository_id != root.repository_id
        || accepted.semantic_root != semantic_root
        || snapshot
            .exact_revision()
            .is_some_and(|revision| revision != accepted.head.revision)
        || accepted.validation_witness != witness_digest
        || accepted.validation_certificate != manifest.certificate
        || accepted.validator_contract != manifest.validator_contract
    {
        return Err(vec![publication_error(
            DiagnosticClass::Corrupt,
            "publication_prepare_base",
            "prepared change inputs do not share one exact accepted base",
        )]);
    }
    Ok(())
}

fn transaction_for_change(
    base: AcceptedBinding,
    authority: &PreparedAuthority,
    delta: &CanonicalDelta,
) -> NormalizedTransaction {
    NormalizedTransaction {
        contract_version: TRANSACTION_CONTRACT_VERSION,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        repository_id: authority.semantic.root.repository_id,
        body: TransactionBody::Change {
            base: base.head.revision,
            base_root: base.semantic_root,
            result_root: authority.semantic.digest,
            owners: delta
                .owners
                .iter()
                .map(|(owner, edit)| OwnerTransactionEdit {
                    owner: *owner,
                    objects: DigestEdit {
                        before: edit.before,
                        after: edit.after.as_ref().map(|(digest, _)| *digest),
                    },
                })
                .collect(),
            type_additions: delta.type_additions.keys().copied().collect(),
            dependencies: delta
                .dependencies
                .iter()
                .map(|(package, edit)| DependencyTransactionEdit {
                    package: *package,
                    objects: DigestEdit {
                        before: edit.before,
                        after: edit.after.as_ref().map(|(digest, _)| *digest),
                    },
                })
                .collect(),
            retirements: delta
                .retirements
                .iter()
                .map(|(owner, edit)| RetirementTransactionEdit {
                    owner: *owner,
                    objects: DigestEdit {
                        before: edit.before,
                        after: edit.after.as_ref().map(|(digest, _)| *digest),
                    },
                })
                .collect(),
        },
    }
}

fn diff_for_change<B: CanonicalBaseRead + ?Sized>(
    base: AcceptedBinding,
    base_snapshot: &B,
    authority: &PreparedAuthority,
    analysis: &PreparedChangeAnalysis,
) -> Result<(SemanticDiff, CanonicalReadWork), Vec<Diagnostic>> {
    let mut owners = BTreeMap::<OwnerKey, OwnerDiffEntry>::new();
    let mut read_work = CanonicalReadWork::default();
    for summary in &analysis.summaries.final_delta.edits {
        let dimensions = dimensions(summary);
        let objects = DigestEdit {
            before: summary.before.as_ref().map(|value| value.record),
            after: summary.after.as_ref().map(|value| value.record),
        };
        let classes = lifecycle_and_dimension_classes(objects, dimensions);
        owners.insert(
            summary.owner,
            OwnerDiffEntry {
                owner: summary.owner,
                objects,
                classes,
                dimensions,
            },
        );
    }
    for (owner, edit) in &analysis.canonical.owners {
        let before_record = if edit.before.is_some() && edit.after.is_some() {
            let read = base_snapshot.read_owner(*owner).map_err(single)?;
            read_work.add(read.work);
            read.value
        } else {
            None
        };
        let after_record = edit.after.as_ref().map(|(_, record)| record);
        let entry = owners.entry(*owner).or_insert_with(|| OwnerDiffEntry {
            owner: *owner,
            objects: DigestEdit {
                before: edit.before,
                after: edit.after.as_ref().map(|(digest, _)| *digest),
            },
            classes: lifecycle_and_dimension_classes(
                DigestEdit {
                    before: edit.before,
                    after: edit.after.as_ref().map(|(digest, _)| *digest),
                },
                SummaryDimensions::default(),
            ),
            dimensions: SummaryDimensions::default(),
        });
        entry.objects = DigestEdit {
            before: edit.before,
            after: edit.after.as_ref().map(|(digest, _)| *digest),
        };
        let mut classes = entry.classes.iter().copied().collect::<BTreeSet<_>>();
        if let (Some(before_record), Some(after_record)) = (before_record.as_ref(), after_record) {
            if before_record.name() != after_record.name() {
                classes.insert(OwnerChangeClass::Renamed);
            }
            if declaration_visibility(Some(before_record))
                != declaration_visibility(Some(after_record))
            {
                classes.insert(OwnerChangeClass::VisibilityChanged);
            }
        }
        entry.classes = classes.into_iter().collect();
    }
    for edit in &analysis.derived.ownership {
        if edit.before == edit.after || edit.before.is_none() || edit.after.is_none() {
            continue;
        }
        let entry = match owners.entry(edit.key) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let object =
                    base_object_digest(edit.key, base_snapshot, &mut read_work).map_err(single)?;
                entry.insert(OwnerDiffEntry {
                    owner: edit.key,
                    objects: DigestEdit {
                        before: Some(object),
                        after: Some(object),
                    },
                    classes: Vec::new(),
                    dimensions: SummaryDimensions::default(),
                })
            }
        };
        let mut classes = entry.classes.iter().copied().collect::<BTreeSet<_>>();
        classes.insert(OwnerChangeClass::Moved);
        entry.classes = classes.into_iter().collect();
    }
    for entry in owners.values_mut() {
        if entry.classes.is_empty() {
            entry.classes.push(OwnerChangeClass::SemanticPayloadChanged);
        }
        entry.classes.sort_unstable();
        entry.classes.dedup();
    }
    Ok((
        SemanticDiff {
            contract_version: SEMANTIC_DIFF_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: authority.semantic.root.repository_id,
            body: SemanticDiffBody::Change {
                base: base.head.revision,
                base_root: base.semantic_root,
                result_root: authority.semantic.digest,
                owners: owners.into_values().collect(),
                type_additions: analysis.canonical.type_additions.keys().copied().collect(),
                dependencies: analysis
                    .canonical
                    .dependencies
                    .iter()
                    .map(|(package, edit)| DependencyDiffEntry {
                        package: *package,
                        objects: DigestEdit {
                            before: edit.before,
                            after: edit.after.as_ref().map(|(digest, _)| *digest),
                        },
                    })
                    .collect(),
                retirements: analysis
                    .canonical
                    .retirements
                    .iter()
                    .map(|(owner, edit)| RetirementDiffEntry {
                        owner: *owner,
                        objects: DigestEdit {
                            before: edit.before,
                            after: edit.after.as_ref().map(|(digest, _)| *digest),
                        },
                    })
                    .collect(),
            },
        },
        read_work,
    ))
}

fn lifecycle_and_dimension_classes(
    objects: DigestEdit<OwnerObjectDigest>,
    dimensions: SummaryDimensions,
) -> Vec<OwnerChangeClass> {
    let mut classes = BTreeSet::new();
    if objects.before.is_none() {
        classes.insert(OwnerChangeClass::Created);
    }
    if objects.after.is_none() {
        classes.insert(OwnerChangeClass::Deleted);
    }
    if dimensions.semantic_interface {
        classes.insert(OwnerChangeClass::PublicInterface);
    }
    if dimensions.implementation && !dimensions.semantic_interface {
        classes.insert(OwnerChangeClass::PrivateImplementation);
    }
    if dimensions.effect || dimensions.capability {
        classes.insert(OwnerChangeClass::EffectOrCapability);
    }
    if dimensions.relations || dimensions.validation_dependencies {
        classes.insert(OwnerChangeClass::RelationSet);
    }
    if dimensions.test
        && !dimensions.semantic_interface
        && !dimensions.implementation
        && !dimensions.type_digest
        && !dimensions.effect
        && !dimensions.capability
    {
        classes.insert(OwnerChangeClass::TestOnly);
    }
    if dimensions.presentation && !dimensions.executable() {
        classes.insert(OwnerChangeClass::PresentationOnly);
    }
    if dimensions.executable() {
        classes.insert(OwnerChangeClass::SemanticPayloadChanged);
    }
    classes.into_iter().collect()
}

fn dimensions(edit: &crate::platform::change::OwnerSummaryEdit) -> SummaryDimensions {
    let change = summary_dimension_change(edit);
    SummaryDimensions {
        semantic_interface: change.semantic_interface,
        implementation: change.implementation,
        type_digest: change.type_digest,
        effect: change.effect,
        capability: change.capability,
        relations: change.relations,
        presentation: change.presentation,
        test: change.test,
        validation_dependencies: change.validation_dependencies,
    }
}

fn declaration_visibility(
    record: Option<&OwnerRecord>,
) -> Option<crate::platform::kernel::DeclarationVisibility> {
    match record? {
        OwnerRecord::Declaration(value) => Some(value.visibility),
        _ => None,
    }
}

fn base_object_digest<B: CanonicalBaseRead + ?Sized>(
    owner: OwnerKey,
    snapshot: &B,
    work: &mut CanonicalReadWork,
) -> Result<OwnerObjectDigest, Diagnostic> {
    let read = snapshot.read_owner(owner)?;
    work.add(read.work);
    let record = read.value.ok_or_else(|| {
        publication_error(
            DiagnosticClass::Corrupt,
            "publication_diff_owner_digest",
            "ownership-only diff names an owner absent from the exact base",
        )
    })?;
    encode_owner(&record).map(|(digest, _)| digest)
}

fn change_counts(analysis: &PreparedChangeAnalysis) -> ChangeCounts {
    let mut counts = ChangeCounts::default();
    for edit in analysis.canonical.owners.values() {
        match (&edit.before, &edit.after) {
            (None, Some(_)) => counts.owners_created = counts.owners_created.saturating_add(1),
            (Some(_), Some(_)) => counts.owners_updated = counts.owners_updated.saturating_add(1),
            (Some(_), None) => counts.owners_deleted = counts.owners_deleted.saturating_add(1),
            (None, None) => {}
        }
    }
    counts.type_objects_added = analysis.canonical.type_additions.len() as u64;
    counts.dependencies_changed = analysis.canonical.dependencies.len() as u64;
    counts.retirements_changed = analysis.canonical.retirements.len() as u64;
    counts.witness_entries_changed = analysis
        .witness
        .edits
        .inserted
        .saturating_add(analysis.witness.edits.replaced)
        .saturating_add(analysis.witness.edits.removed);
    counts
}

fn change_validation(analysis: &PreparedChangeAnalysis) -> ValidationEvidence {
    ValidationEvidence {
        profile: ValidationProfile::IncrementalOwnerFrontier,
        structurally_checked: analysis.validation.structurally_checked.len() as u64,
        semantically_checked: analysis.validation.semantically_checked.len() as u64,
        summaries_reused: analysis.validation.summaries_reused,
        reverse_edges_visited: analysis.summaries.plan.work.reverse_edges_visited,
        tests_selected: analysis.validation.tests_selected,
        tests_executed: 0,
        tests_passed: 0,
        compiler_units_planned: analysis.summaries.plan.compiler_units.len() as u64,
        full_oracle: FullOracleStatus::NotRun,
    }
}

fn change_work(
    authority: &PreparedAuthority,
    analysis: &PreparedChangeAnalysis,
) -> WorkObservation {
    let validation_work = analysis
        .validation
        .work
        .owner_records_checked
        .saturating_add(analysis.validation.work.ownership_entries_checked)
        .saturating_add(analysis.validation.work.type_objects_checked)
        .saturating_add(analysis.validation.work.expression_work);
    let mut witness_reads = analysis.derived.read_work;
    witness_reads.add(analysis.witness_read_work);
    witness_reads.add(analysis.summaries.initial.read_work);
    witness_reads.add(analysis.summaries.final_delta.read_work);
    witness_reads.add(analysis.summaries.plan.work.witness_reads);
    witness_reads.add(analysis.tests.work.witness_reads);
    witness_reads.add(analysis.validation.work.witness_reads);
    WorkObservation {
        validation_work,
        map_pages_read: authority
            .semantic
            .map_work
            .pages_read
            .saturating_add(analysis.witness.work.pages_read)
            .saturating_add(witness_reads.map_pages_read)
            .saturating_add(analysis.canonical_read_work.map_pages_read)
            .saturating_add(authority.semantic.canonical_read_work.map_pages_read),
        map_pages_written: authority
            .semantic
            .map_work
            .pages_written
            .saturating_add(analysis.witness.work.pages_written),
        owner_records_checked: analysis.validation.work.owner_records_checked,
        ownership_entries_checked: analysis.validation.work.ownership_entries_checked,
        type_objects_checked: analysis.validation.work.type_objects_checked,
        expression_work: analysis.validation.work.expression_work,
        relation_edges_visited: analysis
            .summaries
            .plan
            .work
            .reverse_edges_visited
            .saturating_add(analysis.tests.work.relation_edges_visited),
        objects_read: authority
            .store_work
            .objects_read
            .saturating_add(witness_reads.objects_read)
            .saturating_add(analysis.canonical_read_work.objects_read)
            .saturating_add(authority.semantic.canonical_read_work.objects_read),
        objects_staged: authority.store_work.objects_staged,
        bytes_read: authority
            .store_work
            .bytes_read
            .saturating_add(witness_reads.bytes_read)
            .saturating_add(analysis.canonical_read_work.bytes_read)
            .saturating_add(authority.semantic.canonical_read_work.bytes_read),
        bytes_staged: authority.store_work.bytes_staged,
    }
}

fn full_work(full: &crate::platform::change::FullStagedAuthority) -> WorkObservation {
    WorkObservation {
        validation_work: full.witness.report.full_validation.work_consumed,
        map_pages_read: full.binding.semantic.map_work.pages_read,
        map_pages_written: full
            .binding
            .semantic
            .map_work
            .pages_written
            .saturating_add(full.witness.report.map_work.pages_written),
        owner_records_checked: full.witness.report.full_validation.owners_checked,
        type_objects_checked: full.witness.report.full_validation.type_objects_checked,
        relation_edges_visited: full.witness.report.relation_edges,
        ..WorkObservation::default()
    }
}

fn witness_entry_count(witness: &FullWitness) -> u64 {
    let roots = witness.manifest.roots;
    roots
        .owner_summaries
        .entries()
        .saturating_add(roots.namespaces.entries())
        .saturating_add(roots.ownership.entries())
        .saturating_add(roots.forward_relations.entries())
        .saturating_add(roots.reverse_relations.entries())
        .saturating_add(roots.test_dependencies.entries())
}

fn stage_object<S: ImmutableObjectStore + ?Sized>(
    stage: &mut ObjectStage<'_, S>,
    domain: ObjectDomain,
    digest: [u8; 32],
    bytes: &[u8],
    work: &mut StoreWork,
) -> Result<(), Vec<Diagnostic>> {
    stage
        .stage(ObjectKey::from_digest(domain, digest), bytes, work)
        .map(|_| ())
        .map_err(|error| vec![store_diagnostic(error)])
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    Diagnostic::new(class, error.code, error.message)
}

fn single(error: Diagnostic) -> Vec<Diagnostic> {
    vec![error]
}

fn publication_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
