//! Canonical data revisions with an in-memory atomic head and shared immutable history.

use super::*;
use crate::platform::storage::project::{MemoryDataState, diagnostic as store_diagnostic, lock};

impl DataStore {
    pub(crate) fn open_memory(
        memory: &crate::platform::MemProjectStore,
        namespace: String,
        limits: DataLimits,
    ) -> Result<Self, Diagnostic> {
        limits.validate()?;
        validate_name(&namespace, MAXIMUM_DATA_NAMESPACE_BYTES, "data_namespace")?;
        let mut state = lock(&memory.data)?;
        let store_id = match state.store_id {
            Some(store_id) => {
                memory_head(&state, store_id)?;
                store_id
            }
            None => {
                if state.head.is_some() || state.revisions.len() != 0 {
                    return Err(data_error(
                        DiagnosticClass::Corrupt,
                        "data_memory_unpublished",
                        "unpublished memory data store contains accepted revision state",
                    ));
                }
                let store_id = random_identity()?;
                let empty = Snapshot {
                    parent: None,
                    schemas: BTreeMap::new(),
                    records: BTreeMap::new(),
                };
                let bytes = encode_revision_bounded(store_id, &empty, MAXIMUM_DATA_STORE_OBJECTS)?;
                let revision = digest(REVISION_DIGEST_DOMAIN, &bytes);
                let mut revisions = state.revisions.clone();
                revisions
                    .insert(revision, &bytes)
                    .map_err(store_diagnostic)?;
                state.revisions = revisions;
                state.store_id = Some(store_id);
                state.head = Some(revision);
                store_id
            }
        };
        Ok(Self {
            inner: Arc::new(DataStoreInner {
                root: PathBuf::from("memory:data"),
                memory: Some(memory.clone()),
                store_id,
                namespace,
                limits,
                live_transactions: AtomicUsize::new(0),
            }),
        })
    }

    pub(super) fn verify_memory(&self) -> Result<DataVerifyReceipt, Diagnostic> {
        let memory = self
            .inner
            .memory
            .as_ref()
            .ok_or_else(memory_backend_error)?;
        // Pin one head/history pair and release the publication lock before decoding history.
        let state = lock(&memory.data)?.clone();
        let head = memory_head(&state, self.inner.store_id)?;
        if state.revisions.len() > MAXIMUM_DATA_STORE_OBJECTS {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_store_objects",
                "data store object count exceeds the verification bound",
            ));
        }
        let mut reachable = BTreeSet::new();
        let mut cursor = Some(head);
        let mut schemas = 0;
        let mut records = 0;
        let mut bytes_read = 0_u64;
        while let Some(revision) = cursor {
            if !reachable.insert(revision) {
                return Err(data_error(
                    DiagnosticClass::Corrupt,
                    "data_revision_cycle",
                    "accepted data history contains a revision cycle",
                ));
            }
            if reachable.len() > MAXIMUM_DATA_HISTORY_REVISIONS {
                return Err(data_error(
                    DiagnosticClass::Resource,
                    "data_history_revisions",
                    "accepted data history exceeds the verification revision bound",
                ));
            }
            let (snapshot, bytes) = memory_revision(&state, self.inner.store_id, revision)?;
            bytes_read = add_verified_bytes(bytes_read, bytes)?;
            if revision == head {
                schemas = snapshot.schemas.len();
                records = snapshot.records.len();
            }
            cursor = snapshot.parent;
        }
        // Verify unreachable immutable objects as well, matching filesystem verification.
        for (revision, _) in state.revisions.iter() {
            let (_, bytes) = memory_revision(&state, self.inner.store_id, *revision)?;
            bytes_read = add_verified_bytes(bytes_read, bytes)?;
        }
        Ok(DataVerifyReceipt {
            store: format_store(self.inner.store_id),
            revision: format_revision(head),
            revisions: reachable.len(),
            objects: state.revisions.len(),
            schemas,
            records,
            staging_leftovers: 0,
            bytes_read,
        })
    }
}

impl DataStoreInner {
    pub(super) fn head(&self) -> Result<[u8; 32], Diagnostic> {
        match &self.memory {
            Some(memory) => {
                let state = lock(&memory.data)?;
                memory_head(&state, self.store_id)
            }
            None => read_head(&self.root, self.store_id),
        }
    }

