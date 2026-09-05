//! Exact Graph 10 repository reads and one locked, atomic accepted-HEAD publication point.

use super::contract::{HEAD_MAGIC, MAXIMUM_HEAD_BYTES, REVISION_CONTRACT_VERSION};
use super::idempotency::{
    IdempotencyBinding, advance_idempotency_history, empty_idempotency_history,
    lookup_idempotency_history,
};
use super::prepare::validate_history_base;
use super::read_view::RepositoryView;
use super::{
    AcceptedBinding, HeadRecord, NormalizedTransaction, PreparedAuthoredPublication,
    PreparedInitialPublication, PreparedPublication, PublicationOptions, PublicationReceipt,
    RevisionRecord, SemanticDiff, prepare_initial_publication,
};
use crate::platform::change::{AuthoredChangeSet, PrimitiveEdit};
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::kernel::{
    KernelSnapshot, SemanticRoot, decode_root, encode_root, semantic_state_digest_from_root,
};
use crate::platform::package_transport::{
    PackageRevision, PackageTransport, PackageTransportBinding, PackageTransportClosureValidation,
    validate_package_revision_closure, validate_package_transport_closure_admitted,
};
use crate::platform::persistent_map::{MapAdmission, MapWork, MemoryPageStore, OverlayPageStore};
use crate::platform::storage::contract::TARGET_PACK_BYTES;
#[cfg(test)]
use crate::platform::storage::directory::SealCheckpoint;
use crate::platform::storage::directory::{PackDirectoryStore, SealReceipt};
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, ObjectStage, StageOutcome, StoreError,
    StoreErrorClass, StoreWork,
};
use crate::platform::storage::page_store::ObjectPageReader;
use crate::platform::witness::{
    ValidationWitnessManifest, decode_witness_manifest, encode_witness_manifest,
};
use fs2::FileExt;
use rustix::fs::{AtFlags, Dir, Mode, OFlags};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const HEAD_FILE: &str = "HEAD";
const LOCK_FILE: &str = "LOCK";
const HEAD_STAGE_PREFIX: &str = ".HEAD-stage-";
const REPOSITORY_STAGE_PREFIX: &str = ".lkjscript-graph9-stage-";
const PACKAGE_TRANSPORT_DIRECTORY: &str = "PACKAGE-TRANSPORTS";
const PACKAGE_TRANSPORT_SELECTION_FILE: &str = "CURRENT";
const PACKAGE_TRANSPORT_SELECTION_STAGE_PREFIX: &str = ".CURRENT-stage-";

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
pub struct InitialPackageTransport {
    pub digest: crate::platform::kernel::PackageTransportDigest,
    pub container: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PackageStagePoint {
    BeforeObjects,
    AfterFirstObject,
    ObjectsDurable,
    ReadinessSynced,
    ReadinessVisible,
    #[cfg(test)]
    Storage(SealCheckpoint),
}

#[derive(Clone, Debug)]
pub struct PackageTransportStageReceipt {
    pub package: crate::platform::kernel::PackageId,
    pub semantic_revision: crate::platform::semantic_id::RevisionId,
    pub package_revision: crate::platform::kernel::PackageRevisionDigest,
    pub package_transport: crate::platform::kernel::PackageTransportDigest,
    pub previous_transport: Option<crate::platform::kernel::PackageTransportDigest>,
    pub outcome: StageOutcome,
    pub seal: SealReceipt,
    pub current_revision: crate::platform::semantic_id::RevisionId,
    pub work: StoreWork,
    pub closure_packages: usize,
    pub closure_edges: usize,
    pub closure_objects: usize,
    pub container_bytes: usize,
    pub container_commitment: String,
    pub validation_visits: u64,
    pub validation_read_bytes: u64,
}

#[derive(Clone, Debug)]
pub enum PublicationOutcome {
    Accepted {
        current: CurrentPublication,
        seal: SealReceipt,
        store_work: StoreWork,
    },
    AlreadyAccepted {
        accepted: CurrentPublication,
        observed: HeadRecord,
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
        accepted: Box<CurrentPublication>,
        observed: HeadRecord,
    },
    Stale {
        expected: Option<HeadRecord>,
        current: Option<HeadRecord>,
    },
    ConflictingIdempotency {
        accepted: HeadRecord,
    },
}

#[derive(Clone, Debug)]
pub struct ReconciliationResult {
    pub status: ReconciliationStatus,
    pub work: ReconciliationWork,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ReconciliationWork {
    pub objects_read: u64,
    pub bytes_read: u64,
    pub idempotency_lookup_pages_read: u64,
    pub idempotency_lookup_entries_visited: u64,
    pub accepted_publications_loaded: u64,
}

impl ReconciliationWork {
    fn add_store(&mut self, work: StoreWork) {
        self.objects_read = self.objects_read.saturating_add(work.objects_read);
        self.bytes_read = self.bytes_read.saturating_add(work.bytes_read);
    }

    fn add_map(&mut self, work: MapWork) {
        self.idempotency_lookup_pages_read = self
            .idempotency_lookup_pages_read
            .saturating_add(work.pages_read);
        self.idempotency_lookup_entries_visited = self
            .idempotency_lookup_entries_visited
            .saturating_add(work.entries_visited);
    }

    fn add(&mut self, other: Self) {
        self.objects_read = self.objects_read.saturating_add(other.objects_read);
        self.bytes_read = self.bytes_read.saturating_add(other.bytes_read);
        self.idempotency_lookup_pages_read = self
            .idempotency_lookup_pages_read
            .saturating_add(other.idempotency_lookup_pages_read);
        self.idempotency_lookup_entries_visited = self
            .idempotency_lookup_entries_visited
            .saturating_add(other.idempotency_lookup_entries_visited);
        self.accepted_publications_loaded = self
            .accepted_publications_loaded
            .saturating_add(other.accepted_publications_loaded);
    }
}

#[derive(Clone, Debug)]
enum PublicationDecision {
    Ready,
    Accepted {
        accepted: Box<CurrentPublication>,
        observed: HeadRecord,
    },
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
    #[cfg(test)]
    Storage(SealCheckpoint),
}

impl GraphRepository {
    pub fn create(
        destination: &Path,
        logical: &KernelSnapshot,
        intent: Option<String>,
    ) -> Result<CreatedRepository, Diagnostic> {
        Self::create_with_package_transports(destination, logical, intent, &[])
    }

