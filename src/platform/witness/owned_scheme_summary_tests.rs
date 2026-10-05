use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

#[test]
fn original_graph27_implementation_and_operand_summary_material_remains_exact() {
    let seed = b"frozen-generic-implementation-summary";
    let reference = DeclarationReference {
        package: PackageId::migrate(seed, 0),
        declaration: DeclarationId::migrate(seed, 0),
    };
    let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let method = MethodId::migrate(seed, 0);
    let mapping = vec![(method, reference)];
    let owner = OwnerKey::Declaration(reference.declaration);
    let record = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader {
            contract_version: 27,
            owner,
            kind: OwnerKind::OwnedImplementation,
        },
        module: ModuleId::migrate(seed, 0),
        name: Name::new("Flat").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
            implementation_parameters: Vec::new(),
            type_parameters: Vec::new(),
            contract: reference,
            self_type: ty,
            type_arguments: vec![ty],
            methods: vec![OwnedMethodImplementation {
                implementations: Vec::new(),
                method,
                function: reference,
                type_arguments: Vec::new(),
            }],
        }),
    });
    let summary = local_summary(owner, &record, None).unwrap();
    let mut interface = Material::new(OwnerKind::OwnedImplementation);
    interface.piece(1, &DeclarationVisibility::Public).unwrap();
    interface.raw_piece(2, &[10]);
    interface
        .piece(10, &(reference, ty, vec![ty], &mapping))
        .unwrap();
    assert_eq!(
        summary.semantic_interface,
        interface.finish(INTERFACE_DIGEST_DOMAIN)
    );
    let mut implementation = Material::new(OwnerKind::OwnedImplementation);
    implementation.piece(5, &mapping).unwrap();
    assert_eq!(
        summary.implementation,
        implementation.finish(IMPLEMENTATION_DIGEST_DOMAIN)
    );

    let id = ExpressionId::migrate(seed, 0);
    let owner = OwnerKey::Expression(id);
    let expression = OwnerRecord::Expression(ExpressionRecord {
        contract_version: 27,
        id,
        operation: ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Concrete {
                implementations: Vec::new(),
                implementation: reference,
                type_arguments: Vec::new(),
            },
            contract: reference,
            method,
            arguments: Vec::new(),
        },
    });
    let summary = local_summary(owner, &expression, None).unwrap();
    let mut implementation = Material::new(OwnerKind::Expression);
    implementation.piece(1, &id).unwrap();
    implementation
        .piece(
            2,
            &(
                25_u32,
                (0_u32, reference),
                reference,
                method,
                Vec::<ExpressionId>::new(),
            ),
        )
        .unwrap();
    assert_eq!(
        summary.implementation,
        implementation.finish(IMPLEMENTATION_DIGEST_DOMAIN)
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
                implementations: Vec::new(),
                method: MethodId::migrate(seed, 0),
                function: reference,
                type_arguments: types,
            }],
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
