use super::*;
use crate::platform::change::{AuthoredChange, ChangeBudget, OwnerSelector};
use crate::platform::compiler::OptimizationPolicy;

fn rename(repository: &GraphRepository, name: &str, key: &str) -> PreparedAuthoredPublication {
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
            idempotency_key: Some(key.into()),
            intent: None,
        },
    )
    .unwrap()
}

fn history(repository: &GraphRepository) -> Vec<(HeadRecord, Vec<u8>)> {
    let history = repository.history(100).unwrap();
    history
        .entries
        .into_iter()
        .map(|entry| {
            (
                HeadRecord {
                    record: entry.record,
                    revision: entry.revision.revision,
                    ..history.head
                },
                entry.receipt.encode().unwrap().1,
            )
        })
        .collect()
}

#[test]
fn disk_gc_retains_history_idempotency_compilation_and_exact_meaning() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let created = GraphRepository::create(
        &root,
        &crate::platform::kernel::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let repository = created.repository;
    let first = rename(&repository, "first_gc", "gc-first");
    repository.publish(&first.publication).unwrap();
    let second = rename(&repository, "second_gc", "gc-second");
    repository.publish(&second.publication).unwrap();
    let compiled = crate::platform::compiler::build_clean(
        &repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let before = repository.current().unwrap();
    let snapshot = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let recorded = history(&repository);
    let orphan = vec![17; 65_536];
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &orphan);
    let mut store = repository.object_store().unwrap();
    store
        .stage(key, &orphan, &mut StoreWork::default())
        .unwrap();
    store
        .seal_staged(TARGET_PACK_BYTES, &mut StoreWork::default())
        .unwrap();
    drop(store);
    let pinned = repository.view_current().unwrap();
    assert_eq!(
        repository
            .collect_garbage(&ExecutionControl::uncancelled())
            .unwrap_err()
            .code,
        "gc_readers_active"
    );
    assert_eq!(
        pinned.reconstruct_full_oracle().unwrap().value.owners,
        snapshot.owners
    );
    drop(pinned);
    let receipt = repository
        .collect_garbage(&ExecutionControl::uncancelled())
        .unwrap();
    assert!(receipt.objects_reclaimed > 0);
    assert_eq!(receipt.disk.unwrap().packs_after, 1);
    assert_eq!(repository.current().unwrap().head, before.head);
    assert_eq!(history(&repository), recorded);
    assert_eq!(
        repository
            .view_current()
            .unwrap()
            .reconstruct_full_oracle()
            .unwrap()
            .value
            .owners,
        snapshot.owners
    );
    assert_eq!(
        crate::platform::compiler::load_current_compilation(&repository)
            .unwrap()
            .unwrap()
            .digest,
        compiled.manifest_digest
    );
    let PublicationOutcome::AlreadyAccepted { accepted, observed } =
        repository.publish(&first.publication).unwrap()
    else {
        panic!("historical idempotency must retain the original acceptance");
    };
    assert_eq!(accepted.head, first.publication.head);
    assert_eq!(accepted.receipt, first.publication.receipt);
    assert_eq!(observed, before.head);
    assert!(
        repository
            .object_store()
            .unwrap()
            .read(key, orphan.len(), &mut StoreWork::default())
            .unwrap()
            .is_none()
    );
    let again = repository
        .collect_garbage(&ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!(again.disk.unwrap().pack_bytes_reclaimed, 0);
}

#[test]
fn memory_gc_retains_readers_dependencies_and_independent_execution() {
    let memory = MemProjectStore::new(512 * 1_048_576).unwrap();
    crate::platform::create_memory_project(&memory, "gc-command", "command").unwrap();
    let repository = GraphRepository::open_memory(&memory).unwrap();
    crate::platform::execute_in_memory(&memory, vec!["check".into()]).unwrap();
    let pinned = repository.view_current().unwrap();
    let before = pinned.reconstruct_full_oracle().unwrap().value;
    let readiness = read_store_readiness(&repository.object_store().unwrap()).unwrap();
    assert!(!readiness.bindings.is_empty());
    let orphan = vec![31; 32_768];
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, &orphan);
    memory_lock(&memory.graph)
        .unwrap()
        .store
        .objects
        .insert(key, &orphan)
        .unwrap();
    let receipt = repository
        .collect_garbage(&ExecutionControl::uncancelled())
        .unwrap();
    assert!(receipt.objects_reclaimed > 0);
    assert!(receipt.disk.is_none());
    assert_eq!(
        pinned.reconstruct_full_oracle().unwrap().value.owners,
        before.owners
    );
    assert_eq!(
        read_store_readiness(&repository.object_store().unwrap()).unwrap(),
        readiness
    );
    crate::platform::execute_in_memory(&memory, vec!["check".into()]).unwrap();
    let result =
        crate::platform::execute_in_memory(&memory, vec!["run".into(), "main".into()]).unwrap();
    assert!(String::from_utf8(result).unwrap().contains("hello"));
    assert!(
        repository
            .object_store()
            .unwrap()
            .read(key, orphan.len(), &mut StoreWork::default())
            .unwrap()
            .is_none()
    );
}