    /// Creates one privately staged repository whose first accepted revision may bind exact
    /// dependency package transports. The transports are fully validated and made durable inside
    /// the private repository before initial semantic preparation; neither the repository nor a
    /// partial dependency selection is externally visible before the final directory rename.
    pub fn create_with_package_transports(
        destination: &Path,
        logical: &KernelSnapshot,
        intent: Option<String>,
        package_transports: &[InitialPackageTransport],
    ) -> Result<CreatedRepository, Diagnostic> {
        let destination = canonical_new_destination(destination)?;
        let parent = destination.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_parent",
                "Graph 10 repository destination has no existing parent",
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
            let root_directory = open_directory(&stage)?;
            let mut preparation_store =
                PackDirectoryStore::open(&stage).map_err(store_diagnostic)?;
            for package_transport in package_transports {
                let _ = install_package_container(
                    &root_directory,
                    &mut preparation_store,
                    package_transport.digest,
                    &package_transport.container,
                    None,
                )?;
            }
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
                "Graph 10 repository root is not a regular non-symlink directory",
            ));
        }
        let root = root
            .canonicalize()
            .map_err(|error| io_diagnostic("publication_repository_open_canonical", root, error))?;
        let root_directory = open_directory(&root)?;
        let lock = open_or_reconstruct_lock(&root_directory, &root)?;
        FileExt::lock_shared(&lock)
            .map_err(|error| io_diagnostic("publication_repository_read_lock", &root, error))?;
        let store = open_store_shared(&root_directory, &root, &lock)?;
        let _ = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 10 repository has no accepted HEAD",
            )
        })?;
        let repository = Self { root };
        Ok(repository)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn object_store(&self) -> Result<PackDirectoryStore, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic(
                "publication_repository_object_store_lock",
                &self.root,
                error,
            )
        })?;
        open_store_shared(&root_directory, &self.root, &lock)
    }

    pub fn export_package_transport(
        &self,
    ) -> Result<super::read_view::ExportedPackageTransport, Diagnostic> {
        self.view_current()?.export_package_transport()
    }

    pub(crate) fn export_package_container(
        &self,
    ) -> Result<crate::platform::package_transport::source::AdmittedClosure, Diagnostic> {
        self.view_current()?.export_package_container()
    }

    pub(crate) fn staged_package_source(
        &self,
        revision: crate::platform::kernel::PackageRevisionDigest,
    ) -> Result<crate::platform::package_transport::source::AdmittedClosure, Diagnostic> {
        let store = self.object_store()?;
        let closure = resolve_package_transport_closure(
            &store,
            &store,
            revision,
            None,
            None,
            &mut StoreWork::default(),
        )?;
        crate::platform::package_transport::source::collect(
            &store,
            PackageTransportBinding {
                package_revision: revision,
                transport: closure.root_transport_digest,
            },
            &closure.selections,
        )
    }

    /// Admits and atomically stages one complete immutable source closure without changing HEAD.
    pub fn stage_package_transport(
        &self,
        digest: crate::platform::kernel::PackageTransportDigest,
        container: &[u8],
    ) -> Result<PackageTransportStageReceipt, Diagnostic> {
        self.stage_package_transport_at(digest, container, None)
    }

    #[cfg(test)]
    pub(crate) fn stage_package_transport_with_fault(
        &self,
        digest: crate::platform::kernel::PackageTransportDigest,
        container: &[u8],
        fault: PackageStagePoint,
    ) -> Result<PackageTransportStageReceipt, Diagnostic> {
        self.stage_package_transport_at(digest, container, Some(fault))
    }

    fn stage_package_transport_at(
        &self,
        digest: crate::platform::kernel::PackageTransportDigest,
        container: &[u8],
        fault: Option<PackageStagePoint>,
    ) -> Result<PackageTransportStageReceipt, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_exclusive(&lock)
            .map_err(|error| io_diagnostic("publication_package_stage_lock", &self.root, error))?;
        let mut store = open_store_exclusive(&root_directory, &self.root)?;
        let before = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 10 repository has no accepted HEAD",
            )
        })?;
        let installed =
            install_package_container(&root_directory, &mut store, digest, container, fault)?;
        let after = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Corrupt,
                "publication_package_stage_head_missing",
                "package staging made the accepted HEAD unreadable",
            )
        })?;
        if after.head != before.head {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_package_stage_head_changed",
                "operational package staging changed accepted authority",
            ));
        }
        Ok(PackageTransportStageReceipt {
            package: installed.revision.package,
            semantic_revision: installed.revision.revision.revision_id()?,
            package_revision: installed.transport.package_revision,
            package_transport: digest,
            previous_transport: installed.previous_transport,
            outcome: installed.outcome,
            seal: installed.seal,
            current_revision: after.head.revision,
            work: installed.work,
            closure_packages: installed.closure_packages,
            closure_edges: installed.closure_edges,
            closure_objects: installed.closure_objects,
            container_bytes: container.len(),
            container_commitment: blake3::hash(container).to_hex().to_string(),
            validation_visits: installed.validation_visits,
            validation_read_bytes: installed.validation_read_bytes,
        })
    }

    /// Durably stages one exact set of derived compiler objects without changing accepted HEAD.
    ///
    /// The repository publication lock serializes pack installation with accepted publication.
    /// Only compiler units, their small manifest, and persistent-map pages are admitted here;
    /// callers cannot use this operational path to write canonical semantic objects.
    pub(crate) fn stage_compilation_objects(
        &self,
        expected: HeadRecord,
        objects: &BTreeMap<ObjectKey, Vec<u8>>,
    ) -> Result<(SealReceipt, StoreWork), Diagnostic> {
        if objects.is_empty()
            || objects.keys().any(|key| {
                !matches!(
                    key.domain,
                    ObjectDomain::CompilerUnit
                        | ObjectDomain::CompilationManifest
                        | ObjectDomain::PackageRevision
                        | ObjectDomain::MapPage
                )
            })
        {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_compilation_object_domain",
                "derived compilation staging admits only compiler units, one logical package revision, one manifest, and map pages",
            ));
        }
        let manifest_count = objects
            .keys()
            .filter(|key| key.domain == ObjectDomain::CompilationManifest)
            .count();
        if manifest_count != 1 {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_compilation_manifest_count",
                "derived compilation staging requires exactly one compilation manifest",
            ));
        }
        let package_revision_count = objects
            .keys()
            .filter(|key| key.domain == ObjectDomain::PackageRevision)
            .count();
        if package_revision_count != 1 {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_compilation_package_revision_count",
                "derived compilation staging requires exactly one logical package revision",
            ));
        }

        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_exclusive(&lock).map_err(|error| {
            io_diagnostic("publication_compilation_stage_lock", &self.root, error)
        })?;
        let mut store = open_store_exclusive(&root_directory, &self.root)?;
        let before = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 10 repository has no accepted HEAD",
            )
        })?;
        if before.head != expected {
            return Err(repository_error(
                DiagnosticClass::Semantic,
                "publication_compilation_stage_stale",
                "accepted HEAD changed before derived compilation objects could be staged",
            ));
        }
        let mut work = StoreWork::default();
        for (key, bytes) in objects {
            store
                .stage(*key, bytes, &mut work)
                .map_err(store_diagnostic)?;
        }
        let seal = store
            .seal_staged(TARGET_PACK_BYTES, &mut work)
            .map_err(store_diagnostic)?;
        let after = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Corrupt,
                "publication_compilation_stage_head_missing",
                "derived compilation staging made accepted HEAD unreadable",
            )
        })?;
        if after.head != expected {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_compilation_stage_head_changed",
                "derived compilation staging changed accepted authority",
            ));
        }
        Ok((seal, work))
    }

    pub fn current(&self) -> Result<CurrentPublication, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic("publication_repository_read_lock", &self.root, error)
        })?;
        let store = open_store_shared(&root_directory, &self.root, &lock)?;
        read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 10 repository has no accepted HEAD",
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
        let store = open_store_shared(&root_directory, &self.root, &lock)?;
        let current = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 10 repository has no accepted HEAD",
            )
        })?;
        Ok(RepositoryView::new(current, store))
    }

    /// Reopens the exact accepted base bound to an already accepted idempotency key. This is a
    /// read-only retry path: callers must still normalize and prepare the complete request against
    /// the returned view, compare its reviewed commitments, and enter `publish` for the locked
    /// idempotency and HEAD decision.
    pub(crate) fn view_idempotency_base(
        &self,
        key: &str,
        base: crate::platform::semantic_id::RevisionId,
    ) -> Result<Option<RepositoryView>, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic(
                "publication_repository_idempotency_view_lock",
                &self.root,
                error,
            )
        })?;
        let store = open_store_shared(&root_directory, &self.root, &lock)?;
        let current = read_current_optional(&root_directory, &store)?.ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "publication_repository_unpublished",
                "Graph 10 repository has no accepted HEAD",
            )
        })?;
        let binding = if current.receipt.idempotency_key.as_deref() == Some(key) {
            IdempotencyBinding::from_accepted(current.accepted, &current.receipt)?
        } else {
            let reader = ObjectPageReader::new(&store);
            let mut map_work = MapWork::default();
            lookup_idempotency_history(
                current.revision.publication.idempotency_receipts,
                key,
                &reader,
                &mut map_work,
            )?
        };
        let Some(binding) = binding.filter(|binding| binding.base == base) else {
            return Ok(None);
        };
        let accepted = read_publication(&store, binding.result)?;
        validate_idempotency_result(&binding, &accepted)?;
        let [parent] = accepted.revision.publication.parents.as_slice() else {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_repository_idempotency_base",
                "accepted idempotency result does not bind one exact parent revision",
            ));
        };
        if parent.revision != base {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_repository_idempotency_base",
                "accepted idempotency result parent disagrees with its history binding",
            ));
        }
        let base_head = HeadRecord {
            contract_version: REVISION_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: binding.repository_id,
            revision: parent.revision,
            record: parent.record,
        };
        let base_publication = read_publication(&store, base_head)?;
        let hidden_type_objects = match &accepted.semantic_diff.body {
            crate::platform::publication::SemanticDiffBody::Change { type_additions, .. } => {
                type_additions.iter().copied().collect()
            }
            crate::platform::publication::SemanticDiffBody::Bootstrap { .. } => {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_repository_idempotency_diff",
                    "accepted idempotency result has a bootstrap semantic diff",
                ));
            }
        };
        Ok(Some(RepositoryView::new_idempotency_base(
            base_publication,
            store,
            hidden_type_objects,
        )))
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

    /// Prepares one high-level Graph 10 request against the currently observed exact revision.
    pub fn prepare_authored_change(
        &self,
        request: &AuthoredChangeSet,
        options: PublicationOptions,
    ) -> Result<PreparedAuthoredPublication, Vec<Diagnostic>> {
        self.view_current()
            .map_err(|diagnostic| vec![diagnostic])?
            .prepare_authored_change(request, options)
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
    ) -> Result<ReconciliationResult, Diagnostic> {
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic("publication_repository_reconcile_lock", &self.root, error)
        })?;
        let store = open_store_shared(&root_directory, &self.root, &lock)?;
        let current = read_current_optional(&root_directory, &store)?;
        validate_prepared_repository(prepared)?;
        let mut work = ReconciliationWork::default();
        if let Some(current) = current.as_ref() {
            work.add_store(current.store_work);
        }
        let (decision, decision_work) = classify_publication(&store, current.as_ref(), prepared)?;
        work.add(decision_work);
        let status = match decision {
            PublicationDecision::Ready => {
                validate_history_base(
                    current.as_ref().map(|value| value.accepted),
                    prepared.receipt.status,
                    &prepared.transaction,
                    &prepared.semantic_diff,
                )?;
                let mut transition_work = StoreWork::default();
                validate_candidate_idempotency_transition(
                    &store,
                    prepared.revision.publication.idempotency_receipts,
                    current.as_ref().map(|value| value.accepted),
                    &prepared.objects,
                    &mut transition_work,
                )?;
                work.add_store(transition_work);
                ReconciliationStatus::NotStarted {
                    current: current.as_ref().map(|value| value.head),
                }
            }
            PublicationDecision::Accepted { accepted, observed } => {
                ReconciliationStatus::Accepted { accepted, observed }
            }
            PublicationDecision::Stale { expected, current } => {
                ReconciliationStatus::Stale { expected, current }
            }
            PublicationDecision::ConflictingIdempotency(accepted) => {
                ReconciliationStatus::ConflictingIdempotency { accepted }
            }
        };
        Ok(ReconciliationResult { status, work })
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
        let mut store = open_store_exclusive(&root_directory, &self.root)?;
        let current = read_current_optional(&root_directory, &store)?;
        validate_prepared_repository(prepared)?;
        match classify_publication(&store, current.as_ref(), prepared)?.0 {
            PublicationDecision::Ready => {}
            PublicationDecision::Accepted { accepted, observed } => {
                return Ok(PublicationOutcome::AlreadyAccepted {
                    accepted: *accepted,
                    observed,
                });
            }
            PublicationDecision::Stale { expected, current } => {
                return Ok(PublicationOutcome::Stale { expected, current });
            }
            PublicationDecision::ConflictingIdempotency(_) => {
                return Err(repository_error(
                    DiagnosticClass::Source,
                    "publication_repository_idempotency_conflict",
                    "accepted history already binds this idempotency key to a different normalized transaction",
                ));
            }
        }
        validate_history_base(
            current.as_ref().map(|value| value.accepted),
            prepared.receipt.status,
            &prepared.transaction,
            &prepared.semantic_diff,
        )?;
        let mut store_work = StoreWork::default();
        validate_prepared_dependency_sources(&store, prepared)?;
        validate_candidate_idempotency_transition(
            &store,
            prepared.revision.publication.idempotency_receipts,
            current.as_ref().map(|value| value.accepted),
            &prepared.objects,
            &mut store_work,
        )?;
        inject(fault, PublicationPoint::BeforeObjectStage)?;

        for (index, (key, bytes)) in prepared.objects.iter().enumerate() {
            store
                .stage(*key, bytes, &mut store_work)
                .map_err(store_diagnostic)?;
            if index == 0 {
                inject(fault, PublicationPoint::AfterFirstObjectStage)?;
            }
        }
        #[cfg(test)]
        let seal = match fault {
            Some(PublicationPoint::Storage(checkpoint)) => store
                .seal_staged_with_fault(TARGET_PACK_BYTES, &mut store_work, checkpoint)
                .map_err(store_diagnostic)?,
            _ => store
                .seal_staged(TARGET_PACK_BYTES, &mut store_work)
                .map_err(store_diagnostic)?,
        };
        #[cfg(not(test))]
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

