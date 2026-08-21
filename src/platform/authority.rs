//! Content-addressed authored source history with one exact publication owner.

use super::artifact::{
    LoadedArtifact, MAXIMUM_ARTIFACT_BYTES, load_artifact, package_artifact_digest,
};
use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::package::{
    PACKAGE_FILE_NAME, PackageDescriptor, PackageId, decode_package, validate_relative_path,
};
use super::semantic::{ExactDependency, ValidatedPackage, validate_package_documents};
use super::syntax::{SourceLimits, parse_source};
use fs2::FileExt;
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const SOURCE_PROJECT_CONTRACT_VERSION: u16 = 1;
const AUTHORITY_DIRECTORY: &str = "source-v1";
const HEAD_FILE: &str = "HEAD.json";
const LOCK_FILE: &str = "LOCK";
const OBJECTS_DIRECTORY: &str = "objects";
const SOURCES_DIRECTORY: &str = "source";
const DEPENDENCIES_DIRECTORY: &str = "dependency";
const MANIFESTS_DIRECTORY: &str = "manifest";
const RECORDS_DIRECTORY: &str = "records";
const INDEX_DIRECTORY: &str = "revision-index";
const MAXIMUM_INTERNAL_BYTES: usize = 16 * 1_048_576;
const MAXIMUM_ACCEPTED_SOURCE_BYTES: usize = 64 * 1_048_576;
const MAXIMUM_ACCEPTED_DEPENDENCY_BYTES: usize = 512 * 1_048_576;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyArtifact {
    pub alias: String,
    pub bytes: Vec<u8>,
}

