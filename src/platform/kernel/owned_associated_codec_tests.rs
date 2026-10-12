//! Literal predecessor objects and independent frozen-layout witnesses for the additive codecs.
use super::*;
use crate::platform::compiler::{CompilationPayload, CompilationUnit, CompilationUnitKey};
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::package_interface::{PackageInterfaceOwner, PackageInterfaceOwnerDigest};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ExpressionId, ImplementationParameterId, ParameterId,
    TypeParameterId,
};
use crate::platform::storage::object::{ObjectDomain, ObjectKey};

fn hex(value: &str) -> Vec<u8> {
    let (pairs, remainder) = value.as_bytes().as_chunks::<2>();
    assert!(
        remainder.is_empty(),
        "literal hex must contain complete bytes"
    );
    pairs
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn literal(name: &str) -> &'static [u8] {
    match name {
        "owner-contract-base.bin" => {
            include_bytes!("../../../tests/fixtures/associated-codec-base/owner-contract-base.bin")
        }
        "owner-implementation-base.bin" => include_bytes!(
            "../../../tests/fixtures/associated-codec-base/owner-implementation-base.bin"
        ),
        "interface-contract-base.bin" => include_bytes!(
            "../../../tests/fixtures/associated-codec-base/interface-contract-base.bin"
        ),
        "interface-implementation-base.bin" => include_bytes!(
            "../../../tests/fixtures/associated-codec-base/interface-implementation-base.bin"
        ),
        "unit-contract-base.bin" => {
            include_bytes!("../../../tests/fixtures/associated-codec-base/unit-contract-base.bin")
        }
        "unit-implementation-base.bin" => include_bytes!(
            "../../../tests/fixtures/associated-codec-base/unit-implementation-base.bin"
        ),
        _ => panic!("unknown literal predecessor fixture"),
    }
}

fn base_owner(implementation: bool) -> OwnerRecord {
    let (name, id, kind) = if implementation {
        (
            "owner-implementation-base.bin",
            "80b1ddeb01a0e7dea83aead81adef232",
            OwnerKind::OwnedImplementation,
        )
    } else {
        (
            "owner-contract-base.bin",
            "e4fb2deab61c6b0fe468a060497aebbc",
            OwnerKind::OwnedContract,
        )
    };
    let bytes = literal(name);
    decode_owner(
        bytes,
        OwnerKey::Declaration(DeclarationId::parse(&format!("decl_{id}")).unwrap()),
        kind,
        OwnerObjectDigest::of(bytes),
    )
    .unwrap()
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
        DeclarationPayload::OwnedContract(contract) => contract.associated_types.push(member()),
        DeclarationPayload::OwnedImplementation(implementation) => {
            implementation.associated_types.push(AssociatedTypeBinding {
                member: member().id,
                ty: implementation.self_type,
            })
        }
        _ => unreachable!(),
    }
}

#[test]
fn literal_predecessor_parent_objects_preserve_exact_bytes_and_keys() {
    let provenance: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/associated-codec-base/provenance.json"
    ))
    .unwrap();
    assert_eq!(
        provenance["source_sha256"],
        "0115358c9dc9c9a91d0bac7755b619a12f36ca9b5e90b69d649639c8ccdb3a54"
    );
    let fixtures = provenance["fixtures"].as_array().unwrap();
    assert_eq!(fixtures.len(), 6);
    for fixture in fixtures {
        let bytes = literal(fixture["name"].as_str().unwrap());
        let owner = OwnerKey::Declaration(
            DeclarationId::parse(&format!("decl_{}", fixture["id"].as_str().unwrap())).unwrap(),
        );
        let expected: [u8; 32] = hex(fixture["digest"].as_str().unwrap()).try_into().unwrap();
        match fixture["category"].as_str().unwrap() {
            "owner" => {
                let kind = if fixture["kind"] == "contract" {
                    OwnerKind::OwnedContract
                } else {
                    OwnerKind::OwnedImplementation
                };
                let record =
                    decode_owner(bytes, owner, kind, OwnerObjectDigest::from_bytes(expected))
                        .unwrap();
                let OwnerRecord::Declaration(declaration) = &record else {
                    unreachable!()
                };
                assert!(!declaration.payload.has_associated_types());
                assert_eq!(
                    encode_owner(&record).unwrap(),
                    (OwnerObjectDigest::from_bytes(expected), bytes.to_vec())
                );
            }
            "interface" => {
                let record = PackageInterfaceOwner::decode(
                    bytes,
                    owner,
                    PackageInterfaceOwnerDigest::from_bytes(expected),
                )
                .unwrap();
                assert_eq!(
                    record.encode().unwrap(),
                    (
                        PackageInterfaceOwnerDigest::from_bytes(expected),
                        bytes.to_vec()
                    )
                );
            }
            "unit" => {
                let key = ObjectKey::from_digest(ObjectDomain::CompilerUnit, expected);
                let record = CompilationUnit::decode(bytes, key).unwrap();
                assert_eq!(record.source.owner, owner);
                assert_eq!(record.encode().unwrap(), (key, bytes.to_vec()));
            }
            _ => unreachable!(),
        }
    }
}

