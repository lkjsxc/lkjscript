//! Explicit non-executable draft authority over one accepted semantic base.

use super::diagnostic::{Diagnostic, DiagnosticClass};
use super::language::Type;
use super::meaning::GRAPH_CONTRACT_IDENTITY;
use super::packed;
use super::repository::SemanticRepository;
use super::semantic_id::{
    ConflictId, DeclarationId, DraftId, ExpressionId, RepositoryId, RevisionId,
};
use super::semantic_transaction::{
    MAXIMUM_TRANSACTION_AFFECTED_OWNERS, MAXIMUM_TRANSACTION_OPERATIONS, MAXIMUM_TRANSACTION_WORK,
    OwnerSelector, SemanticOperation, SemanticPrecondition, TransactionBudget, TransactionMode,
    TransactionRequest, TransactionResult, TransactionStatus, execute_transaction,
};
use bincode::{Decode, Encode};
use fs2::FileExt;
use serde::Serialize;
use std::collections::BTreeSet;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub const DRAFT_CONTRACT_VERSION: u16 = 4;
pub const MAXIMUM_DRAFT_BYTES: usize = 64 * 1_048_576;
pub const MAXIMUM_DRAFTS: usize = 10_000;
pub const MAXIMUM_DRAFT_HOLES: usize = 100_000;
pub const MAXIMUM_DRAFT_CONFLICTS: usize = 100_000;

const DRAFT_MAGIC: [u8; 8] = *b"LKJDRF04";
const DRAFT_DIGEST_DOMAIN: &str = "lkjscript.semantic-draft.v4";
const DRAFT_DIRECTORY: &str = "drafts";
const REPOSITORY_LOCK: &str = "LOCK";

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftHole {
    pub expression: ExpressionId,
    pub owner: DeclarationId,
    pub expected_type: Type,
    pub candidates: Vec<String>,
}

#[derive(Decode, Encode, Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DraftConflictKind {
    ConcurrentChange,
    DeleteModify,
    RenameCollision,
    MoveCollision,
    Preconditions,
}

#[derive(Decode, Encode, Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftConflict {
    pub id: ConflictId,
    pub kind: DraftConflictKind,
    pub owner: Option<OwnerSelector>,
    pub summary: String,
}

