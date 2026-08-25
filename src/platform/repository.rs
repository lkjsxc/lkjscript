//! Crash-consistent packed semantic repository with one atomic visibility point.

use super::artifact::{
    ArtifactReceipt, MAXIMUM_ARTIFACT_PACKAGES, build_artifact_from_objects, decode_package_object,
    encode_package_object, load_artifact, load_package_object_closure,
};
use super::contract::registry::{
    BACKUP_DIGEST_DOMAIN, BACKUP_ENTRY_DIGEST_DOMAIN, BACKUP_MAGIC, BACKUP_SEGMENT_DIGEST_DOMAIN,
    BACKUP_SEGMENT_MAGIC, BACKUP_SEGMENT_REFERENCE_DIGEST_DOMAIN, CLEANUP_CANDIDATE_DIGEST_DOMAIN,
    CLEANUP_PLAN_DIGEST_DOMAIN,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::{GraphRoot, StoredGraphRoot, StoredGraphRootDelta, StoredGraphRootUpdate};
use super::meaning::{GRAPH_CONTRACT_VERSION, MeaningModule};
use super::packed;
use super::persistent_map::{
    MapError, MapErrorClass, MapWork, MemoryPageStore, PageDigest, PageStore, PageWrite,
    PersistentMap,
};
use super::revision::{
    AffectedOwner, ParentRevision, RECEIPT_CONTRACT_VERSION, REVISION_CONTRACT_VERSION,
    ReceiptStatus, RevisionCore, RevisionRecord, SemanticHead, TransactionReceipt, ValidationFacts,
};
use super::semantic::{
    ExactGraphDependency, ValidatedPackage, canonicalize_graph_package, validate_graph_package,
};
use super::semantic_diff::semantic_diff_digest;
use super::semantic_digest::{
    ArtifactDigest, BackupDigest, CleanupDigest, IndexDigest, ModuleObjectDigest, ReceiptDigest,
    RevisionRecordDigest, RootObjectDigest, SemanticCertificateDigest, SemanticDiffDigest,
    TransactionDigest,
};
use super::semantic_draft::{DraftRecord, MAXIMUM_DRAFT_BYTES, SemanticDraftStore};
use super::semantic_fact::{
    MAXIMUM_SEMANTIC_FACT_MANIFEST_BYTES, SemanticFactManifest, SemanticFactUpdate,
    build_semantic_certificate, build_semantic_facts, update_semantic_facts,
};
use super::semantic_id::{DraftId, ModuleId, RepositoryId, RevisionId};
use super::semantic_query::{PreparedLocalIndexDelta, SemanticQueryIndex};
use super::semantic_summary::{
    MAXIMUM_MODULE_SUMMARY_ENCODED_BYTES, ModuleSemanticSummary, SemanticSummaryDigest,
    build_module_summary,
};
use bincode::{Decode, Encode};
use fs2::FileExt;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const SEMANTIC_STORE_RELATIVE: &str = ".lkjscript/meaning";
pub const MAXIMUM_HEAD_BYTES: usize = 4_096;
pub const MAXIMUM_HISTORY_ITEMS: usize = 10_000;
pub const MAXIMUM_ARTIFACT_OBJECT_BYTES: usize = 256 * 1_048_576;
pub const BACKUP_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_BACKUP_MANIFEST_BYTES: usize = 32 * 1_048_576;
pub const MAXIMUM_BACKUP_SEGMENT_BYTES: usize = 4 * 1_048_576;
pub const BACKUP_SEGMENT_ENTRY_LIMIT: usize = 4_096;
pub const RETENTION_CONTRACT_VERSION: u16 = 1;

const BACKUP_MANIFEST_FILE: &str = "MANIFEST.lkjb";
const BACKUP_SEGMENTS: &str = "segments";

const HEAD_FILE: &str = "HEAD";
const LOCK_FILE: &str = "LOCK";
const MODULE_OBJECTS: &str = "objects/modules";
const ROOT_OBJECTS: &str = "objects/roots";
const MAP_PAGE_OBJECTS: &str = "objects/map-pages";
const REVISION_OBJECTS: &str = "revisions";
const RECEIPT_OBJECTS: &str = "receipts";
const ARTIFACT_OBJECTS: &str = "artifacts";
const DRAFT_OBJECTS: &str = "drafts";
const INDEX_OBJECTS: &str = "indexes";
const SEMANTIC_INDEX_OBJECTS: &str = "indexes/semantic";
const SEMANTIC_FACT_PAGE_OBJECTS: &str = "indexes/semantic/pages";
const SUMMARY_INDEX_OBJECTS: &str = "indexes/semantic/summaries";
const LOCAL_INDEX_OWNER_OBJECTS: &str = "indexes/local-objects/owners";
const LOCAL_INDEX_NAME_OBJECTS: &str = "indexes/local-objects/names";

#[derive(Clone, Copy, Debug)]
pub(crate) enum DisposableIndexPart {
    Manifest,
    SemanticFacts,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LocalIndexObjectKind {
    Owner,
    Name,
}

#[derive(Clone, Debug)]
pub struct SemanticRepository {
    project_root: PathBuf,
    store: PathBuf,
}

struct RepositoryPageStore<'a> {
    store: &'a Path,
    publication_writes: bool,
}

impl<'a> RepositoryPageStore<'a> {
    const fn read_only(store: &'a Path) -> Self {
        Self {
            store,
            publication_writes: false,
        }
    }

    const fn writer(store: &'a Path, publication_writes: bool) -> Self {
        Self {
            store,
            publication_writes,
        }
    }
}

impl PageStore for RepositoryPageStore<'_> {
    fn read_page(
        &self,
        digest: PageDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, MapError> {
        let path = map_page_path(self.store, digest);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(repository_map_error(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_map_page_type",
                        format!(
                            "persistent root page '{}' is not a regular non-symlink file",
                            path.display()
                        ),
                    )));
                }
                read_bounded(
                    &path,
                    maximum_bytes.min(super::persistent_map::MAXIMUM_PAGE_BYTES),
                    "persistent root page",
                )
                .map(Some)
                .map_err(repository_map_error)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(repository_map_error(io_error(
                "repository_map_page_metadata",
                &path,
                error,
            ))),
        }
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        if bytes.len() > super::persistent_map::MAXIMUM_PAGE_BYTES
            || PageDigest::of(bytes) != digest
        {
            return Err(MapError {
                class: MapErrorClass::Corrupt,
                code: "repository_map_page_digest",
                message: "persistent root page bytes do not match their exact key".to_owned(),
            });
        }
        let path = map_page_path(self.store, digest);
        let existed = fs::symlink_metadata(&path).is_ok();
        let result = if self.publication_writes {
            write_publication_immutable(&path, bytes, "persistent root page")
        } else {
            write_immutable(&path, bytes, "persistent root page")
        };
        result.map_err(repository_map_error)?;
        Ok(if existed {
            PageWrite::Reused
        } else {
            PageWrite::Inserted
        })
    }
}

struct SemanticFactPageStore<'a> {
    store: &'a Path,
}

impl SemanticFactPageStore<'_> {
    const fn new(store: &Path) -> SemanticFactPageStore<'_> {
        SemanticFactPageStore { store }
    }
}

impl PageStore for SemanticFactPageStore<'_> {
    fn read_page(
        &self,
        digest: PageDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, MapError> {
        let path = semantic_fact_page_path(self.store, digest);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(repository_map_error(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_semantic_page_type",
                        format!(
                            "derived semantic fact page '{}' is not a regular non-symlink file",
                            path.display()
                        ),
                    )));
                }
                read_bounded(
                    &path,
                    maximum_bytes.min(super::persistent_map::MAXIMUM_PAGE_BYTES),
                    "derived semantic fact page",
                )
                .map(Some)
                .map_err(repository_map_error)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(repository_map_error(io_error(
                "repository_semantic_page_metadata",
                &path,
                error,
            ))),
        }
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        if bytes.len() > super::persistent_map::MAXIMUM_PAGE_BYTES
            || PageDigest::of(bytes) != digest
        {
            return Err(MapError {
                class: MapErrorClass::Corrupt,
                code: "repository_semantic_page_digest",
                message: "derived semantic page bytes do not match their content key".to_owned(),
            });
        }
        let path = semantic_fact_page_path(self.store, digest);
        write_disposable_content(&path, bytes, "derived semantic fact page")
            .map_err(repository_map_error)
    }
}

#[derive(Default)]
struct BackupPageCollector {
    pages: BTreeSet<PageDigest>,
}

impl PageStore for BackupPageCollector {
    fn read_page(
        &self,
        _digest: PageDigest,
        _maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, MapError> {
        Ok(None)
    }

    fn write_page(&mut self, digest: PageDigest, bytes: &[u8]) -> Result<PageWrite, MapError> {
        if bytes.len() > super::persistent_map::MAXIMUM_PAGE_BYTES
            || PageDigest::of(bytes) != digest
        {
            return Err(MapError {
                class: MapErrorClass::Corrupt,
                code: "semantic_backup_map_page_digest",
                message: "reachable map page bytes do not match their exact key".to_owned(),
            });
        }
        Ok(if self.pages.insert(digest) {
            PageWrite::Inserted
        } else {
            PageWrite::Reused
        })
    }
}

#[derive(Clone, Debug)]
pub struct CurrentBinding {
    pub head: SemanticHead,
    pub record: RevisionRecord,
    pub receipt: TransactionReceipt,
    pub stored_root: StoredGraphRoot,
}

#[derive(Clone, Debug)]
pub struct CurrentRevision {
    pub head: SemanticHead,
    pub record: RevisionRecord,
    pub receipt: TransactionReceipt,
    pub stored_root: StoredGraphRoot,
    pub root: GraphRoot,
}

#[derive(Clone, Debug)]
pub struct ReconstructedRevision {
    pub current: CurrentRevision,
    pub modules: Vec<MeaningModule>,
}

#[derive(Clone, Debug)]
pub struct RevisionSnapshot {
    pub record: RevisionRecord,
    pub receipt: TransactionReceipt,
    pub root: GraphRoot,
    pub modules: Vec<MeaningModule>,
}

#[derive(Clone, Debug)]
pub struct InitialPublication {
    pub root: GraphRoot,
    pub modules: Vec<MeaningModule>,
    pub transaction: TransactionDigest,
    pub semantic_diff: SemanticDiffDigest,
    pub intent: Option<String>,
    pub validation_profile: Option<String>,
    pub dependency_artifacts: Vec<DependencyArtifactObject>,
    pub status: ReceiptStatus,
}

#[derive(Clone, Debug)]
pub(crate) struct PreparedValidation {
    expected_base: RevisionId,
    base_root: RootObjectDigest,
    result_root: RootObjectDigest,
    stored_update: StoredGraphRootUpdate,
    changed_modules: Vec<MeaningModule>,
    semantic_facts: SemanticFactUpdate,
    semantic_summaries: Vec<ModuleSemanticSummary>,
    local_index_delta: Option<PreparedLocalIndexDelta>,
    facts: ValidationFacts,
}

impl PreparedValidation {
    pub(crate) fn result_root(&self) -> RootObjectDigest {
        self.result_root
    }

    pub(crate) fn semantic_certificate(&self) -> SemanticCertificateDigest {
        self.semantic_facts.manifest.certificate
    }
}

#[derive(Clone, Debug)]
pub(crate) struct PublicationProposal {
    pub expected_base: RevisionId,
    pub repository_id: RepositoryId,
    pub root: Option<GraphRoot>,
    pub modules: Vec<MeaningModule>,
    pub transaction: TransactionDigest,
    pub idempotency_key: Option<String>,
    pub semantic_diff: SemanticDiffDigest,
    pub status: ReceiptStatus,
    pub affected_owners: Vec<AffectedOwner>,
    pub intent: Option<String>,
    pub dependency_artifacts: Vec<DependencyArtifactObject>,
    pub(crate) prepared_validation: Option<PreparedValidation>,
}

#[derive(Clone, Debug)]
pub struct DependencyArtifactObject {
    pub digest: ArtifactDigest,
    pub bytes: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PublicationOutcome {
    Accepted {
        revision: RevisionId,
        record: RevisionRecordDigest,
        receipt: ReceiptDigest,
    },
    SemanticNoChange {
        revision: RevisionId,
        record: RevisionRecordDigest,
    },
    StaleBase {
        requested: RevisionId,
        current: RevisionId,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorReport {
    pub valid: bool,
    pub deep: bool,
    pub revision: RevisionId,
    pub revisions_checked: usize,
    pub roots_checked: usize,
    pub modules_checked: usize,
    pub receipts_checked: usize,
    pub rebuilt_indexes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReceipt {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub digest: BackupDigest,
    pub segments: usize,
    pub entries: u64,
    pub drafts: u64,
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreReceipt {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub digest: BackupDigest,
    pub segments: usize,
    pub entries: u64,
    pub drafts: u64,
    pub deep_valid: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RetentionReport {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub policy: &'static str,
    pub retained_revisions: u64,
    pub retained_drafts: u64,
    pub retained_objects: u64,
    pub reclaimable_objects: u64,
    pub reclaimable_bytes: u64,
    pub unknown_entries: u64,
    pub derived_objects: u64,
    pub derived_bytes: u64,
    pub plan: CleanupDigest,
    pub destructive_ready: bool,
    pub missing_authority: Vec<&'static str>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum BackupEntryKey {
    Module(ModuleObjectDigest),
    MapPage(PageDigest),
    Root(RootObjectDigest),
    Revision(RevisionId),
    Receipt(ReceiptDigest),
    Artifact(ArtifactDigest),
    Draft(DraftId),
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct BackupEntry {
    key: BackupEntryKey,
    bytes: u64,
    digest: [u8; 32],
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct BackupSegment {
    contract_version: u16,
    ordinal: u64,
    entries: Vec<BackupEntry>,
}

impl BackupSegment {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_entries()?;
        packed::encode(
            BACKUP_SEGMENT_MAGIC,
            BACKUP_SEGMENT_DIGEST_DOMAIN,
            self,
            MAXIMUM_BACKUP_SEGMENT_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            BACKUP_SEGMENT_MAGIC,
            BACKUP_SEGMENT_DIGEST_DOMAIN,
            MAXIMUM_BACKUP_SEGMENT_BYTES,
        )?;
        value.validate_entries()?;
        Ok(value)
    }

    fn validate_entries(&self) -> Result<(), Diagnostic> {
        if self.contract_version != BACKUP_CONTRACT_VERSION {
            return Err(repository_error(
                DiagnosticClass::Source,
                "semantic_backup_contract",
                "backup segment uses an unknown contract",
            ));
        }
        if self.entries.is_empty() || self.entries.len() > BACKUP_SEGMENT_ENTRY_LIMIT {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "semantic_backup_entry_limit",
                format!(
                    "backup segment must contain 1 through {BACKUP_SEGMENT_ENTRY_LIMIT} entries"
                ),
            ));
        }
        if self
            .entries
            .windows(2)
            .any(|pair| pair[0].key >= pair[1].key)
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "semantic_backup_entry_order",
                "backup entries are not uniquely and canonically ordered",
            ));
        }
        if self.entries.iter().any(|entry| entry.bytes == 0) {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "semantic_backup_entry_length",
                "backup entries must bind a nonempty canonical object",
            ));
        }
        Ok(())
    }
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct BackupSegmentReference {
    ordinal: u64,
    digest: [u8; 32],
    entries: u32,
    encoded_bytes: u64,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct BackupManifest {
    contract_version: u16,
    repository_id: RepositoryId,
    head: SemanticHead,
    segments: Vec<BackupSegmentReference>,
    entries: u64,
    drafts: u64,
    payload_bytes: u64,
}

impl BackupManifest {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate()?;
        packed::encode(
            BACKUP_MAGIC,
            BACKUP_DIGEST_DOMAIN,
            self,
            MAXIMUM_BACKUP_MANIFEST_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            BACKUP_MAGIC,
            BACKUP_DIGEST_DOMAIN,
            MAXIMUM_BACKUP_MANIFEST_BYTES,
        )?;
        value.validate()?;
        Ok(value)
    }

    fn validate(&self) -> Result<(), Diagnostic> {
        if self.contract_version != BACKUP_CONTRACT_VERSION
            || self.head.repository_id != self.repository_id
        {
            return Err(repository_error(
                DiagnosticClass::Source,
                "semantic_backup_contract",
                "backup uses an unknown contract or inconsistent repository identity",
            ));
        }
        if self.segments.is_empty() || self.entries == 0 || self.payload_bytes == 0 {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "semantic_backup_manifest_empty",
                "backup manifest must describe at least one segment and canonical object",
            ));
        }
        let mut entries = 0_u64;
        for (index, segment) in self.segments.iter().enumerate() {
            let ordinal = u64::try_from(index).map_err(|_| count_overflow("backup segment"))?;
            if segment.ordinal != ordinal
                || segment.entries == 0
                || usize::try_from(segment.entries)
                    .ok()
                    .is_none_or(|count| count > BACKUP_SEGMENT_ENTRY_LIMIT)
                || segment.encoded_bytes == 0
                || segment.encoded_bytes
                    > u64::try_from(MAXIMUM_BACKUP_SEGMENT_BYTES)
                        .map_err(|_| count_overflow("backup segment bytes"))?
            {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_segment_reference",
                    "backup segment references are not canonical and consecutively ordered",
                ));
            }
            entries = entries
                .checked_add(u64::from(segment.entries))
                .ok_or_else(|| count_overflow("backup entry"))?;
        }
        if entries != self.entries || self.drafts > self.entries {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "semantic_backup_manifest_count",
                "backup manifest aggregate counts disagree with its segment references",
            ));
        }
        Ok(())
    }
}

