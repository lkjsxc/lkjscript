//! Exact Graph 5 repository reads and one locked, atomic accepted-HEAD publication point.

use super::contract::{HEAD_MAGIC, MAXIMUM_HEAD_BYTES, REVISION_CONTRACT_VERSION};
use super::prepare::validate_history_base;
use super::read_view::RepositoryView;
use super::{
    AcceptedBinding, HeadRecord, NormalizedTransaction, PreparedInitialPublication,
    PreparedPublication, PublicationOptions, PublicationReceipt, RevisionRecord, SemanticDiff,
    prepare_initial_publication,
};
use crate::platform::change::PrimitiveEdit;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{KernelSnapshot, SemanticRoot, decode_root, encode_root};
use crate::platform::storage::contract::TARGET_PACK_BYTES;
use crate::platform::storage::directory::{PackDirectoryStore, SealReceipt};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StoreError, StoreErrorClass, StoreWork,
};
use crate::platform::witness::{
    ValidationWitnessManifest, decode_witness_manifest, encode_witness_manifest,
};
use fs2::FileExt;
use rustix::fs::{AtFlags, Mode, OFlags};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const HEAD_FILE: &str = "HEAD";
const LOCK_FILE: &str = "LOCK";
const HEAD_STAGE_PREFIX: &str = ".HEAD-stage-";
const REPOSITORY_STAGE_PREFIX: &str = ".lkjscript-graph5-stage-";

