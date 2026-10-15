//! Flat current codecs and independent associated-inventory semantic witnesses.
use super::*;
use crate::platform::compiler::unit::CompilationTables;
use crate::platform::compiler::{
    BYTECODE_CONTRACT_VERSION, COMPILER_UNIT_CONTRACT_VERSION, CompilationPayload,
    CompilationSource, CompilationUnit, CompilationUnitKey, OptimizationPolicy,
};
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::package_interface::{
    PACKAGE_INTERFACE_CONTRACT_VERSION, PACKAGE_INTERFACE_MAGIC, PackageInterfaceOwner,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ExpressionId, ImplementationParameterId, MethodId,
    ModuleId, ParameterId, ReadScopeParameterId, TypeParameterId,
};
use crate::platform::witness::SemanticDigest;

fn base_owner(implementation: bool) -> OwnerRecord {
    let seed = b"flat-associated-codec";
    let declaration = DeclarationId::migrate(seed, u64::from(implementation));
    let reference = DeclarationReference {
        package: PackageId::migrate(seed, 0),
        declaration,
    };
    let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
        .unwrap()
        .0;
    let method = MethodId::migrate(seed, 0);
    let payload = if implementation {
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![],
            implementation_parameters: vec![],
            contract: reference,
            self_type: ty,
            type_arguments: vec![],
            methods: vec![OwnedMethodImplementation {
                method,
                function: reference,
                type_arguments: vec![],
                implementations: vec![],
                read_scope_arguments: vec![],
            }],
            associated_types: vec![],
        })
    } else {
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: TypeParameterId::migrate(seed, 0),
            type_parameters: vec![],
            methods: vec![OwnedMethod {
                id: method,
                name: Name::new("consume").unwrap(),
                parameters: vec![OwnedMethodParameter {
                    ty,
                    use_mode: ParameterUse::Consume,
                }],
                result: CallableResult {
                    ty,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
            }],
            associated_types: vec![],
        })
    };
    OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(
            OwnerKey::Declaration(declaration),
            if implementation {
                OwnerKind::OwnedImplementation
            } else {
                OwnerKind::OwnedContract
            },
        ),
        module: ModuleId::migrate(seed, 0),
        name: Name::new(if implementation {
            "FlatImplementation"
        } else {
            "FlatContract"
        })
        .unwrap(),
        visibility: DeclarationVisibility::Public,
        payload,
    })
}

fn base_unit(owner: &OwnerRecord) -> CompilationUnit {
    let OwnerRecord::Declaration(record) = owner else {
        unreachable!()
    };
    let package = PackageId::migrate(b"flat-associated-codec", 0);
    let digest = SemanticDigest::from_bytes([7; 32]);
    let source = CompilationSource {
        package,
        owner: owner.owner(),
        kind: owner.kind(),
        semantic_interface: digest,
        implementation: digest,
        type_digest: digest,
        effect: digest,
        capability: digest,
        test: None,
        validation_dependencies: digest,
    };
    let optimization = OptimizationPolicy::DeterministicBaseline;
    let (payload, declarations) = match &record.payload {
        DeclarationPayload::OwnedContract(c) => {
            (CompilationPayload::OwnedContract(c.clone()), vec![])
        }
        DeclarationPayload::OwnedImplementation(i) => {
            let declarations = std::iter::once(i.contract)
                .chain(i.methods.iter().map(|m| m.function))
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            (
                CompilationPayload::OwnedImplementation(i.clone()),
                declarations,
            )
        }
        _ => unreachable!(),
    };
    CompilationUnit {
        contract_version: COMPILER_UNIT_CONTRACT_VERSION,
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        bytecode_contract_version: BYTECODE_CONTRACT_VERSION,
        key: CompilationUnitKey::derive(&source, optimization).unwrap(),
        source,
        optimization,
        tables: CompilationTables {
            declarations,
            fields: vec![],
            cases: vec![],
            requirements: vec![],
            operations: vec![],
            ports: vec![],
            types: owner
                .type_roots()
                .into_iter()
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect(),
            structural_names: vec![],
            read_view_fields: vec![],
            texts: vec![],
        },
        payload,
    }
}

fn member() -> AssociatedTypeMember {
    AssociatedTypeMember {
        id: AssociatedTypeMemberId::from_bytes([71; 16]).unwrap(),
        name: Name::new("Block").unwrap(),
        bound: AssociatedTypeBound::Owned,
    }
}