impl SemanticRepository {
    pub fn discover(start: &Path) -> Result<PathBuf, Diagnostic> {
        let mut current = canonical_existing(start)?;
        if current.is_file() {
            current.pop();
        }
        loop {
            let store = current.join(SEMANTIC_STORE_RELATIVE);
            if store.join(HEAD_FILE).is_file() {
                ensure_directory(&store, "repository_store_type")?;
                return Ok(current);
            }
            if current.join(".lkjscript/source-v1/HEAD.json").exists()
                || current.join("lkjscript.package.json").exists()
            {
                return Err(repository_error(
                    DiagnosticClass::Source,
                    "semantic_predecessor_source_rejected",
                    format!(
                        "'{}' contains source-authored predecessor authority but no current meaning graph",
                        current.display()
                    ),
                ));
            }
            if !current.pop() {
                return Err(repository_error(
                    DiagnosticClass::Source,
                    "semantic_repository_not_found",
                    format!(
                        "no current meaning graph was found from '{}' to the filesystem root",
                        start.display()
                    ),
                ));
            }
        }
    }

    pub fn open(start: &Path) -> Result<Self, Diagnostic> {
        let project_root = Self::discover(start)?;
        let store = project_root.join(SEMANTIC_STORE_RELATIVE);
        let repository = Self {
            project_root,
            store,
        };
        repository.current_binding()?;
        repository.ensure_operational_layout()?;
        Ok(repository)
    }

    /// Recreates only disposable and coordination state omitted by graph transport. Canonical
    /// objects and HEAD are validated before this is called and are never created or changed here.
    fn ensure_operational_layout(&self) -> Result<(), Diagnostic> {
        ensure_directory(&self.store, "repository_store_type")?;
        ensure_or_create_directory(
            &self.store.join(DRAFT_OBJECTS),
            "repository_draft_directory",
        )?;
        ensure_or_create_directory(
            &self.store.join(INDEX_OBJECTS),
            "repository_index_directory",
        )?;
        ensure_or_create_empty_file(&self.store.join(LOCK_FILE), "repository_lock_file")?;
        sync_directory(&self.store)
            .map_err(|error| io_error("repository_operational_layout_sync", &self.store, error))
    }

