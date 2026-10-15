use super::*;
use crate::platform::kernel::Name;
use crate::platform::publication::GraphRepository;
use std::cell::Cell;

fn fixture(name: &str) -> (tempfile::TempDir, AdmittedClosure) {
    let temporary = tempfile::tempdir().unwrap();
    let mut snapshot = crate::platform::kernel::tests::transport_snapshot();
    snapshot.root.package_name = Name::new(name).unwrap();
    let repository = GraphRepository::create(&temporary.path().join("source"), &snapshot, None)
        .unwrap()
        .repository;
    let closure = repository.export_package_container().unwrap();
    (temporary, closure)
}

fn compiled(closure: &AdmittedClosure, dependencies: &[LoadedArtifact]) -> PreparedSupplier {
    let package = &closure.packages[&closure.container.root.package_revision];
    let linked = compile_immutable(package, &closure.container.objects, dependencies).unwrap();
    PreparedSupplier {
        artifact: Arc::new(load_artifact(&linked.artifact.bytes).unwrap()),
        reused: false,
    }
}

fn key(closure: &AdmittedClosure) -> Key {
    Key::admitted(
        &closure.packages[&closure.container.root.package_revision],
        &[],
        &ExecutionControl::uncancelled(),
    )
    .unwrap()
}

fn dependent_fixture(source: &AdmittedClosure) -> (tempfile::TempDir, AdmittedClosure) {
    use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
    use crate::platform::project_creation::{ProjectTemplate, create_project};
    use crate::platform::publication::PublicationOptions;
    let temporary = tempfile::tempdir().unwrap();
    let path = temporary.path().join("dependent");
    create_project(&path, "dependent", ProjectTemplate::Minimal).unwrap();
    let repository = GraphRepository::open(&path).unwrap();
    repository
        .stage_package_transport(
            source.container.root.transport,
            &source.container.encode().unwrap(),
        )
        .unwrap();
    let package = &source.packages[&source.container.root.package_revision];
    let prepared = repository
        .prepare_authored_change(
            &AuthoredChangeSet {
                base: repository.current().unwrap().head.revision,
                preconditions: Vec::new(),
                budget: ChangeBudget::default(),
                changes: vec![AuthoredChange::AddDependency {
                    package: package.revision.package,
                    semantic_revision: package.revision.revision.revision_id().unwrap(),
                    package_revision: source.container.root.package_revision,
                }],
            },
            PublicationOptions::default(),
        )
        .unwrap();
    repository.publish(&prepared.publication).unwrap();
    (temporary, repository.export_package_container().unwrap())
}

#[test]
fn cold_and_shared_supplier_outputs_match_an_independent_clean_compilation() {
    let (_temporary, closure) = fixture("shared-supplier");
    let original = closure.container.encode().unwrap();
    let expected = compiled(&closure, &[]).artifact;
    let pool = Pool::new(4, MAXIMUM_RETAINED_BYTES);
    let calls = Cell::new(0);
    let make = || {
        calls.set(calls.get() + 1);
        Ok(compiled(&closure, &[]))
    };
    let first = pool
        .prepare(key(&closure), &ExecutionControl::uncancelled(), make)
        .unwrap();
    let second = pool
        .prepare(key(&closure), &ExecutionControl::uncancelled(), make)
        .unwrap();
    assert!(!first.reused);
    assert!(second.reused);
    assert_eq!(calls.get(), 1);
    assert!(Arc::ptr_eq(&first.artifact, &second.artifact));
    assert_eq!(first.artifact.manifest, expected.manifest);
    assert_eq!(first.artifact.bundle_digest, expected.bundle_digest);
    assert_eq!(first.artifact.objects, expected.objects);
    assert_eq!(closure.container.encode().unwrap(), original);
}