impl DependencyArtifact {
    pub fn decode(alias: impl Into<String>, bytes: Vec<u8>) -> Result<Self, Diagnostic> {
        if bytes.len() > MAXIMUM_ARTIFACT_BYTES {
            return Err(authority_error(
                DiagnosticClass::Resource,
                "source_dependency_too_large",
                "dependency artifact exceeds the artifact byte limit",
            ));
        }
        load_artifact(&bytes)?;
        Ok(Self {
            alias: alias.into(),
            bytes,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicationKind {
    Published,
    Validated,
    NoChange,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ApplyReceipt {
    pub contract_version: u16,
    pub kind: PublicationKind,
    pub package_id: PackageId,
    pub revision: u64,
    pub record_digest: String,
    pub authored_digest: String,
    pub semantic_digest: String,
    pub semantic_changed: bool,
    pub file_count: usize,
    pub source_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorkingStatus {
    pub contract_version: u16,
    pub package_id: PackageId,
    pub revision: u64,
    pub record_digest: String,
    pub authored_digest: String,
    pub semantic_digest: String,
    pub working_authored_digest: String,
    pub working_semantic_digest: String,
    pub authored_changed: bool,
    pub semantic_changed: bool,
    pub file_count: usize,
    pub source_bytes: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RevisionSummary {
    pub revision: u64,
    pub record_digest: String,
    pub parent_record_digest: Option<String>,
    pub authored_digest: String,
    pub semantic_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct HistoryPage {
    pub contract_version: u16,
    pub package_id: PackageId,
    pub current_revision: u64,
    pub items: Vec<RevisionSummary>,
    pub next_before: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DoctorReceipt {
    pub contract_version: u16,
    pub package_id: PackageId,
    pub current_revision: u64,
    pub current_record_digest: String,
    pub records_checked: u64,
    pub manifests_checked: u64,
    pub source_objects_checked: u64,
    pub dependency_objects_checked: u64,
    pub source_bytes_checked: u64,
    pub dependency_bytes_checked: u64,
    pub deep: bool,
    pub valid: bool,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BackupReceipt {
    pub contract_version: u16,
    pub package_id: PackageId,
    pub current_revision: u64,
    pub current_record_digest: String,
    pub backup_digest: String,
    pub entry_count: usize,
    pub bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AcceptedRevision {
    pub package: ValidatedPackage,
    pub revision: u64,
    pub record_digest: String,
    pub authored_digest: String,
    pub files: BTreeMap<String, Vec<u8>>,
    pub dependencies: Vec<DependencyArtifact>,
}

#[derive(Clone, Debug)]
pub struct ProjectAuthority {
    root: PathBuf,
    authority: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct HeadCore {
    contract_version: u16,
    package_id: PackageId,
    revision: u64,
    record_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct Head {
    contract_version: u16,
    package_id: PackageId,
    revision: u64,
    record_digest: String,
    checksum: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct FileObject {
    path: String,
    digest: String,
    bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RevisionManifest {
    contract_version: u16,
    package_id: PackageId,
    revision: u64,
    authored_digest: String,
    semantic_digest: String,
    package: FileObject,
    modules: Vec<FileObject>,
    dependencies: Vec<DependencyObject>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct DependencyObject {
    alias: String,
    package_id: PackageId,
    revision_digest: String,
    artifact_digest: String,
    object_digest: String,
    bytes: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct RevisionRecord {
    contract_version: u16,
    package_id: PackageId,
    revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_record_digest: Option<String>,
    manifest_digest: String,
    authored_digest: String,
    semantic_digest: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct BackupManifest {
    contract_version: u16,
    package_id: PackageId,
    current_revision: u64,
    current_record_digest: String,
    entries: Vec<BackupEntry>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct BackupEntry {
    path: String,
    digest: String,
    bytes: usize,
}

struct WorkingPackage {
    descriptor: PackageDescriptor,
    validated: ValidatedPackage,
    files: BTreeMap<String, Vec<u8>>,
    authored_digest: String,
    source_bytes: usize,
    dependencies: Vec<DependencyArtifact>,
}

impl ProjectAuthority {
    pub fn initialize(
        root: &Path,
        dependencies: &[DependencyArtifact],
    ) -> Result<(Self, ApplyReceipt), Diagnostic> {
        let root = canonical_project_root(root)?;
        let authority = root.join(".lkjscript").join(AUTHORITY_DIRECTORY);
        if authority.exists() {
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_project_exists",
                format!(
                    "source authority already exists at '{}'",
                    authority.display()
                ),
            ));
        }
        if root.join(".lkjscript").join("project").exists() {
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_predecessor_rejected",
                "the graph-authored predecessor is not a current source project",
            ));
        }
        // Validate every authored byte before creating authority state. A rejected
        // initialization therefore cannot leave a misleading partial project.
        let working = load_working_from_root(&root, dependencies)?;
        fs::create_dir_all(authority.join(OBJECTS_DIRECTORY).join(SOURCES_DIRECTORY))
            .and_then(|()| {
                fs::create_dir_all(authority.join(OBJECTS_DIRECTORY).join(MANIFESTS_DIRECTORY))
            })
            .and_then(|()| {
                fs::create_dir_all(
                    authority
                        .join(OBJECTS_DIRECTORY)
                        .join(DEPENDENCIES_DIRECTORY),
                )
            })
            .and_then(|()| fs::create_dir_all(authority.join(RECORDS_DIRECTORY)))
            .and_then(|()| fs::create_dir_all(authority.join(INDEX_DIRECTORY)))
            .map_err(|error| io_error("source_project_create", &authority, error))?;
        sync_directory(&authority)?;
        let project = Self { root, authority };
        let _lock = project.lock()?;
        if project.head_path().exists() {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_head_unexpected",
                "new authority acquired a HEAD before initial publication",
            ));
        }
        let receipt = project.publish_loaded(None, None, working)?;
        Ok((project, receipt))
    }

    pub fn open(root: &Path) -> Result<Self, Diagnostic> {
        let root = canonical_project_root(root)?;
        let authority = root.join(".lkjscript").join(AUTHORITY_DIRECTORY);
        if !authority.is_dir() {
            if root.join(".lkjscript").join("project").exists() {
                return Err(authority_error(
                    DiagnosticClass::Source,
                    "source_predecessor_rejected",
                    "the graph-authored predecessor is not a current source project",
                ));
            }
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_project_missing",
                format!("no source authority exists beneath '{}'", root.display()),
            ));
        }
        reject_symlink(&authority, "source authority directory")?;
        let project = Self { root, authority };
        project.read_head()?;
        Ok(project)
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn current(&self) -> Result<AcceptedRevision, Diagnostic> {
        let head = self.read_head()?;
        self.reconstruct_record(&head.record_digest)
    }

    pub fn revision(&self, revision: u64) -> Result<AcceptedRevision, Diagnostic> {
        let head = self.read_head()?;
        if revision > head.revision {
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_revision_future",
                format!(
                    "revision {revision} is newer than current revision {}",
                    head.revision
                ),
            ));
        }
        let mut digest = head.record_digest;
        loop {
            let record = self.read_record(&digest)?;
            if record.revision == revision {
                return self.reconstruct(record, digest);
            }
            digest = record.parent_record_digest.ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Corrupt,
                    "source_revision_chain",
                    format!("revision {revision} is absent from the accepted record chain"),
                )
            })?;
        }
    }

    pub fn status(&self, dependencies: &[DependencyArtifact]) -> Result<WorkingStatus, Diagnostic> {
        let current = self.current()?;
        let working = self.load_working(dependencies)?;
        Ok(WorkingStatus {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: current.package.descriptor.package_id,
            revision: current.revision,
            record_digest: current.record_digest,
            authored_digest: current.authored_digest.clone(),
            semantic_digest: current.package.revision_digest.clone(),
            working_authored_digest: working.authored_digest.clone(),
            working_semantic_digest: working.validated.revision_digest.clone(),
            authored_changed: current.authored_digest != working.authored_digest,
            semantic_changed: current.package.revision_digest != working.validated.revision_digest,
            file_count: working.files.len(),
            source_bytes: working.source_bytes,
        })
    }

    pub fn validate_working(
        &self,
        expected_revision: u64,
        expected_record_digest: &str,
        dependencies: &[DependencyArtifact],
    ) -> Result<ApplyReceipt, Diagnostic> {
        let current = self.current()?;
        verify_expected(&current, expected_revision, expected_record_digest)?;
        let working = self.load_working(dependencies)?;
        Ok(validation_receipt(&current, &working))
    }

    pub fn apply_working(
        &self,
        expected_revision: u64,
        expected_record_digest: &str,
        dependencies: &[DependencyArtifact],
    ) -> Result<ApplyReceipt, Diagnostic> {
        let working = self.load_working(dependencies)?;
        let _lock = self.lock()?;
        let head = self.read_head()?;
        if head.revision != expected_revision || head.record_digest != expected_record_digest {
            return Err(stale_error(
                expected_revision,
                expected_record_digest,
                head.revision,
                &head.record_digest,
            ));
        }
        let current = self.reconstruct_record(&head.record_digest)?;
        if current.package.descriptor.package_id != working.descriptor.package_id {
            return Err(authority_error(
                DiagnosticClass::Semantic,
                "source_package_identity_changed",
                "accepted package identity cannot be changed by an edit",
            ));
        }
        self.publish_loaded(Some(&head), Some(&current), working)
    }

    pub fn restore_working(&self, revision: u64) -> Result<(), Diagnostic> {
        let head = self.read_head()?;
        if revision > head.revision {
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_revision_future",
                format!(
                    "revision {revision} is newer than current revision {}",
                    head.revision
                ),
            ));
        }
        let mut digest = head.record_digest;
        let manifest = loop {
            let record = self.read_record(&digest)?;
            if record.revision == revision {
                break self.read_manifest(&record.manifest_digest)?;
            }
            digest = record.parent_record_digest.ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Corrupt,
                    "source_revision_chain",
                    format!("revision {revision} is absent from the accepted record chain"),
                )
            })?;
        };
        let mut files = Vec::with_capacity(manifest.modules.len() + 1);
        files.push(manifest.package);
        files.extend(manifest.modules);
        for file in files {
            let bytes = self.read_source(&file.digest, file.bytes)?;
            let path = confined_working_path(&self.root, &file.path)?;
            atomic_replace(&path, &bytes)?;
        }
        Ok(())
    }

    pub fn history(&self, before: Option<u64>, limit: usize) -> Result<HistoryPage, Diagnostic> {
        if limit == 0 || limit > 1_000 {
            return Err(authority_error(
                DiagnosticClass::Resource,
                "source_history_limit",
                "history limit must be 1 through 1000",
            ));
        }
        let head = self.read_head()?;
        let before = before.unwrap_or(head.revision.saturating_add(1));
        let mut digest = head.record_digest.clone();
        let mut items = Vec::new();
        let mut next_before = None;
        loop {
            let record = self.read_record(&digest)?;
            let parent = record.parent_record_digest.clone();
            if record.revision < before {
                if items.len() == limit {
                    next_before = Some(record.revision.saturating_add(1));
                    break;
                }
                items.push(RevisionSummary {
                    revision: record.revision,
                    record_digest: digest,
                    parent_record_digest: parent.clone(),
                    authored_digest: record.authored_digest,
                    semantic_digest: record.semantic_digest,
                });
            }
            match parent {
                Some(parent) => digest = parent,
                None => break,
            }
        }
        Ok(HistoryPage {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: head.package_id,
            current_revision: head.revision,
            items,
            next_before,
        })
    }

    pub fn doctor(&self, deep: bool) -> Result<DoctorReceipt, Diagnostic> {
        let head = self.read_head()?;
        let mut digest = head.record_digest.clone();
        let mut records = 0u64;
        let mut manifests = 0u64;
        let mut objects = BTreeSet::new();
        let mut dependency_objects = BTreeSet::new();
        let mut source_bytes = 0u64;
        let mut dependency_bytes = 0u64;
        loop {
            let record = self.read_record(&digest)?;
            records = records.saturating_add(1);
            let manifest = self.read_manifest(&record.manifest_digest)?;
            manifests = manifests.saturating_add(1);
            let files = std::iter::once(&manifest.package).chain(manifest.modules.iter());
            for file in files {
                if objects.insert(file.digest.clone()) {
                    self.read_source(&file.digest, file.bytes)?;
                    source_bytes = source_bytes.saturating_add(file.bytes as u64);
                }
            }
            for dependency in &manifest.dependencies {
                if dependency_objects.insert(dependency.object_digest.clone()) {
                    self.read_dependency(&dependency.object_digest, dependency.bytes)?;
                    dependency_bytes = dependency_bytes.saturating_add(dependency.bytes as u64);
                }
            }
            if !deep {
                break;
            }
            match record.parent_record_digest {
                Some(parent) => digest = parent,
                None => break,
            }
        }
        Ok(DoctorReceipt {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: head.package_id,
            current_revision: head.revision,
            current_record_digest: head.record_digest,
            records_checked: records,
            manifests_checked: manifests,
            source_objects_checked: objects.len() as u64,
            dependency_objects_checked: dependency_objects.len() as u64,
            source_bytes_checked: source_bytes,
            dependency_bytes_checked: dependency_bytes,
            deep,
            valid: true,
        })
    }

    pub fn backup_to(&self, destination: &Path) -> Result<BackupReceipt, Diagnostic> {
        reject_existing_destination(destination, "backup destination")?;
        let parent = destination.parent().ok_or_else(|| {
            authority_error(
                DiagnosticClass::Source,
                "source_backup_parent",
                "backup destination has no parent directory",
            )
        })?;
        reject_symlink(parent, "backup parent")?;
        let _lock = self.lock()?;
        let head = self.read_head()?;
        let mut files = BTreeMap::new();
        insert_backup_file(
            &mut files,
            "authority/HEAD.json",
            &self.head_path(),
            MAXIMUM_INTERNAL_BYTES,
        )?;
        let mut record_digest = head.record_digest.clone();
        loop {
            let record = self.read_record(&record_digest)?;
            insert_backup_file(
                &mut files,
                &format!("authority/records/{record_digest}"),
                &self.record_path(&record_digest),
                MAXIMUM_INTERNAL_BYTES,
            )?;
            insert_backup_file(
                &mut files,
                &format!("authority/objects/manifest/{}", record.manifest_digest),
                &self.manifest_path(&record.manifest_digest),
                MAXIMUM_INTERNAL_BYTES,
            )?;
            let manifest = self.read_manifest(&record.manifest_digest)?;
            for source in std::iter::once(&manifest.package).chain(manifest.modules.iter()) {
                let path = format!("authority/objects/source/{}", source.digest);
                if !files.contains_key(&path) {
                    insert_backup_file(
                        &mut files,
                        &path,
                        &self.source_path(&source.digest),
                        SourceLimits::default()
                            .maximum_bytes
                            .max(super::package::MAXIMUM_PACKAGE_BYTES),
                    )?;
                }
            }
            for dependency in &manifest.dependencies {
                let path = format!("authority/objects/dependency/{}", dependency.object_digest);
                if !files.contains_key(&path) {
                    insert_backup_file(
                        &mut files,
                        &path,
                        &self.dependency_path(&dependency.object_digest),
                        MAXIMUM_ARTIFACT_BYTES,
                    )?;
                }
            }
            match record.parent_record_digest {
                Some(parent) => record_digest = parent,
                None => break,
            }
        }
        let entries = files
            .iter()
            .map(|(path, bytes)| BackupEntry {
                path: path.clone(),
                digest: object_digest("backup-entry", bytes),
                bytes: bytes.len(),
            })
            .collect::<Vec<_>>();
        let manifest = BackupManifest {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: head.package_id.clone(),
            current_revision: head.revision,
            current_record_digest: head.record_digest.clone(),
            entries,
        };
        let manifest_bytes = canonical_json(&manifest)?;
        let backup_digest = object_digest("backup", &manifest_bytes);
        let temporary = temporary_path(parent, "backup-directory")?;
        fs::create_dir(&temporary)
            .map_err(|error| io_error("source_backup_create", &temporary, error))?;
        let mut guard = TemporaryDirectory::new(temporary.clone());
        for (path, bytes) in &files {
            let output = confined_backup_path(&temporary, path)?;
            atomic_replace(&output, bytes)?;
        }
        atomic_replace(&temporary.join("backup.json"), &manifest_bytes)?;
        sync_directory(&temporary)?;
        fs::rename(&temporary, destination)
            .map_err(|error| io_error("source_backup_publish", destination, error))?;
        sync_directory(parent)?;
        guard.disarm();
        let bytes = files
            .values()
            .try_fold(manifest_bytes.len() as u64, |total, value| {
                total.checked_add(value.len() as u64)
            })
            .ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Resource,
                    "source_backup_bytes",
                    "backup byte accounting overflowed",
                )
            })?;
        Ok(BackupReceipt {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: head.package_id,
            current_revision: head.revision,
            current_record_digest: head.record_digest,
            backup_digest,
            entry_count: files.len(),
            bytes,
        })
    }

    pub fn restore_backup(backup: &Path, destination: &Path) -> Result<BackupReceipt, Diagnostic> {
        reject_symlink(backup, "backup directory")?;
        if !backup.is_dir() {
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_backup_type",
                "backup path is not a regular directory",
            ));
        }
        reject_existing_destination(destination, "restore destination")?;
        let manifest_bytes = read_regular_bounded(
            &backup.join("backup.json"),
            MAXIMUM_INTERNAL_BYTES,
            "backup manifest",
        )?;
        let manifest: BackupManifest = strict_json(&manifest_bytes, "backup manifest")?;
        if manifest.contract_version != SOURCE_PROJECT_CONTRACT_VERSION {
            return Err(authority_error(
                DiagnosticClass::Source,
                "source_backup_contract",
                "backup has a predecessor or foreign contract",
            ));
        }
        validate_digest(
            &manifest.current_record_digest,
            "backup current record digest",
        )?;
        let mut paths = BTreeSet::new();
        let mut files = BTreeMap::new();
        let mut total = manifest_bytes.len() as u64;
        for entry in &manifest.entries {
            validate_backup_entry_path(&entry.path)?;
            validate_digest(&entry.digest, "backup entry digest")?;
            if !paths.insert(&entry.path) {
                return Err(authority_error(
                    DiagnosticClass::Corrupt,
                    "source_backup_entry_duplicate",
                    format!("backup repeats entry '{}'", entry.path),
                ));
            }
            let input = confined_backup_path(backup, &entry.path)?;
            let maximum = if entry.path.contains("/dependency/") {
                MAXIMUM_ARTIFACT_BYTES
            } else {
                MAXIMUM_INTERNAL_BYTES.max(SourceLimits::default().maximum_bytes)
            };
            let bytes = read_regular_bounded(&input, maximum, "backup entry")?;
            if bytes.len() != entry.bytes || object_digest("backup-entry", &bytes) != entry.digest {
                return Err(authority_error(
                    DiagnosticClass::Corrupt,
                    "source_backup_entry_digest",
                    format!("backup entry '{}' is corrupt", entry.path),
                ));
            }
            total = total.checked_add(bytes.len() as u64).ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Resource,
                    "source_backup_bytes",
                    "backup byte accounting overflowed",
                )
            })?;
            files.insert(entry.path.clone(), bytes);
        }
        let parent = destination.parent().ok_or_else(|| {
            authority_error(
                DiagnosticClass::Source,
                "source_restore_parent",
                "restore destination has no parent directory",
            )
        })?;
        reject_symlink(parent, "restore parent")?;
        let temporary = temporary_path(parent, "restore-directory")?;
        fs::create_dir(&temporary)
            .map_err(|error| io_error("source_restore_create", &temporary, error))?;
        let mut guard = TemporaryDirectory::new(temporary.clone());
        for (path, bytes) in &files {
            let relative = path.strip_prefix("authority/").ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Corrupt,
                    "source_backup_entry_path",
                    "backup authority entry has a foreign prefix",
                )
            })?;
            let output = temporary
                .join(".lkjscript")
                .join(AUTHORITY_DIRECTORY)
                .join(relative);
            atomic_replace(&output, bytes)?;
        }
        let authority = temporary.join(".lkjscript").join(AUTHORITY_DIRECTORY);
        for directory in [
            authority.join(OBJECTS_DIRECTORY).join(SOURCES_DIRECTORY),
            authority.join(OBJECTS_DIRECTORY).join(MANIFESTS_DIRECTORY),
            authority
                .join(OBJECTS_DIRECTORY)
                .join(DEPENDENCIES_DIRECTORY),
            authority.join(RECORDS_DIRECTORY),
            authority.join(INDEX_DIRECTORY),
        ] {
            fs::create_dir_all(&directory)
                .map_err(|error| io_error("source_restore_directory", &directory, error))?;
        }
        let lock = authority.join(LOCK_FILE);
        atomic_replace(&lock, b"")?;
        let project = ProjectAuthority::open(&temporary)?;
        let doctor = project.doctor(true)?;
        if doctor.current_revision != manifest.current_revision
            || doctor.current_record_digest != manifest.current_record_digest
            || doctor.package_id != manifest.package_id
        {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_backup_current",
                "restored authority does not match the backup manifest",
            ));
        }
        project.restore_working(manifest.current_revision)?;
        sync_directory(&temporary)?;
        fs::rename(&temporary, destination)
            .map_err(|error| io_error("source_restore_publish", destination, error))?;
        sync_directory(parent)?;
        guard.disarm();
        Ok(BackupReceipt {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: manifest.package_id,
            current_revision: manifest.current_revision,
            current_record_digest: manifest.current_record_digest,
            backup_digest: object_digest("backup", &manifest_bytes),
            entry_count: files.len(),
            bytes: total,
        })
    }

    fn publish_loaded(
        &self,
        head: Option<&Head>,
        current: Option<&AcceptedRevision>,
        working: WorkingPackage,
    ) -> Result<ApplyReceipt, Diagnostic> {
        if let Some(current) = current
            && current.authored_digest == working.authored_digest
        {
            return Ok(receipt_for_working(current, &working));
        }
        let revision = head.map_or(0, |head| head.revision.saturating_add(1));
        let package_bytes = working.files.get(PACKAGE_FILE_NAME).ok_or_else(|| {
            authority_error(
                DiagnosticClass::Corrupt,
                "source_package_file_missing",
                "loaded working package omitted its package descriptor",
            )
        })?;
        let package_object = self.write_source(PACKAGE_FILE_NAME, package_bytes)?;
        let mut modules = Vec::new();
        for locator in &working.descriptor.modules {
            let bytes = working.files.get(&locator.path).ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Corrupt,
                    "source_module_file_missing",
                    format!("loaded working package omitted '{}'", locator.path),
                )
            })?;
            modules.push(self.write_source(&locator.path, bytes)?);
        }
        modules.sort_by(|left, right| left.path.cmp(&right.path));
        let mut dependencies = Vec::with_capacity(working.dependencies.len());
        for dependency in &working.dependencies {
            let declared = working
                .descriptor
                .dependencies
                .iter()
                .find(|declared| declared.alias == dependency.alias)
                .ok_or_else(|| {
                    authority_error(
                        DiagnosticClass::Corrupt,
                        "source_dependency_foreign",
                        "validated working dependency is absent from its descriptor",
                    )
                })?;
            let object_digest = object_digest("dependency", &dependency.bytes);
            write_immutable(
                &self.dependency_path(&object_digest),
                &dependency.bytes,
                &self.authority,
            )?;
            dependencies.push(DependencyObject {
                alias: dependency.alias.clone(),
                package_id: declared.package_id.clone(),
                revision_digest: declared.revision_digest.clone(),
                artifact_digest: declared.artifact_digest.clone(),
                object_digest,
                bytes: dependency.bytes.len(),
            });
        }
        dependencies.sort_by(|left, right| left.alias.cmp(&right.alias));
        let manifest = RevisionManifest {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: working.descriptor.package_id.clone(),
            revision,
            authored_digest: working.authored_digest.clone(),
            semantic_digest: working.validated.revision_digest.clone(),
            package: package_object,
            modules,
            dependencies,
        };
        let manifest_bytes = canonical_json(&manifest)?;
        let manifest_digest = object_digest("manifest", &manifest_bytes);
        write_immutable(
            &self.manifest_path(&manifest_digest),
            &manifest_bytes,
            &self.authority,
        )?;
        let record = RevisionRecord {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: working.descriptor.package_id.clone(),
            revision,
            parent_record_digest: head.map(|head| head.record_digest.clone()),
            manifest_digest,
            authored_digest: working.authored_digest.clone(),
            semantic_digest: working.validated.revision_digest.clone(),
        };
        let record_bytes = canonical_json(&record)?;
        let record_digest = object_digest("record", &record_bytes);
        write_immutable(
            &self.record_path(&record_digest),
            &record_bytes,
            &self.authority,
        )?;
        let index = format!("{record_digest}\n");
        write_immutable(
            &self.index_path(revision),
            index.as_bytes(),
            &self.authority,
        )?;
        let core = HeadCore {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            package_id: working.descriptor.package_id.clone(),
            revision,
            record_digest: record_digest.clone(),
        };
        let checksum = object_digest("head", &canonical_json(&core)?);
        let new_head = Head {
            contract_version: core.contract_version,
            package_id: core.package_id,
            revision: core.revision,
            record_digest: core.record_digest,
            checksum,
        };
        atomic_replace(&self.head_path(), &canonical_json(&new_head)?)?;
        sync_directory(&self.authority)?;
        Ok(ApplyReceipt {
            contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
            kind: PublicationKind::Published,
            package_id: working.descriptor.package_id,
            revision,
            record_digest,
            authored_digest: working.authored_digest,
            semantic_changed: current.is_none_or(|current| {
                current.package.revision_digest != working.validated.revision_digest
            }),
            semantic_digest: working.validated.revision_digest,
            file_count: working.files.len(),
            source_bytes: working.source_bytes,
        })
    }

    fn reconstruct_record(&self, digest: &str) -> Result<AcceptedRevision, Diagnostic> {
        let record = self.read_record(digest)?;
        self.reconstruct(record, digest.to_owned())
    }

    fn reconstruct(
        &self,
        record: RevisionRecord,
        record_digest: String,
    ) -> Result<AcceptedRevision, Diagnostic> {
        let manifest = self.read_manifest(&record.manifest_digest)?;
        if manifest.package_id != record.package_id
            || manifest.revision != record.revision
            || manifest.authored_digest != record.authored_digest
            || manifest.semantic_digest != record.semantic_digest
        {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_record_mismatch",
                "revision manifest does not match its record",
            ));
        }
        let mut files = BTreeMap::new();
        let package_bytes = self.read_source(&manifest.package.digest, manifest.package.bytes)?;
        files.insert(manifest.package.path.clone(), package_bytes.clone());
        let descriptor = decode_package(&package_bytes)?;
        validate_manifest_dependencies(&manifest, &descriptor)?;
        let mut documents = Vec::new();
        let mut source_bytes = package_bytes.len();
        for file in &manifest.modules {
            let bytes = self.read_source(&file.digest, file.bytes)?;
            source_bytes = source_bytes.saturating_add(bytes.len());
            documents.push(parse_source(
                file.path.clone(),
                &bytes,
                SourceLimits::default(),
            )?);
            files.insert(file.path.clone(), bytes);
        }
        if source_bytes > MAXIMUM_ACCEPTED_SOURCE_BYTES {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_revision_too_large",
                "accepted revision exceeds aggregate source byte policy",
            ));
        }
        let dependency_inputs = manifest
            .dependencies
            .iter()
            .map(|dependency| {
                Ok(DependencyArtifact {
                    alias: dependency.alias.clone(),
                    bytes: self.read_dependency(&dependency.object_digest, dependency.bytes)?,
                })
            })
            .collect::<Result<Vec<_>, Diagnostic>>()?;
        let package =
            validate_with_dependency_artifacts(descriptor, documents, &dependency_inputs)?;
        if package.revision_digest != manifest.semantic_digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_semantic_digest",
                "independent semantic reconstruction differs from the manifest",
            ));
        }
        if authored_digest(&files) != manifest.authored_digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_authored_digest",
                "independent authored reconstruction differs from the manifest",
            ));
        }
        Ok(AcceptedRevision {
            package,
            revision: record.revision,
            record_digest,
            authored_digest: record.authored_digest,
            files,
            dependencies: dependency_inputs,
        })
    }

    fn load_working(
        &self,
        dependencies: &[DependencyArtifact],
    ) -> Result<WorkingPackage, Diagnostic> {
        load_working_from_root(&self.root, dependencies)
    }

    fn read_head(&self) -> Result<Head, Diagnostic> {
        let bytes = read_regular_bounded(&self.head_path(), MAXIMUM_INTERNAL_BYTES, "source HEAD")?;
        let head: Head = strict_json(&bytes, "source HEAD")?;
        if head.contract_version != SOURCE_PROJECT_CONTRACT_VERSION {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_head_contract",
                "source HEAD has a predecessor or foreign contract",
            ));
        }
        validate_digest(&head.record_digest, "source HEAD record digest")?;
        validate_digest(&head.checksum, "source HEAD checksum")?;
        let core = HeadCore {
            contract_version: head.contract_version,
            package_id: head.package_id.clone(),
            revision: head.revision,
            record_digest: head.record_digest.clone(),
        };
        if object_digest("head", &canonical_json(&core)?) != head.checksum {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_head_checksum",
                "source HEAD checksum is invalid",
            ));
        }
        let record = self.read_record(&head.record_digest)?;
        if record.package_id != head.package_id || record.revision != head.revision {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_head_record_mismatch",
                "source HEAD and current revision record disagree",
            ));
        }
        Ok(head)
    }

    fn read_record(&self, digest: &str) -> Result<RevisionRecord, Diagnostic> {
        validate_digest(digest, "revision record digest")?;
        let bytes = read_regular_bounded(
            &self.record_path(digest),
            MAXIMUM_INTERNAL_BYTES,
            "revision record",
        )?;
        if object_digest("record", &bytes) != digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_record_digest",
                "revision record digest is invalid",
            ));
        }
        let record: RevisionRecord = strict_json(&bytes, "revision record")?;
        if record.contract_version != SOURCE_PROJECT_CONTRACT_VERSION {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_record_contract",
                "revision record has a predecessor or foreign contract",
            ));
        }
        validate_digest(&record.manifest_digest, "manifest digest")?;
        validate_digest(&record.authored_digest, "authored digest")?;
        validate_digest(&record.semantic_digest, "semantic digest")?;
        if let Some(parent) = &record.parent_record_digest {
            validate_digest(parent, "parent record digest")?;
        }
        Ok(record)
    }

    fn read_manifest(&self, digest: &str) -> Result<RevisionManifest, Diagnostic> {
        validate_digest(digest, "revision manifest digest")?;
        let bytes = read_regular_bounded(
            &self.manifest_path(digest),
            MAXIMUM_INTERNAL_BYTES,
            "revision manifest",
        )?;
        if object_digest("manifest", &bytes) != digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_digest",
                "revision manifest digest is invalid",
            ));
        }
        let manifest: RevisionManifest = strict_json(&bytes, "revision manifest")?;
        if manifest.contract_version != SOURCE_PROJECT_CONTRACT_VERSION {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_contract",
                "revision manifest has a predecessor or foreign contract",
            ));
        }
        validate_manifest_files(&manifest)?;
        Ok(manifest)
    }

    fn read_source(&self, digest: &str, expected_bytes: usize) -> Result<Vec<u8>, Diagnostic> {
        validate_digest(digest, "source object digest")?;
        let bytes = read_regular_bounded(
            &self.source_path(digest),
            SourceLimits::default()
                .maximum_bytes
                .max(super::package::MAXIMUM_PACKAGE_BYTES),
            "source object",
        )?;
        if bytes.len() != expected_bytes || object_digest("source", &bytes) != digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_object_digest",
                "source object length or digest is invalid",
            ));
        }
        Ok(bytes)
    }

    fn read_dependency(&self, digest: &str, expected_bytes: usize) -> Result<Vec<u8>, Diagnostic> {
        validate_digest(digest, "dependency object digest")?;
        let bytes = read_regular_bounded(
            &self.dependency_path(digest),
            MAXIMUM_ARTIFACT_BYTES,
            "dependency artifact object",
        )?;
        if bytes.len() != expected_bytes || object_digest("dependency", &bytes) != digest {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_dependency_object_digest",
                "dependency object length or digest is invalid",
            ));
        }
        Ok(bytes)
    }

    fn write_source(&self, path: &str, bytes: &[u8]) -> Result<FileObject, Diagnostic> {
        let digest = object_digest("source", bytes);
        write_immutable(&self.source_path(&digest), bytes, &self.authority)?;
        Ok(FileObject {
            path: path.to_owned(),
            digest,
            bytes: bytes.len(),
        })
    }

    fn lock(&self) -> Result<File, Diagnostic> {
        let path = self.authority.join(LOCK_FILE);
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| io_error("source_lock_open", &path, error))?;
        file.lock_exclusive()
            .map_err(|error| io_error("source_lock_acquire", &path, error))?;
        Ok(file)
    }

    fn head_path(&self) -> PathBuf {
        self.authority.join(HEAD_FILE)
    }

    fn source_path(&self, digest: &str) -> PathBuf {
        self.authority
            .join(OBJECTS_DIRECTORY)
            .join(SOURCES_DIRECTORY)
            .join(digest)
    }

    fn manifest_path(&self, digest: &str) -> PathBuf {
        self.authority
            .join(OBJECTS_DIRECTORY)
            .join(MANIFESTS_DIRECTORY)
            .join(digest)
    }

    fn dependency_path(&self, digest: &str) -> PathBuf {
        self.authority
            .join(OBJECTS_DIRECTORY)
            .join(DEPENDENCIES_DIRECTORY)
            .join(digest)
    }

    fn record_path(&self, digest: &str) -> PathBuf {
        self.authority.join(RECORDS_DIRECTORY).join(digest)
    }

    fn index_path(&self, revision: u64) -> PathBuf {
        self.authority
            .join(INDEX_DIRECTORY)
            .join(format!("{revision:020}.ref"))
    }
}

