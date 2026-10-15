use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

#[test]
fn validation_dependencies_bind_the_current_graph_contract() {
    let package = PackageId::migrate(b"read-generation-summary", 0);
    let kind = OwnerKind::Parameter;
    let mut material = vec![kind.tag(), 3];
    material.extend_from_slice(&1_u64.to_le_bytes());
    material.push(crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION as u8);
    let current = validation_dependency_digest(
        package,
        kind,
        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        &[],
        |_| Ok(None),
        |_| Ok(None),
    )
    .unwrap();
    assert_eq!(
        current,
        SemanticDigest::of(VALIDATION_DEPENDENCY_DIGEST_DOMAIN, &material)
    );
}

#[test]
fn scheme_children_and_mapping_applications_contribute_to_summaries() {
    let seed = b"generic-implementation-summary";
    let reference = DeclarationReference {
        package: PackageId::migrate(seed, 0),
        declaration: DeclarationId::migrate(seed, 0),
    };
    let parameters = vec![
        TypeParameterId::migrate(seed, 0),
        TypeParameterId::migrate(seed, 1),
    ];
    let types = [TypeForm::ByteBuffer, TypeForm::OwnedI64Cell]
        .into_iter()
        .map(|form| {
            encode_type_object(&TypeObject::new(form).unwrap())
                .unwrap()
                .0
        })
        .collect::<Vec<_>>();
    let owner = OwnerKey::Declaration(reference.declaration);
    let mut record = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(owner, OwnerKind::OwnedImplementation),
        module: ModuleId::migrate(seed, 0),
        name: Name::new("Flat").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
            implementation_parameters: Vec::new(),
            type_parameters: parameters.clone(),
            contract: reference,
            self_type: types[0],
            type_arguments: Vec::new(),
            methods: vec![OwnedMethodImplementation {
                read_scope_arguments: Vec::new(),
                implementations: Vec::new(),
                method: MethodId::migrate(seed, 0),
                function: reference,
                type_arguments: types,
            }],
            associated_types: Vec::new(),
        }),
    });
    assert_eq!(
        aggregation_children(&record).unwrap(),
        parameters
            .iter()
            .map(|p| (
                OwnershipRole::DeclarationTypeParameter,
                OwnerKey::TypeParameter(*p)
            ))
            .collect::<Vec<_>>()
    );
    let first = local_summary(owner, &record, None).unwrap();
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.methods[0].type_arguments.reverse();
    let second = local_summary(owner, &record, None).unwrap();
    assert_ne!(first.semantic_interface, second.semantic_interface);
    assert_ne!(first.implementation, second.implementation);
    assert_ne!(first.type_digest, second.type_digest);
    assert_eq!(first.presentation, second.presentation);
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.type_parameters.reverse();
    let third = local_summary(owner, &record, None).unwrap();
    assert_ne!(second.semantic_interface, third.semantic_interface);
    assert_eq!(second.implementation, third.implementation);

    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    let prerequisite = ImplementationParameterId::migrate(seed, 0);
    i.implementation_parameters.push(ImplementationParameter {
        id: prerequisite,
        name: Name::new("reader").unwrap(),
        contract: reference,
        self_type: i.self_type,
        type_arguments: Vec::new(),
    });
    let fourth = local_summary(owner, &record, None).unwrap();
    assert_ne!(third.semantic_interface, fourth.semantic_interface);
    assert_eq!(third.implementation, fourth.implementation);

    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.methods[0]
        .implementations
        .push(ImplementationOperand::Parameter {
            scope: reference,
            parameter: prerequisite,
        });
    let fifth = local_summary(owner, &record, None).unwrap();
    assert_ne!(fourth.semantic_interface, fifth.semantic_interface);
    assert_ne!(fourth.implementation, fifth.implementation);
    assert_eq!(fourth.presentation, fifth.presentation);

    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    let ImplementationOperand::Parameter { scope, .. } = &mut i.methods[0].implementations[0]
    else {
        unreachable!()
    };
    scope.declaration = DeclarationId::migrate(seed, 1);
    let sixth = local_summary(owner, &record, None).unwrap();
    assert_ne!(fifth.semantic_interface, sixth.semantic_interface);
    assert_ne!(fifth.implementation, sixth.implementation);
    assert_eq!(fifth.presentation, sixth.presentation);
}
