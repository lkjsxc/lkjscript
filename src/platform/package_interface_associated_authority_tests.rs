//! Raw canonical descriptor prerequisites. These do not admit executable source or mint a seal.
use super::*;
use crate::platform::kernel::*;
use crate::platform::package_transport::{PackageRevision, validate_package_interface_closure};
use crate::platform::semantic_id::*;

const SEED: &[u8] = b"associated-interface-authority";

fn current_header(owner: OwnerKey, kind: OwnerKind) -> OwnerHeader {
    OwnerHeader {
        contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        owner,
        kind,
    }
}

struct Fixture {
    consumer: PackageId,
    supplier: PackageId,
    contract: DeclarationReference,
    function: DeclarationReference,
    implementation: DeclarationReference,
    parameter: ParameterId,
    ordinary: TypeParameterId,
    witness: ImplementationParameterId,
    other_witness: ImplementationParameterId,
    out: AssociatedTypeMemberId,
    spare: AssociatedTypeMemberId,
    projection: TypeObjectDigest,
    contract_member: TypeObjectDigest,
    cell: TypeObjectDigest,
    scalar: TypeObjectDigest,
    consumer_interface: PackageInterfaceValidation,
    supplier_interface: PackageInterfaceValidation,
}

fn intern(types: &mut BTreeMap<TypeObjectDigest, TypeObject>, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    types.insert(digest, object);
    digest
}

fn owner(record: PackageInterfaceRecord) -> PackageInterfaceOwner {
    PackageInterfaceOwner {
        contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
        record,
    }
}

fn declaration(
    id: DeclarationId,
    kind: OwnerKind,
    name: &str,
    payload: PackageInterfaceDeclarationPayload,
) -> PackageInterfaceOwner {
    owner(PackageInterfaceRecord::Declaration(
        PackageInterfaceDeclaration {
            header: current_header(OwnerKey::Declaration(id), kind),
            name: Name::new(name).unwrap(),
            payload,
        },
    ))
}

fn interface(
    owners: BTreeMap<OwnerKey, PackageInterfaceOwner>,
    type_objects: BTreeMap<TypeObjectDigest, TypeObject>,
) -> PackageInterfaceValidation {
    PackageInterfaceValidation {
        owners,
        type_objects,
        reachable_objects: BTreeSet::new(),
        map_work: MapWork::default(),
    }
}