fn receipt_for_working(current: &AcceptedRevision, working: &WorkingPackage) -> ApplyReceipt {
    ApplyReceipt {
        contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
        kind: PublicationKind::NoChange,
        package_id: current.package.descriptor.package_id.clone(),
        revision: current.revision,
        record_digest: current.record_digest.clone(),
        authored_digest: current.authored_digest.clone(),
        semantic_digest: current.package.revision_digest.clone(),
        semantic_changed: current.package.revision_digest != working.validated.revision_digest,
        file_count: working.files.len(),
        source_bytes: working.source_bytes,
    }
}

fn validation_receipt(current: &AcceptedRevision, working: &WorkingPackage) -> ApplyReceipt {
    let authored_changed = current.authored_digest != working.authored_digest;
    ApplyReceipt {
        contract_version: SOURCE_PROJECT_CONTRACT_VERSION,
        kind: if authored_changed {
            PublicationKind::Validated
        } else {
            PublicationKind::NoChange
        },
        package_id: current.package.descriptor.package_id.clone(),
        revision: current.revision,
        record_digest: current.record_digest.clone(),
        authored_digest: working.authored_digest.clone(),
        semantic_digest: working.validated.revision_digest.clone(),
        semantic_changed: current.package.revision_digest != working.validated.revision_digest,
        file_count: working.files.len(),
        source_bytes: working.source_bytes,
    }
}

