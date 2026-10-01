// Uses the source-loader's hostile encoder; no accepted source is modified.
#[test]
fn owned_products_signature_only_transport_rejects_rehashed_graph_18_owner() {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(r#"declarations.begin
(units (module create signatures
  (function create relay (visibility private) (effect pure)
    (parameter create p (type (owned-product (field data ByteBuffer))) (use consume))
    (returns (owned-product (field data ByteBuffer))) (body (local p)))))
declarations.end"#).unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("signatures"), &snapshot, None).unwrap().repository;
    let original = repository.export_package_container().unwrap();
    crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
    let mut replacement = snapshot.owners.values().find(|record| matches!(record,
        OwnerRecord::Declaration(d) if d.name.as_str() == "relay")).unwrap().clone();
    replacement.set_encoding_for_edit(18);
    let hostile = rehash_owner(&original, replacement);
    let bytes = hostile.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
    let error = decoded.admit().unwrap_err();
    assert_eq!(error.code, "kernel_product_generation", "{error:?}");
    let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
    assert!(error.code == "kernel_product_generation" || error.message.contains("kernel_product_generation"), "{error:?}");
    assert_not_ready(&bytes, hostile.root.transport);
}