#[derive(Clone, Debug)]
pub struct GraphRepository {
    root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct CurrentPublication {
    pub head: HeadRecord,
    pub revision: RevisionRecord,
    pub receipt: PublicationReceipt,
    pub transaction: NormalizedTransaction,
    pub semantic_diff: SemanticDiff,
    pub semantic_root: SemanticRoot,
    pub witness: ValidationWitnessManifest,
    pub accepted: AcceptedBinding,
    pub store_work: StoreWork,
}

#[derive(Clone, Debug)]
pub struct CreatedRepository {
    pub repository: GraphRepository,
    pub initial: PreparedInitialPublication,
    pub current: CurrentPublication,
}

#[derive(Clone, Debug)]
pub enum PublicationOutcome {
    Accepted {
        current: CurrentPublication,
        seal: SealReceipt,
        store_work: StoreWork,
    },
    AlreadyAccepted {
        current: CurrentPublication,
    },
    Stale {
        expected: Option<HeadRecord>,
        current: Option<HeadRecord>,
    },
}

#[derive(Clone, Debug)]
pub enum ReconciliationStatus {
    NotStarted {
        current: Option<HeadRecord>,
    },
    Accepted {
        current: Box<CurrentPublication>,
    },
    Stale {
        expected: Option<HeadRecord>,
        current: Option<HeadRecord>,
    },
    ConflictingIdempotency {
        current: HeadRecord,
    },
}

#[derive(Clone, Debug)]
enum PublicationDecision {
    Ready,
    Accepted(Box<CurrentPublication>),
    Stale {
        expected: Option<HeadRecord>,
        current: Option<HeadRecord>,
    },
    ConflictingIdempotency(HeadRecord),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationPoint {
    BeforeObjectStage,
    AfterFirstObjectStage,
    AfterPacksSealed,
    AfterHeadFileSynced,
    AfterHeadRenamed,
    AfterHeadDirectorySynced,
}

impl GraphRepository {
    pub fn create(
        destination: &Path,
        logical: &KernelSnapshot,
        intent: Option<String>,
    ) -> Result<CreatedRepository, Diagnostic> {
        let destination = canonical_new_destination(destination)?;
        let parent = destination.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_parent",
                "Graph 5 repository destination has no existing parent",
            )
        })?;
        let stage = parent.join(format!(
            "{REPOSITORY_STAGE_PREFIX}{}",
            random_hex("publication_repository_stage_random")?
        ));
        initialize_unpublished(&stage)?;
        let staged_repository = Self {
            root: stage.clone(),
        };
        let result = (|| {
            let preparation_store = PackDirectoryStore::open(&stage).map_err(store_diagnostic)?;
            let initial = prepare_initial_publication(logical, &preparation_store, intent)
                .map_err(collapse_diagnostics)?;
            drop(preparation_store);
            let outcome = staged_repository.publish(&initial.publication)?;
            let PublicationOutcome::Accepted { current, .. } = outcome else {
                return Err(repository_error(
                    DiagnosticClass::Infrastructure,
                    "publication_repository_initial_outcome",
                    "new private repository did not accept its only initial publication",
                ));
            };
            Ok((initial, current))
        })();
        let (initial, staged_current) = match result {
            Ok(value) => value,
            Err(error) => {
                remove_owned_repository_stage(&stage);
                return Err(error);
            }
        };
        if let Err(error) = fs::rename(&stage, &destination) {
            remove_owned_repository_stage(&stage);
            return Err(io_diagnostic(
                "publication_repository_initial_rename",
                &destination,
                error,
            ));
        }
        sync_directory(parent).map_err(|error| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "publication_repository_initial_visibility",
                format!(
                    "initial repository rename succeeded but parent durability is indeterminate: {error}"
                ),
            )
        })?;
        let repository = Self { root: destination };
        let current = repository.current()?;
        if current.head != staged_current.head {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "publication_repository_initial_reconcile",
                "visible initial repository disagrees with the privately accepted HEAD",
            ));
        }
        Ok(CreatedRepository {
            repository,
            initial,
            current,
        })
    }

    pub fn open(root: &Path) -> Result<Self, Diagnostic> {
        let metadata = fs::symlink_metadata(root)
            .map_err(|error| io_diagnostic("publication_repository_open_metadata", root, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_repository_open_type",
                "Graph 5 repository root is not a regular non-symlink directory",
            ));
        }
        let root = root
            .canonicalize()
            .map_err(|error| io_diagnostic("publication_repository_open_canonical", root, error))?;
        let repository = Self { root };
        let _ = repository.current()?;
        Ok(repository)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn object_store(&self) -> Result<PackDirectoryStore, Diagnostic> {
        PackDirectoryStore::open(&self.root).map_err(store_diagnostic)
    }

    pub fn current(&self) -> Result<CurrentPublication, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic("publication_repository_read_lock", &self.root, error)
        })?;
        let store = PackDirectoryStore::open(&self.root).map_err(store_diagnostic)?;
        read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 5 repository has no accepted HEAD",
            )
        })
    }

    /// Opens one exact revision-pinned read view. The shared lock protects the HEAD/catalog
    /// observation; immutable append-only packs keep that observed revision readable after the
    /// lock is released and later publications advance HEAD.
    pub fn view_current(&self) -> Result<RepositoryView, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic("publication_repository_view_lock", &self.root, error)
        })?;
        let store = PackDirectoryStore::open(&self.root).map_err(store_diagnostic)?;
        let current = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 5 repository has no accepted HEAD",
            )
        })?;
        Ok(RepositoryView::new(current, store))
    }

    /// Prepares one exact change against the currently observed revision without publishing it.
    pub fn prepare_change(
        &self,
        edits: Vec<PrimitiveEdit>,
        options: PublicationOptions,
    ) -> Result<PreparedPublication, Vec<Diagnostic>> {
        self.view_current()
            .map_err(|diagnostic| vec![diagnostic])?
            .prepare_change(edits, options)
    }

    pub fn publish(
        &self,
        prepared: &PreparedPublication,
    ) -> Result<PublicationOutcome, Diagnostic> {
        self.publish_at(prepared, None)
    }

    pub fn reconcile(
        &self,
        prepared: &PreparedPublication,
    ) -> Result<ReconciliationStatus, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic("publication_repository_reconcile_lock", &self.root, error)
        })?;
        let store = PackDirectoryStore::open(&self.root).map_err(store_diagnostic)?;
        let current = read_current_optional(&root_directory, &store)?;
        validate_prepared_repository(prepared)?;
        Ok(match classify_publication(current.as_ref(), prepared) {
            PublicationDecision::Ready => {
                validate_history_base(
                    current.as_ref().map(|value| value.accepted),
                    prepared.receipt.status,
                    &prepared.transaction,
                    &prepared.semantic_diff,
                )?;
                ReconciliationStatus::NotStarted {
                    current: current.as_ref().map(|value| value.head),
                }
            }
            PublicationDecision::Accepted(current) => ReconciliationStatus::Accepted { current },
            PublicationDecision::Stale { expected, current } => {
                ReconciliationStatus::Stale { expected, current }
            }
            PublicationDecision::ConflictingIdempotency(current) => {
                ReconciliationStatus::ConflictingIdempotency { current }
            }
        })
    }

    #[cfg(test)]
    pub fn publish_with_fault(
        &self,
        prepared: &PreparedPublication,
        point: PublicationPoint,
    ) -> Result<PublicationOutcome, Diagnostic> {
        self.publish_at(prepared, Some(point))
    }

    fn publish_at(
        &self,
        prepared: &PreparedPublication,
        fault: Option<PublicationPoint>,
    ) -> Result<PublicationOutcome, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_exclusive(&lock).map_err(|error| {
            io_diagnostic("publication_repository_write_lock", &self.root, error)
        })?;
        let mut store = PackDirectoryStore::open(&self.root).map_err(store_diagnostic)?;
        let current = read_current_optional(&root_directory, &store)?;
        validate_prepared_repository(prepared)?;
        match classify_publication(current.as_ref(), prepared) {
            PublicationDecision::Ready => {}
            PublicationDecision::Accepted(current) => {
                return Ok(PublicationOutcome::AlreadyAccepted { current: *current });
            }
            PublicationDecision::Stale { expected, current } => {
                return Ok(PublicationOutcome::Stale { expected, current });
            }
            PublicationDecision::ConflictingIdempotency(_) => {
                return Err(repository_error(
                    DiagnosticClass::Source,
                    "publication_repository_idempotency_conflict",
                    "current HEAD already binds this idempotency key to a different normalized transaction",
                ));
            }
        }
        validate_history_base(
            current.as_ref().map(|value| value.accepted),
            prepared.receipt.status,
            &prepared.transaction,
            &prepared.semantic_diff,
        )?;
        inject(fault, PublicationPoint::BeforeObjectStage)?;

        let mut store_work = StoreWork::default();
        for (index, (key, bytes)) in prepared.objects.iter().enumerate() {
            store
                .stage(*key, bytes, &mut store_work)
                .map_err(store_diagnostic)?;
            if index == 0 {
                inject(fault, PublicationPoint::AfterFirstObjectStage)?;
            }
        }
        let seal = store
            .seal_staged(TARGET_PACK_BYTES, &mut store_work)
            .map_err(store_diagnostic)?;
        inject(fault, PublicationPoint::AfterPacksSealed)?;
        verify_prepared_closure(&store, prepared, &mut store_work)?;
        replace_head(&root_directory, &prepared.head_bytes, fault)?;
        let current = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "publication_repository_visibility_missing",
                "published HEAD is not visible after successful atomic replacement",
            )
        })?;
        if current.head != prepared.head {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "publication_repository_visibility_mismatch",
                "visible HEAD differs from the exact prepared publication",
            ));
        }
        Ok(PublicationOutcome::Accepted {
            current,
            seal,
            store_work,
        })
    }

    pub fn head_staging_leftovers(&self) -> Result<Vec<String>, Diagnostic> {
        let mut names = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|error| {
            io_diagnostic("publication_repository_stage_enumerate", &self.root, error)
        })? {
            let entry = entry.map_err(|error| {
                io_diagnostic("publication_repository_stage_enumerate", &self.root, error)
            })?;
            let name = entry.file_name().into_string().map_err(|_| {
                repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_repository_stage_name",
                    "repository root contains a non-UTF-8 entry",
                )
            })?;
            if name.starts_with(HEAD_STAGE_PREFIX) {
                let file_type = entry.file_type().map_err(|error| {
                    io_diagnostic("publication_repository_stage_type", &entry.path(), error)
                })?;
                if file_type.is_symlink() || !file_type.is_file() {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "publication_repository_stage_type",
                        "HEAD stage entry is not a regular non-symlink file",
                    ));
                }
                names.push(name);
            }
        }
        names.sort();
        Ok(names)
    }
}

