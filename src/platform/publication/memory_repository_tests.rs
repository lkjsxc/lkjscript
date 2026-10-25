use super::*;
use crate::platform::change::{AuthoredChange, ChangeBudget, OwnerSelector};
use crate::platform::kernel::{Name, OwnerRecord};

fn fixture(maximum: usize) -> (MemProjectStore, CreatedRepository) {
    let memory = MemProjectStore::new(maximum).unwrap();
    let snapshot = crate::platform::kernel::tests::witness_snapshot();
    let created = GraphRepository::create_memory(&memory, &snapshot, None).unwrap();
    (memory, created)
}

fn rename(
    repository: &GraphRepository,
    name: &str,
    key: Option<&str>,
) -> PreparedAuthoredPublication {
    let view = repository.view_current().unwrap();
    let snapshot = view.reconstruct_full_oracle().unwrap().value;
    let owner = snapshot
        .owners
        .iter()
        .find_map(|(key, record)| matches!(record, OwnerRecord::Module(_)).then_some(*key))
        .unwrap();
    view.prepare_authored_change(
        &AuthoredChangeSet {
            base: view.revision(),
            preconditions: Vec::new(),
            budget: ChangeBudget::default(),
            changes: vec![AuthoredChange::RenameOwner {
                owner: OwnerSelector::Exact { owner },
                name: Name::new(name).unwrap(),
            }],
        },
        PublicationOptions {
            idempotency_key: key.map(str::to_owned),
            intent: None,
        },
    )
    .unwrap()
}

