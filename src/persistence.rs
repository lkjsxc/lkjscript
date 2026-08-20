use crate::artifact;
use crate::codec::{Reader, Writer};
use crate::diff;
use crate::error::{ErrorCode, LkError, Result};
use crate::graph::{Snapshot, Workspace};
use crate::history::{
    HistoryPage, RestorationReceipt, RevisionPublicationOutcome, RevisionRecord,
    RevisionRecordInspection,
};
use crate::ids::{
    ChangeDigest, DraftSymbol, IdempotencyKey, NodeId, Revision, RevisionRecordDigest,
    SnapshotHash, WorkspaceId,
};
use crate::machine;
use crate::query;
use crate::transaction::{
    ApplyTransactionRequest, PreparedTransaction, TransactionMode, TransactionReceipt,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, OnceLock};

const HEAD_MAGIC: [u8; 8] = *b"LKJHDA10";
const HEAD_CHECKSUM_DOMAIN: &str = "lkjscript.workspace-head.checksum.v10";
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
    record_bytes: Vec<u8>,
    record_digest: RevisionRecordDigest,
    head_bytes: Vec<u8>,
}

pub(crate) struct DurableWorkspace {
    directory: PathBuf,
    workspace: Workspace,
    retained_snapshots: BTreeMap<Revision, OnceLock<Arc<Snapshot>>>,
    records: BTreeMap<Revision, RevisionRecordInspection>,
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
        if let Err(error) = create_private_directory(&staging_directory.join("records")) {
            let _ = fs::remove_dir_all(&staging_directory);
            return Err(error);
        }
        let genesis = RevisionRecord::genesis(workspace.head()?);
        let genesis_digest = crate::history::digest(&genesis)?;
        let genesis_snapshot = Arc::clone(workspace.head()?);
        let retained_snapshots = BTreeMap::from([(
            Revision::INITIAL,
            initialized_snapshot_slot(genesis_snapshot),
        )]);
        let mut durable = Self {
            directory: staging_directory.clone(),
            workspace,
            retained_snapshots,
            records: BTreeMap::new(),
            idempotency: None,
        };
        if let Err(error) = durable.publish_snapshot(
            durable.workspace.head()?,
            &genesis,
            None,
            PublicationStep::None,
        ) {
            let _ = fs::remove_dir_all(&staging_directory);
            return Err(error);
        }
        durable.records.insert(
            Revision::INITIAL,
            RevisionRecordInspection {
                digest: genesis_digest,
                record: genesis,
            },
        );
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
        let (head_revision, head_hash, head_record, idempotency) = decode_head(&head_bytes)?;
        let retained_snapshots = scan_revision_artifacts(&directory, id, head_revision)?;
        let head_snapshot = load_snapshot_file(&directory, id, head_revision)?;
        if head_snapshot.hash() != head_hash {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace HEAD hash disagrees with its revision artifact",
            )
            .for_workspace(id)
            .at_revision(head_revision));
        }
        retained_snapshots
            .get(&head_revision)
            .ok_or_else(|| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace HEAD names a missing revision artifact",
                )
                .for_workspace(id)
                .at_revision(head_revision)
            })?
            .set(Arc::clone(&head_snapshot))
            .map_err(|_| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "workspace HEAD snapshot was initialized more than once",
                )
                .for_workspace(id)
                .at_revision(head_revision)
            })?;
        let records = load_revision_records(&directory, id, head_revision)?;
        if records.get(&head_revision).is_none_or(|record| {
            record.digest != head_record || record.record.result_snapshot != head_hash
        }) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace HEAD record digest disagrees with immutable history",
            )
            .for_workspace(id)
            .at_revision(head_revision));
        }
        let workspace = Workspace::from_head_snapshot(id, head_revision, head_snapshot)?;
        let durable = Self {
            directory,
            workspace,
            retained_snapshots,
            records,
            idempotency,
        };
        if let Some(record) = &durable.idempotency {
            let base = durable.snapshot(record.receipt.base_revision)?;
            let published = durable.snapshot(record.receipt.revision)?;
            validate_idempotency_record(record, id, head_revision, base, published)?;
        }
        Ok(durable)
    }

    pub(crate) const fn id(&self) -> WorkspaceId {
        self.workspace.id()
    }

    pub(crate) fn snapshot(&self, revision: Revision) -> Result<&Arc<Snapshot>> {
        let slot = self.retained_snapshots.get(&revision).ok_or_else(|| {
            LkError::new(
                ErrorCode::RevisionNotFound,
                "requested revision is not retained",
            )
            .for_workspace(self.id())
            .at_revision(revision)
        })?;
        if slot.get().is_none() {
            let snapshot = load_snapshot_file(&self.directory, self.id(), revision)?;
            let expected = self.record(revision)?.record.result_snapshot;
            if snapshot.hash() != expected {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "revision record snapshot digest disagrees with its artifact",
                )
                .for_workspace(self.id())
                .at_revision(revision));
            }
            let _ = slot.set(snapshot);
        }
        slot.get().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "retained snapshot could not be initialized",
            )
            .for_workspace(self.id())
            .at_revision(revision)
        })
    }

    pub(crate) fn record(&self, revision: Revision) -> Result<&RevisionRecordInspection> {
        self.records.get(&revision).ok_or_else(|| {
            LkError::new(
                ErrorCode::RevisionNotFound,
                "requested revision record is not retained",
            )
            .for_workspace(self.id())
            .at_revision(revision)
        })
    }

    /// Reconstructs and validates every retained semantic revision and every revision record.
    /// Ordinary open validates the selected snapshot plus the complete compact record chain; this
    /// method is the explicit full-history oracle used by deep doctor.
    pub(crate) fn deep_verify(&self) -> Result<()> {
        self.verify_live_head()?;
        let mut snapshots = BTreeMap::new();
        for revision in self.retained_snapshots.keys().copied() {
            snapshots.insert(revision, Arc::clone(self.snapshot(revision)?));
        }
        let reconstructed =
            Workspace::from_snapshots(self.id(), self.workspace.head_revision(), snapshots)?;
        let mut prior_snapshot = None;
        let mut prior_record = None;
        for revision in self.retained_snapshots.keys() {
            let snapshot = reconstructed.snapshot(*revision)?;
            let inspection = self.record(*revision)?;
            inspection
                .record
                .validate_against(prior_snapshot, snapshot, prior_record)?;
            prior_snapshot = Some(snapshot.as_ref());
            prior_record = Some(inspection.digest);
        }
        if let Some(record) = &self.idempotency {
            let base = reconstructed.snapshot(record.receipt.base_revision)?;
            let published = reconstructed.snapshot(record.receipt.revision)?;
            validate_idempotency_record(
                record,
                self.id(),
                self.workspace.head_revision(),
                base,
                published,
            )?;
        }
        Ok(())
    }

    /// Prepares an exact candidate against the live selected HEAD without publishing it. This is
    /// used by the project surface to return the same semantic preview as validation.
    pub(crate) fn prepare_transaction(
        &self,
        request: &ApplyTransactionRequest,
    ) -> Result<PreparedTransaction> {
        self.verify_live_head()?;
        self.workspace.prepare_transaction(request)
    }

    pub(crate) fn history_page(&self, before: Option<Revision>, limit: u32) -> Result<HistoryPage> {
        if limit == 0 || limit > crate::query::MAX_PAGE_ITEMS {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "history page limit must be within the public page policy",
            )
            .for_workspace(self.id()));
        }
        let head = self.workspace.head_revision();
        let start = before.unwrap_or_else(|| head.next().unwrap_or(head));
        if start > head.next().unwrap_or(head) {
            return Err(LkError::new(
                ErrorCode::RevisionNotFound,
                "history page starts beyond the current revision",
            )
            .for_workspace(self.id())
            .at_revision(start));
        }
        let records = self
            .records
            .range(..start)
            .rev()
            .take(limit as usize)
            .map(|(_, record)| crate::history::RevisionRecordSummary::from(record))
            .collect::<Vec<_>>();
        let next_before = records
            .last()
            .and_then(|record| (record.revision != Revision::INITIAL).then_some(record.revision));
        Ok(HistoryPage {
            workspace: self.id(),
            head,
            records,
            next_before,
        })
    }

    pub(crate) fn restore(
        &mut self,
        source_revision: Revision,
        validate_only: bool,
    ) -> Result<RestorationReceipt> {
        self.verify_live_head()?;
        let current = Arc::clone(self.workspace.head()?);
        let source = Arc::clone(self.snapshot(source_revision)?);
        if source.revision() == current.revision() {
            return Err(LkError::new(
                ErrorCode::NoChange,
                "selected restoration revision is already current",
            )
            .for_workspace(self.id())
            .at_revision(source_revision));
        }
        for (id, _) in source.nodes().filter(|(id, _)| id.is_durable()) {
            if current.node(id).is_err() {
                return Err(LkError::new(
                    ErrorCode::DeleteBlocked,
                    "restoration would resurrect a deleted durable identity",
                )
                .for_workspace(self.id())
                .at_revision(source_revision)
                .for_node(id));
            }
        }
        let revision = current.revision().next().ok_or_else(|| {
            LkError::new(
                ErrorCode::PolicyExceeded,
                "development revision sequence is exhausted",
            )
            .for_workspace(self.id())
        })?;
        let mut tombstones = current.tombstones.clone();
        for (id, _) in current.nodes().filter(|(id, _)| id.is_durable()) {
            if source.node(id).is_err() {
                tombstones.insert(id.serial());
            }
        }
        let candidate = Arc::new(Snapshot::from_parts(
            self.id(),
            revision,
            current.root(),
            current.next_serial(),
            tombstones,
            source.nodes.clone(),
        )?);
        let semantic_diff = diff::between(&current, &candidate);
        if semantic_diff.changes.is_empty() {
            return Err(LkError::new(
                ErrorCode::NoChange,
                "restoration would not change accepted semantic meaning",
            )
            .for_workspace(self.id())
            .at_revision(source_revision));
        }
        let parent_record = self.record(current.revision())?.digest;
        let accepted_change_set = restoration_change_digest(
            self.id(),
            source_revision,
            current.revision(),
            semantic_diff.digest,
        );
        let record = RevisionRecord::transition(
            &current,
            &candidate,
            parent_record,
            accepted_change_set,
            RevisionPublicationOutcome::Restoration,
        )?;
        let publication =
            self.preflight_publication(&candidate, &record, self.idempotency.as_ref())?;
        let record_inspection = RevisionRecordInspection {
            digest: publication.record_digest,
            record,
        };
        let receipt = RestorationReceipt {
            contract_version: crate::history::REVISION_RECORD_VERSION,
            workspace: self.id(),
            source_revision,
            base_revision: current.revision(),
            revision,
            snapshot: candidate.hash(),
            published: !validate_only,
            semantic_diff,
            revision_record: (!validate_only).then_some(record_inspection.clone()),
        };
        for response in [
            serde_json::to_vec(&receipt),
            serde_json::to_vec_pretty(&receipt),
        ] {
            let response = response.map_err(|error| {
                LkError::new(
                    ErrorCode::PolicyExceeded,
                    format!("restoration receipt cannot be encoded: {error}"),
                )
            })?;
            if response.len() > crate::machine::MAX_JSON_OUTPUT_BYTES.saturating_sub(1024) {
                return Err(LkError::new(
                    ErrorCode::PolicyExceeded,
                    "restoration receipt exceeds project response policy",
                ));
            }
        }
        if validate_only {
            return Ok(receipt);
        }
        self.publish_preflighted(&candidate, &publication, PublicationStep::None)?;
        let retained_candidate = Arc::clone(&candidate);
        self.workspace.publish(candidate)?;
        self.retained_snapshots
            .insert(revision, initialized_snapshot_slot(retained_candidate));
        self.records.insert(revision, record_inspection);
        Ok(receipt)
    }

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
        let base = self.workspace.snapshot(transaction.base_revision)?;
        let parent_record = self.record(transaction.base_revision)?.digest;
        let revision_record = RevisionRecord::transition(
            base,
            &prepared.snapshot,
            parent_record,
            ChangeDigest::from_bytes(fingerprint),
            RevisionPublicationOutcome::Accepted,
        )?;
        let response_bytes = preflight_receipt_response(request_id, &prepared.receipt)?;
        if transaction.mode == TransactionMode::ValidateOnly {
            self.preflight_publication(
                &prepared.snapshot,
                &revision_record,
                self.idempotency.as_ref(),
            )?;
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
        let publication = self.preflight_publication(
            &prepared.snapshot,
            &revision_record,
            retained_idempotency.as_ref(),
        )?;
        self.publish_preflighted(&prepared.snapshot, &publication, fault)?;
        let record_inspection = RevisionRecordInspection {
            digest: publication.record_digest,
            record: revision_record,
        };
        let revision = prepared.snapshot.revision();
        let retained_snapshot = Arc::clone(&prepared.snapshot);
        self.workspace.publish(prepared.snapshot)?;
        self.retained_snapshots
            .insert(revision, initialized_snapshot_slot(retained_snapshot));
        self.records
            .insert(record_inspection.record.revision, record_inspection);
        self.idempotency = retained_idempotency;
        Ok((prepared.receipt, response_bytes))
    }

    fn verify_live_head(&self) -> Result<()> {
        let bytes =
            read_head_before_publication(&self.directory, Revision::new(1))?.ok_or_else(|| {
                LkError::new(ErrorCode::ArtifactCorrupt, "live workspace HEAD is missing")
            })?;
        let (revision, hash, record_digest, idempotency) =
            decode_head(&bytes).map_err(|mut error| {
                error.workspace = Some(self.id());
                error
            })?;
        let expected = self.workspace.head()?;
        let expected_record = self.record(expected.revision())?;
        if revision != expected.revision()
            || hash != expected.hash()
            || record_digest != expected_record.digest
            || idempotency != self.idempotency
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "live workspace HEAD changed after engine authority was established",
            )
            .for_workspace(self.id())
            .at_revision(revision));
        }
        Ok(())
    }

    fn preflight_publication(
        &self,
        snapshot: &Snapshot,
        record: &RevisionRecord,
        idempotency: Option<&IdempotencyRecord>,
    ) -> Result<PreparedPublication> {
        let artifact_bytes = artifact::encode(snapshot)?;
        if artifact_bytes.len() > artifact::DecodePolicy::default().maximum_artifact_bytes {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "snapshot artifact exceeds engine persistence policy",
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
        let record_bytes = crate::history::encode(record)?;
        let (decoded_record, record_digest) = crate::history::decode(&record_bytes)?;
        if decoded_record != *record {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "encoded revision record failed persistence round-trip preflight",
            )
            .for_workspace(snapshot.workspace())
            .at_revision(snapshot.revision()));
        }
        let head_bytes = encode_head(
            snapshot.revision(),
            snapshot.hash(),
            record_digest,
            idempotency,
        )?;
        Ok(PreparedPublication {
            artifact_bytes,
            record_bytes,
            record_digest,
            head_bytes,
        })
    }

    fn publish_snapshot(
        &self,
        snapshot: &Snapshot,
        record: &RevisionRecord,
        idempotency: Option<&IdempotencyRecord>,
        fault: PublicationStep,
    ) -> Result<()> {
        let publication = self.preflight_publication(snapshot, record, idempotency)?;
        self.publish_preflighted(snapshot, &publication, fault)
    }

    fn publish_preflighted(
        &self,
        snapshot: &Snapshot,
        publication: &PreparedPublication,
        fault: PublicationStep,
    ) -> Result<()> {
        let bytes = &publication.artifact_bytes;
        let record_bytes = &publication.record_bytes;
        let head_bytes = &publication.head_bytes;
        let revisions = self.directory.join("revisions");
        let records = self.directory.join("records");
        let revision_path = revision_path(&revisions, snapshot.revision());
        let record_path = record_path(&records, snapshot.revision());
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

        let record_existed = match fs::symlink_metadata(&record_path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                if !revision_existed {
                    cleanup_revision(&revision_path, &revisions)?;
                }
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "immutable revision-record path is not a regular file",
                )
                .for_workspace(snapshot.workspace())
                .at_revision(snapshot.revision()));
            }
            Ok(metadata) => {
                if metadata.len() > u64::try_from(record_bytes.len()).unwrap_or(u64::MAX) {
                    if !revision_existed {
                        cleanup_revision(&revision_path, &revisions)?;
                    }
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "immutable revision-record length disagrees with canonical bytes",
                    )
                    .for_workspace(snapshot.workspace())
                    .at_revision(snapshot.revision()));
                }
                true
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => false,
            Err(error) => return Err(error.into()),
        };
        if record_existed {
            if read_bounded_regular_file(
                &record_path,
                record_bytes.len(),
                "immutable revision-record path exceeds canonical byte length",
            )? != record_bytes.as_slice()
            {
                if !revision_existed {
                    cleanup_revision(&revision_path, &revisions)?;
                }
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "immutable revision-record path already contains different bytes",
                )
                .for_workspace(snapshot.workspace())
                .at_revision(snapshot.revision()));
            }
        } else {
            let temporary = match write_temporary(&records, record_bytes) {
                Ok(path) => path,
                Err(error) => {
                    if !revision_existed {
                        cleanup_revision(&revision_path, &revisions)?;
                    }
                    return Err(error);
                }
            };
            if let Err(error) = fs::rename(&temporary, &record_path) {
                fs::remove_file(&temporary)?;
                sync_directory(&records)?;
                if !revision_existed {
                    cleanup_revision(&revision_path, &revisions)?;
                }
                return Err(error.into());
            }
            if let Err(error) = sync_directory(&records) {
                cleanup_revision(&record_path, &records)?;
                if !revision_existed {
                    cleanup_revision(&revision_path, &revisions)?;
                }
                return Err(error);
            }
        }

        let head_temporary = match write_temporary(&self.directory, head_bytes) {
            Ok(path) => path,
            Err(error) => {
                if !record_existed {
                    cleanup_revision(&record_path, &records)?;
                }
                if !revision_existed {
                    cleanup_revision(&revision_path, &revisions)?;
                }
                return Err(error);
            }
        };
        if let Err(error) = inject(fault, PublicationStep::AfterHeadSync) {
            fs::remove_file(&head_temporary)?;
            sync_directory(&self.directory)?;
            if !record_existed {
                cleanup_revision(&record_path, &records)?;
            }
            if !revision_existed {
                cleanup_revision(&revision_path, &revisions)?;
            }
            return Err(error);
        }
        let head_path = self.directory.join("HEAD");
        if let Err(error) = fs::rename(&head_temporary, &head_path) {
            fs::remove_file(&head_temporary)?;
            sync_directory(&self.directory)?;
            if !record_existed {
                cleanup_revision(&record_path, &records)?;
            }
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
            if !record_existed {
                cleanup_revision(&record_path, &records)?;
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

fn restoration_change_digest(
    workspace: WorkspaceId,
    source: Revision,
    base: Revision,
    semantic_diff: ChangeDigest,
) -> ChangeDigest {
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.restoration-change.v1");
    hasher.update(&workspace.as_bytes());
    hasher.update(&source.get().to_le_bytes());
    hasher.update(&base.get().to_le_bytes());
    hasher.update(&semantic_diff.as_bytes());
    ChangeDigest::from_bytes(*hasher.finalize().as_bytes())
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
    base: &Snapshot,
    published: &Snapshot,
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
    if base.workspace() != workspace
        || base.revision() != receipt.base_revision
        || published.workspace() != workspace
        || published.revision() != receipt.revision
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "persisted idempotency snapshots have invalid publication identity",
        )
        .for_workspace(workspace)
        .at_revision(receipt.revision));
    }
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
        let valid_identity = if node.is_function_local() {
            published.node(*node).is_ok()
        } else {
            node.serial() >= base.next_serial()
                && node.serial() < published.next_serial()
                && (published.node(*node).is_ok() || published.contains_tombstone(node.serial()))
        };
        if !selected_symbols.insert(*symbol)
            || !selected_serials.insert(node.serial())
            || node.workspace() != workspace
            || !valid_identity
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
        if (name == "HEAD" && file_type.is_file())
            || ((name == "revisions" || name == "records") && file_type.is_dir())
        {
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
            "engine state directory must be an explicit absolute path",
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
            "transaction receipt binding count exceeds HEAD10 encoding",
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
        let node = NodeId::from_encoded(WorkspaceId::from_bytes(node_workspace), serial).map_err(
            |error| {
                LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    format!("workspace HEAD contains an invalid receipt node identity: {error}"),
                )
            },
        )?;
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
    record: RevisionRecordDigest,
    idempotency: Option<&IdempotencyRecord>,
) -> Result<Vec<u8>> {
    let mut writer = Writer::new();
    writer.fixed(&HEAD_MAGIC);
    writer.u64(revision.get());
    writer.fixed(&hash.as_bytes());
    writer.fixed(&record.as_bytes());
    writer.bool(idempotency.is_some());
    if let Some(record) = idempotency {
        writer.fixed(&record.key.as_bytes());
        writer.fixed(&record.fingerprint);
        put_transaction_receipt(&mut writer, &record.receipt)?;
    }
    let body = writer.finish();
    let checksum = head_checksum(&body);
    let mut bytes = Vec::with_capacity(body.len() + checksum.len());
    bytes.extend_from_slice(&body);
    bytes.extend_from_slice(&checksum);
    if bytes.len() > MAXIMUM_HEAD_BYTES {
        return Err(LkError::new(
            ErrorCode::PolicyExceeded,
            "workspace HEAD exceeds encoder byte policy",
        ));
    }
    Ok(bytes)
}

