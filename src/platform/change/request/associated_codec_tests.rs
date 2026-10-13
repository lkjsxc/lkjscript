//! Literal authored commitment layouts, independent of the production writer helpers.
use super::*;
use crate::platform::kernel::{AssociatedTypeBound, PackageId};
use crate::platform::semantic_id::{AssociatedTypeMemberId, ImplementationParameterId, MethodId};

fn declaration(ordinal: u64) -> AuthoredDeclarationReference {
    AuthoredDeclarationReference::Exact {
        package: PackageId::migrate(b"associated-intent", 1),
        declaration: DeclarationId::migrate(b"associated-intent", ordinal),
    }
}

fn member() -> AuthoredAssociatedTypeMember {
    AuthoredAssociatedTypeMember {
        id: AssociatedTypeMemberId::migrate(b"associated-intent", 1),
        name: Name::new("Out").unwrap(),
        bound: AssociatedTypeBound::Owned,
    }
}

fn count(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn exact(bytes: &mut Vec<u8>, ordinal: u64) {
    bytes.push(2);
    bytes.extend_from_slice(&PackageId::migrate(b"associated-intent", 1).bytes());
    bytes.extend_from_slice(&DeclarationId::migrate(b"associated-intent", ordinal).bytes());
}

fn operation(create: bool, contract: bool, associated: bool) -> AuthoredChange {
    let members = if associated { vec![member()] } else { vec![] };
    let bindings = if associated {
        vec![AuthoredAssociatedTypeBinding {
            member: member().id,
            value: AuthoredAssociatedTypeBindingValue::Owned {
                ty: AuthoredType::OwnedI64Cell {},
            },
        }]
    } else {
        vec![]
    };
    let methods = vec![AuthoredOwnedMethod {
        id: MethodId::migrate(b"associated-intent", 1),
        name: Name::new("read").unwrap(),
        parameters: vec![],
        result: AuthoredCallableResult::value(AuthoredType::I64 {}),
        read_scope_parameters: vec![],
        read_scope_sources: vec![],
        effect: AuthoredFunctionEffect::Pure {},
    }];
    let mappings = vec![AuthoredOwnedMethodImplementation {
        method: methods[0].id,
        function: declaration(7),
        type_arguments: vec![],
        read_scope_arguments: vec![],
        implementations: vec![],
    }];
    let self_type = if contract {
        AuthoredType::TypeParameter {
            parameter: AuthoredTypeParameterReference::Id {
                parameter: TypeParameterId::migrate(b"associated-intent", 1),
            },
        }
    } else {
        AuthoredType::OwnedI64Cell {}
    };
    let selected = DeclarationSelector::Id {
        declaration: DeclarationId::migrate(b"associated-intent", 2),
    };
    match (create, contract) {
        (true, true) => AuthoredChange::CreateOwnedContract {
            symbol: "$contract".into(),
            module: ModuleSelector::Id {
                module: ModuleId::migrate(b"associated-intent", 1),
            },
            name: Name::new("Contract").unwrap(),
            visibility: DeclarationVisibility::Public,
            self_type,
            type_parameters: vec![],
            methods,
            associated_types: members,
        },
        (false, true) => AuthoredChange::SetOwnedContract {
            declaration: selected,
            self_type,
            type_parameters: vec![],
            methods,
            associated_types: members,
        },
        (true, false) => AuthoredChange::CreateOwnedImplementation {
            symbol: "$implementation".into(),
            module: ModuleSelector::Id {
                module: ModuleId::migrate(b"associated-intent", 1),
            },
            name: Name::new("Implementation").unwrap(),
            visibility: DeclarationVisibility::Public,
            contract: declaration(3),
            type_parameters: vec![],
            implementation_parameters: vec![],
            self_type,
            type_arguments: vec![],
            methods: mappings,
            associated_types: bindings,
        },
        (false, false) => AuthoredChange::SetOwnedImplementation {
            declaration: selected,
            contract: declaration(3),
            type_parameters: vec![],
            implementation_parameters: vec![],
            self_type,
            type_arguments: vec![],
            methods: mappings,
            associated_types: bindings,
        },
    }
}

fn encode(change: AuthoredChange) -> Vec<u8> {
    canonical_authored_intent_bytes(&AuthoredChangeSet {
        base: RevisionId::from_digest([9; 32]),
        preconditions: vec![],
        changes: vec![change],
        budget: ChangeBudget::default(),
    })
    .unwrap()
}

fn expected(create: bool, contract: bool, associated: bool) -> Vec<u8> {
    let mut bytes = b"LKJACR39".to_vec();
    bytes.extend_from_slice(&[9; 32]);
    count(&mut bytes, 0); // Preconditions.
    count(&mut bytes, 1); // Operations.
    bytes.push(match (create, contract) {
        (true, true) => 100,
        (true, false) => 101,
        (false, true) => 102,
        (false, false) => 103,
    });
    if create {
        bytes.push(2); // Declaration symbol kind.
        count(&mut bytes, 1); // First declaration in this request.
        bytes.push(1); // Exact module selector.
        bytes.extend_from_slice(&ModuleId::migrate(b"associated-intent", 1).bytes());
        let name = if contract {
            b"Contract".as_slice()
        } else {
            b"Implementation".as_slice()
        };
        count(&mut bytes, name.len() as u64);
        bytes.extend_from_slice(name);
        bytes.push(3); // Public.
    } else {
        bytes.push(1); // Exact declaration selector.
        bytes.extend_from_slice(&DeclarationId::migrate(b"associated-intent", 2).bytes());
    }
    if contract {
        bytes.extend_from_slice(&[8, 1]); // Exact Self type parameter.
        bytes.extend_from_slice(&TypeParameterId::migrate(b"associated-intent", 1).bytes());
        count(&mut bytes, 0); // Complete ordinary parameter inventory.
        count(&mut bytes, 1); // Methods.
        bytes.extend_from_slice(&MethodId::migrate(b"associated-intent", 1).bytes());
        count(&mut bytes, 4);
        bytes.extend_from_slice(b"read");
        count(&mut bytes, 0); // Method parameters.
        bytes.extend_from_slice(&[3, 1]); // I64 result; Value relation.
        count(&mut bytes, 0); // Scope parameter inventory.
        count(&mut bytes, 0); // Scope source inventory.
        bytes.push(1); // Pure effect.
        count(&mut bytes, u64::from(associated));
        if associated {
            bytes.extend_from_slice(&member().id.bytes());
            count(&mut bytes, 3);
            bytes.extend_from_slice(b"Out");
            bytes.push(1); // Owned bound.
        }
    } else {
        exact(&mut bytes, 3);
        count(&mut bytes, 0); // Ordinary parameters.
        count(&mut bytes, 0); // Prerequisite parameters.
        bytes.push(21); // Self Cell.
        count(&mut bytes, 0); // Contract actuals.
        count(&mut bytes, 1); // Mappings.
        bytes.extend_from_slice(&MethodId::migrate(b"associated-intent", 1).bytes());
        exact(&mut bytes, 7);
        count(&mut bytes, 0); // Complete mapping ordinary actuals.
        count(&mut bytes, 0); // Complete mapping scope actuals.
        count(&mut bytes, 0); // Complete mapping prerequisites.
        count(&mut bytes, u64::from(associated));
        if associated {
            bytes.extend_from_slice(&member().id.bytes());
            bytes.extend_from_slice(&[1, 21]); // Owned binding; Cell.
        }
    }
    bytes
}

#[test]
fn associated_create_set_commit_complete_fields_and_preserve_empty_inventory_literals() {
    for create in [false, true] {
        for contract in [false, true] {
            for associated in [false, true] {
                assert_eq!(
                    encode(operation(create, contract, associated)),
                    expected(create, contract, associated)
                );
            }
        }
    }
}

#[test]
fn associated_projection_commitment_retains_every_authority_coordinate_and_nested_profile() {
    let parameter = ImplementationParameterId::migrate(b"associated-intent", 1);
    let types = [
        AuthoredType::AssociatedContractMember {
            contract: declaration(3),
            member: member().id,
        },
        AuthoredType::AssociatedWitnessProjection {
            scope: declaration(5),
            parameter,
            contract: declaration(3),
            member: member().id,
        },
    ];
    for (index, ty) in types.iter().enumerate() {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer.authored_type(ty, &BTreeMap::new(), 1).unwrap();
        let mut literal = vec![25 + index as u8];
        if index == 1 {
            exact(&mut literal, 5);
            literal.extend_from_slice(&parameter.bytes());
        }
        exact(&mut literal, 3);
        literal.extend_from_slice(&member().id.bytes());
        assert_eq!(writer.bytes, literal);
        let mut change = operation(false, true, false);
        let AuthoredChange::SetOwnedContract { methods, .. } = &mut change else {
            unreachable!()
        };
        methods[0].result.ty = AuthoredType::Option {
            item: Box::new(ty.clone()),
        };
        let encoded = encode(change);
        assert_eq!(&encoded[..8], b"LKJACR39");
        assert_eq!(encoded[56], 102); // Complete layout also includes the empty member inventory.
    }
    let mut mutations = vec![types[1].clone(); 4];
    if let AuthoredType::AssociatedWitnessProjection { scope, .. } = &mut mutations[0] {
        *scope = declaration(6);
    }
    if let AuthoredType::AssociatedWitnessProjection { parameter, .. } = &mut mutations[1] {
        *parameter = ImplementationParameterId::migrate(b"associated-intent", 2);
    }
    if let AuthoredType::AssociatedWitnessProjection { contract, .. } = &mut mutations[2] {
        *contract = declaration(4);
    }
    if let AuthoredType::AssociatedWitnessProjection { member, .. } = &mut mutations[3] {
        *member = AssociatedTypeMemberId::migrate(b"associated-intent", 2);
    }
    let encode_type = |ty: &AuthoredType| {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer.authored_type(ty, &BTreeMap::new(), 1).unwrap();
        writer.bytes
    };
    for mutation in mutations {
        assert_ne!(encode_type(&types[1]), encode_type(&mutation));
    }
}
