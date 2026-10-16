//! The memory backend uses the same accepted-object and closure checks as disk publication.

use super::*;

#[cfg(test)]
#[path = "memory_repository_tests.rs"]
mod tests;

impl GraphRepository {
    pub(crate) fn create_memory(
        memory: &MemProjectStore,
        logical: &KernelSnapshot,
        intent: Option<String>,
    ) -> Result<CreatedRepository, Diagnostic> {
        Self::create_memory_with_package_transports(memory, logical, intent, &[])
    }

    pub(crate) fn create_memory_with_package_transports(
        memory: &MemProjectStore,
        logical: &KernelSnapshot,
        intent: Option<String>,
        package_transports: &[InitialPackageTransport],
    ) -> Result<CreatedRepository, Diagnostic> {
        let mut state = memory_lock(&memory.graph)?;
        if state.head.is_some() {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_repository_exists",
                "memory project already has accepted authority",
            ));
        }
        let mut candidate = state.clone();
        let mut store = StorageBackend::Memory(candidate.store.clone());
        for transport in package_transports {
            install_package_container(
                None,
                &mut store,
                transport.digest,
                &transport.container,
                None,
                &ExecutionControl::uncancelled(),
            )?;
        }
        let initial =
            prepare_initial_publication(logical, &store, intent).map_err(collapse_diagnostics)?;
        let StorageBackend::Memory(store) = store else {
            return Err(memory_backend_error());
        };
        candidate.store = store;
        let outcome = publish_memory_state(&mut candidate, &initial.publication, None)?;
        let PublicationOutcome::Accepted { current, .. } = outcome else {
            return Err(repository_error(
                DiagnosticClass::Infrastructure,
                "publication_repository_initial_outcome",
                "initial memory publication was not accepted",
            ));
        };
        *state = candidate;
        Ok(CreatedRepository {
            repository: Self {
                root: PathBuf::from("memory:project"),
                memory: Some(memory.clone()),
            },
            initial,
            current,
        })
    }

    pub(crate) fn open_memory(memory: &MemProjectStore) -> Result<Self, Diagnostic> {
        let repository = Self {
            root: PathBuf::from("memory:project"),
            memory: Some(memory.clone()),
        };
        repository.current()?;
        Ok(repository)
    }

    pub(crate) fn is_memory(&self) -> bool {
        self.memory.is_some()
    }

    #[cfg(test)]
    pub(crate) fn test_memory_inventory(
        &self,
    ) -> Result<Option<crate::platform::storage::project::MemoryProjectInventory>, Diagnostic> {
        self.memory
            .as_ref()
            .map(MemProjectStore::test_inventory)
            .transpose()
    }

    pub(crate) fn memory_compilation_head(&self) -> Result<Option<Vec<u8>>, Diagnostic> {
        let Some(memory) = &self.memory else {
            return Ok(None);
        };
        Ok(memory_lock(&memory.graph)?
            .compilation_head
            .as_ref()
            .map(|bytes| bytes.as_slice().to_vec()))
    }

    pub(crate) fn set_memory_compilation_head(&self, bytes: &[u8]) -> Result<(), Diagnostic> {
        let memory = self.memory.as_ref().ok_or_else(memory_backend_error)?;
        let mut state = memory_lock(&memory.graph)?;
        let value = MemoryBytes::copy(&memory.arena, bytes).map_err(store_diagnostic)?;
        state.compilation_head = Some(value);
        Ok(())
    }

    /// Observe the selected head and immutable object index under one publication lock.
    pub(super) fn head_snapshot(&self) -> Result<(Option<HeadRecord>, StorageBackend), Diagnostic> {
        if let Some(memory) = &self.memory {
            let state = memory_lock(&memory.graph)?;
            let head = state
                .head
                .as_ref()
                .map(|bytes| HeadRecord::decode(bytes.as_slice()))
                .transpose()?;
            return Ok((head, StorageBackend::Memory(state.store.clone())));
        }
        let root_directory = open_directory(&self.root)?;
        let lock = open_lock(&root_directory)?;
        FileExt::lock_shared(&lock).map_err(|error| {
            io_diagnostic("publication_repository_read_lock", &self.root, error)
        })?;
        let store = open_store_shared(&root_directory, &self.root, &lock)?;
        let head = read_optional_regular_at(
            &root_directory,
            HEAD_FILE,
            MAXIMUM_HEAD_BYTES,
            "publication_repository_head_read",
        )?
        .map(|bytes| HeadRecord::decode(&bytes))
        .transpose()?;
        Ok((head, store))
    }

    pub(super) fn snapshot(
        &self,
    ) -> Result<(Option<CurrentPublication>, StorageBackend), Diagnostic> {
        let (head, store) = self.head_snapshot()?;
        let current = head
            .map(|head| read_publication(&store, head))
            .transpose()?;
        Ok((current, store))
    }

    pub(super) fn publish_memory(
        &self,
        prepared: &PreparedPublication,
        fault: Option<PublicationPoint>,
    ) -> Result<PublicationOutcome, Diagnostic> {
        #[cfg(test)]
        if matches!(
            fault,
            Some(PublicationPoint::Storage(_) | PublicationPoint::HeadDirectorySyncFailed)
        ) {
            return Err(repository_error(
                DiagnosticClass::Source,
                "publication_memory_fault",
                "filesystem durability fault points require a disk repository",
            ));
        }
        let memory = self.memory.as_ref().ok_or_else(memory_backend_error)?;
        let mut state = memory_lock(&memory.graph)?;
        publish_memory_state(&mut state, prepared, fault)
    }

    pub(super) fn stage_memory_compilation(
        &self,
        expected: HeadRecord,
        objects: &BTreeMap<ObjectKey, Vec<u8>>,
    ) -> Result<(SealReceipt, StoreWork), Diagnostic> {
        let memory = self.memory.as_ref().ok_or_else(memory_backend_error)?;
        let mut state = memory_lock(&memory.graph)?;
        let before = state
            .head
            .as_ref()
            .map(|bytes| HeadRecord::decode(bytes.as_slice()))
            .transpose()?;
        if before != Some(expected) {
            return Err(repository_error(
                DiagnosticClass::Semantic,
                "publication_compilation_stage_stale",
                "accepted HEAD changed before derived compilation objects could be staged",
            ));
        }
        let mut store = StorageBackend::Memory(state.store.clone());
        // Read the bound accepted objects under the publication lock before deriving anything.
        read_publication(&store, expected)?;
        let mut work = StoreWork::default();
        for (key, bytes) in objects {
            store
                .stage(*key, bytes, &mut work)
                .map_err(store_diagnostic)?;
        }
        let seal = store
            .seal_staged(TARGET_PACK_BYTES, &mut work)
            .map_err(store_diagnostic)?;
        let StorageBackend::Memory(store) = store else {
            return Err(memory_backend_error());
        };
        state.store = store;
        Ok((seal, work))
    }

    pub(super) fn stage_memory_package(
        &self,
        digest: crate::platform::kernel::PackageTransportDigest,
        container: &[u8],
        fault: Option<PackageStagePoint>,
        control: &ExecutionControl,
    ) -> Result<PackageTransportStageReceipt, Diagnostic> {
        let memory = self.memory.as_ref().ok_or_else(memory_backend_error)?;
        let mut state = memory_lock(&memory.graph)?;
        control.check().map_err(execution_diagnostic)?;
        let head = state.head.as_ref().ok_or_else(unpublished)?;
        let head = HeadRecord::decode(head.as_slice())?;
        let mut store = StorageBackend::Memory(state.store.clone());
        read_publication(&store, head)?;
        let installed =
            install_package_container(None, &mut store, digest, container, fault, control)?;
        read_publication(&store, head)?;
        let StorageBackend::Memory(store) = store else {
            return Err(memory_backend_error());
        };
        state.store = store;
        package_stage_checkpoint(fault, PackageStagePoint::ReadinessVisible)?;
        Ok(PackageTransportStageReceipt {
            package: installed.revision.package,
            semantic_revision: installed.revision.revision.revision_id()?,
            package_revision: installed.transport.package_revision,
            package_transport: digest,
            previous_transport: installed.previous_transport,
            outcome: installed.outcome,
            seal: installed.seal,
            current_revision: head.revision,
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
}

fn publish_memory_state(
    state: &mut MemoryGraphState,
    prepared: &PreparedPublication,
    fault: Option<PublicationPoint>,
) -> Result<PublicationOutcome, Diagnostic> {
    let mut store = StorageBackend::Memory(state.store.clone());
    let current = state
        .head
        .as_ref()
        .map(|head| read_publication(&store, HeadRecord::decode(head.as_slice())?))
        .transpose()?;
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
    let mut work = StoreWork::default();
    validate_prepared_dependency_sources(&store, prepared)?;
    validate_candidate_idempotency_transition(
        &store,
        prepared.revision.publication.idempotency_receipts,
        current.as_ref().map(|value| value.accepted),
        &prepared.objects,
        &mut work,
    )?;
    inject(fault, PublicationPoint::BeforeObjectStage)?;
    for (index, (key, bytes)) in prepared.objects.iter().enumerate() {
        store
            .stage(*key, bytes, &mut work)
            .map_err(store_diagnostic)?;
        if index == 0 {
            inject(fault, PublicationPoint::AfterFirstObjectStage)?;
        }
    }
    let seal = store
        .seal_staged(TARGET_PACK_BYTES, &mut work)
        .map_err(store_diagnostic)?;
    inject(fault, PublicationPoint::AfterPacksSealed)?;
    let limits = CanonicalReadAdmission::default();
    #[cfg(test)]
    let limits = match fault {
        Some(PublicationPoint::ReferenceAdmission(limits)) => limits,
        _ => limits,
    };
    verify_prepared_closure(&store, current.as_ref(), prepared, &mut work, limits)?;
    let next_head = HeadRecord::decode(&prepared.head_bytes)?;
    let current = read_publication(&store, next_head)?;
    let StorageBackend::Memory(store) = store else {
        return Err(memory_backend_error());
    };
    let head = MemoryBytes::copy(&store.arena, &prepared.head_bytes).map_err(store_diagnostic)?;
    inject(fault, PublicationPoint::AfterHeadFileSynced)?;
    // This is the only publication point. All capacity, closure, and exact-byte checks precede it.
    state.store = store;
    state.head = Some(head);
    inject(fault, PublicationPoint::AfterHeadRenamed)?;
    inject(fault, PublicationPoint::AfterHeadDirectorySynced)?;
    Ok(PublicationOutcome::Accepted {
        current,
        seal,
        store_work: work,
    })
}

pub(super) fn unpublished() -> Diagnostic {
    repository_error(
        DiagnosticClass::Source,
        "publication_repository_unpublished",
        "repository has no accepted HEAD",
    )
}

fn memory_backend_error() -> Diagnostic {
    repository_error(
        DiagnosticClass::Infrastructure,
        "publication_memory_backend",
        "memory repository operation received a disk store",
    )
}