#[derive(Decode, Encode, Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRecord {
    pub contract_version: u16,
    pub graph_contract: String,
    pub repository_id: RepositoryId,
    pub id: DraftId,
    pub base_revision: RevisionId,
    pub generation: u64,
    pub operations: Vec<SemanticOperation>,
    pub preconditions: Vec<SemanticPrecondition>,
    pub holes: Vec<DraftHole>,
    pub conflicts: Vec<DraftConflict>,
    pub intent: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftSummary {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub id: DraftId,
    pub base_revision: RevisionId,
    pub current_revision: RevisionId,
    pub generation: u64,
    pub operations: usize,
    pub preconditions: usize,
    pub holes: usize,
    pub conflicts: usize,
    pub publishable_shape: bool,
    pub intent: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftMutationReceipt {
    pub contract_version: u16,
    pub repository_id: RepositoryId,
    pub id: DraftId,
    pub status: &'static str,
    pub base_revision: RevisionId,
    pub generation: u64,
    pub appended_operations: usize,
    pub total_operations: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DraftRebaseResult {
    pub updated: bool,
    pub draft: DraftSummary,
    pub validation: TransactionResult,
}

impl DraftRecord {
    pub(crate) fn encode(&self) -> Result<Vec<u8>, Diagnostic> {
        self.validate_shape()?;
        packed::encode(DRAFT_MAGIC, DRAFT_DIGEST_DOMAIN, self, MAXIMUM_DRAFT_BYTES)
    }

    pub(crate) fn decode(bytes: &[u8]) -> Result<Self, Diagnostic> {
        let value: Self =
            packed::decode(bytes, DRAFT_MAGIC, DRAFT_DIGEST_DOMAIN, MAXIMUM_DRAFT_BYTES)?;
        value.validate_shape()?;
        Ok(value)
    }

    fn validate_shape(&self) -> Result<(), Diagnostic> {
        if self.contract_version != DRAFT_CONTRACT_VERSION
            || self.graph_contract != GRAPH_CONTRACT_IDENTITY
        {
            return Err(draft_error(
                DiagnosticClass::Source,
                "semantic_draft_contract",
                "draft uses an unknown draft or graph contract",
            ));
        }
        if self.operations.len() > MAXIMUM_TRANSACTION_OPERATIONS
            || self.preconditions.len() > MAXIMUM_TRANSACTION_OPERATIONS
            || self.holes.len() > MAXIMUM_DRAFT_HOLES
            || self.conflicts.len() > MAXIMUM_DRAFT_CONFLICTS
            || self
                .intent
                .as_ref()
                .is_some_and(|value| value.len() > 4_096)
        {
            return Err(draft_error(
                DiagnosticClass::Resource,
                "semantic_draft_item_limit",
                "draft exceeds a canonical item or intent bound",
            ));
        }
        let holes = self
            .holes
            .iter()
            .map(|hole| hole.expression)
            .collect::<BTreeSet<_>>();
        let conflicts = self
            .conflicts
            .iter()
            .map(|conflict| conflict.id)
            .collect::<BTreeSet<_>>();
        if holes.len() != self.holes.len() || conflicts.len() != self.conflicts.len() {
            return Err(draft_error(
                DiagnosticClass::Corrupt,
                "semantic_draft_identity_duplicate",
                "draft hole or conflict identity is duplicated",
            ));
        }
        Ok(())
    }

    fn transaction_request(&self, idempotency_key: Option<String>) -> TransactionRequest {
        TransactionRequest {
            contract_version: super::semantic_transaction::TRANSACTION_CONTRACT_VERSION,
            graph_contract: GRAPH_CONTRACT_IDENTITY.to_owned(),
            repository_id: self.repository_id,
            base_revision: self.base_revision,
            draft: None,
            idempotency_key,
            preconditions: self.preconditions.clone(),
            operations: self.operations.clone(),
            budget: TransactionBudget {
                maximum_operations: self.operations.len().max(1),
                maximum_work: MAXIMUM_TRANSACTION_WORK,
                maximum_affected_owners: MAXIMUM_TRANSACTION_AFFECTED_OWNERS,
            },
            intent: self.intent.clone(),
        }
    }
}

pub struct SemanticDraftStore<'a> {
    repository: &'a SemanticRepository,
}

impl<'a> SemanticDraftStore<'a> {
    pub fn new(repository: &'a SemanticRepository) -> Self {
        Self { repository }
    }

    pub fn count(&self) -> Result<usize, Diagnostic> {
        let directory = self.directory();
        ensure_directory(&directory, "semantic_draft_directory")?;
        let mut count = 0usize;
        for entry in fs::read_dir(&directory)
            .map_err(|error| draft_io("semantic_draft_list", &directory, error))?
        {
            let entry =
                entry.map_err(|error| draft_io("semantic_draft_list", &directory, error))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| draft_io("semantic_draft_type", &path, error))?;
            if file_type.is_symlink()
                || !file_type.is_file()
                || path.extension().and_then(|value| value.to_str()) != Some("lkjd")
            {
                return Err(draft_error(
                    DiagnosticClass::Corrupt,
                    "semantic_draft_directory_entry",
                    format!(
                        "draft directory contains an invalid entry '{}''",
                        path.display()
                    ),
                ));
            }
            count = count.checked_add(1).ok_or_else(draft_count_exhausted)?;
            if count > MAXIMUM_DRAFTS {
                return Err(draft_count_exhausted());
            }
        }
        Ok(count)
    }

    pub fn validate_all(&self) -> Result<usize, Diagnostic> {
        let directory = self.directory();
        ensure_directory(&directory, "semantic_draft_directory")?;
        let mut ids = Vec::new();
        for entry in fs::read_dir(&directory)
            .map_err(|error| draft_io("semantic_draft_list", &directory, error))?
        {
            let entry =
                entry.map_err(|error| draft_io("semantic_draft_list", &directory, error))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|error| draft_io("semantic_draft_type", &path, error))?;
            if file_type.is_symlink()
                || !file_type.is_file()
                || path.extension().and_then(|value| value.to_str()) != Some("lkjd")
            {
                return Err(draft_error(
                    DiagnosticClass::Corrupt,
                    "semantic_draft_directory_entry",
                    format!(
                        "draft directory contains an invalid entry '{}'",
                        path.display()
                    ),
                ));
            }
            let id = path
                .file_stem()
                .and_then(|value| value.to_str())
                .and_then(|value| value.parse::<DraftId>().ok())
                .ok_or_else(|| {
                    draft_error(
                        DiagnosticClass::Corrupt,
                        "semantic_draft_directory_entry",
                        format!(
                            "draft directory contains an invalid entry '{}'",
                            path.display()
                        ),
                    )
                })?;
            ids.push(id);
            if ids.len() > MAXIMUM_DRAFTS {
                return Err(draft_count_exhausted());
            }
        }
        ids.sort();
        for id in &ids {
            self.read(*id)?;
        }
        Ok(ids.len())
    }

    pub fn create(
        &self,
        base_revision: Option<RevisionId>,
        intent: Option<String>,
    ) -> Result<DraftSummary, Diagnostic> {
        if intent.as_ref().is_some_and(|value| value.len() > 4_096) {
            return Err(draft_error(
                DiagnosticClass::Resource,
                "semantic_draft_intent_limit",
                "draft intent exceeds 4096 bytes",
            ));
        }
        let current = self.repository.current_binding()?;
        let base_revision = base_revision.unwrap_or(current.head.revision);
        let historical = self.repository.reconstruct_revision(base_revision)?;
        if historical.record.core.repository_id != current.head.repository_id {
            return Err(draft_error(
                DiagnosticClass::Source,
                "semantic_draft_foreign_base",
                "draft base belongs to a foreign repository",
            ));
        }
        let lock = self.lock_exclusive()?;
        if self.count()? >= MAXIMUM_DRAFTS {
            FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))?;
            return Err(draft_count_exhausted());
        }
        let id = DraftId::generate()?;
        let record = DraftRecord {
            contract_version: DRAFT_CONTRACT_VERSION,
            graph_contract: GRAPH_CONTRACT_IDENTITY.to_owned(),
            repository_id: current.head.repository_id,
            id,
            base_revision,
            generation: 0,
            operations: Vec::new(),
            preconditions: Vec::new(),
            holes: Vec::new(),
            conflicts: Vec::new(),
            intent,
        };
        write_new(&self.path(id), &record.encode()?)?;
        sync_directory(&self.directory())?;
        FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))?;
        self.summary(&record)
    }

    pub fn status(&self, id: DraftId) -> Result<DraftSummary, Diagnostic> {
        self.summary(&self.read(id)?)
    }

    pub fn append(&self, request: &TransactionRequest) -> Result<DraftMutationReceipt, Diagnostic> {
        request.validate_envelope()?;
        let id = request.draft.ok_or_else(|| {
            draft_error(
                DiagnosticClass::Source,
                "semantic_draft_required",
                "draft append request omits its exact draft identity",
            )
        })?;
        if request.operations.is_empty() {
            return Err(draft_error(
                DiagnosticClass::Source,
                "semantic_draft_empty_append",
                "draft append requires at least one semantic operation",
            ));
        }
        if request.idempotency_key.is_some() {
            return Err(draft_error(
                DiagnosticClass::Source,
                "semantic_draft_idempotency",
                "idempotency keys bind accepted publication, not draft append operations",
            ));
        }
        let lock = self.lock_exclusive()?;
        let mut draft = self.read(id)?;
        self.check_request_binding(&draft, request)?;
        let appended = request.operations.len();
        draft.operations.extend(request.operations.clone());
        draft.preconditions.extend(request.preconditions.clone());
        if draft.operations.len() > MAXIMUM_TRANSACTION_OPERATIONS
            || draft.preconditions.len() > MAXIMUM_TRANSACTION_OPERATIONS
        {
            FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))?;
            return Err(draft_error(
                DiagnosticClass::Resource,
                "semantic_draft_operation_limit",
                "draft append exceeds the transaction operation bound",
            ));
        }
        draft.generation = draft.generation.checked_add(1).ok_or_else(|| {
            draft_error(
                DiagnosticClass::Resource,
                "semantic_draft_generation",
                "draft generation is exhausted",
            )
        })?;
        replace(&self.path(id), &draft.encode()?)?;
        FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))?;
        Ok(DraftMutationReceipt {
            contract_version: DRAFT_CONTRACT_VERSION,
            repository_id: draft.repository_id,
            id,
            status: "draft_updated",
            base_revision: draft.base_revision,
            generation: draft.generation,
            appended_operations: appended,
            total_operations: draft.operations.len(),
        })
    }

    pub fn evaluate(
        &self,
        request: &TransactionRequest,
        mode: TransactionMode,
    ) -> Result<TransactionResult, Diagnostic> {
        if mode == TransactionMode::Apply {
            return Err(draft_error(
                DiagnosticClass::Source,
                "semantic_draft_evaluate_mode",
                "draft evaluation may plan or validate but cannot publish",
            ));
        }
        request.validate_envelope()?;
        let id = request.draft.ok_or_else(|| {
            draft_error(
                DiagnosticClass::Source,
                "semantic_draft_required",
                "draft evaluation request omits its exact draft identity",
            )
        })?;
        let draft = self.read(id)?;
        self.check_request_binding(&draft, request)?;
        let mut combined = draft.transaction_request(request.idempotency_key.clone());
        combined.operations.extend(request.operations.clone());
        combined.preconditions.extend(request.preconditions.clone());
        combined.budget = request.budget;
        combined.intent = request.intent.clone().or(draft.intent);
        execute_transaction(self.repository, &combined, mode)
    }

    pub fn publish(
        &self,
        id: DraftId,
        idempotency_key: Option<String>,
    ) -> Result<TransactionResult, Diagnostic> {
        let draft = self.read(id)?;
        if !draft.holes.is_empty() || !draft.conflicts.is_empty() || draft.operations.is_empty() {
            return Err(draft_error(
                DiagnosticClass::Semantic,
                "semantic_draft_unresolved",
                "draft has no operations or retains unresolved holes or conflicts",
            ));
        }
        let result = execute_transaction(
            self.repository,
            &draft.transaction_request(idempotency_key),
            TransactionMode::Apply,
        )?;
        if matches!(
            result.status,
            TransactionStatus::AcceptedChange
                | TransactionStatus::SemanticNoChange
                | TransactionStatus::Replayed
        ) {
            self.drop(id)?;
        }
        Ok(result)
    }

    pub fn rebase(
        &self,
        id: DraftId,
        new_base: RevisionId,
    ) -> Result<DraftRebaseResult, Diagnostic> {
        let current = self.repository.current_binding()?;
        if new_base != current.head.revision {
            return Err(draft_error(
                DiagnosticClass::Source,
                "semantic_draft_rebase_target",
                "draft rebase target must be the exact current accepted revision",
            ));
        }
        let mut draft = self.read(id)?;
        let original_generation = draft.generation;
        let original_base = draft.base_revision;
        draft.base_revision = new_base;
        let validation = if draft.operations.is_empty() {
            return Err(draft_error(
                DiagnosticClass::Semantic,
                "semantic_draft_empty",
                "an empty draft has no semantic delta to rebase",
            ));
        } else {
            execute_transaction(
                self.repository,
                &draft.transaction_request(None),
                TransactionMode::Validate,
            )?
        };
        let updated = matches!(
            validation.status,
            TransactionStatus::Validated | TransactionStatus::SemanticNoChange
        );
        if updated {
            let lock = self.lock_exclusive()?;
            let observed = self.read(id)?;
            if observed.generation != original_generation || observed.base_revision != original_base
            {
                FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))?;
                return Err(draft_error(
                    DiagnosticClass::Semantic,
                    "semantic_draft_stale_generation",
                    "draft changed concurrently during rebase",
                ));
            }
            draft.generation = draft.generation.checked_add(1).ok_or_else(|| {
                draft_error(
                    DiagnosticClass::Resource,
                    "semantic_draft_generation",
                    "draft generation is exhausted",
                )
            })?;
            replace(&self.path(id), &draft.encode()?)?;
            FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))?;
        }
        let summary_record = if updated { draft } else { self.read(id)? };
        Ok(DraftRebaseResult {
            updated,
            draft: self.summary(&summary_record)?,
            validation,
        })
    }

    pub fn drop(&self, id: DraftId) -> Result<(), Diagnostic> {
        let lock = self.lock_exclusive()?;
        let path = self.path(id);
        read_regular(&path)?;
        fs::remove_file(&path).map_err(|error| draft_io("semantic_draft_drop", &path, error))?;
        sync_directory(&self.directory())?;
        FileExt::unlock(&lock).map_err(|error| self.unlock_error(error))
    }

    fn summary(&self, draft: &DraftRecord) -> Result<DraftSummary, Diagnostic> {
        let current = self.repository.current_binding()?;
        Ok(DraftSummary {
            contract_version: DRAFT_CONTRACT_VERSION,
            repository_id: draft.repository_id,
            id: draft.id,
            base_revision: draft.base_revision,
            current_revision: current.head.revision,
            generation: draft.generation,
            operations: draft.operations.len(),
            preconditions: draft.preconditions.len(),
            holes: draft.holes.len(),
            conflicts: draft.conflicts.len(),
            publishable_shape: !draft.operations.is_empty()
                && draft.holes.is_empty()
                && draft.conflicts.is_empty()
                && draft.base_revision == current.head.revision,
            intent: draft.intent.clone(),
        })
    }

    fn check_request_binding(
        &self,
        draft: &DraftRecord,
        request: &TransactionRequest,
    ) -> Result<(), Diagnostic> {
        if request.repository_id != draft.repository_id
            || request.base_revision != draft.base_revision
        {
            return Err(draft_error(
                DiagnosticClass::Semantic,
                "semantic_draft_base",
                "draft request repository or exact base does not match the persisted draft",
            ));
        }
        Ok(())
    }

    fn read(&self, id: DraftId) -> Result<DraftRecord, Diagnostic> {
        let bytes = read_regular(&self.path(id))?;
        let record = DraftRecord::decode(&bytes)?;
        let repository = self.repository.current_binding()?;
        if record.id != id || record.repository_id != repository.head.repository_id {
            return Err(draft_error(
                DiagnosticClass::Corrupt,
                "semantic_draft_binding",
                "draft physical key or repository identity is inconsistent",
            ));
        }
        self.repository.reconstruct_revision(record.base_revision)?;
        Ok(record)
    }

    fn directory(&self) -> PathBuf {
        self.repository.store_path().join(DRAFT_DIRECTORY)
    }

    fn path(&self, id: DraftId) -> PathBuf {
        self.directory().join(format!("{id}.lkjd"))
    }

    fn lock_exclusive(&self) -> Result<File, Diagnostic> {
        let path = self.repository.store_path().join(REPOSITORY_LOCK);
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .map_err(|error| draft_io("semantic_draft_lock_open", &path, error))?;
        lock.lock_exclusive()
            .map_err(|error| draft_io("semantic_draft_lock", &path, error))?;
        Ok(lock)
    }

    fn unlock_error(&self, error: std::io::Error) -> Diagnostic {
        draft_io(
            "semantic_draft_unlock",
            &self.repository.store_path().join(REPOSITORY_LOCK),
            error,
        )
    }
}