fn extend(owner: &mut OwnerRecord) {
    let OwnerRecord::Declaration(record) = owner else {
        unreachable!()
    };
    match &mut record.payload {
        DeclarationPayload::OwnedContract(c) => c.associated_types.push(member()),
        DeclarationPayload::OwnedImplementation(i) => {
            i.associated_types.push(AssociatedTypeBinding {
                member: member().id,
                value: AssociatedTypeBindingValue::Owned { ty: i.self_type },
            })
        }
        _ => unreachable!(),
    }
}

#[test]
fn flat_parent_codecs_preserve_empty_and_nonempty_associated_inventories() {
    for implementation in [false, true] {
        let mut previous = None;
        for populated in [false, true] {
            let mut owner = base_owner(implementation);
            if populated {
                extend(&mut owner);
            }
            let direct = bincode::encode_to_vec(&owner, bincode::config::standard()).unwrap();
            let (decoded, consumed): (OwnerRecord, _) =
                bincode::decode_from_slice(&direct, bincode::config::standard()).unwrap();
            assert_eq!(consumed, direct.len());
            assert_eq!(decoded, owner);
            let (digest, bytes) = encode_owner(&owner).unwrap();
            assert_eq!(&bytes[..8], &contract::OWNER_MAGIC);
            assert_eq!(
                decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
                owner
            );
            let interface = PackageInterfaceOwner {
                contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
                record: PackageInterfaceRecord::project_public(&owner)
                    .unwrap()
                    .unwrap(),
            };
            let (interface_digest, interface_bytes) = interface.encode().unwrap();
            assert_eq!(&interface_bytes[..8], &PACKAGE_INTERFACE_MAGIC);
            assert_eq!(
                PackageInterfaceOwner::decode(&interface_bytes, owner.owner(), interface_digest)
                    .unwrap(),
                interface
            );
            let unit = base_unit(&owner);
            let (unit_key, unit_bytes) = unit.encode().unwrap();
            assert_eq!(
                &unit_bytes[..8],
                &crate::platform::compiler::COMPILER_UNIT_MAGIC
            );
            assert_eq!(
                CompilationUnit::decode(&unit_bytes, unit_key).unwrap(),
                unit
            );
            if let Some((old_owner, old_interface, old_unit)) = previous {
                assert_ne!(digest, old_owner);
                assert_ne!(interface_digest, old_interface);
                assert_ne!(unit_bytes, old_unit);
            }
            previous = Some((digest, interface_digest, unit_bytes));
        }
    }
}

#[test]
fn associated_types_bind_exact_contract_member_and_lexical_witness_identity() {
    let contract = DeclarationReference {
        package: PackageId::from_bytes([1; 16]).unwrap(),
        declaration: DeclarationId::from_bytes([2; 16]).unwrap(),
    };
    let scope = DeclarationReference {
        package: contract.package,
        declaration: DeclarationId::from_bytes([3; 16]).unwrap(),
    };
    let parameter = ImplementationParameterId::from_bytes([4; 16]).unwrap();
    let forms = [
        TypeForm::AssociatedContractMember {
            contract,
            member: member().id,
        },
        TypeForm::AssociatedWitnessProjection {
            scope,
            parameter,
            contract,
            member: member().id,
        },
    ];
    for form in forms {
        let object = TypeObject::new(form).unwrap();
        let (digest, bytes) = encode_type_object(&object).unwrap();
        assert_eq!(&bytes[..8], &contract::TYPE_OBJECT_MAGIC);
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), object);
        let canonical = crate::platform::packed::encode_canonical(
            contract::TYPE_OBJECT_MAGIC,
            contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
            &object,
            contract::MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert_eq!(canonical, bytes);
    }
    let projection = |scope, parameter, contract, member| {
        encode_type_object(
            &TypeObject::new(TypeForm::AssociatedWitnessProjection {
                scope,
                parameter,
                contract,
                member,
            })
            .unwrap(),
        )
        .unwrap()
        .0
    };
    let original = projection(scope, parameter, contract, member().id);
    assert_ne!(
        original,
        projection(
            scope,
            ImplementationParameterId::from_bytes([5; 16]).unwrap(),
            contract,
            member().id
        )
    );
    assert_ne!(
        original,
        projection(contract, parameter, contract, member().id)
    );
    assert_ne!(original, projection(scope, parameter, scope, member().id));
    assert_ne!(
        original,
        projection(
            scope,
            parameter,
            contract,
            AssociatedTypeMemberId::from_bytes([6; 16]).unwrap()
        )
    );
}