    pub fn initialize(
        project_root: &Path,
        initial: InitialPublication,
    ) -> Result<(Self, TransactionReceipt), Diagnostic> {
        let project_root = canonical_existing(project_root)?;
        ensure_directory(&project_root, "repository_project_type")?;
        initial.root.validate_modules(&initial.modules)?;
        if initial.root.repository_id.bytes() == [0; 16] {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_identity_zero",
                "repository identity may not be all zeroes",
            ));
        }
        let semantic_parent = project_root.join(".lkjscript");
        ensure_or_create_directory(&semantic_parent, "repository_parent")?;
        let store = project_root.join(SEMANTIC_STORE_RELATIVE);
        if fs::symlink_metadata(&store).is_ok() {
            return Err(repository_error(
                DiagnosticClass::Source,
                "semantic_repository_exists",
                format!(
                    "current meaning authority already exists at '{}'",
                    store.display()
                ),
            ));
        }
        let stage_identity = RepositoryId::generate()?;
        let stage = semantic_parent.join(format!(".meaning-stage-{}", stage_identity));
        create_store_layout(&stage)?;
        let initialized = initialize_stage(&stage, initial);
        let (head, receipt) = match initialized {
            Ok(value) => value,
            Err(error) => {
                remove_owned_stage(&stage);
                return Err(error);
            }
        };
        if let Err(error) = fs::rename(&stage, &store) {
            remove_owned_stage(&stage);
            return Err(io_error("repository_publish_initial", &store, error));
        }
        sync_directory(&semantic_parent).map_err(|error| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_visibility_indeterminate",
                format!(
                    "initial HEAD rename succeeded but parent durability is indeterminate: {error}"
                ),
            )
        })?;
        let repository = Self {
            project_root,
            store,
        };
        let observed = repository.current_binding()?;
        if observed.head != head {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "repository_initial_reconcile",
                "published initial HEAD does not match the visible repository",
            ));
        }
        Ok((repository, receipt))
    }

    pub fn project_root(&self) -> &Path {
        &self.project_root
    }

    pub(crate) fn store_path(&self) -> &Path {
        &self.store
    }

    pub fn current(&self) -> Result<CurrentRevision, Diagnostic> {
        let binding = self.current_binding()?;
        let root = binding.stored_root.reconstruct(
            &RepositoryPageStore::read_only(&self.store),
            &mut MapWork::default(),
        )?;
        Ok(CurrentRevision {
            head: binding.head,
            record: binding.record,
            receipt: binding.receipt,
            stored_root: binding.stored_root,
            root,
        })
    }

    /// Verifies the visible authority chain without traversing any persistent collection page.
    /// Exact lookups and revision-pinned operations use this path so repository size does not
    /// become an implicit cost of opening a project.
    pub fn current_binding(&self) -> Result<CurrentBinding, Diagnostic> {
        let head = self.read_head()?;
        let record = self.read_revision(head.revision)?;
        if record.digest()? != head.record
            || record.revision != head.revision
            || record.core.repository_id != head.repository_id
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_head_binding",
                "HEAD does not bind the exact visible revision record",
            ));
        }
        let receipt = self.read_receipt(record.receipt)?;
        if receipt.result != record.revision
            || receipt.repository_id != head.repository_id
            || receipt.semantic_diff != record.core.semantic_diff
            || receipt.transaction != record.core.transaction
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_receipt_binding",
                "revision record and transaction receipt disagree",
            ));
        }
        let stored_root = self.read_stored_root(record.core.root)?;
        if stored_root.repository_id != head.repository_id {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_root_identity",
                "graph root belongs to a foreign repository identity",
            ));
        }
        Ok(CurrentBinding {
            head,
            record,
            receipt,
            stored_root,
        })
    }

    pub fn reconstruct_current(&self) -> Result<ReconstructedRevision, Diagnostic> {
        let current = self.current()?;
        let modules = self.read_modules(&current.root)?;
        current.root.validate_modules(&modules)?;
        Ok(ReconstructedRevision { current, modules })
    }

    pub fn reconstruct_revision(
        &self,
        revision: RevisionId,
    ) -> Result<RevisionSnapshot, Diagnostic> {
        let record = self.read_revision(revision)?;
        let receipt = self.read_receipt(record.receipt)?;
        if receipt.result != revision
            || receipt.repository_id != record.core.repository_id
            || receipt.transaction != record.core.transaction
            || receipt.semantic_diff != record.core.semantic_diff
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_revision_receipt_binding",
                "requested revision and its receipt disagree",
            ));
        }
        let root = self.read_root(record.core.root)?;
        let modules = self.read_modules(&root)?;
        root.validate_modules(&modules)?;
        Ok(RevisionSnapshot {
            record,
            receipt,
            root,
            modules,
        })
    }

    /// Creates a new canonical repository from the exact semantic snapshot in a graph-native
    /// artifact. This is new authority with a history-free initial revision: artifact history is
    /// deliberately not imported, while repository and semantic owner identities are preserved.
    /// Predecessor source and predecessor artifact contracts reject.
    pub fn initialize_from_artifact(
        project_root: &Path,
        artifact_bytes: &[u8],
    ) -> Result<(Self, TransactionReceipt), Diagnostic> {
        let loaded = load_artifact(artifact_bytes)?;
        let root_bytes = loaded
            .package_object(loaded.root_package_artifact)
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_import_root_object",
                    "graph artifact omits its root package object",
                )
            })?;
        let root_object = decode_package_object(root_bytes)?;
        let dependency_artifacts = loaded
            .package_objects
            .iter()
            .filter(|(digest, _)| **digest != loaded.root_package_artifact)
            .map(|(digest, bytes)| DependencyArtifactObject {
                digest: *digest,
                bytes: bytes.clone(),
            })
            .collect();
        Self::initialize(
            project_root,
            InitialPublication {
                root: root_object.root,
                modules: root_object.modules,
                transaction: root_object.revision.core.transaction,
                semantic_diff: root_object.revision.core.semantic_diff,
                intent: root_object.receipt.intent,
                validation_profile: Some(root_object.receipt.validation.profile),
                dependency_artifacts,
                status: ReceiptStatus::ImportAccepted,
            },
        )
    }

    /// Builds the deterministic executable package closure from the exact visible graph
    /// revision. No maintained text is rendered or parsed on this path.
    pub fn build_artifact(&self) -> Result<(Vec<u8>, ArtifactReceipt), Diagnostic> {
        let reconstructed = self.reconstruct_current()?;
        let (root_digest, root_bytes) = encode_package_object(
            reconstructed.current.record.clone(),
            reconstructed.current.receipt.clone(),
            reconstructed.current.root.clone(),
            reconstructed.modules,
        )?;
        let mut objects = BTreeMap::new();
        objects.insert(root_digest, root_bytes);
        for binding in &reconstructed.current.root.dependencies {
            let loaded = load_stored_artifact_closure(&self.store, binding.artifact)?;
            for (digest, bytes) in loaded.package_objects {
                match objects.get(&digest) {
                    Some(existing) if existing != &bytes => {
                        return Err(repository_error(
                            DiagnosticClass::Corrupt,
                            "repository_artifact_object_conflict",
                            "two dependency closures bind one artifact key to different bytes",
                        ));
                    }
                    Some(_) => {}
                    None => {
                        objects.insert(digest, bytes);
                    }
                }
            }
        }
        build_artifact_from_objects(root_digest, objects)
    }

    pub(crate) fn canonicalize_proposal(
        &self,
        expected_base: RevisionId,
        base: &GraphRoot,
        base_stored: &StoredGraphRoot,
        root: &mut GraphRoot,
        modules: &mut [MeaningModule],
    ) -> Result<PreparedValidation, Diagnostic> {
        let dependencies = root.dependencies.clone();
        let loaded = dependencies
            .iter()
            .map(|binding| load_stored_artifact_closure(&self.store, binding.artifact))
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let exact = dependencies
            .iter()
            .zip(&loaded)
            .map(|(binding, artifact)| {
                let package = artifact.packages.get(&binding.package_id).ok_or_else(|| {
                    repository_error(
                        DiagnosticClass::Semantic,
                        "repository_dependency_package_missing",
                        format!(
                            "dependency artifact '{}' does not contain package '{}'",
                            binding.alias,
                            binding.package_id.as_str()
                        ),
                    )
                })?;
                Ok(ExactGraphDependency {
                    alias: &binding.alias,
                    package,
                    artifact: binding.artifact,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let validated = canonicalize_graph_package(root, modules, &exact)?;
        let delta = StoredGraphRootDelta::between(base, root)?;
        let stored_update =
            base_stored.apply_delta(&RepositoryPageStore::read_only(&self.store), &delta)?;
        let result_root = stored_update.root.digest()?;
        let base_root = base_stored.digest()?;
        let changed_modules = delta
            .module_upserts
            .iter()
            .map(|reference| {
                let module = modules
                    .iter()
                    .find(|module| module.module_id == reference.id)
                    .ok_or_else(|| {
                        repository_error(
                            DiagnosticClass::Infrastructure,
                            "repository_prepared_module_missing",
                            "prepared root delta lost one changed semantic module",
                        )
                    })?;
                if module.digest()? != reference.object {
                    return Err(repository_error(
                        DiagnosticClass::Infrastructure,
                        "repository_prepared_module_digest",
                        "prepared changed module does not bind its root object digest",
                    ));
                }
                Ok(module.clone())
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let semantic_summaries = build_semantic_summaries(&root.package_id, modules)?;
        let semantic_facts = build_semantic_facts(
            root.repository_id,
            &root.package_id,
            RevisionId::from_digest([0; 32]),
            result_root,
            &semantic_summaries,
        )?;
        let mut facts = validation_facts(&validated)?;
        facts.profile = "prepared_once_full_oracle".to_owned();
        Ok(PreparedValidation {
            expected_base,
            base_root,
            result_root,
            stored_update,
            changed_modules,
            semantic_facts,
            semantic_summaries,
            local_index_delta: None,
            facts,
        })
    }

    pub(crate) fn canonicalize_slice(
        &self,
        root: &mut GraphRoot,
        modules: &mut [MeaningModule],
    ) -> Result<ValidatedPackage, Diagnostic> {
        let dependencies = root.dependencies.clone();
        let loaded = dependencies
            .iter()
            .map(|binding| load_stored_artifact_closure(&self.store, binding.artifact))
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let exact = dependencies
            .iter()
            .zip(&loaded)
            .map(|(binding, artifact)| {
                let package = artifact.packages.get(&binding.package_id).ok_or_else(|| {
                    repository_error(
                        DiagnosticClass::Semantic,
                        "repository_dependency_package_missing",
                        format!(
                            "dependency artifact '{}' does not contain package '{}'",
                            binding.alias,
                            binding.package_id.as_str()
                        ),
                    )
                })?;
                Ok(ExactGraphDependency {
                    alias: &binding.alias,
                    package,
                    artifact: binding.artifact,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        canonicalize_graph_package(root, modules, &exact)
    }

    pub(crate) fn prepare_local_validation(
        &self,
        current: &CurrentBinding,
        delta: StoredGraphRootDelta,
        changed_modules: Vec<MeaningModule>,
        validated: &ValidatedPackage,
        profile: &str,
    ) -> Result<PreparedValidation, Diagnostic> {
        let update = current
            .stored_root
            .apply_delta(&RepositoryPageStore::read_only(&self.store), &delta)?;
        let result_root = update.root.digest()?;
        let expected = delta
            .module_upserts
            .iter()
            .map(|reference| (reference.id, reference.object))
            .collect::<BTreeMap<_, _>>();
        let observed = changed_modules
            .iter()
            .map(|module| Ok((module.module_id, module.digest()?)))
            .collect::<Result<BTreeMap<_, _>, Diagnostic>>()?;
        if expected != observed {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "repository_local_preparation_modules",
                "local preparation modules do not equal the persistent-root delta",
            ));
        }
        let local_index_delta = SemanticQueryIndex::prepare_local_index_delta(
            self,
            current,
            &delta,
            &changed_modules,
            result_root,
        )
        .ok()
        .flatten();
        let semantic_summaries =
            build_semantic_summaries(&current.stored_root.package_id, &changed_modules)?;
        let semantic_before = delta
            .module_removals
            .iter()
            .map(|reference| {
                let module = self.read_module(reference.object)?;
                if module.module_id != reference.id {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_semantic_before_owner",
                        "base module object does not match its persistent-root identity",
                    ));
                }
                build_module_summary(&current.stored_root.package_id, &module)
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let (base_semantic_facts, _) = self.load_or_rebuild_semantic_facts(current)?;
        let semantic_facts = update_semantic_facts(
            &base_semantic_facts,
            &SemanticFactPageStore::new(&self.store),
            RevisionId::from_digest([0; 32]),
            result_root,
            &semantic_before,
            &semantic_summaries,
        )?;
        let mut facts = validation_facts(validated)?;
        facts.profile = profile.to_owned();
        Ok(PreparedValidation {
            expected_base: current.head.revision,
            base_root: current.record.core.root,
            result_root,
            stored_update: update,
            changed_modules,
            semantic_facts,
            semantic_summaries,
            local_index_delta,
            facts,
        })
    }

    pub(crate) fn semantic_certificate_for_modules(
        package: &super::package::PackageId,
        modules: &[MeaningModule],
    ) -> Result<SemanticCertificateDigest, Diagnostic> {
        let summaries = build_semantic_summaries(package, modules)?;
        build_semantic_certificate(&summaries)
    }

    pub(crate) fn dependency_binding_by_package(
        &self,
        current: &CurrentBinding,
        package: &super::package::PackageId,
    ) -> Result<Option<super::graph::DependencyBinding>, Diagnostic> {
        current.stored_root.dependency_by_package(
            &RepositoryPageStore::read_only(&self.store),
            package,
            &mut MapWork::default(),
        )
    }

    pub(crate) fn module_reference_by_id(
        &self,
        current: &CurrentBinding,
        id: ModuleId,
    ) -> Result<Option<super::graph::ModuleObjectRef>, Diagnostic> {
        current.stored_root.module_by_id(
            &RepositoryPageStore::read_only(&self.store),
            id,
            &mut MapWork::default(),
        )
    }

    pub(crate) fn tombstone_by_identity(
        &self,
        current: &CurrentBinding,
        identity: &super::graph::TombstoneIdentity,
    ) -> Result<Option<super::graph::Tombstone>, Diagnostic> {
        current.stored_root.tombstone_by_identity(
            &RepositoryPageStore::read_only(&self.store),
            identity,
            &mut MapWork::default(),
        )
    }

    pub(crate) fn module_reference_by_name(
        &self,
        current: &CurrentBinding,
        name: &str,
    ) -> Result<Option<super::graph::ModuleObjectRef>, Diagnostic> {
        current.stored_root.module_by_name(
            &RepositoryPageStore::read_only(&self.store),
            name,
            &mut MapWork::default(),
        )
    }

    pub fn module_by_id(
        &self,
        revision: RevisionId,
        module_id: ModuleId,
    ) -> Result<MeaningModule, Diagnostic> {
        let record = self.read_revision(revision)?;
        let root = self.read_stored_root(record.core.root)?;
        let reference = root
            .module_by_id(
                &RepositoryPageStore::read_only(&self.store),
                module_id,
                &mut MapWork::default(),
            )?
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Source,
                    "repository_module_missing",
                    format!("revision {revision} has no module {module_id}"),
                )
            })?;
        self.read_module(reference.object)
    }

    pub fn module_by_name(
        &self,
        revision: RevisionId,
        name: &str,
    ) -> Result<MeaningModule, Diagnostic> {
        let record = self.read_revision(revision)?;
        let root = self.read_stored_root(record.core.root)?;
        let reference = root
            .module_by_name(
                &RepositoryPageStore::read_only(&self.store),
                name,
                &mut MapWork::default(),
            )?
            .ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Source,
                    "repository_module_missing",
                    format!("revision {revision} has no module '{name}'"),
                )
            })?;
        self.read_module(reference.object)
    }

    pub fn read_revision(&self, revision: RevisionId) -> Result<RevisionRecord, Diagnostic> {
        let path = revision_path(&self.store, revision);
        RevisionRecord::decode(&read_bounded(
            &path,
            super::revision::MAXIMUM_REVISION_BYTES + 50,
            "revision record",
        )?)
    }

    pub fn read_receipt(&self, digest: ReceiptDigest) -> Result<TransactionReceipt, Diagnostic> {
        let path = receipt_path(&self.store, digest);
        let bytes = read_bounded(
            &path,
            super::revision::MAXIMUM_RECEIPT_BYTES + 50,
            "transaction receipt",
        )?;
        if ReceiptDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_receipt_digest",
                "receipt bytes do not match their physical key",
            ));
        }
        TransactionReceipt::decode(&bytes)
    }

    pub fn read_stored_root(
        &self,
        digest: RootObjectDigest,
    ) -> Result<StoredGraphRoot, Diagnostic> {
        let path = root_path(&self.store, digest);
        let bytes = read_bounded(
            &path,
            super::graph::MAXIMUM_STORED_ROOT_BYTES + 50,
            "graph root object",
        )?;
        if RootObjectDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_root_digest",
                "graph root bytes do not match their physical key",
            ));
        }
        StoredGraphRoot::decode(&bytes)
    }

    pub fn read_root(&self, digest: RootObjectDigest) -> Result<GraphRoot, Diagnostic> {
        self.read_stored_root(digest)?.reconstruct(
            &RepositoryPageStore::read_only(&self.store),
            &mut MapWork::default(),
        )
    }

    pub fn read_module(
        &self,
        digest: super::semantic_digest::ModuleObjectDigest,
    ) -> Result<MeaningModule, Diagnostic> {
        let path = module_path(&self.store, digest);
        let bytes = read_bounded(
            &path,
            super::meaning::MAXIMUM_MODULE_SEGMENT_BYTES + 50,
            "meaning module object",
        )?;
        if super::semantic_digest::ModuleObjectDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_module_digest",
                "meaning module bytes do not match their physical key",
            ));
        }
        MeaningModule::decode(&bytes)
    }

    fn read_modules(&self, root: &GraphRoot) -> Result<Vec<MeaningModule>, Diagnostic> {
        root.modules
            .iter()
            .map(|reference| self.read_module(reference.object))
            .collect()
    }

    fn read_head(&self) -> Result<SemanticHead, Diagnostic> {
        SemanticHead::decode(&read_bounded(
            &self.store.join(HEAD_FILE),
            MAXIMUM_HEAD_BYTES,
            "semantic HEAD",
        )?)
    }

    pub(crate) fn publish(
        &self,
        mut proposal: PublicationProposal,
    ) -> Result<(PublicationOutcome, Option<TransactionReceipt>), Diagnostic> {
        self.publish_with_additional_parent(&mut proposal, None)
    }

    pub(crate) fn publish_merge(
        &self,
        mut proposal: PublicationProposal,
        additional_parent: ParentRevision,
    ) -> Result<(PublicationOutcome, Option<TransactionReceipt>), Diagnostic> {
        self.publish_with_additional_parent(&mut proposal, Some(additional_parent))
    }

    fn publish_with_additional_parent(
        &self,
        proposal: &mut PublicationProposal,
        additional_parent: Option<ParentRevision>,
    ) -> Result<(PublicationOutcome, Option<TransactionReceipt>), Diagnostic> {
        if proposal.prepared_validation.is_none() {
            proposal
                .root
                .as_ref()
                .ok_or_else(|| {
                    repository_error(
                        DiagnosticClass::Infrastructure,
                        "repository_proposal_root_missing",
                        "an unprepared publication requires a complete logical root",
                    )
                })?
                .validate_modules(&proposal.modules)?;
        }
        let lock_path = self.store.join(LOCK_FILE);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| io_error("repository_lock_open", &lock_path, error))?;
        lock.lock_exclusive()
            .map_err(|error| io_error("repository_lock", &lock_path, error))?;
        let result = self.publish_locked(proposal, additional_parent);
        FileExt::unlock(&lock).map_err(|error| io_error("repository_unlock", &lock_path, error))?;
        result
    }

    fn publish_locked(
        &self,
        proposal: &mut PublicationProposal,
        additional_parent: Option<ParentRevision>,
    ) -> Result<(PublicationOutcome, Option<TransactionReceipt>), Diagnostic> {
        let current = self.current_binding()?;
        if current.head.revision != proposal.expected_base {
            return Ok((
                PublicationOutcome::StaleBase {
                    requested: proposal.expected_base,
                    current: current.head.revision,
                },
                None,
            ));
        }
        if proposal.repository_id != current.head.repository_id {
            return Err(repository_error(
                DiagnosticClass::Source,
                "repository_foreign_identity",
                "proposed graph root belongs to a foreign repository",
            ));
        }
        let (
            stored,
            changed_modules,
            prepared_facts,
            semantic_facts,
            semantic_summaries,
            local_index_delta,
        ) = if let Some(prepared) = proposal.prepared_validation.as_ref() {
            if prepared.expected_base != proposal.expected_base
                || prepared.base_root != current.record.core.root
            {
                return Err(repository_error(
                    DiagnosticClass::Infrastructure,
                    "repository_prepared_base_binding",
                    "prepared validation does not bind the exact visible base",
                ));
            }
            (
                prepared.stored_update.clone(),
                prepared.changed_modules.clone(),
                Some(prepared.facts.clone()),
                prepared.semantic_facts.clone(),
                prepared.semantic_summaries.clone(),
                prepared.local_index_delta.clone(),
            )
        } else {
            let proposed_root = proposal.root.as_ref().ok_or_else(|| {
                repository_error(
                    DiagnosticClass::Infrastructure,
                    "repository_proposal_root_missing",
                    "an unprepared publication requires a complete logical root",
                )
            })?;
            let current_root = current.stored_root.reconstruct(
                &RepositoryPageStore::read_only(&self.store),
                &mut MapWork::default(),
            )?;
            let delta = StoredGraphRootDelta::between(&current_root, proposed_root)?;
            let stored = current
                .stored_root
                .apply_delta(&RepositoryPageStore::read_only(&self.store), &delta)?;
            let semantic_summaries =
                build_semantic_summaries(&proposed_root.package_id, &proposal.modules)?;
            let semantic_facts = build_semantic_facts(
                proposed_root.repository_id,
                &proposed_root.package_id,
                RevisionId::from_digest([0; 32]),
                stored.root.digest()?,
                &semantic_summaries,
            )?;
            (
                stored,
                proposal.modules.clone(),
                None,
                semantic_facts,
                semantic_summaries,
                None,
            )
        };
        let root_bytes = stored.root.encode()?;
        let root_digest = stored.root.digest()?;
        if let Some(prepared) = proposal.prepared_validation.as_ref()
            && (prepared.result_root != root_digest
                || proposal.semantic_diff
                    != semantic_diff_digest(prepared.base_root, prepared.result_root))
        {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "repository_prepared_result_binding",
                "prepared validation does not bind the exact proposed result",
            ));
        }
        if root_digest == current.record.core.root {
            return Ok((
                PublicationOutcome::SemanticNoChange {
                    revision: current.head.revision,
                    record: current.head.record,
                },
                None,
            ));
        }

        for artifact in &proposal.dependency_artifacts {
            write_artifact_at(&self.store, artifact)?;
        }

        let mut parents = vec![ParentRevision {
            revision: current.head.revision,
            record: current.head.record,
        }];
        if let Some(parent) = additional_parent {
            if parent.revision == current.head.revision {
                return Err(repository_error(
                    DiagnosticClass::Source,
                    "repository_merge_parent_duplicate",
                    "merge parents must name two distinct accepted revisions",
                ));
            }
            let record = self.read_revision(parent.revision)?;
            if record.digest()? != parent.record
                || record.core.repository_id != current.head.repository_id
            {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_merge_parent_binding",
                    "additional merge parent does not bind an exact revision in this repository",
                ));
            }
            parents.push(parent);
            parents.sort();
        }
        let core = RevisionCore {
            contract_version: REVISION_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: proposal.repository_id,
            parents,
            root: root_digest,
            semantic_certificate: semantic_facts.manifest.certificate,
            semantic_diff: proposal.semantic_diff,
            transaction: proposal.transaction,
        };
        let revision = core.revision_id()?;
        let mut semantic_facts = semantic_facts;
        semantic_facts.manifest = semantic_facts.manifest.rebind_revision(revision)?;
        let validation = match prepared_facts {
            Some(facts) => facts,
            None => validation_facts(&validate_repository_graph(
                &self.store,
                proposal.root.as_ref().ok_or_else(|| {
                    repository_error(
                        DiagnosticClass::Infrastructure,
                        "repository_proposal_root_missing",
                        "an unprepared publication requires a complete logical root",
                    )
                })?,
                &proposal.modules,
                Some(revision),
            )?)?,
        };
        for module in &changed_modules {
            let bytes = module.encode()?;
            write_publication_immutable(
                &module_path(&self.store, module.digest()?),
                &bytes,
                "meaning module object",
            )?;
        }
        persist_map_pages(&self.store, &stored.pages, true)?;
        write_publication_immutable(
            &root_path(&self.store, root_digest),
            &root_bytes,
            "graph root object",
        )?;
        // Semantic summaries and dependency facts are disposable acceleration. Their
        // publication must never prevent an otherwise valid canonical revision from becoming
        // visible; any partial generation is discarded or rebuilt on the next read.
        let _ = self.write_semantic_cache(&semantic_facts, &semantic_summaries);
        if let Some(local_index_delta) = &local_index_delta {
            // Exact owner/name indexes are disposable acceleration. New content-addressed shards
            // are installed before their revision-bound manifest, and any failure deliberately
            // leaves canonical publication authoritative and rebuildable.
            let _ = SemanticQueryIndex::install_local_index_delta(
                self,
                local_index_delta,
                proposal.expected_base,
                current.record.core.root,
                revision,
                root_digest,
            );
        } else if let Some(root) = proposal.root.as_ref() {
            // Full-candidate preparation already owns the complete validated graph. Seed the
            // exact disposable generation from those in-memory values so the next exact query
            // does not have to construct the broad relation index merely to recover a cache.
            let _ = SemanticQueryIndex::seed_local_index(
                self,
                revision,
                root_digest,
                root,
                &proposal.modules,
            );
        }
        proposal.affected_owners.sort();
        proposal.affected_owners.dedup();
        let receipt = TransactionReceipt {
            contract_version: RECEIPT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: proposal.repository_id,
            status: proposal.status,
            base: Some(proposal.expected_base),
            result: revision,
            transaction: proposal.transaction,
            idempotency_key: proposal.idempotency_key.clone(),
            semantic_diff: proposal.semantic_diff,
            affected_owners: proposal.affected_owners.clone(),
            validation,
            intent: proposal.intent.clone(),
        };
        let receipt_bytes = receipt.encode()?;
        let receipt_digest = ReceiptDigest::of(&receipt_bytes);
        write_publication_immutable(
            &receipt_path(&self.store, receipt_digest),
            &receipt_bytes,
            "transaction receipt",
        )?;
        let record = RevisionRecord::new(core, receipt_digest)?;
        let record_bytes = record.encode()?;
        let record_digest = record.digest()?;
        write_publication_immutable(
            &revision_path(&self.store, revision),
            &record_bytes,
            "revision record",
        )?;
        sync_publication_objects(&self.store)?;
        let head = SemanticHead {
            contract_version: REVISION_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: proposal.repository_id,
            revision,
            record: record_digest,
        };
        replace_head(&self.store, &head)?;
        Ok((
            PublicationOutcome::Accepted {
                revision,
                record: record_digest,
                receipt: receipt_digest,
            },
            Some(receipt),
        ))
    }

    pub fn history(
        &self,
        start: Option<RevisionId>,
        limit: usize,
    ) -> Result<Vec<RevisionRecord>, Diagnostic> {
        if limit == 0 || limit > MAXIMUM_HISTORY_ITEMS {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_history_limit",
                format!("history limit must be 1 through {MAXIMUM_HISTORY_ITEMS}"),
            ));
        }
        let start = start.unwrap_or(self.read_head()?.revision);
        let mut pending = std::collections::VecDeque::from([start]);
        let mut seen = std::collections::BTreeSet::new();
        let mut records = Vec::new();
        while records.len() < limit {
            let Some(next) = pending.pop_front() else {
                break;
            };
            if !seen.insert(next) {
                continue;
            }
            let record = self.read_revision(next)?;
            pending.extend(record.core.parents.iter().map(|value| value.revision));
            records.push(record);
        }
        Ok(records)
    }

    pub fn is_ancestor(
        &self,
        ancestor: RevisionId,
        descendant: RevisionId,
        maximum_work: usize,
    ) -> Result<bool, Diagnostic> {
        if maximum_work == 0 || maximum_work > MAXIMUM_HISTORY_ITEMS {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_ancestry_limit",
                format!("ancestry work must be 1 through {MAXIMUM_HISTORY_ITEMS}"),
            ));
        }
        let mut pending = vec![descendant];
        let mut seen = std::collections::BTreeSet::new();
        while let Some(revision) = pending.pop() {
            if revision == ancestor {
                return Ok(true);
            }
            if !seen.insert(revision) {
                continue;
            }
            if seen.len() > maximum_work {
                return Err(repository_error(
                    DiagnosticClass::Resource,
                    "repository_ancestry_exhausted",
                    "revision ancestry exhausted its declared work budget",
                ));
            }
            let record = self.read_revision(revision)?;
            pending.extend(record.core.parents.iter().map(|parent| parent.revision));
        }
        Ok(false)
    }

    pub fn receipt_for_idempotency(
        &self,
        key: &str,
    ) -> Result<Option<TransactionReceipt>, Diagnostic> {
        let records = self.reachable_records(self.read_head()?.revision, MAXIMUM_HISTORY_ITEMS)?;
        let mut found = None;
        for record in records {
            let receipt = self.read_receipt(record.receipt)?;
            if receipt.idempotency_key.as_deref() != Some(key) {
                continue;
            }
            if found.is_some() {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_idempotency_duplicate",
                    "two accepted receipts retain one idempotency key",
                ));
            }
            found = Some(receipt);
        }
        Ok(found)
    }

    fn reachable_records(
        &self,
        start: RevisionId,
        limit: usize,
    ) -> Result<Vec<RevisionRecord>, Diagnostic> {
        let mut pending = vec![start];
        let mut seen = std::collections::BTreeSet::new();
        let mut records = Vec::new();
        while let Some(revision) = pending.pop() {
            if !seen.insert(revision) {
                continue;
            }
            if records.len() >= limit {
                return Err(repository_error(
                    DiagnosticClass::Resource,
                    "repository_reachable_history_limit",
                    "reachable revision traversal exhausted its declared item limit",
                ));
            }
            let record = self.read_revision(revision)?;
            pending.extend(record.core.parents.iter().map(|parent| parent.revision));
            records.push(record);
        }
        records.sort_by_key(|record| record.revision);
        Ok(records)
    }

    pub fn doctor(&self, deep: bool) -> Result<DoctorReport, Diagnostic> {
        let current = self.current()?;
        if !deep {
            return Ok(DoctorReport {
                valid: true,
                deep: false,
                revision: current.head.revision,
                revisions_checked: 1,
                roots_checked: 1,
                modules_checked: 0,
                receipts_checked: 1,
                rebuilt_indexes: 0,
            });
        }
        let mut pending = vec![(current.head.revision, current.head.record)];
        let mut seen = BTreeMap::<RevisionId, RevisionRecordDigest>::new();
        let mut revisions_checked = 0usize;
        let mut roots_checked = 0usize;
        let mut modules_checked = 0usize;
        let mut receipts_checked = 0usize;
        while let Some((revision, expected_record)) = pending.pop() {
            if !seen.contains_key(&revision) && seen.len() >= MAXIMUM_HISTORY_ITEMS {
                return Err(repository_error(
                    DiagnosticClass::Resource,
                    "repository_doctor_history_limit",
                    format!("deep doctor exceeds {MAXIMUM_HISTORY_ITEMS} retained revisions"),
                ));
            }
            if let Some(previous) = seen.insert(revision, expected_record) {
                if previous != expected_record {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_history_parent_conflict",
                        "retained history binds one revision to conflicting records",
                    ));
                }
                continue;
            }
            let record = self.read_revision(revision)?;
            if record.digest()? != expected_record {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_history_record_binding",
                    "history parent does not bind its exact revision record",
                ));
            }
            let receipt = self.read_receipt(record.receipt)?;
            if receipt.result != record.revision
                || receipt.repository_id != record.core.repository_id
                || receipt.transaction != record.core.transaction
                || receipt.semantic_diff != record.core.semantic_diff
            {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_history_receipt_binding",
                    "historical receipt disagrees with its revision identity or semantic inputs",
                ));
            }
            let root = self.read_root(record.core.root)?;
            if root.repository_id != record.core.repository_id {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_history_root_binding",
                    "historical graph root belongs to a foreign repository identity",
                ));
            }
            let modules = self.read_modules(&root)?;
            root.validate_modules(&modules)?;
            revisions_checked = checked_increment(revisions_checked, "revision")?;
            roots_checked = checked_increment(roots_checked, "root")?;
            modules_checked = modules_checked
                .checked_add(modules.len())
                .ok_or_else(|| count_overflow("module"))?;
            receipts_checked = checked_increment(receipts_checked, "receipt")?;
            pending.extend(
                record
                    .core
                    .parents
                    .iter()
                    .map(|parent| (parent.revision, parent.record)),
            );
        }
        let rebuilt_query = SemanticQueryIndex::current(self)?.rebuilt_index();
        let rebuilt_semantic = self
            .load_or_rebuild_semantic_facts(&self.current_binding()?)?
            .1;
        let rebuilt_indexes = usize::from(rebuilt_query) + usize::from(rebuilt_semantic);
        Ok(DoctorReport {
            valid: true,
            deep: true,
            revision: current.head.revision,
            revisions_checked,
            roots_checked,
            modules_checked,
            receipts_checked,
            rebuilt_indexes,
        })
    }

    /// Computes an exact, read-only inventory for the current retention policy. Destructive
    /// cleanup remains disabled until pins and active-reader leases become explicit authority.
    pub fn retention_preview(&self) -> Result<RetentionReport, Diagnostic> {
        let lock_path = self.store.join(LOCK_FILE);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| io_error("repository_retention_lock_open", &lock_path, error))?;
        FileExt::lock_shared(&lock)
            .map_err(|error| io_error("repository_retention_lock", &lock_path, error))?;
        let result = (|| {
            let head = self.read_head()?;
            let (retained, retained_drafts) = self.collect_backup_entries(&head)?;
            let retained_paths = retained
                .iter()
                .map(|key| backup_entry_path(&self.store, key))
                .collect::<BTreeSet<_>>();
            let actual = canonical_store_files(&self.store)?;
            let mut reclaimable = Vec::new();
            let mut unknown_entries = 0_u64;
            for path in actual {
                if retained_paths.contains(&path) {
                    continue;
                }
                let relative = path.strip_prefix(&self.store).map_err(|_| {
                    repository_error(
                        DiagnosticClass::Infrastructure,
                        "repository_retention_relative_path",
                        "canonical object inventory escaped the repository store",
                    )
                })?;
                let bytes = fs::symlink_metadata(&path)
                    .map_err(|error| io_error("repository_retention_metadata", &path, error))?
                    .len();
                if canonical_object_path_shape(relative) {
                    reclaimable.push((relative.to_path_buf(), bytes, hash_file(&path)?));
                } else {
                    unknown_entries = unknown_entries
                        .checked_add(1)
                        .ok_or_else(|| count_overflow("unknown retention entry"))?;
                }
            }
            reclaimable.sort_by(|left, right| left.0.cmp(&right.0));
            let reclaimable_bytes = reclaimable.iter().try_fold(0_u64, |total, value| {
                total
                    .checked_add(value.1)
                    .ok_or_else(|| count_overflow("reclaimable bytes"))
            })?;
            let (derived_objects, derived_bytes) =
                directory_file_totals(&self.store.join(INDEX_OBJECTS))?;
            let retained_revisions = u64::try_from(
                retained
                    .iter()
                    .filter(|key| matches!(key, BackupEntryKey::Revision(_)))
                    .count(),
            )
            .map_err(|_| count_overflow("retained revisions"))?;
            let plan = retention_plan_digest(
                head.repository_id,
                head.revision,
                &retained,
                &reclaimable,
                unknown_entries,
            )?;
            Ok(RetentionReport {
                contract_version: RETENTION_CONTRACT_VERSION,
                repository_id: head.repository_id,
                revision: head.revision,
                policy: "head_parent_dag_and_live_draft_bases",
                retained_revisions,
                retained_drafts,
                retained_objects: u64::try_from(retained.len())
                    .map_err(|_| count_overflow("retained objects"))?,
                reclaimable_objects: u64::try_from(reclaimable.len())
                    .map_err(|_| count_overflow("reclaimable objects"))?,
                reclaimable_bytes,
                unknown_entries,
                derived_objects,
                derived_bytes,
                plan,
                destructive_ready: false,
                missing_authority: vec![
                    "revision_pins",
                    "active_reader_leases",
                    "registered_backup_roots",
                ],
            })
        })();
        FileExt::unlock(&lock)
            .map_err(|error| io_error("repository_retention_unlock", &lock_path, error))?;
        result
    }

    /// Writes a deterministic segmented backup directory through one atomic visibility point.
    /// Canonical objects are copied one at a time; the complete authority is never accumulated in
    /// one in-memory container.
    pub fn backup_to(&self, output: &Path) -> Result<BackupReceipt, Diagnostic> {
        let (output, parent, stage) = prepare_backup_stage(output)?;
        let lock_path = self.store.join(LOCK_FILE);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&lock_path)
            .map_err(|error| io_error("repository_backup_lock_open", &lock_path, error))?;
        FileExt::lock_shared(&lock)
            .map_err(|error| io_error("repository_backup_lock", &lock_path, error))?;
        let result = (|| {
            let head = self.read_head()?;
            let (entries, draft_count) = self.collect_backup_entries(&head)?;
            fs::create_dir(&stage)
                .map_err(|error| io_error("semantic_backup_stage_create", &stage, error))?;
            for relative in [
                MODULE_OBJECTS,
                MAP_PAGE_OBJECTS,
                ROOT_OBJECTS,
                REVISION_OBJECTS,
                RECEIPT_OBJECTS,
                ARTIFACT_OBJECTS,
                DRAFT_OBJECTS,
            ] {
                let directory = stage.join(relative);
                fs::create_dir_all(&directory).map_err(|error| {
                    io_error("semantic_backup_payload_directory", &directory, error)
                })?;
            }
            let segment_directory = stage.join(BACKUP_SEGMENTS);
            fs::create_dir(&segment_directory).map_err(|error| {
                io_error(
                    "semantic_backup_segment_directory",
                    &segment_directory,
                    error,
                )
            })?;

            let mut segment_references = Vec::new();
            let mut payload_bytes = 0_u64;
            let mut total_bytes = 0_u64;
            let mut remaining = entries.iter();
            for ordinal in 0_usize.. {
                let keys = remaining
                    .by_ref()
                    .take(BACKUP_SEGMENT_ENTRY_LIMIT)
                    .collect::<Vec<_>>();
                if keys.is_empty() {
                    break;
                }
                let ordinal =
                    u64::try_from(ordinal).map_err(|_| count_overflow("backup segment ordinal"))?;
                let mut segment_entries = Vec::with_capacity(keys.len());
                for key in keys {
                    let bytes = read_backup_entry(&self.store, key)?;
                    validate_backup_entry(key, &bytes)?;
                    let length = u64::try_from(bytes.len())
                        .map_err(|_| count_overflow("backup entry bytes"))?;
                    let digest = backup_entry_digest(&bytes);
                    write_immutable(
                        &backup_entry_path(&stage, key),
                        &bytes,
                        "segmented backup object",
                    )?;
                    payload_bytes = payload_bytes
                        .checked_add(length)
                        .ok_or_else(|| count_overflow("backup payload bytes"))?;
                    segment_entries.push(BackupEntry {
                        key: (*key).clone(),
                        bytes: length,
                        digest,
                    });
                }
                let segment = BackupSegment {
                    contract_version: BACKUP_CONTRACT_VERSION,
                    ordinal,
                    entries: segment_entries,
                };
                let bytes = segment.encode()?;
                let digest = backup_segment_digest(&bytes);
                let encoded_bytes = u64::try_from(bytes.len())
                    .map_err(|_| count_overflow("backup segment bytes"))?;
                let path = backup_segment_path(&stage, ordinal, digest);
                write_immutable(&path, &bytes, "backup index segment")?;
                total_bytes = total_bytes
                    .checked_add(encoded_bytes)
                    .ok_or_else(|| count_overflow("backup total bytes"))?;
                segment_references.push(BackupSegmentReference {
                    ordinal,
                    digest,
                    entries: u32::try_from(segment.entries.len())
                        .map_err(|_| count_overflow("backup segment entries"))?,
                    encoded_bytes,
                });
            }
            let manifest = BackupManifest {
                contract_version: BACKUP_CONTRACT_VERSION,
                repository_id: head.repository_id,
                head,
                segments: segment_references,
                entries: u64::try_from(entries.len())
                    .map_err(|_| count_overflow("backup entries"))?,
                drafts: draft_count,
                payload_bytes,
            };
            let bytes = manifest.encode()?;
            write_new_file(&stage.join(BACKUP_MANIFEST_FILE), &bytes, "backup manifest")?;
            total_bytes = total_bytes
                .checked_add(payload_bytes)
                .and_then(|total| total.checked_add(u64::try_from(bytes.len()).ok()?))
                .ok_or_else(|| count_overflow("backup total bytes"))?;
            sync_backup_tree(&stage)?;
            let receipt = BackupReceipt {
                contract_version: BACKUP_CONTRACT_VERSION,
                repository_id: manifest.repository_id,
                revision: manifest.head.revision,
                digest: BackupDigest::of(&bytes),
                segments: manifest.segments.len(),
                entries: manifest.entries,
                drafts: manifest.drafts,
                bytes: total_bytes,
            };
            Ok(receipt)
        })();
        let unlock = FileExt::unlock(&lock)
            .map_err(|error| io_error("repository_backup_unlock", &lock_path, error));
        let receipt = match (result, unlock) {
            (Ok(receipt), Ok(())) => receipt,
            (Err(error), _) | (Ok(_), Err(error)) => {
                remove_backup_stage(&stage);
                return Err(error);
            }
        };
        if let Err(error) = fs::rename(&stage, &output) {
            remove_backup_stage(&stage);
            return Err(io_error("semantic_backup_publish", &output, error));
        }
        sync_directory(&parent).map_err(|error| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "semantic_backup_visibility_indeterminate",
                format!(
                    "segmented backup is visible but parent durability is indeterminate: {error}"
                ),
            )
        })?;
        Ok(receipt)
    }

    pub fn restore_backup_from(
        project_root: &Path,
        backup: &Path,
    ) -> Result<(Self, RestoreReceipt), Diagnostic> {
        let backup = canonical_existing(backup)?;
        ensure_directory(&backup, "semantic_backup_directory")?;
        let manifest_bytes = read_bounded(
            &backup.join(BACKUP_MANIFEST_FILE),
            MAXIMUM_BACKUP_MANIFEST_BYTES,
            "backup manifest",
        )?;
        let manifest = BackupManifest::decode(&manifest_bytes)?;
        let digest = BackupDigest::of(&manifest_bytes);
        let project_root = canonical_existing(project_root)?;
        ensure_directory(&project_root, "repository_restore_project_type")?;
        let semantic_parent = project_root.join(".lkjscript");
        ensure_or_create_directory(&semantic_parent, "repository_restore_parent")?;
        let store = project_root.join(SEMANTIC_STORE_RELATIVE);
        if fs::symlink_metadata(&store).is_ok() {
            return Err(repository_error(
                DiagnosticClass::Source,
                "semantic_repository_exists",
                format!(
                    "current meaning authority already exists at '{}'",
                    store.display()
                ),
            ));
        }
        let stage_identity = RepositoryId::generate()?;
        let stage = semantic_parent.join(format!(".meaning-stage-{stage_identity}"));
        create_store_layout(&stage)?;
        let restored = (|| {
            let mut observed_entries = 0_u64;
            let mut observed_payload_bytes = 0_u64;
            let mut previous_key = None::<BackupEntryKey>;
            let mut observed_keys = BTreeSet::new();
            for reference in &manifest.segments {
                let path = backup_segment_path(&backup, reference.ordinal, reference.digest);
                let bytes =
                    read_bounded(&path, MAXIMUM_BACKUP_SEGMENT_BYTES, "backup index segment")?;
                if backup_segment_digest(&bytes) != reference.digest
                    || u64::try_from(bytes.len()).ok() != Some(reference.encoded_bytes)
                {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "semantic_backup_segment_digest",
                        "backup segment bytes do not match their manifest binding",
                    ));
                }
                let segment = BackupSegment::decode(&bytes)?;
                if segment.ordinal != reference.ordinal
                    || u32::try_from(segment.entries.len()).ok() != Some(reference.entries)
                {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "semantic_backup_segment_binding",
                        "backup segment content does not match its manifest reference",
                    ));
                }
                for entry in segment.entries {
                    if previous_key.as_ref().is_some_and(|key| key >= &entry.key) {
                        return Err(repository_error(
                            DiagnosticClass::Corrupt,
                            "semantic_backup_entry_order",
                            "backup entries are not globally unique and canonically ordered",
                        ));
                    }
                    let source = backup_entry_path(&backup, &entry.key);
                    let object = read_backup_entry(&backup, &entry.key)?;
                    if u64::try_from(object.len()).ok() != Some(entry.bytes)
                        || backup_entry_digest(&object) != entry.digest
                    {
                        return Err(repository_error(
                            DiagnosticClass::Corrupt,
                            "semantic_backup_entry_digest",
                            format!(
                                "backup payload '{}' does not match its segment binding",
                                source.display()
                            ),
                        ));
                    }
                    validate_backup_entry(&entry.key, &object)?;
                    write_immutable(
                        &backup_entry_path(&stage, &entry.key),
                        &object,
                        "restored canonical object",
                    )?;
                    observed_entries = observed_entries
                        .checked_add(1)
                        .ok_or_else(|| count_overflow("restored backup entries"))?;
                    observed_payload_bytes = observed_payload_bytes
                        .checked_add(entry.bytes)
                        .ok_or_else(|| count_overflow("restored backup bytes"))?;
                    previous_key = Some(entry.key.clone());
                    observed_keys.insert(entry.key);
                }
            }
            if observed_entries != manifest.entries
                || observed_payload_bytes != manifest.payload_bytes
            {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_aggregate",
                    "restored backup counts disagree with its manifest",
                ));
            }
            let mut expected_backup_files = observed_keys
                .iter()
                .map(|key| backup_entry_path(&backup, key))
                .collect::<BTreeSet<_>>();
            expected_backup_files.extend(manifest.segments.iter().map(|reference| {
                backup_segment_path(&backup, reference.ordinal, reference.digest)
            }));
            expected_backup_files.insert(backup.join(BACKUP_MANIFEST_FILE));
            let mut actual_backup_files = Vec::new();
            collect_regular_files(&backup, &mut actual_backup_files)?;
            if expected_backup_files != actual_backup_files.into_iter().collect() {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_file_inventory",
                    "backup files do not exactly match its manifest and segment entries",
                ));
            }
            sync_publication_objects(&stage)?;
            sync_tree_directories(&stage.join(ARTIFACT_OBJECTS))?;
            replace_head(&stage, &manifest.head)?;
            let staged = Self {
                project_root: project_root.clone(),
                store: stage.clone(),
            };
            let (retained_keys, retained_drafts) = staged.collect_backup_entries(&manifest.head)?;
            if retained_keys != observed_keys || retained_drafts != manifest.drafts {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_reachability",
                    "backup entries are not exactly the HEAD-and-draft retained closure",
                ));
            }
            let report = staged.doctor(true)?;
            let drafts = SemanticDraftStore::new(&staged).validate_all()?;
            Ok((report, drafts))
        })();
        let (report, drafts) = match restored {
            Ok(evidence) => evidence,
            Err(error) => {
                remove_owned_stage(&stage);
                return Err(error);
            }
        };
        if let Err(error) = fs::rename(&stage, &store) {
            remove_owned_stage(&stage);
            return Err(io_error("repository_restore_publish", &store, error));
        }
        sync_directory(&semantic_parent).map_err(|error| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_restore_visibility_indeterminate",
                format!(
                    "restored authority is visible but parent durability is indeterminate: {error}"
                ),
            )
        })?;
        let repository = Self {
            project_root,
            store,
        };
        let visible = repository.current_binding()?;
        if visible.head != manifest.head {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "repository_restore_reconcile",
                "visible restored HEAD differs from the privately verified authority",
            ));
        }
        Ok((
            repository,
            RestoreReceipt {
                contract_version: BACKUP_CONTRACT_VERSION,
                repository_id: manifest.repository_id,
                revision: manifest.head.revision,
                digest,
                segments: manifest.segments.len(),
                entries: manifest.entries,
                drafts: u64::try_from(drafts).map_err(|_| count_overflow("restored drafts"))?,
                deep_valid: report.valid,
            },
        ))
    }

    fn collect_backup_entries(
        &self,
        head: &SemanticHead,
    ) -> Result<(BTreeSet<BackupEntryKey>, u64), Diagnostic> {
        let mut entries = BTreeSet::new();
        let mut pending = vec![(head.revision, Some(head.record))];

        let mut draft_count = 0_u64;
        let drafts = self.store.join(DRAFT_OBJECTS);
        for entry in fs::read_dir(&drafts)
            .map_err(|error| io_error("semantic_backup_draft_list", &drafts, error))?
        {
            let entry =
                entry.map_err(|error| io_error("semantic_backup_draft_list", &drafts, error))?;
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)
                .map_err(|error| io_error("semantic_backup_draft_type", &path, error))?;
            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.parse::<DraftId>().ok())
                .ok_or_else(|| {
                    repository_error(
                        DiagnosticClass::Corrupt,
                        "semantic_backup_draft_name",
                        "draft authority has a noncanonical physical name",
                    )
                })?;
            if metadata.file_type().is_symlink()
                || !metadata.is_file()
                || path.extension().and_then(|value| value.to_str()) != Some("lkjd")
            {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_draft_type",
                    "draft authority is not a regular canonical draft object",
                ));
            }
            let bytes = read_bounded(&path, MAXIMUM_DRAFT_BYTES + 50, "semantic draft authority")?;
            let draft = DraftRecord::decode(&bytes)?;
            if draft.id != id || draft.repository_id != head.repository_id {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_draft_binding",
                    "draft identity or repository binding is inconsistent",
                ));
            }
            pending.push((draft.base_revision, None));
            entries.insert(BackupEntryKey::Draft(id));
            draft_count = draft_count
                .checked_add(1)
                .ok_or_else(|| count_overflow("backup draft"))?;
        }

        let mut seen = BTreeMap::<RevisionId, RevisionRecordDigest>::new();
        let source = RepositoryPageStore::read_only(&self.store);
        while let Some((revision, expected_record)) = pending.pop() {
            let record = self.read_revision(revision)?;
            let record_digest = record.digest()?;
            if expected_record.is_some_and(|expected| expected != record_digest) {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_parent_binding",
                    "revision parent or HEAD record digest does not match stored bytes",
                ));
            }
            if let Some(previous) = seen.insert(revision, record_digest) {
                if previous != record_digest {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "semantic_backup_revision_binding",
                        "one retained revision is bound to conflicting record digests",
                    ));
                }
                continue;
            }
            if seen.len() > MAXIMUM_HISTORY_ITEMS {
                return Err(repository_error(
                    DiagnosticClass::Resource,
                    "semantic_backup_history_limit",
                    format!("backup exceeds {MAXIMUM_HISTORY_ITEMS} retained revisions"),
                ));
            }
            entries.insert(BackupEntryKey::Revision(revision));
            let receipt = self.read_receipt(record.receipt)?;
            if receipt.result != record.revision
                || receipt.repository_id != record.core.repository_id
                || receipt.transaction != record.core.transaction
                || receipt.semantic_diff != record.core.semantic_diff
            {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_receipt_binding",
                    "retained receipt disagrees with its revision identity or semantic inputs",
                ));
            }
            entries.insert(BackupEntryKey::Receipt(record.receipt));
            let stored_root = self.read_stored_root(record.core.root)?;
            if stored_root.repository_id != record.core.repository_id {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "semantic_backup_root_binding",
                    "retained graph root belongs to a foreign repository identity",
                ));
            }
            entries.insert(BackupEntryKey::Root(record.core.root));

            let mut page_collector = BackupPageCollector::default();
            let mut page_work = MapWork::default();
            for map_root in stored_root_map_roots(&stored_root) {
                PersistentMap::from_root(map_root)
                    .copy_reachable(&source, &mut page_collector, &mut page_work)
                    .map_err(repository_map_diagnostic)?;
            }
            entries.extend(
                page_collector
                    .pages
                    .into_iter()
                    .map(BackupEntryKey::MapPage),
            );

            stored_root.for_each_module_reference(
                &source,
                &mut MapWork::default(),
                |reference| {
                    self.read_module(reference.object)?;
                    entries.insert(BackupEntryKey::Module(reference.object));
                    Ok(())
                },
            )?;
            stored_root.for_each_dependency_binding(
                &source,
                &mut MapWork::default(),
                |dependency| {
                    collect_stored_artifact_keys(&self.store, dependency.artifact, &mut entries)
                },
            )?;
            pending.extend(
                record
                    .core
                    .parents
                    .iter()
                    .map(|parent| (parent.revision, Some(parent.record))),
            );
        }
        Ok((entries, draft_count))
    }

    pub fn write_artifact_object(
        &self,
        digest: ArtifactDigest,
        bytes: &[u8],
    ) -> Result<(), Diagnostic> {
        if bytes.len() > MAXIMUM_ARTIFACT_OBJECT_BYTES || ArtifactDigest::of(bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Source,
                "repository_artifact_digest",
                "artifact bytes exceed the bound or do not match the declared digest",
            ));
        }
        write_immutable(
            &artifact_path(&self.store, digest),
            bytes,
            "dependency artifact",
        )
    }

    pub fn read_artifact_object(&self, digest: ArtifactDigest) -> Result<Vec<u8>, Diagnostic> {
        let bytes = read_bounded(
            &artifact_path(&self.store, digest),
            MAXIMUM_ARTIFACT_OBJECT_BYTES,
            "dependency artifact",
        )?;
        if ArtifactDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_artifact_digest",
                "dependency artifact does not match its physical key",
            ));
        }
        Ok(bytes)
    }

    /// Loads the revision-bound semantic dependency certificate when its disposable bytes match
    /// the accepted revision binding. Missing or malformed cache state is rebuilt from canonical
    /// modules; it never changes accepted meaning.
    fn load_or_rebuild_semantic_facts(
        &self,
        current: &CurrentBinding,
    ) -> Result<(SemanticFactManifest, bool), Diagnostic> {
        // Every semantic-fact file is disposable acceleration. A malformed manifest, unsafe
        // filesystem object, or unreadable page is therefore a cache miss; canonical graph
        // reconstruction below remains the independent authority and will still fail closed if
        // canonical bytes are damaged.
        let cached = self
            .read_index_part(
                current.head.revision,
                DisposableIndexPart::SemanticFacts,
                MAXIMUM_SEMANTIC_FACT_MANIFEST_BYTES,
            )
            .ok()
            .flatten()
            .and_then(|bytes| SemanticFactManifest::decode(&bytes).ok())
            .filter(|manifest| {
                manifest.revision == current.head.revision
                    && manifest.repository_id == current.head.repository_id
                    && manifest.package_id == current.stored_root.package_id
                    && manifest.canonical_root == current.record.core.root
                    && manifest.certificate == current.record.core.semantic_certificate
                    && manifest
                        .verify(&SemanticFactPageStore::new(&self.store))
                        .is_ok()
            });
        if let Some(manifest) = cached {
            return Ok((manifest, false));
        }

        let snapshot = self.reconstruct_revision(current.head.revision)?;
        let summaries = build_semantic_summaries(&snapshot.root.package_id, &snapshot.modules)?;
        let update = build_semantic_facts(
            current.head.repository_id,
            &snapshot.root.package_id,
            current.head.revision,
            current.record.core.root,
            &summaries,
        )?;
        if update.manifest.certificate != current.record.core.semantic_certificate {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_semantic_certificate_binding",
                "accepted revision does not bind the semantic certificate rebuilt from canonical meaning",
            ));
        }
        let manifest = update.manifest.clone();
        let _ = self.write_semantic_cache(&update, &summaries);
        Ok((manifest, true))
    }

    fn write_semantic_cache(
        &self,
        update: &SemanticFactUpdate,
        summaries: &[ModuleSemanticSummary],
    ) -> Result<(), Diagnostic> {
        update.manifest.validate()?;
        for summary in summaries {
            summary.validate()?;
            let bytes = summary.encode()?;
            self.write_summary_object(summary.digest, &bytes)?;
        }
        let mut page_store = SemanticFactPageStore::new(&self.store);
        for (digest, bytes) in update.pages.objects() {
            page_store
                .write_page(digest, bytes)
                .map_err(repository_map_diagnostic)?;
        }
        let bytes = update.manifest.encode()?;
        self.write_index_part(
            update.manifest.revision,
            DisposableIndexPart::SemanticFacts,
            &bytes,
            MAXIMUM_SEMANTIC_FACT_MANIFEST_BYTES,
        )
    }

    fn write_summary_object(
        &self,
        digest: SemanticSummaryDigest,
        bytes: &[u8],
    ) -> Result<(), Diagnostic> {
        if bytes.len() > MAXIMUM_MODULE_SUMMARY_ENCODED_BYTES {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_summary_limit",
                "disposable semantic summary exceeds its single-object decoder budget",
            ));
        }
        let decoded = ModuleSemanticSummary::decode(bytes)?;
        if decoded.digest != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_summary_digest",
                "disposable semantic summary bytes do not match their content-addressed key",
            ));
        }
        let root = self.store.join(SUMMARY_INDEX_OBJECTS);
        ensure_or_create_directory(
            &self.store.join(INDEX_OBJECTS),
            "repository_index_directory",
        )?;
        ensure_or_create_directory(
            &self.store.join(SEMANTIC_INDEX_OBJECTS),
            "repository_semantic_index_directory",
        )?;
        ensure_or_create_directory(&root, "repository_summary_directory")?;
        let path = summary_object_path(&self.store, digest);
        let parent = path.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_summary_parent",
                "disposable semantic summary path has no parent",
            )
        })?;
        ensure_or_create_directory(parent, "repository_summary_shard")?;
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_summary_type",
                format!(
                    "disposable semantic summary '{}' is not a regular non-symlink file",
                    path.display()
                ),
            ));
        }
        let temporary = parent.join(format!(".summary-stage-{}", RepositoryId::generate()?));
        let result = (|| {
            // Cache objects do not require per-object durability. Canonical publication performs
            // its own batch durability barrier, while cache loss remains a safe rebuild case.
            write_new_file_with_sync(&temporary, bytes, "temporary semantic summary", false)?;
            fs::rename(&temporary, &path)
                .map_err(|error| io_error("repository_summary_publish", &path, error))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    /// Reads a disposable revision-bound index object. Missing indexes are normal and callers
    /// must rebuild them from canonical graph objects.
    pub(crate) fn read_index_object(
        &self,
        revision: RevisionId,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let path = index_path(&self.store, revision);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_index_type",
                        format!(
                            "disposable index '{}' is not a regular non-symlink file",
                            path.display()
                        ),
                    ));
                }
                read_bounded(&path, maximum_bytes, "disposable semantic index").map(Some)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error("repository_index_metadata", &path, error)),
        }
    }

    /// Atomically replaces a disposable index. This is acceleration state and never participates
    /// in revision identity, backup reachability, or accepted publication.
    pub(crate) fn write_index_object(
        &self,
        revision: RevisionId,
        bytes: &[u8],
        maximum_bytes: usize,
    ) -> Result<(), Diagnostic> {
        if bytes.len() > maximum_bytes {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_index_limit",
                format!("disposable semantic index exceeds {maximum_bytes} bytes"),
            ));
        }
        let index_root = self.store.join(INDEX_OBJECTS);
        ensure_directory(&index_root, "repository_index_directory")?;
        let path = index_path(&self.store, revision);
        let parent = path.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_index_parent",
                "disposable index path has no parent",
            )
        })?;
        ensure_or_create_directory(parent, "repository_index_shard")?;
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_index_type",
                format!(
                    "disposable index '{}' is not a regular non-symlink file",
                    path.display()
                ),
            ));
        }
        let temporary = parent.join(format!(".index-stage-{}", RepositoryId::generate()?));
        let result = (|| {
            write_new_file(&temporary, bytes, "temporary semantic index")?;
            fs::rename(&temporary, &path)
                .map_err(|error| io_error("repository_index_publish", &path, error))?;
            sync_directory(parent)
                .map_err(|error| io_error("repository_index_sync", parent, error))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub(crate) fn read_local_index_object(
        &self,
        kind: LocalIndexObjectKind,
        digest: IndexDigest,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let path = local_index_object_path(&self.store, kind, digest);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_local_index_object_type",
                        format!(
                            "disposable local index object '{}' is not a regular non-symlink file",
                            path.display()
                        ),
                    ));
                }
                read_bounded(&path, maximum_bytes, "disposable local index object").map(Some)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error(
                "repository_local_index_object_metadata",
                &path,
                error,
            )),
        }
    }

    pub(crate) fn write_local_index_object(
        &self,
        kind: LocalIndexObjectKind,
        digest: IndexDigest,
        bytes: &[u8],
        maximum_bytes: usize,
    ) -> Result<(), Diagnostic> {
        if bytes.len() > maximum_bytes {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_local_index_object_limit",
                format!("disposable local index object exceeds {maximum_bytes} bytes"),
            ));
        }
        if IndexDigest::of(bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "repository_local_index_object_digest",
                "disposable local index object bytes do not bind their content-addressed key",
            ));
        }
        ensure_directory(
            &self.store.join(INDEX_OBJECTS),
            "repository_index_directory",
        )?;
        let object_directory = self.store.join(match kind {
            LocalIndexObjectKind::Owner => LOCAL_INDEX_OWNER_OBJECTS,
            LocalIndexObjectKind::Name => LOCAL_INDEX_NAME_OBJECTS,
        });
        let object_parent = object_directory.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_local_index_object_parent",
                "disposable local index object directory has no parent",
            )
        })?;
        ensure_or_create_directory(object_parent, "repository_local_index_object_root")?;
        ensure_or_create_directory(&object_directory, "repository_local_index_object_directory")?;
        let path = local_index_object_path(&self.store, kind, digest);
        let parent = path.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_local_index_object_parent",
                "disposable local index object path has no parent",
            )
        })?;
        ensure_or_create_directory(parent, "repository_local_index_object_directory")?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_local_index_object_type",
                        format!(
                            "disposable local index object '{}' is not a regular non-symlink file",
                            path.display()
                        ),
                    ));
                }
                if metadata.len() == bytes.len() as u64
                    && fs::read(&path).map_err(|error| {
                        io_error("repository_local_index_object_read", &path, error)
                    })? == bytes
                {
                    return Ok(());
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                return Err(io_error(
                    "repository_local_index_object_metadata",
                    &path,
                    error,
                ));
            }
        }
        let temporary = parent.join(format!(".local-index-stage-{}", RepositoryId::generate()?));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| {
                    io_error("repository_local_index_object_create", &temporary, error)
                })?;
            file.write_all(bytes).map_err(|error| {
                io_error("repository_local_index_object_write", &temporary, error)
            })?;
            drop(file);
            fs::rename(&temporary, &path)
                .map_err(|error| io_error("repository_local_index_object_publish", &path, error))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    pub(crate) fn read_index_part(
        &self,
        revision: RevisionId,
        part: DisposableIndexPart,
        maximum_bytes: usize,
    ) -> Result<Option<Vec<u8>>, Diagnostic> {
        let path = index_part_path(&self.store, revision, part);
        match fs::symlink_metadata(&path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() || !metadata.is_file() {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "repository_index_part_type",
                        format!(
                            "disposable index part '{}' is not a regular non-symlink file",
                            path.display()
                        ),
                    ));
                }
                read_bounded(&path, maximum_bytes, "disposable semantic index part").map(Some)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_error("repository_index_part_metadata", &path, error)),
        }
    }

    pub(crate) fn write_index_part(
        &self,
        revision: RevisionId,
        part: DisposableIndexPart,
        bytes: &[u8],
        maximum_bytes: usize,
    ) -> Result<(), Diagnostic> {
        if bytes.len() > maximum_bytes {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_index_part_limit",
                format!("disposable semantic index part exceeds {maximum_bytes} bytes"),
            ));
        }
        let index_root = self.store.join(INDEX_OBJECTS);
        ensure_directory(&index_root, "repository_index_directory")?;
        let path = index_part_path(&self.store, revision, part);
        let parent = path.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_index_part_parent",
                "disposable index part path has no parent",
            )
        })?;
        let revision_directory = index_revision_directory(&self.store, revision);
        let revision_parent = revision_directory.parent().ok_or_else(|| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_index_revision_parent",
                "disposable index revision path has no parent",
            )
        })?;
        ensure_or_create_directory(revision_parent, "repository_index_shard")?;
        ensure_or_create_directory(&revision_directory, "repository_index_revision")?;
        if parent != revision_directory {
            ensure_or_create_directory(parent, "repository_index_part_directory")?;
        }
        if let Ok(metadata) = fs::symlink_metadata(&path)
            && (metadata.file_type().is_symlink() || !metadata.is_file())
        {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_index_part_type",
                format!(
                    "disposable index part '{}' is not a regular non-symlink file",
                    path.display()
                ),
            ));
        }
        let temporary = parent.join(format!(".index-part-stage-{}", RepositoryId::generate()?));
        let result = (|| {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)
                .map_err(|error| io_error("repository_index_part_create", &temporary, error))?;
            file.write_all(bytes)
                .map_err(|error| io_error("repository_index_part_write", &temporary, error))?;
            drop(file);
            fs::rename(&temporary, &path)
                .map_err(|error| io_error("repository_index_part_publish", &path, error))?;
            Ok(())
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }
}

