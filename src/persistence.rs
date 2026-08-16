use crate::artifact;
use crate::codec::{Reader, Writer};
use crate::diff;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, Workspace};
use crate::ids::{
    ChangeDigest, DraftSymbol, IdempotencyKey, NodeId, Revision, SnapshotHash, WorkspaceId,
};
use crate::machine;
use crate::query;
use crate::transaction::{ApplyTransactionRequest, TransactionMode, TransactionReceipt};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const HEAD_MAGIC: [u8; 8] = *b"LKJHEAD4";
pub const MAXIMUM_HEAD_BYTES: usize = 16 * 1024;
static TEMP_SERIAL: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Eq, PartialEq)]
pub(crate) struct IdempotencyRecord {
    pub key: IdempotencyKey,
    pub fingerprint: [u8; 32],
    pub receipt: TransactionReceipt,
}

struct PreparedPublication {
    artifact_bytes: Vec<u8>,
    head_bytes: Vec<u8>,
}

pub(crate) struct DurableWorkspace {
    directory: PathBuf,
    workspace: Workspace,
    idempotency: Option<IdempotencyRecord>,
}

impl DurableWorkspace {
    #[cfg(test)]
    pub(crate) fn create(state_directory: &Path, id: WorkspaceId) -> Result<Self> {
        Self::create_preflighted(state_directory, id, |_| Ok(())).map(|(workspace, ())| workspace)
    }