fn load_working_from_root(
    root: &Path,
    dependencies: &[DependencyArtifact],
) -> Result<WorkingPackage, Diagnostic> {
    let package_path = confined_working_path(root, PACKAGE_FILE_NAME)?;
    let package_bytes = read_regular_bounded(
        &package_path,
        super::package::MAXIMUM_PACKAGE_BYTES,
        "package descriptor",
    )?;
    let descriptor = decode_package(&package_bytes)?;
    let mut files = BTreeMap::new();
    files.insert(PACKAGE_FILE_NAME.to_owned(), package_bytes.clone());
    let mut documents = Vec::new();
    let mut source_bytes = package_bytes.len();
    for locator in &descriptor.modules {
        let path = confined_working_path(root, &locator.path)?;
        let bytes = read_regular_bounded(
            &path,
            SourceLimits::default().maximum_bytes,
            "module source",
        )?;
        source_bytes = source_bytes.saturating_add(bytes.len());
        if source_bytes > MAXIMUM_ACCEPTED_SOURCE_BYTES {
            return Err(authority_error(
                DiagnosticClass::Resource,
                "source_working_total",
                format!("working source exceeds {MAXIMUM_ACCEPTED_SOURCE_BYTES} aggregate bytes"),
            ));
        }
        documents.push(parse_source(
            locator.path.clone(),
            &bytes,
            SourceLimits::default(),
        )?);
        files.insert(locator.path.clone(), bytes);
    }
    let validated =
        validate_with_dependency_artifacts(descriptor.clone(), documents, dependencies)?;
    let authored_digest = authored_digest(&files);
    Ok(WorkingPackage {
        descriptor,
        validated,
        files,
        authored_digest,
        source_bytes,
        dependencies: dependencies.to_vec(),
    })
}