fn open_validated_store(
    root_directory: &File,
    root: &Path,
) -> Result<PackDirectoryStore, Diagnostic> {
    let store = PackDirectoryStore::open(root).map_err(store_diagnostic)?;
    let _ = read_current_optional(root_directory, &store)?;
    Ok(store)
}

fn recover_validated_store(
    root_directory: &File,
    root: &Path,
    initial: &Diagnostic,
) -> Result<PackDirectoryStore, Diagnostic> {
    let reason = format!(
        "exclusive catalog recovery after {}: {}",
        initial.code, initial.message
    );
    let store = PackDirectoryStore::recover_catalog(root, reason).map_err(store_diagnostic)?;
    let _ = read_current_optional(root_directory, &store)?;
    Ok(store)
}

/// Opens and validates one catalog snapshot while the caller holds the shared repository lock.
/// A failed healthy observation is rechecked and, if still failing, rebuilt exactly once under
/// the exclusive lock before atomically downgrading to a shared observation.
fn open_store_shared(
    root_directory: &File,
    root: &Path,
    lock: &File,
) -> Result<PackDirectoryStore, Diagnostic> {
    let initial = match open_validated_store(root_directory, root) {
        Ok(store) => return Ok(store),
        Err(error) => error,
    };
    FileExt::unlock(lock)
        .map_err(|error| io_diagnostic("publication_catalog_unlock", root, error))?;
    FileExt::lock_exclusive(lock)
        .map_err(|error| io_diagnostic("publication_catalog_recovery_lock", root, error))?;
    let store = match open_validated_store(root_directory, root) {
        Ok(store) => store,
        Err(rechecked) => {
            recover_validated_store(root_directory, root, &rechecked).map_err(|mut error| {
                error.notes.push(format!(
                    "initial healthy catalog observation failed at {}",
                    initial.code
                ));
                error
            })?
        }
    };
    FileExt::lock_shared(lock)
        .map_err(|error| io_diagnostic("publication_catalog_recovery_downgrade", root, error))?;
    Ok(store)
}

