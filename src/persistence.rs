use crate::artifact;
use crate::codec::{Reader, Writer};
use crate::diff;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, Workspace};
use crate::ids::{IdempotencyKey, Revision, SnapshotHash, WorkspaceId};
use crate::protocol;
use crate::transaction::{Transaction, TransactionResult};
use std::collections::BTreeMap;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

const HEAD_MAGIC: [u8; 8] = *b"LKJHEAD1";
const MAXIMUM_HEAD_BYTES: usize = protocol::MAXIMUM_FRAME_BYTES + 4096;
static TEMP_SERIAL: AtomicU64 = AtomicU64::new(1);

#[derive(Clone)]
pub(crate) struct IdempotencyRecord {
    pub key: IdempotencyKey,
    pub fingerprint: [u8; 32],
    pub result: TransactionResult,
}

pub(crate) struct DurableWorkspace {
    directory: PathBuf,
    workspace: Workspace,
    idempotency: Option<IdempotencyRecord>,
}

impl DurableWorkspace {
    pub(crate) fn create(state_directory: &Path, id: WorkspaceId) -> Result<Self> {
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
        let serial = TEMP_SERIAL.fetch_add(1, Ordering::Relaxed);
        let staging_directory =
            workspaces_directory.join(format!(".creating-{}-{}-{serial}", id, std::process::id()));
        create_private_directory(&staging_directory)?;
        if let Err(error) = create_private_directory(&staging_directory.join("revisions")) {
            let _ = fs::remove_dir_all(&staging_directory);
            return Err(error);
        }
        let workspace = match Workspace::new(id) {
            Ok(workspace) => workspace,
            Err(error) => {
                let _ = fs::remove_dir_all(&staging_directory);
                return Err(error);
            }
        };
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
        Ok(durable)
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

    pub(crate) fn head(&self) -> Result<&Arc<Snapshot>> {
        self.workspace.head()
    }

    pub(crate) fn apply(
        &mut self,
        transaction: &Transaction,
        fingerprint: [u8; 32],
    ) -> Result<TransactionResult> {
        if transaction.workspace != self.id() {
            return Err(LkError::new(
                ErrorCode::WrongWorkspace,
                "transaction names a different workspace",
            )
            .for_workspace(self.id()));
        }
        if let (Some(key), Some(record)) = (transaction.idempotency_key, &self.idempotency)
            && key == record.key
        {
            self.workspace.snapshot(transaction.base_revision)?;
            if fingerprint == record.fingerprint
                && record.result.base_revision == transaction.base_revision
            {
                return Ok(record.result.clone());
            }
            return Err(LkError::new(
                ErrorCode::IdempotencyConflict,
                "idempotency key was already used for a different transaction",
            )
            .for_workspace(self.id()));
        }
        let prepared = self.workspace.prepare_transaction(transaction)?;
        protocol::encoded_response_size(
            crate::ids::RequestId::new(0),
            &crate::protocol::Response::TransactionApplied(prepared.result.clone()),
        )?;
        if transaction.dry_run {
            return Ok(prepared.result);
        }
        let next_idempotency = transaction.idempotency_key.map(|key| IdempotencyRecord {
            key,
            fingerprint,
            result: prepared.result.clone(),
        });
        let retained_idempotency = next_idempotency
            .clone()
            .or_else(|| self.idempotency.clone());
        self.publish_snapshot(
            &prepared.snapshot,
            retained_idempotency.as_ref(),
            PublicationStep::None,
        )?;
        self.workspace.publish(prepared.snapshot)?;
        self.idempotency = retained_idempotency;
        Ok(prepared.result)
    }

    fn publish_snapshot(
        &self,
        snapshot: &Snapshot,
        idempotency: Option<&IdempotencyRecord>,
        fault: PublicationStep,
    ) -> Result<()> {
        let bytes = artifact::encode(snapshot)?;
        if bytes.len() > artifact::DecodePolicy::default().maximum_artifact_bytes {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "snapshot artifact exceeds daemon persistence policy",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision()));
        }
        let decoded = artifact::decode(&bytes)?;
        if decoded != *snapshot {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "encoded snapshot failed persistence round-trip preflight",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision()));
        }
        let revisions = self.directory.join("revisions");
        let revision_path = revision_path(&revisions, snapshot.revision());
        let old_head = read_head_before_publication(&self.directory, snapshot.revision())?;
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
            )? != bytes
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
            let temporary = write_temporary(&revisions, &bytes)?;
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

        let head_bytes = encode_head(snapshot.revision(), snapshot.hash(), idempotency)?;
        let head_temporary = match write_temporary(&self.directory, &head_bytes) {
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
        transaction: &Transaction,
        fault: PublicationStep,
    ) -> Result<TransactionResult> {
        let prepared = self.workspace.prepare_transaction(transaction)?;
        self.publish_snapshot(&prepared.snapshot, None, fault)?;
        self.workspace.publish(prepared.snapshot)?;
        Ok(prepared.result)
    }
}