fn fixture() -> Fixture {
    let consumer = PackageId::migrate(SEED, 0);
    let supplier = PackageId::migrate(SEED, 1);
    let contract = DeclarationReference {
        package: supplier,
        declaration: DeclarationId::migrate(SEED, 0),
    };
    let function = DeclarationReference {
        package: consumer,
        declaration: DeclarationId::migrate(SEED, 1),
    };
    let implementation = DeclarationReference {
        package: supplier,
        declaration: DeclarationId::migrate(SEED, 2),
    };
    let mapped_function = DeclarationReference {
        package: supplier,
        declaration: DeclarationId::migrate(SEED, 3),
    };
    let ordinary = TypeParameterId::migrate(SEED, 0);
    let contract_self = TypeParameterId::migrate(SEED, 1);
    let parameter = ParameterId::migrate(SEED, 0);
    let mapped_parameter = ParameterId::migrate(SEED, 1);
    let witness = ImplementationParameterId::migrate(SEED, 0);
    let other_witness = ImplementationParameterId::migrate(SEED, 1);
    let out = AssociatedTypeMemberId::migrate(SEED, 0);
    let spare = AssociatedTypeMemberId::migrate(SEED, 1);
    let method = MethodId::migrate(SEED, 0);
    let mut consumer_types = BTreeMap::new();
    let self_type = intern(
        &mut consumer_types,
        TypeForm::TypeParameter {
            parameter: ordinary,
        },
    );
    let projection = intern(
        &mut consumer_types,
        TypeForm::AssociatedWitnessProjection {
            scope: function,
            parameter: witness,
            contract,
            member: out,
        },
    );
    let mut supplier_types = BTreeMap::new();
    let cell = intern(&mut supplier_types, TypeForm::OwnedI64Cell);
    let buffer = intern(&mut supplier_types, TypeForm::ByteBuffer);
    let scalar = intern(&mut supplier_types, TypeForm::I64);
    let contract_member = intern(
        &mut supplier_types,
        TypeForm::AssociatedContractMember {
            contract,
            member: out,
        },
    );
    let mut consumer_owners = BTreeMap::new();
    consumer_owners.insert(
        OwnerKey::Declaration(function.declaration),
        declaration(
            function.declaration,
            OwnerKind::PureFunction,
            "borrow_selected",
            PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                implementation_parameters: vec![
                    ImplementationParameter {
                        id: witness,
                        name: Name::new("reader").unwrap(),
                        contract,
                        self_type,
                        type_arguments: vec![],
                    },
                    ImplementationParameter {
                        id: other_witness,
                        name: Name::new("other_reader").unwrap(),
                        contract,
                        self_type,
                        type_arguments: vec![],
                    },
                ],
                requirement_parameters: vec![],
                effect_parameters: vec![],
                type_parameters: vec![ordinary],
                parameters: vec![parameter],
                result: CallableResult {
                    ty: projection,
                    relation: ResultRelation::ReadFrom { source: parameter },
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
            }),
        ),
    );
    consumer_owners.insert(
        OwnerKey::TypeParameter(ordinary),
        owner(PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: current_header(OwnerKey::TypeParameter(ordinary), OwnerKind::TypeParameter),
            declaration: function.declaration,
            name: Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        })),
    );
    consumer_owners.insert(
        OwnerKey::Parameter(parameter),
        owner(PackageInterfaceRecord::Parameter(ParameterRecord {
            header: current_header(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(function.declaration),
            name: Name::new("value").unwrap(),
            ty: projection,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        })),
    );
    let mut supplier_owners = BTreeMap::new();
    supplier_owners.insert(
        OwnerKey::Declaration(contract.declaration),
        declaration(
            contract.declaration,
            OwnerKind::OwnedContract,
            "Reader",
            PackageInterfaceDeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: contract_self,
                type_parameters: vec![],
                associated_types: vec![
                    AssociatedTypeMember {
                        id: spare,
                        name: Name::new("Spare").unwrap(),
                        bound: AssociatedTypeBound::Owned,
                    },
                    AssociatedTypeMember {
                        id: out,
                        name: Name::new("Out").unwrap(),
                        bound: AssociatedTypeBound::Owned,
                    },
                ],
                methods: vec![OwnedMethod {
                    id: method,
                    name: Name::new("read").unwrap(),
                    parameters: vec![OwnedMethodParameter {
                        ty: contract_member,
                        use_mode: ParameterUse::Borrow,
                    }],
                    result: CallableResult {
                        ty: contract_member,
                        relation: ResultRelation::ReadFrom { source: 0 },
                    },
                    read_scope_parameters: vec![],
                    read_scope_sources: vec![],
                    effect: FunctionEffect::Pure,
                }],
            }),
        ),
    );
    supplier_owners.insert(
        OwnerKey::TypeParameter(contract_self),
        owner(PackageInterfaceRecord::TypeParameter(TypeParameterRecord {
            header: current_header(
                OwnerKey::TypeParameter(contract_self),
                OwnerKind::TypeParameter,
            ),
            declaration: contract.declaration,
            name: Name::new("Self").unwrap(),
            constraints: TypeParameterConstraints::Owned,
        })),
    );
    let mut associated_types = vec![
        AssociatedTypeBinding {
            member: out,
            value: AssociatedTypeBindingValue::Owned { ty: cell },
        },
        AssociatedTypeBinding {
            member: spare,
            value: AssociatedTypeBindingValue::Owned { ty: buffer },
        },
    ];
    associated_types.sort_by_key(|binding| binding.member);
    supplier_owners.insert(
        OwnerKey::Declaration(implementation.declaration),
        declaration(
            implementation.declaration,
            OwnerKind::OwnedImplementation,
            "ReaderCell",
            PackageInterfaceDeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![],
                implementation_parameters: vec![],
                contract,
                self_type: cell,
                type_arguments: vec![],
                associated_types,
                methods: vec![OwnedMethodImplementation {
                    read_scope_arguments: vec![],
                    method,
                    function: mapped_function,
                    type_arguments: vec![],
                    implementations: vec![],
                }],
            }),
        ),
    );
    supplier_owners.insert(
        OwnerKey::Declaration(mapped_function.declaration),
        declaration(
            mapped_function.declaration,
            OwnerKind::PureFunction,
            "read_cell",
            PackageInterfaceDeclarationPayload::Function(PackageFunctionSignature {
                implementation_parameters: vec![],
                requirement_parameters: vec![],
                effect_parameters: vec![],
                type_parameters: vec![],
                parameters: vec![mapped_parameter],
                result: CallableResult {
                    ty: cell,
                    relation: ResultRelation::ReadFrom {
                        source: mapped_parameter,
                    },
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
            }),
        ),
    );
    supplier_owners.insert(
        OwnerKey::Parameter(mapped_parameter),
        owner(PackageInterfaceRecord::Parameter(ParameterRecord {
            header: current_header(OwnerKey::Parameter(mapped_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(mapped_function.declaration),
            name: Name::new("cell").unwrap(),
            ty: cell,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        })),
    );
    Fixture {
        consumer,
        supplier,
        contract,
        function,
        implementation,
        parameter,
        ordinary,
        witness,
        other_witness,
        out,
        spare,
        projection,
        contract_member,
        cell,
        scalar,
        consumer_interface: interface(consumer_owners, consumer_types),
        supplier_interface: interface(supplier_owners, supplier_types),
    }
}

fn local_proof(
    f: &Fixture,
    source: OwnerKey,
    form: &TypeForm,
) -> Result<AssociatedInterfaceProof, Diagnostic> {
    AssociatedInterfaceAdmission::new(&mut |_| Ok(())).validate_form(
        f.consumer,
        source,
        form,
        &f.consumer_interface.owners,
        &f.consumer_interface.type_objects,
        &mut |_| Ok(None),
    )
}

fn function_mut(f: &mut Fixture) -> &mut PackageFunctionSignature {
    let PackageInterfaceRecord::Declaration(declaration) = &mut f
        .consumer_interface
        .owners
        .get_mut(&OwnerKey::Declaration(f.function.declaration))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut declaration.payload else {
        unreachable!()
    };
    signature
}

fn bindings_mut(f: &mut Fixture) -> &mut Vec<AssociatedTypeBinding> {
    let PackageInterfaceRecord::Declaration(declaration) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Declaration(f.implementation.declaration))
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
    &mut implementation.associated_types
}

fn private_output(f: &mut Fixture) -> (TypeObjectDigest, FieldId) {
    let declaration_id = DeclarationId::migrate(SEED, 4);
    let field_id = FieldId::migrate(SEED, 0);
    let reference = DeclarationReference {
        package: f.supplier,
        declaration: declaration_id,
    };
    let ty = intern(
        &mut f.supplier_interface.type_objects,
        TypeForm::Named {
            declaration: reference,
        },
    );
    let value = declaration(
        declaration_id,
        OwnerKind::OwnedRecord,
        "PrivateCell",
        PackageInterfaceDeclarationPayload::OwnedRecord {
            type_parameters: vec![],
            fields: vec![field_id],
        },
    );
    let PackageInterfaceRecord::Declaration(record) = value.record else {
        unreachable!()
    };
    f.supplier_interface.owners.insert(
        OwnerKey::Declaration(declaration_id),
        owner(PackageInterfaceRecord::RepresentationDeclaration(record)),
    );
    f.supplier_interface.owners.insert(
        OwnerKey::Field(field_id),
        owner(PackageInterfaceRecord::Field(FieldRecord {
            header: current_header(OwnerKey::Field(field_id), OwnerKind::Field),
            declaration: declaration_id,
            name: Name::new("cell").unwrap(),
            ty: f.cell,
        })),
    );
    let out = f.out;
    bindings_mut(f)
        .iter_mut()
        .find(|binding| binding.member == out)
        .unwrap()
        .value = AssociatedTypeBindingValue::Owned { ty };
    let mapped = DeclarationId::migrate(SEED, 3);
    let PackageInterfaceRecord::Declaration(record) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Declaration(mapped))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut record.payload else {
        unreachable!()
    };
    signature.result.ty = ty;
    let PackageInterfaceRecord::Parameter(parameter) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Parameter(ParameterId::migrate(SEED, 1)))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    parameter.ty = ty;
    (ty, field_id)
}

// Full descriptor fixtures reach private layout through a public owned nominal. Public
// signatures and implementation bindings cannot directly name a private representation.
fn public_private_output(f: &mut Fixture) -> (TypeObjectDigest, FieldId) {
    let (private, private_field) = private_output(f);
    let declaration_id = DeclarationId::migrate(SEED, 5);
    let field_id = FieldId::migrate(SEED, 1);
    let ty = intern(
        &mut f.supplier_interface.type_objects,
        TypeForm::Named {
            declaration: DeclarationReference {
                package: f.supplier,
                declaration: declaration_id,
            },
        },
    );
    f.supplier_interface.owners.insert(
        OwnerKey::Declaration(declaration_id),
        declaration(
            declaration_id,
            OwnerKind::OwnedRecord,
            "PublicCell",
            PackageInterfaceDeclarationPayload::OwnedRecord {
                type_parameters: vec![],
                fields: vec![field_id],
            },
        ),
    );
    f.supplier_interface.owners.insert(
        OwnerKey::Field(field_id),
        owner(PackageInterfaceRecord::Field(FieldRecord {
            header: current_header(OwnerKey::Field(field_id), OwnerKind::Field),
            declaration: declaration_id,
            name: Name::new("private_cell").unwrap(),
            ty: private,
        })),
    );
    let out = f.out;
    bindings_mut(f)
        .iter_mut()
        .find(|binding| binding.member == out)
        .unwrap()
        .value = AssociatedTypeBindingValue::Owned { ty };
    let mapped = DeclarationId::migrate(SEED, 3);
    let PackageInterfaceRecord::Declaration(record) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Declaration(mapped))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &mut record.payload else {
        unreachable!()
    };
    signature.result.ty = ty;
    let PackageInterfaceRecord::Parameter(parameter) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Parameter(ParameterId::migrate(SEED, 1)))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    parameter.ty = ty;
    (ty, private_field)
}