#[test]
fn flat_codec_does_not_replace_associated_inventory_semantic_admission() {
    for implementation in [false, true] {
        let mut owner = base_owner(implementation);
        extend(&mut owner);
        extend(&mut owner);
        assert!(encode_owner(&owner).is_err());
        let interface = PackageInterfaceOwner {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: PackageInterfaceRecord::project_public(&owner)
                .unwrap()
                .unwrap(),
        };
        assert!(interface.encode().is_err());
        assert!(base_unit(&owner).encode().is_err());
    }
}

#[test]
fn read_view_family_inventory_codecs_preserve_scope_identity() {
    for implementation in [false, true] {
        let mut owner = base_owner(implementation);
        let scope_parameter = ReadScopeParameter {
            id: ReadScopeParameterId::from_bytes([72; 16]).unwrap(),
            name: Name::new("window").unwrap(),
        };
        let OwnerRecord::Declaration(record) = &mut owner else {
            unreachable!()
        };
        match &mut record.payload {
            DeclarationPayload::OwnedContract(contract) => {
                let mut family = member();
                family.bound = AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: scope_parameter.clone(),
                };
                contract.associated_types.push(family);
            }
            DeclarationPayload::OwnedImplementation(implementation) => {
                implementation.associated_types.push(AssociatedTypeBinding {
                    member: member().id,
                    value: AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter: scope_parameter.clone(),
                        ty: implementation.self_type,
                    },
                });
            }
            _ => unreachable!(),
        }
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], &contract::OWNER_MAGIC);
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        let interface = PackageInterfaceOwner {
            contract_version: PACKAGE_INTERFACE_CONTRACT_VERSION,
            record: PackageInterfaceRecord::project_public(&owner)
                .unwrap()
                .unwrap(),
        };
        let (digest, bytes) = interface.encode().unwrap();
        assert_eq!(
            PackageInterfaceOwner::decode(&bytes, owner.owner(), digest).unwrap(),
            interface
        );
        let unit = base_unit(&owner);
        let (key, bytes) = unit.encode().unwrap();
        assert_eq!(CompilationUnit::decode(&bytes, key).unwrap(), unit);
    }
}

#[test]
fn binding_roots_and_compiler_tables_include_every_associated_rhs() {
    let mut owner = base_owner(true);
    extend(&mut owner);
    let OwnerRecord::Declaration(record) = &mut owner else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(implementation) = &mut record.payload else {
        unreachable!()
    };
    let rhs = TypeObjectDigest::from_bytes([217; 32]);
    *implementation.associated_types[0].value.ty_mut() = rhs;
    assert!(owner.type_roots().contains(&rhs));
    assert!(
        PackageInterfaceRecord::project_public(&owner)
            .unwrap()
            .unwrap()
            .type_roots()
            .contains(&rhs)
    );
    let mut unit = base_unit(&base_owner(true));
    let CompilationPayload::OwnedImplementation(implementation) = &mut unit.payload else {
        unreachable!()
    };
    implementation.associated_types.push(AssociatedTypeBinding {
        member: member().id,
        value: AssociatedTypeBindingValue::Owned { ty: rhs },
    });
    assert_eq!(
        unit.encode().unwrap_err().code,
        "compiler_unit_owned_implementation_type"
    );
    unit.tables.types.push(rhs);
    assert!(unit.encode().is_ok());
}