    pub(crate) fn create_preflighted<T>(
        state_directory: &Path,
        id: WorkspaceId,
        preflight: impl FnOnce(&Snapshot) -> Result<T>,
    ) -> Result<(Self, T)> {
        let final_directory = workspace_directory(state_directory, id);
        match fs::symlink_metadata(&final_directory) {
            Ok(_) => {
                return Err(LkError::new(
                    ErrorCode::WorkspaceExists,
                    "workspace state directory already exists",
                )
                .for_workspace(id));
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let workspaces_directory = state_directory.join("workspaces");
        let workspace = Workspace::new(id)?;
        let preflighted = preflight(workspace.head()?)?;
        let serial = TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
        let staging_directory =
            workspaces_directory.join(format!(".creating-{}-{}-{serial}", id, std::process::id()));
        create_private_directory(&staging_directory)?;
        if let Err(error) = create_private_directory(&staging_directory.join("revisions")) {
            let _ = fs::remove_dir_all(&staging_directory);
            return Err(error);
        }
        let mut durable = Self {
            directory: staging_directory.clone(),
            workspace,
            idempotency: None,
        };
        if let Err(error) =
            durable.publish_snapshot(durable.workspace.head()?, None, PublicationStep::None)
        {
            let _ = fs::remove_dir_all(&staging_directory);
            return Err(error);
        }
        if let Err(error) = fs::rename(&staging_directory, &final_directory) {
            let _ = fs::remove_dir_all(&staging_directory);
            return Err(error.into());
        }
        durable.directory = final_directory;
        if let Err(error) = sync_directory(&workspaces_directory) {
            let rollback = fs::remove_dir_all(&durable.directory)
                .map_err(LkError::from)
                .and_then(|()| sync_directory(&workspaces_directory));
            if let Err(rollback_error) = rollback {
                return Err(LkError::new(
                    ErrorCode::CommitOutcomeUnknown,
                    format!(
                        "workspace creation sync failed and rollback also failed: publication={error}; rollback={rollback_error}"
                    ),
                )
                .for_workspace(id));
            }
            return Err(error);
        }
        Ok((durable, preflighted))
    }

    pub(crate) fn open(state_directory: &Path, id: WorkspaceId) -> Result<Self> {
        let directory = workspace_directory(state_directory, id);
        reject_symlink(&directory)?;
        cleanup_workspace_temporary_files(&directory)?;
        let head_path = directory.join("HEAD");
        let head_metadata = fs::symlink_metadata(&head_path).map_err(|error| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!("cannot inspect workspace HEAD: {error}"),
            )
            .for_workspace(id)
        })?;
        if head_metadata.file_type().is_symlink() || !head_metadata.is_file() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace HEAD is not a regular file",
            )
            .for_workspace(id));
        }
        if head_metadata.len() > u64::try_from(MAXIMUM_HEAD_BYTES).unwrap_or(u64::MAX) {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "workspace HEAD exceeds decoder byte policy",
            )
            .for_workspace(id));
        }
        let head_bytes = read_bounded_regular_file(
            &head_path,
            MAXIMUM_HEAD_BYTES,
            "workspace HEAD exceeds decoder byte policy",
        )
        .map_err(|mut error| {
            error.workspace = Some(id);
            error
        })?;
        let (head_revision, head_hash, idempotency) = decode_head(&head_bytes)?;
        let revisions_directory = directory.join("revisions");
        reject_symlink(&revisions_directory)?;
        let mut snapshots = BTreeMap::new();
        let mut removed_orphans = false;
        for entry in fs::read_dir(&revisions_directory)? {
            let entry = entry?;
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace revisions directory contains a symlink",
                )
                .for_workspace(id));
            }
            if !file_type.is_file() {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace revisions directory contains a non-file entry",
                )
                .for_workspace(id));
            }
            let file_name = entry.file_name();
            let Some(file_name) = file_name.to_str() else {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace revision has a non-UTF-8 file name",
                )
                .for_workspace(id));
            };
            if is_temporary_file_name(file_name) {
                fs::remove_file(entry.path())?;
                removed_orphans = true;
                continue;
            }
            let Some(stem) = file_name.strip_suffix(".lkjscript") else {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace revisions directory contains an unknown file",
                )
                .for_workspace(id));
            };
            let revision_value = stem.parse::<u64>().map_err(|_| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace revision file name is invalid",
                )
                .for_workspace(id)
            })?;
            let revision = Revision::new(revision_value);
            if file_name != revision_file_name(revision) {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace revision file name is not canonical",
                )
                .for_workspace(id)
                .at_revision(revision));
            }
            if revision > head_revision {
                fs::remove_file(entry.path())?;
                removed_orphans = true;
                continue;
            }
            let maximum_artifact_bytes =
                u64::try_from(artifact::DecodePolicy::default().maximum_artifact_bytes)
                    .unwrap_or(u64::MAX);
            if entry.metadata()?.len() > maximum_artifact_bytes {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "revision artifact exceeds decoder byte policy",
                )
                .for_workspace(id)
                .at_revision(revision));
            }
            let bytes = read_bounded_regular_file(
                &entry.path(),
                artifact::DecodePolicy::default().maximum_artifact_bytes,
                "revision artifact exceeds decoder byte policy",
            )?;
            let snapshot = artifact::decode(&bytes)?;
            if snapshot.workspace() != id || snapshot.revision() != revision {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "revision artifact identity disagrees with its durable path",
                )
                .for_workspace(id)
                .at_revision(revision));
            }
            if snapshots.insert(revision, Arc::new(snapshot)).is_some() {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace contains duplicate revision artifacts",
                )
                .for_workspace(id)
                .at_revision(revision));
            }
        }
        if removed_orphans {
            sync_directory(&revisions_directory)?;
        }
        let expected_count = head_revision.get().checked_add(1).ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace head revision cannot form a retained history length",
            )
            .for_workspace(id)
        })?;
        let actual_count = u64::try_from(snapshots.len()).map_err(|_| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "retained snapshot count overflows canonical representation",
            )
            .for_workspace(id)
        })?;
        if actual_count != expected_count {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace retained revision history has a gap",
            )
            .for_workspace(id)
            .at_revision(head_revision));
        }
        let head_snapshot = snapshots.get(&head_revision).ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace HEAD names a missing revision artifact",
            )
            .for_workspace(id)
            .at_revision(head_revision)
        })?;
        if head_snapshot.hash() != head_hash {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace HEAD hash disagrees with its revision artifact",
            )
            .for_workspace(id)
            .at_revision(head_revision));
        }
        if let Some(record) = &idempotency {
            validate_idempotency_record(record, id, head_revision, &snapshots)?;
        }
        let workspace = Workspace::from_snapshots(id, head_revision, snapshots)?;
        Ok(Self {
            directory,
            workspace,
            idempotency,
        })
    }

    pub(crate) const fn id(&self) -> WorkspaceId {
        self.workspace.id()
    }

    pub(crate) fn snapshot(&self, revision: Revision) -> Result<&Arc<Snapshot>> {
        self.workspace.snapshot(revision)
    }

    #[cfg(test)]
    pub(crate) fn head(&self) -> Result<&Arc<Snapshot>> {
        self.workspace.head()
    }

    #[cfg(test)]
    pub(crate) fn apply(
        &mut self,
        request: &ApplyTransactionRequest,
        fingerprint: [u8; 32],
    ) -> Result<TransactionReceipt> {
        self.apply_at_step(
            request,
            fingerprint,
            crate::ids::RequestId::new(1),
            PublicationStep::None,
        )
        .map(|(receipt, _)| receipt)
    }

    pub(crate) fn apply_with_response(
        &mut self,
        request: &ApplyTransactionRequest,
        fingerprint: [u8; 32],
        request_id: crate::ids::RequestId,
    ) -> Result<(TransactionReceipt, Vec<u8>)> {
        self.apply_at_step(request, fingerprint, request_id, PublicationStep::None)
    }

    fn apply_at_step(
        &mut self,
        request: &ApplyTransactionRequest,
        fingerprint: [u8; 32],
        request_id: crate::ids::RequestId,
        fault: PublicationStep,
    ) -> Result<(TransactionReceipt, Vec<u8>)> {
        self.verify_live_head()?;
        let transaction = &request.transaction;
        if transaction.workspace != self.id() {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "transaction names a different workspace",
            )
            .for_workspace(self.id()));
        }
        if transaction.mode == TransactionMode::ValidateOnly
            && transaction.idempotency_key.is_some()
        {
            return Err(LkError::new(
                ErrorCode::InvalidOperand,
                "validate-only transactions cannot carry an idempotency key",
            )
            .for_workspace(self.id()));
        }
        if let (Some(key), Some(record)) = (transaction.idempotency_key, &self.idempotency)
            && key == record.key
        {
            if fingerprint == record.fingerprint
                && record.receipt.base_revision == transaction.base_revision
            {
                let receipt = record.receipt.clone();
                let response_bytes = preflight_receipt_response(request_id, &receipt)?;
                return Ok((receipt, response_bytes));
            }
            return Err(LkError::new(
                ErrorCode::IdempotencyConflict,
                "idempotency key was already used for a different transaction",
            )
            .for_workspace(self.id()));
        }
        let prepared = self.workspace.prepare_transaction(request)?;
        let response_bytes = preflight_receipt_response(request_id, &prepared.receipt)?;
        if transaction.mode == TransactionMode::ValidateOnly {
            self.preflight_publication(&prepared.snapshot, self.idempotency.as_ref())?;
            return Ok((prepared.receipt, response_bytes));
        }
        let next_idempotency = transaction.idempotency_key.map(|key| IdempotencyRecord {
            key,
            fingerprint,
            receipt: prepared.receipt.clone(),
        });
        let retained_idempotency = next_idempotency
            .clone()
            .or_else(|| self.idempotency.clone());
        let publication =
            self.preflight_publication(&prepared.snapshot, retained_idempotency.as_ref())?;
        self.publish_preflighted(&prepared.snapshot, &publication, fault)?;
        self.workspace.publish(prepared.snapshot)?;
        self.idempotency = retained_idempotency;
        Ok((prepared.receipt, response_bytes))
    }

    fn verify_live_head(&self) -> Result<()> {
        let bytes =
            read_head_before_publication(&self.directory, Revision::new(1))?.ok_or_else(|| {
                LkError::new(ErrorCode::ArtifactCorrupt, "live workspace HEAD is missing")
            })?;
        let (revision, hash, idempotency) = decode_head(&bytes).map_err(|mut error| {
            error.workspace = Some(self.id());
            error
        })?;
        let expected = self.workspace.head()?;
        if revision != expected.revision()
            || hash != expected.hash()
            || idempotency != self.idempotency
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "live workspace HEAD changed after daemon authority was established",
            )
            .for_workspace(self.id())
            .at_revision(revision));
        }
        Ok(())
    }

    fn preflight_publication(
        &self,
        snapshot: &Snapshot,
        idempotency: Option<&IdempotencyRecord>,
    ) -> Result<PreparedPublication> {
        let artifact_bytes = artifact::encode(snapshot)?;
        if artifact_bytes.len() > artifact::DecodePolicy::default().maximum_artifact_bytes {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "snapshot artifact exceeds daemon persistence policy",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision()));
        }
        let decoded = artifact::decode(&artifact_bytes)?;
        if decoded != *snapshot {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "encoded snapshot failed persistence round-trip preflight",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision()));
        }
        let head_bytes = encode_head(snapshot.revision(), snapshot.hash(), idempotency)?;
        Ok(PreparedPublication {
            artifact_bytes,
            head_bytes,
        })
    }

    fn publish_snapshot(
        &self,
        snapshot: &Snapshot,
        idempotency: Option<&IdempotencyRecord>,
        fault: PublicationStep,
    ) -> Result<()> {
        let publication = self.preflight_publication(snapshot, idempotency)?;
        self.publish_preflighted(snapshot, &publication, fault)
    }

    fn publish_preflighted(
        &self,
        snapshot: &Snapshot,
        publication: &PreparedPublication,
        fault: PublicationStep,
    ) -> Result<()> {
        let bytes = &publication.artifact_bytes;
        let head_bytes = &publication.head_bytes;
        let revisions = self.directory.join("revisions");
        let revision_path = revision_path(&revisions, snapshot.revision());
        let old_head = read_head_before_publication(&self.directory, snapshot.revision())?;
        if old_head.is_some() {
            self.verify_live_head()?;
        } else if self.directory.join("HEAD").exists() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "live workspace HEAD disappeared during publication preflight",
            )
            .for_workspace(snapshot.workspace()));
        }
        let revision_existed = match fs::symlink_metadata(&revision_path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "immutable revision path is not a regular file",
                )
                .for_workspace(snapshot.workspace())
                .at_revision(snapshot.revision()));
            }
            Ok(metadata) => {
                if metadata.len() > u64::try_from(bytes.len()).unwrap_or(u64::MAX) {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "immutable revision path length disagrees with canonical bytes",
                    )
                    .for_workspace(snapshot.workspace())
                    .at_revision(snapshot.revision()));
                }
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if revision_existed {
            if read_bounded_regular_file(
                &revision_path,
                bytes.len(),
                "immutable revision path exceeds canonical byte length",
            )? != bytes.as_slice()
            {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "immutable revision path already contains different bytes",
                )
                .for_workspace(snapshot.workspace())
                .at_revision(snapshot.revision()));
            }
        } else {
            inject(fault, PublicationStep::BeforeRevisionWrite)?;
            let temporary = write_temporary(&revisions, bytes)?;
            if let Err(error) = inject(fault, PublicationStep::AfterRevisionSync) {
                fs::remove_file(&temporary)?;
                sync_directory(&revisions)?;
                return Err(error);
            }
            if let Err(error) = fs::rename(&temporary, &revision_path) {
                fs::remove_file(&temporary)?;
                sync_directory(&revisions)?;
                return Err(error.into());
            }
            if let Err(error) = sync_directory(&revisions) {
                cleanup_revision(&revision_path, &revisions)?;
                return Err(error);
            }
            if let Err(error) = inject(fault, PublicationStep::AfterRevisionRename) {
                cleanup_revision(&revision_path, &revisions)?;
                return Err(error);
            }
        }

        let head_temporary = match write_temporary(&self.directory, head_bytes) {
            Ok(path) => path,
            Err(error) => {
                if !revision_existed {
                    cleanup_revision(&revision_path, &revisions)?;
                }
                return Err(error);
            }
        };
        if let Err(error) = inject(fault, PublicationStep::AfterHeadSync) {
            fs::remove_file(&head_temporary)?;
            sync_directory(&self.directory)?;
            if !revision_existed {
                cleanup_revision(&revision_path, &revisions)?;
            }
            return Err(error);
        }
        let head_path = self.directory.join("HEAD");
        if let Err(error) = fs::rename(&head_temporary, &head_path) {
            fs::remove_file(&head_temporary)?;
            sync_directory(&self.directory)?;
            if !revision_existed {
                cleanup_revision(&revision_path, &revisions)?;
            }
            return Err(error.into());
        }
        if let Err(error) = inject(fault, PublicationStep::AfterHeadRename)
            .and_then(|_| sync_directory(&self.directory))
        {
            if let Err(restore_error) = restore_head(&self.directory, old_head.as_deref()) {
                return Err(LkError::new(
                    ErrorCode::CommitOutcomeUnknown,
                    format!(
                        "workspace HEAD publication failed and rollback also failed: publication={error}; rollback={restore_error}"
                    ),
                )
                .for_workspace(snapshot.workspace())
                .at_revision(snapshot.revision()));
            }
            if !revision_existed {
                cleanup_revision(&revision_path, &revisions)?;
            }
            return Err(error);
        }
        Ok(())
    }

    #[cfg(test)]
    fn apply_with_fault(
        &mut self,
        request: &ApplyTransactionRequest,
        fingerprint: [u8; 32],
        fault: PublicationStep,
    ) -> Result<TransactionReceipt> {
        self.apply_at_step(request, fingerprint, crate::ids::RequestId::new(1), fault)
            .map(|(receipt, _)| receipt)
    }
}