fn classify_publication(
    current: Option<&CurrentPublication>,
    prepared: &PreparedPublication,
) -> PublicationDecision {
    if let Some(current) = current {
        if current.head == prepared.head {
            return PublicationDecision::Accepted(Box::new(current.clone()));
        }
        if let Some(key) = prepared.receipt.idempotency_key.as_ref()
            && current.receipt.idempotency_key.as_ref() == Some(key)
        {
            if current.receipt.bases == prepared.receipt.bases
                && current.receipt.transaction == prepared.receipt.transaction
                && current.head.repository_id == prepared.head.repository_id
            {
                return PublicationDecision::Accepted(Box::new(current.clone()));
            }
            return PublicationDecision::ConflictingIdempotency(current.head);
        }
    }
    let observed = current.map(|value| value.head);
    if observed == prepared.expected_base {
        PublicationDecision::Ready
    } else {
        PublicationDecision::Stale {
            expected: prepared.expected_base,
            current: observed,
        }
    }
}

fn read_current_optional(
    root_directory: &File,
    store: &PackDirectoryStore,
) -> Result<Option<CurrentPublication>, Diagnostic> {
    let Some(head_bytes) = read_optional_regular_at(
        root_directory,
        HEAD_FILE,
        MAXIMUM_HEAD_BYTES,
        "publication_repository_head_read",
    )?
    else {
        return Ok(None);
    };
    let head = HeadRecord::decode(&head_bytes)?;
    let mut store_work = StoreWork::default();
    let revision_bytes = read_required(
        store,
        ObjectDomain::Revision,
        head.record.bytes(),
        &mut store_work,
    )?;
    let revision = RevisionRecord::decode(&revision_bytes, head.record)?;
    let receipt_bytes = read_required(
        store,
        ObjectDomain::Receipt,
        revision.receipt.bytes(),
        &mut store_work,
    )?;
    let receipt = PublicationReceipt::decode(&receipt_bytes, revision.receipt)?;
    let transaction_bytes = read_required(
        store,
        ObjectDomain::Transaction,
        revision.core.transaction.bytes(),
        &mut store_work,
    )?;
    let transaction = NormalizedTransaction::decode(&transaction_bytes, revision.core.transaction)?;
    let diff_bytes = read_required(
        store,
        ObjectDomain::SemanticDiff,
        revision.core.semantic_diff.bytes(),
        &mut store_work,
    )?;
    let semantic_diff = SemanticDiff::decode(&diff_bytes, revision.core.semantic_diff)?;
    let root_bytes = read_required(
        store,
        ObjectDomain::SemanticRoot,
        revision.core.semantic_root.bytes(),
        &mut store_work,
    )?;
    let semantic_root = decode_root(&root_bytes, revision.core.semantic_root)?;
    let witness_bytes = read_required(
        store,
        ObjectDomain::ValidationWitness,
        revision.core.validation_witness.bytes(),
        &mut store_work,
    )?;
    let witness = decode_witness_manifest(&witness_bytes, revision.core.validation_witness)?;
    let accepted =
        AcceptedBinding::verify(head, &revision, revision.core.validation_witness, &witness)?;
    if semantic_root.repository_id != head.repository_id
        || semantic_root.package_id != witness.package_id
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_root_witness",
            "semantic root and validation witness belong to different repository or package authority",
        ));
    }
    if receipt.repository_id != head.repository_id
        || receipt.result != revision.revision
        || receipt.transaction != revision.core.transaction
        || receipt.semantic_diff != revision.core.semantic_diff
        || receipt.bases
            != revision
                .core
                .parents
                .iter()
                .map(|parent| parent.revision)
                .collect::<Vec<_>>()
        || transaction.repository_id != head.repository_id
        || semantic_diff.repository_id != head.repository_id
        || transaction.result_root() != revision.core.semantic_root
        || semantic_diff.result_root() != revision.core.semantic_root
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_receipt_binding",
            "revision, receipt, and exact parent identities do not form one accepted record",
        ));
    }
    let base = match revision.core.parents.as_slice() {
        [] => None,
        [parent] => Some(load_parent_binding(
            store,
            head.repository_id,
            *parent,
            &mut store_work,
        )?),
        [_, _] => {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_repository_merge_unimplemented",
                "two-parent Graph 5 history is reserved but not readable before typed merge cutover",
            ));
        }
        _ => unreachable!("revision codec rejects more than two parents"),
    };
    validate_history_base(base, receipt.status, &transaction, &semantic_diff)?;
    Ok(Some(CurrentPublication {
        head,
        revision,
        receipt,
        transaction,
        semantic_diff,
        semantic_root,
        witness,
        accepted,
        store_work,
    }))
}

