use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

#[test]
fn interface15_implementation_is_literal_and_cannot_carry_new_scheme_fields() {
    let seed = b"generic-implementation-interface";
    let declaration = DeclarationId::migrate(seed, 0);
    let owner = OwnerKey::Declaration(declaration);
    let header = OwnerHeader {
        contract_version: 27,
        owner,
        kind: OwnerKind::OwnedImplementation,
    };
    let reference = DeclarationReference {
        package: PackageId::migrate(seed, 0),
        declaration,
    };
    let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let bytes = crate::platform::packed::encode(
        *b"LKJPIF15",
        "lkjscript.package-interface-owner-envelope.v15",
        &(
            15_u16,
            0_u32,
            header,
            Name::new("Flat").unwrap(),
            6_u32,
            reference,
            ty,
            vec![ty],
            vec![(MethodId::migrate(seed, 0), reference)],
        ),
        MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
    )
    .unwrap();
    let digest = PackageInterfaceOwnerDigest::of(&bytes);
    let mut value = PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap();
    assert_eq!(value.encode().unwrap(), (digest, bytes));
    let PackageInterfaceRecord::Declaration(d) = &mut value.record else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    assert!(i.type_parameters.is_empty());
    assert!(i.implementation_parameters.is_empty());
    assert!(i.methods[0].type_arguments.is_empty());
    assert!(i.methods[0].implementations.is_empty());
    i.type_parameters.push(TypeParameterId::migrate(seed, 0));
    assert!(value.encode().is_err());
    value.contract_version = 16;
    let PackageInterfaceRecord::Declaration(d) = &mut value.record else {
        unreachable!()
    };
    d.header.contract_version = 28;
    let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.methods[0].type_arguments.push(ty);
    let (digest, bytes) = value.encode().unwrap();
    assert_eq!(&bytes[..8], b"LKJPIF16");
    assert_eq!(
        PackageInterfaceOwner::decode(&bytes, owner, digest).unwrap(),
        value
    );
    let disguised = crate::platform::packed::encode(
        *b"LKJPIF15",
        "lkjscript.package-interface-owner-envelope.v15",
        &value,
        MAXIMUM_PACKAGE_INTERFACE_OWNER_BYTES,
    )
    .unwrap();
    assert!(
        PackageInterfaceOwner::decode(
            &disguised,
            owner,
            PackageInterfaceOwnerDigest::of(&disguised)
        )
        .is_err()
    );
}

#[test]
fn frozen_interface16_rejects_prerequisites_and_retains_original_bytes() {
    let seed = b"prerequisite-interface-cut";
    let declaration = DeclarationId::migrate(seed, 0);
    let reference = DeclarationReference {
        package: PackageId::migrate(seed, 0),
        declaration,
    };
    let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let mut value = PackageInterfaceOwner {
        contract_version: 16,
        record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
            header: OwnerHeader {
                contract_version: 28,
                owner: OwnerKey::Declaration(declaration),
                kind: OwnerKind::OwnedImplementation,
            },
            name: Name::new("Delegating").unwrap(),
            payload: PackageInterfaceDeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![],
                implementation_parameters: vec![],
                contract: reference,
                self_type: ty,
                type_arguments: vec![],
                methods: vec![OwnedMethodImplementation {
                    method: MethodId::migrate(seed, 0),
                    function: reference,
                    type_arguments: vec![],
                    implementations: vec![],
                }],
            }),
        }),
    };
    let original = value.encode().unwrap();
    assert_eq!(&original.1[..8], b"LKJPIF16");
    assert_eq!(
        PackageInterfaceOwner::decode(&original.1, value.owner(), original.0)
            .unwrap()
            .encode()
            .unwrap(),
        original
    );
    let PackageInterfaceRecord::Declaration(d) = &mut value.record else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.implementation_parameters.push(ImplementationParameter {
        id: ImplementationParameterId::migrate(seed, 0),
        name: Name::new("reader").unwrap(),
        contract: reference,
        self_type: ty,
        type_arguments: vec![],
    });
    i.methods[0]
        .implementations
        .push(ImplementationOperand::Parameter {
            scope: reference,
            parameter: i.implementation_parameters[0].id,
        });
    assert!(value.encode().is_err());
    value.contract_version = PACKAGE_INTERFACE_CONTRACT_VERSION;
    let PackageInterfaceRecord::Declaration(d) = &mut value.record else {
        unreachable!()
    };
    d.header.contract_version = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    let (digest, bytes) = value.encode().unwrap();
    assert_eq!(&bytes[..8], b"LKJPIF18");
    assert_eq!(
        PackageInterfaceOwner::decode(&bytes, value.owner(), digest).unwrap(),
        value
    );
}