fn validate_with_dependency_artifacts(
    descriptor: PackageDescriptor,
    documents: Vec<super::syntax::SourceDocument>,
    dependencies: &[DependencyArtifact],
) -> Result<ValidatedPackage, Diagnostic> {
    if descriptor.dependencies.len() != dependencies.len() {
        return Err(authority_error(
            DiagnosticClass::Semantic,
            "source_dependency_count",
            format!(
                "package declares {} dependencies but {} artifact inputs were supplied",
                descriptor.dependencies.len(),
                dependencies.len()
            ),
        ));
    }
    let mut aliases = BTreeSet::new();
    let mut total_bytes = 0usize;
    let mut loaded: Vec<(&DependencyArtifact, LoadedArtifact)> = Vec::new();
    for dependency in dependencies {
        if !aliases.insert(dependency.alias.as_str()) {
            return Err(authority_error(
                DiagnosticClass::Semantic,
                "source_dependency_duplicate",
                format!(
                    "dependency artifact alias '{}' is repeated",
                    dependency.alias
                ),
            ));
        }
        total_bytes = total_bytes
            .checked_add(dependency.bytes.len())
            .ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Resource,
                    "source_dependency_total",
                    "dependency artifact byte accounting overflowed",
                )
            })?;
        if total_bytes > MAXIMUM_ACCEPTED_DEPENDENCY_BYTES {
            return Err(authority_error(
                DiagnosticClass::Resource,
                "source_dependency_total",
                format!(
                    "dependency artifacts exceed {MAXIMUM_ACCEPTED_DEPENDENCY_BYTES} aggregate bytes"
                ),
            ));
        }
        loaded.push((dependency, load_artifact(&dependency.bytes)?));
    }
    let mut exact = Vec::with_capacity(descriptor.dependencies.len());
    for declared in &descriptor.dependencies {
        let (_, artifact) = loaded
            .iter()
            .find(|(input, _)| input.alias == declared.alias)
            .ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Semantic,
                    "source_dependency_missing",
                    format!("dependency artifact '{}' is absent", declared.alias),
                )
            })?;
        let package = artifact.packages.get(&declared.package_id).ok_or_else(|| {
            authority_error(
                DiagnosticClass::Semantic,
                "source_dependency_package",
                format!(
                    "dependency artifact '{}' omits package identity '{}'",
                    declared.alias,
                    declared.package_id.as_str()
                ),
            )
        })?;
        if package.revision_digest != declared.revision_digest
            || package_artifact_digest(package) != declared.artifact_digest
        {
            return Err(authority_error(
                DiagnosticClass::Semantic,
                "source_dependency_identity",
                format!(
                    "dependency artifact '{}' has a foreign exact identity",
                    declared.alias
                ),
            ));
        }
        exact.push(ExactDependency {
            alias: &declared.alias,
            package,
            artifact_digest: &declared.artifact_digest,
        });
    }
    validate_package_documents(descriptor.clone(), documents, &exact)
}