/// Opens and validates one catalog snapshot while the caller already owns the exclusive lock.
fn open_store_exclusive(
    root_directory: &File,
    root: &Path,
) -> Result<PackDirectoryStore, Diagnostic> {
    match open_validated_store(root_directory, root) {
        Ok(store) => Ok(store),
        Err(error) => recover_validated_store(root_directory, root, &error),
    }
}

fn classify_publication(
    store: &PackDirectoryStore,
    current: Option<&CurrentPublication>,
    prepared: &PreparedPublication,
) -> Result<(PublicationDecision, ReconciliationWork), Diagnostic> {
    let mut work = ReconciliationWork::default();
    if let Some(current) = current {
        if current.head == prepared.head {
            return Ok((
                PublicationDecision::Accepted {
                    accepted: Box::new(current.clone()),
                    observed: current.head,
                },
                work,
            ));
        }
        if let Some(key) = prepared.receipt.idempotency_key.as_ref()
            && current.receipt.idempotency_key.as_ref() == Some(key)
        {
            let binding = IdempotencyBinding::from_accepted(current.accepted, &current.receipt)?
                .ok_or_else(|| {
                    repository_error(
                        DiagnosticClass::Corrupt,
                        "publication_repository_idempotency_current",
                        "current keyed receipt did not produce its exact idempotency binding",
                    )
                })?;
            if binding.matches_prepared(&prepared.receipt) {
                return Ok((
                    PublicationDecision::Accepted {
                        accepted: Box::new(current.clone()),
                        observed: current.head,
                    },
                    work,
                ));
            }
            return Ok((
                PublicationDecision::ConflictingIdempotency(current.head),
                work,
            ));
        }
        if let Some(key) = prepared.receipt.idempotency_key.as_deref() {
            let reader = ObjectPageReader::new(store);
            let mut map_work = MapWork::default();
            let binding = lookup_idempotency_history(
                current.revision.publication.idempotency_receipts,
                key,
                &reader,
                &mut map_work,
            )?;
            work.add_map(map_work);
            work.add_store(reader.work());
            if let Some(binding) = binding {
                if !binding.matches_prepared(&prepared.receipt) {
                    return Ok((
                        PublicationDecision::ConflictingIdempotency(binding.result),
                        work,
                    ));
                }
                let accepted = read_publication(store, binding.result)?;
                work.add_store(accepted.store_work);
                work.accepted_publications_loaded =
                    work.accepted_publications_loaded.saturating_add(1);
                validate_idempotency_result(&binding, &accepted)?;
                return Ok((
                    PublicationDecision::Accepted {
                        accepted: Box::new(accepted),
                        observed: current.head,
                    },
                    work,
                ));
            }
        }
    }
    let observed = current.map(|value| value.head);
    if observed == prepared.expected_base {
        Ok((PublicationDecision::Ready, work))
    } else {
        Ok((
            PublicationDecision::Stale {
                expected: prepared.expected_base,
                current: observed,
            },
            work,
        ))
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
    read_publication(store, head).map(Some)
}

fn read_publication(
    store: &PackDirectoryStore,
    head: HeadRecord,
) -> Result<CurrentPublication, Diagnostic> {
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
        revision.publication.receipt.bytes(),
        &mut store_work,
    )?;
    let receipt = PublicationReceipt::decode(&receipt_bytes, revision.publication.receipt)?;
    let transaction_bytes = read_required(
        store,
        ObjectDomain::Transaction,
        revision.publication.transaction.bytes(),
        &mut store_work,
    )?;
    let transaction =
        NormalizedTransaction::decode(&transaction_bytes, revision.publication.transaction)?;
    let diff_bytes = read_required(
        store,
        ObjectDomain::SemanticDiff,
        revision.publication.semantic_diff.bytes(),
        &mut store_work,
    )?;
    let semantic_diff = SemanticDiff::decode(&diff_bytes, revision.publication.semantic_diff)?;
    let root_bytes = read_required(
        store,
        ObjectDomain::SemanticRoot,
        revision.publication.semantic_root.bytes(),
        &mut store_work,
    )?;
    let semantic_root = decode_root(&root_bytes, revision.publication.semantic_root)?;
    let semantic_state = semantic_state_digest_from_root(&semantic_root)?;
    let witness_bytes = read_required(
        store,
        ObjectDomain::ValidationWitness,
        revision.publication.validation.witness.bytes(),
        &mut store_work,
    )?;
    let witness = decode_witness_manifest(&witness_bytes, revision.publication.validation.witness)?;
    let accepted = AcceptedBinding::verify(
        head,
        &revision,
        revision.publication.validation.witness,
        &witness,
    )?;
    if semantic_state != revision.core.semantic_state
        || semantic_state != accepted.semantic_state
        || semantic_root.repository_id != head.repository_id
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
        || receipt.transaction != revision.publication.transaction
        || receipt.semantic_diff != revision.publication.semantic_diff
        || receipt.bases != revision.core.parents
        || transaction.repository_id != head.repository_id
        || semantic_diff.repository_id != head.repository_id
        || transaction.result_root() != revision.publication.semantic_root
        || semantic_diff.result_root() != revision.publication.semantic_root
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_receipt_binding",
            "revision, receipt, and exact parent identities do not form one accepted record",
        ));
    }
    let base = match revision.publication.parents.as_slice() {
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
                "two-parent Graph 10 history is reserved but not readable before typed merge cutover",
            ));
        }
        _ => unreachable!("revision codec rejects more than two parents"),
    };
    validate_history_base(base, receipt.status, &transaction, &semantic_diff)?;
    validate_idempotency_transition(
        store,
        revision.publication.idempotency_receipts,
        base,
        &mut store_work,
    )?;
    Ok(CurrentPublication {
        head,
        revision,
        receipt,
        transaction,
        semantic_diff,
        semantic_root,
        witness,
        accepted,
        store_work,
    })
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
        revision.publication.semantic_root.bytes(),
        work,
    )?;
    let root = decode_root(&root_bytes, revision.publication.semantic_root)?;
    let semantic_state = semantic_state_digest_from_root(&root)?;
    let witness_bytes = read_required(
        store,
        ObjectDomain::ValidationWitness,
        revision.publication.validation.witness.bytes(),
        work,
    )?;
    let witness = decode_witness_manifest(&witness_bytes, revision.publication.validation.witness)?;
    if semantic_state != revision.core.semantic_state
        || root.repository_id != repository_id
        || root.package_id != witness.package_id
    {
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
        revision.publication.validation.witness,
        &witness,
    )
}