#[test]
fn corrupt_missing_and_cancelled_reachability_never_changes_publication_or_storage() {
    let memory = MemProjectStore::new(32 * 1_048_576).unwrap();
    crate::platform::create_memory_project(&memory, "gc-corruption", "minimal").unwrap();
    let repository = GraphRepository::open_memory(&memory).unwrap();
    let root = repository
        .current()
        .unwrap()
        .revision
        .publication
        .semantic_root;
    let key = ObjectKey::from_digest(ObjectDomain::SemanticRoot, root.bytes());
    let healthy = memory_lock(&memory.graph)
        .unwrap()
        .store
        .objects
        .get(&key)
        .unwrap()
        .as_slice()
        .to_vec();
    for malformed in [true, false] {
        let mut graph = memory_lock(&memory.graph).unwrap();
        if malformed {
            graph.store.objects.insert(key, b"corrupt").unwrap();
        } else {
            graph
                .store
                .objects
                .retain(|candidate| *candidate != key)
                .unwrap();
        }
        drop(graph);
        let before = memory.test_inventory().unwrap();
        assert!(
            repository
                .collect_garbage(&ExecutionControl::uncancelled())
                .is_err()
        );
        assert_eq!(memory.test_inventory().unwrap(), before);
        memory_lock(&memory.graph)
            .unwrap()
            .store
            .objects
            .insert(key, &healthy)
            .unwrap();
    }
    let before = memory.test_inventory().unwrap();
    let error = repository
        .collect_garbage(&ExecutionControl::cancel_after_checks(3))
        .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Cancelled);
    assert_eq!(memory.test_inventory().unwrap(), before);
}

#[test]
fn inferred_parallel_types_are_reconstructed_metadata_not_missing_storage_roots() {
    let memory = MemProjectStore::new(512 * 1_048_576).unwrap();
    crate::platform::create_memory_project(&memory, "gc-inferred", "command").unwrap();
    let proposal = r#"(tx (base @head)
      (upsert-module parallel_gc
        (upsert-function left (returns I64) (effect (task)) (body (i64 11)))
        (upsert-function right (returns I64) (effect (task)) (body (i64 29)))
        (upsert-function pick (returns I64) (effect (task))
          (body (field (parallel (call left) (call right)) (name left))))))"#;
    crate::platform::execute_in_memory(
        &memory,
        vec!["tx".into(), "--input".into(), proposal.into()],
    )
    .unwrap();
    let repository = GraphRepository::open_memory(&memory).unwrap();
    let compiled = crate::platform::compiler::build_clean(
        &repository,
        OptimizationPolicy::DeterministicBaseline,
    )
    .unwrap();
    let store = repository.object_store().unwrap();
    let control = ExecutionControl::uncancelled();
    let selected = Retained::new(&store, &control);
    let mut inferred = BTreeSet::new();
    selected
        .map(compiled.manifest.units, |key, value| {
            let binding = CompilationBinding::decode(value, EncodedOwnerKey::decode(key)?)?;
            let unit = CompilationUnit::decode(
                &selected.required(ObjectDomain::CompilerUnit, binding.object.bytes())?,
                binding.object.object_key(),
            )?;
            for digest in unit.tables.types {
                if !store
                    .contains(
                        ObjectKey::from_digest(ObjectDomain::Type, digest.bytes()),
                        &mut StoreWork::default(),
                    )
                    .unwrap()
                {
                    inferred.insert(digest);
                }
            }
            Ok(())
        })
        .unwrap();
    assert!(
        !inferred.is_empty(),
        "fixture must exercise types absent from physical storage"
    );
    repository
        .collect_garbage(&ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!(
        crate::platform::compiler::load_current_compilation(&repository)
            .unwrap()
            .unwrap()
            .digest,
        compiled.manifest_digest
    );
    crate::platform::execute_in_memory(&memory, vec!["check".into()]).unwrap();
}

#[test]
fn gc_reference_queue_crosses_former_million_entry_ceiling_without_duplicate_visits() {
    let mut queue = Queue::default();
    for index in 0_u64..1_000_001 {
        let mut bytes = [0; 32];
        bytes[..8].copy_from_slice(&index.to_be_bytes());
        let item = Pending::Type(TypeObjectDigest::from_bytes(bytes));
        queue.push(item).unwrap();
        queue.push(item).unwrap();
    }
    assert_eq!(queue.seen.len(), 1_000_001);
    assert_eq!(queue.pending.len(), 1_000_001);
}
