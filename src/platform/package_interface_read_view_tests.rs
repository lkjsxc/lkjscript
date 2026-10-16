use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

fn fixture() -> (
    PackageId,
    BTreeMap<OwnerKey, PackageInterfaceOwner>,
    BTreeMap<TypeObjectDigest, TypeObject>,
    DeclarationId,
    ParameterId,
) {
    let seed = b"read-view-interface-independent-source";
    let package = PackageId::migrate(seed, 0);
    let view_id = DeclarationId::migrate(seed, 0);
    let function_id = DeclarationId::migrate(seed, 1);
    let source = ParameterId::migrate(seed, 0);
    let formal = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(seed, 0),
        name: Name::new("read_scope").unwrap(),
    };
    let scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(DeclarationReference {
            package,
            declaration: function_id,
        }),
        parameter: formal.id,
    });
    let backing = TypeObject::new(TypeForm::ByteBuffer).unwrap();
    let backing_type = encode_type_object(&backing).unwrap().0;
    let result = TypeObject::new(TypeForm::ReadViewApplied {
        declaration: DeclarationReference {
            package,
            declaration: view_id,
        },
        arguments: Vec::new(),
        read_scope: scope,
    })
    .unwrap();
    let result_type = encode_type_object(&result).unwrap().0;
    let values = [
        PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
            header: OwnerHeader::new(OwnerKey::Declaration(view_id), OwnerKind::ReadView),
            name: Name::new("MetadataRead").unwrap(),
            payload: PackageInterfaceDeclarationPayload::ReadView(ReadViewDeclaration {
                type_parameters: Vec::new(),
                scope_parameter: formal.clone(),
                fields: Vec::new(),
            }),
        }),
        PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
            header: OwnerHeader::new(OwnerKey::Declaration(function_id), OwnerKind::PureFunction),
            name: Name::new("metadata").unwrap(),
            payload: PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                implementation_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: vec![source],
                result: CallableResult {
                    ty: result_type,
                    relation: ResultRelation::ViewFrom { source, scope },
                },
                read_scope_parameters: vec![formal],
                read_scope_sources: vec![ReadScopeSource { source, scope }],
                effect: FunctionEffect::Pure,
            }),
        }),
        PackageInterfaceRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(source), OwnerKind::Parameter),
            parent: ParameterParent::Function(function_id),
            name: Name::new("source").unwrap(),
            ty: backing_type,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    ];
    let owners = values
        .into_iter()
        .map(|record| {
            let owner = PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record,
            };
            (owner.owner(), owner)
        })
        .collect();
    (
        package,
        owners,
        BTreeMap::from([(backing_type, backing), (result_type, result)]),
        function_id,
        source,
    )
}

fn region_form(
    types: &mut BTreeMap<TypeObjectDigest, TypeObject>,
    form: TypeForm,
) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    types.insert(digest, object);
    digest
}