    pub(super) fn snapshot(&self) -> Result<([u8; 32], Snapshot), Diagnostic> {
        match &self.memory {
            Some(memory) => {
                let state = lock(&memory.data)?;
                let head = memory_head(&state, self.store_id)?;
                let snapshot = memory_revision(&state, self.store_id, head)?.0;
                Ok((head, snapshot))
            }
            None => {
                let head = read_head(&self.root, self.store_id)?;
                Ok((head, read_revision(&self.root, self.store_id, head)?.0))
            }
        }
    }
}

impl DataTransaction {
    pub(super) fn commit_memory<F>(
        &mut self,
        hook: &mut F,
        maximum_objects: usize,
    ) -> Result<DataCommitOutcome, Diagnostic>
    where
        F: FnMut(CommitCheckpoint) -> Result<(), Diagnostic>,
    {
        let memory = self
            .store
            .memory
            .as_ref()
            .ok_or_else(memory_backend_error)?;
        let mut state = lock(&memory.data)?;
        let actual = memory_head(&state, self.store.store_id)?;
        if actual != self.base {
            return Ok(DataCommitOutcome::Conflict {
                expected: format_revision(self.base),
                actual: format_revision(actual),
            });
        }
        self.snapshot.parent = Some(self.base);
        let bytes = encode_revision_bounded(self.store.store_id, &self.snapshot, maximum_objects)?;
        let revision = digest(REVISION_DIGEST_DOMAIN, &bytes);
        if state.revisions.len() >= MAXIMUM_DATA_STORE_OBJECTS
            && state.revisions.get(&revision).is_none()
        {
            return Err(data_error(
                DiagnosticClass::Resource,
                "data_store_objects",
                "data store object count exceeds its physical history bound",
            ));
        }
        if let Some(existing) = state.revisions.get(&revision)
            && existing.as_slice() != bytes.as_slice()
        {
            return Err(data_error(
                DiagnosticClass::Corrupt,
                "data_revision_identity_collision",
                "existing immutable data revision disagrees with its content identity",
            ));
        }
        hook(CommitCheckpoint::BeforeRevisionStage)?;
        let mut revisions = state.revisions.clone();
        revisions
            .insert(revision, &bytes)
            .map_err(store_diagnostic)?;
        hook(CommitCheckpoint::RevisionBytesWritten)?;
        hook(CommitCheckpoint::RevisionStageSynced)?;
        hook(CommitCheckpoint::RevisionPublished)?;
        hook(CommitCheckpoint::HeadStageSynced)?;
        // Capacity and canonical encoding succeed before the only visibility point.
        state.revisions = revisions;
        state.head = Some(revision);
        hook(CommitCheckpoint::HeadPublished)?;
        hook(CommitCheckpoint::RootSynced)?;
        Ok(DataCommitOutcome::Committed {
            revision: format_revision(revision),
            durable_bytes: bytes.len(),
            fsync_publications: 0,
        })
    }
}

fn memory_head(state: &MemoryDataState, store_id: [u8; 32]) -> Result<[u8; 32], Diagnostic> {
    if state.store_id != Some(store_id) {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_store_identity_changed",
            "memory data store identity changed while the store was open",
        ));
    }
    state.head.ok_or_else(|| {
        data_error(
            DiagnosticClass::Corrupt,
            "data_head_missing",
            "memory data store has no accepted HEAD",
        )
    })
}

fn memory_revision(
    state: &MemoryDataState,
    store_id: [u8; 32],
    revision: [u8; 32],
) -> Result<(Snapshot, usize), Diagnostic> {
    let bytes = state.revisions.get(&revision).ok_or_else(|| {
        data_error(
            DiagnosticClass::Corrupt,
            "data_revision_missing",
            "memory HEAD references an absent immutable revision",
        )
    })?;
    let bytes = bytes.as_slice();
    if bytes.len() > MAXIMUM_DATA_REVISION_BYTES {
        return Err(data_error(
            DiagnosticClass::Resource,
            "data_revision_bytes",
            "immutable data revision exceeds its read bound",
        ));
    }
    if digest(REVISION_DIGEST_DOMAIN, bytes) != revision {
        return Err(data_error(
            DiagnosticClass::Corrupt,
            "data_revision_digest",
            "immutable data revision content disagrees with its identity",
        ));
    }
    Ok((decode_revision(bytes, store_id)?, bytes.len()))
}

