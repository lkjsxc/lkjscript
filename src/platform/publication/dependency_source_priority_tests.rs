use super::*;
use crate::platform::kernel::{Name, PackageRevisionDigest};
use crate::platform::package_transport::source::PackageReadiness;
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::semantic_id::{RepositoryId, RevisionId};
use std::cell::RefCell;

std::thread_local! {
    static COLLECTIONS: RefCell<Option<Vec<PackageRevisionDigest>>> = const { RefCell::new(None) };
}

pub(super) fn record_collection(revision: PackageRevisionDigest) {
    COLLECTIONS.with(|calls| {
        if let Some(calls) = calls.borrow_mut().as_mut() {
            calls.push(revision);
        }
    });
}

fn begin_collections() {
    COLLECTIONS.with(|calls| *calls.borrow_mut() = Some(Vec::new()));
}

fn collections() -> Vec<PackageRevisionDigest> {
    COLLECTIONS.with(|calls| calls.borrow_mut().take().unwrap_or_default())
}

fn dependency(byte: u8) -> DependencyRecord {
    DependencyRecord {
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package: PackageId::from_bytes([byte; 16]).unwrap(),
        semantic_revision: RevisionId::from_digest([byte; 32]),
        package_revision: PackageRevisionDigest::from_bytes([byte; 32]),
    }
}

fn ordered(direct: &[DependencyRecord], incoming: &[DependencyRecord]) -> (Vec<usize>, u64) {
    let objects = BTreeMap::<crate::platform::storage::object::ObjectKey, Vec<u8>>::new();
    let store = CollectingStore::new(&objects);
    let control = ExecutionControl::uncancelled();
    let mut work = DependencySourcePriorityWork::new(&store, &|| Ok(()), &control);
    let mut priority = DependencySourcePriority::new(direct, &mut work).unwrap();
    for child in incoming {
        priority
            .observe_dependency(child, direct, &mut work)
            .unwrap();
    }
    let order = priority.into_order(&mut work).unwrap();
    (order, work.consumed())
}

#[test]
fn dependency_source_priority_preserves_canonical_ties_and_all_cycle_fallback_roots() {
    let direct = [dependency(1), dependency(2), dependency(3), dependency(4)];
    assert_eq!(
        ordered(&direct, &[direct[0].clone(), direct[1].clone()]).0,
        [2, 3, 0, 1]
    );
    // Every root is mentioned in a cycle-shaped metadata input. Priority proves neither
    // acyclicity nor coverage; retain all roots in original order for native admission.
    assert_eq!(ordered(&direct, &direct).0, [0, 1, 2, 3]);
    assert!(ordered(&[], &[]).0.is_empty());
    let order = ordered(&direct, &[direct[1].clone()]).0;
    let mut covered = BTreeSet::new();
    let mut attempted = Vec::new();
    for index in order {
        if !covered.contains(&direct[index].package_revision) {
            attempted.push(index);
            // Simulate only this root's own successful complete coverage. Metadata
            // classification by itself can never suppress the final uncovered root.
            covered.insert(direct[index].package_revision);
        }
    }
    assert_eq!(attempted, [0, 2, 3, 1]);
}

#[test]
fn dependency_source_priority_requires_the_complete_exact_descendant_record() {
    let direct = [dependency(1), dependency(2)];
    for field in 0..3 {
        let mut wrong = direct[0].clone();
        match field {
            0 => wrong.graph_contract_version -= 1,
            1 => wrong.semantic_revision = RevisionId::from_digest([0x81; 32]),
            _ => wrong.package_revision = PackageRevisionDigest::from_bytes([0x82; 32]),
        }
        assert_eq!(ordered(&direct, &[wrong]).0, [0, 1]);
    }
    assert_eq!(ordered(&direct, &[direct[0].clone()]).0, [1, 0]);
}

#[test]
fn dependency_source_priority_reserves_shared_capacity_and_actual_control_before_growth() {
    let direct = [dependency(1), dependency(2), dependency(3)];
    let (_, required) = ordered(&direct, &[direct[0].clone()]);
    assert!(required > 0);
    let objects = BTreeMap::<crate::platform::storage::object::ObjectKey, Vec<u8>>::new();
    let store = CollectingStore::new(&objects);
    for (earlier, succeeds) in [
        (MAXIMUM_VALIDATION_VISITS - required, true),
        (MAXIMUM_VALIDATION_VISITS - required + 1, false),
    ] {
        let control = ExecutionControl::uncancelled();
        let mut work = DependencySourcePriorityWork::new(&store, &|| Ok(()), &control);
        work.consumed = earlier;
        let result = DependencySourcePriority::new(&direct, &mut work).and_then(|mut priority| {
            priority.observe_dependency(&direct[0], &direct, &mut work)?;
            priority.into_order(&mut work)
        });
        assert_eq!(result.is_ok(), succeeds);
        if let Err(error) = result {
            assert_eq!(error.class, DiagnosticClass::Resource);
        }
        assert!(work.consumed() <= MAXIMUM_VALIDATION_VISITS);
    }
    let control = ExecutionControl::uncancelled();
    control.cancel();
    let mut work = DependencySourcePriorityWork::new(&store, &|| Ok(()), &control);
    assert_eq!(
        DependencySourcePriority::new(&direct, &mut work)
            .err()
            .unwrap()
            .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(work.consumed(), 0);
}

fn empty_source(package: PackageId, seed: &[u8]) -> KernelSnapshot {
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: package,
            package_name: Name::new("priority_fixture").unwrap(),
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
    }
}