fn validate_idempotency_transition(
    store: &PackDirectoryStore,
    observed: crate::platform::persistent_map::MapRoot,
    base: Option<AcceptedBinding>,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    let (expected, generated) = derive_idempotency_transition(store, base, work)?;
    require_idempotency_transition(observed, expected)?;
    validate_idempotency_pages(store, &generated, None, work)?;

    let reader = ObjectPageReader::new(store);
    let mut map_work = MapWork::default();
    let _ = lookup_idempotency_history(
        observed,
        "lkjscript-integrity-probe",
        &reader,
        &mut map_work,
    )?;
    work.add(reader.work());
    Ok(())
}

fn validate_candidate_idempotency_transition(
    store: &PackDirectoryStore,
    observed: crate::platform::persistent_map::MapRoot,
    base: Option<AcceptedBinding>,
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    let (expected, generated) = derive_idempotency_transition(store, base, work)?;
    require_idempotency_transition(observed, expected)?;
    validate_idempotency_pages(store, &generated, Some(objects), work)
}

fn derive_idempotency_transition(
    store: &PackDirectoryStore,
    base: Option<AcceptedBinding>,
    work: &mut StoreWork,
) -> Result<(crate::platform::persistent_map::MapRoot, MemoryPageStore), Diagnostic> {
    let mut map_work = MapWork::default();
    match base {
        None => {
            let mut pages = MemoryPageStore::default();
            let root = empty_idempotency_history(&mut pages, &mut map_work)?;
            Ok((root, pages))
        }
        Some(base) => {
            let receipt_bytes =
                read_required(store, ObjectDomain::Receipt, base.receipt.bytes(), work)?;
            let receipt = PublicationReceipt::decode(&receipt_bytes, base.receipt)?;
            let binding = IdempotencyBinding::from_accepted(base, &receipt)?;
            let reader = ObjectPageReader::new(store);
            let mut overlay = OverlayPageStore::new(&reader);
            let expected = advance_idempotency_history(
                base.idempotency_receipts,
                binding.as_ref(),
                &mut overlay,
                &mut map_work,
            )?;
            let generated = overlay.into_pages();
            work.add(reader.work());
            Ok((expected, generated))
        }
    }
}

fn validate_idempotency_pages(
    store: &PackDirectoryStore,
    generated: &MemoryPageStore,
    candidate: Option<&BTreeMap<ObjectKey, Vec<u8>>>,
    work: &mut StoreWork,
) -> Result<(), Diagnostic> {
    for (digest, expected) in generated.objects() {
        let key = ObjectKey::from_digest(ObjectDomain::MapPage, digest.bytes());
        if let Some(observed) = candidate.and_then(|objects| objects.get(&key)) {
            if observed.as_slice() != expected {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_repository_idempotency_page_bytes",
                    "prepared idempotency page differs from its exact derived bytes",
                ));
            }
            continue;
        }
        let observed = store
            .read(key, ObjectDomain::MapPage.maximum_bytes(), work)
            .map_err(store_diagnostic)?
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_repository_idempotency_page_missing",
                    "exact derived idempotency page is absent from prepared and accepted storage",
                )
            })?;
        if observed.as_slice() != expected {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_repository_idempotency_page_bytes",
                "stored idempotency page differs from its exact derived bytes",
            ));
        }
    }
    Ok(())
}

fn require_idempotency_transition(
    observed: crate::platform::persistent_map::MapRoot,
    expected: crate::platform::persistent_map::MapRoot,
) -> Result<(), Diagnostic> {
    if observed != expected {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_idempotency_root",
            "revision idempotency root is not the exact parent root advanced by its accepted parent receipt",
        ));
    }
    Ok(())
}

fn validate_idempotency_result(
    binding: &IdempotencyBinding,
    accepted: &CurrentPublication,
) -> Result<(), Diagnostic> {
    if accepted.head != binding.result
        || accepted.revision.publication.receipt != binding.receipt
        || accepted.receipt.repository_id != binding.repository_id
        || accepted.receipt.idempotency_key.as_deref() != Some(binding.key.as_str())
        || accepted.receipt.bases.as_slice() != [binding.base]
        || accepted.receipt.transaction != binding.transaction
    {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "publication_repository_idempotency_result",
            "idempotency history binding disagrees with its exact accepted publication",
        ));
    }
    Ok(())
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
        || prepared.authority.semantic.state != prepared.accepted.semantic_state
        || prepared.authority.semantic.digest != prepared.accepted.semantic_root
        || prepared.revision.core.semantic_state != prepared.authority.semantic.state
        || prepared.revision.publication.semantic_root != prepared.authority.semantic.digest
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
        || prepared.revision.publication.transaction != prepared.transaction_digest
        || prepared.revision.publication.semantic_diff != prepared.semantic_diff_digest
        || prepared.revision.publication.receipt != prepared.receipt_digest
        || prepared.receipt.transaction != prepared.revision.publication.transaction
        || prepared.receipt.semantic_diff != prepared.revision.publication.semantic_diff
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
    let reader = ObjectPageReader::new(store);
    let mut map_work = MapWork::default();
    let _ = lookup_idempotency_history(
        prepared.revision.publication.idempotency_receipts,
        "lkjscript-integrity-probe",
        &reader,
        &mut map_work,
    )?;
    work.add(reader.work());
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

pub(super) fn resolve_package_transport_closure<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    catalog: &PackDirectoryStore,
    root_revision: crate::platform::kernel::PackageRevisionDigest,
    root_override: Option<PackageTransportBinding>,
    expected: Option<&crate::platform::kernel::DependencyRecord>,
    work: &mut StoreWork,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    resolve_package_transport_closure_admitted(
        store,
        catalog,
        root_revision,
        root_override,
        expected,
        work,
        &mut MapAdmission::unbounded(),
    )
}