fn revision(package: PackageId, n: u8, dependencies: Vec<DependencyRecord>) -> PackageRevision {
    PackageRevision {
        contract_version: 1,
        graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        package,
        revision: crate::platform::publication::RevisionCore {
            contract_version: crate::platform::publication::contract::REVISION_CONTRACT_VERSION,
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(SEED, u64::from(n)),
            parents: vec![],
            semantic_state: SemanticStateDigest::from_bytes([n; 32]),
        },
        interface: PackageInterfaceDigest::from_bytes([n; 32]),
        dependencies,
    }
}

fn closure(
    f: Fixture,
) -> (
    BTreeMap<PackageRevisionDigest, PackageRevision>,
    BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
    PackageRevisionDigest,
    PackageRevisionDigest,
) {
    let supplier = revision(f.supplier, 1, vec![]);
    let supplier_id = supplier.encode().unwrap().0;
    let consumer = revision(
        f.consumer,
        2,
        vec![DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: f.supplier,
            semantic_revision: supplier.revision.revision_id().unwrap(),
            package_revision: supplier_id,
        }],
    );
    let consumer_id = consumer.encode().unwrap().0;
    (
        BTreeMap::from([(supplier_id, supplier), (consumer_id, consumer)]),
        BTreeMap::from([
            (supplier_id, f.supplier_interface),
            (consumer_id, f.consumer_interface),
        ]),
        consumer_id,
        supplier_id,
    )
}

#[test]
fn associated_interface_local_proof_retains_foreign_obligation_and_exact_lexical_coordinates() {
    let f = fixture();
    let form = &f.consumer_interface.type_objects[&f.projection].form;
    assert!(
        matches!(form, TypeForm::AssociatedWitnessProjection { parameter, member, .. } if *parameter == f.witness && *member == f.out)
    );
    assert_eq!(
        local_proof(&f, OwnerKey::Declaration(f.function.declaration), form).unwrap(),
        AssociatedInterfaceProof::Deferred(f.contract)
    );
    assert_eq!(
        local_proof(&f, OwnerKey::Parameter(f.parameter), form).unwrap(),
        AssociatedInterfaceProof::Deferred(f.contract)
    );
    for mutation in 0..5 {
        let mut form = form.clone();
        let TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract,
            member,
        } = &mut form
        else {
            unreachable!()
        };
        match mutation {
            0 => scope.declaration = DeclarationId::migrate(SEED, 99),
            1 => *parameter = ImplementationParameterId::migrate(SEED, 99),
            2 => contract.declaration = DeclarationId::migrate(SEED, 99),
            3 => scope.package = f.supplier,
            _ => {
                *parameter = f.other_witness;
                *member = f.spare;
            }
        }
        let result = local_proof(&f, OwnerKey::Parameter(f.parameter), &form);
        if mutation == 4 {
            assert_eq!(
                result.unwrap(),
                AssociatedInterfaceProof::Deferred(f.contract)
            );
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Semantic);
        }
    }
    let mut wrong_self = fixture();
    let wrong_self_type = wrong_self.projection;
    function_mut(&mut wrong_self).implementation_parameters[0].self_type = wrong_self_type;
    assert!(local_proof(&wrong_self, OwnerKey::Parameter(wrong_self.parameter), form).is_err());
    let mut orphan = fixture();
    function_mut(&mut orphan).parameters.clear();
    assert!(local_proof(&orphan, OwnerKey::Parameter(orphan.parameter), form).is_err());
}

#[test]
fn associated_interface_occurrences_require_current_owners_and_forbid_private_field_authority() {
    for version in [0, u16::MAX] {
        for key_kind in 0..3 {
            let mut f = fixture();
            let key = match key_kind {
                0 => OwnerKey::Parameter(f.parameter),
                1 => OwnerKey::Declaration(f.function.declaration),
                _ => OwnerKey::TypeParameter(f.ordinary),
            };
            let value = f.consumer_interface.owners.get_mut(&key).unwrap();
            match &mut value.record {
                PackageInterfaceRecord::Parameter(p) => p.header.contract_version = version,
                PackageInterfaceRecord::Declaration(d) => d.header.contract_version = version,
                PackageInterfaceRecord::TypeParameter(p) => p.header.contract_version = version,
                _ => unreachable!(),
            }
            assert!(
                local_proof(
                    &f,
                    OwnerKey::Parameter(f.parameter),
                    &f.consumer_interface.type_objects[&f.projection].form
                )
                .is_err()
            );
        }
    }
    let mut f = fixture();
    let field = FieldId::migrate(SEED, 0);
    f.consumer_interface.owners.insert(
        OwnerKey::Field(field),
        owner(PackageInterfaceRecord::Field(FieldRecord {
            header: current_header(OwnerKey::Field(field), OwnerKind::Field),
            declaration: f.function.declaration,
            name: Name::new("private_view").unwrap(),
            ty: f.projection,
        })),
    );
    assert!(
        local_proof(
            &f,
            OwnerKey::Field(field),
            &f.consumer_interface.type_objects[&f.projection].form
        )
        .is_err()
    );
    f.consumer_interface
        .owners
        .get_mut(&OwnerKey::Parameter(f.parameter))
        .unwrap()
        .contract_version = 0;
    assert!(
        local_proof(
            &f,
            OwnerKey::Parameter(f.parameter),
            &f.consumer_interface.type_objects[&f.projection].form
        )
        .is_err()
    );
}

#[test]
fn associated_interface_exact_closure_discharges_foreign_obligations_and_checks_unused_bindings() {
    let (revisions, interfaces, _, _) = closure(fixture());
    validate_package_interface_closure(&revisions, &interfaces).unwrap();
    let mut private = fixture();
    let (_, field) = private_output(&mut private);
    let (revisions, interfaces, _, _) = closure(private);
    validate_package_interface_closure(&revisions, &interfaces).unwrap();
    let mut private = fixture();
    private_output(&mut private);
    let forbidden = private.contract_member;
    let PackageInterfaceRecord::Field(record) = &mut private
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Field(field))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    record.ty = forbidden;
    let (revisions, interfaces, _, _) = closure(private);
    assert!(validate_package_interface_closure(&revisions, &interfaces).is_err());
    for mutation in 0..7 {
        let mut f = fixture();
        match mutation {
            0 => {
                bindings_mut(&mut f).pop();
            }
            1 => {
                let scalar = f.scalar;
                let spare = f.spare;
                bindings_mut(&mut f)
                    .iter_mut()
                    .find(|binding| binding.member == spare)
                    .unwrap()
                    .value = AssociatedTypeBindingValue::Owned { ty: scalar };
            }
            2 => {
                let projection = f.contract_member;
                bindings_mut(&mut f)[0].value =
                    AssociatedTypeBindingValue::Owned { ty: projection };
            }
            3 => {
                bindings_mut(&mut f).reverse();
            }
            4 => {
                let invalid = AssociatedTypeMemberId::migrate(SEED, 99);
                bindings_mut(&mut f)[0].member = invalid;
            }
            5 => {
                let projection = intern(
                    &mut f.consumer_interface.type_objects,
                    TypeForm::AssociatedWitnessProjection {
                        scope: f.function,
                        parameter: f.witness,
                        contract: f.contract,
                        member: AssociatedTypeMemberId::migrate(SEED, 99),
                    },
                );
                function_mut(&mut f).result.ty = projection;
            }
            _ => {
                let id = ParameterId::migrate(SEED, 99);
                f.consumer_interface.owners.insert(
                    OwnerKey::Parameter(id),
                    owner(PackageInterfaceRecord::Parameter(ParameterRecord {
                        header: current_header(OwnerKey::Parameter(id), OwnerKind::Parameter),
                        parent: ParameterParent::Function(f.function.declaration),
                        name: Name::new("orphan").unwrap(),
                        ty: f.projection,
                        use_mode: ParameterUse::Borrow,
                        resource_requirement: None,
                    })),
                );
            }
        }
        let (revisions, interfaces, _, _) = closure(f);
        assert_eq!(
            validate_package_interface_closure(&revisions, &interfaces)
                .unwrap_err()
                .class,
            DiagnosticClass::Semantic
        );
    }
}