fn create_source(
    directory: &Path,
    name: &str,
    package: PackageId,
    imports: &[&super::super::super::ExportedPackageTransport],
) -> (
    GraphRepository,
    super::super::super::ExportedPackageTransport,
) {
    use crate::platform::change::{AuthoredChange, ChangeBudget};
    let created = GraphRepository::create(
        &directory.join(name),
        &empty_source(package, name.as_bytes()),
        None,
    )
    .unwrap();
    if !imports.is_empty() {
        let mut changes = Vec::new();
        for imported in imports {
            created
                .repository
                .stage_package_transport(imported.transport_digest, &imported.container)
                .unwrap();
            changes.push(AuthoredChange::AddDependency {
                package: imported.revision.package,
                semantic_revision: imported.revision.revision.revision_id().unwrap(),
                package_revision: imported.revision_digest,
            });
        }
        let prepared = created
            .repository
            .prepare_authored_change(
                &AuthoredChangeSet {
                    base: created.current.head.revision,
                    preconditions: Vec::new(),
                    budget: ChangeBudget::default(),
                    changes,
                },
                PublicationOptions::default(),
            )
            .unwrap();
        created.repository.publish(&prepared.publication).unwrap();
    }
    let exported = created.repository.export_package_transport().unwrap();
    (created.repository, exported)
}