#[test]
fn nonempty_parent_inventories_have_disjoint_complete_envelopes() {
    for implementation in [false, true] {
        let mut owner = base_owner(implementation);
        extend(&mut owner);
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], b"LKJOAT01");
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        assert!(
            bincode::encode_to_vec(&owner, bincode::config::standard()).is_err(),
            "base owner encoding cannot lose an associated inventory"
        );
        let interface = PackageInterfaceOwner {
            contract_version: 21,
            record: PackageInterfaceRecord::project_public(&owner)
                .unwrap()
                .unwrap(),
        };
        let (interface_digest, interface_bytes) = interface.encode().unwrap();
        assert_eq!(&interface_bytes[..8], b"LKJPAT01");
        assert_eq!(
            PackageInterfaceOwner::decode(&interface_bytes, owner.owner(), interface_digest)
                .unwrap(),
            interface
        );
        assert!(
            bincode::encode_to_vec(&interface, bincode::config::standard()).is_err(),
            "base interface encoding cannot lose a nested inventory"
        );
        let unit_bytes = literal(if implementation {
            "unit-implementation-base.bin"
        } else {
            "unit-contract-base.bin"
        });
        let mut unit = CompilationUnit::decode(
            unit_bytes,
            ObjectKey::for_bytes(ObjectDomain::CompilerUnit, unit_bytes),
        )
        .unwrap();
        let tag = match &mut unit.payload {
            CompilationPayload::OwnedContract(contract) => {
                contract.associated_types.push(member());
                1
            }
            CompilationPayload::OwnedImplementation(implementation) => {
                implementation.associated_types.push(AssociatedTypeBinding {
                    member: member().id,
                    ty: implementation.self_type,
                });
                2
            }
            _ => unreachable!(),
        };
        let old_key = unit.key;
        unit.key =
            CompilationUnitKey::derive_for_tag(&unit.source, unit.optimization, Some(tag)).unwrap();
        assert_ne!(unit.key, old_key);
        let (unit_key, extended_bytes) = unit.encode().unwrap();
        assert_eq!(&extended_bytes[..8], b"LKJCAU01");
        assert_eq!(
            CompilationUnit::decode(&extended_bytes, unit_key).unwrap(),
            unit
        );
        assert!(
            bincode::encode_to_vec(&unit, bincode::config::standard()).is_err(),
            "base unit encoding cannot lose a nested inventory"
        );
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
        assert_eq!(&bytes[..8], b"LKJTAT01");
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), object);
        let base_bytes = crate::platform::packed::encode(
            contract::TYPE_OBJECT_MAGIC,
            contract::TYPE_OBJECT_ENVELOPE_DOMAIN,
            &object,
            contract::MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert!(
            decode_type_object(&base_bytes, TypeObjectDigest::of(&base_bytes)).is_err(),
            "a base type envelope cannot alias the associated form"
        );
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
fn explicit_empty_extension_and_predecessor_inventory_loss_are_refused() {
    use super::owned_associated_wire::{AssociatedOwnedBody, AssociatedOwnerWire};
    for implementation in [false, true] {
        let mut owner = base_owner(implementation);
        extend(&mut owner);
        let OwnerRecord::Declaration(record) = owner.clone() else {
            unreachable!()
        };
        let mut body = AssociatedOwnedBody::from_payload(&record.payload)
            .unwrap()
            .unwrap();
        match &mut body {
            AssociatedOwnedBody::Contract(_, members) => members.clear(),
            AssociatedOwnedBody::Implementation(_, bindings) => bindings.clear(),
        }
        let bytes = bincode::encode_to_vec(body, bincode::config::standard()).unwrap();
        assert!(
            bincode::decode_from_slice::<AssociatedOwnedBody, _>(
                &bytes,
                bincode::config::standard()
            )
            .is_err()
        );
        assert!(super::wire25::OwnerRecord25::try_from(owner.clone()).is_err());
        assert!(super::wire26::OwnerRecord26::try_from(owner.clone()).is_err());
        assert!(super::wire27::OwnerRecord27::try_from(owner.clone()).is_err());
        assert!(super::wire28::OwnerRecord28::try_from(owner.clone()).is_err());
        let interface = PackageInterfaceRecord::project_public(&owner)
            .unwrap()
            .unwrap();
        assert!(super::wire25::PackageInterfaceRecord25::try_from(interface.clone()).is_err());
        assert!(super::wire26::PackageInterfaceRecord26::try_from(interface.clone()).is_err());
        assert!(super::wire27::PackageInterfaceRecord27::try_from(interface.clone()).is_err());
        assert!(super::wire28::PackageInterfaceRecord28::try_from(interface).is_err());
        let mut historical = record;
        historical.header.contract_version = 28;
        let historical_owner = OwnerRecord::Declaration(historical.clone());
        assert_eq!(
            encode_owner(&historical_owner).unwrap_err().code,
            "kernel_associated_encoding_generation"
        );
        // An extension checksum does not grant an old logical header the new capability.
        let raw = crate::platform::packed::encode(
            contract::OWNED_ASSOCIATED_OWNER_MAGIC,
            contract::OWNED_ASSOCIATED_OWNER_ENVELOPE_DOMAIN,
            &AssociatedOwnerWire(historical),
            contract::MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        assert!(
            decode_owner(
                &raw,
                owner.owner(),
                owner.kind(),
                OwnerObjectDigest::of(&raw)
            )
            .is_err()
        );
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
    implementation.associated_types[0].ty = rhs;
    assert!(owner.type_roots().contains(&rhs));
    assert!(
        PackageInterfaceRecord::project_public(&owner)
            .unwrap()
            .unwrap()
            .type_roots()
            .contains(&rhs)
    );
    let bytes = literal("unit-implementation-base.bin");
    let mut unit = CompilationUnit::decode(
        bytes,
        ObjectKey::for_bytes(ObjectDomain::CompilerUnit, bytes),
    )
    .unwrap();
    let CompilationPayload::OwnedImplementation(implementation) = &mut unit.payload else {
        unreachable!()
    };
    implementation.associated_types.push(AssociatedTypeBinding {
        member: member().id,
        ty: rhs,
    });
    unit.key =
        CompilationUnitKey::derive_for_tag(&unit.source, unit.optimization, Some(2)).unwrap();
    assert_eq!(
        unit.encode().unwrap_err().code,
        "compiler_unit_owned_implementation_type"
    );
    unit.tables.types.push(rhs);
    assert!(unit.encode().is_ok());
}

#[test]
fn independent_literal_historical_nested_payloads_preserve_empty_inventories() {
    // Literal Graph25 positional payloads, deliberately specified independently of the
    // successor wire structs: declaration owner/header, public name, and empty methods.
    let cases = [
        "011901031111111111111111111111111111111119022222222222222222222222222222222201430208113333333333333333333333333333333300",
        "01190103111111111111111111111111111111111a0222222222222222222222222222222222014902091244444444444444444444444444444444035555555555555555555555555555555502666666666666666666666666666666666666666666666666666666666666666600",
    ];
    for encoded in cases {
        let bytes = hex(encoded);
        let (old, consumed): (super::wire25::OwnerRecord25, _) =
            bincode::decode_from_slice(&bytes, bincode::config::standard()).unwrap();
        assert_eq!(consumed, bytes.len());
        let mut owner: OwnerRecord = old.into();
        assert_eq!(
            bincode::encode_to_vec(
                super::wire25::OwnerRecord25::try_from(owner.clone()).unwrap(),
                bincode::config::standard()
            )
            .unwrap(),
            bytes
        );
        let interface = PackageInterfaceRecord::project_public(&owner)
            .unwrap()
            .unwrap();
        let old = super::wire25::PackageInterfaceRecord25::try_from(interface.clone()).unwrap();
        let wire_bytes = bincode::encode_to_vec(old, bincode::config::standard()).unwrap();
        // Remove only the canonical owner discriminator and module/visibility fields;
        // the expected interface discriminator and contract/implementation payload tag
        // are literal frozen ordinals, not derived with a successor encoder.
        let mut expected_interface = vec![0];
        expected_interface.extend_from_slice(&bytes[1..21]);
        expected_interface.extend_from_slice(&bytes[38..40]);
        expected_interface.push(if bytes[41] == 8 { 5 } else { 6 });
        expected_interface.extend_from_slice(&bytes[42..]);
        assert_eq!(wire_bytes, expected_interface);
        extend(&mut owner);
        let error = super::wire25::OwnerRecord25::try_from(owner).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "kernel_associated_encoding_generation");
    }
}

/// Complete raw logical input for semantic occurrence controls. This is a
/// FullValidator fixture; literal transport/owner/unit bytes above are untouched.
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
                    result: scalar,
                    result_borrow: None,
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
                result: projection,
                result_borrow: None,
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
fn codec_associated_envelopes_require_exact_occurrence_scope_and_owner_generation() {
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
            record.header.contract_version = 32;
        } else if mutation == 1 {
            let OwnerRecord::Declaration(record) = snapshot
                .owners
                .get_mut(&OwnerKey::Declaration(scope.declaration))
                .expect("function")
            else {
                panic!("function");
            };
            record.header.contract_version = 32;
        } else {
            let original_result = match snapshot
                .owners
                .get(&OwnerKey::Declaration(scope.declaration))
                .expect("function")
            {
                OwnerRecord::Declaration(function) => match &function.payload {
                    DeclarationPayload::Function(signature) => signature.result,
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
            signature.result = digest;
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
            "kernel_owned_associated_generation"
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
