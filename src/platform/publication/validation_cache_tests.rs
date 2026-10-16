//! Proof reuse must retain neither project storage nor permission to skip changed bytes.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use super::*;
use crate::platform::MemProjectStore;
use crate::platform::publication::{CreatedRepository, GraphRepository, RepositoryView};
use crate::platform::semantic_id::{RepositoryId, RevisionId};

fn create(memory: &MemProjectStore, seed: u64) -> CreatedRepository {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    snapshot.root.repository_id = RepositoryId::migrate(b"validation-cache", seed);
    GraphRepository::create_memory(memory, &snapshot, None).unwrap()
}

fn proof(repository: &GraphRepository) -> (ViewStore, Arc<CachedValidation>) {
    let current = repository.current().unwrap();
    let key = CurrentValidationKey {
        root: current.accepted.semantic_root,
        revision: current.head.revision,
        publication: current.head.record,
        validator: crate::platform::witness::contract::validator_contract_digest(),
        implementation: crate::platform::canonical_schema::VALIDATOR_IMPLEMENTATION_HASH,
    };
    let snapshot = RepositoryView::new(current, repository.object_store().unwrap())
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let mut objects = BTreeSet::new();
    let mut read_bytes = 0_u64;
    for owner in snapshot.owners.values() {
        let (digest, bytes) = crate::platform::kernel::encode_owner(owner).unwrap();
        objects.insert(ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()));
        read_bytes += bytes.len() as u64;
    }
    let context = Arc::new(RevalidatedBase::rebuild(snapshot, &|| Ok(())).unwrap());
    context.require_current().unwrap();
    let store = ViewStore::new(repository.object_store().unwrap());
    let selections = dependency_selections(
        &context.snapshot,
        &super::super::repository::read_store_readiness(store.persistent_base()).unwrap(),
    )
    .unwrap();
    let entry = cache_entry(key, read_bytes, objects, selections, context, &store).unwrap();
    (store, Arc::new(entry))
}

#[test]
fn interleaved_projects_reuse_distinct_complete_proofs() {
    let first_memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let second_memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let first = create(&first_memory, 1);
    let second = create(&second_memory, 2);
    let (first_store, first_proof) = proof(&first.repository);
    let (second_store, second_proof) = proof(&second.repository);
    assert_ne!(first_proof.key, second_proof.key);
    let mut cache = ValidationCache::default();
    cache.retain(first_proof.clone());
    cache.retain(second_proof.clone());
    for (store, expected) in [
        (&first_store, &first_proof),
        (&second_store, &second_proof),
        (&first_store, &first_proof),
        (&second_store, &second_proof),
    ] {
        let reused = cache.get(expected.key).unwrap();
        assert!(Arc::ptr_eq(&reused.context, &expected.context));
        let work = recheck(&reused, store, &|| Ok(()), REBUILD_LIMITS).unwrap();
        assert_eq!(work.objects_reused, expected.objects.len() as u64);
        assert_eq!(work.objects_read, 0);
        assert_eq!(work.bytes_read, 0);
    }
}

fn weighted(proof: &CachedValidation, ordinal: u8, bytes: u64) -> Arc<CachedValidation> {
    Arc::new(CachedValidation {
        key: CurrentValidationKey {
            revision: RevisionId::from_digest([ordinal; 32]),
            ..proof.key
        },
        objects: Box::new([]),
        selections: Box::new([]),
        context: proof.context.clone(),
        retained_bytes: bytes,
    })
}

