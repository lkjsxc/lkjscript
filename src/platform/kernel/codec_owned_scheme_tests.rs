use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

fn reference() -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(b"generic-implementation-codec", 0),
        declaration: DeclarationId::migrate(b"generic-implementation-codec", 0),
    }
}

fn owned_type(form: TypeForm) -> TypeObjectDigest {
    encode_type_object(&TypeObject::new(form).unwrap())
        .unwrap()
        .0
}

fn scheme() -> OwnerRecord {
    let seed = b"generic-implementation-codec";
    let parameter = TypeParameterId::migrate(seed, 0);
    let parameter_type = owned_type(TypeForm::TypeParameter { parameter });
    OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(
            OwnerKey::Declaration(reference().declaration),
            OwnerKind::OwnedImplementation,
        ),
        module: ModuleId::migrate(seed, 0),
        name: Name::new("Flat").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![parameter],
            contract: reference(),
            self_type: owned_type(TypeForm::OwnedSequence {
                item: parameter_type,
            }),
            type_arguments: vec![parameter_type],
            methods: vec![OwnedMethodImplementation {
                method: MethodId::migrate(seed, 0),
                function: reference(),
                type_arguments: vec![parameter_type, owned_type(TypeForm::ByteBuffer)],
            }],
        }),
    })
}

#[test]
fn current_scheme_binds_ordered_parameters_and_every_mapping_argument() {
    let original = scheme();
    let (digest, bytes) = encode_owner(&original).unwrap();
    assert_eq!(&bytes[..8], b"LKJOWN28");
    assert_eq!(
        decode_owner(&bytes, original.owner(), original.kind(), digest).unwrap(),
        original
    );
    let OwnerRecord::Declaration(d) = &original else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &d.payload else {
        unreachable!()
    };
    let expected = std::iter::once(i.self_type)
        .chain(i.type_arguments.iter().copied())
        .chain(
            i.methods
                .iter()
                .flat_map(|m| m.type_arguments.iter().copied()),
        )
        .collect::<Vec<_>>();
    assert_eq!(original.type_roots(), expected);
    assert_eq!(
        PackageInterfaceRecord::project_public(&original)
            .unwrap()
            .unwrap()
            .type_roots(),
        expected
    );
    let mut changed = original.clone();
    let OwnerRecord::Declaration(d) = &mut changed else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.methods[0].type_arguments.reverse();
    assert_ne!(encode_owner(&changed).unwrap().0, digest);
    let OwnerRecord::Declaration(d) = &mut changed else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.type_parameters
        .push(TypeParameterId::migrate(b"generic-implementation-codec", 1));
    let before = encode_owner(&changed).unwrap().0;
    let OwnerRecord::Declaration(d) = &mut changed else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.type_parameters.reverse();
    assert_ne!(encode_owner(&changed).unwrap().0, before);
}

#[test]
fn applied_witness_identity_retains_unused_ordered_arguments() {
    let arguments = vec![
        owned_type(TypeForm::OwnedI64Cell),
        owned_type(TypeForm::ByteBuffer),
    ];
    for method_call in [false, true] {
        let witness = ImplementationOperand::Concrete {
            implementation: reference(),
            type_arguments: arguments.clone(),
        };
        let operation = if method_call {
            ExpressionOperation::MethodCall {
                witness,
                contract: reference(),
                method: MethodId::migrate(b"generic-implementation-codec", 0),
                arguments: Vec::new(),
            }
        } else {
            ExpressionOperation::ImplementationCall {
                requirement_arguments: Vec::new(),
                effect_arguments: Vec::new(),
                function: reference(),
                type_arguments: Vec::new(),
                implementations: vec![witness],
                arguments: Vec::new(),
            }
        };
        let mut owner = OwnerRecord::Expression(
            ExpressionRecord::new(
                ExpressionId::migrate(b"generic-implementation-codec", 0),
                operation,
            )
            .unwrap(),
        );
        assert_eq!(owner.type_roots(), arguments);
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        let OwnerRecord::Expression(e) = &mut owner else {
            unreachable!()
        };
        let witness = match &mut e.operation {
            ExpressionOperation::MethodCall { witness, .. } => witness,
            ExpressionOperation::ImplementationCall {
                implementations, ..
            } => &mut implementations[0],
            _ => unreachable!(),
        };
        let ImplementationOperand::Concrete { type_arguments, .. } = witness else {
            unreachable!()
        };
        type_arguments.reverse();
        assert_ne!(encode_owner(&owner).unwrap().0, digest);
        for generation in 18..=27 {
            owner.set_encoding_for_edit(generation);
            assert_eq!(
                encode_owner(&owner).unwrap_err().code,
                "kernel_implementation_scheme_generation"
            );
        }
    }
}

#[test]
fn predecessor_implementation_bytes_remain_literal_in_graph26_and27() {
    let seed = b"generic-implementation-codec";
    let self_type = owned_type(TypeForm::OwnedI64Cell);
    let type_arguments = vec![owned_type(TypeForm::ByteBuffer)];
    let mappings = vec![(MethodId::migrate(seed, 0), reference())];
    for generation in [26, 27] {
        let header = OwnerHeader {
            contract_version: generation,
            owner: OwnerKey::Declaration(reference().declaration),
            kind: OwnerKind::OwnedImplementation,
        };
        let magic = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
        let domain = format!("lkjscript.kernel.owner-envelope.v{generation}");
        let bytes = packed::encode(
            magic,
            &domain,
            &(
                1_u32,
                header,
                ModuleId::migrate(seed, 0),
                Name::new("Flat").unwrap(),
                DeclarationVisibility::Public,
                9_u32,
                reference(),
                self_type,
                &type_arguments,
                &mappings,
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let digest = OwnerObjectDigest::of(&bytes);
        let mut owner = decode_owner(&bytes, header.owner, header.kind, digest).unwrap();
        assert_eq!(encode_owner(&owner).unwrap(), (digest, bytes));
        let OwnerRecord::Declaration(d) = &mut owner else {
            unreachable!()
        };
        let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
            unreachable!()
        };
        assert!(i.type_parameters.is_empty());
        assert!(i.methods[0].type_arguments.is_empty());
        i.methods[0].type_arguments.push(self_type);
        assert_eq!(
            encode_owner(&owner).unwrap_err().code,
            "kernel_implementation_scheme_generation"
        );
    }
}

#[test]
fn schemes_cannot_downcast_or_exceed_structural_mapping_limits() {
    let mut owner = scheme();
    for generation in 18..=27 {
        owner.set_encoding_for_edit(generation);
        assert_eq!(
            encode_owner(&owner).unwrap_err().code,
            "kernel_implementation_scheme_generation"
        );
        let magic = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
        let domain = format!("lkjscript.kernel.owner-envelope.v{generation}");
        let disguised = packed::encode(magic, &domain, &owner, MAXIMUM_OWNER_OBJECT_BYTES).unwrap();
        assert!(
            decode_owner(
                &disguised,
                owner.owner(),
                owner.kind(),
                OwnerObjectDigest::of(&disguised)
            )
            .is_err()
        );
    }
    owner.set_encoding_for_edit(28);
    let OwnerRecord::Declaration(d) = &mut owner else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.methods[0].type_arguments = vec![i.self_type; contract::MAXIMUM_CHILDREN + 1];
    assert!(encode_owner(&owner).is_err());
}