#[test]
fn region_interface_admits_shared_loans_and_rejects_value_kind_or_scope_erasure() {
    let (package, mut owners, mut types, function, source) = fixture();
    let parameter = TypeParameterId::migrate(b"interface-region-kind", 0);
    let owner = PackageInterfaceOwner {
        contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
        record: PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: function,
            name: Name::new("R").unwrap(),
            constraints: TypeParameterConstraints::Region,
        }),
    };
    owners.insert(owner.owner(), owner);
    let region = region_form(&mut types, TypeForm::TypeParameter { parameter });
    let PackageInterfaceRecord::Declaration(declaration) = &mut owners
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.type_parameters.push(parameter);
    let ResultRelation::ViewFrom { scope, .. } = signature.result.relation else {
        unreachable!()
    };
    let shared = region_form(
        &mut types,
        TypeForm::RegionCap {
            region,
            mode: RegionMode::Shared { scope },
        },
    );
    signature.result.ty = shared;
    let PackageInterfaceRecord::Parameter(input) =
        &mut owners.get_mut(&OwnerKey::Parameter(source)).unwrap().record
    else {
        unreachable!()
    };
    input.ty = shared;
    validate_owner_closure(package, &owners, &types).unwrap();

    let mut consumed = owners.clone();
    let PackageInterfaceRecord::Parameter(input) = &mut consumed
        .get_mut(&OwnerKey::Parameter(source))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    input.use_mode = ParameterUse::Consume;
    assert!(validate_owner_closure(package, &consumed, &types).is_err());

    let mut escaped = owners.clone();
    let PackageInterfaceRecord::Declaration(declaration) = &mut escaped
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.result.relation = ResultRelation::Value;
    assert!(validate_owner_closure(package, &escaped, &types).is_err());

    // Change only the exact result scope. Correct type bytes and a positive
    // view category cannot replace the declared source-scope relationship.
    let foreign_scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(DeclarationReference {
            package,
            declaration: function,
        }),
        parameter: ReadScopeParameterId::migrate(b"absent-region-scope", 0),
    });
    let wrong = region_form(
        &mut types,
        TypeForm::RegionCap {
            region,
            mode: RegionMode::Shared {
                scope: foreign_scope,
            },
        },
    );
    let mut wrong_scope = owners.clone();
    let PackageInterfaceRecord::Declaration(declaration) = &mut wrong_scope
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.result.ty = wrong;
    assert!(validate_owner_closure(package, &wrong_scope, &types).is_err());

    let unit = region_form(&mut types, TypeForm::Unit);
    let i64_type = region_form(&mut types, TypeForm::I64);
    let pointer = region_form(
        &mut types,
        TypeForm::RelPtr {
            region,
            item: i64_type,
        },
    );
    let PackageInterfaceRecord::Declaration(declaration) = &mut owners
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.result = CallableResult {
        ty: unit,
        relation: ResultRelation::Value,
    };
    signature.read_scope_parameters.clear();
    signature.read_scope_sources.clear();
    let PackageInterfaceRecord::Parameter(input) =
        &mut owners.get_mut(&OwnerKey::Parameter(source)).unwrap().record
    else {
        unreachable!()
    };
    input.ty = pointer;
    input.use_mode = ParameterUse::Unrestricted;
    validate_owner_closure(package, &owners, &types).unwrap();
    let identity_list = region_form(&mut types, TypeForm::List { item: region });
    let identity_item = region_form(
        &mut types,
        TypeForm::RelPtr {
            region,
            item: region,
        },
    );
    let shared_list = region_form(&mut types, TypeForm::List { item: shared });
    for ty in [region, identity_list, identity_item, shared_list] {
        let PackageInterfaceRecord::Parameter(input) =
            &mut owners.get_mut(&OwnerKey::Parameter(source)).unwrap().record
        else {
            unreachable!()
        };
        input.ty = ty;
        assert!(
            validate_owner_closure(package, &owners, &types).is_err(),
            "accepted {ty}"
        );
    }
}

#[test]
fn metadata_only_view_still_retains_its_exact_selected_source() {
    let (package, mut owners, types, function, source) = fixture();
    validate_owner_closure(package, &owners, &types).unwrap();
    let owner = owners.get(&OwnerKey::Declaration(function)).unwrap();
    let (digest, bytes) = owner.encode().unwrap();
    assert_eq!(&bytes[..8], b"LKJPIF22");
    assert_eq!(
        PackageInterfaceOwner::decode(&bytes, owner.owner(), digest).unwrap(),
        *owner
    );
    let mut missing_relationship = owners.clone();
    let PackageInterfaceRecord::Declaration(declaration) = &mut missing_relationship
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.read_scope_sources.clear();
    assert!(validate_owner_closure(package, &missing_relationship, &types).is_err());
    let PackageInterfaceRecord::Parameter(parameter) =
        &mut owners.get_mut(&OwnerKey::Parameter(source)).unwrap().record
    else {
        unreachable!()
    };
    parameter.use_mode = ParameterUse::Consume;
    assert!(validate_owner_closure(package, &owners, &types).is_err());
}

#[test]
fn equal_formal_identity_in_another_binder_grants_no_scope() {
    let (package, owners, mut types, function, _) = fixture();
    let owner = owners.get(&OwnerKey::Declaration(function)).unwrap();
    let PackageInterfaceRecord::Declaration(declaration) = &owner.record else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &declaration.payload else {
        unreachable!()
    };
    let object = types.get_mut(&signature.result.ty).unwrap();
    let TypeForm::ReadViewApplied {
        declaration: view,
        read_scope,
        ..
    } = &mut object.form
    else {
        unreachable!()
    };
    let ReadScopeArgument::Parameter(reference) = read_scope else {
        unreachable!()
    };
    reference.binder = ReadScopeBinder::Declaration(*view);
    assert!(validate_owner_closure(package, &owners, &types).is_err());
}