fn stored_root_map_roots(root: &StoredGraphRoot) -> [super::persistent_map::MapRoot; 6] {
    [
        root.modules,
        root.module_names,
        root.dependencies,
        root.dependency_aliases,
        root.targets,
        root.tombstones,
    ]
}

fn persist_map_pages(
    store: &Path,
    pages: &MemoryPageStore,
    publication_writes: bool,
) -> Result<(), Diagnostic> {
    let mut destination = RepositoryPageStore::writer(store, publication_writes);
    for (digest, bytes) in pages.objects() {
        destination
            .write_page(digest, bytes)
            .map_err(repository_map_diagnostic)?;
    }
    Ok(())
}

fn repository_map_error(error: Diagnostic) -> MapError {
    MapError {
        class: match error.class {
            DiagnosticClass::Source | DiagnosticClass::Semantic => MapErrorClass::Input,
            DiagnosticClass::Resource => MapErrorClass::Resource,
            DiagnosticClass::Corrupt => MapErrorClass::Corrupt,
            DiagnosticClass::Capability
            | DiagnosticClass::Cancelled
            | DiagnosticClass::Infrastructure => MapErrorClass::Store,
        },
        code: "repository_map_store",
        message: format!("{}: {}", error.code, error.message),
    }
}

fn repository_map_diagnostic(error: MapError) -> Diagnostic {
    repository_error(
        match error.class {
            MapErrorClass::Input => DiagnosticClass::Source,
            MapErrorClass::Resource => DiagnosticClass::Resource,
            MapErrorClass::Corrupt => DiagnosticClass::Corrupt,
            MapErrorClass::Store => DiagnosticClass::Infrastructure,
        },
        error.code,
        error.message,
    )
}

