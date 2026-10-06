// Included by source.rs: exercise its actual attachment loop with admitted package owners.

fn dependency_copy_empty(seed: &[u8]) -> KernelSnapshot {
    use crate::platform::persistent_map::{MapContentDigest, PageDigest};
    use crate::platform::semantic_id::RepositoryId;
    let empty = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(seed, 0),
            package_id: PackageId::migrate(seed, 0),
            package_name: Name::new("copy_fixture").unwrap(),
            owners: empty,
            dependencies: empty,
            retirements: empty,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}

fn dependency_copy_importer(seed: &[u8], imports: &[&AdmittedClosure]) -> AdmittedClosure {
    use crate::platform::change::{AuthoredChange, AuthoredChangeSet, ChangeBudget};
    use crate::platform::publication::PublicationOptions;
    let directory = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &directory.path().join("importer"),
        &dependency_copy_empty(seed),
        None,
    )
    .unwrap();
    let mut changes = Vec::new();
    for imported in imports {
        let binding = imported.container.root;
        created
            .repository
            .stage_package_transport(binding.transport, &imported.container.encode().unwrap())
            .unwrap();
        let package = &imported.packages[&binding.package_revision];
        changes.push(AuthoredChange::AddDependency {
            package: package.revision.package,
            semantic_revision: package.revision.revision.revision_id().unwrap(),
            package_revision: binding.package_revision,
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
    created.repository.export_package_container().unwrap()
}

fn dependency_copy_triangle(
    signature: &str,
) -> (AdmittedClosure, PackageRevisionDigest, PackageRevisionDigest) {
    let snapshot =
        crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(signature)
            .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let supplier = GraphRepository::create(&directory.path().join("supplier"), &snapshot, None)
        .unwrap()
        .repository
        .export_package_container()
        .unwrap();
    let wrapper = dependency_copy_importer(b"copy-wrapper", &[&supplier]);
    let root = dependency_copy_importer(b"copy-root", &[&wrapper, &supplier]);
    assert_eq!((root.packages.len(), root.dependency_edges), (3, 3));
    (
        root,
        supplier.container.root.package_revision,
        wrapper.container.root.package_revision,
    )
}

fn dependency_copy_store(
    objects: &BTreeMap<ObjectKey, Vec<u8>>,
    maximum_visits: u64,
) -> CollectingStore<'_, BTreeMap<ObjectKey, Vec<u8>>> {
    let mut store = CollectingStore::new(objects);
    store.maximum_visits = maximum_visits;
    store.admission = RefCell::new(StoreReadAdmission::new(StoreReadLimits {
        maximum_catalog_lookups: maximum_visits,
        maximum_objects: maximum_visits,
        maximum_bytes: MAXIMUM_VALIDATION_READ_BYTES,
    }));
    // Earlier semantic work and dependency copies consume the same aggregate remainder.
    store.charge_visits(1).unwrap();
    store
}

fn dependency_copy_interfaces(
    source: &AdmittedClosure,
) -> BTreeMap<PackageRevisionDigest, PackageInterfaceValidation> {
    source
        .packages
        .iter()
        .map(|(revision, package)| {
            let interface = crate::platform::package_interface::validate_package_interface(
                package.revision.package,
                package.transport.interface_owners,
                &source.container.objects,
                &mut StoreWork::default(),
            )
            .unwrap();
            (*revision, interface)
        })
        .collect()
}

#[test]
fn dependency_interface_fanout_refuses_before_next_owner_or_type_copy_and_exact_fit_succeeds() {
    let (source, supplier, wrapper) = dependency_copy_triangle(
        "declarations.begin\n(units (module create supplier (function create value (visibility public) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n",
    );
    let interfaces = dependency_copy_interfaces(&source);
    assert_eq!(
        (interfaces[&supplier].owners.len(), interfaces[&supplier].type_objects.len()),
        (1, 1),
    );
    // Independently: three incoming edges, two shared maps (one empty), one owner,
    // two scalar copies and the earlier visit total nine. Tiny interfaces add overhead.
    for (maximum, expected_copies, success) in [
        (1, (0, 0), false),
        (2, (0, 0), false),
        (3, (0, 0), false),
        (4, (1, 0), false),
        (5, (1, 1), false),
        (9, (1, 2), true),
    ] {
        let store = dependency_copy_store(&source.container.objects, maximum);
        DEPENDENCY_COPY_COUNTS.with(|counts| counts.set((0, 0)));
        assert_eq!(store.charge_visits(u64::MAX).unwrap_err().code, "package_source_budget");
        assert_eq!(store.visits(), 1);
        assert_eq!(DEPENDENCY_COPY_COUNTS.with(|counts| counts.get()), (0, 0));
        let mut result = Ok(());
        let mut pool = SharedDependencyInterfaces::new(&interfaces, &store);
        for revision in [wrapper, source.container.root.package_revision] {
            let expected = &source.packages[&revision].snapshot;
            let mut snapshot = expected.clone();
            snapshot.dependency_interfaces.clear();
            snapshot.dependency_types.clear();
            result = pool.attach(&mut snapshot);
            if result.is_err() {
                if maximum <= 2 {
                    assert!(snapshot.dependency_types.is_empty());
                }
                break;
            }
            assert_eq!(snapshot.dependency_interfaces, expected.dependency_interfaces);
            assert_eq!(snapshot.dependency_types, expected.dependency_types);
        }
        assert_eq!(
            DEPENDENCY_COPY_COUNTS.with(|counts| counts.get()),
            expected_copies,
        );
        assert_eq!(store.visits(), maximum);
        assert_eq!(result.is_ok(), success);
        if let Err(error) = result {
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, "package_source_budget");
            assert!(error.message.contains("aggregate validation visits"));
        }
    }
    let exact = source
        .container
        .admit_with_budget(source.validation_visits, source.validation_read_bytes)
        .unwrap();
    assert_eq!(exact.validation_visits, source.validation_visits);
    assert!(
        source
            .container
            .admit_with_budget(source.validation_visits - 1, source.validation_read_bytes)
            .is_err(),
    );
    let oracle = crate::platform::package_transport::oracle::reconstruct(&source.container).unwrap();
    for package in exact.packages.values() {
        assert_eq!(
            package.snapshot.owners,
            oracle.snapshots[&package.revision.package].owners,
        );
        assert_eq!(
            package.snapshot.types,
            oracle.snapshots[&package.revision.package].types,
        );
    }
}