fn decode_head(
    bytes: &[u8],
) -> Result<(
    Revision,
    SnapshotHash,
    RevisionRecordDigest,
    Option<IdempotencyRecord>,
)> {
    if bytes.len() < SnapshotHash::BYTE_LEN {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace HEAD is truncated before its checksum",
        ));
    }
    let body_length = bytes.len() - SnapshotHash::BYTE_LEN;
    let (body, encoded_checksum) = bytes.split_at(body_length);
    if head_checksum(body).as_slice() != encoded_checksum {
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
    let mut record = [0_u8; RevisionRecordDigest::BYTE_LEN];
    record.copy_from_slice(
        reader
            .fixed(RevisionRecordDigest::BYTE_LEN)
            .map_err(head_codec)?,
    );
    let record = RevisionRecordDigest::from_bytes(record);
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
    Ok((revision, hash, record, idempotency))
}

fn head_checksum(body: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new_derive_key(HEAD_CHECKSUM_DOMAIN);
    hasher.update(body);
    *hasher.finalize().as_bytes()
}

fn reject_existing_symlink_components(path: &Path) -> Result<()> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(LkError::new(
                    ErrorCode::Io,
                    "engine state path contains a symbolic-link component",
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
                    "engine state path is not a real directory",
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

fn record_file_name(revision: Revision) -> String {
    format!("{:020}.lkjrecord", revision.get())
}

fn record_path(directory: &Path, revision: Revision) -> PathBuf {
    directory.join(record_file_name(revision))
}

fn initialized_snapshot_slot(snapshot: Arc<Snapshot>) -> OnceLock<Arc<Snapshot>> {
    OnceLock::from(snapshot)
}

fn scan_revision_artifacts(
    workspace_directory: &Path,
    workspace: WorkspaceId,
    head: Revision,
) -> Result<BTreeMap<Revision, OnceLock<Arc<Snapshot>>>> {
    let directory = workspace_directory.join("revisions");
    reject_symlink(&directory)?;
    let maximum_artifact_bytes =
        u64::try_from(artifact::DecodePolicy::default().maximum_artifact_bytes).unwrap_or(u64::MAX);
    let mut snapshots = BTreeMap::new();
    let mut removed_orphans = false;
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revisions directory contains a nonregular entry",
            )
            .for_workspace(workspace));
        }
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision has a non-UTF-8 file name",
            )
            .for_workspace(workspace)
        })?;
        if is_temporary_file_name(name) {
            fs::remove_file(entry.path())?;
            removed_orphans = true;
            continue;
        }
        let stem = name.strip_suffix(".lkjscript").ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revisions directory contains an unknown file",
            )
            .for_workspace(workspace)
        })?;
        let revision = Revision::new(stem.parse::<u64>().map_err(|_| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision file name is invalid",
            )
            .for_workspace(workspace)
        })?);
        if name != revision_file_name(revision) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision file name is not canonical",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
        if revision > head {
            fs::remove_file(entry.path())?;
            removed_orphans = true;
            continue;
        }
        let metadata = fs::symlink_metadata(entry.path())?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision path is not a regular file",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
        if metadata.len() > maximum_artifact_bytes {
            return Err(LkError::new(
                ErrorCode::PolicyExceeded,
                "revision artifact exceeds decoder byte policy",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
        if snapshots.insert(revision, OnceLock::new()).is_some() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace contains duplicate revision artifacts",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
    }
    if removed_orphans {
        sync_directory(&directory)?;
    }
    let expected_count = head.get().checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace head revision cannot form a retained history length",
        )
        .for_workspace(workspace)
    })?;
    if u64::try_from(snapshots.len()).ok() != Some(expected_count) {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace retained revision history has a gap",
        )
        .for_workspace(workspace)
        .at_revision(head));
    }
    for (expected, revision) in (0..expected_count).map(Revision::new).zip(snapshots.keys()) {
        if expected != *revision {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace retained revision history is not contiguous",
            )
            .for_workspace(workspace)
            .at_revision(expected));
        }
    }
    Ok(snapshots)
}