fn load_parent_binding(
    store: &PackDirectoryStore,
    repository_id: crate::platform::semantic_id::RepositoryId,
    parent: super::ParentRevision,
    work: &mut StoreWork,
) -> Result<AcceptedBinding, Diagnostic> {
    let revision_bytes = read_required(store, ObjectDomain::Revision, parent.record.bytes(), work)?;
    let revision = RevisionRecord::decode(&revision_bytes, parent.record)?;
    if revision.revision != parent.revision || revision.core.repository_id != repository_id {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_parent_binding",
            "revision parent identity or repository disagrees with its exact record",
        ));
    }
    let root_bytes = read_required(
        store,
        ObjectDomain::SemanticRoot,
        revision.core.semantic_root.bytes(),
        work,
    )?;
    let root = decode_root(&root_bytes, revision.core.semantic_root)?;
    let witness_bytes = read_required(
        store,
        ObjectDomain::ValidationWitness,
        revision.core.validation_witness.bytes(),
        work,
    )?;
    let witness = decode_witness_manifest(&witness_bytes, revision.core.validation_witness)?;
    if root.repository_id != repository_id || root.package_id != witness.package_id {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_parent_root",
            "parent semantic root and witness belong to different authority",
        ));
    }
    AcceptedBinding::verify(
        HeadRecord {
            contract_version: REVISION_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id,
            revision: parent.revision,
            record: parent.record,
        },
        &revision,
        revision.core.validation_witness,
        &witness,
    )
}

