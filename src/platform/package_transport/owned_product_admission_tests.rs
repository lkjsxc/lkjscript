// Uses the source-loader's hostile encoder; no accepted source is modified.
#[test]
fn owned_products_signature_only_transport_rejects_rehashed_nonowning_product() {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create signatures
  (function create relay (visibility private) (effect pure)
    (parameter create p (type (owned-product (field data ByteBuffer))) (use consume))
    (returns (owned-product (field data ByteBuffer))) (body (local p)))))
declarations.end"#,
    )
    .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("signatures"), &snapshot, None)
        .unwrap()
        .repository;
    let original = repository.export_package_container().unwrap();
    crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
    let (ordinary, ordinary_bytes) =
        encode_type_object(&TypeObject::new(TypeForm::I64).unwrap()).unwrap();
    let product = snapshot
        .types
        .values()
        .find_map(|object| match &object.form {
            TypeForm::OwnedProduct { fields } => Some(fields.clone()),
            _ => None,
        })
        .unwrap();
    let mut fields = product;
    assert_eq!(fields.len(), 1);
    fields[0].ty = ordinary;
    let (invalid, invalid_bytes) =
        encode_type_object(&TypeObject::new(TypeForm::OwnedProduct { fields }).unwrap()).unwrap();
    for mut replacement in snapshot
        .owners
        .values()
        .filter(|record| {
            matches!(record,
        OwnerRecord::Declaration(d) if d.name.as_str() == "relay")
                || matches!(record, OwnerRecord::Parameter(_))
        })
        .cloned()
    {
        // A complete signature cannot claim owned product custody for an entirely
        // ordinary payload, even though this private relay is never called.
        match &mut replacement {
            OwnerRecord::Declaration(declaration) => {
                let DeclarationPayload::Function(function) = &mut declaration.payload else {
                    unreachable!()
                };
                function.result.ty = invalid;
            }
            OwnerRecord::Parameter(parameter) => parameter.ty = invalid,
            _ => unreachable!(),
        }
        let mut hostile = rehash_owner(&original, replacement);
        hostile.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Type, ordinary.bytes()),
            ordinary_bytes.clone(),
        );
        hostile.objects.insert(
            ObjectKey::from_digest(ObjectDomain::Type, invalid.bytes()),
            invalid_bytes.clone(),
        );
        let bytes = hostile.encode().unwrap();
        let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
        let error = decoded.admit().unwrap_err();
        assert_eq!(error.code, "kernel_owned_product", "{error:?}");
        let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
        assert!(
            error.code == "kernel_owned_product" || error.message.contains("kernel_owned_product"),
            "{error:?}"
        );
        assert_not_ready(&bytes, hostile.root.transport);
    }
}
