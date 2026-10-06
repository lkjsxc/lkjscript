// Included by source.rs. Identical serialized import graphs, before/after sharing.
#[test]
fn dependency_interface_fanout_matched_source_measurements() {
    use std::time::Instant;
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        include_str!("../../../examples/demanded-callable-proof/library.lkjc"),
    ).unwrap();
    let directory = tempfile::tempdir().unwrap();
    let supplier = GraphRepository::create(&directory.path().join("supplier"), &snapshot, None)
        .unwrap().repository.export_package_container().unwrap();
    let mut wrappers = Vec::new();
    for index in 0..8 {
        wrappers.push(dependency_copy_importer(format!("interface-fanout-{index}").as_bytes(), &[&supplier]));
    }
    for fanout in [1, 4, 8] {
        let imports: Vec<_> = std::iter::once(&supplier).chain(wrappers[..fanout].iter()).collect();
        let source = if let Some(root) = std::env::var_os("LKJSCRIPT_INTERFACE_BASELINE") {
            let path = std::path::PathBuf::from(root).join(format!("fanout-{fanout}.lkjp"));
            { let bytes = std::fs::read(path).unwrap(); let mut cursor = Cursor(&bytes); assert_eq!(cursor.take(8).unwrap(), CONTAINER_MAGIC); let expected = cursor.binding().unwrap().transport; PackageContainer::decode(&bytes, expected).unwrap().admit().unwrap() }
        } else {
            dependency_copy_importer(format!("interface-root-{fanout}").as_bytes(), &imports)
        };
        let (key, supplier) = source.packages.iter()
            .max_by_key(|(_, package)| package.interface_owners.len()).unwrap();
        let owner_count = supplier.interface_owners.len();
        let type_count = supplier.interface_types.len();
        DEPENDENCY_COPY_COUNTS.with(|counts| counts.set((0, 0)));
        let start = Instant::now();
        let admitted = source.container.admit().unwrap();
        let elapsed = start.elapsed().as_nanos();
        let (owner_copies, type_copies) = DEPENDENCY_COPY_COUNTS.with(|counts| counts.get());
        let imports: Vec<_> = admitted.packages.values()
            .filter_map(|package| package.snapshot.dependency_interfaces.get(key)).collect();
        assert_eq!(imports.len(), fanout + 1);
        assert!(imports.windows(2).all(|pair| std::sync::Arc::ptr_eq(pair[0], pair[1])));
        assert_eq!(owner_copies, owner_count);
        assert_eq!(type_copies, type_count * (fanout + 1));
        assert_eq!(source.container.encode().unwrap(), admitted.container.encode().unwrap());
        println!("interface-fanout {}", serde_json::json!({
            "fanout": fanout, "packages": admitted.packages.len(),
            "dependency_edges": admitted.dependency_edges,
            "owners_per_supplier": owner_count, "types_per_supplier": type_count,
            "owner_copies": owner_copies, "type_copies": type_copies,
            "validation_visits": admitted.validation_visits,
            "validation_read_bytes": admitted.validation_read_bytes,
            "admission_ns": elapsed,
        }));
    }
}