#[test]
fn a_warm_supplier_cannot_hide_new_source_or_dependency_corruption() {
    let (_temporary, closure) = fixture("supplier-tamper");
    let revision = closure.container.root.package_revision;
    let healthy = ExecutionControl::uncancelled();
    let baseline = prepare(&closure, revision, &[], &healthy).unwrap();
    let warm = prepare(&closure, revision, &[], &healthy).unwrap();
    assert!(warm.reused);
    assert!(Arc::ptr_eq(&baseline.artifact, &warm.artifact));

    // The normal caller performs this complete admission before every cache lookup.
    // Corrupting a private/unused source object cannot be hidden by the warm compiler output.
    let mut damaged_source = closure.container.clone();
    let bytes = damaged_source
        .objects
        .values_mut()
        .find(|bytes| !bytes.is_empty())
        .unwrap();
    bytes[0] ^= 1;
    assert!(damaged_source.admit().is_err());

    let mut damaged_binding = closure.clone();
    damaged_binding
        .packages
        .get_mut(&revision)
        .unwrap()
        .revision
        .interface = crate::platform::kernel::PackageInterfaceDigest::from_bytes([0; 32]);
    assert_eq!(
        prepare(&damaged_binding, revision, &[], &healthy)
            .unwrap_err()
            .code,
        "lifecycle_supplier_cache_binding"
    );

    let mut damaged_dependency = (*baseline.artifact).clone();
    let bytes = damaged_dependency
        .objects
        .values_mut()
        .find(|bytes| !bytes.is_empty())
        .unwrap();
    bytes[0] ^= 1;
    assert_eq!(
        prepare(&closure, revision, &[damaged_dependency], &healthy)
            .unwrap_err()
            .code,
        "artifact_current_admission_binding"
    );
    let after = prepare(&closure, revision, &[], &healthy).unwrap();
    assert!(after.reused);
    assert_eq!(after.artifact.objects, baseline.artifact.objects);
}

#[test]
fn a_different_complete_dependency_artifact_cannot_reuse_an_unrelated_verdict() {
    let (_first_root, first) = fixture("supplier-original-dependency");
    let (_second_root, second) = fixture("supplier-different-dependency");
    let healthy = ExecutionControl::uncancelled();
    let revision = first.container.root.package_revision;
    let baseline = prepare(&first, revision, &[], &healthy).unwrap();
    let unrelated = compiled(&second, &[]).artifact;
    let selected = Key::admitted(
        &first.packages[&revision],
        std::slice::from_ref(unrelated.as_ref()),
        &healthy,
    )
    .unwrap();
    assert_ne!(key(&first), selected);
    // The ordinary strict compiler/linker must reject this foreign package revision. A
    // warm no-dependency cache entry cannot turn it into the earlier successful artifact.
    assert!(prepare(&first, revision, &[(*unrelated).clone()], &healthy).is_err());
    assert_eq!(
        prepare(&first, revision, &[], &healthy)
            .unwrap()
            .artifact
            .objects,
        baseline.artifact.objects
    );
}

#[test]
fn forged_dependency_digest_labels_cannot_replace_verified_artifact_content() {
    let (_first_root, first) = fixture("supplier-label-original");
    let (_second_root, second) = fixture("supplier-label-foreign");
    let (_dependent_root, dependent) = dependent_fixture(&first);
    let original = compiled(&first, &[]).artifact;
    let mut forged = (*compiled(&second, &[]).artifact).clone();
    let healthy = ExecutionControl::uncancelled();
    forged.manifest_digest = original.manifest_digest;
    forged.bundle_digest = original.bundle_digest;
    // Both are independently admitted, but their actual manifests and source objects differ.
    assert_ne!(
        original.admitted_content_digest(|| Ok(())).unwrap(),
        forged.admitted_content_digest(|| Ok(())).unwrap()
    );
    let revision = dependent.container.root.package_revision;
    let baseline = prepare(
        &dependent,
        revision,
        std::slice::from_ref(original.as_ref()),
        &healthy,
    )
    .unwrap();
    assert!(prepare(&dependent, revision, &[forged], &healthy).is_err());
    let after = prepare(
        &dependent,
        revision,
        std::slice::from_ref(original.as_ref()),
        &healthy,
    )
    .unwrap();
    assert!(after.reused);
    assert!(Arc::ptr_eq(&after.artifact, &baseline.artifact));
}