#[test]
fn view_source_forwards_its_existing_scope_without_an_owned_source_row() {
    let (package, mut owners, types, function, source) = fixture();
    let PackageInterfaceRecord::Declaration(declaration) = &mut owners
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.read_scope_sources.clear();
    let ty = signature.result.ty;
    let PackageInterfaceRecord::Parameter(parameter) =
        &mut owners.get_mut(&OwnerKey::Parameter(source)).unwrap().record
    else {
        unreachable!()
    };
    parameter.ty = ty;
    validate_owner_closure(package, &owners, &types).unwrap();
}

#[test]
fn ordinary_composite_result_cannot_hide_a_read_view() {
    let (package, mut owners, mut types, function, _) = fixture();
    let PackageInterfaceRecord::Declaration(declaration) = &mut owners
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    let composite = TypeObject::new(TypeForm::List {
        item: signature.result.ty,
    })
    .unwrap();
    let digest = encode_type_object(&composite).unwrap().0;
    types.insert(digest, composite);
    signature.result = CallableResult {
        ty: digest,
        relation: ResultRelation::Value,
    };
    assert!(validate_owner_closure(package, &owners, &types).is_err());
}

#[test]
fn private_view_representation_does_not_grant_public_nominal_signature_access() {
    let (package, mut owners, types, _, _) = fixture();
    let key = owners
        .iter()
        .find_map(|(key, owner)| (owner.kind() == OwnerKind::ReadView).then_some(*key))
        .unwrap();
    let owner = owners.get_mut(&key).unwrap();
    let PackageInterfaceRecord::Declaration(declaration) = owner.record.clone() else {
        unreachable!()
    };
    owner.record = PackageInterfaceRecord::RepresentationDeclaration(declaration);
    assert!(validate_owner_closure(package, &owners, &types).is_err());
}

#[test]
fn read_from_view_source_retains_foreign_layout_obligation_and_owned_result() {
    let (package, mut owners, mut types, function, source) = fixture();
    let PackageInterfaceRecord::Declaration(declaration) = &mut owners
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    let old_view_type = signature.result.ty;
    let mut source_view = types[&old_view_type].clone();
    let TypeForm::ReadViewApplied {
        declaration: supplier,
        ..
    } = &mut source_view.form
    else {
        unreachable!()
    };
    supplier.package = PackageId::migrate(b"foreign-readable-view-supplier", 0);
    let source_view_type = encode_type_object(&source_view).unwrap().0;
    types.insert(source_view_type, source_view);
    let owned_result = types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::ByteBuffer).then_some(*ty))
        .unwrap();
    signature.result = CallableResult {
        ty: owned_result,
        relation: ResultRelation::ReadFrom { source },
    };
    signature.read_scope_sources.clear();
    let PackageInterfaceRecord::Parameter(parameter) =
        &mut owners.get_mut(&OwnerKey::Parameter(source)).unwrap().record
    else {
        unreachable!()
    };
    parameter.ty = source_view_type;
    validate_owner_closure(package, &owners, &types).unwrap();
    let PackageInterfaceRecord::Declaration(declaration) = &mut owners
        .get_mut(&OwnerKey::Declaration(function))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature.result.ty = source_view_type;
    assert!(validate_owner_closure(package, &owners, &types).is_err());
}

struct PrivateMappingInterface {
    package: PackageId,
    implementation: DeclarationReference,
    target: DeclarationReference,
    formal: ReadScopeParameter,
    unit: TypeObjectDigest,
    owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    types: BTreeMap<TypeObjectDigest, TypeObject>,
}

