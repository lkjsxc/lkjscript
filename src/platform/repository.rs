//! Crash-consistent packed semantic repository with one atomic visibility point.

use super::artifact::{
    ArtifactReceipt, MAXIMUM_ARTIFACT_PACKAGES, build_artifact_from_objects, decode_package_object,
    encode_package_object, load_artifact, load_package_object_closure,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::graph::GraphRoot;
use super::meaning::{GRAPH_CONTRACT_VERSION, MeaningModule};
use super::packed;
use super::revision::{
    AffectedOwner, ParentRevision, RECEIPT_CONTRACT_VERSION, REVISION_CONTRACT_VERSION,
    ReceiptStatus, RevisionCore, RevisionRecord, SemanticHead, TransactionReceipt, ValidationFacts,
};
use super::semantic::{
    ExactGraphDependency, ValidatedPackage, canonicalize_graph_package, validate_graph_package,
};
use super::semantic_digest::{
    ArtifactDigest, BackupDigest, ModuleObjectDigest, ReceiptDigest, RevisionRecordDigest,
    RootObjectDigest, SemanticDiffDigest, TransactionDigest,
};
use super::semantic_draft::{DraftRecord, MAXIMUM_DRAFT_BYTES, SemanticDraftStore};
use super::semantic_id::{DraftId, ModuleId, RepositoryId, RevisionId};
use super::semantic_query::SemanticQueryIndex;
use bincode::{Decode, Encode};
use fs2::FileExt;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const SEMANTIC_STORE_RELATIVE: &str = ".lkjscript/meaning";
pub const MAXIMUM_HEAD_BYTES: usize = 4_096;
pub const MAXIMUM_HISTORY_ITEMS: usize = 10_000;
pub const MAXIMUM_ARTIFACT_OBJECT_BYTES: usize = 256 * 1_048_576;
pub const BACKUP_CONTRACT_VERSION: u16 = 1;
pub const MAXIMUM_BACKUP_BYTES: usize = 128 * 1_048_576;
pub const MAXIMUM_BACKUP_ENTRIES: usize = 2_000_000;

const BACKUP_MAGIC: [u8; 8] = *b"LKJBKP01";
const BACKUP_DIGEST_DOMAIN: &str = "lkjscript.semantic-backup.v1";

const HEAD_FILE: &str = "HEAD";
const LOCK_FILE: &str = "LOCK";
const MODULE_OBJECTS: &str = "objects/modules";
const ROOT_OBJECTS: &str = "objects/roots";
const REVISION_OBJECTS: &str = "revisions";
const RECEIPT_OBJECTS: &str = "receipts";
const ARTIFACT_OBJECTS: &str = "artifacts";
const DRAFT_OBJECTS: &str = "drafts";
const INDEX_OBJECTS: &str = "indexes";

#[derive(Clone, Copy, Debug)]
pub(crate) enum DisposableIndexPart {
    Manifest,
    Owners(u8),
    Names(u8),
}

#[derive(Clone, Debug)]
pub struct SemanticRepository {
    project_root: PathBuf,
    store: PathBuf,
}

#[derive(Clone, Debug)]
pub struct CurrentRevision {
    pub head: SemanticHead,
    pub record: RevisionRecord,
    pub receipt: TransactionReceipt,
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
}

#[derive(Clone, Debug)]
pub struct PublicationProposal {
    pub expected_base: RevisionId,
    pub root: GraphRoot,
    pub modules: Vec<MeaningModule>,
    pub transaction: TransactionDigest,
    pub idempotency_key: Option<String>,
    pub semantic_diff: SemanticDiffDigest,
    pub status: ReceiptStatus,
    pub affected_owners: Vec<AffectedOwner>,
    pub intent: Option<String>,
    pub dependency_artifacts: Vec<DependencyArtifactObject>,
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
    pub entries: usize,
    pub drafts: usize,
    pub bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RestoreReceipt {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub revision: RevisionId,
    pub digest: BackupDigest,
    pub entries: usize,
    pub drafts: usize,
    pub deep_valid: bool,
}

#[derive(Decode, Encode, Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
enum BackupEntryKey {
    Module(ModuleObjectDigest),
    Root(RootObjectDigest),
    Revision(RevisionId),
    Receipt(ReceiptDigest),
    Artifact(ArtifactDigest),
    Draft(DraftId),
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct BackupEntry {
    key: BackupEntryKey,
    bytes: Vec<u8>,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq)]
struct BackupBundle {
    contract_version: u16,
    repository_id: RepositoryId,
    head: SemanticHead,
    entries: Vec<BackupEntry>,
}

impl BackupBundle {
    fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_entries()?;
        packed::encode(
            BACKUP_MAGIC,
            BACKUP_DIGEST_DOMAIN,
            self,
            MAXIMUM_BACKUP_BYTES,
        )
    }

    fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self = packed::decode(
            bytes,
            BACKUP_MAGIC,
            BACKUP_DIGEST_DOMAIN,
            MAXIMUM_BACKUP_BYTES,
        )?;
        value.validate_entries()?;
        Ok(value)
    }

    fn validate_entries(&self) -> Result<(), Diagnostic> {
        if self.contract_version != BACKUP_CONTRACT_VERSION
            || self.head.repository_id != self.repository_id
        {
            return Err(repository_error(
                DiagnosticClass::Source,
                "semantic_backup_contract",
                "backup uses an unknown contract or inconsistent repository identity",
            ));
        }
        if self.entries.is_empty() || self.entries.len() > MAXIMUM_BACKUP_ENTRIES {
            return Err(repository_error(
                DiagnosticClass::Resource,
                "semantic_backup_entry_limit",
                format!("backup must contain 1 through {MAXIMUM_BACKUP_ENTRIES} entries"),
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
        for entry in &self.entries {
            validate_backup_entry(entry)?;
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
        repository.current()?;
        Ok(repository)
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
        let observed = repository.current()?;
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
        let root = self.read_root(record.core.root)?;
        if root.repository_id != head.repository_id {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_root_identity",
                "graph root belongs to a foreign repository identity",
            ));
        }
        Ok(CurrentRevision {
            head,
            record,
            receipt,
            root,
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

    pub fn canonicalize_proposal(
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

    pub fn module_by_id(
        &self,
        revision: RevisionId,
        module_id: ModuleId,
    ) -> Result<MeaningModule, Diagnostic> {
        let record = self.read_revision(revision)?;
        let root = self.read_root(record.core.root)?;
        let reference = root
            .modules
            .iter()
            .find(|reference| reference.id == module_id)
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
        let root = self.read_root(record.core.root)?;
        let reference = root
            .modules
            .iter()
            .find(|reference| reference.name == name)
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

    pub fn read_root(&self, digest: RootObjectDigest) -> Result<GraphRoot, Diagnostic> {
        let path = root_path(&self.store, digest);
        let bytes = read_bounded(
            &path,
            super::graph::MAXIMUM_ROOT_BYTES + 50,
            "graph root object",
        )?;
        if RootObjectDigest::of(&bytes) != digest {
            return Err(repository_error(
                DiagnosticClass::Corrupt,
                "repository_root_digest",
                "graph root bytes do not match their physical key",
            ));
        }
        GraphRoot::decode(&bytes)
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

    pub fn publish(
        &self,
        mut proposal: PublicationProposal,
    ) -> Result<(PublicationOutcome, Option<TransactionReceipt>), Diagnostic> {
        self.publish_with_additional_parent(&mut proposal, None)
    }

    pub fn publish_merge(
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
        proposal.root.validate_modules(&proposal.modules)?;
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
        let current = self.current()?;
        if current.head.revision != proposal.expected_base {
            return Ok((
                PublicationOutcome::StaleBase {
                    requested: proposal.expected_base,
                    current: current.head.revision,
                },
                None,
            ));
        }
        if proposal.root.repository_id != current.head.repository_id {
            return Err(repository_error(
                DiagnosticClass::Source,
                "repository_foreign_identity",
                "proposed graph root belongs to a foreign repository",
            ));
        }
        let root_bytes = proposal.root.encode()?;
        let root_digest = RootObjectDigest::of(&root_bytes);
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
            repository_id: proposal.root.repository_id,
            parents,
            root: root_digest,
            semantic_diff: proposal.semantic_diff,
            transaction: proposal.transaction,
        };
        let revision = core.revision_id()?;
        let validated = validate_repository_graph(
            &self.store,
            &proposal.root,
            &proposal.modules,
            Some(revision),
        )?;
        for module in &proposal.modules {
            let bytes = module.encode()?;
            write_publication_immutable(
                &module_path(&self.store, module.digest()?),
                &bytes,
                "meaning module object",
            )?;
        }
        write_publication_immutable(
            &root_path(&self.store, root_digest),
            &root_bytes,
            "graph root object",
        )?;
        proposal.affected_owners.sort();
        proposal.affected_owners.dedup();
        let receipt = TransactionReceipt {
            contract_version: RECEIPT_CONTRACT_VERSION,
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: proposal.root.repository_id,
            status: proposal.status,
            base: Some(proposal.expected_base),
            result: revision,
            transaction: proposal.transaction,
            idempotency_key: proposal.idempotency_key.clone(),
            semantic_diff: proposal.semantic_diff,
            affected_owners: proposal.affected_owners.clone(),
            validation: validation_facts(&validated)?,
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
            repository_id: proposal.root.repository_id,
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
            if receipt.result != record.revision {
                return Err(repository_error(
                    DiagnosticClass::Corrupt,
                    "repository_history_receipt_binding",
                    "historical receipt names a different result revision",
                ));
            }
            let root = self.read_root(record.core.root)?;
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
        let rebuilt_indexes = usize::from(SemanticQueryIndex::current(self)?.rebuilt_index());
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

    pub fn backup(&self) -> Result<(Vec<u8>, BackupReceipt), Diagnostic> {
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
            let mut entries = BTreeMap::<BackupEntryKey, Vec<u8>>::new();
            let mut pending = vec![head.revision];
            let mut seen = std::collections::BTreeSet::new();
            while let Some(revision) = pending.pop() {
                if !seen.insert(revision) {
                    continue;
                }
                if seen.len() > MAXIMUM_HISTORY_ITEMS {
                    return Err(repository_error(
                        DiagnosticClass::Resource,
                        "semantic_backup_history_limit",
                        format!("backup exceeds {MAXIMUM_HISTORY_ITEMS} retained revisions"),
                    ));
                }
                let record = self.read_revision(revision)?;
                insert_backup_entry(
                    &mut entries,
                    BackupEntryKey::Revision(revision),
                    record.encode()?,
                )?;
                let receipt = self.read_receipt(record.receipt)?;
                insert_backup_entry(
                    &mut entries,
                    BackupEntryKey::Receipt(record.receipt),
                    receipt.encode()?,
                )?;
                let root = self.read_root(record.core.root)?;
                insert_backup_entry(
                    &mut entries,
                    BackupEntryKey::Root(record.core.root),
                    root.encode()?,
                )?;
                for reference in &root.modules {
                    let module = self.read_module(reference.object)?;
                    insert_backup_entry(
                        &mut entries,
                        BackupEntryKey::Module(reference.object),
                        module.encode()?,
                    )?;
                }
                for dependency in &root.dependencies {
                    let loaded = load_stored_artifact_closure(&self.store, dependency.artifact)?;
                    for (digest, bytes) in loaded.package_objects {
                        insert_backup_entry(&mut entries, BackupEntryKey::Artifact(digest), bytes)?;
                    }
                }
                pending.extend(record.core.parents.iter().map(|parent| parent.revision));
            }
            let mut draft_count = 0usize;
            let drafts = self.store.join(DRAFT_OBJECTS);
            for entry in fs::read_dir(&drafts)
                .map_err(|error| io_error("semantic_backup_draft_list", &drafts, error))?
            {
                let entry = entry
                    .map_err(|error| io_error("semantic_backup_draft_list", &drafts, error))?;
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
                let bytes =
                    read_bounded(&path, MAXIMUM_DRAFT_BYTES + 50, "semantic draft authority")?;
                let draft = DraftRecord::decode(&bytes)?;
                if draft.id != id || draft.repository_id != head.repository_id {
                    return Err(repository_error(
                        DiagnosticClass::Corrupt,
                        "semantic_backup_draft_binding",
                        "draft identity or repository binding is inconsistent",
                    ));
                }
                self.reconstruct_revision(draft.base_revision)?;
                insert_backup_entry(&mut entries, BackupEntryKey::Draft(id), bytes)?;
                draft_count = checked_increment(draft_count, "draft")?;
            }
            let bundle = BackupBundle {
                contract_version: BACKUP_CONTRACT_VERSION,
                repository_id: head.repository_id,
                head,
                entries: entries
                    .into_iter()
                    .map(|(key, bytes)| BackupEntry { key, bytes })
                    .collect(),
            };
            let bytes = bundle.encode()?;
            let receipt = BackupReceipt {
                contract_version: BACKUP_CONTRACT_VERSION,
                repository_id: head.repository_id,
                revision: head.revision,
                digest: BackupDigest::of(&bytes),
                entries: bundle.entries.len(),
                drafts: draft_count,
                bytes: bytes.len(),
            };
            Ok((bytes, receipt))
        })();
        FileExt::unlock(&lock)
            .map_err(|error| io_error("repository_backup_unlock", &lock_path, error))?;
        result
    }

    pub fn restore_backup(
        project_root: &Path,
        backup: &[u8],
    ) -> Result<(Self, RestoreReceipt), Diagnostic> {
        let bundle = BackupBundle::decode(backup)?;
        let digest = BackupDigest::of(backup);
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
            for entry in &bundle.entries {
                match entry.key {
                    BackupEntryKey::Module(digest) => write_immutable(
                        &module_path(&stage, digest),
                        &entry.bytes,
                        "restored meaning module",
                    )?,
                    BackupEntryKey::Root(digest) => write_immutable(
                        &root_path(&stage, digest),
                        &entry.bytes,
                        "restored graph root",
                    )?,
                    BackupEntryKey::Revision(revision) => write_immutable(
                        &revision_path(&stage, revision),
                        &entry.bytes,
                        "restored revision record",
                    )?,
                    BackupEntryKey::Receipt(digest) => write_immutable(
                        &receipt_path(&stage, digest),
                        &entry.bytes,
                        "restored transaction receipt",
                    )?,
                    BackupEntryKey::Artifact(digest) => write_immutable(
                        &artifact_path(&stage, digest),
                        &entry.bytes,
                        "restored dependency artifact",
                    )?,
                    BackupEntryKey::Draft(id) => write_immutable(
                        &stage.join(DRAFT_OBJECTS).join(format!("{id}.lkjd")),
                        &entry.bytes,
                        "restored semantic draft",
                    )?,
                }
            }
            sync_publication_objects(&stage)?;
            sync_tree_directories(&stage.join(ARTIFACT_OBJECTS))?;
            replace_head(&stage, &bundle.head)?;
            let staged = Self {
                project_root: project_root.clone(),
                store: stage.clone(),
            };
            staged.doctor(true)?;
            SemanticDraftStore::new(&staged).validate_all()?;
            Ok(())
        })();
        if let Err(error) = restored {
            remove_owned_stage(&stage);
            return Err(error);
        }
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
        let report = repository.doctor(true)?;
        let drafts = SemanticDraftStore::new(&repository).validate_all()?;
        Ok((
            repository,
            RestoreReceipt {
                contract_version: BACKUP_CONTRACT_VERSION,
                repository_id: bundle.repository_id,
                revision: bundle.head.revision,
                digest,
                entries: bundle.entries.len(),
                drafts,
                deep_valid: report.valid,
            },
        ))
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

fn insert_backup_entry(
    entries: &mut BTreeMap<BackupEntryKey, Vec<u8>>,
    key: BackupEntryKey,
    bytes: Vec<u8>,
) -> Result<(), Diagnostic> {
    match entries.get(&key) {
        Some(existing) if existing != &bytes => Err(repository_error(
            DiagnosticClass::Corrupt,
            "semantic_backup_object_conflict",
            "one backup object key resolves to different canonical bytes",
        )),
        Some(_) => Ok(()),
        None => {
            if entries.len() >= MAXIMUM_BACKUP_ENTRIES {
                return Err(repository_error(
                    DiagnosticClass::Resource,
                    "semantic_backup_entry_limit",
                    format!("backup exceeds {MAXIMUM_BACKUP_ENTRIES} entries"),
                ));
            }
            entries.insert(key, bytes);
            Ok(())
        }
    }
}

fn validate_backup_entry(entry: &BackupEntry) -> Result<(), Diagnostic> {
    match entry.key {
        BackupEntryKey::Module(digest) => {
            if MeaningModule::decode(&entry.bytes)?.digest()? != digest {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Root(digest) => {
            if GraphRoot::decode(&entry.bytes)?.digest()? != digest {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Revision(revision) => {
            let record = RevisionRecord::decode(&entry.bytes)?;
            if record.revision != revision || record.core.revision_id()? != revision {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Receipt(digest) => {
            if ReceiptDigest::of(&TransactionReceipt::decode(&entry.bytes)?.encode()?) != digest {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Artifact(digest) => {
            if entry.bytes.len() > MAXIMUM_ARTIFACT_OBJECT_BYTES
                || ArtifactDigest::of(&entry.bytes) != digest
            {
                return Err(backup_digest_error());
            }
        }
        BackupEntryKey::Draft(id) => {
            let draft = DraftRecord::decode(&entry.bytes)?;
            if draft.id != id {
                return Err(backup_digest_error());
            }
        }
    }
    Ok(())
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
    let root_bytes = root.encode()?;
    let root_digest = RootObjectDigest::of(&root_bytes);
    write_immutable(
        &root_path(store, root_digest),
        &root_bytes,
        "graph root object",
    )?;
    let core = RevisionCore {
        contract_version: REVISION_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: root.repository_id,
        parents: Vec::new(),
        root: root_digest,
        semantic_diff,
        transaction,
    };
    let revision = core.revision_id()?;
    let validated = validate_repository_graph(store, &root, &modules, Some(revision))?;
    let mut validation = validation_facts(&validated)?;
    if let Some(profile) = validation_profile {
        validation.profile = profile;
    }
    let receipt = TransactionReceipt {
        contract_version: RECEIPT_CONTRACT_VERSION,
        graph_contract_version: GRAPH_CONTRACT_VERSION,
        repository_id: root.repository_id,
        status: ReceiptStatus::ImportAccepted,
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
        ROOT_OBJECTS,
        REVISION_OBJECTS,
        RECEIPT_OBJECTS,
    ] {
        let path = store.join(relative);
        sync_tree_directories(&path)?;
    }
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    Ok(())
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
        DisposableIndexPart::Owners(bucket) => {
            revision.join("owners").join(format!("{bucket:02x}.lkix"))
        }
        DisposableIndexPart::Names(bucket) => {
            revision.join("names").join(format!("{bucket:02x}.lkix"))
        }
    }
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
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path).map_err(|error| io_error(code, path, error))
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
            },
        )
        .expect("initialize");
        let initial = repository.reconstruct_current().expect("reconstruct");

        let (no_change, receipt) = repository
            .publish(PublicationProposal {
                expected_base: initial.current.head.revision,
                root: root.clone(),
                modules: modules.clone(),
                transaction: TransactionDigest::of(b"no-change"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"empty"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: Vec::new(),
                intent: None,
                dependency_artifacts: Vec::new(),
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
                root: renamed_root.clone(),
                modules: renamed_modules.clone(),
                transaction: TransactionDigest::of(b"stale"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"rename"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: Vec::new(),
                intent: None,
                dependency_artifacts: Vec::new(),
            })
            .expect("stale");
        assert!(matches!(stale, PublicationOutcome::StaleBase { .. }));
        assert!(receipt.is_none());

        let declaration_id = renamed_modules[0].declarations[0].id;
        let (accepted, receipt) = repository
            .publish(PublicationProposal {
                expected_base: initial.current.head.revision,
                root: renamed_root,
                modules: renamed_modules,
                transaction: TransactionDigest::of(b"rename"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"rename"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: vec![AffectedOwner::Declaration(declaration_id)],
                intent: Some("rename fixture record".to_owned()),
                dependency_artifacts: Vec::new(),
            })
            .expect("accepted");
        assert!(matches!(accepted, PublicationOutcome::Accepted { .. }));
        assert_eq!(receipt.expect("receipt").affected_owners.len(), 1);
        let reconstructed = repository.reconstruct_current().expect("current");
        assert_eq!(reconstructed.modules[0].declarations[0].id, declaration_id);
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
                root: left_root,
                modules: left_modules,
                transaction: TransactionDigest::of(b"left"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"left"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: vec![AffectedOwner::Declaration(declaration_id)],
                intent: None,
                dependency_artifacts: Vec::new(),
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
                root: right_root,
                modules: base_modules,
                transaction: TransactionDigest::of(b"right"),
                idempotency_key: None,
                semantic_diff: SemanticDiffDigest::of(b"right"),
                status: ReceiptStatus::AcceptedChange,
                affected_owners: Vec::new(),
                intent: None,
                dependency_artifacts: Vec::new(),
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
            },
        )
        .expect("initialize");
        let (first, first_receipt) = repository.backup().expect("first backup");
        let (second, second_receipt) = repository.backup().expect("second backup");
        assert_eq!(first, second);
        assert_eq!(first_receipt, second_receipt);

        let destination = tempfile::TempDir::new().expect("temporary restore project");
        let (restored, receipt) =
            SemanticRepository::restore_backup(destination.path(), &first).expect("restore");
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
        let mut corrupt = first;
        let index = corrupt.len() / 2;
        corrupt[index] ^= 1;
        assert!(SemanticRepository::restore_backup(corrupt_destination.path(), &corrupt).is_err());
        assert!(
            !corrupt_destination
                .path()
                .join(SEMANTIC_STORE_RELATIVE)
                .exists()
        );
    }
}