#[test]
fn associated_interface_exact_closure_rejects_foreign_selection_and_representation_authority() {
    for version in [0, u16::MAX] {
        for mutation in 0..2 {
            let (mut revisions, interfaces, consumer, supplier) = closure(fixture());
            let selected = revisions
                .get_mut(&if mutation == 0 { consumer } else { supplier })
                .unwrap();
            selected.graph_contract_version = version;
            selected.revision.graph_contract_version = version;
            assert_eq!(
                validate_package_interface_closure(&revisions, &interfaces)
                    .unwrap_err()
                    .class,
                DiagnosticClass::Semantic
            );
        }
    }
    // The outer envelope does not upgrade the selected semantic meaning.
    for mutation in 0..2 {
        let (mut revisions, interfaces, consumer, supplier) = closure(fixture());
        revisions
            .get_mut(&if mutation == 0 { consumer } else { supplier })
            .unwrap()
            .revision
            .graph_contract_version = u16::MAX;
        assert_eq!(
            validate_package_interface_closure(&revisions, &interfaces)
                .unwrap_err()
                .class,
            DiagnosticClass::Semantic
        );
    }
    let f = fixture();
    let contract_key = OwnerKey::Declaration(f.contract.declaration);
    let (mut revisions, interfaces, consumer, _) = closure(f);
    revisions.get_mut(&consumer).unwrap().dependencies.clear();
    assert!(validate_package_interface_closure(&revisions, &interfaces).is_err());
    let (revisions, mut interfaces, _, supplier) = closure(fixture());
    let value = interfaces
        .get_mut(&supplier)
        .unwrap()
        .owners
        .get_mut(&contract_key)
        .unwrap();
    let PackageInterfaceRecord::Declaration(declaration) = value.record.clone() else {
        unreachable!()
    };
    value.record = PackageInterfaceRecord::RepresentationDeclaration(declaration);
    assert!(validate_package_interface_closure(&revisions, &interfaces).is_err());
}

#[test]
fn associated_interface_shared_walk_preserves_resource_and_cancellation_failures() {
    let f = fixture();
    let mut charged = 0;
    let mut visit = |count| {
        charged += count;
        if charged > 1 {
            Err(Diagnostic::new(
                DiagnosticClass::Resource,
                "fixture_capacity",
                "shared capacity exhausted",
            ))
        } else {
            Ok(())
        }
    };
    let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
    let mut unresolved = |_| Ok(None);
    assert_eq!(
        admission
            .validate_occurrences(
                f.consumer,
                OwnerKey::Parameter(f.parameter),
                &[f.projection],
                false,
                &f.consumer_interface.owners,
                &f.consumer_interface.type_objects,
                &mut unresolved
            )
            .unwrap_err()
            .class,
        DiagnosticClass::Resource
    );
    let mut cancelled = |_| {
        Err(Diagnostic::new(
            DiagnosticClass::Cancelled,
            "fixture_cancelled",
            "cancelled",
        ))
    };
    let mut admission = AssociatedInterfaceAdmission::new(&mut cancelled);
    assert_eq!(
        admission
            .validate_occurrences(
                f.consumer,
                OwnerKey::Parameter(f.parameter),
                &[f.projection],
                false,
                &f.consumer_interface.owners,
                &f.consumer_interface.type_objects,
                &mut unresolved
            )
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(admission.storage.bytes, 0);
    let mut visit = |_| Ok(());
    let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
    admission.storage.maximum = 0;
    assert_eq!(
        admission
            .validate_occurrences(
                f.consumer,
                OwnerKey::Parameter(f.parameter),
                &[f.projection],
                false,
                &f.consumer_interface.owners,
                &f.consumer_interface.type_objects,
                &mut unresolved
            )
            .unwrap_err()
            .class,
        DiagnosticClass::Resource
    );
    // Measure allowance use, then share one ledger across two exact occurrences. This is
    // a capacity boundary control, not a semantic oracle from measured production work.
    let mut complete = 0_u64;
    {
        let mut visit = |count| {
            complete += count;
            Ok(())
        };
        let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
        for source in [
            OwnerKey::Parameter(f.parameter),
            OwnerKey::Declaration(f.function.declaration),
        ] {
            assert_eq!(
                admission
                    .validate_form(
                        f.consumer,
                        source,
                        &f.consumer_interface.type_objects[&f.projection].form,
                        &f.consumer_interface.owners,
                        &f.consumer_interface.type_objects,
                        &mut |_| Ok(None)
                    )
                    .unwrap(),
                AssociatedInterfaceProof::Deferred(f.contract)
            );
        }
    }
    for maximum in [complete - 1, complete] {
        let mut used = 0;
        let mut visit = |count| {
            used += count;
            if used > maximum {
                Err(Diagnostic::new(
                    DiagnosticClass::Resource,
                    "fixture_capacity",
                    "shared capacity exhausted",
                ))
            } else {
                Ok(())
            }
        };
        let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
        let result = [
            OwnerKey::Parameter(f.parameter),
            OwnerKey::Declaration(f.function.declaration),
        ]
        .into_iter()
        .try_for_each(|source| {
            admission
                .validate_form(
                    f.consumer,
                    source,
                    &f.consumer_interface.type_objects[&f.projection].form,
                    &f.consumer_interface.owners,
                    &f.consumer_interface.type_objects,
                    &mut |_| Ok(None),
                )
                .map(|_| ())
        });
        if maximum == complete {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Resource);
        }
    }
}