fn verify_expected(
    current: &AcceptedRevision,
    expected_revision: u64,
    expected_record_digest: &str,
) -> Result<(), Diagnostic> {
    if current.revision != expected_revision || current.record_digest != expected_record_digest {
        return Err(stale_error(
            expected_revision,
            expected_record_digest,
            current.revision,
            &current.record_digest,
        ));
    }
    Ok(())
}

fn stale_error(
    expected_revision: u64,
    expected: &str,
    actual_revision: u64,
    actual: &str,
) -> Diagnostic {
    authority_error(
        DiagnosticClass::Source,
        "source_stale_base",
        format!(
            "expected revision {expected_revision} record {expected}; current revision is {actual_revision} record {actual}"
        ),
    )
}

fn validate_manifest_files(manifest: &RevisionManifest) -> Result<(), Diagnostic> {
    if manifest.package.path != PACKAGE_FILE_NAME {
        return Err(authority_error(
            DiagnosticClass::Corrupt,
            "source_manifest_package_path",
            "manifest package descriptor path is not current",
        ));
    }
    let mut paths = BTreeSet::new();
    paths.insert(PACKAGE_FILE_NAME);
    for file in &manifest.modules {
        validate_relative_path(&file.path, "manifest module path", true)?;
        if !paths.insert(&file.path) {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_path_duplicate",
                format!("manifest repeats path '{}'", file.path),
            ));
        }
        validate_digest(&file.digest, "manifest source digest")?;
    }
    validate_digest(&manifest.package.digest, "manifest package digest")?;
    let mut dependency_aliases = BTreeSet::new();
    for dependency in &manifest.dependencies {
        if !dependency_aliases.insert(&dependency.alias) {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_dependency_duplicate",
                format!("manifest repeats dependency alias '{}'", dependency.alias),
            ));
        }
        validate_digest(&dependency.revision_digest, "dependency revision digest")?;
        validate_digest(&dependency.artifact_digest, "dependency artifact digest")?;
        validate_digest(&dependency.object_digest, "dependency object digest")?;
        if dependency.bytes > MAXIMUM_ARTIFACT_BYTES {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_dependency_size",
                "manifest dependency object exceeds its byte limit",
            ));
        }
    }
    Ok(())
}

fn validate_manifest_dependencies(
    manifest: &RevisionManifest,
    descriptor: &PackageDescriptor,
) -> Result<(), Diagnostic> {
    if manifest.dependencies.len() != descriptor.dependencies.len() {
        return Err(authority_error(
            DiagnosticClass::Corrupt,
            "source_manifest_dependency_count",
            "manifest dependency count differs from the package descriptor",
        ));
    }
    for declared in &descriptor.dependencies {
        let retained = manifest
            .dependencies
            .iter()
            .find(|dependency| dependency.alias == declared.alias)
            .ok_or_else(|| {
                authority_error(
                    DiagnosticClass::Corrupt,
                    "source_manifest_dependency_missing",
                    format!("manifest omits package dependency '{}'", declared.alias),
                )
            })?;
        if retained.package_id != declared.package_id
            || retained.revision_digest != declared.revision_digest
            || retained.artifact_digest != declared.artifact_digest
        {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_manifest_dependency_identity",
                format!(
                    "manifest dependency '{}' has a foreign exact identity",
                    declared.alias
                ),
            ));
        }
    }
    Ok(())
}

fn authored_digest(files: &BTreeMap<String, Vec<u8>>) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.authored-revision.v1");
    for (path, bytes) in files {
        hasher.update(&(path.len() as u64).to_be_bytes());
        hasher.update(path.as_bytes());
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    hex(hasher.finalize().as_bytes())
}