fn validate_backup_entry(key: &BackupEntryKey, bytes: &[u8]) -> Result<(), Diagnostic> {
    match *key {
        BackupEntryKey::Module(digest) => {
            if MeaningModule::decode(bytes)?.digest()? != digest {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::MapPage(digest) => {
            if bytes.len() > super::persistent_map::MAXIMUM_PAGE_BYTES
                || PageDigest::of(bytes) != digest
            {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Root(digest) => {
            if StoredGraphRoot::decode(bytes)?.digest()? != digest {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Revision(revision) => {
            let record = RevisionRecord::decode(bytes)?;
            if record.revision != revision || record.core.revision_id()? != revision {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Receipt(digest) => {
            TransactionReceipt::decode(bytes)?;
            if ReceiptDigest::of(bytes) != digest {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Artifact(digest) => {
            if bytes.len() > MAXIMUM_ARTIFACT_OBJECT_BYTES || ArtifactDigest::of(bytes) != digest {
                return Err(backup_digest_error());
            }
            decode_package_object(bytes)?;
        }
        BackupEntryKey::Draft(id) => {
            let draft = DraftRecord::decode(bytes)?;
            if draft.id != id {
                return Err(backup_digest_error());
            }
        }
    }
    Ok(())
}

fn read_backup_entry(root: &Path, key: &BackupEntryKey) -> Result<Vec<u8>, Diagnostic> {
    let (maximum, label) = match key {
        BackupEntryKey::Module(_) => (
            super::meaning::MAXIMUM_MODULE_SEGMENT_BYTES + 50,
            "backup meaning module",
        ),
        BackupEntryKey::MapPage(_) => (
            super::persistent_map::MAXIMUM_PAGE_BYTES + 50,
            "backup persistent root page",
        ),
        BackupEntryKey::Root(_) => (
            super::graph::MAXIMUM_STORED_ROOT_BYTES + 50,
            "backup graph root",
        ),
        BackupEntryKey::Revision(_) => (
            super::revision::MAXIMUM_REVISION_BYTES + 50,
            "backup revision record",
        ),
        BackupEntryKey::Receipt(_) => (
            super::revision::MAXIMUM_RECEIPT_BYTES + 50,
            "backup transaction receipt",
        ),
        BackupEntryKey::Artifact(_) => (
            MAXIMUM_ARTIFACT_OBJECT_BYTES + 50,
            "backup dependency artifact",
        ),
        BackupEntryKey::Draft(_) => (MAXIMUM_DRAFT_BYTES + 50, "backup semantic draft"),
    };
    read_bounded(&backup_entry_path(root, key), maximum, label)
}

fn backup_entry_path(root: &Path, key: &BackupEntryKey) -> PathBuf {
    match *key {
        BackupEntryKey::Module(digest) => module_path(root, digest),
        BackupEntryKey::MapPage(digest) => map_page_path(root, digest),
        BackupEntryKey::Root(digest) => root_path(root, digest),
        BackupEntryKey::Revision(revision) => revision_path(root, revision),
        BackupEntryKey::Receipt(digest) => receipt_path(root, digest),
        BackupEntryKey::Artifact(digest) => artifact_path(root, digest),
        BackupEntryKey::Draft(id) => root.join(DRAFT_OBJECTS).join(format!("{id}.lkjd")),
    }
}

fn backup_segment_path(root: &Path, ordinal: u64, digest: [u8; 32]) -> PathBuf {
    root.join(BACKUP_SEGMENTS).join(format!(
        "{ordinal:016x}-{}.lkbs",
        super::semantic_id::encode_hex(&digest)
    ))
}

fn backup_entry_digest(bytes: &[u8]) -> [u8; 32] {
    domain_digest(BACKUP_ENTRY_DIGEST_DOMAIN, bytes)
}

fn backup_segment_digest(bytes: &[u8]) -> [u8; 32] {
    domain_digest(BACKUP_SEGMENT_REFERENCE_DIGEST_DOMAIN, bytes)
}

fn domain_digest(domain: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(domain);
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    *hasher.finalize().as_bytes()
}

fn prepare_backup_stage(output: &Path) -> Result<(PathBuf, PathBuf, PathBuf), Diagnostic> {
    let output = if output.is_absolute() {
        output.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|error| {
                repository_error(
                    DiagnosticClass::Infrastructure,
                    "semantic_backup_current_directory",
                    format!("current directory is unavailable: {error}"),
                )
            })?
            .join(output)
    };
    let file_name = output
        .file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            repository_error(
                DiagnosticClass::Source,
                "semantic_backup_output_name",
                "backup output must have a portable UTF-8 final component",
            )
        })?;
    let parent = output.parent().ok_or_else(|| {
        repository_error(
            DiagnosticClass::Source,
            "semantic_backup_output_parent",
            "backup output has no parent directory",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error("semantic_backup_output_parent", parent, error))?;
    let parent = canonical_existing(parent)?;
    ensure_directory(&parent, "semantic_backup_output_parent")?;
    let output = parent.join(file_name);
    match fs::symlink_metadata(&output) {
        Ok(_) => {
            return Err(repository_error(
                DiagnosticClass::Source,
                "semantic_backup_destination_exists",
                format!("backup destination '{}' already exists", output.display()),
            ));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(io_error("semantic_backup_destination", &output, error)),
    }
    let stage = parent.join(format!(
        ".{file_name}.backup-stage-{}",
        RepositoryId::generate()?
    ));
    Ok((output, parent, stage))
}

fn sync_backup_tree(stage: &Path) -> Result<(), Diagnostic> {
    sync_directory_tree(stage)?;
    sync_directory(stage).map_err(|error| io_error("semantic_backup_stage_sync", stage, error))
}

fn sync_directory_tree(root: &Path) -> Result<(), Diagnostic> {
    ensure_directory(root, "semantic_backup_tree_type")?;
    for entry in
        fs::read_dir(root).map_err(|error| io_error("semantic_backup_tree_read", root, error))?
    {
        let entry = entry.map_err(|error| io_error("semantic_backup_tree_entry", root, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| io_error("semantic_backup_tree_type", &path, error))?;
        if file_type.is_symlink() {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "semantic_backup_tree_symlink",
                format!(
                    "backup stage contains forbidden symlink '{}'",
                    path.display()
                ),
            ));
        }
        if file_type.is_dir() {
            sync_directory_tree(&path)?;
        } else if !file_type.is_file() {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "semantic_backup_tree_entry_type",
                format!(
                    "backup stage contains unsupported entry '{}'",
                    path.display()
                ),
            ));
        }
    }
    sync_directory(root).map_err(|error| io_error("semantic_backup_tree_sync", root, error))
}

fn remove_backup_stage(stage: &Path) {
    if stage
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with('.') && value.contains(".backup-stage-repo_"))
    {
        let _ = fs::remove_dir_all(stage);
    }
}

fn canonical_store_files(store: &Path) -> Result<Vec<PathBuf>, Diagnostic> {
    let mut files = Vec::new();
    for relative in [
        MODULE_OBJECTS,
        MAP_PAGE_OBJECTS,
        ROOT_OBJECTS,
        REVISION_OBJECTS,
        RECEIPT_OBJECTS,
        ARTIFACT_OBJECTS,
        DRAFT_OBJECTS,
    ] {
        collect_regular_files(&store.join(relative), &mut files)?;
    }
    files.sort();
    Ok(files)
}

fn collect_regular_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), Diagnostic> {
    ensure_directory(root, "repository_inventory_directory")?;
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in fs::read_dir(&directory)
            .map_err(|error| io_error("repository_inventory_read", &directory, error))?
        {
            let entry =
                entry.map_err(|error| io_error("repository_inventory_entry", &directory, error))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| io_error("repository_inventory_type", &path, error))?;
            if file_type.is_symlink() {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_inventory_symlink",
                    format!(
                        "canonical object inventory found symlink '{}'",
                        path.display()
                    ),
                ));
            }
            if file_type.is_dir() {
                pending.push(path);
            } else if file_type.is_file() {
                files.push(path);
            } else {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_inventory_entry_type",
                    format!(
                        "canonical object inventory found unsupported entry '{}'",
                        path.display()
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn directory_file_totals(root: &Path) -> Result<(u64, u64), Diagnostic> {
    let mut files = Vec::new();
    collect_regular_files(root, &mut files)?;
    files.into_iter().try_fold((0_u64, 0_u64), |totals, path| {
        let bytes = fs::symlink_metadata(&path)
            .map_err(|error| io_error("repository_inventory_metadata", &path, error))?
            .len();
        Ok((
            totals
                .0
                .checked_add(1)
                .ok_or_else(|| count_overflow("derived object"))?,
            totals
                .1
                .checked_add(bytes)
                .ok_or_else(|| count_overflow("derived bytes"))?,
        ))
    })
}

fn canonical_object_path_shape(relative: &Path) -> bool {
    let parts = relative
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect::<Option<Vec<_>>>();
    let Some(parts) = parts else {
        return false;
    };
    match parts.as_slice() {
        ["objects", "modules", shard, file] => canonical_sharded_name(shard, file, "lkjm"),
        ["objects", "map-pages", shard, file] => canonical_sharded_name(shard, file, "lkjp"),
        ["objects", "roots", shard, file] => canonical_sharded_name(shard, file, "lkjr"),
        ["revisions", shard, file] => canonical_sharded_name(shard, file, "lkjv"),
        ["receipts", shard, file] => canonical_sharded_name(shard, file, "lkjt"),
        ["artifacts", shard, file] => canonical_sharded_name(shard, file, "lkja"),
        ["drafts", file] => file
            .strip_suffix(".lkjd")
            .and_then(|name| name.parse::<DraftId>().ok())
            .is_some(),
        _ => false,
    }
}

fn canonical_sharded_name(shard: &str, file: &str, extension: &str) -> bool {
    let Some(encoded) = file.strip_suffix(&format!(".{extension}")) else {
        return false;
    };
    encoded.len() == 64
        && shard.len() == 2
        && encoded.starts_with(shard)
        && encoded
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hash_file(path: &Path) -> Result<[u8; 32], Diagnostic> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("repository_inventory_metadata", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "repository_inventory_file_type",
            format!(
                "inventory candidate '{}' is not a regular file",
                path.display()
            ),
        ));
    }
    let maximum = u64::try_from(MAXIMUM_ARTIFACT_OBJECT_BYTES + 50)
        .map_err(|_| count_overflow("inventory file bound"))?;
    if metadata.len() > maximum {
        return Err(repository_error(
            DiagnosticClass::Resource,
            "repository_inventory_file_limit",
            format!(
                "inventory candidate '{}' exceeds the largest canonical object bound",
                path.display()
            ),
        ));
    }
    let mut file = File::open(path)
        .map_err(|error| io_error("repository_inventory_file_open", path, error))?;
    let mut hasher = blake3::Hasher::new_derive_key(CLEANUP_CANDIDATE_DIGEST_DOMAIN);
    hasher.update(&metadata.len().to_be_bytes());
    let mut buffer = [0_u8; 64 * 1024];
    let mut observed = 0_u64;
    loop {
        let count = file
            .read(&mut buffer)
            .map_err(|error| io_error("repository_inventory_file_read", path, error))?;
        if count == 0 {
            break;
        }
        observed = observed
            .checked_add(u64::try_from(count).map_err(|_| count_overflow("inventory read"))?)
            .ok_or_else(|| count_overflow("inventory read"))?;
        hasher.update(&buffer[..count]);
    }
    if observed != metadata.len() {
        return Err(repository_error(
            DiagnosticClass::Infrastructure,
            "repository_inventory_file_changed",
            format!(
                "inventory candidate '{}' changed while read",
                path.display()
            ),
        ));
    }
    Ok(*hasher.finalize().as_bytes())
}

fn retention_plan_digest(
    repository: RepositoryId,
    revision: RevisionId,
    retained: &BTreeSet<BackupEntryKey>,
    reclaimable: &[(PathBuf, u64, [u8; 32])],
    unknown_entries: u64,
) -> Result<CleanupDigest, Diagnostic> {
    let mut hasher = blake3::Hasher::new_derive_key(CLEANUP_PLAN_DIGEST_DOMAIN);
    hasher.update(&repository.bytes());
    hasher.update(&revision.bytes());
    hasher.update(
        &u64::try_from(retained.len())
            .map_err(|_| count_overflow("retention digest entries"))?
            .to_be_bytes(),
    );
    let configuration = bincode::config::standard()
        .with_little_endian()
        .with_variable_int_encoding();
    for key in retained {
        let bytes = bincode::encode_to_vec(key, configuration).map_err(|error| {
            repository_error(
                DiagnosticClass::Infrastructure,
                "repository_retention_digest_encode",
                format!("retained object key could not be encoded: {error}"),
            )
        })?;
        hasher.update(
            &u64::try_from(bytes.len())
                .map_err(|_| count_overflow("retention key bytes"))?
                .to_be_bytes(),
        );
        hasher.update(&bytes);
    }
    hasher.update(
        &u64::try_from(reclaimable.len())
            .map_err(|_| count_overflow("reclaimable digest entries"))?
            .to_be_bytes(),
    );
    for (path, bytes, digest) in reclaimable {
        let encoded = path.as_os_str().as_encoded_bytes();
        hasher.update(
            &u64::try_from(encoded.len())
                .map_err(|_| count_overflow("retention path bytes"))?
                .to_be_bytes(),
        );
        hasher.update(encoded);
        hasher.update(&bytes.to_be_bytes());
        hasher.update(digest);
    }
    hasher.update(&unknown_entries.to_be_bytes());
    Ok(CleanupDigest::from_bytes(*hasher.finalize().as_bytes()))
}

fn backup_digest_error() -> Diagnostic {
    repository_error(
        DiagnosticClass::Corrupt,
        "semantic_backup_object_digest",
        "backup object bytes do not match their typed canonical key",
    )
}

fn initialize_stage(
    store: &Path,
    initial: InitialPublication,
) -> Result<(SemanticHead, TransactionReceipt), Diagnostic> {
    let InitialPublication {
        root,
        modules,
        transaction,
        semantic_diff,
        intent,
        validation_profile,
        dependency_artifacts,
        status,
    } = initial;
    for artifact in &dependency_artifacts {
        write_artifact_at(store, artifact)?;
    }
    for module in &modules {
        let bytes = module.encode()?;
        write_immutable(
            &module_path(store, module.digest()?),
            &bytes,
            "meaning module object",
        )?;
    }
    let stored = StoredGraphRoot::build(&root)?;
    persist_map_pages(store, &stored.pages, false)?;
    let root_bytes = stored.root.encode()?;
    let root_digest = stored.root.digest()?;
    write_immutable(
        &root_path(store, root_digest),
        &root_bytes,
        "graph root object",
    )?;
    let semantic_summaries = build_semantic_summaries(&root.package_id, &modules)?;
    let mut semantic_facts = build_semantic_facts(
        root.repository_id,
        &root.package_id,
        RevisionId::from_digest([0; 32]),
        root_digest,
        &semantic_summaries,
    )?;
    let core = RevisionCore {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: root.repository_id,
        parents: Vec::new(),
        root: root_digest,
        semantic_certificate: semantic_facts.manifest.certificate,
        semantic_diff,
        transaction,
    };
    let revision = core.revision_id()?;
    semantic_facts.manifest = semantic_facts.manifest.rebind_revision(revision)?;
    let validated = validate_repository_graph(store, &root, &modules, Some(revision))?;
    let mut validation = validation_facts(&validated)?;
    if let Some(profile) = validation_profile {
        validation.profile = profile;
    }
    let repository = SemanticRepository {
        project_root: PathBuf::new(),
        store: store.to_path_buf(),
    };
    let _ = repository.write_semantic_cache(&semantic_facts, &semantic_summaries);
    // The initial graph is already fully materialized and validated. Seed the disposable exact
    // owner/name generation without creating the broad relation index; failure cannot change the
    // staged canonical authority and is recovered lazily.
    let _ =
        SemanticQueryIndex::seed_local_index(&repository, revision, root_digest, &root, &modules);
    let receipt = TransactionReceipt {
        contract_version: RECEIPT_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: root.repository_id,
        status,
        base: None,
        result: revision,
        transaction,
        idempotency_key: None,
        semantic_diff,
        affected_owners: Vec::new(),
        validation,
        intent,
    };
    let receipt_bytes = receipt.encode()?;
    let receipt_digest = receipt.digest()?;
    write_immutable(
        &receipt_path(store, receipt_digest),
        &receipt_bytes,
        "transaction receipt",
    )?;
    let record = RevisionRecord::new(core, receipt_digest)?;
    let record_bytes = record.encode()?;
    let record_digest = record.digest()?;
    write_immutable(
        &revision_path(store, revision),
        &record_bytes,
        "revision record",
    )?;
    sync_publication_objects(store)?;
    let head = SemanticHead {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: root.repository_id,
        revision,
        record: record_digest,
    };
    let head_bytes = head.encode()?;
    write_immutable(&store.join(HEAD_FILE), &head_bytes, "semantic HEAD")?;
    sync_directory(store).map_err(|error| io_error("repository_stage_sync", store, error))?;
    Ok((head, receipt))
}

fn validate_repository_graph(
    store: &Path,
    root: &GraphRoot,
    modules: &[MeaningModule],
    accepted_revision: Option<RevisionId>,
) -> Result<ValidatedPackage, Diagnostic> {
    let mut loaded = Vec::with_capacity(root.dependencies.len());
    for binding in &root.dependencies {
        loaded.push(load_stored_artifact_closure(store, binding.artifact)?);
    }
    let mut exact = Vec::with_capacity(root.dependencies.len());
    for (binding, artifact) in root.dependencies.iter().zip(&loaded) {
        let package = artifact.packages.get(&binding.package_id).ok_or_else(|| {
            repository_error(
                DiagnosticClass::Semantic,
                "repository_dependency_package_missing",
                format!(
                    "dependency artifact '{}' does not contain package '{}'",
                    binding.alias,
                    binding.package_id.as_str()
                ),
            )
        })?;
        exact.push(ExactGraphDependency {
            alias: &binding.alias,
            package,
            artifact: binding.artifact,
        });
    }
    let validated = validate_graph_package(root, modules.to_vec(), &exact, accepted_revision)?;

    let reconstructed_root = GraphRoot::decode(&root.encode()?)?;
    let reconstructed_modules = modules
        .iter()
        .map(|module| MeaningModule::decode(&module.encode()?))
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    let oracle = validate_graph_package(
        &reconstructed_root,
        reconstructed_modules,
        &exact,
        accepted_revision,
    )?;
    if validated != oracle {
        return Err(repository_error(
            DiagnosticClass::Infrastructure,
            "repository_full_oracle_mismatch",
            "direct graph validation disagrees with independent packed reconstruction",
        ));
    }
    Ok(validated)
}

fn build_semantic_summaries(
    package: &super::package::PackageId,
    modules: &[MeaningModule],
) -> Result<Vec<ModuleSemanticSummary>, Diagnostic> {
    let mut summaries = modules
        .iter()
        .map(|module| build_module_summary(package, module))
        .collect::<Result<Vec<_>, Diagnostic>>()?;
    summaries.sort_by_key(|summary| summary.module);
    Ok(summaries)
}

fn load_stored_artifact_closure(
    store: &Path,
    root: ArtifactDigest,
) -> Result<super::artifact::LoadedArtifact, Diagnostic> {
    let mut objects = BTreeMap::new();
    let mut pending = vec![root];
    while let Some(digest) = pending.pop() {
        if objects.contains_key(&digest) {
            continue;
        }
        let bytes = read_bounded(
            &artifact_path(store, digest),
            MAXIMUM_ARTIFACT_OBJECT_BYTES,
            "dependency package artifact",
        )?;
        if ArtifactDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_dependency_artifact_digest",
                "dependency package artifact does not match its canonical key",
            ));
        }
        let package = decode_package_object(&bytes)?;
        pending.extend(package.dependencies());
        objects.insert(digest, bytes);
        if objects.len() > MAXIMUM_ARTIFACT_PACKAGES {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_dependency_artifact_count",
                "dependency package closure exceeds its package bound",
            ));
        }
    }
    load_package_object_closure(root, objects)
}

