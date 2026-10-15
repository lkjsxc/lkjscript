use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

#[test]
fn interface_flat_schema_preserves_prerequisites_and_recursive_witnesses() {
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
        contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
        record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
            header: OwnerHeader {
                contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
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
                    read_scope_arguments: vec![],
                    method: MethodId::migrate(seed, 0),
                    function: reference,
                    type_arguments: vec![],
                    implementations: vec![],
                }],
                associated_types: Vec::new(),
            }),
        }),
    };
    let original = value.encode().unwrap();
    assert_eq!(&original.1[..8], &PACKAGE_INTERFACE_MAGIC);
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
    let (digest, bytes) = value.encode().unwrap();
    assert_eq!(&bytes[..8], &PACKAGE_INTERFACE_MAGIC);
    assert_ne!(digest, original.0);
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
    let contract = DeclarationReference {
        package,
        declaration: DeclarationId::migrate(seed, 1),
    };
    let mapped_function = DeclarationReference {
        package,
        declaration: DeclarationId::migrate(seed, 2),
    };
    let contract_self = TypeParameterId::migrate(seed, 1);
    let contract_item = TypeParameterId::migrate(seed, 2);
    let mapped_parameter = TypeParameterId::migrate(seed, 3);
    let method = MethodId::migrate(seed, 0);
    let header = |owner, kind| OwnerHeader {
        contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        owner,
        kind,
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
    let mapped_parameter_object = TypeObject::new(TypeForm::TypeParameter {
        parameter: mapped_parameter,
    })
    .unwrap();
    let mapped_parameter_type = encode_type_object(&mapped_parameter_object).unwrap().0;
    let owner = OwnerRecord::Declaration(DeclarationRecord {
        header: header(
            OwnerKey::Declaration(declaration),
            OwnerKind::OwnedImplementation,
        ),
        module: ModuleId::migrate(seed, 0),
        name: Name::new("Flat").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![parameter],
            implementation_parameters: vec![],
            contract,
            self_type,
            type_arguments: vec![parameter_type],
            methods: vec![OwnedMethodImplementation {
                read_scope_arguments: vec![],
                method,
                function: mapped_function,
                type_arguments: vec![mapped_type],
                implementations: vec![],
            }],
            associated_types: Vec::new(),
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
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: projected,
        },
    )]);
    // Shared contract method identity remains distinct from the implementation and target.
    // The target's complete one-parameter scheme maps U to ByteBuffer, so the result matches
    // the method's ByteBuffer result without inventing self-declaration authority.
    for (reference, kind, name, payload) in [
        (
            contract,
            OwnerKind::OwnedContract,
            "Reader",
            PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: contract_self,
                type_parameters: vec![contract_item],
                associated_types: Vec::new(),
                methods: vec![OwnedMethod {
                    id: method,
                    name: Name::new("read").unwrap(),
                    parameters: Vec::new(),
                    result: CallableResult {
                        ty: mapped_type,
                        relation: ResultRelation::Value,
                    },
                    read_scope_parameters: Vec::new(),
                    read_scope_sources: Vec::new(),
                    effect: FunctionEffect::Pure,
                }],
            }),
        ),
        (
            mapped_function,
            OwnerKind::PureFunction,
            "read",
            PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                implementation_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: vec![mapped_parameter],
                parameters: Vec::new(),
                result: CallableResult {
                    ty: mapped_parameter_type,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: FunctionEffect::Pure,
            }),
        ),
    ] {
        let key = OwnerKey::Declaration(reference.declaration);
        owners.insert(
            key,
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                    header: header(key, kind),
                    name: Name::new(name).unwrap(),
                    payload,
                }),
            },
        );
    }
    for (id, declaration, name) in [
        (contract_self, contract.declaration, "Self"),
        (contract_item, contract.declaration, "Item"),
        (mapped_parameter, mapped_function.declaration, "U"),
    ] {
        owners.insert(
            OwnerKey::TypeParameter(id),
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                    header: header(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
                    declaration,
                    name: Name::new(name).unwrap(),
                    constraints: TypeParameterConstraints::Owned,
                }),
            },
        );
    }
    let types = BTreeMap::from([
        (parameter_type, parameter_object),
        (self_type, self_object),
        (mapped_type, mapped_object),
        (mapped_parameter_type, mapped_parameter_object),
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
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                header: header(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
                declaration,
                name: Name::new("T").unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
        },
    );
    validate_owner_closure(package, &owners, &types).unwrap();
    for owner in owners.values() {
        let (digest, bytes) = owner.encode().unwrap();
        assert_eq!(
            PackageInterfaceOwner::decode(&bytes, owner.owner(), digest).unwrap(),
            owner.clone()
        );
    }
    // Keep the old counterfeit self references as explicit bad contract and target attacks.
    for wrong_contract in [true, false] {
        let mut malformed = owners.clone();
        let PackageInterfaceRecord::Declaration(record) = &mut malformed
            .get_mut(&OwnerKey::Declaration(declaration))
            .unwrap()
            .record
        else {
            unreachable!()
        };
        let PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) =
            &mut record.payload
        else {
            unreachable!()
        };
        if wrong_contract {
            implementation.contract = reference;
        } else {
            implementation.methods[0].function = reference;
        }
        let error = validate_owner_closure(package, &malformed, &types).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "package_interface_read_view");
        assert!(error.message.contains(if wrong_contract {
            "scope authority is not an Owned contract"
        } else {
            "target is not a named function signature"
        }));
    }
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