fn read_regular(path: &Path) -> Result<Vec<u8>, Diagnostic> {
    let metadata =
        fs::symlink_metadata(path).map_err(|error| draft_io("semantic_draft_open", path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(draft_error(
            DiagnosticClass::Corrupt,
            "semantic_draft_file_type",
            format!("draft '{}'' is not a regular file", path.display()),
        ));
    }
    let length = usize::try_from(metadata.len()).map_err(|_| draft_size_exhausted())?;
    if length > MAXIMUM_DRAFT_BYTES + 50 {
        return Err(draft_size_exhausted());
    }
    let mut file =
        File::open(path).map_err(|error| draft_io("semantic_draft_open", path, error))?;
    let mut bytes = Vec::with_capacity(length);
    Read::by_ref(&mut file)
        .take((MAXIMUM_DRAFT_BYTES as u64) + 51)
        .read_to_end(&mut bytes)
        .map_err(|error| draft_io("semantic_draft_read", path, error))?;
    if bytes.len() != length {
        return Err(draft_error(
            DiagnosticClass::Corrupt,
            "semantic_draft_changed",
            "draft changed during its bounded read",
        ));
    }
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), Diagnostic> {
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .map_err(|error| draft_io("semantic_draft_create", path, error))?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|error| draft_io("semantic_draft_write", path, error))
}