fn validate_idempotency_record(
    record: &IdempotencyRecord,
    workspace: WorkspaceId,
    head: Revision,
    snapshots: &BTreeMap<Revision, Arc<Snapshot>>,
) -> Result<()> {
    let result = &record.result;
    let expected_revision = result.base_revision.next();
    if !result.published
        || result.workspace != workspace
        || result.revision > head
        || expected_revision != Some(result.revision)
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency result has invalid publication identity",
        )
        .for_workspace(workspace));
    }
    let base = snapshots.get(&result.base_revision).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency base revision is not retained",
        )
        .for_workspace(workspace)
        .at_revision(result.base_revision)
    })?;
    let published = snapshots.get(&result.revision).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency result revision is not retained",
        )
        .for_workspace(workspace)
        .at_revision(result.revision)
    })?;
    if published.hash() != result.hash || diff::between(base, published) != result.diff {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency result disagrees with retained snapshots",
        )
        .for_workspace(workspace)
        .at_revision(result.revision));
    }
    let expected_allocations = published
        .next_serial()
        .checked_sub(base.next_serial())
        .ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "persisted idempotency allocator transition moved backward",
            )
            .for_workspace(workspace)
        })?;
    if u64::try_from(result.allocations.len()).ok() != Some(expected_allocations) {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency allocations do not cover the allocator transition",
        )
        .for_workspace(workspace));
    }
    let mut previous_handle = None;
    let mut allocated_serials = std::collections::BTreeSet::new();
    for (handle, node) in &result.allocations {
        if previous_handle.is_some_and(|previous| *handle <= previous)
            || !allocated_serials.insert(node.serial())
            || node.workspace() != workspace
            || node.serial() < base.next_serial()
            || node.serial() >= published.next_serial()
            || (published.node(*node).is_err() && !published.contains_tombstone(node.serial()))
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "persisted idempotency allocation mapping is invalid",
            )
            .for_workspace(workspace)
            .for_node(*node));
        }
        previous_handle = Some(*handle);
    }
    for (offset, serial) in allocated_serials.into_iter().enumerate() {
        let offset = u64::try_from(offset).map_err(|_| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "persisted allocation offset overflows node serials",
            )
        })?;
        if base.next_serial().checked_add(offset) != Some(serial) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "persisted idempotency allocation serials are not contiguous",
            )
            .for_workspace(workspace));
        }
    }
    Ok(())
}