fn validate_prepared_repository(prepared: &PreparedPublication) -> Result<(), Diagnostic> {
    let (semantic_root_digest, semantic_root_bytes) =
        encode_root(&prepared.authority.semantic.root)?;
    let (witness_digest, witness_bytes) =
        encode_witness_manifest(&prepared.authority.witness.manifest)?;
    let (transaction_digest, transaction_bytes) = prepared.transaction.encode()?;
    let (semantic_diff_digest, semantic_diff_bytes) = prepared.semantic_diff.encode()?;
    let (receipt_digest, receipt_bytes) = prepared.receipt.encode()?;
    let (revision_digest, revision_bytes) = prepared.revision.encode()?;
    let accepted = AcceptedBinding::verify(
        prepared.head,
        &prepared.revision,
        prepared.authority.witness.digest,
        &prepared.authority.witness.manifest,
    )?;
    if prepared.head_bytes != prepared.head.encode()?
        || accepted != prepared.accepted
        || prepared.head.repository_id != prepared.authority.semantic.root.repository_id
        || prepared.authority.semantic.digest != prepared.accepted.semantic_root
        || prepared.authority.witness.digest != prepared.accepted.validation_witness
        || (semantic_root_digest, semantic_root_bytes.as_slice())
            != (
                prepared.authority.semantic.digest,
                prepared.authority.semantic.bytes.as_slice(),
            )
        || (witness_digest, witness_bytes.as_slice())
            != (
                prepared.authority.witness.digest,
                prepared.authority.witness.bytes.as_slice(),
            )
        || (transaction_digest, transaction_bytes.as_slice())
            != (
                prepared.transaction_digest,
                prepared.transaction_bytes.as_slice(),
            )
        || (semantic_diff_digest, semantic_diff_bytes.as_slice())
            != (
                prepared.semantic_diff_digest,
                prepared.semantic_diff_bytes.as_slice(),
            )
        || (receipt_digest, receipt_bytes.as_slice())
            != (prepared.receipt_digest, prepared.receipt_bytes.as_slice())
        || (revision_digest, revision_bytes.as_slice())
            != (prepared.revision_digest, prepared.revision_bytes.as_slice())
        || prepared.receipt.repository_id != prepared.head.repository_id
        || prepared.receipt.result != prepared.revision.revision
        || prepared.receipt.transaction != prepared.revision.core.transaction
        || prepared.receipt.semantic_diff != prepared.revision.core.semantic_diff
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_prepared_binding",
            "prepared publication does not form one exact repository authority binding",
        ));
    }
    Ok(())
}