fn preflight_receipt_response(
    request_id: crate::ids::RequestId,
    receipt: &TransactionReceipt,
) -> Result<Vec<u8>> {
    machine::encode_response(
        request_id,
        &crate::protocol::Response::TransactionReceipt(receipt.clone()),
        false,
    )
    .map_err(|error| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            format!("transaction response exceeds JSON boundary policy: {error}"),
        )
    })
}

fn validate_idempotency_record(
    record: &IdempotencyRecord,
    workspace: WorkspaceId,
    head: Revision,
    snapshots: &BTreeMap<Revision, Arc<Snapshot>>,
) -> Result<()> {
    let receipt = &record.receipt;
    let expected_revision = receipt.base_revision.next();
    if !receipt.published
        || receipt.workspace != workspace
        || receipt.revision > head
        || expected_revision != Some(receipt.revision)
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency receipt has invalid publication identity",
        )
        .for_workspace(workspace));
    }
    let base = snapshots.get(&receipt.base_revision).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency base revision is not retained",
        )
        .for_workspace(workspace)
        .at_revision(receipt.base_revision)
    })?;
    let published = snapshots.get(&receipt.revision).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency result revision is not retained",
        )
        .for_workspace(workspace)
        .at_revision(receipt.revision)
    })?;
    let semantic_diff = diff::between(base, published);
    let blockers_before = query::workspace_blockers(base);
    let blockers_after = query::workspace_blockers(published);
    if published.hash() != receipt.hash
        || semantic_diff.change_count() != receipt.change_count
        || semantic_diff.digest != receipt.change_digest
        || blockers_before.is_empty() != receipt.complete_before
        || blockers_after.is_empty() != receipt.complete_after
        || u64::try_from(blockers_before.len()).ok() != Some(receipt.blocker_count_before)
        || u64::try_from(blockers_after.len()).ok() != Some(receipt.blocker_count_after)
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency receipt disagrees with retained snapshots",
        )
        .for_workspace(workspace)
        .at_revision(receipt.revision));
    }
    let expected_created = published
        .next_serial()
        .checked_sub(base.next_serial())
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "persisted idempotency allocator transition moved backward",
            )
            .for_workspace(workspace)
        })?;
    if receipt.created_count != expected_created {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency created count disagrees with allocator transition",
        )
        .for_workspace(workspace));
    }
    if receipt.returned_bindings.len() > crate::transaction::MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency bindings exceed response policy",
        )
        .for_workspace(workspace));
    }
    let mut selected_symbols = std::collections::BTreeSet::new();
    let mut selected_serials = std::collections::BTreeSet::new();
    for (symbol, node) in &receipt.returned_bindings {
        if !selected_symbols.insert(*symbol)
            || !selected_serials.insert(node.serial())
            || node.workspace() != workspace
            || node.serial() < base.next_serial()
            || node.serial() >= published.next_serial()
            || (published.node(*node).is_err() && !published.contains_tombstone(node.serial()))
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "persisted idempotency selected binding is invalid",
            )
            .for_workspace(workspace)
            .for_node(*node));
        }
    }
    Ok(())
}

pub(crate) fn list_workspace_ids(state_directory: &Path) -> Result<Vec<WorkspaceId>> {
    let directory = state_directory.join("workspaces");
    ensure_private_directory(&directory)?;
    let mut ids = BTreeSet::new();
    let mut removed_staging = false;
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspaces directory contains a symlink",
            ));
        }
        if !file_type.is_dir() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspaces directory contains an unexpected file",
            ));
        }
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace directory has a non-UTF-8 identity",
            )
        })?;
        if is_workspace_staging_name(name) {
            fs::remove_dir_all(entry.path())?;
            removed_staging = true;
            continue;
        }
        let id = name.parse::<WorkspaceId>().map_err(|error| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!("workspace directory identity is invalid: {error}"),
            )
        })?;
        if name != id.to_string() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace directory identity is not canonical",
            ));
        }
        if !ids.insert(id) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace directory identities are ambiguous",
            ));
        }
    }
    if removed_staging {
        sync_directory(&directory)?;
    }
    Ok(ids.into_iter().collect())
}

fn cleanup_workspace_temporary_files(directory: &Path) -> Result<()> {
    let mut removed = false;
    for entry in fs::read_dir(directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace directory has a non-UTF-8 entry",
            )
        })?;
        if (name == "HEAD" && file_type.is_file()) || (name == "revisions" && file_type.is_dir()) {
            continue;
        }
        if is_temporary_file_name(name) && file_type.is_file() {
            fs::remove_file(entry.path())?;
            removed = true;
            continue;
        }
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace directory contains an unknown or invalid entry",
        ));
    }
    if removed {
        sync_directory(directory)?;
    }
    Ok(())
}

fn is_temporary_file_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(".tmp-") else {
        return false;
    };
    let mut parts = rest.split('-');
    let process = parts.next().and_then(|value| value.parse::<u32>().ok());
    let serial = parts.next().and_then(|value| value.parse::<u64>().ok());
    process.is_some() && serial.is_some() && parts.next().is_none()
}

fn is_workspace_staging_name(name: &str) -> bool {
    let Some(rest) = name.strip_prefix(".creating-") else {
        return false;
    };
    let mut parts = rest.split('-');
    let workspace = parts
        .next()
        .and_then(|value| value.parse::<WorkspaceId>().ok());
    let process = parts.next().and_then(|value| value.parse::<u32>().ok());
    let serial = parts.next().and_then(|value| value.parse::<u64>().ok());
    workspace.is_some() && process.is_some() && serial.is_some() && parts.next().is_none()
}

pub(crate) fn ensure_state_directory(path: &Path) -> Result<()> {
    if !path.is_absolute() {
        return Err(LkError::new(
            ErrorCode::Io,
            "daemon state directory must be an explicit absolute path",
        ));
    }
    reject_existing_symlink_components(path)?;
    ensure_private_directory(path)?;
    ensure_private_directory(&path.join("workspaces"))?;
    sync_directory(path)?;
    if let Some(parent) = path.parent() {
        sync_directory(parent)?;
    }
    Ok(())
}

pub(crate) fn workspace_directory(state_directory: &Path, id: WorkspaceId) -> PathBuf {
    state_directory.join("workspaces").join(id.to_hex())
}