fn object_digest(kind: &str, bytes: &[u8]) -> String {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.source-object.v1");
    hasher.update(&(kind.len() as u64).to_be_bytes());
    hasher.update(kind.as_bytes());
    hasher.update(&(bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    hex(hasher.finalize().as_bytes())
}

fn canonical_json(value: &impl Serialize) -> Result<Vec<u8>, Diagnostic> {
    let mut bytes = serde_json::to_vec(value).map_err(|error| {
        authority_error(
            DiagnosticClass::Infrastructure,
            "source_internal_encode",
            format!("internal authority encoding failed: {error}"),
        )
    })?;
    bytes.push(b'\n');
    Ok(bytes)
}

fn strict_json<T: DeserializeOwned>(bytes: &[u8], label: &str) -> Result<T, Diagnostic> {
    let mut decoder = serde_json::Deserializer::from_slice(bytes);
    let value = T::deserialize(&mut decoder).map_err(|error| {
        authority_error(
            DiagnosticClass::Corrupt,
            "source_internal_json",
            format!("{label} is malformed: {error}"),
        )
    })?;
    decoder.end().map_err(|error| {
        authority_error(
            DiagnosticClass::Corrupt,
            "source_internal_trailing",
            format!("{label} has trailing input: {error}"),
        )
    })?;
    Ok(value)
}

fn canonical_project_root(root: &Path) -> Result<PathBuf, Diagnostic> {
    let metadata = fs::symlink_metadata(root)
        .map_err(|error| io_error("source_root_metadata", root, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(authority_error(
            DiagnosticClass::Source,
            "source_root_type",
            "project root must be an existing non-symlink directory",
        ));
    }
    fs::canonicalize(root).map_err(|error| io_error("source_root_canonicalize", root, error))
}

fn confined_working_path(root: &Path, relative: &str) -> Result<PathBuf, Diagnostic> {
    if relative != PACKAGE_FILE_NAME {
        validate_relative_path(relative, "working source path", true)?;
    }
    let mut current = root.to_path_buf();
    for component in relative.split('/') {
        current.push(component);
        if current.exists() {
            reject_symlink(&current, "working source component")?;
        }
    }
    Ok(current)
}

fn reject_symlink(path: &Path, label: &str) -> Result<(), Diagnostic> {
    let metadata = fs::symlink_metadata(path)
        .map_err(|error| io_error("source_path_metadata", path, error))?;
    if metadata.file_type().is_symlink() {
        return Err(authority_error(
            DiagnosticClass::Source,
            "source_path_symlink",
            format!("{label} '{}' is a symbolic link", path.display()),
        ));
    }
    Ok(())
}

fn read_regular_bounded(path: &Path, maximum: usize, label: &str) -> Result<Vec<u8>, Diagnostic> {
    reject_symlink(path, label)?;
    let metadata =
        fs::metadata(path).map_err(|error| io_error("source_read_metadata", path, error))?;
    if !metadata.is_file() {
        return Err(authority_error(
            DiagnosticClass::Source,
            "source_file_type",
            format!("{label} '{}' is not a regular file", path.display()),
        ));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| {
        authority_error(
            DiagnosticClass::Resource,
            "source_file_length",
            format!("{label} length cannot be represented"),
        )
    })?;
    if length > maximum {
        return Err(authority_error(
            DiagnosticClass::Resource,
            "source_file_too_large",
            format!("{label} has {length} bytes; the limit is {maximum}"),
        ));
    }
    let file = File::open(path).map_err(|error| io_error("source_read_open", path, error))?;
    let mut bytes = Vec::with_capacity(length);
    file.take((maximum as u64).saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| io_error("source_read", path, error))?;
    if bytes.len() > maximum || bytes.len() != length {
        return Err(authority_error(
            DiagnosticClass::Resource,
            "source_file_changed",
            format!("{label} changed while it was read or exceeded its limit"),
        ));
    }
    Ok(bytes)
}

fn write_immutable(path: &Path, bytes: &[u8], sync_root: &Path) -> Result<(), Diagnostic> {
    if path.exists() {
        let current = read_regular_bounded(path, bytes.len(), "immutable authority object")?;
        if current != bytes {
            return Err(authority_error(
                DiagnosticClass::Corrupt,
                "source_object_collision",
                format!(
                    "immutable object '{}' has conflicting bytes",
                    path.display()
                ),
            ));
        }
        return Ok(());
    }
    let parent = path.parent().ok_or_else(|| {
        authority_error(
            DiagnosticClass::Infrastructure,
            "source_object_parent",
            "immutable object has no parent directory",
        )
    })?;
    let temporary = temporary_path(parent, "object")?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| io_error("source_object_create", &temporary, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("source_object_write", &temporary, error))?;
    match fs::hard_link(&temporary, path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            let current = read_regular_bounded(path, bytes.len(), "immutable authority object")?;
            if current != bytes {
                let _ = fs::remove_file(&temporary);
                return Err(authority_error(
                    DiagnosticClass::Corrupt,
                    "source_object_collision",
                    format!(
                        "immutable object '{}' has conflicting bytes",
                        path.display()
                    ),
                ));
            }
        }
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(io_error("source_object_publish", path, error));
        }
    }
    fs::remove_file(&temporary)
        .map_err(|error| io_error("source_object_temp_remove", &temporary, error))?;
    sync_directory(parent)?;
    sync_directory(sync_root)?;
    Ok(())
}

fn atomic_replace(path: &Path, bytes: &[u8]) -> Result<(), Diagnostic> {
    let parent = path.parent().ok_or_else(|| {
        authority_error(
            DiagnosticClass::Infrastructure,
            "source_replace_parent",
            "replacement path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| io_error("source_replace_parent", parent, error))?;
    let temporary = temporary_path(parent, "replace")?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temporary)
        .map_err(|error| io_error("source_replace_create", &temporary, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| io_error("source_replace_write", &temporary, error))?;
    fs::rename(&temporary, path)
        .map_err(|error| io_error("source_replace_publish", path, error))?;
    sync_directory(parent)
}

fn temporary_path(parent: &Path, kind: &str) -> Result<PathBuf, Diagnostic> {
    let mut random = [0u8; 8];
    getrandom::fill(&mut random).map_err(|error| {
        authority_error(
            DiagnosticClass::Infrastructure,
            "source_temporary_random",
            format!("temporary file randomness failed: {error}"),
        )
    })?;
    Ok(parent.join(format!(".lkjscript-{kind}-{}", hex(&random))))
}

fn sync_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|file| file.sync_all())
        .map_err(|error| io_error("source_directory_sync", path, error))
}

fn validate_digest(value: &str, label: &str) -> Result<(), Diagnostic> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(authority_error(
            DiagnosticClass::Corrupt,
            "source_digest_encoding",
            format!("{label} is not 64 lowercase hexadecimal characters"),
        ));
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut result = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        result.push(DIGITS[(byte >> 4) as usize] as char);
        result.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    result
}

fn authority_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

fn io_error(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    authority_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn insert_backup_file(
    output: &mut BTreeMap<String, Vec<u8>>,
    path: &str,
    source: &Path,
    maximum: usize,
) -> Result<(), Diagnostic> {
    if output.contains_key(path) {
        return Ok(());
    }
    output.insert(
        path.to_owned(),
        read_regular_bounded(source, maximum, "authority backup input")?,
    );
    Ok(())
}

fn reject_existing_destination(path: &Path, label: &str) -> Result<(), Diagnostic> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(authority_error(
            DiagnosticClass::Source,
            "source_destination_exists",
            format!("{label} '{}' already exists", path.display()),
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_error("source_destination_metadata", path, error)),
    }
}

fn validate_backup_entry_path(path: &str) -> Result<(), Diagnostic> {
    super::package::validate_relative_path(path, "backup entry path", false)?;
    let allowed = path == "authority/HEAD.json"
        || path
            .strip_prefix("authority/records/")
            .is_some_and(|digest| validate_digest(digest, "backup record digest").is_ok())
        || path
            .strip_prefix("authority/objects/source/")
            .is_some_and(|digest| validate_digest(digest, "backup source digest").is_ok())
        || path
            .strip_prefix("authority/objects/manifest/")
            .is_some_and(|digest| validate_digest(digest, "backup manifest digest").is_ok())
        || path
            .strip_prefix("authority/objects/dependency/")
            .is_some_and(|digest| validate_digest(digest, "backup dependency digest").is_ok());
    if !allowed {
        return Err(authority_error(
            DiagnosticClass::Corrupt,
            "source_backup_entry_path",
            format!("backup entry path '{path}' is not part of the current authority layout"),
        ));
    }
    Ok(())
}

fn confined_backup_path(root: &Path, relative: &str) -> Result<PathBuf, Diagnostic> {
    validate_backup_entry_path(relative)?;
    let mut current = root.to_path_buf();
    for component in relative.split('/') {
        current.push(component);
        if fs::symlink_metadata(&current).is_ok() {
            reject_symlink(&current, "backup path component")?;
        }
    }
    Ok(current)
}

struct TemporaryDirectory {
    path: Option<PathBuf>,
}

impl TemporaryDirectory {
    fn new(path: PathBuf) -> Self {
        Self { path: Some(path) }
    }

    fn disarm(&mut self) {
        self.path = None;
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        if let Some(path) = &self.path {
            let _ = fs::remove_dir_all(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{build_artifact, package_artifact_digest};
    use tempfile::TempDir;

    fn write(path: &Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
        fs::write(path, bytes).expect("write fixture");
    }

    fn project_files(root: &Path) {
        write(
            &root.join(PACKAGE_FILE_NAME),
            br#"{"contract_version":1,"package_id":"1234567890abcdef1234567890abcdef","name":"sample","modules":[{"name":"main","path":"src/main.lkj"}],"dependencies":[],"targets":[]}
"#,
        );
        write(
            &root.join("src/main.lkj"),
            b"; comment\n(module main (export Item) (record Item (name Text)))\n",
        );
    }

    #[test]
    fn exact_publication_no_change_and_formatting_history() {
        let temporary = TempDir::new().expect("temporary project");
        project_files(temporary.path());
        let (project, initial) =
            ProjectAuthority::initialize(temporary.path(), &[]).expect("initialize");
        assert_eq!(initial.kind, PublicationKind::Published);
        assert_eq!(initial.revision, 0);

        let no_change = project
            .apply_working(0, &initial.record_digest, &[])
            .expect("no change");
        assert_eq!(no_change.kind, PublicationKind::NoChange);
        assert_eq!(no_change.record_digest, initial.record_digest);

        write(
            &temporary.path().join("src/main.lkj"),
            b"; changed formatting\n(module main\n  (export Item)\n  (record Item (name Text)))\n",
        );
        let validated = project
            .validate_working(0, &initial.record_digest, &[])
            .expect("validate formatting proposal");
        assert_eq!(validated.kind, PublicationKind::Validated);
        assert_ne!(validated.authored_digest, initial.authored_digest);
        assert_eq!(validated.semantic_digest, initial.semantic_digest);
        assert!(!validated.semantic_changed);
        assert_eq!(project.current().expect("still current").revision, 0);

        let changed = project
            .apply_working(0, &initial.record_digest, &[])
            .expect("formatting publication");
        assert_eq!(changed.kind, PublicationKind::Published);
        assert_eq!(changed.revision, 1);
        assert!(!changed.semantic_changed);
        assert_ne!(changed.authored_digest, initial.authored_digest);
        assert_eq!(changed.semantic_digest, initial.semantic_digest);

        let stale = project
            .apply_working(0, &initial.record_digest, &[])
            .expect_err("stale base rejects even after a no-change proposal");
        assert_eq!(stale.code, "source_stale_base");
    }

    #[test]
    fn current_history_and_independent_reconstruction_agree() {
        let temporary = TempDir::new().expect("temporary project");
        project_files(temporary.path());
        let (project, initial) =
            ProjectAuthority::initialize(temporary.path(), &[]).expect("initialize");
        write(
            &temporary.path().join("src/main.lkj"),
            b"(module main (export Item) (record Item (name Text) (revision I64)))\n",
        );
        let second = project
            .apply_working(0, &initial.record_digest, &[])
            .expect("semantic change");
        assert!(second.semantic_changed);
        let current = project.current().expect("current reconstructs");
        let first = project.revision(0).expect("history reconstructs");
        assert_eq!(current.revision, 1);
        assert_eq!(first.revision, 0);
        assert_ne!(
            current.package.revision_digest,
            first.package.revision_digest
        );
        assert_eq!(
            project.doctor(true).expect("deep doctor").records_checked,
            2
        );

        project.restore_working(0).expect("restore proposal");
        let status = project.status(&[]).expect("restored status");
        assert!(status.authored_changed);
        assert_eq!(
            status.working_semantic_digest,
            first.package.revision_digest
        );
    }

    #[test]
    fn predecessor_and_corrupt_head_reject() {
        let predecessor = TempDir::new().expect("predecessor");
        fs::create_dir_all(predecessor.path().join(".lkjscript")).expect("marker parent");
        write(&predecessor.path().join(".lkjscript/project"), b"old\n");
        let error = ProjectAuthority::open(predecessor.path()).expect_err("predecessor rejects");
        assert_eq!(error.code, "source_predecessor_rejected");

        let temporary = TempDir::new().expect("temporary project");
        project_files(temporary.path());
        let (project, _) = ProjectAuthority::initialize(temporary.path(), &[]).expect("initialize");
        let mut head = fs::read(project.head_path()).expect("head bytes");
        let index = head.iter().position(|byte| *byte == b'a').unwrap_or(0);
        head[index] = if head[index] == b'a' { b'b' } else { b'a' };
        fs::write(project.head_path(), head).expect("corrupt head");
        let error = ProjectAuthority::open(temporary.path()).expect_err("corrupt head rejects");
        assert!(matches!(error.class, DiagnosticClass::Corrupt));
    }

    #[test]
    fn rejected_initialization_leaves_no_authority() {
        let temporary = TempDir::new().expect("temporary project");
        project_files(temporary.path());
        write(
            &temporary.path().join("src/main.lkj"),
            b"(module main (record Broken (value MissingType)))\n",
        );
        let error = ProjectAuthority::initialize(temporary.path(), &[])
            .expect_err("invalid source must reject initialization");
        assert!(matches!(error.class, DiagnosticClass::Semantic));
        assert!(!temporary.path().join(".lkjscript/source-v1").exists());
    }

    #[test]
    fn rejected_intrinsic_proposal_publishes_no_revision() {
        let temporary = TempDir::new().expect("temporary project");
        project_files(temporary.path());
        let (project, initial) =
            ProjectAuthority::initialize(temporary.path(), &[]).expect("initialize");
        write(
            &temporary.path().join("src/main.lkj"),
            b"(module main (extern forged ((value Text)) Text native.forged))\n",
        );

        let validation = project
            .validate_working(0, &initial.record_digest, &[])
            .expect_err("unknown intrinsic rejects validate-only");
        assert_eq!(validation.code, "intrinsic_unknown");
        let application = project
            .apply_working(0, &initial.record_digest, &[])
            .expect_err("unknown intrinsic rejects publication");
        assert_eq!(application.code, "intrinsic_unknown");

        let current = project
            .current()
            .expect("accepted authority remains readable");
        assert_eq!(current.revision, 0);
        assert_eq!(current.record_digest, initial.record_digest);
        assert_eq!(
            project.doctor(true).expect("deep doctor").records_checked,
            1
        );
    }

    #[test]
    fn accepted_revision_retains_exact_dependency_for_offline_reconstruction() {
        let dependency_descriptor = decode_package(
            br#"{"contract_version":1,"package_id":"abcdef1234567890abcdef1234567890","name":"library","modules":[{"name":"core","path":"src/core.lkj"}],"dependencies":[],"targets":[]}"#,
        )
        .expect("dependency descriptor");
        let dependency_document = parse_source(
            "src/core.lkj",
            b"(module core (export Shared) (record Shared (value Text)))\n",
            SourceLimits::default(),
        )
        .expect("dependency source");
        let dependency =
            validate_package_documents(dependency_descriptor, vec![dependency_document], &[])
                .expect("dependency package");
        let (artifact_bytes, _) =
            build_artifact(&dependency, &[&dependency]).expect("dependency artifact");

        let temporary = TempDir::new().expect("temporary project");
        let root_descriptor = format!(
            "{{\"contract_version\":1,\"package_id\":\"1234567890abcdef1234567890abcdef\",\"name\":\"sample\",\"modules\":[{{\"name\":\"main\",\"path\":\"src/main.lkj\"}}],\"dependencies\":[{{\"alias\":\"library\",\"package_id\":\"abcdef1234567890abcdef1234567890\",\"revision_digest\":\"{}\",\"artifact_digest\":\"{}\",\"artifact\":\"dependencies/library.lkja\"}}],\"targets\":[]}}\n",
            dependency.revision_digest,
            package_artifact_digest(&dependency),
        );
        write(
            &temporary.path().join(PACKAGE_FILE_NAME),
            root_descriptor.as_bytes(),
        );
        write(
            &temporary.path().join("src/main.lkj"),
            b"(module main (import library library.core) (record UsesShared (item library.Shared)))\n",
        );
        let input =
            DependencyArtifact::decode("library", artifact_bytes).expect("dependency input");
        let (project, _) = ProjectAuthority::initialize(temporary.path(), &[input])
            .expect("initialize dependent project");

        // No caller supplies dependencies to current or historical reconstruction.
        let current = project.current().expect("offline current reconstruction");
        assert_eq!(current.package.descriptor.dependencies.len(), 1);
        assert_eq!(
            project
                .doctor(true)
                .expect("offline deep doctor")
                .dependency_objects_checked,
            1
        );
    }

    #[test]
    fn portable_backup_restores_history_and_working_source() {
        let temporary = TempDir::new().expect("temporary project");
        let root = temporary.path().join("original");
        fs::create_dir(&root).expect("project root");
        project_files(&root);
        let (project, initial) = ProjectAuthority::initialize(&root, &[]).expect("initialize");
        write(
            &root.join("src/main.lkj"),
            b"(module main (export Item) (record Item (name Text) (revision I64)))\n",
        );
        project
            .apply_working(0, &initial.record_digest, &[])
            .expect("publish second revision");
        let backup = temporary.path().join("portable-backup");
        let backup_receipt = project.backup_to(&backup).expect("create backup");
        assert!(backup_receipt.entry_count >= 7);

        let restored = temporary.path().join("restored");
        let restore_receipt =
            ProjectAuthority::restore_backup(&backup, &restored).expect("restore backup");
        assert_eq!(backup_receipt.backup_digest, restore_receipt.backup_digest);
        let restored_project = ProjectAuthority::open(&restored).expect("open restored project");
        assert_eq!(
            restored_project
                .current()
                .expect("restored current")
                .revision,
            1
        );
        assert_eq!(
            restored_project
                .revision(0)
                .expect("restored history")
                .revision,
            0
        );
        assert_eq!(
            fs::read(restored.join("src/main.lkj")).expect("restored working source"),
            b"(module main (export Item) (record Item (name Text) (revision I64)))\n"
        );
    }
}