/// Complete logical input for independent FullValidator occurrence controls.
fn codec_occurrence_snapshot() -> (
    KernelSnapshot,
    DeclarationReference,
    DeclarationReference,
    OwnerKey,
) {
    let mut snapshot = super::tests::witness_snapshot();
    let seed = b"codec-associated-real-occurrences";
    let module = snapshot
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .expect("existing module");
    let contract_id = DeclarationId::migrate(seed, 0);
    let scope_id = DeclarationId::migrate(seed, 1);
    let contract = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: contract_id,
    };
    let scope = DeclarationReference {
        package: snapshot.root.package_id,
        declaration: scope_id,
    };
    let contract_self = TypeParameterId::migrate(seed, 0);
    let caller_self = TypeParameterId::migrate(seed, 1);
    for (parameter, declaration) in [(contract_self, contract_id), (caller_self, scope_id)] {
        snapshot.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration,
                name: Name::new("Self").expect("name"),
                constraints: TypeParameterConstraints::Owned,
            }),
        );
    }
    let mut intern = |form| {
        let object = TypeObject::new(form).expect("canonical fixture type");
        let digest = encode_type_object(&object)
            .expect("canonical fixture bytes")
            .0;
        snapshot.types.insert(digest, object);
        digest
    };
    let self_type = intern(TypeForm::TypeParameter {
        parameter: caller_self,
    });
    let scalar = intern(TypeForm::I64);
    let witness = ImplementationParameterId::migrate(seed, 0);
    let projection = intern(TypeForm::AssociatedWitnessProjection {
        scope,
        parameter: witness,
        contract,
        member: member().id,
    });
    snapshot.owners.insert(
        OwnerKey::Declaration(contract_id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(contract_id), OwnerKind::OwnedContract),
            module,
            name: Name::new("CodecOccurrenceContract").expect("name"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: contract_self,
                type_parameters: vec![],
                associated_types: vec![member()],
                methods: vec![OwnedMethod {
                    id: crate::platform::semantic_id::MethodId::migrate(seed, 0),
                    name: Name::new("inspect").expect("name"),
                    parameters: vec![OwnedMethodParameter {
                        ty: scalar,
                        use_mode: ParameterUse::Unrestricted,
                    }],
                    result: CallableResult {
                        ty: scalar,
                        relation: ResultRelation::Value,
                    },
                    read_scope_parameters: vec![],
                    read_scope_sources: vec![],
                    effect: FunctionEffect::Pure,
                }],
            }),
        }),
    );
    let parameter_id = ParameterId::migrate(seed, 0);
    let parameter = OwnerKey::Parameter(parameter_id);
    snapshot.owners.insert(
        parameter,
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(parameter, OwnerKind::Parameter),
            parent: ParameterParent::Function(scope_id),
            name: Name::new("value").expect("name"),
            ty: projection,
            use_mode: ParameterUse::Consume,
            resource_requirement: None,
        }),
    );
    let body = ExpressionId::migrate(seed, 0);
    snapshot.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                body,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(parameter_id),
                },
            )
            .expect("canonical local"),
        ),
    );
    snapshot.owners.insert(
        OwnerKey::Declaration(scope_id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(scope_id), OwnerKind::PureFunction),
            module,
            name: Name::new("codecOccurrenceForward").expect("name"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![caller_self],
                implementation_parameters: vec![ImplementationParameter {
                    id: witness,
                    name: Name::new("output").expect("name"),
                    contract,
                    self_type,
                    type_arguments: vec![],
                }],
                requirement_parameters: vec![],
                effect_parameters: vec![],
                parameters: vec![parameter_id],
                result: CallableResult {
                    ty: projection,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
                body,
            }),
        }),
    );
    // FullValidator consumes complete logical records; persistent-map integrity
    // remains the repository's job. Keep the actual logical inventory count exact.
    let root = snapshot.root.owners;
    snapshot.root.owners = crate::platform::persistent_map::MapRoot::from_parts(
        root.page(),
        u64::try_from(snapshot.owners.len()).expect("bounded fixture inventory"),
        root.content(),
    );
    (snapshot, scope, contract, parameter)
}