fn verify_prepared_closure(
    store: &PackDirectoryStore,
    prepared: &PreparedPublication,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    for (key, expected) in &prepared.objects {
        let observed = store
            .read(*key, key.domain.maximum_bytes(), work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_repository_object_missing",
                    format!(
                        "sealed publication omits required {} object",
                        key.domain.name()
                    ),
                )
            })?;
        if &observed != expected {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_repository_object_bytes",
                "sealed immutable object differs from prepared bytes",
            ));
        }
    }
    for key in [
        ObjectKey::from_digest(
            ObjectDomain::SemanticRoot,
            prepared.authority.semantic.digest.bytes(),
        ),
        ObjectKey::from_digest(
            ObjectDomain::ValidationWitness,
            prepared.authority.witness.digest.bytes(),
        ),
        ObjectKey::from_digest(
            ObjectDomain::Transaction,
            prepared.transaction_digest.bytes(),
        ),
        ObjectKey::from_digest(
            ObjectDomain::SemanticDiff,
            prepared.semantic_diff_digest.bytes(),
        ),
        ObjectKey::from_digest(ObjectDomain::Receipt, prepared.receipt_digest.bytes()),
        ObjectKey::from_digest(ObjectDomain::Revision, prepared.revision_digest.bytes()),
    ] {
        if !store.contains(key, work).map_err(store_diagnostic)? {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_repository_closure_missing",
                format!(
                    "accepted history closure omits {} object",
                    key.domain.name()
                ),
            ));
        }
    }
    Ok(())
}

fn read_required(
    store: &PackDirectoryStore,
    domain: ObjectDomain,
    digest: [u8; 32],
    work: &mut StoreWork,
) -> Result<Vec<u8>, Diagnostic> {
    store
        .read(
            ObjectKey::from_digest(domain, digest),
            domain.maximum_bytes(),
            work,
        )
        .map_err(store_diagnostic)?
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Corrupt,
                "publication_repository_object_missing",
                format!(
                    "accepted HEAD references a missing {} object",
                    domain.name()
                ),
            )
        })
}

fn initialize_unpublished(root: &Path) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(root) {
        Ok(_) => {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_repository_stage_exists",
                "private Graph 5 repository stage already exists",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(io_diagnostic(
                "publication_repository_stage_metadata",
                root,
                error,
            ));
        }
    }
    let store = PackDirectoryStore::initialize(root).map_err(store_diagnostic)?;
    drop(store);
    let root_directory = open_directory(root)?;
    let fd = rustix::fs::openat(
        &root_directory,
        LOCK_FILE,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| rustix_diagnostic("publication_repository_lock_create", error))?;
    let lock = File::from(fd);
    lock.sync_all()
        .map_err(|error| io_diagnostic("publication_repository_lock_sync", root, error))?;
    root_directory
        .sync_all()
        .map_err(|error| io_diagnostic("publication_repository_layout_sync", root, error))
}