#[test]
fn proof_cache_evicts_lru_at_both_limits_and_rejects_oversized_contexts() {
    let memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let created = create(&memory, 3);
    let (store, proof) = proof(&created.repository);
    let mut cache = ValidationCache::default();
    for ordinal in 0..MAXIMUM_CACHE_ENTRIES as u8 {
        cache.retain(weighted(&proof, ordinal, 1_024));
    }
    let oldest = weighted(&proof, 0, 1_024);
    cache.get(oldest.key).unwrap();
    cache.retain(weighted(&proof, 64, 1_024));
    assert_eq!(cache.entries.len(), MAXIMUM_CACHE_ENTRIES);
    assert!(cache.get(oldest.key).is_some());
    assert!(cache.get(weighted(&proof, 1, 1_024).key).is_none());
    assert_eq!(cache.retained_bytes, 64 * 1_024);

    let mut cache = ValidationCache::default();
    for ordinal in 0..4 {
        cache.retain(weighted(&proof, ordinal, MAXIMUM_CACHE_BYTES / 3));
    }
    assert_eq!(cache.entries.len(), 3);
    assert!(cache.get(oldest.key).is_none());
    assert_eq!(cache.retained_bytes, 3 * (MAXIMUM_CACHE_BYTES / 3));
    cache.retain(weighted(&proof, 9, MAXIMUM_CACHE_BYTES + 1));
    assert_eq!(cache.entries.len(), 3);
    assert!(cache.get(weighted(&proof, 9, 0).key).is_none());
    assert!(
        cache_entry(
            proof.key,
            MAXIMUM_CACHE_BYTES,
            BTreeSet::new(),
            Vec::new(),
            proof.context.clone(),
            &store
        )
        .is_none()
    );
}

#[test]
fn retained_proof_releases_all_dropped_project_arena_bytes() {
    let monitor = MemProjectStore::new(16 * 1_048_576).unwrap();
    let baseline = monitor.used_bytes();
    let mut cache = ValidationCache::default();
    let key;
    {
        let fork = monitor.fork().unwrap();
        let created = create(&fork, 4);
        let (store, proof) = proof(&created.repository);
        key = proof.key;
        cache.retain(proof.clone());
        assert!(recheck(&proof, &store, &|| Ok(()), REBUILD_LIMITS).is_ok());
        assert!(monitor.used_bytes() > baseline);
    }
    assert_eq!(monitor.used_bytes(), baseline);
    let retained = cache.get(key).unwrap();
    assert!(retained.objects.iter().all(|object| {
        object
            .memory
            .as_ref()
            .is_some_and(|memory| memory.upgrade().is_none())
    }));
    retained.context.require_current().unwrap();
}