fn replace(path: &Path, bytes: &[u8]) -> Result<(), Diagnostic> {
    let parent = path.parent().ok_or_else(|| {
        draft_error(
            DiagnosticClass::Infrastructure,
            "semantic_draft_parent",
            "draft path has no parent",
        )
    })?;
    let temporary = parent.join(format!(".draft-stage-{}", RepositoryId::generate()?));
    let result = (|| {
        write_new(&temporary, bytes)?;
        fs::rename(&temporary, path)
            .map_err(|error| draft_io("semantic_draft_publish", path, error))?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn ensure_directory(path: &Path, code: &str) -> Result<(), Diagnostic> {
    let metadata = fs::symlink_metadata(path).map_err(|error| draft_io(code, path, error))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(draft_error(
            DiagnosticClass::Corrupt,
            code,
            format!("'{}' is not a trusted directory", path.display()),
        ));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), Diagnostic> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| draft_io("semantic_draft_sync", path, error))
}

fn draft_count_exhausted() -> Diagnostic {
    draft_error(
        DiagnosticClass::Resource,
        "semantic_draft_count",
        format!("repository exceeds {MAXIMUM_DRAFTS} retained drafts"),
    )
}

fn draft_size_exhausted() -> Diagnostic {
    draft_error(
        DiagnosticClass::Resource,
        "semantic_draft_size",
        format!("draft exceeds {MAXIMUM_DRAFT_BYTES} payload bytes"),
    )
}

fn draft_io(code: &str, path: &Path, error: std::io::Error) -> Diagnostic {
    draft_error(
        DiagnosticClass::Infrastructure,
        code,
        format!("{}: {error}", path.display()),
    )
}

fn draft_error(class: DiagnosticClass, code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(class, code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::{
        GRAPH_CONTRACT_VERSION, InitialPublication, MeaningModule, MigrationIdentityAllocator,
        ModuleObjectRef, PackageId, SemanticDiffDigest, TransactionDigest,
    };
    use crate::platform::{GraphRoot, SourceLimits, parse_module, parse_source};

    struct Fixture {
        _temporary: tempfile::TempDir,
        repository: SemanticRepository,
        declaration: DeclarationId,
    }

    fn fixture() -> Fixture {
        let temporary = tempfile::TempDir::new().expect("temporary draft repository");
        let document = parse_source(
            "fixture.lkj",
            b"(module sample (record Item (name Text)))\n",
            SourceLimits::default(),
        )
        .expect("source oracle");
        let module = parse_module(&document).expect("module oracle");
        let mut allocator = MigrationIdentityAllocator::new(b"draft-fixture".to_vec());
        let meaning = MeaningModule::import(module, &mut allocator).expect("meaning import");
        let declaration = meaning.declarations[0].id;
        let mut root = GraphRoot {
            graph_contract_version: GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(b"draft-fixture", 1),
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
        let mut modules = vec![meaning];
        crate::platform::semantic::canonicalize_graph_package(&mut root, &mut modules, &[])
            .expect("canonical fixture");
        let (repository, _) = SemanticRepository::initialize(
            temporary.path(),
            InitialPublication {
                root,
                modules,
                transaction: TransactionDigest::of(b"draft fixture import"),
                semantic_diff: SemanticDiffDigest::of(b"draft fixture initial"),
                intent: None,
                validation_profile: None,
                dependency_artifacts: Vec::new(),
                status: crate::platform::ReceiptStatus::ImportAccepted,
            },
        )
        .expect("repository");
        Fixture {
            _temporary: temporary,
            repository,
            declaration,
        }
    }

    fn append_request(fixture: &Fixture, draft: &DraftSummary) -> TransactionRequest {
        TransactionRequest {
            contract_version: super::super::semantic_transaction::TRANSACTION_CONTRACT_VERSION,
            graph_contract: GRAPH_CONTRACT_IDENTITY.to_owned(),
            repository_id: draft.repository_id,
            base_revision: draft.base_revision,
            draft: Some(draft.id),
            idempotency_key: None,
            preconditions: Vec::new(),
            operations: vec![SemanticOperation::RenameDeclaration {
                declaration: fixture.declaration,
                new_name: "Entry".to_owned(),
            }],
            budget: TransactionBudget::default(),
            intent: None,
        }
    }

    #[test]
    fn draft_append_validate_and_publish_are_separate_from_accepted_authority() {
        let fixture = fixture();
        let base = fixture.repository.current().expect("base").head.revision;
        let store = SemanticDraftStore::new(&fixture.repository);
        let draft = store
            .create(Some(base), Some("rename retained owner".to_owned()))
            .expect("create");
        assert_eq!(store.count().expect("count"), 1);
        let receipt = store
            .append(&append_request(&fixture, &draft))
            .expect("append");
        assert_eq!(receipt.total_operations, 1);
        assert_eq!(
            fixture
                .repository
                .current()
                .expect("unchanged")
                .head
                .revision,
            base
        );

        let validation = execute_transaction(
            &fixture.repository,
            &store
                .read(draft.id)
                .expect("draft")
                .transaction_request(None),
            TransactionMode::Validate,
        )
        .expect("validate");
        assert_eq!(validation.status, TransactionStatus::Validated);
        assert_eq!(
            fixture
                .repository
                .current()
                .expect("still unchanged")
                .head
                .revision,
            base
        );

        let published = store
            .publish(draft.id, Some("publish-draft".to_owned()))
            .expect("publish");
        assert_eq!(published.status, TransactionStatus::AcceptedChange);
        assert_eq!(store.count().expect("empty draft store"), 0);
        let current = fixture.repository.reconstruct_current().expect("current");
        assert_eq!(current.modules[0].declarations[0].id, fixture.declaration);
        assert_eq!(current.modules[0].declarations[0].name, "Entry");
    }

    #[test]
    fn stale_draft_stays_non_executable_until_explicit_rebase() {
        let fixture = fixture();
        let base = fixture.repository.current().expect("base").head.revision;
        let store = SemanticDraftStore::new(&fixture.repository);
        let draft = store.create(Some(base), None).expect("create");
        store
            .append(&append_request(&fixture, &draft))
            .expect("append");
        let direct = TransactionRequest {
            contract_version: super::super::semantic_transaction::TRANSACTION_CONTRACT_VERSION,
            graph_contract: GRAPH_CONTRACT_IDENTITY.to_owned(),
            repository_id: draft.repository_id,
            base_revision: base,
            draft: None,
            idempotency_key: None,
            preconditions: Vec::new(),
            operations: vec![SemanticOperation::SetPackageMetadata {
                name: "fixture-next".to_owned(),
            }],
            budget: TransactionBudget::default(),
            intent: None,
        };
        let changed = execute_transaction(&fixture.repository, &direct, TransactionMode::Apply)
            .expect("advance accepted authority");
        let current = changed.published_revision.expect("current");
        let stale = store.publish(draft.id, None).expect("stale publication");
        assert_eq!(stale.status, TransactionStatus::StaleBase);
        assert_eq!(store.count().expect("draft retained"), 1);
        let rebased = store.rebase(draft.id, current).expect("rebase");
        assert!(rebased.updated);
        assert_eq!(rebased.draft.base_revision, current);
        assert_eq!(
            store
                .publish(draft.id, None)
                .expect("publish rebased")
                .status,
            TransactionStatus::AcceptedChange
        );
    }

    #[test]
    fn backup_and_restore_preserve_non_executable_draft_authority() {
        let fixture = fixture();
        let base = fixture.repository.current().expect("base").head.revision;
        let store = SemanticDraftStore::new(&fixture.repository);
        let draft = store
            .create(Some(base), Some("retained draft".to_owned()))
            .expect("create draft");
        store
            .append(&append_request(&fixture, &draft))
            .expect("append draft");
        let backup_root = tempfile::TempDir::new().expect("backup parent");
        let backup = backup_root.path().join("drafts.lkjb");
        let backup_receipt = fixture
            .repository
            .backup_to(&backup)
            .expect("backup with draft");
        assert_eq!(backup_receipt.drafts, 1);

        let destination = tempfile::TempDir::new().expect("restore destination");
        let (restored, restore_receipt) =
            SemanticRepository::restore_backup_from(destination.path(), &backup)
                .expect("restore with draft");
        assert_eq!(restore_receipt.drafts, 1);
        assert_eq!(
            restored.current().expect("restored accepted").head.revision,
            base
        );
        let restored_store = SemanticDraftStore::new(&restored);
        let status = restored_store.status(draft.id).expect("restored draft");
        assert_eq!(status.operations, 1);
        assert_eq!(status.base_revision, base);
        assert_eq!(
            restored_store
                .publish(draft.id, None)
                .expect("publish restored draft")
                .status,
            TransactionStatus::AcceptedChange
        );
    }
}