#[test]
fn associated_interface_public_descriptors_preserve_contextual_owned_authority_before_exact_closure()
 {
    let f = fixture();
    let (_, consumer) = load_public_interface(f.consumer, &f.consumer_interface).unwrap();
    let (_, supplier) = load_public_interface(f.supplier, &f.supplier_interface).unwrap();
    // Complete descriptor validation checks actual occurrence authority. Context-free lookup
    // still cannot grant a projected type Owned, Transferable, Shareable or capture properties.
    assert!(!interface_owned_type(
        f.consumer,
        f.projection,
        &consumer.type_objects,
        &consumer.owners
    ));
    assert!(!interface_owned_type(
        f.supplier,
        f.contract_member,
        &supplier.type_objects,
        &supplier.owners
    ));
    let selected = public_fixture_closure(f).unwrap();
    validate_package_interface_closure(&selected.revisions, &selected.interfaces).unwrap();

    let local = local_function_fixture();
    load_public_interface(local.consumer, &local.consumer_interface).unwrap();
    let mut private = fixture();
    public_private_output(&mut private);
    let selected = public_fixture_closure(private).unwrap();
    validate_package_interface_closure(&selected.revisions, &selected.interfaces).unwrap();
    // Isolate an inventory-only interface: nonempty members/bindings are meaningful even when
    // no reachable signature type uses a member form. The unused inventory remains complete.
    let mut inventory_only = fixture();
    let cell = inventory_only.cell;
    method_mut(&mut inventory_only).parameters[0].ty = cell;
    method_mut(&mut inventory_only).result.ty = cell;
    let (_, supplier) =
        load_public_interface(inventory_only.supplier, &inventory_only.supplier_interface).unwrap();
    assert!(
        supplier
            .type_objects
            .values()
            .all(|object| !object.form.is_associated())
    );
    let selected = public_fixture_closure(inventory_only).unwrap();
    validate_package_interface_closure(&selected.revisions, &selected.interfaces).unwrap();
}

fn function_signature(f: &Fixture) -> &PackageFunctionSignature {
    let PackageInterfaceRecord::Declaration(declaration) =
        &f.consumer_interface.owners[&OwnerKey::Declaration(f.function.declaration)].record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::Function(signature) = &declaration.payload else {
        unreachable!()
    };
    signature
}

fn function_borrow_proof(
    f: &Fixture,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<Option<[InterfaceBorrowRequirementProof; 2]>, Diagnostic> {
    validate_interface_function_borrow_result(
        f.consumer,
        f.function.declaration,
        function_signature(f),
        &f.consumer_interface.type_objects,
        &f.consumer_interface.owners,
        admission,
    )
}

fn method_mut(f: &mut Fixture) -> &mut OwnedMethod {
    let PackageInterfaceRecord::Declaration(declaration) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Declaration(f.contract.declaration))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &mut declaration.payload
    else {
        unreachable!()
    };
    &mut contract.methods[0]
}

fn method_borrow_proof(
    f: &Fixture,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<Option<[InterfaceBorrowRequirementProof; 2]>, Diagnostic> {
    let PackageInterfaceRecord::Declaration(declaration) =
        &f.supplier_interface.owners[&OwnerKey::Declaration(f.contract.declaration)].record
    else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &declaration.payload else {
        unreachable!()
    };
    validate_interface_method_borrow_result(
        f.supplier,
        OwnerKey::Declaration(f.contract.declaration),
        &contract.methods[0],
        &f.supplier_interface.type_objects,
        &f.supplier_interface.owners,
        admission,
    )
}

fn local_function_fixture() -> Fixture {
    let mut f = fixture();
    let local_contract = DeclarationReference {
        package: f.consumer,
        declaration: f.contract.declaration,
    };
    let local_member = intern(
        &mut f.consumer_interface.type_objects,
        TypeForm::AssociatedContractMember {
            contract: local_contract,
            member: f.out,
        },
    );
    let mut contract_owner =
        f.supplier_interface.owners[&OwnerKey::Declaration(f.contract.declaration)].clone();
    let PackageInterfaceRecord::Declaration(declaration) = &mut contract_owner.record else {
        unreachable!()
    };
    let PackageInterfaceDeclarationPayload::OwnedContract(contract) = &mut declaration.payload
    else {
        unreachable!()
    };
    // Preserve a complete, structurally valid local contract. Its borrowed method uses the local
    // member coordinate; the symbolic function needs no selected concrete implementation.
    contract.methods[0].parameters[0].ty = local_member;
    contract.methods[0].result.ty = local_member;
    contract.validate_local().unwrap();
    let parameter = OwnerKey::TypeParameter(contract.self_parameter);
    f.consumer_interface
        .owners
        .insert(parameter, f.supplier_interface.owners[&parameter].clone());
    f.consumer_interface.owners.insert(
        OwnerKey::Declaration(f.contract.declaration),
        contract_owner,
    );
    f.contract = local_contract;
    let contract = f.contract;
    for parameter in &mut function_mut(&mut f).implementation_parameters {
        parameter.contract = contract;
    }
    let projection = intern(
        &mut f.consumer_interface.type_objects,
        TypeForm::AssociatedWitnessProjection {
            scope: f.function,
            parameter: f.witness,
            contract,
            member: f.out,
        },
    );
    f.projection = projection;
    function_mut(&mut f).result.ty = projection;
    let PackageInterfaceRecord::Parameter(parameter) = &mut f
        .consumer_interface
        .owners
        .get_mut(&OwnerKey::Parameter(f.parameter))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    parameter.ty = projection;
    f
}

#[test]
fn associated_interface_borrow_checks_route_contract_and_function_owned_requirements() {
    assert_eq!(
        method_borrow_proof(
            &fixture(),
            &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))
        )
        .unwrap(),
        Some([InterfaceBorrowRequirementProof::Owned; 2])
    );
    assert_eq!(
        function_borrow_proof(
            &local_function_fixture(),
            &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))
        )
        .unwrap(),
        Some([InterfaceBorrowRequirementProof::Owned; 2])
    );
    for mutation in 0..4 {
        let mut method = fixture();
        let scalar = method.scalar;
        let selected = method_mut(&mut method);
        match mutation {
            0 => selected.parameters[0].use_mode = ParameterUse::Consume,
            1 => selected.parameters[0].ty = scalar,
            2 => selected.result.ty = scalar,
            _ => {
                selected.effect = FunctionEffect::Task {
                    effect_parameters: vec![],
                    requirements: vec![],
                }
            }
        }
        assert!(
            method_borrow_proof(
                &method,
                &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))
            )
            .is_err()
        );
        let mut function = local_function_fixture();
        let scalar = intern(&mut function.consumer_interface.type_objects, TypeForm::I64);
        match mutation {
            0 | 1 => {
                let PackageInterfaceRecord::Parameter(parameter) = &mut function
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Parameter(function.parameter))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                if mutation == 0 {
                    parameter.use_mode = ParameterUse::Consume;
                } else {
                    parameter.ty = scalar;
                }
            }
            2 => function_mut(&mut function).result.ty = scalar,
            _ => {
                function_mut(&mut function).effect = FunctionEffect::Task {
                    effect_parameters: vec![],
                    requirements: vec![],
                };
                let PackageInterfaceRecord::Declaration(declaration) = &mut function
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Declaration(function.function.declaration))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                declaration.header.kind = OwnerKind::TaskFunction;
            }
        }
        assert!(
            function_borrow_proof(
                &function,
                &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))
            )
            .is_err()
        );
    }
}