fn load_snapshot_file(
    workspace_directory: &Path,
    workspace: WorkspaceId,
    revision: Revision,
) -> Result<Arc<Snapshot>> {
    let bytes = read_bounded_regular_file(
        &revision_path(&workspace_directory.join("revisions"), revision),
        artifact::DecodePolicy::default().maximum_artifact_bytes,
        "revision artifact exceeds decoder byte policy",
    )
    .map_err(|mut error| {
        error.workspace = Some(workspace);
        error.revision = Some(revision);
        error
    })?;
    let snapshot = artifact::decode(&bytes).map_err(|mut error| {
        error.workspace = Some(workspace);
        error.revision = Some(revision);
        error
    })?;
    if snapshot.workspace() != workspace || snapshot.revision() != revision {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "revision artifact identity disagrees with its durable path",
        )
        .for_workspace(workspace)
        .at_revision(revision));
    }
    Ok(Arc::new(snapshot))
}

fn load_revision_records(
    workspace_directory: &Path,
    workspace: WorkspaceId,
    head: Revision,
) -> Result<BTreeMap<Revision, RevisionRecordInspection>> {
    let directory = workspace_directory.join("records");
    reject_symlink(&directory)?;
    let mut records = BTreeMap::new();
    let mut removed_orphans = false;
    for entry in fs::read_dir(&directory)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        if file_type.is_symlink() || !file_type.is_file() {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace records directory contains a nonregular entry",
            )
            .for_workspace(workspace));
        }
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision record has a non-UTF-8 file name",
            )
            .for_workspace(workspace)
        })?;
        if is_temporary_file_name(name) {
            fs::remove_file(entry.path())?;
            removed_orphans = true;
            continue;
        }
        let stem = name.strip_suffix(".lkjrecord").ok_or_else(|| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace records directory contains an unknown file",
            )
            .for_workspace(workspace)
        })?;
        let revision = Revision::new(stem.parse::<u64>().map_err(|_| {
            LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision-record file name is invalid",
            )
            .for_workspace(workspace)
        })?);
        if name != record_file_name(revision) {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace revision-record file name is not canonical",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
        if revision > head {
            fs::remove_file(entry.path())?;
            removed_orphans = true;
            continue;
        }
        let bytes = read_bounded_regular_file(
            &entry.path(),
            crate::history::MAXIMUM_REVISION_RECORD_BYTES,
            "revision record exceeds decoder byte policy",
        )?;
        let (record, digest) = crate::history::decode(&bytes)?;
        if record.workspace != workspace || record.revision != revision {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "revision-record identity disagrees with its durable path",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
        if records
            .insert(revision, RevisionRecordInspection { digest, record })
            .is_some()
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "workspace contains duplicate revision records",
            )
            .for_workspace(workspace)
            .at_revision(revision));
        }
    }
    if removed_orphans {
        sync_directory(&directory)?;
    }
    let expected_count = head.get().checked_add(1).ok_or_else(|| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace head revision cannot form a retained history length",
        )
        .for_workspace(workspace)
    })?;
    if u64::try_from(records.len()).ok() != Some(expected_count) {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "workspace revision-record history has a gap",
        )
        .for_workspace(workspace)
        .at_revision(head));
    }
    validate_revision_record_chain(workspace, head, &records)?;
    Ok(records)
}