#[test]
fn codec_associated_schema_requires_exact_occurrence_scope_and_current_owner_header() {
    let (base, scope, contract, parameter) = codec_occurrence_snapshot();
    validate_full(&base).expect("healthy exact current occurrence is complete");
    for mutation in 0..5 {
        let mut snapshot = base.clone();
        if mutation == 0 {
            let OwnerRecord::Parameter(record) =
                snapshot.owners.get_mut(&parameter).expect("parameter")
            else {
                panic!("parameter");
            };
            record.header.contract_version = u16::MAX;
        } else if mutation == 1 {
            let OwnerRecord::Declaration(record) = snapshot
                .owners
                .get_mut(&OwnerKey::Declaration(scope.declaration))
                .expect("function")
            else {
                panic!("function");
            };
            record.header.contract_version = u16::MAX;
        } else {
            let original_result = match snapshot
                .owners
                .get(&OwnerKey::Declaration(scope.declaration))
                .expect("function")
            {
                OwnerRecord::Declaration(function) => match &function.payload {
                    DeclarationPayload::Function(signature) => signature.result.ty,
                    _ => panic!("signature"),
                },
                _ => panic!("function"),
            };
            let mut form = snapshot.types[&original_result].form.clone();
            if mutation == 2 {
                let TypeForm::AssociatedWitnessProjection { scope, .. } = &mut form else {
                    panic!("projection");
                };
                *scope = contract;
            } else if mutation == 3 {
                form = TypeForm::AssociatedContractMember {
                    contract,
                    member: member().id,
                };
            } else {
                let foreign = DeclarationReference {
                    package: PackageId::from_bytes([92; 16]).expect("foreign package"),
                    declaration: contract.declaration,
                };
                let TypeForm::AssociatedWitnessProjection {
                    contract: projected_contract,
                    ..
                } = &mut form
                else {
                    panic!("projection");
                };
                *projected_contract = foreign;
                let OwnerRecord::Declaration(function) = snapshot
                    .owners
                    .get_mut(&OwnerKey::Declaration(scope.declaration))
                    .expect("function")
                else {
                    panic!("function");
                };
                let DeclarationPayload::Function(signature) = &mut function.payload else {
                    panic!("signature");
                };
                signature.implementation_parameters[0].contract = foreign;
                // A cached exact-looking interface supplies no authored edge.
                // The form is genuinely consumed by the local parameter/result.
                let imported = PackageInterfaceRecord::project_public(
                    snapshot
                        .owners
                        .get(&OwnerKey::Declaration(contract.declaration))
                        .expect("contract"),
                )
                .expect("public projection")
                .expect("public contract");
                snapshot.dependency_interfaces.insert(
                    PackageRevisionDigest::from_bytes([93; 32]),
                    std::sync::Arc::new(std::collections::BTreeMap::from([(
                        OwnerKey::Declaration(contract.declaration),
                        imported,
                    )])),
                );
                assert!(!snapshot.dependencies.contains_key(&foreign.package));
            }
            let object = TypeObject::new(form).expect("canonical mutant type");
            let digest = encode_type_object(&object)
                .expect("canonical mutant bytes")
                .0;
            snapshot.types.insert(digest, object);
            let OwnerRecord::Parameter(record) =
                snapshot.owners.get_mut(&parameter).expect("parameter")
            else {
                panic!("parameter");
            };
            record.ty = digest;
            let OwnerRecord::Declaration(function) = snapshot
                .owners
                .get_mut(&OwnerKey::Declaration(scope.declaration))
                .expect("function")
            else {
                panic!("function");
            };
            let DeclarationPayload::Function(signature) = &mut function.payload else {
                panic!("signature");
            };
            signature.result.ty = digest;
            assert_ne!(
                digest, original_result,
                "each form mutant changes its digest"
            );
            assert!(
                snapshot
                    .owners
                    .values()
                    .all(|owner| { !owner.type_roots().contains(&original_result) }),
                "the replaced projection has no remaining owner roots",
            );
            assert!(
                snapshot
                    .types
                    .values()
                    .all(|object| { !object.child_types().contains(&original_result) }),
                "the replaced projection has no remaining child roots",
            );
            snapshot
                .types
                .remove(&original_result)
                .expect("remove only the obsolete original projection");
        }
        let errors =
            validate_full(&snapshot).expect_err("encoding cannot grant occurrence authority");
        let code = if mutation < 2 {
            "kernel_owner_header"
        } else if mutation == 4 {
            // Exact authored closure authority is checked before member lookup.
            "kernel_owned_contract"
        } else {
            "kernel_owned_associated"
        };
        let expected_message =
            (mutation == 4).then_some("owned reference is outside the declared package closure");
        assert!(
            errors.iter().any(|error| {
                error.class == DiagnosticClass::Semantic
                    && error.code == code
                    && expected_message.is_none_or(|message| error.message == message)
            }),
            "{mutation}: {errors:#?}",
        );
        assert!(
            errors
                .iter()
                .all(|error| error.code != "owned_associated_semantics_unsupported")
        );
        assert!(errors.iter().all(|error| !matches!(
            error.code.as_str(),
            "kernel_full_map_count"
                | "kernel_full_owner_key"
                | "kernel_full_owner_parent"
                | "kernel_full_type_unreachable"
        )));
    }
}