impl PrivateMappingInterface {
    fn new() -> Self {
        let seed = b"private-mapped-function-interface-obligation";
        let package = PackageId::migrate(seed, 0);
        let reference = |index| DeclarationReference {
            package,
            declaration: DeclarationId::migrate(seed, index),
        };
        let contract = reference(0);
        let implementation = reference(1);
        let target = reference(2);
        let self_parameter = TypeParameterId::migrate(seed, 0);
        let method = MethodId::migrate(seed, 0);
        let formal = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(seed, 0),
            name: Name::new("unused_scope").unwrap(),
        };
        let unit_object = TypeObject::new(TypeForm::Unit).unwrap();
        let unit = encode_type_object(&unit_object).unwrap().0;
        let cell_object = TypeObject::new(TypeForm::OwnedI64Cell).unwrap();
        let cell = encode_type_object(&cell_object).unwrap().0;
        let values = [
            PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(contract.declaration),
                    OwnerKind::OwnedContract,
                ),
                name: Name::new("Observe").unwrap(),
                payload: PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
                    self_parameter,
                    type_parameters: vec![],
                    associated_types: vec![],
                    methods: vec![OwnedMethod {
                        id: method,
                        name: Name::new("observe").unwrap(),
                        parameters: vec![],
                        result: CallableResult {
                            ty: unit,
                            relation: ResultRelation::Value,
                        },
                        read_scope_parameters: vec![formal.clone()],
                        read_scope_sources: vec![],
                        effect: FunctionEffect::Pure,
                    }],
                }),
            }),
            PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(self_parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: contract.declaration,
                name: Name::new("Self").unwrap(),
                constraints: TypeParameterConstraints::Owned,
            }),
            PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(implementation.declaration),
                    OwnerKind::OwnedImplementation,
                ),
                name: Name::new("PrivateObserve").unwrap(),
                payload: PackageInterfaceDeclarationPayload::OwnedImplementation(
                    OwnedImplementation {
                        type_parameters: vec![],
                        implementation_parameters: vec![],
                        contract,
                        self_type: cell,
                        type_arguments: vec![],
                        associated_types: vec![],
                        methods: vec![OwnedMethodImplementation {
                            method,
                            function: target,
                            type_arguments: vec![],
                            implementations: vec![],
                            read_scope_arguments: vec![ReadScopeArgument::Parameter(
                                ReadScopeReference {
                                    binder: ReadScopeBinder::ContractMethod { contract, method },
                                    parameter: formal.id,
                                },
                            )],
                        }],
                    },
                ),
            }),
        ];
        let owners = values
            .into_iter()
            .map(|record| {
                let owner = PackageInterfaceOwner {
                    contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                    record,
                };
                (owner.owner(), owner)
            })
            .collect();
        Self {
            package,
            implementation,
            target,
            formal,
            unit,
            owners,
            types: BTreeMap::from([(unit, unit_object), (cell, cell_object)]),
        }
    }

    fn mapping(&mut self) -> &mut OwnedMethodImplementation {
        let PackageInterfaceRecord::Declaration(declaration) = &mut self
            .owners
            .get_mut(&OwnerKey::Declaration(self.implementation.declaration))
            .unwrap()
            .record
        else {
            unreachable!()
        };
        let PackageInterfaceDeclarationPayload::OwnedImplementation(implementation) =
            &mut declaration.payload
        else {
            unreachable!()
        };
        &mut implementation.methods[0]
    }

    fn public_target(&mut self, formals: Vec<ReadScopeParameter>) {
        let owner = OwnerKey::Declaration(self.target.declaration);
        self.owners.insert(
            owner,
            PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                    header: OwnerHeader::new(owner, OwnerKind::PureFunction),
                    name: Name::new("observe_target").unwrap(),
                    payload: PackageInterfaceDeclarationPayload::Function(
                        PackageFunctionSignature {
                            implementation_parameters: vec![],
                            requirement_parameters: vec![],
                            effect_parameters: vec![],
                            type_parameters: vec![],
                            parameters: vec![],
                            result: CallableResult {
                                ty: self.unit,
                                relation: ResultRelation::Value,
                            },
                            read_scope_parameters: formals,
                            read_scope_sources: vec![],
                            effect: FunctionEffect::Pure,
                        },
                    ),
                }),
            },
        );
    }

    fn deferred(&self) -> Result<bool, Diagnostic> {
        let key = OwnerKey::Declaration(self.implementation.declaration);
        let PackageInterfaceRecord::Declaration(declaration) = &self.owners[&key].record else {
            unreachable!()
        };
        read_views::validate_declaration_resolved_metered(
            self.package,
            key,
            &declaration.payload,
            &self.owners,
            &self.types,
            &mut |_| Ok(None),
            &mut |_| Ok(()),
        )
    }

    fn transport(&self) -> Result<(), Diagnostic> {
        use crate::platform::package_transport::{
            PACKAGE_REVISION_CONTRACT_VERSION, PackageRevision,
        };
        let digest = PackageRevisionDigest::from_bytes([51; 32]);
        let revision = PackageRevision {
            contract_version: PACKAGE_REVISION_CONTRACT_VERSION,
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            package: self.package,
            revision: crate::platform::publication::RevisionCore {
                contract_version: crate::platform::publication::contract::REVISION_CONTRACT_VERSION,
                graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
                repository_id: RepositoryId::migrate(b"private-mapping-interface", 0),
                parents: vec![],
                semantic_state: SemanticStateDigest::from_bytes([52; 32]),
            },
            interface: PackageInterfaceDigest::from_bytes([53; 32]),
            dependencies: vec![],
        };
        let interface = PackageInterfaceValidation {
            owners: self.owners.clone(),
            type_objects: self.types.clone(),
            reachable_objects: BTreeSet::new(),
            map_work: MapWork::default(),
        };
        crate::platform::package_transport::validate_package_interface_closure(
            &BTreeMap::from([(digest, revision)]),
            &BTreeMap::from([(digest, interface)]),
        )
    }
}

