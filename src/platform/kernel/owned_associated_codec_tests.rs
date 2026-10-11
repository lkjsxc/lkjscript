//! Literal predecessor objects and independent frozen-layout witnesses for the additive codecs.
use super::*;
use crate::platform::compiler::{CompilationPayload, CompilationUnit, CompilationUnitKey};
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::package_interface::{PackageInterfaceOwner, PackageInterfaceOwnerDigest};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ImplementationParameterId,
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

#[test]
fn codec_foundation_cannot_admit_associated_meaning_before_semantic_integration() {
    let base = super::tests::witness_snapshot();
    let mut local = base.clone();
    let mut owner = base_owner(false);
    extend(&mut owner);
    local.owners.insert(owner.owner(), owner.clone());
    assert!(
        validate_full(&local)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "owned_associated_semantics_unsupported")
    );
    let mut imported = base.clone();
    let interface = PackageInterfaceRecord::project_public(&owner)
        .unwrap()
        .unwrap();
    imported.dependency_interfaces.insert(
        PackageRevisionDigest::from_bytes([91; 32]),
        std::sync::Arc::new(std::collections::BTreeMap::from([(
            owner.owner(),
            interface,
        )])),
    );
    assert!(
        validate_full(&imported)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "owned_associated_semantics_unsupported")
    );
    let form = TypeForm::AssociatedContractMember {
        contract: DeclarationReference {
            package: base.root.package_id,
            declaration: DeclarationId::from_bytes([2; 16]).unwrap(),
        },
        member: member().id,
    };
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    for dependency in [false, true] {
        let mut snapshot = base.clone();
        if dependency {
            snapshot.dependency_types.insert(digest, object.clone());
        } else {
            snapshot.types.insert(digest, object.clone());
        }
        assert!(
            validate_full(&snapshot)
                .unwrap_err()
                .iter()
                .any(|error| error.code == "owned_associated_semantics_unsupported")
        );
    }
}
