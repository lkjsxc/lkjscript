// Included by source.rs: actual source admission, independent reconstruction and reclamation.
const SHARED_INTERFACE_SIGNATURE: &str = "declarations.begin\n(units (module create supplier (function create relay (visibility public) (parameter create value (type (record (value I64)))) (returns (record (value I64))) (effect pure) (body (local value)))))\ndeclarations.end\n";

#[test]
fn shared_dependency_interfaces_retain_exact_records_without_sharing_private_snapshots() {
    use std::sync::Arc;
    let (source, supplier, wrapper) = dependency_copy_triangle(SHARED_INTERFACE_SIGNATURE);
    let root = &source.packages[&source.container.root.package_revision].snapshot;
    let nested = &source.packages[&wrapper].snapshot;
    assert!(Arc::ptr_eq(&root.dependency_interfaces[&supplier], &nested.dependency_interfaces[&supplier]));
    let mut changed = root.clone();
    assert!(Arc::ptr_eq(&root.dependency_interfaces[&supplier], &changed.dependency_interfaces[&supplier]));
    Arc::make_mut(changed.dependency_interfaces.get_mut(&supplier).unwrap()).clear();
    assert!(!root.dependency_interfaces[&supplier].is_empty());
    assert!(!Arc::ptr_eq(&root.dependency_interfaces[&supplier], &changed.dependency_interfaces[&supplier]));
    changed.dependency_types.clear();
    assert!(!root.dependency_types.is_empty());
    // The source-disjoint oracle still reads and reconstructs every interface itself.
    let oracle = crate::platform::package_transport::oracle::reconstruct(&source.container).unwrap();
    for package in source.packages.values() {
        let independent = &oracle.snapshots[&package.revision.package];
        assert_eq!(package.snapshot.dependency_interfaces, independent.dependency_interfaces);
        assert_eq!(package.snapshot.dependency_types, independent.dependency_types);
        for (revision, records) in &package.snapshot.dependency_interfaces {
            assert!(!Arc::ptr_eq(records, &independent.dependency_interfaces[revision]));
        }
    }
}

#[test]
fn shared_dependency_interfaces_are_admission_local_and_reclaim_after_last_snapshot() {
    use std::sync::Arc;
    let (source, supplier, _) = dependency_copy_triangle(SHARED_INTERFACE_SIGNATURE);
    let second = source.container.admit().unwrap();
    let key = source.container.root.package_revision;
    let root = &source.packages[&key].snapshot;
    let other = &second.packages[&key].snapshot;
    assert_eq!(root.dependency_interfaces, other.dependency_interfaces);
    assert!(!Arc::ptr_eq(&root.dependency_interfaces[&supplier], &other.dependency_interfaces[&supplier]));
    let weak = Arc::downgrade(&root.dependency_interfaces[&supplier]);
    let retained = root.clone();
    drop(source);
    assert!(weak.upgrade().is_some());
    drop(retained);
    assert!(weak.upgrade().is_none(), "neither another admission nor a global cache retains the map");
    assert!(!second.packages[&key].snapshot.dependency_interfaces[&supplier].is_empty());
}

#[test]
fn shared_dependency_interfaces_never_turn_previous_admission_into_input_authority() {
    let (source, _, _) = dependency_copy_triangle(SHARED_INTERFACE_SIGNATURE);
    let accepted = source.container.admit().unwrap();
    let original = accepted.container.encode().unwrap();
    let mut corrupted = accepted.container.clone();
    let bytes = corrupted.objects.values_mut().next().unwrap();
    assert!(!bytes.is_empty());
    bytes[0] ^= 1;
    assert!(corrupted.admit().is_err());
    assert!(crate::platform::package_transport::oracle::reconstruct(&corrupted).is_err());
    assert_eq!(source.container.admit().unwrap().container.encode().unwrap(), original);
}

#[test]
fn shared_dependency_interface_exact_budget_rejects_partial_attachment() {
    // These totals are derived from actual written owner/child counts, not the
    // implementation's observed cost. The last refused allocation may be an
    // empty interface entry or a type copy depending on exact package-ID order.
    for (signature, exact, copies) in [
        ("declarations.begin\n(units (module create supplier (function create value (visibility public) (returns Unit) (effect pure) (body (unit)))))\ndeclarations.end\n", 9, (1, 2)),
        (SHARED_INTERFACE_SIGNATURE, 15, (2, 4)),
    ] {
        let (source, _, wrapper) = dependency_copy_triangle(signature);
        let interfaces = dependency_copy_interfaces(&source);
        for maximum in [exact - 1, exact] {
            let store = dependency_copy_store(&source.container.objects, maximum);
            let mut pool = SharedDependencyInterfaces::new(&interfaces, &store);
            DEPENDENCY_COPY_COUNTS.with(|counts| counts.set((0, 0)));
            let mut result = Ok(());
            for revision in [wrapper, source.container.root.package_revision] {
                let expected = &source.packages[&revision].snapshot;
                let mut candidate = expected.clone();
                candidate.dependency_interfaces.clear();
                candidate.dependency_types.clear();
                result = pool.attach(&mut candidate);
                let complete = candidate.dependency_interfaces == expected.dependency_interfaces
                    && candidate.dependency_types == expected.dependency_types;
                if result.is_err() {
                    assert!(!complete, "refusal cannot report a complete attachment");
                    break;
                }
                assert!(complete);
            }
            let observed = DEPENDENCY_COPY_COUNTS.with(|counts| counts.get());
            assert!(observed.0 <= copies.0 && observed.1 <= copies.1);
            assert!(store.visits() <= maximum);
            if maximum == exact {
                result.unwrap();
                assert_eq!(observed, copies);
                assert_eq!(store.visits(), exact);
            } else {
                let error = result.unwrap_err();
                assert_eq!(error.class, DiagnosticClass::Resource);
                assert_eq!(error.code, "package_source_budget");
            }
        }
    }
}