#[test]
fn immutable_identity_reuse_keeps_limits_and_rechecks_replaced_bytes() {
    let memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let created = create(&memory, 5);
    let (store, proof) = proof(&created.repository);
    let work = recheck(&proof, &store, &|| Ok(()), REBUILD_LIMITS).unwrap();
    assert_eq!(work.objects_reused, proof.objects.len() as u64);
    for limits in [
        StoreReadLimits {
            maximum_catalog_lookups: 0,
            ..REBUILD_LIMITS
        },
        StoreReadLimits {
            maximum_objects: 0,
            ..REBUILD_LIMITS
        },
        StoreReadLimits {
            maximum_bytes: 0,
            ..REBUILD_LIMITS
        },
    ] {
        assert_eq!(
            recheck(&proof, &store, &|| Ok(()), limits)
                .unwrap_err()
                .class,
            DiagnosticClass::Resource
        );
    }
    let checks = std::cell::Cell::new(0);
    let error = recheck(
        &proof,
        &store,
        &|| {
            checks.set(checks.get() + 1);
            Err(Diagnostic::new(
                DiagnosticClass::Cancelled,
                "cache_test_cancel",
                "cancelled",
            ))
        },
        REBUILD_LIMITS,
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(checks.get(), 1);

    // Independent memory allocations with equal content need a fresh content check, not a miss.
    let second_memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let second = create(&second_memory, 5);
    let second_store = ViewStore::new(second.repository.object_store().unwrap());
    let work = recheck(&proof, &second_store, &|| Ok(()), REBUILD_LIMITS).unwrap();
    assert_eq!(work.objects_reused, 0);
    assert_eq!(work.objects_read, proof.objects.len() as u64);
    assert!(work.bytes_read > 0);

    // A hostile test replaces the allocation behind an unchanged key, bypassing normal staging.
    // Identity mismatch must fall through to content verification before any proof is reused.
    let key = proof.objects[0].key;
    {
        let mut graph = second_memory.graph.lock().unwrap();
        let mut bytes = graph.store.objects.get(&key).unwrap().as_slice().to_vec();
        bytes[0] ^= 0x80;
        graph.store.objects.insert(key, &bytes).unwrap();
    }
    let altered = ViewStore::new(second.repository.object_store().unwrap());
    assert_eq!(
        recheck(&proof, &altered, &|| Ok(()), REBUILD_LIMITS)
            .unwrap_err()
            .class,
        DiagnosticClass::Corrupt
    );
    // The earlier pinned view still holds the unchanged allocations.
    assert!(recheck(&proof, &second_store, &|| Ok(()), REBUILD_LIMITS).is_ok());
    let missing = ViewStore::new(StorageBackend::Memory(
        MemProjectStore::new(16 * 1_048_576)
            .unwrap()
            .graph
            .lock()
            .unwrap()
            .store
            .clone(),
    ));
    assert_eq!(
        recheck(&proof, &missing, &|| Ok(()), REBUILD_LIMITS)
            .unwrap_err()
            .code,
        "publication_current_validation_object_missing"
    );
}

#[test]
fn disk_proof_hits_recheck_every_recorded_object_after_tampering() {
    use std::io::{Read, Seek, SeekFrom, Write};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("repository");
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    snapshot.root.repository_id = RepositoryId::migrate(b"validation-cache-disk", 0);
    let created = GraphRepository::create(&root, &snapshot, None).unwrap();
    let (store, proof) = proof(&created.repository);
    let mut cache = ValidationCache::default();
    cache.retain(proof.clone());
    let work = recheck(
        &cache.get(proof.key).unwrap(),
        &store,
        &|| Ok(()),
        REBUILD_LIMITS,
    )
    .unwrap();
    assert_eq!(work.objects_reused, 0);
    assert_eq!(work.objects_read, proof.objects.len() as u64);
    let key = proof.objects.last().unwrap().key;
    let location = store.base.catalog_location(key).unwrap().unwrap();
    let head = std::fs::read(root.join("HEAD")).unwrap();
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(root.join("packs").join(location.pack.file_name()))
        .unwrap();
    file.seek(SeekFrom::Start(location.offset)).unwrap();
    let mut byte = [0_u8; 1];
    file.read_exact(&mut byte).unwrap();
    byte[0] ^= 0x80;
    file.seek(SeekFrom::Start(location.offset)).unwrap();
    file.write_all(&byte).unwrap();
    file.sync_all().unwrap();
    assert_eq!(
        recheck(
            &cache.get(proof.key).unwrap(),
            &store,
            &|| Ok(()),
            REBUILD_LIMITS
        )
        .unwrap_err()
        .class,
        DiagnosticClass::Corrupt
    );
    assert_eq!(std::fs::read(root.join("HEAD")).unwrap(), head);
}

fn empty_project(memory: &MemProjectStore, seed: u64) -> CreatedRepository {
    use crate::platform::kernel::{Name, PackageId, SemanticRoot};
    use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    let snapshot = KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::canonical_schema::SCHEMA_EPOCH,
            repository_id: RepositoryId::migrate(b"validation-selection", seed),
            package_id: PackageId::migrate(b"validation-selection", seed),
            package_name: Name::new("selection").unwrap(),
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    };
    GraphRepository::create_memory(memory, &snapshot, None).unwrap()
}