fn read_head_before_publication(
    directory: &Path,
    candidate_revision: Revision,
) -> Result<Option<Vec<u8>>> {
    let path = directory.join("HEAD");
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace HEAD is not a regular file",
            )
            .at_revision(candidate_revision))
        }
        Ok(metadata) => {
            if metadata.len() > u64::try_from(MAXIMUM_HEAD_BYTES).unwrap_or(u64::MAX) {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "workspace HEAD exceeds decoder byte policy",
                )
                .at_revision(candidate_revision));
            }
            Ok(Some(read_bounded_regular_file(
                &path,
                MAXIMUM_HEAD_BYTES,
                "workspace HEAD exceeds decoder byte policy",
            )?))
        }
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound
                && candidate_revision == Revision::INITIAL =>
        {
            Ok(None)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "existing workspace lost its durable HEAD",
        )
        .at_revision(candidate_revision)),
        Err(error) => Err(error.into()),
    }
}

fn put_transaction_receipt(writer: &mut Writer, receipt: &TransactionReceipt) -> Result<()> {
    if receipt.returned_bindings.len() > crate::transaction::MAX_RETURNED_BINDINGS {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "transaction receipt bindings exceed response policy",
        ));
    }
    writer.fixed(&receipt.workspace.as_bytes());
    writer.u64(receipt.base_revision.get());
    writer.u64(receipt.revision.get());
    writer.fixed(&receipt.hash.as_bytes());
    writer.bool(receipt.published);
    writer.u64(receipt.created_count);
    writer.u64(u64::try_from(receipt.returned_bindings.len()).map_err(|_| {
        LkError::new(
            ErrorCode::PolicyExceeded,
            "transaction receipt binding count exceeds HEAD4 encoding",
        )
    })?);
    for (symbol, node) in &receipt.returned_bindings {
        writer.string(&symbol.to_string()).map_err(head_codec)?;
        writer.fixed(&node.workspace().as_bytes());
        writer.u64(node.serial());
    }
    writer.u64(receipt.change_count);
    writer.fixed(&receipt.change_digest.as_bytes());
    writer.bool(receipt.complete_before);
    writer.bool(receipt.complete_after);
    writer.u64(receipt.blocker_count_before);
    writer.u64(receipt.blocker_count_after);
    Ok(())
}

fn read_transaction_receipt(reader: &mut Reader<'_>) -> Result<TransactionReceipt> {
    let mut workspace = [0_u8; WorkspaceId::BYTE_LEN];
    workspace.copy_from_slice(reader.fixed(WorkspaceId::BYTE_LEN).map_err(head_codec)?);
    let workspace = WorkspaceId::from_bytes(workspace);
    let base_revision = Revision::new(reader.u64().map_err(head_codec)?);
    let revision = Revision::new(reader.u64().map_err(head_codec)?);
    let mut hash = [0_u8; SnapshotHash::BYTE_LEN];
    hash.copy_from_slice(reader.fixed(SnapshotHash::BYTE_LEN).map_err(head_codec)?);
    let published = reader.bool().map_err(head_codec)?;
    let created_count = reader.u64().map_err(head_codec)?;
    let count = reader
        .count(crate::transaction::MAX_RETURNED_BINDINGS)
        .map_err(head_codec)?;
    let mut returned_bindings = Vec::with_capacity(count);
    for _ in 0..count {
        let symbol_text = reader
            .string(crate::ids::MAX_DRAFT_SYMBOL_BYTES)
            .map_err(head_codec)?;
        let symbol = DraftSymbol::parse(&symbol_text).map_err(|message| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                format!("workspace HEAD contains an invalid draft symbol: {message}"),
            )
        })?;
        let mut node_workspace = [0_u8; WorkspaceId::BYTE_LEN];
        node_workspace.copy_from_slice(reader.fixed(WorkspaceId::BYTE_LEN).map_err(head_codec)?);
        let serial = reader.u64().map_err(head_codec)?;
        let node =
            NodeId::new(WorkspaceId::from_bytes(node_workspace), serial).map_err(|error| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    format!("workspace HEAD contains an invalid receipt node identity: {error}"),
                )
            })?;
        returned_bindings.push((symbol, node));
    }
    let change_count = reader.u64().map_err(head_codec)?;
    let mut change_digest = [0_u8; ChangeDigest::BYTE_LEN];
    change_digest.copy_from_slice(reader.fixed(ChangeDigest::BYTE_LEN).map_err(head_codec)?);
    let complete_before = reader.bool().map_err(head_codec)?;
    let complete_after = reader.bool().map_err(head_codec)?;
    let blocker_count_before = reader.u64().map_err(head_codec)?;
    let blocker_count_after = reader.u64().map_err(head_codec)?;
    Ok(TransactionReceipt {
        workspace,
        base_revision,
        revision,
        hash: SnapshotHash::from_bytes(hash),
        published,
        created_count,
        returned_bindings,
        change_count,
        change_digest: ChangeDigest::from_bytes(change_digest),
        complete_before,
        complete_after,
        blocker_count_before,
        blocker_count_after,
    })
}

fn encode_head(
    revision: Revision,
    hash: SnapshotHash,
    idempotency: Option<&IdempotencyRecord>,
) -> Result<Vec<u8>> {
    let mut writer = Writer::new();
    writer.fixed(&HEAD_MAGIC);
    writer.u64(revision.get());
    writer.fixed(&hash.as_bytes());
    writer.bool(idempotency.is_some());
    if let Some(record) = idempotency {
        writer.fixed(&record.key.as_bytes());
        writer.fixed(&record.fingerprint);
        put_transaction_receipt(&mut writer, &record.receipt)?;
    }
    let body = writer.finish();
    let checksum = blake3::hash(&body);
    let mut bytes = Vec::with_capacity(body.len() + checksum.as_bytes().len());
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(checksum.as_bytes());
    if bytes.len() > MAXIMUM_HEAD_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "workspace HEAD exceeds encoder byte policy",
        ));
    }
    Ok(bytes)
}

fn decode_head(bytes: &[u8]) -> Result<(Revision, SnapshotHash, Option<IdempotencyRecord>)> {
    if bytes.len() < SnapshotHash::BYTE_LEN {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace HEAD is truncated before its checksum",
        ));
    }
    let body_length = bytes.len() - SnapshotHash::BYTE_LEN;
    let (body, encoded_checksum) = bytes.split_at(body_length);
    if blake3::hash(body).as_bytes() != encoded_checksum {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace HEAD checksum is invalid",
        ));
    }
    let mut reader = Reader::new(body);
    if reader.fixed(HEAD_MAGIC.len()).map_err(head_codec)? != HEAD_MAGIC {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace HEAD magic is invalid",
        ));
    }
    let revision = Revision::new(reader.u64().map_err(head_codec)?);
    let mut hash = [0_u8; SnapshotHash::BYTE_LEN];
    hash.copy_from_slice(reader.fixed(SnapshotHash::BYTE_LEN).map_err(head_codec)?);
    let hash = SnapshotHash::from_bytes(hash);
    let idempotency = if reader.bool().map_err(head_codec)? {
        let mut key = [0_u8; 16];
        key.copy_from_slice(reader.fixed(16).map_err(head_codec)?);
        let mut fingerprint = [0_u8; 32];
        fingerprint.copy_from_slice(reader.fixed(32).map_err(head_codec)?);
        let receipt = read_transaction_receipt(&mut reader)?;
        Some(IdempotencyRecord {
            key: IdempotencyKey::from_bytes(key),
            fingerprint,
            receipt,
        })
    } else {
        None
    };
    reader.finish().map_err(head_codec)?;
    Ok((revision, hash, idempotency))
}

fn reject_existing_symlink_components(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "daemon state path contains a symbolic-link component",
                ));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => break,
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn create_private_directory(path: &Path) -> Result<()> {
    fs::create_dir(path)?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn ensure_private_directory(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) => {
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "daemon state path is not a real directory",
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir(path)?;
        }
        Err(error) => return Err(error.into()),
    }
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

fn reject_symlink(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(LkError::new(
            ErrorCode::Io,
            "durable workspace path is not a real directory",
        ));
    }
    Ok(())
}

fn revision_file_name(revision: Revision) -> String {
    format!("{:020}.lkjscript", revision.get())
}

fn revision_path(directory: &Path, revision: Revision) -> PathBuf {
    directory.join(revision_file_name(revision))
}