#[test]
fn memory_forks_share_bytes_and_preserve_pinned_revision_and_history() {
    let (memory, created) = fixture(16 * 1_048_576);
    let original = created.repository.view_current().unwrap();
    let original_snapshot = original.reconstruct_full_oracle().unwrap().value;
    let before_fork = memory.used_bytes();
    let fork = memory.fork().unwrap();
    assert!(
        memory.used_bytes() - before_fork < 4_096,
        "fork copied object payloads or indexes"
    );
    let repository = GraphRepository::open_memory(&fork).unwrap();
    let pinned = repository.view_current().unwrap();
    let prepared = rename(&repository, "changed_in_fork", Some("memory-key"));
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    assert_ne!(repository.current().unwrap().head, original.current().head);
    assert_eq!(
        created.repository.current().unwrap().head,
        original.current().head
    );
    assert_eq!(
        pinned.reconstruct_full_oracle().unwrap().value.owners,
        original_snapshot.owners
    );
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::AlreadyAccepted { .. }
    ));
    assert!(matches!(
        repository.reconcile(&prepared.publication).unwrap().status,
        ReconciliationStatus::Accepted { .. }
    ));
    let recovered = repository
        .view_idempotency_base("memory-key", original.revision())
        .unwrap()
        .unwrap();
    assert_eq!(recovered.current().head, original.current().head);
    let ancestor = repository
        .view_ancestor_from(
            &repository.view_current().unwrap(),
            original.revision(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(ancestor.current().head, original.current().head);
    repository.history(2).unwrap();
    assert!(repository.head_staging_leftovers().unwrap().is_empty());
}

#[test]
fn memory_observation_pins_complete_history_without_holding_publication_locks() {
    use crate::platform::contributor::{memory_project_head, visit_memory_project};
    use crate::platform::data::{DataExpectation, DataKey, DataKeyPart, DataLimits};
    use std::collections::BTreeMap;

    fn records(memory: &MemProjectStore) -> BTreeMap<String, Vec<u8>> {
        let mut records = BTreeMap::new();
        visit_memory_project(memory, |name, bytes| {
            assert!(records.insert(name.to_owned(), bytes.to_vec()).is_none());
        })
        .unwrap();
        records
    }

    let (memory, created) = fixture(16 * 1_048_576);
    let store = memory
        .data_store("observation", DataLimits::default())
        .unwrap();
    let key = DataKey::new(vec![DataKeyPart::I64(1)], &DataLimits::default()).unwrap();
    let mut transaction = store.begin().unwrap();
    transaction
        .put("facts", &key, vec![1, 2, 3], DataExpectation::Missing)
        .unwrap();
    let prepared = rename(&created.repository, "after_observation", None);
    let before = records(&memory);
    assert!(before.contains_key("data/identity") && before.contains_key("data/head"));
    assert!(
        before
            .keys()
            .any(|name| name.starts_with("data/revisions/"))
    );
    let mut transaction = Some(transaction);
    let mut observed = BTreeMap::new();
    visit_memory_project(&memory, |name, bytes| {
        if let Some(transaction) = transaction.take() {
            assert_eq!(
                memory_project_head(&memory).unwrap().unwrap(),
                before["graph/head"]
            );
            created.repository.publish(&prepared.publication).unwrap();
            transaction.commit().unwrap();
        }
        assert!(observed.insert(name.to_owned(), bytes.to_vec()).is_none());
    })
    .unwrap();
    assert_eq!(
        observed, before,
        "callbacks see the pinned pre-publication snapshot"
    );
    let after = records(&memory);
    assert_ne!(after["graph/head"], before["graph/head"]);
    assert_ne!(after["data/head"], before["data/head"]);
    for (name, bytes) in &before {
        if name.starts_with("graph/objects/") || name.starts_with("data/revisions/") {
            assert_eq!(after.get(name), Some(bytes), "retained history omitted");
        }
    }
    let fork = memory.fork().unwrap();
    assert_eq!(records(&fork), after);
}

#[test]
fn memory_concurrent_publications_compare_and_swap_one_accepted_head() {
    let (_memory, created) = fixture(16 * 1_048_576);
    let left = rename(&created.repository, "left_wins", None);
    let right = rename(&created.repository, "right_wins", None);
    let barrier = std::sync::Barrier::new(2);
    let (left, right) = std::thread::scope(|scope| {
        let a = scope.spawn(|| {
            barrier.wait();
            created.repository.publish(&left.publication).unwrap()
        });
        let b = scope.spawn(|| {
            barrier.wait();
            created.repository.publish(&right.publication).unwrap()
        });
        (a.join().unwrap(), b.join().unwrap())
    });
    assert!(matches!(
        (&left, &right),
        (
            PublicationOutcome::Accepted { .. },
            PublicationOutcome::Stale { .. }
        ) | (
            PublicationOutcome::Stale { .. },
            PublicationOutcome::Accepted { .. }
        )
    ));
    let current = created.repository.current().unwrap();
    let accepted = match left {
        PublicationOutcome::Accepted { current, .. } => current,
        _ => match right {
            PublicationOutcome::Accepted { current, .. } => current,
            _ => unreachable!(),
        },
    };
    assert_eq!(current.head, accepted.head);
}

#[test]
fn memory_quota_and_missing_closure_reject_without_partial_authority_or_leaked_bytes() {
    let tiny = MemProjectStore::new(4_096).unwrap();
    let empty_bytes = tiny.used_bytes();
    assert_eq!(
        GraphRepository::create_memory(
            &tiny,
            &crate::platform::kernel::tests::witness_snapshot(),
            None
        )
        .unwrap_err()
        .code,
        "memory_project_capacity"
    );
    assert_eq!(tiny.used_bytes(), empty_bytes);
    assert!(GraphRepository::open_memory(&tiny).is_err());
    let (baseline, _) = fixture(16 * 1_048_576);
    let (memory, created) = fixture(baseline.used_bytes() + 1_024);
    let prepared = rename(&created.repository, "over_capacity", None);
    let before = memory.used_bytes();
    let error = created
        .repository
        .publish(&prepared.publication)
        .unwrap_err();
    assert_eq!(error.code, "memory_project_capacity");
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
    assert_eq!(memory.used_bytes(), before);

    let (memory, created) = fixture(16 * 1_048_576);
    let mut prepared = rename(&created.repository, "missing_object", None).publication;
    let key = prepared
        .objects
        .keys()
        .find(|key| key.domain == ObjectDomain::Owner)
        .copied()
        .unwrap();
    prepared.objects.remove(&key);
    let before = memory.used_bytes();
    assert!(created.repository.publish(&prepared).is_err());
    assert_eq!(
        created.repository.current().unwrap().head,
        created.current.head
    );
    assert_eq!(memory.used_bytes(), before);
}

#[test]
fn memory_and_disk_authority_compile_and_execute_identically() {
    let (_memory, memory) = fixture(32 * 1_048_576);
    let temporary = tempfile::tempdir().unwrap();
    let disk = GraphRepository::create(
        &temporary.path().join("disk"),
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    assert_eq!(memory.current.head, disk.current.head);
    for repository in [&memory.repository, &disk.repository] {
        let prepared = rename(repository, "same_accepted_edit", None);
        repository.publish(&prepared.publication).unwrap();
    }
    assert_eq!(
        memory.repository.current().unwrap().head,
        disk.repository.current().unwrap().head
    );
    let memory_program =
        crate::platform::normalized_lifecycle::prepare_repository(memory.repository.clone())
            .unwrap();
    let disk_program =
        crate::platform::normalized_lifecycle::prepare_repository(disk.repository).unwrap();
    assert_eq!(memory_program.artifact_bytes, disk_program.artifact_bytes);
    let checked = memory_program
        .check(&ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!(checked.failed, 0);
    assert_eq!(checked.passed, 1);
    let reused =
        crate::platform::normalized_lifecycle::prepare_repository(memory.repository.clone())
            .unwrap();
    assert_eq!(
        reused.cache_profile,
        crate::platform::normalized_lifecycle::LifecycleCacheProfile::ExactCurrent
    );
    assert_eq!(reused.artifact_bytes, memory_program.artifact_bytes);
    assert!(
        !memory.repository.root().exists(),
        "memory compilation created a filesystem repository"
    );
}

#[test]
fn memory_package_readiness_is_atomic_and_forked_with_complete_sources() {
    let (_source_memory, source) = fixture(16 * 1_048_576);
    let exported = source.repository.export_package_transport().unwrap();
    let mut target_snapshot = crate::platform::kernel::tests::witness_snapshot();
    // An empty independent consumer has no identities or edges from the producer.
    target_snapshot.owners.clear();
    target_snapshot.types.clear();
    target_snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        crate::platform::persistent_map::PageDigest::from_bytes([0; 32]),
        0,
        crate::platform::persistent_map::MapContentDigest::from_bytes([0; 32]),
    );
    target_snapshot.root.repository_id =
        crate::platform::semantic_id::RepositoryId::migrate(b"memory-consumer", 0);
    target_snapshot.root.package_id =
        crate::platform::kernel::PackageId::migrate(b"memory-consumer", 0);
    let memory = MemProjectStore::new(32 * 1_048_576).unwrap();
    let target = GraphRepository::create_memory_with_package_transports(
        &memory,
        &target_snapshot,
        None,
        &[InitialPackageTransport {
            digest: exported.transport_digest,
            container: exported.container.clone(),
        }],
    )
    .unwrap();
    let prepared = target
        .repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: target.current.head.revision,
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
    target.repository.publish(&prepared.publication).unwrap();
    let fork = memory.fork().unwrap();
    let fork = GraphRepository::open_memory(&fork).unwrap();
    let closure = fork.export_package_container().unwrap();
    assert_eq!(closure.packages.len(), 2);
    let before = fork.current().unwrap().head;
    let staged = fork
        .stage_package_transport(exported.transport_digest, &exported.container)
        .unwrap();
    assert_eq!(staged.outcome, StageOutcome::Reused);
    assert_eq!(fork.current().unwrap().head, before);
    assert_eq!(
        fork.staged_package_source(exported.revision_digest)
            .unwrap()
            .container
            .objects,
        source
            .repository
            .export_package_container()
            .unwrap()
            .container
            .objects
    );
}

#[test]
fn memory_reads_enforce_admission_before_copying_objects() {
    let (_memory, created) = fixture(16 * 1_048_576);
    let store = created.repository.object_store().unwrap();
    let (key, bytes) = created.initial.publication.objects.iter().next().unwrap();
    let mut work = StoreWork::default();
    let mut admission = StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: Some(1),
        maximum_objects: Some(1),
        maximum_bytes: Some((bytes.len() - 1) as u64),
    });
    assert!(
        store
            .read_admitted(*key, key.domain.maximum_bytes(), &mut admission, &mut work)
            .is_err()
    );
    assert_eq!(work.objects_read, 0);
    assert_eq!(work.bytes_read, 0);
    assert_eq!(
        store
            .read(*key, bytes.len(), &mut StoreWork::default())
            .unwrap()
            .as_ref(),
        Some(bytes)
    );
}