#[test]
fn associated_interface_borrow_foreign_requirements_remain_deferred_until_exact_closure() {
    let f = fixture();
    assert_eq!(
        function_borrow_proof(&f, &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))).unwrap(),
        Some([InterfaceBorrowRequirementProof::DeferredAssociated(f.contract); 2])
    );
    let nominal = DeclarationReference {
        package: f.supplier,
        declaration: DeclarationId::migrate(SEED, 88),
    };
    let mut nominal_types = f.consumer_interface.type_objects.clone();
    let named = intern(
        &mut nominal_types,
        TypeForm::Named {
            declaration: nominal,
        },
    );
    assert_eq!(
        interface_borrow_requirement_at(
            f.consumer,
            OwnerKey::Parameter(f.parameter),
            named,
            &nominal_types,
            &f.consumer_interface.owners,
            &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))
        )
        .unwrap(),
        Some(InterfaceBorrowRequirementProof::DeferredNominal(nominal))
    );
    for missing in [false, true] {
        let mut f = fixture();
        let projection = intern(
            &mut f.consumer_interface.type_objects,
            TypeForm::AssociatedWitnessProjection {
                scope: f.function,
                parameter: f.other_witness,
                contract: f.contract,
                member: if missing {
                    AssociatedTypeMemberId::migrate(SEED, 99)
                } else {
                    f.spare
                },
            },
        );
        function_mut(&mut f).result.ty = projection;
        // Equal Self does not choose the first witness or infer a member. The unresolved exact
        // foreign member is a requirement only; the retained closure decides its validity.
        assert_eq!(
            function_borrow_proof(&f, &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(())))
                .unwrap(),
            Some([InterfaceBorrowRequirementProof::DeferredAssociated(f.contract); 2])
        );
        assert!(!interface_owned_type(
            f.consumer,
            projection,
            &f.consumer_interface.type_objects,
            &f.consumer_interface.owners
        ));
        let (revisions, interfaces, _, _) = closure(f);
        let result = validate_package_interface_closure(&revisions, &interfaces);
        if missing {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Semantic);
        } else {
            result.unwrap();
        }
    }
    let f = fixture();
    assert!(
        function_borrow_proof(&f, &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(()))).is_ok()
    );
    let contract_key = OwnerKey::Declaration(f.contract.declaration);
    let (revisions, mut interfaces, _, supplier) = closure(f);
    let supplier = interfaces.get_mut(&supplier).unwrap();
    let contract = supplier.owners.get_mut(&contract_key).unwrap();
    let PackageInterfaceRecord::Declaration(declaration) = contract.record.clone() else {
        unreachable!()
    };
    contract.record = PackageInterfaceRecord::RepresentationDeclaration(declaration);
    assert_eq!(
        validate_package_interface_closure(&revisions, &interfaces)
            .unwrap_err()
            .class,
        DiagnosticClass::Semantic
    );
}

#[test]
fn associated_interface_borrow_function_uses_exact_source_parameter_owner() {
    for mutation in 0..7 {
        let mut f = fixture();
        match mutation {
            0 => function_mut(&mut f).parameters.clear(),
            1 => {
                let id = f.parameter;
                function_mut(&mut f).parameters.push(id);
            }
            2 => {
                let PackageInterfaceRecord::Parameter(parameter) = &mut f
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Parameter(f.parameter))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                parameter.parent = ParameterParent::Function(DeclarationId::migrate(SEED, 99));
            }
            3 | 4 => {
                let PackageInterfaceRecord::Parameter(parameter) = &mut f
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Parameter(f.parameter))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                parameter.header.contract_version = if mutation == 3 { 0 } else { u16::MAX };
            }
            5 => {
                let PackageInterfaceRecord::Parameter(parameter) = &mut f
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Parameter(f.parameter))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                parameter.header.owner = OwnerKey::Parameter(ParameterId::migrate(SEED, 99));
            }
            _ => {
                let scalar = intern(&mut f.consumer_interface.type_objects, TypeForm::ByteBuffer);
                function_mut(&mut f).result.ty = scalar;
                let PackageInterfaceRecord::Parameter(parameter) = &mut f
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Parameter(f.parameter))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                parameter.ty = scalar;
                parameter.header.kind = OwnerKind::Binding;
            }
        }
        assert!(
            function_borrow_proof(&f, &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(())))
                .is_err()
        );
    }
    function_borrow_proof(
        &fixture(),
        &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(())),
    )
    .unwrap();
}

#[test]
fn associated_interface_borrow_source_and_result_share_capacity_and_cancellation() {
    use crate::platform::execution::ExecutionControl;
    use crate::platform::execution::normalized::execution_diagnostic;
    let f = fixture();
    let mut complete = 0_u64;
    {
        let mut visit = |count| {
            complete += count;
            Ok(())
        };
        function_borrow_proof(&f, &mut AssociatedInterfaceAdmission::new(&mut visit)).unwrap();
    }
    let mut source_work = 0_u64;
    {
        let mut visit = |count| {
            source_work += count;
            Ok(())
        };
        interface_borrow_requirement_at(
            f.consumer,
            OwnerKey::Parameter(f.parameter),
            f.projection,
            &f.consumer_interface.type_objects,
            &f.consumer_interface.owners,
            &mut AssociatedInterfaceAdmission::new(&mut visit),
        )
        .unwrap();
    }
    assert!(complete > source_work && source_work > 0);
    // These measured fits test cumulative capacity, not semantic soundness from a finite prefix.
    for maximum in [complete - 1, complete] {
        let control = ExecutionControl::uncancelled();
        let mut visit = |_| control.check().map_err(execution_diagnostic);
        let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
        admission.maximum_work = usize::try_from(maximum).unwrap();
        let result = function_borrow_proof(&f, &mut admission);
        if maximum == complete {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Resource);
        }
    }
    let control = ExecutionControl::uncancelled();
    let mut charged = 0_u64;
    {
        let mut visit = |count| {
            if charged == source_work && count == 0 {
                control.cancel();
            }
            control.check().map_err(execution_diagnostic)?;
            charged += count;
            Ok(())
        };
        let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
        let error = function_borrow_proof(&f, &mut admission).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Cancelled);
        assert_eq!(error.code, "execution_cancelled");
        assert_eq!(admission.work, usize::try_from(source_work).unwrap());
    }
    assert!(control.is_cancelled());
    assert_eq!(charged, source_work);
    // Entry cancellation also precedes a malformed source lookup, retaining no proof growth.
    let mut malformed = fixture();
    function_mut(&mut malformed).result.relation = ResultRelation::ReadFrom {
        source: ParameterId::migrate(SEED, 99),
    };
    let control = ExecutionControl::uncancelled();
    control.cancel();
    let mut visit = |_| control.check().map_err(execution_diagnostic);
    let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
    assert_eq!(
        function_borrow_proof(&malformed, &mut admission)
            .unwrap_err()
            .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(admission.work, 0);
    assert_eq!(admission.storage.bytes, 0);
    let fresh = ExecutionControl::uncancelled();
    function_borrow_proof(
        &f,
        &mut AssociatedInterfaceAdmission::new(&mut |_| {
            fresh.check().map_err(execution_diagnostic)
        }),
    )
    .unwrap();
}

fn load_public_interface(
    package: PackageId,
    interface: &PackageInterfaceValidation,
) -> Result<(PackageInterfaceDigest, PackageInterfaceValidation), Diagnostic> {
    let types: BTreeMap<_, _> = interface
        .type_objects
        .iter()
        .map(|(digest, object)| encode_type_object(object).map(|(_, bytes)| (*digest, bytes)))
        .collect::<Result<_, _>>()?;
    let built = build_package_interface(&interface.owners, &types)?;
    let validated = validate_package_interface(
        package,
        built.root,
        &built.objects,
        &mut StoreWork::default(),
    )?;
    Ok((
        package_interface_digest(package, built.root.content_root())?,
        validated,
    ))
}