#[test]
fn generic_interface_selects_and_admits_exact_owned_parameter_children_and_mapping_roots() {
    let seed = b"generic-implementation-interface-closure";
    let package = PackageId::migrate(seed, 0);
    let declaration = DeclarationId::migrate(seed, 0);
    let parameter = TypeParameterId::migrate(seed, 0);
    let reference = DeclarationReference {
        package,
        declaration,
    };
    let parameter_object = TypeObject::new(TypeForm::TypeParameter { parameter }).unwrap();
    let parameter_type = encode_type_object(&parameter_object).unwrap().0;
    let self_object = TypeObject::new(TypeForm::OwnedSequence {
        item: parameter_type,
    })
    .unwrap();
    let self_type = encode_type_object(&self_object).unwrap().0;
    let mapped_object = TypeObject::new(TypeForm::ByteBuffer).unwrap();
    let mapped_type = encode_type_object(&mapped_object).unwrap().0;
    let owner = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(
            OwnerKey::Declaration(declaration),
            OwnerKind::OwnedImplementation,
        ),
        module: ModuleId::migrate(seed, 0),
        name: Name::new("Flat").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![parameter],
            implementation_parameters: vec![],
            contract: reference,
            self_type,
            type_arguments: vec![parameter_type],
            methods: vec![OwnedMethodImplementation {
                method: MethodId::migrate(seed, 0),
                function: reference,
                type_arguments: vec![mapped_type],
                implementations: vec![],
            }],
        }),
    });
    let mut selection = PackageInterfaceSelection::new(package);
    selection.observe_declaration(&owner).unwrap();
    assert!(selection.contains(OwnerKey::TypeParameter(parameter)));
    let projected = PackageInterfaceRecord::project_public(&owner)
        .unwrap()
        .unwrap();
    assert_eq!(
        projected.type_roots(),
        vec![self_type, parameter_type, mapped_type]
    );
    let mut owners = BTreeMap::from([(
        owner.owner(),
        PackageInterfaceOwner {
            contract_version: 16,
            record: projected,
        },
    )]);
    let types = BTreeMap::from([
        (parameter_type, parameter_object),
        (self_type, self_object),
        (mapped_type, mapped_object),
    ]);
    assert_eq!(
        validate_owner_closure(package, &owners, &types)
            .unwrap_err()
            .code,
        "package_interface_child_missing"
    );
    owners.insert(
        OwnerKey::TypeParameter(parameter),
        PackageInterfaceOwner {
            contract_version: 16,
            record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration,
                name: Name::new("T").unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
        },
    );
    validate_owner_closure(package, &owners, &types).unwrap();
    validate_interface_type_reference(
        package,
        owner.owner(),
        &TypeForm::TypeParameter { parameter },
        &owners,
    )
    .unwrap();
    let PackageInterfaceRecord::TypeParameter(p) = &mut owners
        .get_mut(&OwnerKey::TypeParameter(parameter))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    p.constraints = TypeParameterConstraints::None;
    assert!(validate_owner_closure(package, &owners, &types).is_err());
    let PackageInterfaceRecord::TypeParameter(p) = &mut owners
        .get_mut(&OwnerKey::TypeParameter(parameter))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    p.constraints = TypeParameterConstraints::Owned;
    p.declaration = DeclarationId::migrate(seed, 1);
    assert_eq!(
        validate_owner_closure(package, &owners, &types)
            .unwrap_err()
            .code,
        "package_interface_child_parent"
    );
    assert_eq!(
        validate_interface_type_reference(
            package,
            owner.owner(),
            &TypeForm::TypeParameter { parameter },
            &owners
        )
        .unwrap_err()
        .code,
        "package_interface_type_parameter_scope"
    );
}