fn canonical_new_destination(destination: &Path) -> Result<PathBuf, Diagnostic> {
    match fs::symlink_metadata(destination) {
        Ok(_) => {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_repository_exists",
                "Graph 5 repository destination already exists",
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            return Err(io_diagnostic(
                "publication_repository_destination_metadata",
                destination,
                error,
            ));
        }
    }
    let name = destination.file_name().ok_or_else(|| {
        repository_error(
            DiagnosticClass::Source,
            "publication_repository_destination",
            "Graph 5 repository destination must name one new directory",
        )
    })?;
    let parent = destination.parent().ok_or_else(|| {
        repository_error(
            DiagnosticClass::Source,
            "publication_repository_parent",
            "Graph 5 repository destination has no existing parent",
        )
    })?;
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };
    let metadata = fs::symlink_metadata(parent)
        .map_err(|error| io_diagnostic("publication_repository_parent_metadata", parent, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(repository_error(
            DiagnosticClass::Source,
            "publication_repository_parent_type",
            "Graph 5 repository parent is not a regular non-symlink directory",
        ));
    }
    let parent = parent
        .canonicalize()
        .map_err(|error| io_diagnostic("publication_repository_parent_canonical", parent, error))?;
    Ok(parent.join(name))
}

fn open_directory(path: &Path) -> Result<File, Diagnostic> {
    let fd = rustix::fs::open(
        path,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| rustix_diagnostic("publication_repository_directory_open", error))?;
    Ok(File::from(fd))
}

fn open_lock(root_directory: &File) -> Result<File, Diagnostic> {
    let fd = rustix::fs::openat(
        root_directory,
        LOCK_FILE,
        OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| rustix_diagnostic("publication_repository_lock_open", error))?;
    let file = File::from(fd);
    let metadata = file.metadata().map_err(|error| {
        io_diagnostic(
            "publication_repository_lock_metadata",
            Path::new(LOCK_FILE),
            error,
        )
    })?;
    if !metadata.is_file() || metadata.len() != 0 {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_lock_type",
            "Graph 5 publication lock is not an empty regular file",
        ));
    }
    Ok(file)
}

fn read_optional_regular_at(
    directory: &File,
    name: &str,
    maximum: usize,
    code: &'static str,
) -> Result<Option<Vec<u8>>, Diagnostic> {
    let fd = match rustix::fs::openat(
        directory,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => return Err(rustix_diagnostic(code, error)),
    };
    let mut file = File::from(fd);
    let metadata = file
        .metadata()
        .map_err(|error| io_diagnostic(code, Path::new(name), error))?;
    if !metadata.is_file() {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            code,
            format!("repository entry '{name}' is not a regular non-symlink file"),
        ));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| {
        repository_error(
            DiagnosticClass::Resource,
            code,
            format!("repository entry '{name}' length does not fit this platform"),
        )
    })?;
    if length > maximum {
        return Err(repository_error(
            DiagnosticClass::Resource,
            code,
            format!("repository entry '{name}' exceeds {maximum} bytes"),
        ));
    }
    let mut bytes = Vec::with_capacity(length);
    Read::by_ref(&mut file)
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_diagnostic(code, Path::new(name), error))?;
    if bytes.len() != length || bytes.len() > maximum {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            code,
            format!("repository entry '{name}' changed during its bounded read"),
        ));
    }
    Ok(Some(bytes))
}