struct PublicFixtureClosure {
    revisions: BTreeMap<PackageRevisionDigest, PackageRevision>,
    interfaces: BTreeMap<PackageRevisionDigest, PackageInterfaceValidation>,
}

fn public_fixture_closure(f: Fixture) -> Result<PublicFixtureClosure, Diagnostic> {
    public_fixture_closure_with(f, load_public_interface)
}

fn public_fixture_closure_with(
    f: Fixture,
    mut load: impl FnMut(
        PackageId,
        &PackageInterfaceValidation,
    )
        -> Result<(PackageInterfaceDigest, PackageInterfaceValidation), Diagnostic>,
) -> Result<PublicFixtureClosure, Diagnostic> {
    let (supplier_root, supplier_interface) = load(f.supplier, &f.supplier_interface)?;
    let (consumer_root, consumer_interface) = load(f.consumer, &f.consumer_interface)?;
    let mut supplier = revision(f.supplier, 1, vec![]);
    supplier.interface = supplier_root;
    let supplier_id = supplier.encode()?.0;
    let mut consumer = revision(
        f.consumer,
        2,
        vec![DependencyRecord {
            graph_contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
            package: f.supplier,
            semantic_revision: supplier.revision.revision_id()?,
            package_revision: supplier_id,
        }],
    );
    consumer.interface = consumer_root;
    let consumer_id = consumer.encode()?.0;
    Ok(PublicFixtureClosure {
        revisions: BTreeMap::from([(supplier_id, supplier), (consumer_id, consumer)]),
        interfaces: BTreeMap::from([
            (supplier_id, supplier_interface),
            (consumer_id, consumer_interface),
        ]),
    })
}

fn immutable_type_phase_build(interface: &PackageInterfaceValidation) -> PackageInterfaceBuild {
    let types = interface
        .type_objects
        .iter()
        .map(|(digest, object)| (*digest, encode_type_object(object).unwrap().1))
        .collect();
    build_package_interface(&interface.owners, &types).unwrap()
}

fn load_interface_type_phase(
    package: PackageId,
    interface: &PackageInterfaceValidation,
    admission: &mut AssociatedInterfaceAdmission<'_>,
) -> Result<BTreeMap<TypeObjectDigest, TypeObject>, Diagnostic> {
    let built = immutable_type_phase_build(interface);
    validate_type_closure(
        package,
        &interface.owners,
        &built.objects,
        &mut StoreWork::default(),
        admission,
    )
    .map(|(objects, _)| objects)
}

fn nonborrow_nested_fixture() -> Fixture {
    let mut f = fixture();
    let product = intern(
        &mut f.consumer_interface.type_objects,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: Name::new("selected").unwrap(),
                ty: f.projection,
            }],
        },
    );
    function_mut(&mut f).result.relation = ResultRelation::Value;
    function_mut(&mut f).result.ty = product;
    let PackageInterfaceRecord::Parameter(parameter) = &mut f
        .consumer_interface
        .owners
        .get_mut(&OwnerKey::Parameter(f.parameter))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    parameter.ty = product;
    parameter.use_mode = ParameterUse::Consume;
    f
}

#[test]
fn associated_interface_complete_type_phase_checks_nonborrow_nested_and_each_owner_occurrence() {
    let f = nonborrow_nested_fixture();
    load_public_interface(f.consumer, &f.consumer_interface).unwrap();
    let mut visit = |_| Ok(());
    let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
    let types =
        load_interface_type_phase(f.consumer, &f.consumer_interface, &mut admission).unwrap();
    assert!(types.contains_key(&f.projection));
    let self_type = function_signature(&f).implementation_parameters[0].self_type;
    assert!(types.contains_key(&self_type));
    assert!(admission.work > 0);
    validate_owner_closure_metered(
        f.consumer,
        &f.consumer_interface.owners,
        &types,
        &mut admission,
    )
    .unwrap();
    for mutation in 0..4 {
        let mut f = nonborrow_nested_fixture();
        match mutation {
            0 => {
                let id = ParameterId::migrate(SEED, 99);
                f.consumer_interface.owners.insert(
                    OwnerKey::Parameter(id),
                    owner(PackageInterfaceRecord::Parameter(ParameterRecord {
                        header: current_header(OwnerKey::Parameter(id), OwnerKind::Parameter),
                        parent: ParameterParent::Function(f.function.declaration),
                        name: Name::new("orphan").unwrap(),
                        ty: f.projection,
                        use_mode: ParameterUse::Consume,
                        resource_requirement: None,
                    })),
                );
            }
            1 | 2 => {
                let PackageInterfaceRecord::Parameter(parameter) = &mut f
                    .consumer_interface
                    .owners
                    .get_mut(&OwnerKey::Parameter(f.parameter))
                    .unwrap()
                    .record
                else {
                    unreachable!()
                };
                parameter.header.contract_version = if mutation == 1 { 0 } else { u16::MAX };
            }
            _ => {
                let malformed = intern(
                    &mut f.consumer_interface.type_objects,
                    TypeForm::AssociatedWitnessProjection {
                        scope: DeclarationReference {
                            package: f.consumer,
                            declaration: DeclarationId::migrate(SEED, 99),
                        },
                        parameter: f.witness,
                        contract: f.contract,
                        member: f.out,
                    },
                );
                function_mut(&mut f).result.ty = malformed;
            }
        }
        let error = load_public_interface(f.consumer, &f.consumer_interface).unwrap_err();
        if matches!(mutation, 1 | 2) {
            assert_eq!(error.class, DiagnosticClass::Corrupt);
            assert_eq!(error.code, "package_interface_closure");
            assert!(error.message.contains("foreign graph contract"));
        } else {
            assert_eq!(
                error.class,
                DiagnosticClass::Semantic,
                "mutation {mutation}: {error:?}"
            );
            assert_eq!(error.code, "package_interface_associated_authority");
        }
    }
    // One digest is legitimate in the contract method and forbidden in a private field. Neither
    // absence of a Borrow result nor prior success under another owner suppresses this occurrence.
    let mut f = fixture();
    let (_, field) = public_private_output(&mut f);
    let member = f.contract_member;
    let PackageInterfaceRecord::Field(record) = &mut f
        .supplier_interface
        .owners
        .get_mut(&OwnerKey::Field(field))
        .unwrap()
        .record
    else {
        unreachable!()
    };
    record.ty = member;
    let error = load_public_interface(f.supplier, &f.supplier_interface).unwrap_err();
    assert_eq!(error.code, "package_interface_associated_authority");
}