pub(super) fn resolve_package_transport_closure_admitted<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    catalog: &PackDirectoryStore,
    root_revision: crate::platform::kernel::PackageRevisionDigest,
    root_override: Option<PackageTransportBinding>,
    expected: Option<&crate::platform::kernel::DependencyRecord>,
    work: &mut StoreWork,
    interface_map_admission: &mut MapAdmission,
) -> Result<PackageTransportClosureValidation, Diagnostic> {
    let logical = validate_package_revision_closure(store, root_revision, expected, work)?;
    let readiness = read_package_readiness(catalog.root())?;
    let mut selections = Vec::with_capacity(logical.revisions.len());
    for revision_digest in logical.revisions.keys() {
        let transport =
            match root_override.filter(|binding| binding.package_revision == *revision_digest) {
                Some(binding) => binding.transport,
                None => *readiness
                    .bindings
                    .get(revision_digest)
                    .ok_or_else(|| missing_package_transport(*revision_digest))?,
            };
        selections.push(PackageTransportBinding {
            package_revision: *revision_digest,
            transport,
        });
    }
    validate_package_transport_closure_admitted(
        store,
        root_revision,
        &selections,
        expected,
        work,
        interface_map_admission,
    )
}

pub(super) fn validate_prepared_dependency_sources(
    store: &PackDirectoryStore,
    prepared: &PreparedPublication,
) -> Result<(), Diagnostic> {
    use crate::platform::kernel::{PackageId, decode_dependency, decode_dependency_binding};
    use crate::platform::package_transport::source::{
        MAXIMUM_VALIDATION_READ_BYTES, MAXIMUM_VALIDATION_VISITS, collect_with_budget, entries,
        required,
    };
    let root = &prepared.authority.semantic.root;
    if root.dependencies.entries() == 0 {
        return Ok(());
    }
    let mut overlay = ObjectStage::new(store);
    let mut work = StoreWork::default();
    for (key, bytes) in &prepared.objects {
        overlay
            .stage(*key, bytes, &mut work)
            .map_err(store_diagnostic)?;
    }
    let overlay = crate::platform::package_transport::source::CollectingStore::new(&overlay);
    let mut direct = Vec::new();
    for (key, bytes) in entries(&overlay, root.dependencies)? {
        let package = key
            .as_slice()
            .try_into()
            .ok()
            .and_then(PackageId::from_bytes)
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Corrupt,
                    "publication_dependency_key",
                    "candidate dependency key is invalid",
                )
            })?;
        let binding = decode_dependency_binding(&bytes)?;
        let bytes = required(
            &overlay,
            ObjectDomain::Dependency,
            binding.object.bytes(),
            &mut work,
        )?;
        direct.push(decode_dependency(&bytes, &package, binding.object)?);
    }
    let readiness = read_package_readiness(store.root())?;
    let mut pending = std::collections::VecDeque::from(direct.clone());
    let mut union = BTreeMap::new();
    let mut edge_count = direct.len();
    while let Some(dependency) = pending.pop_front() {
        if dependency.package == root.package_id {
            return Err(repository_error(
                DiagnosticClass::Semantic,
                "package_revision_closure_package_conflict",
                "dependency closure refers back to the editable root package",
            ));
        }
        if let Some(previous) = union.get(&dependency.package) {
            if previous != &dependency {
                return Err(repository_error(
                    DiagnosticClass::Semantic,
                    "package_revision_closure_package_conflict",
                    format!(
                        "conflicting exact revisions for package {}; choose one complete closure and replan",
                        dependency.package
                    ),
                ));
            }
            continue;
        }
        if union.len() + 1 >= crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "package_revision_closure_count",
                "candidate closure exceeds the package admission including its root",
            ));
        }
        let bytes = required(
            &overlay,
            ObjectDomain::PackageRevision,
            dependency.package_revision.bytes(),
            &mut work,
        )?;
        let revision = PackageRevision::decode(&bytes, dependency.package_revision)?;
        revision.matches_dependency(dependency.package_revision, &dependency)?;
        if !readiness
            .bindings
            .contains_key(&dependency.package_revision)
        {
            return Err(missing_package_transport(dependency.package_revision));
        }
        edge_count = edge_count
            .checked_add(revision.dependencies.len())
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Resource,
                    "package_revision_closure_edge_count",
                    "candidate closure edge count overflowed",
                )
            })?;
        if edge_count > crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE_EDGES {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "package_revision_closure_edge_count",
                "candidate closure exceeds dependency edge admission",
            ));
        }
        pending.extend(revision.dependencies);
        union.insert(dependency.package, dependency);
    }
    let mut semantic_visits = 0_u64;
    let mut checked = BTreeSet::new();
    for dependency in direct {
        if checked.contains(&dependency.package_revision) {
            continue;
        }
        let closure = resolve_package_transport_closure(
            &overlay,
            store,
            dependency.package_revision,
            None,
            Some(&dependency),
            &mut work,
        )?;
        let before_reads = overlay.visits();
        let visits = MAXIMUM_VALIDATION_VISITS
            .checked_sub(before_reads)
            .and_then(|remaining| remaining.checked_sub(semantic_visits))
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Resource,
                    "package_source_budget",
                    "candidate closure exhausted aggregate validation visits",
                )
            })?;
        let read_bytes = MAXIMUM_VALIDATION_READ_BYTES - overlay.read_bytes();
        let admitted = collect_with_budget(
            &overlay,
            PackageTransportBinding {
                package_revision: dependency.package_revision,
                transport: closure.root_transport_digest,
            },
            &closure.selections,
            visits,
            read_bytes,
        )?;
        semantic_visits = semantic_visits
            .checked_add(admitted.validation_visits - (overlay.visits() - before_reads))
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Resource,
                    "package_source_budget",
                    "candidate closure aggregate validation visits overflowed",
                )
            })?;
        checked.extend(admitted.packages.keys().copied());
    }
    Ok(())
}