pub(crate) fn list_workspace_ids(state_directory: &Path) -> Result<Vec<WorkspaceId>> {
    let directory = state_directory.join("workspaces");
    ensure_private_directory(&directory)?;
    let mut ids = Vec::new();
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
        ids.push(id);
    }
    if removed_staging {
        sync_directory(&directory)?;
    }
    ids.sort();
    Ok(ids)
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
        protocol::put_transaction_result(&mut writer, &record.result)?;
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
        let result = protocol::read_transaction_result(&mut reader).map_err(|mut error| {
            if error.code != ErrorCode::PolicyExceeded {
                error.code = ErrorCode::ArtifactCorrupt;
            }
            error.message = format!(
                "workspace HEAD idempotency payload is invalid: {}",
                error.message
            );
            error
        })?;
        Some(IdempotencyRecord {
            key: IdempotencyKey::from_bytes(key),
            fingerprint,
            result,
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

fn revision_path(directory: &Path, revision: Revision) -> PathBuf {
    directory.join(format!("{:020}.lkjscript", revision.get()))
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
    use crate::ids::LocalHandle;
    use crate::schema::SemanticType;
    use crate::transaction::TransactionOp;

    fn create_package(id: WorkspaceId) -> Transaction {
        Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            dry_run: false,
            operations: vec![TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "package".to_owned(),
            }],
        }
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
    fn persisted_idempotency_result_is_semantically_validated() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([9; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let mut transaction = create_package(id);
        transaction.idempotency_key = Some(IdempotencyKey::from_bytes([0x91; 16]));
        let fingerprint = protocol::transaction_fingerprint(&transaction).expect("fingerprint");
        workspace
            .apply(&transaction, fingerprint)
            .expect("keyed transaction");
        let valid_record = workspace.idempotency.clone().expect("idempotency record");
        let head = workspace.head().expect("head");
        let head_revision = head.revision();
        let head_hash = head.hash();
        let root = head.root();
        let head_path = workspace.directory.join("HEAD");

        let mut unpublished = valid_record.clone();
        unpublished.result.published = false;
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
        missing.result.allocations.clear();
        let forged = encode_head(head_revision, head_hash, Some(&missing)).expect("forged HEAD");
        fs::write(&head_path, forged).expect("write forged HEAD");
        assert_eq!(
            DurableWorkspace::open(temporary.path(), id)
                .err()
                .expect("missing idempotency allocation")
                .code,
            ErrorCode::ArtifactCorrupt
        );

        let mut prior = valid_record;
        prior.result.allocations[0].1 = root;
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
    fn persistence_policy_rejects_unreopenable_names_before_commit() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        ensure_state_directory(temporary.path()).expect("state directory");
        let id = WorkspaceId::from_bytes([8; 16]);
        let mut workspace = DurableWorkspace::create(temporary.path(), id).expect("workspace");
        let head_path = workspace_directory(temporary.path(), id).join("HEAD");
        let before = fs::read(&head_path).expect("initial head");
        let transaction = Transaction {
            workspace: id,
            base_revision: Revision::INITIAL,
            idempotency_key: None,
            dry_run: false,
            operations: vec![TransactionOp::CreatePackage {
                handle: LocalHandle::new(1),
                name: "x".repeat(artifact::DecodePolicy::default().maximum_name_bytes + 1),
            }],
        };
        let fingerprint = protocol::transaction_fingerprint(&transaction).expect("fingerprint");
        assert_eq!(
            workspace
                .apply(&transaction, fingerprint)
                .expect_err("unreopenable artifact must reject")
                .code,
            ErrorCode::PolicyExceeded
        );
        assert_eq!(fs::read(head_path).expect("unchanged head"), before);
        assert_eq!(
            workspace.head().expect("head").revision(),
            Revision::INITIAL
        );
    }

    #[test]
    fn failures_before_commit_leave_head_and_allocator_unchanged() {
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
            let before = fs::read(path.join("HEAD")).expect("read original head");
            let error = workspace
                .apply_with_fault(&create_package(id), fault)
                .expect_err("fault must reject publication");
            assert_eq!(error.code, ErrorCode::Io);
            assert_eq!(
                workspace.head().expect("head").revision(),
                Revision::INITIAL
            );
            assert_eq!(
                fs::read(path.join("HEAD")).expect("read head after fault"),
                before
            );
            let reopened =
                DurableWorkspace::open(temporary.path(), id).expect("old head must reopen");
            assert_eq!(reopened.head().expect("head").next_serial(), 2);
        }
        let _ = SemanticType::Unit;
    }
}