fn collect_stored_artifact_keys(
    store: &Path,
    root: ArtifactDigest,
    entries: &mut BTreeSet<BackupEntryKey>,
) -> Result<(), Diagnostic> {
    let mut pending = vec![root];
    let mut seen = BTreeSet::new();
    while let Some(digest) = pending.pop() {
        if !seen.insert(digest) {
            continue;
        }
        let bytes = read_bounded(
            &artifact_path(store, digest),
            MAXIMUM_ARTIFACT_OBJECT_BYTES,
            "dependency package artifact",
        )?;
        if ArtifactDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_dependency_artifact_digest",
                "dependency package artifact does not match its canonical key",
            ));
        }
        let package = decode_package_object(&bytes)?;
        pending.extend(package.dependencies());
        entries.insert(BackupEntryKey::Artifact(digest));
        if seen.len() > MAXIMUM_ARTIFACT_PACKAGES {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "repository_dependency_artifact_count",
                "dependency package closure exceeds its package bound",
            ));
        }
    }
    Ok(())
}

fn validation_facts(package: &ValidatedPackage) -> Result<ValidationFacts, Diagnostic> {
    let declarations = package.modules.iter().try_fold(0usize, |total, module| {
        total.checked_add(module.module.declarations.len())
    });
    let declarations = declarations.ok_or_else(|| count_overflow("declaration"))?;
    Ok(ValidationFacts {
        profile: "full_graph_and_reconstruction".to_owned(),
        graph_valid: true,
        full_oracle_equal: true,
        modules_checked: u64::try_from(package.modules.len())
            .map_err(|_| count_overflow("module"))?,
        declarations_checked: u64::try_from(declarations)
            .map_err(|_| count_overflow("declaration"))?,
    })
}