fn read_bounded_regular_file(path: &Path, maximum: usize, policy_message: &str) -> Result<Vec<u8>> {
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    if !metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "durable path is not a regular file",
        ));
    }
    if metadata.len() > u64::try_from(maximum).unwrap_or(u64::MAX) {
        return Err(LkError::new(ErrorCode::PolicyExceeded, policy_message));
    }
    let read_limit = u64::try_from(maximum).unwrap_or(u64::MAX).saturating_add(1);
    let mut bytes = Vec::with_capacity(
        usize::try_from(metadata.len())
            .unwrap_or(maximum)
            .min(maximum),
    );
    file.take(read_limit).read_to_end(&mut bytes)?;
    if bytes.len() > maximum {
        return Err(LkError::new(ErrorCode::PolicyExceeded, policy_message));
    }
    Ok(bytes)
}

fn write_temporary(directory: &Path, bytes: &[u8]) -> Result<PathBuf> {
    let serial = TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
    let path = directory.join(format!(".tmp-{}-{serial}", std::process::id()));
    let result = (|| -> Result<()> {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .mode(0o600)
            .open(&path)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

fn sync_directory(path: &Path) -> Result<()> {
    File::open(path)?.sync_all()?;
    Ok(())
}

fn cleanup_revision(path: &Path, directory: &Path) -> Result<()> {
    fs::remove_file(path)?;
    sync_directory(directory)
}

fn restore_head(directory: &Path, bytes: Option<&[u8]>) -> Result<()> {
    let path = directory.join("HEAD");
    if let Some(bytes) = bytes {
        let temporary = write_temporary(directory, bytes)?;
        fs::rename(temporary, path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    sync_directory(directory)
}

fn head_codec(error: crate::codec::CodecError) -> LkError {
    LkError::new(
        ErrorCode::ArtifactCorrupt,
        format!("workspace HEAD decoding failed: {error}"),
    )
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PublicationStep {
    None,
    BeforeRevisionWrite,
    AfterRevisionSync,
    AfterRevisionRename,
    AfterHeadSync,
    AfterHeadRename,
}

fn inject(actual: PublicationStep, expected: PublicationStep) -> Result<()> {
    if actual == expected {
        Err(LkError::new(
            ErrorCode::Io,
            format!("injected publication failure at {expected:?}"),
        ))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ids::DraftSymbol;
    use crate::schema::{Node, OperationKind, SemanticType, TypeDraft, ValueDraft};
    use crate::transaction::{
        ExpressionDraft, ExpressionKindDraft, FunctionBodyDraft, MatchArmDraft, NodeTarget,
        ProductFieldDraft, SumVariantDraft, Transaction, TransactionOp, TransactionResponseSpec,
        YieldingBodyDraft,
    };

    fn create_package(id: WorkspaceId) -> Transaction {
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "package".to_owned(),
            }],
        }
    }

    fn request(transaction: &Transaction) -> ApplyTransactionRequest {
        let mut return_symbols: Vec<DraftSymbol> = transaction
            .operations
            .iter()
            .filter_map(TransactionOp::created_symbol)
            .collect();
        return_symbols.sort();
        ApplyTransactionRequest {
            transaction: transaction.clone(),
            response: TransactionResponseSpec { return_symbols },
        }
    }

    #[test]
    fn nominal_declarations_survive_format_three_restart_and_rederive_layout() {
        let temporary = tempfile::tempdir().expect("state");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x94; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "p".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: NodeTarget::Draft(DraftSymbol::generated(1)),
                    name: "m".into(),
                },
                TransactionOp::CreateProductType {
                    symbol: DraftSymbol::generated(3),
                    module: NodeTarget::Draft(DraftSymbol::generated(2)),
                    name: "Reading".into(),
                    fields: vec![ProductFieldDraft {
                        symbol: DraftSymbol::generated(4),
                        name: "value".into(),
                        ty: TypeDraft::I64,
                    }],
                },
            ],
        };
        workspace
            .apply(&request(&transaction), [0x94; 32])
            .expect("commit");
        drop(workspace);
        let reopened = DurableWorkspace::open(temporary.path(), id).expect("restart");
        let head = reopened.head().expect("head");
        assert_eq!(head.revision(), Revision::new(1));
        let declaration = head
            .nodes()
            .find_map(|(node, record)| {
                matches!(record, Node::ProductType { name, .. } if name == "Reading")
                    .then_some(node)
            })
            .expect("reading");
        let layouts = crate::type_layout::derive_layouts(head).expect("layouts");
        let crate::type_layout::DerivedLayout::Representable(layout) =
            layouts.get(&declaration).expect("layout")
        else {
            panic!("representable")
        };
        assert_eq!((layout.size, layout.align, layout.cells), (8, 8, 1));
    }

    #[test]
    fn nominal_operation_and_match_graph_survives_format_three_restart_and_retained_query() {
        let temporary = tempfile::tempdir().expect("state");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x96; 16]);
        let local = |value| NodeTarget::Draft(DraftSymbol::generated(value));
        let result = |value| ValueDraft::OperationResult {
            operation: local(value),
            output: 0,
        };
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "p".into(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: local(1),
                    name: "m".into(),
                },
                TransactionOp::CreateSumType {
                    symbol: DraftSymbol::generated(3),
                    module: local(2),
                    name: "Maybe".into(),
                    variants: vec![
                        SumVariantDraft {
                            symbol: DraftSymbol::generated(4),
                            name: "none".into(),
                            payload: None,
                        },
                        SumVariantDraft {
                            symbol: DraftSymbol::generated(5),
                            name: "some".into(),
                            payload: Some(TypeDraft::I64),
                        },
                    ],
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(6),
                    module: local(2),
                    name: "match_it".into(),
                    parameters: Vec::new(),
                    result: TypeDraft::I64,
                    body: Some(FunctionBodyDraft {
                        operations: vec![
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(7)),
                                operation: ExpressionKindDraft::ConstI64(9),
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(8)),
                                operation: ExpressionKindDraft::ConstructVariant {
                                    variant: local(5),
                                    payload: Some(result(7)),
                                },
                            },
                            ExpressionDraft {
                                symbol: Some(DraftSymbol::generated(9)),
                                operation: ExpressionKindDraft::MatchSum {
                                    scrutinee: result(8),
                                    result: TypeDraft::I64,
                                    arms: vec![
                                        MatchArmDraft {
                                            variant: local(5),
                                            payload_symbol: Some(DraftSymbol::generated(10)),
                                            body: YieldingBodyDraft {
                                                operations: Vec::new(),
                                                yield_value: ValueDraft::BlockArgument(local(10)),
                                            },
                                        },
                                        MatchArmDraft {
                                            variant: local(4),
                                            payload_symbol: None,
                                            body: YieldingBodyDraft {
                                                operations: vec![ExpressionDraft {
                                                    symbol: Some(DraftSymbol::generated(11)),
                                                    operation: ExpressionKindDraft::ConstI64(0),
                                                }],
                                                yield_value: result(11),
                                            },
                                        },
                                    ],
                                },
                            },
                        ],
                        return_value: result(9),
                    }),
                },
            ],
        };
        workspace
            .apply(&request(&transaction), [0x96; 32])
            .expect("commit nominal match");
        drop(workspace);

        let reopened = DurableWorkspace::open(temporary.path(), id).expect("artifact3 restart");
        let retained = reopened
            .snapshot(Revision::new(1))
            .expect("retained revision");
        let declaration = retained
            .nodes()
            .find_map(|(node, record)| {
                matches!(record, Node::SumType { name, .. } if name == "Maybe").then_some(node)
            })
            .expect("sum declaration");
        let queried = crate::query::execute(
            retained,
            &crate::query::Query::NominalType {
                declaration,
                page: crate::query::PageRequest {
                    after: None,
                    limit: 2,
                },
            },
            None,
        )
        .expect("retained nominal query");
        let crate::query::QueryResult::NominalType(queried) = queried else {
            panic!("nominal result")
        };
        assert_eq!(queried.name, "Maybe");
        assert_eq!(queried.members.items.len(), 2);
        let arms = retained
            .nodes()
            .find_map(|(_, node)| match node {
                Node::Operation {
                    operation: OperationKind::MatchSum { arms, .. },
                    ..
                } => Some(arms),
                _ => None,
            })
            .expect("retained match");
        assert_eq!(arms.len(), 2);
        let first_variant = match &queried.members.items[0] {
            crate::query::NominalMemberFact::SumVariant { variant, .. } => *variant,
            _ => panic!("sum member"),
        };
        assert_eq!(arms[0].variant, first_variant);
    }

    #[test]
    fn state_directory_rejects_relative_and_symlinked_paths() {
        use std::os::unix::fs::symlink;

        let temporary = tempfile::tempdir().expect("temporary directory");
        let real = temporary.path().join("real");
        fs::create_dir(&real).expect("real directory");
        let linked = temporary.path().join("linked");
        symlink(&real, &linked).expect("state symlink");
        assert_eq!(
            ensure_state_directory(&linked.join("state"))
                .expect_err("symlink component must reject")
                .code,
            ErrorCode::Io
        );
        assert_eq!(
            ensure_state_directory(Path::new("relative-state"))
                .expect_err("relative state must reject")
                .code,
            ErrorCode::Io
        );
    }

    #[test]
    fn recognized_incomplete_workspace_staging_is_removed_on_startup() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x41; 16]);
        let staging = temporary
            .path()
            .join("workspaces")
            .join(format!(".creating-{id}-123-456"));
        fs::create_dir(&staging).expect("staging directory");
        fs::write(staging.join("partial"), b"partial").expect("partial file");
        assert!(
            list_workspace_ids(temporary.path())
                .expect("recover staging")
                .is_empty()
        );
        assert!(!staging.exists());
    }

    #[test]
    fn noncanonical_workspace_and_revision_path_aliases_reject() {
        let workspace_state = tempfile::tempdir().expect("workspace alias state");
        ensure_state_directory(workspace_state.path()).expect("state directory");
        let workspace_id = WorkspaceId::from_bytes([0xab; 16]);
        DurableWorkspace::create(workspace_state.path(), workspace_id).expect("workspace");
        let canonical = workspace_directory(workspace_state.path(), workspace_id);
        let alias = canonical
            .parent()
            .expect("workspaces directory")
            .join(workspace_id.to_string().to_uppercase());
        fs::rename(&canonical, &alias).expect("rename to uppercase alias");
        assert_eq!(
            list_workspace_ids(workspace_state.path())
                .expect_err("uppercase workspace alias must reject")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let revision_state = tempfile::tempdir().expect("revision alias state");
        ensure_state_directory(revision_state.path()).expect("state directory");
        let revision_id = WorkspaceId::from_bytes([0x44; 16]);
        DurableWorkspace::create(revision_state.path(), revision_id).expect("workspace");
        let revisions = workspace_directory(revision_state.path(), revision_id).join("revisions");
        fs::rename(
            revision_path(&revisions, Revision::INITIAL),
            revisions.join("0.lkjscript"),
        )
        .expect("rename to decimal alias");
        assert_eq!(
            DurableWorkspace::open(revision_state.path(), revision_id)
                .err()
                .expect("decimal revision alias must reject")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn only_strictly_named_temporary_files_are_recovered() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x42; 16]);
        DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let directory = workspace_directory(temporary.path(), id);
        let recognized = directory.join(".tmp-123-456");
        fs::write(&recognized, b"partial").expect("recognized temporary");
        DurableWorkspace::open(temporary.path(), id).expect("recover recognized temporary");
        assert!(!recognized.exists());

        fs::write(directory.join(".tmp-not-owned"), b"unknown").expect("unknown file");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("unknown temporary must reject")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn restart_rejects_history_that_clears_a_surviving_package_entry() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x45; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::Commit,
            operations: vec![
                TransactionOp::CreatePackage {
                    symbol: DraftSymbol::generated(1),
                    name: "package".to_owned(),
                },
                TransactionOp::CreateModule {
                    symbol: DraftSymbol::generated(2),
                    package: NodeTarget::Draft(DraftSymbol::generated(1)),
                    name: "module".to_owned(),
                },
                TransactionOp::CreateFunction {
                    symbol: DraftSymbol::generated(3),
                    module: NodeTarget::Draft(DraftSymbol::generated(2)),
                    name: "function".to_owned(),
                    parameters: Vec::new(),
                    result: SemanticType::I64.into(),
                    body: None,
                },
                TransactionOp::SetEntryFunction {
                    package: NodeTarget::Draft(DraftSymbol::generated(1)),
                    function: NodeTarget::Draft(DraftSymbol::generated(3)),
                },
            ],
        };
        let accepted_request = request(&transaction);
        let fingerprint = machine::transaction_fingerprint(&accepted_request).expect("fingerprint");
        workspace
            .apply(&accepted_request, fingerprint)
            .expect("selected entry commit");
        let previous = workspace.head().expect("revision one").clone();
        let mut nodes = previous.nodes.clone();
        let package = nodes
            .iter_mut()
            .find_map(|(id, node)| match node {
                Node::Package { entry: Some(_), .. } => Some((*id, node)),
                _ => None,
            })
            .expect("selected package");
        let Node::Package { entry, .. } = package.1 else {
            panic!("package kind")
        };
        *entry = None;
        let forged = Snapshot::from_parts(
            id,
            Revision::new(2),
            previous.root,
            previous.next_serial,
            previous.tombstones.clone(),
            nodes,
        )
        .expect("individually valid cleared entry snapshot");
        let directory = workspace_directory(temporary.path(), id);
        fs::write(
            revision_path(&directory.join("revisions"), forged.revision()),
            artifact::encode(&forged).expect("forged artifact"),
        )
        .expect("write forged revision");
        fs::write(
            directory.join("HEAD"),
            encode_head(forged.revision(), forged.hash(), None).expect("forged HEAD"),
        )
        .expect("write forged HEAD");
        drop(workspace);
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("restart must reject cleared entry history")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn head_checksum_and_file_size_policy_reject_corrupt_durable_state() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");

        let checksum_id = WorkspaceId::from_bytes([6; 16]);
        DurableWorkspace::create(temporary.path(), checksum_id).expect("workspace");
        let checksum_head = workspace_directory(temporary.path(), checksum_id).join("HEAD");
        let mut bytes = fs::read(&checksum_head).expect("head bytes");
        let last = bytes.last_mut().expect("head checksum byte");
        *last ^= 1;
        fs::write(&checksum_head, bytes).expect("corrupt head");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), checksum_id)
                .err()
                .expect("corrupt HEAD must reject")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let oversized_head_id = WorkspaceId::from_bytes([0x70; 16]);
        DurableWorkspace::create(temporary.path(), oversized_head_id).expect("workspace");
        let oversized_head = workspace_directory(temporary.path(), oversized_head_id).join("HEAD");
        OpenOptions::new()
            .write(true)
            .open(&oversized_head)
            .expect("HEAD file")
            .set_len(u64::try_from(MAXIMUM_HEAD_BYTES).expect("HEAD policy") + 1)
            .expect("extend HEAD");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), oversized_head_id)
                .err()
                .expect("oversized HEAD must reject before read")
                .code,
            ErrorCode::PolicyExceeded
        );

        let size_id = WorkspaceId::from_bytes([7; 16]);
        DurableWorkspace::create(temporary.path(), size_id).expect("workspace");
        let revision = revision_path(
            &workspace_directory(temporary.path(), size_id).join("revisions"),
            Revision::INITIAL,
        );
        let file = OpenOptions::new()
            .write(true)
            .open(revision)
            .expect("revision file");
        file.set_len(
            u64::try_from(artifact::DecodePolicy::default().maximum_artifact_bytes)
                .expect("artifact policy")
                + 1,
        )
        .expect("extend sparse revision");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), size_id)
                .err()
                .expect("oversized revision must reject before read")
                .code,
            ErrorCode::PolicyExceeded
        );
    }

    #[test]
    fn maximum_compact_receipt_keeps_head_below_explicit_policy() {
        let workspace = WorkspaceId::from_bytes([0x17; 16]);
        let returned_bindings = (0..crate::transaction::MAX_RETURNED_BINDINGS)
            .map(|index| {
                let prefix = format!("symbol_{index}_");
                let symbol = format!(
                    "{prefix}{}",
                    "x".repeat(crate::ids::MAX_DRAFT_SYMBOL_BYTES - prefix.len())
                );
                (
                    DraftSymbol::new(&symbol),
                    crate::ids::NodeId::new(workspace, u64::try_from(index).expect("serial") + 2)
                        .expect("node"),
                )
            })
            .collect();
        let record = IdempotencyRecord {
            key: IdempotencyKey::from_bytes([0x17; 16]),
            fingerprint: [0x23; 32],
            receipt: TransactionReceipt {
                workspace,
                base_revision: Revision::new(1),
                revision: Revision::new(2),
                hash: SnapshotHash::from_bytes([0x34; 32]),
                published: true,
                created_count: u64::MAX,
                returned_bindings,
                change_count: u64::MAX,
                change_digest: crate::ids::ChangeDigest::from_bytes([0x45; 32]),
                complete_before: false,
                complete_after: true,
                blocker_count_before: u64::MAX,
                blocker_count_after: 0,
            },
        };
        let bytes = encode_head(
            Revision::new(2),
            SnapshotHash::from_bytes([0x34; 32]),
            Some(&record),
        )
        .expect("maximum compact HEAD");
        assert!(bytes.len() < MAXIMUM_HEAD_BYTES);
        let (_, _, decoded) = decode_head(&bytes).expect("decode compact HEAD");
        assert_eq!(decoded.expect("idempotency").receipt, record.receipt);
    }

    #[test]
    fn publication_rejects_live_head_tampering_before_replacement() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x19; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let head = workspace.head().expect("head");
        let forged = encode_head(head.revision(), SnapshotHash::from_bytes([0x55; 32]), None)
            .expect("forged but decodable HEAD");
        let head_path = workspace_directory(temporary.path(), id).join("HEAD");
        fs::write(&head_path, &forged).expect("tamper live HEAD");
        let revision_before =
            fs::read_dir(workspace_directory(temporary.path(), id).join("revisions"))
                .expect("revisions")
                .count();
        let error = workspace
            .apply(&request(&create_package(id)), [0x22; 32])
            .expect_err("tampered live HEAD must reject");
        assert_eq!(error.code, ErrorCode::ArtifactCorrupt);
        assert_eq!(fs::read(&head_path).expect("HEAD remains tampered"), forged);
        assert_eq!(
            fs::read_dir(workspace_directory(temporary.path(), id).join("revisions"))
                .expect("revisions")
                .count(),
            revision_before
        );
        assert_eq!(
            workspace.head().expect("in-memory head").revision(),
            Revision::INITIAL
        );
    }

    #[test]
    fn every_apply_path_verifies_live_head_before_replay_or_validation() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");

        let replay_id = WorkspaceId::from_bytes([0x1a; 16]);
        let mut replay =
            DurableWorkspace::create(temporary.path(), replay_id).expect("replay workspace");
        let mut keyed = create_package(replay_id);
        keyed.idempotency_key = Some(IdempotencyKey::from_bytes([0x31; 16]));
        let keyed_request = request(&keyed);
        let fingerprint = machine::transaction_fingerprint(&keyed_request).expect("fingerprint");
        replay
            .apply(&keyed_request, fingerprint)
            .expect("keyed commit");
        let replay_head = workspace_directory(temporary.path(), replay_id).join("HEAD");
        let mut corrupt = fs::read(&replay_head).expect("HEAD");
        *corrupt.last_mut().expect("checksum") ^= 1;
        fs::write(&replay_head, corrupt).expect("corrupt replay HEAD");
        assert_eq!(
            replay
                .apply(&keyed_request, fingerprint)
                .expect_err("corrupt exact replay")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let conflict_id = WorkspaceId::from_bytes([0x1b; 16]);
        let mut conflict =
            DurableWorkspace::create(temporary.path(), conflict_id).expect("conflict workspace");
        let mut first = create_package(conflict_id);
        first.idempotency_key = Some(IdempotencyKey::from_bytes([0x32; 16]));
        let first_request = request(&first);
        let first_fingerprint =
            machine::transaction_fingerprint(&first_request).expect("fingerprint");
        conflict
            .apply(&first_request, first_fingerprint)
            .expect("keyed commit");
        let head = conflict.head().expect("head");
        let forged = encode_head(
            head.revision(),
            SnapshotHash::from_bytes([0x77; 32]),
            conflict.idempotency.as_ref(),
        )
        .expect("forged HEAD");
        fs::write(
            workspace_directory(temporary.path(), conflict_id).join("HEAD"),
            forged,
        )
        .expect("replace HEAD");
        let mut different = first_request.clone();
        different.transaction.operations[0] = TransactionOp::RenameNode {
            node: NodeTarget::Existing(head.root()),
            name: "different".into(),
        };
        let different_fingerprint =
            machine::transaction_fingerprint(&different).expect("fingerprint");
        assert_eq!(
            conflict
                .apply(&different, different_fingerprint)
                .expect_err("replaced conflict HEAD")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let validate_id = WorkspaceId::from_bytes([0x1c; 16]);
        let mut validate =
            DurableWorkspace::create(temporary.path(), validate_id).expect("validate workspace");
        fs::remove_file(workspace_directory(temporary.path(), validate_id).join("HEAD"))
            .expect("remove HEAD");
        let mut validate_transaction = create_package(validate_id);
        validate_transaction.mode = TransactionMode::ValidateOnly;
        assert_eq!(
            validate
                .apply(&request(&validate_transaction), [0x44; 32])
                .expect_err("missing validate HEAD")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let prepare_id = WorkspaceId::from_bytes([0x1d; 16]);
        let mut prepare =
            DurableWorkspace::create(temporary.path(), prepare_id).expect("prepare workspace");
        let prepare_head = workspace_directory(temporary.path(), prepare_id).join("HEAD");
        fs::write(&prepare_head, b"not a HEAD").expect("corrupt HEAD");
        assert_eq!(
            prepare
                .apply(&request(&create_package(prepare_id)), [0x55; 32])
                .expect_err("corrupt preparation HEAD")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn head4_unkeyed_grammar_remains_fixed_and_deterministic() {
        let revision = Revision::new(7);
        let hash = SnapshotHash::from_bytes([0xa5; SnapshotHash::BYTE_LEN]);
        let first = encode_head(revision, hash, None).expect("HEAD4 encode");
        assert_eq!(
            first,
            encode_head(revision, hash, None).expect("deterministic HEAD4")
        );

        let mut expected_body = Vec::new();
        expected_body.extend_from_slice(b"LKJHEAD4");
        expected_body.extend_from_slice(&7_u64.to_le_bytes());
        expected_body.extend_from_slice(&[0xa5; SnapshotHash::BYTE_LEN]);
        expected_body.push(0);
        assert_eq!(&first[..expected_body.len()], expected_body.as_slice());
        assert_eq!(first.len(), expected_body.len() + SnapshotHash::BYTE_LEN);
        assert_eq!(
            &first[expected_body.len()..],
            blake3::hash(&expected_body).as_bytes()
        );
        let (decoded_revision, decoded_hash, decoded_record) =
            decode_head(&first).expect("HEAD4 decode");
        assert_eq!((decoded_revision, decoded_hash), (revision, hash));
        assert!(decoded_record.is_none());
    }

    #[test]
    fn exact_commit_response_preflight_uses_real_id_and_fails_before_publication() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x67; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let request = request(&create_package(id));
        let fingerprint = machine::transaction_fingerprint(&request).expect("fingerprint");
        let head_path = workspace_directory(temporary.path(), id).join("HEAD");
        let head_before = fs::read(&head_path).expect("HEAD before preflight");

        assert_eq!(
            workspace
                .apply_with_response(&request, fingerprint, crate::ids::RequestId::new(0))
                .expect_err("zero request ID must fail exact response preflight")
                .code,
            ErrorCode::PolicyExceeded
        );
        assert_eq!(
            fs::read(&head_path).expect("HEAD after preflight"),
            head_before
        );
        assert_eq!(
            workspace.head().expect("in-memory head").revision(),
            Revision::INITIAL
        );

        let (receipt, bytes) = workspace
            .apply_with_response(&request, fingerprint, crate::ids::RequestId::new(91))
            .expect("commit with exact response preflight");
        let envelope = machine::decode_response(&bytes).expect("preflighted response JSON");
        assert_eq!(envelope.request_id, crate::ids::RequestId::new(91));
        assert_eq!(
            envelope.response,
            crate::protocol::Response::TransactionReceipt(receipt)
        );
    }

    #[test]
    fn head_version_three_magic_is_rejected_without_compatibility_reader() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([0x18; 16]);
        DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let head_path = workspace_directory(temporary.path(), id).join("HEAD");
        let mut bytes = fs::read(&head_path).expect("head bytes");
        bytes[..8].copy_from_slice(b"LKJHEAD3");
        let body_length = bytes.len() - SnapshotHash::BYTE_LEN;
        let checksum = blake3::hash(&bytes[..body_length]);
        bytes[body_length..].copy_from_slice(checksum.as_bytes());
        fs::write(&head_path, bytes).expect("old head magic");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("HEAD3 must reject")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn persisted_idempotency_receipt_is_semantically_validated() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([9; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let mut transaction = create_package(id);
        transaction.idempotency_key = Some(IdempotencyKey::from_bytes([0x91; 16]));
        let request = request(&transaction);
        let fingerprint = machine::transaction_fingerprint(&request).expect("fingerprint");
        let accepted = workspace
            .apply(&request, fingerprint)
            .expect("keyed transaction");
        let mut conflicting = request.clone();
        conflicting.transaction.base_revision = Revision::new(999);
        let conflicting_fingerprint =
            machine::transaction_fingerprint(&conflicting).expect("conflicting fingerprint");
        assert_eq!(
            workspace
                .apply(&conflicting, conflicting_fingerprint)
                .expect_err("matching key with a future base must conflict")
                .code,
            ErrorCode::IdempotencyConflict
        );
        assert_eq!(
            workspace
                .apply(&request, fingerprint)
                .expect("exact replay"),
            accepted
        );
        let valid_record = workspace.idempotency.clone().expect("idempotency record");
        let head = workspace.head().expect("head");
        let head_revision = head.revision();
        let head_hash = head.hash();
        let root = head.root();
        let head_path = workspace.directory.join("HEAD");

        let mut unpublished = valid_record.clone();
        unpublished.receipt.published = false;
        let forged =
            encode_head(head_revision, head_hash, Some(&unpublished)).expect("forged HEAD");
        fs::write(&head_path, forged).expect("write forged HEAD");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("unpublished idempotency result")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let mut missing = valid_record.clone();
        missing.receipt.created_count = 0;
        let forged = encode_head(head_revision, head_hash, Some(&missing)).expect("forged HEAD");
        fs::write(&head_path, forged).expect("write forged HEAD");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("wrong idempotency created count")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let mut wrong_digest = valid_record.clone();
        wrong_digest.receipt.change_digest = crate::ids::ChangeDigest::from_bytes([0xff; 32]);
        let forged =
            encode_head(head_revision, head_hash, Some(&wrong_digest)).expect("forged HEAD");
        fs::write(&head_path, forged).expect("write forged HEAD");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("wrong idempotency digest")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let mut prior = valid_record;
        prior.receipt.returned_bindings[0].1 = root;
        let forged = encode_head(head_revision, head_hash, Some(&prior)).expect("forged HEAD");
        fs::write(head_path, forged).expect("write forged HEAD");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("prior identity allocation")
                .code,
            ErrorCode::ArtifactCorrupt
        );
    }

    #[test]
    fn validate_only_and_commit_share_persistence_policy_preflight() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([8; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let directory = workspace_directory(temporary.path(), id);
        let head_path = directory.join("HEAD");
        let before_head = fs::read(&head_path).expect("initial head");
        let revision_files = || {
            let mut names: Vec<_> = fs::read_dir(directory.join("revisions"))
                .expect("revision directory")
                .map(|entry| entry.expect("revision entry").file_name())
                .collect();
            names.sort();
            names
        };
        let before_revisions = revision_files();
        let mut transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            mode: TransactionMode::ValidateOnly,
            operations: vec![TransactionOp::CreatePackage {
                symbol: DraftSymbol::generated(1),
                name: "x".repeat(artifact::DecodePolicy::default().maximum_name_bytes + 1),
            }],
        };
        for mode in [TransactionMode::ValidateOnly, TransactionMode::Commit] {
            transaction.mode = mode;
            let request = request(&transaction);
            let fingerprint = machine::transaction_fingerprint(&request).expect("fingerprint");
            assert_eq!(
                workspace
                    .apply(&request, fingerprint)
                    .expect_err("unreopenable artifact must reject in both modes")
                    .code,
                ErrorCode::PolicyExceeded
            );
            assert_eq!(fs::read(&head_path).expect("unchanged head"), before_head);
            assert_eq!(revision_files(), before_revisions);
            assert_eq!(
                workspace.head().expect("head").revision(),
                Revision::INITIAL
            );
            assert_eq!(workspace.head().expect("head").next_serial(), 2);
        }
    }

    #[test]
    fn keyed_head4_publication_faults_preserve_prior_replay_and_allocator() {
        let temporary = tempfile::tempdir().expect("temporary state directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([5; 16]);
        for fault in [
            PublicationStep::BeforeRevisionWrite,
            PublicationStep::AfterRevisionSync,
            PublicationStep::AfterRevisionRename,
            PublicationStep::AfterHeadSync,
            PublicationStep::AfterHeadRename,
        ] {
            let path = workspace_directory(temporary.path(), id);
            if path.exists() {
                fs::remove_dir_all(&path).expect("remove prior test workspace");
            }
            let mut workspace =
                DurableWorkspace::create(temporary.path(), id).expect("durable workspace creation");
            let mut prior_transaction = create_package(id);
            prior_transaction.idempotency_key = Some(IdempotencyKey::from_bytes([0x51; 16]));
            let prior_request = request(&prior_transaction);
            let prior_fingerprint =
                machine::transaction_fingerprint(&prior_request).expect("prior fingerprint");
            let prior_receipt = workspace
                .apply(&prior_request, prior_fingerprint)
                .expect("prior keyed commit");
            let package = prior_receipt.returned_bindings[0].1;
            let before = fs::read(path.join("HEAD")).expect("read prior keyed HEAD");

            let candidate = ApplyTransactionRequest {
                transaction: Transaction {
                    workspace: id,
                    base_revision: Revision::new(1),
                    idempotency_key: Some(IdempotencyKey::from_bytes([0x52; 16])),
                    mode: TransactionMode::Commit,
                    operations: vec![TransactionOp::CreateModule {
                        symbol: DraftSymbol::generated(2),
                        package: crate::transaction::NodeTarget::Existing(package),
                        name: "module".to_owned(),
                    }],
                },
                response: TransactionResponseSpec {
                    return_symbols: vec![DraftSymbol::generated(2)],
                },
            };
            let candidate_fingerprint =
                machine::transaction_fingerprint(&candidate).expect("candidate fingerprint");
            let error = workspace
                .apply_with_fault(&candidate, candidate_fingerprint, fault)
                .expect_err("fault must reject publication");
            assert_eq!(error.code, ErrorCode::Io);
            assert_eq!(workspace.head().expect("head").revision(), Revision::new(1));
            assert_eq!(workspace.head().expect("head").next_serial(), 3);
            assert_eq!(
                workspace
                    .idempotency
                    .as_ref()
                    .expect("prior idempotency")
                    .receipt,
                prior_receipt
            );
            assert_eq!(
                fs::read(path.join("HEAD")).expect("read head after fault"),
                before
            );

            let mut reopened =
                DurableWorkspace::open(temporary.path(), id).expect("prior HEAD must reopen");
            assert_eq!(reopened.head().expect("head").next_serial(), 3);
            assert_eq!(
                reopened
                    .idempotency
                    .as_ref()
                    .expect("reopened idempotency")
                    .receipt,
                prior_receipt
            );
            assert_eq!(
                reopened
                    .apply(&prior_request, prior_fingerprint)
                    .expect("prior exact replay after fault"),
                prior_receipt
            );
        }
        let _ = SemanticType::Unit;
    }
}