pub(super) fn logical_dependency_inventory<S: ImmutableObjectStore + ?Sized>(
    store: &S,
    dependencies: crate::platform::persistent_map::MapRoot,
) -> Result<BTreeMap<crate::platform::kernel::PackageId, PackageRevision>, Diagnostic> {
    use crate::platform::kernel::{PackageId, decode_dependency, decode_dependency_binding};
    use crate::platform::package_transport::source::{entries, required};
    let mut work = StoreWork::default();
    let mut pending = std::collections::VecDeque::new();
    for (key, value) in entries(store, dependencies)? {
        let package = PackageId::from_bytes(key.as_slice().try_into().map_err(|_| {
            repository_error(
                DiagnosticClass::Corrupt,
                "package_review_key",
                "invalid dependency key",
            )
        })?)
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Corrupt,
                "package_review_key",
                "zero dependency key",
            )
        })?;
        let binding = decode_dependency_binding(&value)?;
        pending.push_back(decode_dependency(
            &required(
                store,
                ObjectDomain::Dependency,
                binding.object.bytes(),
                &mut work,
            )?,
            &package,
            binding.object,
        )?);
    }
    let mut result = BTreeMap::<PackageId, PackageRevision>::new();
    let mut edges = pending.len();
    while let Some(dependency) = pending.pop_front() {
        if let Some(revision) = result.get(&dependency.package) {
            revision.matches_dependency(revision.encode()?.0, &dependency)?;
            continue;
        }
        if result.len() >= crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "package_review_count",
                "package review exceeded exact closure package limit",
            ));
        }
        let revision = PackageRevision::decode(
            &required(
                store,
                ObjectDomain::PackageRevision,
                dependency.package_revision.bytes(),
                &mut work,
            )?,
            dependency.package_revision,
        )?;
        revision.matches_dependency(dependency.package_revision, &dependency)?;
        edges = edges
            .checked_add(revision.dependencies.len())
            .filter(|edges| {
                *edges <= crate::platform::package_transport::MAXIMUM_PACKAGE_CLOSURE_EDGES
            })
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Resource,
                    "package_review_edges",
                    "package review exceeded exact closure edge limit",
                )
            })?;
        pending.extend(revision.dependencies.clone());
        result.insert(dependency.package, revision);
    }
    Ok(result)
}

fn read_package_readiness(
    root: &Path,
) -> Result<crate::platform::package_transport::source::PackageReadiness, Diagnostic> {
    let root = open_directory(root)?;
    let Some(directory) = open_optional_directory_at(
        &root,
        PACKAGE_TRANSPORT_DIRECTORY,
        "publication_package_transport_directory_open",
    )?
    else {
        return Ok(Default::default());
    };
    read_package_readiness_at(&directory)
}

fn read_package_readiness_at(
    directory: &File,
) -> Result<crate::platform::package_transport::source::PackageReadiness, Diagnostic> {
    use crate::platform::package_transport::source::{MAXIMUM_READINESS_BYTES, PackageReadiness};
    match read_optional_regular_at(
        directory,
        PACKAGE_TRANSPORT_SELECTION_FILE,
        MAXIMUM_READINESS_BYTES,
        "publication_package_readiness_read",
    )? {
        Some(bytes) => PackageReadiness::decode(&bytes),
        None => Ok(PackageReadiness::default()),
    }
}

fn install_package_container(
    root: &File,
    store: &mut PackDirectoryStore,
    digest: crate::platform::kernel::PackageTransportDigest,
    bytes: &[u8],
    fault: Option<PackageStagePoint>,
) -> Result<InstalledPackageTransport, Diagnostic> {
    use crate::platform::package_transport::source::PackageContainer;
    let container = PackageContainer::decode(bytes, digest)?;
    // Reserve the owning store's possible existing-object comparison and final durable readback
    // before semantic validation. Neither stage nor readback may reset the admission counters.
    let reserved_reads = (container.encoded_size()? as u64)
        .checked_mul(2)
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Resource,
                "package_source_budget",
                "staging validation-read reservation overflow; restage a smaller closure",
            )
        })?;
    let reserved_visits = (container.objects.len() as u64)
        .checked_mul(3)
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Resource,
                "package_source_budget",
                "staging validation-visit reservation overflow; restage a smaller closure",
            )
        })?;
    let admitted = container.admit_with_budget(
        crate::platform::package_transport::source::MAXIMUM_VALIDATION_VISITS - reserved_visits,
        crate::platform::package_transport::source::MAXIMUM_VALIDATION_READ_BYTES - reserved_reads,
    )?;
    let package = admitted
        .packages
        .get(&container.root.package_revision)
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Corrupt,
                "publication_package_source_root",
                "validated container lost its root",
            )
        })?;
    let directory = ensure_child_directory_at(
        root,
        PACKAGE_TRANSPORT_DIRECTORY,
        "publication_package_transport_directory_create",
        "publication_package_transport_directory_open",
    )?;
    let mut readiness = read_package_readiness_at(&directory)?;
    let previous_transport = readiness
        .bindings
        .get(&container.root.package_revision)
        .copied();
    for selection in &container.selections {
        readiness
            .bindings
            .insert(selection.package_revision, selection.transport);
    }
    let ready_bytes = readiness.encode()?;
    let mut work = StoreWork::default();
    let stage_input_visits = container.objects.len() as u64;
    let mut install_admission = crate::platform::storage::object::StoreReadAdmission::new(
        crate::platform::storage::object::StoreReadLimits {
            maximum_catalog_lookups: reserved_visits,
            maximum_objects: crate::platform::package_transport::source::MAXIMUM_VALIDATION_VISITS
                - admitted.validation_visits
                - stage_input_visits,
            maximum_bytes: crate::platform::package_transport::source::MAXIMUM_VALIDATION_READ_BYTES
                - admitted.validation_read_bytes,
        },
    );
    package_stage_checkpoint(fault, PackageStagePoint::BeforeObjects)?;
    for (index, (key, value)) in container.objects.iter().enumerate() {
        store
            .stage_admitted(*key, value, &mut install_admission, &mut work)
            .map_err(store_diagnostic)?;
        if index == 0 {
            package_stage_checkpoint(fault, PackageStagePoint::AfterFirstObject)?;
        }
    }
    #[cfg(test)]
    let seal = if let Some(PackageStagePoint::Storage(checkpoint)) = fault {
        store
            .seal_staged_with_fault(TARGET_PACK_BYTES, &mut work, checkpoint)
            .map_err(store_diagnostic)?
    } else {
        store
            .seal_staged(TARGET_PACK_BYTES, &mut work)
            .map_err(store_diagnostic)?
    };
    #[cfg(not(test))]
    let seal = store
        .seal_staged(TARGET_PACK_BYTES, &mut work)
        .map_err(store_diagnostic)?;
    package_stage_checkpoint(fault, PackageStagePoint::ObjectsDurable)?;
    // Re-read immutable bytes before the sole readiness visibility point. No source falls back.
    for (key, expected) in &container.objects {
        let actual = store
            .read_admitted(
                *key,
                key.domain.maximum_bytes(),
                &mut install_admission,
                &mut work,
            )
            .map_err(store_diagnostic)?;
        if actual.as_ref() != Some(expected) {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "publication_package_source_verify",
                "installed canonical source differs from validated input; restage exact source",
            ));
        }
    }
    publish_package_readiness(&directory, &ready_bytes, fault)?;
    Ok(InstalledPackageTransport {
        revision: package.revision.clone(),
        transport: package.transport.clone(),
        previous_transport,
        outcome: if previous_transport == Some(digest) {
            StageOutcome::Reused
        } else {
            StageOutcome::Inserted
        },
        seal,
        work,
        closure_packages: admitted.packages.len(),
        closure_edges: admitted.dependency_edges,
        closure_objects: container.objects.len(),
        validation_visits: admitted.validation_visits + stage_input_visits + work.objects_read,
        validation_read_bytes: admitted.validation_read_bytes + work.bytes_read,
    })
}