fn add_verified_bytes(previous: u64, bytes: usize) -> Result<u64, Diagnostic> {
    previous.checked_add(bytes as u64).ok_or_else(|| {
        data_error(
            DiagnosticClass::Resource,
            "data_verify_bytes",
            "data verification byte counter overflowed",
        )
    })
}

fn memory_backend_error() -> Diagnostic {
    data_error(
        DiagnosticClass::Infrastructure,
        "data_memory_backend",
        "memory operation received a filesystem data store",
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(value: i64) -> DataKey {
        DataKey::new(vec![DataKeyPart::I64(value)], &DataLimits::default()).unwrap()
    }

    #[test]
    fn memory_transactions_preserve_snapshot_cas_namespaces_and_ordered_scans() {
        let memory = crate::platform::MemProjectStore::new(1_048_576).unwrap();
        let store = memory.data_store("app", DataLimits::default()).unwrap();
        let empty = store.begin().unwrap();
        let mut first = store.begin().unwrap();
        let mut stale = store.begin().unwrap();
        first
            .schema_set(
                "facts",
                &DataSchemaExpectation::Missing,
                DataSchema {
                    identity: "test".into(),
                    digest: vec![1; 32],
                },
            )
            .unwrap();
        for value in [3, 1, 2] {
            first
                .put(
                    "facts",
                    &key(value),
                    vec![value as u8],
                    DataExpectation::Missing,
                )
                .unwrap();
        }
        stale
            .put("facts", &key(4), vec![4], DataExpectation::Missing)
            .unwrap();
        assert!(matches!(
            first.commit().unwrap(),
            DataCommitOutcome::Committed {
                fsync_publications: 0,
                ..
            }
        ));
        assert!(matches!(
            stale.commit().unwrap(),
            DataCommitOutcome::Conflict { .. }
        ));
        assert!(empty.get("facts", &key(1)).unwrap().is_none());
        assert!(empty.schema_read("facts").unwrap().is_none());
        let reader = store.begin().unwrap();
        let page = reader
            .scan(
                "facts",
                &[],
                DataScanDirection::Forward,
                2,
                1_024,
                100,
                None,
            )
            .unwrap();
        assert_eq!(
            page.items
                .iter()
                .map(|item| item.key.clone())
                .collect::<Vec<_>>(),
            vec![key(1), key(2)]
        );
        let remaining = reader
            .scan(
                "facts",
                &[],
                DataScanDirection::Forward,
                2,
                1_024,
                100,
                page.continuation.as_deref(),
            )
            .unwrap();
        assert_eq!(remaining.items[0].key, key(3));
        assert!(remaining.continuation.is_none());
        let foreign = memory.data_store("other", DataLimits::default()).unwrap();
        assert_eq!(foreign.store_identity(), store.store_identity());
        assert!(
            foreign
                .begin()
                .unwrap()
                .get("facts", &key(1))
                .unwrap()
                .is_none()
        );
        let verified = store.verify().unwrap();
        assert_eq!(verified.revisions, 2);
        assert_eq!(verified.records, 3);
        assert_eq!(verified.schemas, 1);
        assert_eq!(verified.staging_leftovers, 0);
    }

    #[test]
    fn memory_forks_share_data_history_and_isolate_later_transactions() {
        let memory = crate::platform::MemProjectStore::new(1_048_576).unwrap();
        let store = memory.data_store("app", DataLimits::default()).unwrap();
        let mut initial = store.begin().unwrap();
        initial
            .put(
                "facts",
                &key(1),
                vec![9; 64 * 1_024],
                DataExpectation::Missing,
            )
            .unwrap();
        initial.commit().unwrap();
        let before = memory.used_bytes();
        let fork = memory.fork().unwrap();
        assert!(memory.used_bytes() - before < 4_096);
        let fork_store = fork.data_store("app", DataLimits::default()).unwrap();
        assert_eq!(
            fork_store.current_revision().unwrap(),
            store.current_revision().unwrap()
        );
        let mut mutation = fork_store.begin().unwrap();
        let entry = mutation.get("facts", &key(1)).unwrap().unwrap();
        mutation
            .put(
                "facts",
                &key(1),
                vec![5],
                DataExpectation::Exact(entry.revision),
            )
            .unwrap();
        mutation.commit().unwrap();
        assert_eq!(
            fork_store
                .begin()
                .unwrap()
                .get("facts", &key(1))
                .unwrap()
                .unwrap()
                .value,
            vec![5]
        );
        assert_eq!(
            store
                .begin()
                .unwrap()
                .get("facts", &key(1))
                .unwrap()
                .unwrap()
                .value,
            vec![9; 64 * 1_024]
        );
        assert_ne!(
            fork_store.current_revision().unwrap(),
            store.current_revision().unwrap()
        );
        drop(fork_store);
        drop(fork);
        assert_eq!(
            memory.used_bytes(),
            before,
            "fork-only revisions and index allocations leaked"
        );
    }

    #[test]
    fn memory_quota_failure_keeps_revision_and_capacity_unchanged() {
        let memory = crate::platform::MemProjectStore::new(8_192).unwrap();
        let store = memory.data_store("app", DataLimits::default()).unwrap();
        let before = memory.used_bytes();
        let head = store.current_revision().unwrap();
        let mut transaction = store.begin().unwrap();
        transaction
            .put("facts", &key(1), vec![7; 16_384], DataExpectation::Missing)
            .unwrap();
        let error = transaction.commit().unwrap_err();
        assert_eq!(error.code, "memory_project_capacity");
        assert_eq!(store.current_revision().unwrap(), head);
        assert_eq!(memory.used_bytes(), before);
        assert!(
            store
                .begin()
                .unwrap()
                .get("facts", &key(1))
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn memory_interruption_respects_the_single_visibility_point() {
        for point in [
            CommitCheckpoint::BeforeRevisionStage,
            CommitCheckpoint::RevisionBytesWritten,
            CommitCheckpoint::RevisionStageSynced,
            CommitCheckpoint::RevisionPublished,
            CommitCheckpoint::HeadStageSynced,
            CommitCheckpoint::HeadPublished,
            CommitCheckpoint::RootSynced,
        ] {
            let memory = crate::platform::MemProjectStore::new(1_048_576).unwrap();
            let store = memory.data_store("app", DataLimits::default()).unwrap();
            let before = store.current_revision().unwrap();
            let mut transaction = store.begin().unwrap();
            transaction
                .put("facts", &key(1), vec![8], DataExpectation::Missing)
                .unwrap();
            let error = transaction
                .commit_inner(&mut |observed| {
                    if observed == point {
                        Err(data_error(
                            DiagnosticClass::Cancelled,
                            "memory_interrupted",
                            "injected data publication interruption",
                        ))
                    } else {
                        Ok(())
                    }
                })
                .unwrap_err();
            assert_eq!(error.code, "memory_interrupted");
            let visible = matches!(
                point,
                CommitCheckpoint::HeadPublished | CommitCheckpoint::RootSynced
            );
            assert_eq!(store.current_revision().unwrap() != before, visible);
            assert_eq!(
                store
                    .begin()
                    .unwrap()
                    .get("facts", &key(1))
                    .unwrap()
                    .is_some(),
                visible
            );
            store.verify().unwrap();
        }
    }

    #[test]
    fn memory_admission_and_corruption_release_live_transaction_slots() {
        let memory = crate::platform::MemProjectStore::new(1_048_576).unwrap();
        let limits = DataLimits {
            maximum_live_transactions: 1,
            ..DataLimits::default()
        };
        let store = memory.data_store("app", limits).unwrap();
        let first = store.begin().unwrap();
        assert!(store.begin().is_err());
        drop(first);
        let head = lock(&memory.data).unwrap().head;
        lock(&memory.data).unwrap().head = Some([17; 32]);
        assert_eq!(store.begin().unwrap_err().code, "data_revision_missing");
        lock(&memory.data).unwrap().head = head;
        store.begin().unwrap();
    }
}