#[test]
fn dependency_interface_copy_reserves_owner_and_type_children_before_growth() {
    let (source, supplier, wrapper) = dependency_copy_triangle(
        "declarations.begin\n(units (module create supplier (function create relay (visibility public) (parameter create value (type (record (value I64)))) (returns (record (value I64))) (effect pure) (body (local value)))))\ndeclarations.end\n",
    );
    let interfaces = dependency_copy_interfaces(&source);
    assert_eq!(
        (interfaces[&supplier].owners.len(), interfaces[&supplier].type_objects.len()),
        (2, 2),
    );
    // Independently: function + parameter child + parameter owner = three; scalar type +
    // structural type + its field child = three more per importing package.
    // Three edges + two maps + one owner inventory (3) + two type inventories (6)
    // + the earlier visit total fifteen; owner records are copied only once.
    for (maximum, expected_copies, success) in [(9, (2, 2), false), (15, (2, 4), true)] {
        let store = dependency_copy_store(&source.container.objects, maximum);
        DEPENDENCY_COPY_COUNTS.with(|counts| counts.set((0, 0)));
        let mut result = Ok(());
        let mut pool = SharedDependencyInterfaces::new(&interfaces, &store);
        for revision in [wrapper, source.container.root.package_revision] {
            let mut snapshot = source.packages[&revision].snapshot.clone();
            snapshot.dependency_interfaces.clear();
            snapshot.dependency_types.clear();
            result = pool.attach(&mut snapshot);
            if result.is_err() {
                break;
            }
        }
        assert_eq!(
            DEPENDENCY_COPY_COUNTS.with(|counts| counts.get()),
            expected_copies,
        );
        assert_eq!(store.visits(), maximum);
        assert_eq!(result.is_ok(), success);
        if let Err(error) = result {
            assert_eq!(error.code, "package_source_budget");
        }
    }
}