fn package_stage_checkpoint(
    fault: Option<PackageStagePoint>,
    point: PackageStagePoint,
) -> Result<(), Diagnostic> {
    if fault == Some(point) {
        return Err(repository_error(
            DiagnosticClass::Cancelled,
            if point == PackageStagePoint::ReadinessVisible {
                "package_stage_visible_ack_lost"
            } else {
                "package_stage_interrupted"
            },
            format!(
                "package staging interrupted at {point:?}; inspect readiness and retry the exact source container"
            ),
        ));
    }
    Ok(())
}

fn publish_package_readiness(
    directory: &File,
    bytes: &[u8],
    fault: Option<PackageStagePoint>,
) -> Result<(), Diagnostic> {
    let temporary = format!(
        "{PACKAGE_TRANSPORT_SELECTION_STAGE_PREFIX}{}",
        random_hex("publication_package_readiness_random")?
    );
    let fd = rustix::fs::openat(
        directory,
        temporary.as_str(),
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .map_err(|error| rustix_diagnostic("publication_package_readiness_stage", error))?;
    let mut file = File::from(fd);
    let result = (|| {
        file.write_all(bytes)
            .and_then(|()| file.sync_all())
            .map_err(|error| {
                io_diagnostic(
                    "publication_package_readiness_write",
                    Path::new(PACKAGE_TRANSPORT_SELECTION_FILE),
                    error,
                )
            })?;
        package_stage_checkpoint(fault, PackageStagePoint::ReadinessSynced)?;
        rustix::fs::renameat(
            directory,
            temporary.as_str(),
            directory,
            PACKAGE_TRANSPORT_SELECTION_FILE,
        )
        .map_err(|error| rustix_diagnostic("publication_package_readiness_publish", error))?;
        directory.sync_all().map_err(|error| repository_error(DiagnosticClass::Infrastructure,
            "publication_package_readiness_visible", format!("complete package readiness is visible; acknowledgement durability is indeterminate: {error}; inspect or retry staging")))?;
        package_stage_checkpoint(fault, PackageStagePoint::ReadinessVisible)?;
        Ok(())
    })();
    drop(file);
    if result.is_err() {
        let _ = rustix::fs::unlinkat(directory, temporary.as_str(), AtFlags::empty());
    }
    result
}

fn missing_package_transport(
    revision: crate::platform::kernel::PackageRevisionDigest,
) -> Diagnostic {
    repository_error(
        DiagnosticClass::Semantic,
        "package_transport_missing",
        format!(
            "no complete validated source is ready for package revision {revision}; export its exact closure, restage, and replan"
        ),
    )
}

struct InstalledPackageTransport {
    revision: PackageRevision,
    transport: PackageTransport,
    previous_transport: Option<crate::platform::kernel::PackageTransportDigest>,
    outcome: StageOutcome,
    seal: SealReceipt,
    work: StoreWork,
    closure_packages: usize,
    closure_edges: usize,
    closure_objects: usize,
    validation_visits: u64,
    validation_read_bytes: u64,
}

fn open_optional_directory_at(
    parent: &File,
    name: &str,
    code: &'static str,
) -> Result<Option<File>, Diagnostic> {
    let fd = match rustix::fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => return Err(rustix_diagnostic(code, error)),
    };
    Ok(Some(File::from(fd)))
}

fn ensure_child_directory_at(
    parent: &File,
    name: &str,
    create_code: &'static str,
    open_code: &'static str,
) -> Result<File, Diagnostic> {
    let created = match rustix::fs::mkdirat(parent, name, Mode::from_raw_mode(0o700)) {
        Ok(()) => true,
        Err(error) if error == rustix::io::Errno::EXIST => false,
        Err(error) => return Err(rustix_diagnostic(create_code, error)),
    };
    let directory = open_optional_directory_at(parent, name, open_code)?.ok_or_else(|| {
        repository_error(
            DiagnosticClass::Infrastructure,
            open_code,
            "created package transport directory is not visible",
        )
    })?;
    if created {
        parent
            .sync_all()
            .map_err(|error| io_diagnostic(create_code, Path::new(name), error))?;
    }
    Ok(directory)
}

fn initialize_unpublished(root: &Path) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(root) {
        Ok(_) => {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_repository_stage_exists",
                "private Graph 10 repository stage already exists",
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
    let _transport_directory = ensure_child_directory_at(
        &root_directory,
        PACKAGE_TRANSPORT_DIRECTORY,
        "publication_package_transport_directory_create",
        "publication_package_transport_directory_open",
    )?;
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
                "Graph 10 repository destination already exists",
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
            "Graph 10 repository destination must name one new directory",
        )
    })?;
    let parent = destination.parent().ok_or_else(|| {
        repository_error(
            DiagnosticClass::Source,
            "publication_repository_parent",
            "Graph 10 repository destination has no existing parent",
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
            "Graph 10 repository parent is not a regular non-symlink directory",
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

fn open_lock_optional(root_directory: &File) -> Result<Option<File>, Diagnostic> {
    let fd = match rustix::fs::openat(
        root_directory,
        LOCK_FILE,
        OFlags::RDWR | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    ) {
        Ok(fd) => fd,
        Err(error) if error == rustix::io::Errno::NOENT => return Ok(None),
        Err(error) => {
            return Err(rustix_diagnostic("publication_repository_lock_open", error));
        }
    };
    let file = File::from(fd);
    validate_lock(file).map(Some)
}

fn open_lock(root_directory: &File) -> Result<File, Diagnostic> {
    open_lock_optional(root_directory)?.ok_or_else(|| {
        rustix_diagnostic("publication_repository_lock_open", rustix::io::Errno::NOENT)
    })
}

fn validate_lock(file: File) -> Result<File, Diagnostic> {
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
            "Graph 10 publication lock is not an empty regular file",
        ));
    }
    Ok(file)
}

fn open_or_reconstruct_lock(root_directory: &File, root: &Path) -> Result<File, Diagnostic> {
    if let Some(lock) = open_lock_optional(root_directory)? {
        return Ok(lock);
    }
    match rustix::fs::openat(
        root_directory,
        LOCK_FILE,
        OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    ) {
        Ok(fd) => {
            let lock = validate_lock(File::from(fd))?;
            lock.sync_all()
                .map_err(|error| io_diagnostic("publication_repository_lock_sync", root, error))?;
            root_directory.sync_all().map_err(|error| {
                io_diagnostic("publication_repository_layout_sync", root, error)
            })?;
            FileExt::lock_exclusive(&lock).map_err(|error| {
                io_diagnostic("publication_repository_lock_recovery", root, error)
            })?;
            let store = open_store_exclusive(root_directory, root)?;
            let _ = read_current_optional(root_directory, &store)?.ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Source,
                    "publication_repository_unpublished",
                    "Graph 10 repository has no accepted HEAD",
                )
            })?;
            FileExt::unlock(&lock).map_err(|error| {
                io_diagnostic("publication_repository_lock_recovery_release", root, error)
            })?;
            Ok(lock)
        }
        Err(error) if error == rustix::io::Errno::EXIST => open_lock(root_directory),
        Err(error) => Err(rustix_diagnostic(
            "publication_repository_lock_create",
            error,
        )),
    }
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
            "Graph 10 preparation failed without a diagnostic",
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