fn validate_revision_record_chain(
    workspace: WorkspaceId,
    head: Revision,
    records: &BTreeMap<Revision, RevisionRecordInspection>,
) -> Result<()> {
    let root = NodeId::new(workspace, 1).map_err(|error| {
        LkError::new(
            ErrorCode::ArtifactCorrupt,
            format!("canonical workspace root identity is invalid: {error}"),
        )
        .for_workspace(workspace)
    })?;
    let mut prior: Option<&RevisionRecordInspection> = None;
    for (expected_value, (revision, inspection)) in (0..=head.get()).zip(records) {
        let expected = Revision::new(expected_value);
        let record = &inspection.record;
        if *revision != expected
            || record.version != crate::history::REVISION_RECORD_VERSION
            || record.workspace != workspace
            || record.revision != expected
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "revision-record identity or order is inconsistent",
            )
            .for_workspace(workspace)
            .at_revision(expected));
        }
        match prior {
            None => {
                if expected != Revision::INITIAL
                    || record.parent_revision.is_some()
                    || record.parent_snapshot.is_some()
                    || record.parent_record.is_some()
                    || record.outcome != RevisionPublicationOutcome::Genesis
                    || record.change_count != 0
                    || record.created.as_slice() != [root]
                    || !record.deleted.is_empty()
                    || !record.modified.is_empty()
                    || !record.function_bodies_changed.is_empty()
                    || !record.target_definitions_changed.is_empty()
                    || !record.affected_targets.is_empty()
                    || record.accepted_change_set != record.semantic_diff
                {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "genesis revision-record facts are not canonical",
                    )
                    .for_workspace(workspace)
                    .at_revision(expected));
                }
            }
            Some(previous) => {
                if record.parent_revision != Some(previous.record.revision)
                    || record.parent_snapshot != Some(previous.record.result_snapshot)
                    || record.parent_record != Some(previous.digest)
                    || record.outcome == RevisionPublicationOutcome::Genesis
                    || record.change_count == 0
                {
                    return Err(LkError::new(
                        ErrorCode::ArtifactCorrupt,
                        "revision-record parent chain is inconsistent",
                    )
                    .for_workspace(workspace)
                    .at_revision(expected));
                }
            }
        }
        for nodes in [
            &record.created,
            &record.deleted,
            &record.modified,
            &record.function_bodies_changed,
            &record.target_definitions_changed,
            &record.affected_targets,
        ] {
            if nodes.windows(2).any(|pair| pair[0] >= pair[1])
                || nodes
                    .iter()
                    .any(|node| node.workspace() != workspace || node.is_function_local())
            {
                return Err(LkError::new(
                    ErrorCode::ArtifactCorrupt,
                    "revision-record entity facts are not canonical",
                )
                .for_workspace(workspace)
                .at_revision(expected));
            }
        }
        let created = record.created.iter().copied().collect::<BTreeSet<_>>();
        let deleted = record.deleted.iter().copied().collect::<BTreeSet<_>>();
        let modified = record.modified.iter().copied().collect::<BTreeSet<_>>();
        if !created.is_disjoint(&deleted)
            || !created.is_disjoint(&modified)
            || !deleted.is_disjoint(&modified)
            || !record.function_bodies_changed.iter().all(|node| {
                created.contains(node) || deleted.contains(node) || modified.contains(node)
            })
            || !record.target_definitions_changed.iter().all(|node| {
                created.contains(node) || deleted.contains(node) || modified.contains(node)
            })
            || (expected != Revision::INITIAL
                && u64::try_from(created.len() + deleted.len() + modified.len())
                    .ok()
                    .is_none_or(|count| count > record.change_count))
        {
            return Err(LkError::new(
                ErrorCode::ArtifactCorrupt,
                "revision-record change sets are inconsistent",
            )
            .for_workspace(workspace)
            .at_revision(expected));
        }
        prior = Some(inspection);
    }
    Ok(())
}

fn read_bounded_regular_file(path: &Path, maximum: usize, policy_message: &str) -> Result<Vec<u8>> {
    let path_metadata = fs::symlink_metadata(path)?;
    if path_metadata.file_type().is_symlink() || !path_metadata.is_file() {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "durable path is not a regular file",
        ));
    }
    let file = File::open(path)?;
    let metadata = file.metadata()?;
    let current_path_metadata = fs::symlink_metadata(path)?;
    if !metadata.is_file()
        || current_path_metadata.file_type().is_symlink()
        || !current_path_metadata.is_file()
        || metadata.dev() != path_metadata.dev()
        || metadata.ino() != path_metadata.ino()
        || metadata.dev() != current_path_metadata.dev()
        || metadata.ino() != current_path_metadata.ino()
    {
        return Err(LkError::new(
            ErrorCode::ArtifactCorrupt,
            "durable regular file changed during validated open",
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
mod tests;