fn write_artifact_at(store: &Path, artifact: &DependencyArtifactObject) -> Result<(), Diagnostic> {
    if artifact.bytes.len() > MAXIMUM_ARTIFACT_OBJECT_BYTES
        || ArtifactDigest::of(&artifact.bytes) != artifact.digest
    {
        return Err(repository_error(
            DiagnosticClass::Source,
            "repository_artifact_digest",
            "dependency artifact exceeds its bound or does not match the declared digest",
        ));
    }
    write_immutable(
        &artifact_path(store, artifact.digest),
        &artifact.bytes,
        "dependency artifact",
    )
}

fn create_store_layout(store: &Path) -> Result<(), Diagnostic> {
    fs::create_dir(store).map_err(|error| io_error("repository_stage_create", store, error))?;
    for relative in [
        MODULE_OBJECTS,
        MAP_PAGE_OBJECTS,
        ROOT_OBJECTS,
        REVISION_OBJECTS,
        RECEIPT_OBJECTS,
        ARTIFACT_OBJECTS,
        DRAFT_OBJECTS,
        INDEX_OBJECTS,
    ] {
        let path = store.join(relative);
        fs::create_dir_all(&path)
            .map_err(|error| io_error("repository_layout_create", &path, error))?;
    }
    let lock_path = store.join(LOCK_FILE);
    write_new_file(&lock_path, &[], "repository lock")?;
    sync_directory(store).map_err(|error| io_error("repository_layout_sync", store, error))
}

fn replace_head(store: &Path, head: &SemanticHead) -> Result<(), Diagnostic> {
    let identity = RepositoryId::generate()?;
    let temporary = store.join(format!(".HEAD-{}", identity));
    let bytes = head.encode()?;
    write_new_file(&temporary, &bytes, "temporary semantic HEAD")?;
    fs::rename(&temporary, store.join(HEAD_FILE)).map_err(|error| {
        let _ = fs::remove_file(&temporary);
        io_error("repository_head_rename", &store.join(HEAD_FILE), error)
    })?;
    sync_directory(store).map_err(|error| {
        repository_error(
            DiagnosticClass::Infrastructure,
            "repository_visibility_indeterminate",
            format!("HEAD rename succeeded but directory durability is indeterminate: {error}"),
        )
    })
}

fn sync_publication_objects(store: &Path) -> Result<(), Diagnostic> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        let directory =
            File::open(store).map_err(|error| io_error("repository_syncfs_open", store, error))?;
        rustix::fs::syncfs(directory).map_err(|error| {
            io_error(
                "repository_syncfs",
                store,
                std::io::Error::from_raw_os_error(error.raw_os_error()),
            )
        })?;
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    for relative in [
        MODULE_OBJECTS,
        MAP_PAGE_OBJECTS,
        ROOT_OBJECTS,
        REVISION_OBJECTS,
        RECEIPT_OBJECTS,
        ARTIFACT_OBJECTS,
        DRAFT_OBJECTS,
    ] {
        let path = store.join(relative);
        sync_tree_directories(&path)?;
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        sync_directory(&store.join("objects"))
            .map_err(|error| io_error("repository_sync_objects", &store.join("objects"), error))?;
        sync_directory(store).map_err(|error| io_error("repository_sync_store", store, error))
    }
}

fn sync_tree_directories(root: &Path) -> Result<(), Diagnostic> {
    ensure_directory(root, "repository_sync_tree_type")?;
    for entry in
        fs::read_dir(root).map_err(|error| io_error("repository_sync_read", root, error))?
    {
        let entry = entry.map_err(|error| io_error("repository_sync_entry", root, error))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|error| io_error("repository_sync_type", &path, error))?;
        if file_type.is_symlink() {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_sync_symlink",
                format!("repository contains forbidden symlink '{}'", path.display()),
            ));
        }
        if file_type.is_dir() {
            sync_directory(&path)
                .map_err(|error| io_error("repository_sync_directory", &path, error))?;
        }
    }
    sync_directory(root).map_err(|error| io_error("repository_sync_root", root, error))
}

fn write_immutable(path: &Path, bytes: &[u8], label: &str) -> Result<(), Diagnostic> {
    write_immutable_with_sync(path, bytes, label, true)
}

fn write_publication_immutable(path: &Path, bytes: &[u8], label: &str) -> Result<(), Diagnostic> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        write_immutable_with_sync(path, bytes, label, false)
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    {
        write_immutable(path, bytes, label)
    }
}

fn write_disposable_content(
    path: &Path,
    bytes: &[u8],
    label: &str,
) -> Result<PageWrite, Diagnostic> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_disposable_type",
                format!(
                    "existing {label} '{}' is not a regular file",
                    path.display()
                ),
            ));
        }
        if read_bounded(path, bytes.len(), label).is_ok_and(|existing| existing == bytes) {
            return Ok(PageWrite::Reused);
        }
    }
    let parent = path.parent().ok_or_else(|| {
        repository_error(
            DiagnosticClass::Infrastructure,
            "repository_disposable_parent",
            "disposable repository object path has no parent",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error("repository_disposable_parent", parent, error))?;
    let temporary = parent.join(format!(".disposable-stage-{}", RepositoryId::generate()?));
    let result = (|| {
        write_new_file_with_sync(&temporary, bytes, label, false)?;
        fs::rename(&temporary, path)
            .map_err(|error| io_error("repository_disposable_publish", path, error))?;
        Ok(PageWrite::Inserted)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn write_immutable_with_sync(
    path: &Path,
    bytes: &[u8],
    label: &str,
    sync_file: bool,
) -> Result<(), Diagnostic> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_immutable_type",
                format!(
                    "existing {label} '{}' is not a regular file",
                    path.display()
                ),
            ));
        }
        let existing = read_bounded(path, bytes.len(), label)?;
        if existing == bytes {
            return Ok(());
        }
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "repository_immutable_conflict",
            format!("existing {label} '{}' has foreign bytes", path.display()),
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        repository_error(
            DiagnosticClass::Infrastructure,
            "repository_object_parent",
            "repository object path has no parent",
        )
    })?;
    fs::create_dir_all(parent)
        .map_err(|error| io_error("repository_object_parent", parent, error))?;
    write_new_file_with_sync(path, bytes, label, sync_file)
}

fn write_new_file(path: &Path, bytes: &[u8], label: &str) -> Result<(), Diagnostic> {
    write_new_file_with_sync(path, bytes, label, true)
}

fn write_new_file_with_sync(
    path: &Path,
    bytes: &[u8],
    label: &str,
    sync_file: bool,
) -> Result<(), Diagnostic> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| io_error("repository_file_create", path, error))?;
    file.write_all(bytes).map_err(|error| {
        io_error(
            "repository_file_write",
            path,
            std::io::Error::new(error.kind(), format!("{label}: {error}")),
        )
    })?;
    if sync_file {
        file.sync_all().map_err(|error| {
            io_error(
                "repository_file_write",
                path,
                std::io::Error::new(error.kind(), format!("{label}: {error}")),
            )
        })?;
    }
    Ok(())
}

fn read_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("repository_read_metadata", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(repository_error(
            DiagnosticClass::Corrupt,
            "repository_read_type",
            format!(
                "{label} '{}' is not a regular non-symlink file",
                path.display()
            ),
        ));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| {
        repository_error(
            DiagnosticClass::Resource,
            "repository_read_length",
            format!("{label} length cannot be represented"),
        )
    })?;
    if length > maximum {
        return Err(repository_error(
            DiagnosticClass::Resource,
            "repository_read_limit",
            format!("{label} exceeds {maximum} bytes"),
        ));
    }
    let mut file =
        File::open(path).map_err(|error| io_error("repository_read_open", path, error))?;
    let mut bytes = Vec::with_capacity(length);
    Read::by_ref(&mut file)
        .take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("repository_read", path, error))?;
    if bytes.len() != length || bytes.len() > maximum {
        return Err(repository_error(
            DiagnosticClass::Resource,
            "repository_read_changed",
            format!("{label} changed during its bounded read"),
        ));
    }
    Ok(bytes)
}

fn module_path(store: &Path, digest: super::semantic_digest::ModuleObjectDigest) -> PathBuf {
    sharded_digest_path(store, MODULE_OBJECTS, &digest.bytes(), "lkjm")
}

fn map_page_path(store: &Path, digest: PageDigest) -> PathBuf {
    sharded_digest_path(store, MAP_PAGE_OBJECTS, &digest.bytes(), "lkjp")
}

fn root_path(store: &Path, digest: RootObjectDigest) -> PathBuf {
    sharded_digest_path(store, ROOT_OBJECTS, &digest.bytes(), "lkjr")
}

fn receipt_path(store: &Path, digest: ReceiptDigest) -> PathBuf {
    sharded_digest_path(store, RECEIPT_OBJECTS, &digest.bytes(), "lkjt")
}

fn artifact_path(store: &Path, digest: ArtifactDigest) -> PathBuf {
    sharded_digest_path(store, ARTIFACT_OBJECTS, &digest.bytes(), "lkja")
}

fn revision_path(store: &Path, revision: RevisionId) -> PathBuf {
    sharded_digest_path(store, REVISION_OBJECTS, &revision.bytes(), "lkjv")
}

fn index_path(store: &Path, revision: RevisionId) -> PathBuf {
    sharded_digest_path(store, INDEX_OBJECTS, &revision.bytes(), "lkji")
}

fn index_revision_directory(store: &Path, revision: RevisionId) -> PathBuf {
    let encoded = super::semantic_id::encode_hex(&revision.bytes());
    store.join(INDEX_OBJECTS).join(&encoded[..2]).join(encoded)
}

fn index_part_path(store: &Path, revision: RevisionId, part: DisposableIndexPart) -> PathBuf {
    let revision = index_revision_directory(store, revision);
    match part {
        DisposableIndexPart::Manifest => revision.join("local-manifest.lkix"),
        DisposableIndexPart::SemanticFacts => revision.join("facts.lkix"),
    }
}

fn local_index_object_path(
    store: &Path,
    kind: LocalIndexObjectKind,
    digest: IndexDigest,
) -> PathBuf {
    let directory = match kind {
        LocalIndexObjectKind::Owner => LOCAL_INDEX_OWNER_OBJECTS,
        LocalIndexObjectKind::Name => LOCAL_INDEX_NAME_OBJECTS,
    };
    sharded_digest_path(store, directory, &digest.bytes(), "lkix")
}

fn summary_object_path(store: &Path, digest: SemanticSummaryDigest) -> PathBuf {
    sharded_digest_path(store, SUMMARY_INDEX_OBJECTS, &digest.bytes(), "lkss")
}

fn semantic_fact_page_path(store: &Path, digest: PageDigest) -> PathBuf {
    sharded_digest_path(store, SEMANTIC_FACT_PAGE_OBJECTS, &digest.bytes(), "lksp")
}

fn sharded_digest_path(store: &Path, directory: &str, bytes: &[u8], extension: &str) -> PathBuf {
    let encoded = super::semantic_id::encode_hex(bytes);
    store
        .join(directory)
        .join(&encoded[..2])
        .join(format!("{encoded}.{extension}"))
}

fn ensure_or_create_directory(path: &Path, code: &str) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(path) {
        Ok(_) => ensure_directory(path, code),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => match fs::create_dir(path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                ensure_directory(path, code)
            }
            Err(error) => Err(io_error(code, path, error)),
        },
        Err(error) => Err(io_error(code, path, error)),
    }
}

fn ensure_or_create_empty_file(path: &Path, code: &str) -> Result<(), Diagnostic> {
    let validate = || -> Result<(), Diagnostic> {
        let metadata = fs::symlink_metadata(path).map_err(|error| io_error(code, path, error))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() != 0 {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                code,
                format!(
                    "'{}' is not an empty regular non-symlink file",
                    path.display()
                ),
            ));
        }
        Ok(())
    };
    match fs::symlink_metadata(path) {
        Ok(_) => validate(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match OpenOptions::new().create_new(true).write(true).open(path) {
                Ok(file) => file.sync_all().map_err(|error| io_error(code, path, error)),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => validate(),
                Err(error) => Err(io_error(code, path, error)),
            }
        }
        Err(error) => Err(io_error(code, path, error)),
    }
}

fn ensure_directory(path: &Path, code: &str) -> Result<(), Diagnostic> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(code, path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(repository_error(
            DiagnosticClass::Source,
            code,
            format!(
                "'{}' is not a regular non-symlink directory",
                path.display()
            ),
        ));
    }
    Ok(())
}

fn canonical_existing(path: &Path) -> Result<PathBuf, Diagnostic> {
    path.canonicalize()
        .map_err(|error| io_error("repository_canonical_path", path, error))
}

fn sync_directory(path: &Path) -> std::io::Result<()> {
    File::open(path)?.sync_all()
}

fn remove_owned_stage(stage: &Path) {
    if stage
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|value| value.starts_with(".meaning-stage-repo_"))
    {
        let _ = fs::remove_dir_all(stage);
    }
}

fn checked_increment(value: usize, label: &str) -> Result<usize, Diagnostic> {
    value.checked_add(1).ok_or_else(|| count_overflow(label))
}

fn count_overflow(label: &str) -> Diagnostic {
    repository_error(
        DiagnosticClass::Resource,
        "repository_count_overflow",
        format!("{label} count overflowed"),
    )
}