#[test]
fn schema_validator_transport_and_ordered_dependencies_partition_compiler_reuse() {
    let (_temporary, closure) = fixture("supplier-key-context");
    let base = key(&closure);
    let mut schema = base.clone();
    schema.schema[0] ^= 1;
    let mut validator = base.clone();
    validator.validator[0] ^= 1;
    let mut transport = base.clone();
    transport.transport[0] ^= 1;
    let mut left = base.clone();
    left.dependencies = vec![
        DependencyKey {
            manifest: [1; 32],
            bundle: [2; 32],
            admitted_content: [3; 32],
        },
        DependencyKey {
            manifest: [4; 32],
            bundle: [5; 32],
            admitted_content: [6; 32],
        },
    ];
    let mut right = base.clone();
    right.dependencies = left.dependencies.iter().rev().cloned().collect();
    let pool = Pool::new(8, MAXIMUM_RETAINED_BYTES);
    let artifact = compiled(&closure, &[]).artifact;
    let calls = Cell::new(0);
    for binding in [base, schema, validator, transport, left, right] {
        let first = pool
            .prepare(binding.clone(), &ExecutionControl::uncancelled(), || {
                calls.set(calls.get() + 1);
                Ok(PreparedSupplier {
                    artifact: Arc::clone(&artifact),
                    reused: false,
                })
            })
            .unwrap();
        assert!(!first.reused);
        let second = pool
            .prepare(binding, &ExecutionControl::uncancelled(), || unreachable!())
            .unwrap();
        assert!(second.reused);
    }
    assert_eq!(calls.get(), 6);
}

#[test]
fn cancelled_or_failed_preparations_leave_no_retained_prefix_and_allow_retry() {
    let (_temporary, closure) = fixture("supplier-cancel-retry");
    let pool = Pool::new(2, MAXIMUM_RETAINED_BYTES);
    let binding = key(&closure);
    let healthy = ExecutionControl::uncancelled();
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        pool.prepare(binding.clone(), &cancelled, || unreachable!())
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert!(
        pool.prepare(binding.clone(), &healthy, || Err(binding_error()))
            .is_err()
    );
    let during = ExecutionControl::uncancelled();
    assert_eq!(
        pool.prepare(binding.clone(), &during, || {
            let prepared = compiled(&closure, &[]);
            during.cancel();
            Ok(prepared)
        })
        .unwrap_err()
        .class,
        DiagnosticClass::Cancelled
    );
    assert!(pool.lock().unwrap().entries.is_empty());
    assert_eq!(pool.lock().unwrap().retained_bytes, 0);
    let retried = pool
        .prepare(binding.clone(), &healthy, || Ok(compiled(&closure, &[])))
        .unwrap();
    assert!(!retried.reused);
    assert!(
        pool.prepare(binding, &healthy, || unreachable!())
            .unwrap()
            .reused
    );
}

#[test]
fn a_cancelled_waiter_does_not_cancel_or_duplicate_another_invocations_preparation() {
    let (_temporary, closure) = fixture("supplier-cancel-waiter");
    let binding = key(&closure);
    let pool = Arc::new(Pool::new(2, MAXIMUM_RETAINED_BYTES));
    let artifact = compiled(&closure, &[]).artifact;
    let (started, waiting) = std::sync::mpsc::channel();
    let (release, released) = std::sync::mpsc::channel();
    let owner_pool = Arc::clone(&pool);
    let owner_binding = binding.clone();
    let owner = std::thread::spawn(move || {
        owner_pool.prepare(owner_binding, &ExecutionControl::uncancelled(), || {
            started.send(()).unwrap();
            released.recv_timeout(Duration::from_secs(5)).unwrap();
            Ok(PreparedSupplier {
                artifact,
                reused: false,
            })
        })
    });
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let cancelled = pool
        .prepare(
            binding.clone(),
            &ExecutionControl::cancel_after_checks(1),
            || unreachable!(),
        )
        .unwrap_err();
    assert_eq!(cancelled.class, DiagnosticClass::Cancelled);
    assert_eq!(pool.lock().unwrap().entries.len(), 1);
    release.send(()).unwrap();
    let prepared = owner.join().unwrap().unwrap();
    let reused = pool
        .prepare(binding, &ExecutionControl::uncancelled(), || unreachable!())
        .unwrap();
    assert!(reused.reused);
    assert!(Arc::ptr_eq(&prepared.artifact, &reused.artifact));
}