#[test]
fn dependency_source_priority_admits_two_maximal_roots_for_the_nine_supplier_overlap_shape() {
    // This five-direct-root fixture has nine suppliers and reproduces the prior
    // five source collections of sizes 2/3/4/4/8. The real planner has nine direct
    // roots; its four other direct suppliers were already covered.
    // Fresh owned empty sources exercise the product, not copied proofs.
    let directory = tempfile::tempdir().unwrap();
    let pkg = |id: &str| id.parse::<PackageId>().unwrap();
    let p = |raw: &str| pkg(&format!("pkg_{raw}"));
    let (_, standard) = create_source(
        directory.path(),
        "std",
        p("10000000000000000000000000000001"),
        &[],
    );
    let (_, read) = create_source(
        directory.path(),
        "read",
        p("0715b5ff15f245ce3bb0f8bfc254e338"),
        &[&standard],
    );
    let (_, base) = create_source(
        directory.path(),
        "base",
        p("64743b873112737840db706fc6107a1b"),
        &[&standard],
    );
    let (_, writer) = create_source(
        directory.path(),
        "writer",
        p("0dba40dfd321184dd3f5f962c65f062b"),
        &[&base],
    );
    let (_, family) = create_source(
        directory.path(),
        "family",
        p("83566b3778f52f9d2c248e0bc66613ac"),
        &[&standard],
    );
    let (_, relay) = create_source(
        directory.path(),
        "relay",
        p("7f2c7a229f1a9f475f0b208e6a40a323"),
        &[&base, &family],
    );
    let (_, event) = create_source(
        directory.path(),
        "event",
        p("da6c35a66052e6eb317abd2a0ad0c8ff"),
        &[&standard],
    );
    let (_, state) = create_source(
        directory.path(),
        "state",
        p("cd6a4dc19d5bdd6eec42eca1054df235"),
        &[&read, &event],
    );
    let (_, planner) = create_source(
        directory.path(),
        "planner",
        p("e3f8cd073b6666a47808b3bc43721c8e"),
        &[&read, &writer, &relay, &event],
    );
    let (target, _) = create_source(
        directory.path(),
        "target",
        PackageId::from_bytes([0xf1; 16]).unwrap(),
        &[&read, &writer, &relay, &state, &planner],
    );
    let before_head = fs::read(target.root().join("HEAD")).unwrap();
    let before_ready = fs::read(target.root().join("PACKAGE-TRANSPORTS/CURRENT")).unwrap();
    let current = target.current().unwrap();
    assert_eq!(current.semantic_root.dependencies.entries(), 5);
    let store = target.object_store().unwrap();
    begin_collections();
    let observed = super::super::validate_dependency_sources_controlled(
        &store,
        &store,
        &current.semantic_root,
        &|| Ok(()),
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert!(observed.0 <= MAXIMUM_VALIDATION_VISITS);
    assert!(!observed.1);
    assert_eq!(
        collections(),
        [state.revision_digest, planner.revision_digest]
    );
    let exact = target.export_package_container().unwrap();
    assert_eq!(exact.packages.len(), 10);
    let independent =
        crate::platform::package_transport::oracle::reconstruct(&exact.container).unwrap();
    for source in [
        &standard, &read, &base, &writer, &family, &relay, &event, &state, &planner,
    ] {
        assert!(exact.packages.contains_key(&source.revision_digest));
        assert_eq!(
            exact.packages[&source.revision_digest].revision.package,
            source.revision.package
        );
        assert!(independent.snapshots.contains_key(&source.revision.package));
    }
    assert_eq!(fs::read(target.root().join("HEAD")).unwrap(), before_head);
    assert_eq!(
        fs::read(target.root().join("PACKAGE-TRANSPORTS/CURRENT")).unwrap(),
        before_ready
    );
}

#[test]
fn dependency_source_priority_never_hides_covered_descendant_source_or_readiness_absence() {
    use crate::platform::storage::object::{
        ImmutableObjectStore, ObjectKey, StageOutcome, StoreError, StoreErrorClass,
        StoreReadAdmission, StoreWork,
    };
    struct Hide<'a> {
        base: &'a StorageBackend,
        key: ObjectKey,
    }
    impl ImmutableObjectStore for Hide<'_> {
        fn read(
            &self,
            key: ObjectKey,
            maximum: Option<usize>,
            work: &mut StoreWork,
        ) -> Result<Option<Vec<u8>>, StoreError> {
            self.read_admitted(key, maximum, &mut StoreReadAdmission::unbounded(), work)
        }
        fn read_admitted(
            &self,
            key: ObjectKey,
            maximum: Option<usize>,
            admission: &mut StoreReadAdmission,
            work: &mut StoreWork,
        ) -> Result<Option<Vec<u8>>, StoreError> {
            let value = self.base.read_admitted(key, maximum, admission, work)?;
            Ok(if key == self.key { None } else { value })
        }
        fn stage(
            &mut self,
            _: ObjectKey,
            _: &[u8],
            _: &mut StoreWork,
        ) -> Result<StageOutcome, StoreError> {
            Err(StoreError::new(
                StoreErrorClass::Input,
                "test_source_read_only",
                "read only",
            ))
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create body (function create choose (visibility private) (parameter create flag (type Bool)) (returns I64) (effect pure) (body (if (local flag) (i64 17) (i64 23))))))\ndeclarations.end\n").unwrap();
    let leaf = GraphRepository::create(&directory.path().join("leaf"), &snapshot, None)
        .unwrap()
        .repository
        .export_package_transport()
        .unwrap();
    let (_, wrapper) = create_source(
        directory.path(),
        "wrapper",
        PackageId::from_bytes([0xff; 16]).unwrap(),
        &[&leaf],
    );
    let (target, _) = create_source(
        directory.path(),
        "target",
        PackageId::from_bytes([0xf2; 16]).unwrap(),
        &[&leaf, &wrapper],
    );
    let current = target.current().unwrap();
    let head = fs::read(target.root().join("HEAD")).unwrap();
    let ready_path = target.root().join("PACKAGE-TRANSPORTS/CURRENT");
    let ready = fs::read(&ready_path).unwrap();
    let admitted = crate::platform::package_transport::source::PackageContainer::decode(
        &leaf.container,
        leaf.transport_digest,
    )
    .unwrap()
    .admit()
    .unwrap();
    let private = admitted.packages[&leaf.revision_digest]
        .snapshot
        .owners
        .values()
        .find(|owner| matches!(owner, crate::platform::kernel::OwnerRecord::Expression(_)))
        .unwrap();
    let digest = crate::platform::kernel::encode_owner(private).unwrap().0;
    let store = target.object_store().unwrap();
    let hidden = Hide {
        base: &store,
        key: ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes()),
    };
    begin_collections();
    let missing = super::super::validate_dependency_sources_controlled(
        &hidden,
        &store,
        &current.semantic_root,
        &|| Ok(()),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(missing.code, "package_source_missing");
    assert_eq!(collections(), [wrapper.revision_digest]);
    drop(store);
    let mut without_leaf = PackageReadiness::decode(&ready).unwrap();
    without_leaf.bindings.remove(&leaf.revision_digest).unwrap();
    fs::write(&ready_path, without_leaf.encode().unwrap()).unwrap();
    let store = target.object_store().unwrap();
    begin_collections();
    let missing = super::super::validate_dependency_sources_controlled(
        &store,
        &store,
        &current.semantic_root,
        &|| Ok(()),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert_eq!(missing.code, "package_transport_missing");
    assert!(
        collections().is_empty(),
        "all union readiness is checked before any source proof"
    );
    assert_eq!(fs::read(target.root().join("HEAD")).unwrap(), head);
    fs::write(&ready_path, &ready).unwrap();
    assert_eq!(fs::read(&ready_path).unwrap(), ready);
}