#[test]
fn private_mapped_functions_are_deferred_without_public_interface_authority() {
    let mut fixture = PrivateMappingInterface::new();
    let hidden = OwnerKey::Declaration(fixture.target.declaration);
    assert!(!fixture.owners.contains_key(&hidden));
    assert_eq!(fixture.deferred(), Ok(true));
    validate_owner_closure(fixture.package, &fixture.owners, &fixture.types).unwrap();
    fixture.transport().unwrap();
    assert!(!fixture.owners.contains_key(&hidden));
    // A present public target is checked immediately and needs no private-source obligation.
    fixture.public_target(vec![fixture.formal.clone()]);
    assert_eq!(fixture.deferred(), Ok(false));
    validate_owner_closure(fixture.package, &fixture.owners, &fixture.types).unwrap();
    fixture.transport().unwrap();
}

#[test]
fn deferred_private_mappings_preserve_contract_scope_and_present_target_checks() {
    for case in 0..6 {
        let mut fixture = PrivateMappingInterface::new();
        match case {
            0 => {
                let ReadScopeArgument::Parameter(actual) =
                    &mut fixture.mapping().read_scope_arguments[0]
                else {
                    unreachable!()
                };
                let ReadScopeBinder::ContractMethod { method, .. } = &mut actual.binder else {
                    unreachable!()
                };
                *method = MethodId::migrate(b"foreign-mapping-method", 0);
            }
            1 => {
                let ReadScopeArgument::Parameter(actual) =
                    &mut fixture.mapping().read_scope_arguments[0]
                else {
                    unreachable!()
                };
                actual.parameter = ReadScopeParameterId::migrate(b"absent-mapping-formal", 0);
            }
            2 => {
                fixture.mapping().read_scope_arguments[0] = ReadScopeArgument::Lexical {
                    callable: fixture.target,
                    binding: BindingId::migrate(b"lexical-mapping", 0),
                }
            }
            3 => {
                let key = OwnerKey::Declaration(fixture.target.declaration);
                fixture.owners.insert(
                    key,
                    PackageInterfaceOwner {
                        contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                        record: PackageInterfaceRecord::Declaration(PackageInterfaceDeclaration {
                            header: OwnerHeader::new(key, OwnerKind::Constant),
                            name: Name::new("not_callable").unwrap(),
                            payload: PackageInterfaceDeclarationPayload::Constant {
                                ty: fixture.unit,
                            },
                        }),
                    },
                );
            }
            4 => fixture.public_target(vec![]),
            5 => {
                fixture.mapping().function.package =
                    PackageId::migrate(b"foreign-private-mapping", 0)
            }
            _ => unreachable!(),
        }
        if case == 5 {
            assert_eq!(fixture.deferred(), Ok(true)); // foreign dependency metadata is also incomplete here
        } else {
            assert!(fixture.deferred().is_err(), "interface case {case}");
        }
        assert!(fixture.transport().is_err(), "transport case {case}");
    }
}
