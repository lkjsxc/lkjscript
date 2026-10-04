// Included by the source-loader test owner; hostile encodings never enter authoring.

const SEQUENCE_SIGNATURE_SOURCE: &str = r#"declarations.begin
(units (module create sequence-signatures
  (external create empty-buffer (visibility private) (implementation core.buffer.empty)
    (returns ByteBuffer))
  (function create relay (visibility private) (effect pure)
    (parameter create p (type (owned-sequence ByteBuffer)) (use consume))
    (returns (owned-sequence ByteBuffer)) (body (local p)))
  (function create unused (visibility private) (effect pure)
    (parameter create p (type (owned-sequence ByteBuffer)) (use consume))
    (returns Unit) (body (unit)))
  (function create empty (visibility private) (effect pure)
    (returns (owned-sequence ByteBuffer))
    (body (sequence-empty (type (owned-sequence ByteBuffer)))))))
declarations.end"#;

fn sequence_signature_transport() -> AdmittedClosure {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        SEQUENCE_SIGNATURE_SOURCE,
    )
    .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(&temporary.path().join("source"), &snapshot, None)
        .unwrap()
        .repository;
    repository.export_package_container().unwrap()
}

fn assert_sequence_source_rejected(hostile: &PackageContainer, expected: &str) {
    let bytes = hostile.encode().unwrap();
    let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
    let error = decoded.admit().unwrap_err();
    assert_eq!(error.code, expected, "{error:?}");
    let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
    assert!(
        error.code == expected || error.message.contains(expected),
        "{error:?}"
    );
    assert_not_ready(&bytes, hostile.root.transport);
}

#[test]
fn owned_sequence_transport_rejects_rehashed_predecessor_signatures_and_syntax() {
    let original = sequence_signature_transport();
    let independent =
        crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
    let package = &original.packages[&original.container.root.package_revision];
    assert_eq!(package.snapshot.root.graph_contract_version, 25);
    assert_eq!(
        independent.snapshots[&package.snapshot.root.package_id].types,
        package.snapshot.types
    );
    // Neither an uncalled signature nor unused construction can conceal Graph 25 meaning.
    for mut replacement in package.snapshot.owners.values().filter(|record| {
        matches!(record, OwnerRecord::Declaration(record) if matches!(record.name.as_str(), "relay" | "unused"))
            || matches!(record, OwnerRecord::Expression(record) if matches!(record.operation, ExpressionOperation::SequenceEmpty { .. }))
    }).cloned() {
        replacement.set_encoding_for_edit(24);
        let hostile = if matches!(replacement, OwnerRecord::Expression(_)) {
            assert_eq!(encode_owner(&replacement).unwrap_err().code, "kernel_sequence_generation");
            let bytes = crate::platform::packed::encode(
                *b"LKJOWN24",
                "lkjscript.kernel.owner-envelope.v24",
                &replacement,
                crate::platform::kernel::contract::MAXIMUM_OWNER_OBJECT_BYTES,
            ).unwrap();
            rehash_encoded_owner(&original, replacement, OwnerObjectDigest::of(&bytes), bytes)
        } else {
            rehash_owner(&original, replacement)
        };
        assert_sequence_source_rejected(&hostile, "kernel_sequence_generation");
    }
}

#[test]
fn owned_sequence_unused_external_result_cannot_cross_intrinsic_boundary() {
    let original = sequence_signature_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let sequence = package
        .snapshot
        .types
        .iter()
        .find_map(|(digest, object)| {
            matches!(object.form, TypeForm::OwnedSequence { .. }).then_some(*digest)
        })
        .unwrap();
    let mut replacement = package.snapshot.owners.values().find(|record| {
        matches!(record, OwnerRecord::Declaration(record) if record.name.as_str() == "empty-buffer")
    }).unwrap().clone();
    let OwnerRecord::Declaration(declaration) = &mut replacement else {
        unreachable!()
    };
    let DeclarationPayload::External(external) = &mut declaration.payload else {
        unreachable!()
    };
    external.result = sequence;
    assert_sequence_source_rejected(&rehash_owner(&original, replacement), "intrinsic_signature");
}

#[test]
fn owned_sequence_unused_parameter_still_admits_its_complete_element_contract() {
    let original = sequence_signature_transport();
    let package = &original.packages[&original.container.root.package_revision];
    let unused = package
        .snapshot
        .owners
        .values()
        .find_map(|record| match record {
            OwnerRecord::Declaration(record) if record.name.as_str() == "unused" => {
                Some(record.header.owner)
            }
            _ => None,
        })
        .unwrap();
    let OwnerKey::Declaration(unused) = unused else {
        unreachable!()
    };
    let mut replacement = package.snapshot.owners.values().find(|record| {
        matches!(record, OwnerRecord::Parameter(record) if record.parent == ParameterParent::Function(unused))
    }).unwrap().clone();
    let (i64_type, i64_bytes) =
        encode_type_object(&TypeObject::new(TypeForm::I64).unwrap()).unwrap();
    let (invalid, invalid_bytes) =
        encode_type_object(&TypeObject::new(TypeForm::OwnedSequence { item: i64_type }).unwrap())
            .unwrap();
    let OwnerRecord::Parameter(parameter) = &mut replacement else {
        unreachable!()
    };
    parameter.ty = invalid;
    let mut hostile = rehash_owner(&original, replacement);
    hostile.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Type, i64_type.bytes()),
        i64_bytes,
    );
    hostile.objects.insert(
        ObjectKey::from_digest(ObjectDomain::Type, invalid.bytes()),
        invalid_bytes,
    );
    assert_sequence_source_rejected(&hostile, "kernel_owned_sequence");
}
