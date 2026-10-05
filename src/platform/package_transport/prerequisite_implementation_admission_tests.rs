// Included by the source-loader owner; mutations target private meaning and keep
// original public commitments while rehashing every enclosing source identity.

#[test]
fn prerequisite_transport_rejects_rehashed_unused_mappings_and_nested_applications() {
    let snapshot = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        crate::platform::compiler::tests::prerequisite_implementation_admission_tests::SOURCE,
    )
    .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let original = GraphRepository::create(
        &directory.path().join("prerequisite-source"),
        &snapshot,
        None,
    )
    .unwrap()
    .repository
    .export_package_container()
    .unwrap();
    let accepted =
        crate::platform::package_transport::oracle::reconstruct(&original.container).unwrap();
    assert_eq!(
        accepted.snapshots[&snapshot.root.package_id].owners,
        snapshot.owners
    );
    let scheme = borrowed_transport_declaration(&snapshot, "Delegating");
    let scope = match &scheme {
        OwnerRecord::Declaration(d) => DeclarationReference {
            package: snapshot.root.package_id,
            declaration: match d.header.owner {
                OwnerKey::Declaration(id) => id,
                _ => unreachable!(),
            },
        },
        _ => unreachable!(),
    };
    let mut attacks = Vec::new();
    for fault in [
        "mapping-missing",
        "mapping-order",
        "prerequisite-self",
        "parameter-scope",
        "parameter-identity",
    ] {
        let mut replacement = scheme.clone();
        let OwnerRecord::Declaration(d) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
            unreachable!()
        };
        match fault {
            "mapping-missing" => {
                i.methods[0].implementations.pop();
            }
            "mapping-order" => i.methods[0].implementations.swap(0, 1),
            "prerequisite-self" => {
                i.implementation_parameters[0].self_type = *snapshot
                    .types
                    .iter()
                    .find_map(|(ty, object)| {
                        matches!(object.form, TypeForm::OwnedI64Cell).then_some(ty)
                    })
                    .unwrap()
            }
            "parameter-scope" => {
                let ImplementationOperand::Parameter { scope, .. } =
                    &mut i.methods[0].implementations[0]
                else {
                    unreachable!()
                };
                let OwnerRecord::Declaration(delegate) =
                    borrowed_transport_declaration(&snapshot, "delegate")
                else {
                    unreachable!()
                };
                let OwnerKey::Declaration(declaration) = delegate.header.owner else {
                    unreachable!()
                };
                scope.declaration = declaration;
            }
            "parameter-identity" => {
                let ImplementationOperand::Parameter { parameter, .. } =
                    &mut i.methods[0].implementations[0]
                else {
                    unreachable!()
                };
                *parameter =
                    crate::platform::semantic_id::ImplementationParameterId::from_bytes([0xed; 16])
                        .unwrap();
            }
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    let expression = snapshot.owners.values().find(|owner| matches!(owner,
        OwnerRecord::Expression(record) if matches!(&record.operation, ExpressionOperation::MethodCall { witness: ImplementationOperand::Concrete { implementation, implementations, .. }, .. }
            if *implementation == scope && !implementations.is_empty())
    )).unwrap().clone();
    let unused = borrowed_transport_declaration(&snapshot, "UnusedNested");
    for fault in [
        "unused-nested-missing",
        "unused-nested-order",
        "unused-nested-phantom",
    ] {
        let mut replacement = unused.clone();
        let OwnerRecord::Declaration(d) = &mut replacement else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
            unreachable!()
        };
        let ImplementationOperand::Concrete {
            implementations,
            type_arguments,
            ..
        } = &mut i.methods[0].implementations[0]
        else {
            unreachable!()
        };
        match fault {
            "unused-nested-missing" => {
                implementations.pop();
            }
            "unused-nested-order" => implementations.swap(0, 1),
            "unused-nested-phantom" => type_arguments.swap(0, 1),
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    for fault in ["nested-missing", "nested-order", "nested-phantom"] {
        let mut replacement = expression.clone();
        let OwnerRecord::Expression(record) = &mut replacement else {
            unreachable!()
        };
        let ExpressionOperation::MethodCall {
            witness:
                ImplementationOperand::Concrete {
                    implementations,
                    type_arguments,
                    ..
                },
            ..
        } = &mut record.operation
        else {
            unreachable!()
        };
        match fault {
            "nested-missing" => {
                implementations.pop();
            }
            "nested-order" => implementations.swap(0, 1),
            "nested-phantom" => type_arguments.swap(0, 1),
            _ => unreachable!(),
        }
        attacks.push((fault, replacement));
    }
    for (fault, replacement) in attacks {
        let (digest, bytes) = encode_owner(&replacement).unwrap();
        assert_eq!(
            decode_owner(&bytes, replacement.owner(), replacement.kind(), digest).unwrap(),
            replacement
        );
        let mut invalid = snapshot.clone();
        invalid
            .owners
            .insert(replacement.owner(), replacement.clone());
        assert!(
            !crate::platform::kernel::memory_reference::accepts(&invalid),
            "memory accepted {fault}"
        );
        let hostile = rehash_owner(&original, replacement);
        let bytes = hostile.encode().unwrap();
        let decoded = PackageContainer::decode(&bytes, hostile.root.transport).unwrap();
        let error = decoded.admit().unwrap_err();
        assert!(
            error.code == "kernel_owned_contract"
                || error.message.contains("kernel_owned_contract"),
            "{fault}: {error:?}"
        );
        let error = crate::platform::package_transport::oracle::reconstruct(&decoded).unwrap_err();
        assert!(
            error.message.contains("kernel_owned_contract"),
            "{fault}: {error:?}"
        );
        assert_not_ready(&bytes, hostile.root.transport);
    }
}