#[test]
fn bounded_capacity_evicts_or_bypasses_without_rejecting_valid_compilation() {
    let (_temporary, closure) = fixture("supplier-finite-capacity");
    let binding = key(&closure);
    let artifact = compiled(&closure, &[]).artifact;
    let bytes = retained_bytes(&binding, &artifact).unwrap();
    let pool = Pool::new(1, bytes);
    for ordinal in 0..4 {
        let mut next = binding.clone();
        next.schema[0] ^= ordinal;
        let result = pool
            .prepare(next, &ExecutionControl::uncancelled(), || {
                Ok(PreparedSupplier {
                    artifact: Arc::clone(&artifact),
                    reused: false,
                })
            })
            .unwrap();
        assert!(!result.reused);
        let state = pool.lock().unwrap();
        assert_eq!(state.entries.len(), 1);
        assert!(state.retained_bytes <= bytes);
    }
    let tiny = Pool::new(1, bytes - 1);
    for _ in 0..2 {
        assert!(
            !tiny
                .prepare(binding.clone(), &ExecutionControl::uncancelled(), || Ok(
                    PreparedSupplier {
                        artifact: Arc::clone(&artifact),
                        reused: false
                    }
                ))
                .unwrap()
                .reused
        );
        assert!(tiny.lock().unwrap().entries.is_empty());
        assert_eq!(tiny.lock().unwrap().retained_bytes, 0);
    }
}

#[test]
fn exact_builtin_code_is_reused_only_after_comparing_its_current_source_bytes() {
    let standard = crate::platform::builtin_standard::BuiltinStandard::load().unwrap();
    let container =
        PackageContainer::decode(standard.transport_bytes(), standard.package_transport).unwrap();
    let closure = container.admit().unwrap();
    let healthy = ExecutionControl::uncancelled();
    let cached = prepare(&closure, standard.package_revision, &[], &healthy).unwrap();
    assert!(cached.reused);
    assert_eq!(cached.artifact.manifest, standard.artifact.manifest);
    assert_eq!(
        cached.artifact.bundle_digest,
        standard.artifact.bundle_digest
    );
    assert_eq!(cached.artifact.objects, standard.artifact.objects);

    let mut damaged = closure.clone();
    damaged
        .container
        .objects
        .values_mut()
        .find(|bytes| !bytes.is_empty())
        .unwrap()[0] ^= 1;
    assert_eq!(
        embedded_standard(
            &damaged.packages[&standard.package_revision],
            &damaged,
            &[],
            &healthy
        )
        .unwrap_err()
        .code,
        "lifecycle_supplier_cache_binding"
    );
    assert!(damaged.container.admit().is_err());
}

#[test]
fn a_published_slots_drop_cannot_remove_a_new_owner_after_concurrent_eviction() {
    let (_temporary, closure) = fixture("supplier-slot-custody");
    let binding = key(&closure);
    let artifact = compiled(&closure, &[]).artifact;
    let pool = Pool::new(1, MAXIMUM_RETAINED_BYTES);
    let reserved_bytes = key_retained_bytes(&binding).unwrap();
    {
        let mut state = pool.lock().unwrap();
        state.entries.push_back(Entry {
            key: binding.clone(),
            value: None,
            reserved_bytes,
        });
        state.retained_bytes = reserved_bytes;
    }
    let mut completed = Slot {
        pool: &pool,
        key: binding.clone(),
        published: false,
    };
    completed
        .publish(&artifact, &ExecutionControl::uncancelled())
        .unwrap();
    {
        let mut state = pool.lock().unwrap();
        assert!(evict_ready(&mut state));
        state.entries.push_back(Entry {
            key: binding.clone(),
            value: None,
            reserved_bytes,
        });
        state.retained_bytes = reserved_bytes;
    }
    let replacement = Slot {
        pool: &pool,
        key: binding,
        published: false,
    };
    drop(completed);
    assert_eq!(pool.lock().unwrap().entries.len(), 1);
    assert!(pool.lock().unwrap().entries[0].value.is_none());
    drop(replacement);
    assert!(pool.lock().unwrap().entries.is_empty());
    assert_eq!(pool.lock().unwrap().retained_bytes, 0);
}