fn repository_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn io_error(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    repository_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        GRAPH_CONTRACT_VERSION, MigrationIdentityAllocator, ModuleObjectRef, PackageId,
        SourceLimits, parse_module, parse_source,
    };

    fn directory_snapshot(root: &Path) -> Vec<(PathBuf, Vec<u8>)> {
        fn visit(root: &Path, path: &Path, output: &mut Vec<(PathBuf, Vec<u8>)>) {
            let mut entries = fs::read_dir(path)
                .expect("read snapshot directory")
                .map(|entry| entry.expect("snapshot entry"))
                .collect::<Vec<_>>();
            entries.sort_by_key(|entry| entry.file_name());
            for entry in entries {
                let path = entry.path();
                if entry.file_type().expect("snapshot type").is_dir() {
                    visit(root, &path, output);
                } else {
                    output.push((
                        path.strip_prefix(root)
                            .expect("relative snapshot")
                            .to_path_buf(),
                        fs::read(path).expect("snapshot bytes"),
                    ));
                }
            }
        }
        let mut output = Vec::new();
        visit(root, root, &mut output);
        output
    }

    fn fixture() -> (GraphRoot, Vec<MeaningModule>) {
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source");
        let module = parse_module(&document).expect("module");
        let mut allocator = MigrationIdentityAllocator::new(b"repository-fixture".to_vec());
        let meaning = MeaningModule::import(module, &mut allocator).expect("meaning");
        let root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"repository-fixture", 1),
            package_id: PackageId::parse("10000000000000000000000000000001").expect("package"),
            package_name: "fixture".to_owned(),
            modules: vec![ModuleObjectRef {
                id: meaning.module_id,
                name: meaning.module.name.clone(),
                object: meaning.digest().expect("module digest"),
            }],
            dependencies: Vec::new(),
            targets: Vec::new(),
            tombstones: Vec::new(),
        };
        (root, vec![meaning])
    }

    #[test]
    fn exact_publication_is_atomic_no_change_and_stale_are_nonpublishing() {
        let temporary = tempfile::TempDir::new().expect("temporary project");
        let (root, modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root: root.clone(),
                modules: modules.clone(),
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");
        let initial = repository.reconstruct_current().expect("reconstruct");
        assert!(
            repository
                .read_index_part(
                    initial.current.head.revision,
                    DisposableIndexPart::Manifest,
                    64 * 1024 + 50,
                )
                .expect("read initial exact index manifest")
                .is_some(),
            "initial publication seeds the disposable exact index"
        );

        let (no_change, receipt) = repository
            .publish(PublicationProposal {
                expected_base: initial.current.head.revision,
                repository_id: initial.current.head.repository_id,
                root: Some(root.clone()),
                modules: modules.clone(),
                transaction: TransactionDigest::of(b"no-change"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"empty"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: Vec::new(),
                intent: None,
                dependency_artifacts: Vec::new(),
                prepared_validation: None,
            })
            .expect("no change");
        assert!(matches!(
            no_change,
            PublicationOutcome::SemanticNoChange { .. }
        ));
        assert!(receipt.is_none());

        let mut renamed_modules = modules;
        renamed_modules[0].module.declarations[0] =
            match renamed_modules[0].module.declarations[0].clone() {
                super::super::language::Declaration::Record(mut record) => {
                    record.name = "Entry".to_owned();
                    super::super::language::Declaration::Record(record)
                }
                _ => unreachable!("fixture record"),
            };
        renamed_modules[0].declarations[0].name = "Entry".to_owned();
        let mut renamed_root = root;
        renamed_root.modules[0].object = renamed_modules[0].digest().expect("renamed digest");

        let foreign_base = RevisionId::from_digest([7; 32]);
        let (stale, receipt) = repository
            .publish(PublicationProposal {
                expected_base: foreign_base,
                repository_id: initial.current.head.repository_id,
                root: Some(renamed_root.clone()),
                modules: renamed_modules.clone(),
                transaction: TransactionDigest::of(b"stale"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"rename"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: Vec::new(),
                intent: None,
                dependency_artifacts: Vec::new(),
                prepared_validation: None,
            })
            .expect("stale");
        assert!(matches!(stale, PublicationOutcome::StaleBase { .. }));
        assert!(receipt.is_none());

        let declaration_id = renamed_modules[0].declarations[0].id;
        let (accepted, receipt) = repository
            .publish(PublicationProposal {
                expected_base: initial.current.head.revision,
                repository_id: initial.current.head.repository_id,
                root: Some(renamed_root),
                modules: renamed_modules,
                transaction: TransactionDigest::of(b"rename"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"rename"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: vec![AffectedOwner::Declaration(declaration_id)],
                intent: Some("rename fixture record".to_owned()),
                dependency_artifacts: Vec::new(),
                prepared_validation: None,
            })
            .expect("accepted");
        assert!(matches!(accepted, PublicationOutcome::Accepted { .. }));
        assert_eq!(receipt.expect("receipt").affected_owners.len(), 1);
        let reconstructed = repository.reconstruct_current().expect("current");
        assert_eq!(reconstructed.modules[0].declarations[0].id, declaration_id);
        assert!(
            repository
                .read_index_part(
                    reconstructed.current.head.revision,
                    DisposableIndexPart::Manifest,
                    64 * 1024 + 50,
                )
                .expect("read full-publication exact index manifest")
                .is_some(),
            "full-candidate publication seeds the disposable exact index"
        );
        assert_eq!(repository.history(None, 10).expect("history").len(), 2);
        assert_eq!(
            repository.doctor(true).expect("doctor").revisions_checked,
            2
        );
    }

    #[test]
    fn two_parent_merge_publication_retains_and_checks_the_complete_dag() {
        use crate::platform::semantic_merge::{
            SEMANTIC_MERGE_CONTRACT_VERSION, SemanticMergeRequest, SemanticMergeStatus,
            merge_revisions,
        };

        let temporary = tempfile::TempDir::new().expect("temporary project");
        let (base_root, base_modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root: base_root.clone(),
                modules: base_modules.clone(),
                transaction: TransactionDigest::of(b"merge base"),
                semantic_diff: SemanticDiffDigest::of(b"merge base"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize merge fixture");
        let base = repository.reconstruct_current().expect("base");

        let mut left_root = base_root.clone();
        let mut left_modules = base_modules.clone();
        let declaration_id = left_modules[0].declarations[0].id;
        left_modules[0].declarations[0].name = "ItemLeft".to_owned();
        let super::super::language::Declaration::Record(record) =
            &mut left_modules[0].module.declarations[0]
        else {
            panic!("record fixture");
        };
        record.name = "ItemLeft".to_owned();
        left_root.modules[0].object = left_modules[0].digest().expect("left module");
        let (left_outcome, _) = repository
            .publish(PublicationProposal {
                expected_base: base.current.head.revision,
                repository_id: base.current.head.repository_id,
                root: Some(left_root),
                modules: left_modules,
                transaction: TransactionDigest::of(b"left"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"left"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: vec![AffectedOwner::Declaration(declaration_id)],
                intent: None,
                dependency_artifacts: Vec::new(),
                prepared_validation: None,
            })
            .expect("publish left");
        let PublicationOutcome::Accepted {
            revision: left_revision,
            ..
        } = left_outcome
        else {
            panic!("left accepted");
        };

        replace_head(&repository.store, &base.current.head).expect("restore base head for branch");
        let mut right_root = base_root;
        right_root.package_name = "fixture-right".to_owned();
        let (right_outcome, _) = repository
            .publish(PublicationProposal {
                expected_base: base.current.head.revision,
                repository_id: base.current.head.repository_id,
                root: Some(right_root),
                modules: base_modules,
                transaction: TransactionDigest::of(b"right"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"right"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: Vec::new(),
                intent: None,
                dependency_artifacts: Vec::new(),
                prepared_validation: None,
            })
            .expect("publish right");
        let PublicationOutcome::Accepted {
            revision: right_revision,
            ..
        } = right_outcome
        else {
            panic!("right accepted");
        };

        let merged = merge_revisions(
            &repository,
            &SemanticMergeRequest {
                contract_version: SEMANTIC_MERGE_CONTRACT_VERSION,
                base_revision: base.current.head.revision,
                left_revision,
                right_revision,
                maximum_work: 100,
                intent: Some("merge fixture branches".to_owned()),
            },
            true,
        )
        .expect("merge branches");
        assert_eq!(merged.status, SemanticMergeStatus::AcceptedChange);
        assert!(merged.conflicts.is_empty());
        let current = repository.reconstruct_current().expect("merged current");
        assert_eq!(current.current.record.core.parents.len(), 2);
        assert_eq!(current.current.root.package_name, "fixture-right");
        assert_eq!(current.modules[0].declarations[0].name, "ItemLeft");
        assert_eq!(repository.history(None, 10).expect("DAG history").len(), 4);
        assert_eq!(
            repository
                .doctor(true)
                .expect("deep DAG doctor")
                .revisions_checked,
            4
        );
    }

    #[test]
    fn corrupted_module_never_decodes_as_authority() {
        let temporary = tempfile::TempDir::new().expect("temporary project");
        let (root, modules) = fixture();
        let digest = root.modules[0].object;
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");
        let path = module_path(&repository.store, digest);
        let mut bytes = fs::read(&path).expect("read module");
        bytes[20] ^= 1;
        fs::write(path, bytes).expect("corrupt fixture");
        assert!(repository.reconstruct_current().is_err());
    }

    #[test]
    fn open_recreates_only_transport_omitted_operational_state() {
        let temporary = tempfile::TempDir::new().expect("temporary project");
        let (root, modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");
        let expected = repository.current().expect("current").head;
        fs::remove_dir(repository.store.join(DRAFT_OBJECTS)).expect("remove empty drafts");
        fs::remove_dir_all(repository.store.join(INDEX_OBJECTS))
            .expect("remove disposable indexes");
        fs::remove_file(repository.store.join(LOCK_FILE)).expect("remove lock");
        drop(repository);

        let reopened =
            SemanticRepository::open(temporary.path()).expect("reopen transported graph");
        assert_eq!(reopened.current().expect("reopened current").head, expected);
        assert!(reopened.store.join(DRAFT_OBJECTS).is_dir());
        assert!(reopened.store.join(INDEX_OBJECTS).is_dir());
        assert!(reopened.store.join(LOCK_FILE).is_file());
        assert_eq!(
            fs::metadata(reopened.store.join(LOCK_FILE))
                .expect("lock metadata")
                .len(),
            0
        );
        reopened
            .backup_to(&temporary.path().join("transport.lkjb"))
            .expect("backup after transport");
    }

    #[test]
    fn backup_is_deterministic_and_restore_reconstructs_exact_authority() {
        let source = tempfile::TempDir::new().expect("temporary source project");
        let (root, modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            source.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");
        let first = source.path().join("first.lkjb");
        let second = source.path().join("second.lkjb");
        let first_receipt = repository.backup_to(&first).expect("first backup");
        let second_receipt = repository.backup_to(&second).expect("second backup");
        assert_eq!(directory_snapshot(&first), directory_snapshot(&second));
        assert_eq!(first_receipt, second_receipt);
        assert!(repository.backup_to(&first).is_err());

        let destination = tempfile::TempDir::new().expect("temporary restore project");
        let (restored, receipt) =
            SemanticRepository::restore_backup_from(destination.path(), &first).expect("restore");
        assert!(receipt.deep_valid);
        assert_eq!(receipt.digest, first_receipt.digest);
        assert_eq!(
            restored.current().expect("restored current").head,
            repository.current().expect("source current").head
        );
        assert_eq!(
            restored.build_artifact().expect("restored artifact").0,
            repository.build_artifact().expect("source artifact").0
        );

        let corrupt_destination = tempfile::TempDir::new().expect("corrupt restore project");
        let segment = fs::read_dir(first.join(BACKUP_SEGMENTS))
            .expect("segments")
            .next()
            .expect("one segment")
            .expect("segment entry")
            .path();
        let mut corrupt = fs::read(&segment).expect("segment bytes");
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        fs::write(&segment, corrupt).expect("corrupt segment");
        assert!(
            SemanticRepository::restore_backup_from(corrupt_destination.path(), &first).is_err()
        );
        assert!(
            !corrupt_destination
                .path()
                .join(SEMANTIC_STORE_RELATIVE)
                .exists()
        );
    }

    #[test]
    fn semantic_fact_page_loss_and_corruption_rebuild_without_changing_authority() {
        let source = tempfile::TempDir::new().expect("temporary source project");
        let (root, modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            source.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");
        let current = repository.current_binding().expect("current binding");
        let head_before = fs::read(repository.store.join(HEAD_FILE)).expect("HEAD bytes");
        let (manifest, rebuilt) = repository
            .load_or_rebuild_semantic_facts(&current)
            .expect("seeded facts");
        assert!(!rebuilt);

        let summary_page =
            semantic_fact_page_path(&repository.store, manifest.roots.summaries.page());
        fs::write(&summary_page, b"corrupt-derived-page").expect("corrupt fact page");
        let (recovered, rebuilt) = repository
            .load_or_rebuild_semantic_facts(&current)
            .expect("rebuild corrupt facts");
        assert!(rebuilt);
        recovered
            .verify(&SemanticFactPageStore::new(&repository.store))
            .expect("recovered facts verify");

        let test_page = semantic_fact_page_path(&repository.store, recovered.roots.tests.page());
        fs::remove_file(&test_page).expect("remove disposable fact page");
        let (recovered, rebuilt) = repository
            .load_or_rebuild_semantic_facts(&current)
            .expect("rebuild missing facts");
        assert!(rebuilt);
        recovered
            .verify(&SemanticFactPageStore::new(&repository.store))
            .expect("restored facts verify");
        assert_eq!(
            fs::read(repository.store.join(HEAD_FILE)).expect("HEAD after rebuild"),
            head_before
        );
    }

    #[test]
    fn segmented_backup_rejects_missing_reordered_and_legacy_inputs_before_visibility() {
        let source = tempfile::TempDir::new().expect("temporary source project");
        let (root, modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            source.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");

        let missing = source.path().join("missing.lkjb");
        repository.backup_to(&missing).expect("missing backup");
        let missing_segment = fs::read_dir(missing.join(BACKUP_SEGMENTS))
            .expect("segments")
            .next()
            .expect("one segment")
            .expect("segment entry")
            .path();
        fs::remove_file(missing_segment).expect("remove segment");
        let missing_destination = tempfile::TempDir::new().expect("missing destination");
        assert!(
            SemanticRepository::restore_backup_from(missing_destination.path(), &missing).is_err()
        );
        assert!(
            !missing_destination
                .path()
                .join(SEMANTIC_STORE_RELATIVE)
                .exists()
        );

        let reordered = source.path().join("reordered.lkjb");
        repository.backup_to(&reordered).expect("reordered backup");
        let manifest_path = reordered.join(BACKUP_MANIFEST_FILE);
        let mut manifest = BackupManifest::decode(&fs::read(&manifest_path).expect("manifest"))
            .expect("decode manifest");
        let reference = manifest.segments.first_mut().expect("segment reference");
        let old_segment_path = backup_segment_path(&reordered, reference.ordinal, reference.digest);
        let mut segment =
            BackupSegment::decode(&fs::read(&old_segment_path).expect("read ordered segment"))
                .expect("decode ordered segment");
        assert!(segment.entries.len() > 1);
        segment.entries.swap(0, 1);
        let malformed = packed::encode(
            BACKUP_SEGMENT_MAGIC,
            BACKUP_SEGMENT_DIGEST_DOMAIN,
            &segment,
            MAXIMUM_BACKUP_SEGMENT_BYTES,
        )
        .expect("encode deliberately reordered segment");
        reference.digest = backup_segment_digest(&malformed);
        reference.encoded_bytes = u64::try_from(malformed.len()).expect("segment length");
        let malformed_path = backup_segment_path(&reordered, reference.ordinal, reference.digest);
        write_new_file(&malformed_path, &malformed, "malformed backup segment")
            .expect("write malformed segment");
        fs::write(&manifest_path, manifest.encode().expect("updated manifest"))
            .expect("write updated manifest");
        let reordered_destination = tempfile::TempDir::new().expect("reordered destination");
        assert!(
            SemanticRepository::restore_backup_from(reordered_destination.path(), &reordered,)
                .is_err()
        );
        assert!(
            !reordered_destination
                .path()
                .join(SEMANTIC_STORE_RELATIVE)
                .exists()
        );

        let legacy = source.path().join("legacy.lkjb");
        fs::write(&legacy, b"LKJBKP03 predecessor").expect("legacy fixture");
        let legacy_destination = tempfile::TempDir::new().expect("legacy destination");
        let error = SemanticRepository::restore_backup_from(legacy_destination.path(), &legacy)
            .expect_err("legacy monolith must reject");
        assert_eq!(error.code, "semantic_backup_directory");
        assert!(
            !legacy_destination
                .path()
                .join(SEMANTIC_STORE_RELATIVE)
                .exists()
        );
    }

    #[test]
    fn retention_preview_is_exact_read_only_and_refuses_destructive_readiness() {
        let source = tempfile::TempDir::new().expect("temporary source project");
        let (root, modules) = fixture();
        let (repository, _) = SemanticRepository::initialize(
            source.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"import"),
                semantic_diff: SemanticDiffDigest::of(b"initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: ReceiptStatus::ImportAccepted,
            },
        )
        .expect("initialize");
        let clean = repository.retention_preview().expect("clean preview");
        assert_eq!(clean.reclaimable_objects, 0);
        assert!(!clean.destructive_ready);

        let orphan = b"unreachable-corrupt-module-object";
        let orphan_digest = ModuleObjectDigest::of(orphan);
        write_immutable(
            &module_path(&repository.store, orphan_digest),
            orphan,
            "orphan cleanup fixture",
        )
        .expect("write orphan");
        let before = directory_snapshot(&repository.store);
        let report = repository.retention_preview().expect("orphan preview");
        let after = directory_snapshot(&repository.store);
        assert_eq!(before, after);
        assert_eq!(report.reclaimable_objects, 1);
        assert_eq!(
            report.reclaimable_bytes,
            u64::try_from(orphan.len()).expect("orphan bytes")
        );
        assert_ne!(report.plan, clean.plan);
        assert!(!report.destructive_ready);
        assert_eq!(report.missing_authority.len(), 3);
    }
}