#[test]
fn changed_used_readiness_selection_forces_full_admission_with_old_objects_present() {
    use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
    use crate::platform::kernel::{PackageRevisionDigest, PackageTransportDigest};
    use crate::platform::publication::PublicationOptions;

    let supplier_memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let supplier = empty_project(&supplier_memory, 1);
    let exported = supplier.repository.export_package_transport().unwrap();
    let memory = MemProjectStore::new(16 * 1_048_576).unwrap();
    let consumer = empty_project(&memory, 2);
    consumer
        .repository
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    let prepared = consumer
        .repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: consumer.current.head.revision,
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::AddDependency {
                    package: exported.revision.package,
                    semantic_revision: exported.revision.revision.revision_id().unwrap(),
                    package_revision: exported.revision_digest,
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap();
    consumer.repository.publish(&prepared.publication).unwrap();
    let (pinned, _) = consumer
        .repository
        .view_current_with_source(&crate::platform::execution::ExecutionControl::uncancelled())
        .unwrap();
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    let retained = consumer
        .repository
        .view_current_with_source(&control)
        .unwrap()
        .0
        .into_source_validation_proof()
        .unwrap();
    consumer.repository.view_current().unwrap();
    let head = consumer.repository.current().unwrap().head;
    let (store, proof) = proof(&consumer.repository);
    assert_eq!(proof.selections.len(), 1);
    assert!(cached(proof.key, &store, &|| Ok(())).unwrap().is_some());
    let mut readiness =
        super::super::repository::read_store_readiness(store.persistent_base()).unwrap();
    let original_readiness = readiness.clone();
    let set_readiness =
        |readiness: &crate::platform::package_transport::source::PackageReadiness| {
            let mut graph = memory.graph.lock().unwrap();
            graph.store.readiness =
                Some(MemoryBytes::copy(&graph.store.arena, &readiness.encode().unwrap()).unwrap());
        };
    // Unused selections grant no authority and cannot invalidate a complete proof's exact inputs.
    readiness.bindings.insert(
        PackageRevisionDigest::from_bytes([0x71; 32]),
        PackageTransportDigest::from_bytes([0x72; 32]),
    );
    set_readiness(&readiness);
    assert!(
        consumer
            .repository
            .recheck_source_validation_proof(&retained, &control)
            .unwrap()
            .is_some()
    );
    assert!(
        selections_match(
            &consumer.repository.object_store().unwrap(),
            &proof.selections,
            &|| Ok(()),
        )
        .unwrap()
    );
    consumer
        .repository
        .view_current()
        .unwrap()
        .require_current_validation()
        .unwrap();

    readiness.bindings.insert(
        exported.revision_digest,
        PackageTransportDigest::from_bytes([0x73; 32]),
    );
    set_readiness(&readiness);
    let changed = ViewStore::new(consumer.repository.object_store().unwrap());
    assert_eq!(
        consumer
            .repository
            .recheck_source_validation_proof(&retained, &control)
            .unwrap_err()
            .code,
        "publication_current_validation_selection_changed"
    );
    assert!(!selections_match(&changed.base, &proof.selections, &|| Ok(())).unwrap());
    // Every original immutable object still verifies. Only the source selection changed.
    let (StorageBackend::Memory(original), StorageBackend::Memory(current)) =
        (&store.base, &changed.base)
    else {
        panic!("memory fixture");
    };
    for (key, bytes) in original.objects.iter() {
        let current = current.objects.get(key).unwrap().as_slice();
        assert_eq!(current, bytes.as_slice());
        key.verify(current).unwrap();
    }
    recheck(&proof, &changed, &|| Ok(()), REBUILD_LIMITS).unwrap();
    assert!(cached(proof.key, &changed, &|| Ok(())).unwrap().is_none());
    assert!(consumer.repository.view_current().is_err());
    assert_eq!(
        consumer
            .repository
            .recheck_exported_source(&pinned)
            .unwrap_err()
            .code,
        "publication_current_validation_selection_changed"
    );
    pinned.require_current_validation().unwrap();
    assert_eq!(consumer.repository.current().unwrap().head, head);
    set_readiness(&original_readiness);
    consumer
        .repository
        .view_current()
        .unwrap()
        .require_current_validation()
        .unwrap();
}