fn replace_head(
    root_directory: &File,
    bytes: &[u8],
    fault: Option<PublicationPoint>,
) -> Result<(), Diagnostic> {
    if bytes.len() > MAXIMUM_HEAD_BYTES
        || bytes.len() < HEAD_MAGIC.len()
        || &bytes[..HEAD_MAGIC.len()] != HEAD_MAGIC.as_slice()
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_head_candidate",
            "prepared HEAD bytes exceed their bound or use a foreign magic value",
        ));
    }
    let temporary = format!(
        "{HEAD_STAGE_PREFIX}{}",
        random_hex("publication_repository_head_random")?
    );
    let fd = rustix::fs::openat(
        root_directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| rustix_diagnostic("publication_repository_head_stage", error))?;
    let mut file = File::from(fd);
    if let Err(error) = file.write_all(bytes).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = rustix::fs::unlinkat(root_directory, temporary.as_str(), AtFlags::empty());
        return Err(io_diagnostic(
            "publication_repository_head_write",
            Path::new(HEAD_FILE),
            error,
        ));
    }
    drop(file);
    inject(fault, PublicationPoint::AfterHeadFileSynced)?;
    if let Err(error) = rustix::fs::renameat(
        root_directory,
        temporary.as_str(),
        root_directory,
        HEAD_FILE,
    ) {
        let _ = rustix::fs::unlinkat(root_directory, temporary.as_str(), AtFlags::empty());
        return Err(rustix_diagnostic(
            "publication_repository_head_publish",
            error,
        ));
    }
    inject(fault, PublicationPoint::AfterHeadRenamed)?;
    root_directory.sync_all().map_err(|error| {
        repository_error(
            DiagnosticClass::Infrastructure,
            "publication_repository_visibility_indeterminate",
            format!("HEAD rename succeeded but directory durability is indeterminate: {error}"),
        )
    })?;
    inject(fault, PublicationPoint::AfterHeadDirectorySynced)
}

fn inject(fault: Option<PublicationPoint>, observed: PublicationPoint) -> Result<(), Diagnostic> {
    if fault == Some(observed) {
        return Err(repository_error(
            DiagnosticClass::Infrastructure,
            "publication_repository_injected_interruption",
            format!("deterministic publication interruption at {observed:?}"),
        ));
    }
    Ok(())
}

fn random_hex(code: &'static str) -> Result<String, Diagnostic> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes).map_err(|error| {
        repository_error(
            DiagnosticClass::Infrastructure,
            code,
            format!("secure private-stage randomness is unavailable: {error}"),
        )
    })?;
    Ok(crate::platform::semantic_id::encode_hex(&bytes))
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

fn remove_owned_repository_stage(stage: &Path) {
    if stage
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with(REPOSITORY_STAGE_PREFIX))
    {
        let _ = fs::remove_dir_all(stage);
    }
}

fn collapse_diagnostics(mut diagnostics: Vec<Diagnostic>) -> Diagnostic {
    if diagnostics.is_empty() {
        return repository_error(
            DiagnosticClass::Infrastructure,
            "publication_repository_empty_diagnostics",
            "Graph 5 preparation failed without a diagnostic",
        );
    }
    let additional = diagnostics.len().saturating_sub(1);
    let mut first = diagnostics.remove(0);
    if additional > 0 {
        first.notes.push(format!(
            "{additional} additional deterministic preparation diagnostics omitted"
        ));
    }
    first
}

fn store_diagnostic(error: StoreError) -> Diagnostic {
    let class = match error.class {
        StoreErrorClass::Input => DiagnosticClass::Source,
        StoreErrorClass::Resource => DiagnosticClass::Resource,
        StoreErrorClass::Corrupt => DiagnosticClass::Corrupt,
        StoreErrorClass::Io => DiagnosticClass::Infrastructure,
    };
    repository_error(class, error.code, error.message)
}

fn rustix_diagnostic(code: &'static str, error: rustix::io::Errno) -> Diagnostic {
    repository_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("filesystem operation failed: {error}"),
    )
}

fn io_diagnostic(code: &'static str, path: &Path, error: std::io::Error) -> Diagnostic {
    repository_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn repository_error(
    class: DiagnosticClass,
    code: impl Into<String>,
    message: impl Into<String>,
) -> Diagnostic {
    Diagnostic::new(class, code, message)
}