#[test]
fn associated_interface_loaded_foreign_nonborrow_closure_discharges_unused_binding_obligations() {
    for mutation in 0..3 {
        let mut f = nonborrow_nested_fixture();
        match mutation {
            1 => {
                bindings_mut(&mut f).pop();
            }
            2 => {
                let scalar = f.scalar;
                let spare = f.spare;
                bindings_mut(&mut f)
                    .iter_mut()
                    .find(|binding| binding.member == spare)
                    .unwrap()
                    .value = AssociatedTypeBindingValue::Owned { ty: scalar };
            }
            _ => {}
        }
        // Even an unused binding must satisfy the complete selected contract. The scalar
        // RHS now fails the independent descriptor loader as well as selected discharge.
        let selected = if mutation == 2 {
            let error = load_public_interface(f.supplier, &f.supplier_interface).unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Semantic);
            assert_eq!(error.code, "package_interface_read_view");
            assert!(error.message.contains("Owned binding RHS"));
            let supplier = f.supplier;
            public_fixture_closure_with(f, |package, interface| {
                if package != supplier {
                    return load_public_interface(package, interface);
                }
                // Neutral hostile fixture: preserve canonical bytes and recompute the exact
                // interface/revision bindings while bypassing only descriptor admission, so
                // the separate selected-closure validator must also refuse the invalid RHS.
                let built = immutable_type_phase_build(interface);
                let mut raw = interface.clone();
                raw.type_objects = load_interface_type_phase(
                    package,
                    interface,
                    &mut AssociatedInterfaceAdmission::new(&mut |_| Ok(())),
                )?;
                Ok((
                    package_interface_digest(package, built.root.content_root())?,
                    raw,
                ))
            })
            .unwrap()
        } else {
            public_fixture_closure(f).unwrap()
        };
        let result = validate_package_interface_closure(&selected.revisions, &selected.interfaces);
        if mutation == 0 {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Semantic);
        }
    }
}

#[test]
fn associated_interface_type_and_borrow_phases_share_capacity_and_real_cancellation() {
    use crate::platform::execution::ExecutionControl;
    use crate::platform::execution::normalized::execution_diagnostic;
    let f = fixture();
    let callback_work = std::cell::Cell::new(0_u64);
    let mut visit = |count| {
        callback_work.set(callback_work.get().checked_add(count).unwrap());
        Ok(())
    };
    let mut measured = AssociatedInterfaceAdmission::new(&mut visit);
    let types =
        load_interface_type_phase(f.consumer, &f.consumer_interface, &mut measured).unwrap();
    let type_work = measured.work;
    let type_callback_work = callback_work.get();
    validate_owner_closure_metered(
        f.consumer,
        &f.consumer_interface.owners,
        &types,
        &mut measured,
    )
    .unwrap();
    let complete = measured.work;
    let complete_callback_work = callback_work.get();
    assert!(complete > type_work && type_work > 0);
    // The ordinary type traversal also debits the actual shared callback; its work is not part
    // of the associated-only counter. Both phases retain one cumulative callback allowance.
    assert!(type_callback_work > u64::try_from(type_work).unwrap());
    assert!(complete_callback_work > type_callback_work);
    // A measured exact fit/one-short is capacity evidence, not a finite-prefix semantic proof.
    for maximum in [complete - 1, complete] {
        let control = ExecutionControl::uncancelled();
        let mut visit = |_| control.check().map_err(execution_diagnostic);
        let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
        admission.maximum_work = maximum;
        let types =
            load_interface_type_phase(f.consumer, &f.consumer_interface, &mut admission).unwrap();
        let result = validate_owner_closure_metered(
            f.consumer,
            &f.consumer_interface.owners,
            &types,
            &mut admission,
        );
        if maximum == complete {
            result.unwrap();
        } else {
            assert_eq!(result.unwrap_err().class, DiagnosticClass::Resource);
        }
    }
    // Separately bound the actual callback, including ordinary traversal plus demanded
    // associated proof work. No internal associated limit is shortened in this control.
    for maximum in [complete_callback_work - 1, complete_callback_work] {
        let control = ExecutionControl::uncancelled();
        let remaining = std::cell::Cell::new(maximum);
        let mut visit = |count| {
            control.check().map_err(execution_diagnostic)?;
            let next = remaining.get().checked_sub(count).ok_or_else(|| {
                interface_error(
                    DiagnosticClass::Resource,
                    "associated_interface_test_callback_work",
                    "the shared interface validation callback exhausted its visit allowance",
                )
            })?;
            remaining.set(next);
            Ok(())
        };
        let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
        let types =
            load_interface_type_phase(f.consumer, &f.consumer_interface, &mut admission).unwrap();
        assert_eq!(maximum - remaining.get(), type_callback_work);
        let result = validate_owner_closure_metered(
            f.consumer,
            &f.consumer_interface.owners,
            &types,
            &mut admission,
        );
        if maximum == complete_callback_work {
            result.unwrap();
            assert_eq!(remaining.get(), 0);
            assert_eq!(admission.work, complete);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, "associated_interface_test_callback_work");
            assert!(admission.work <= complete);
        }
    }
    // The complete public entry also retains owner reservations and per-object charges in
    // that same callback. Exercise its actual wiring, beyond manually composing private phases.
    let built = immutable_type_phase_build(&f.consumer_interface);
    let public_callback_work = std::cell::Cell::new(0_u64);
    validate_package_interface_metered(
        f.consumer,
        built.root,
        &built.objects,
        &mut StoreWork::default(),
        &mut MapAdmission::unbounded(),
        &mut |count| {
            public_callback_work.set(public_callback_work.get().checked_add(count).unwrap());
            Ok(())
        },
    )
    .unwrap();
    let public_complete = public_callback_work.get();
    assert!(public_complete > complete_callback_work);
    for maximum in [public_complete - 1, public_complete] {
        let control = ExecutionControl::uncancelled();
        let remaining = std::cell::Cell::new(maximum);
        let mut visit = |count| {
            control.check().map_err(execution_diagnostic)?;
            let next = remaining.get().checked_sub(count).ok_or_else(|| {
                interface_error(
                    DiagnosticClass::Resource,
                    "associated_interface_test_callback_work",
                    "the shared interface validation callback exhausted its visit allowance",
                )
            })?;
            remaining.set(next);
            Ok(())
        };
        let result = validate_package_interface_metered(
            f.consumer,
            built.root,
            &built.objects,
            &mut StoreWork::default(),
            &mut MapAdmission::unbounded(),
            &mut visit,
        );
        if maximum == public_complete {
            result.unwrap();
            assert_eq!(remaining.get(), 0);
        } else {
            let error = result.unwrap_err();
            assert_eq!(error.class, DiagnosticClass::Resource);
            assert_eq!(error.code, "associated_interface_test_callback_work");
        }
    }
    let control = ExecutionControl::uncancelled();
    let mut visit = |_| control.check().map_err(execution_diagnostic);
    let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
    let types =
        load_interface_type_phase(f.consumer, &f.consumer_interface, &mut admission).unwrap();
    let before = admission.work;
    control.cancel();
    assert_eq!(
        validate_owner_closure_metered(
            f.consumer,
            &f.consumer_interface.owners,
            &types,
            &mut admission
        )
        .unwrap_err()
        .class,
        DiagnosticClass::Cancelled
    );
    assert_eq!(admission.work, before);
    let fresh = ExecutionControl::uncancelled();
    let mut visit = |_| fresh.check().map_err(execution_diagnostic);
    let mut admission = AssociatedInterfaceAdmission::new(&mut visit);
    let types =
        load_interface_type_phase(f.consumer, &f.consumer_interface, &mut admission).unwrap();
    validate_owner_closure_metered(
        f.consumer,
        &f.consumer_interface.owners,
        &types,
        &mut admission,
    )
    .unwrap();
}
