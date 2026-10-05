//! Strict current/predecessor owner codecs and unchanged base TypeObject 10.

use super::contract::{
    DEPENDENCY_ENVELOPE_DOMAIN, DEPENDENCY_MAGIC, MAXIMUM_DEPENDENCY_BYTES,
    MAXIMUM_OWNER_OBJECT_BYTES, MAXIMUM_RETIREMENT_BYTES, MAXIMUM_ROOT_BYTES,
    MAXIMUM_TYPE_OBJECT_BYTES, OWNER_ENVELOPE_DOMAIN, OWNER_MAGIC, RETIREMENT_ENVELOPE_DOMAIN,
    RETIREMENT_MAGIC, ROOT_ENVELOPE_DOMAIN, ROOT_MAGIC, TYPE_OBJECT_ENVELOPE_DOMAIN,
    TYPE_OBJECT_MAGIC,
};
use super::digest::{
    DependencyObjectDigest, OwnerObjectDigest, RetirementObjectDigest, SemanticRootDigest,
    TypeObjectDigest,
};
use super::id::{OwnerKey, OwnerKind, PackageId};
use super::owner::{OwnerBinding, OwnerRecord};
use super::root::{
    DependencyBinding, DependencyRecord, RetirementBinding, RetirementRecord, SemanticRoot,
};
use super::type_object::TypeObject;
use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
use crate::platform::packed;

pub const OWNER_BINDING_BYTES: usize = 33;
pub const DEPENDENCY_BINDING_BYTES: usize = 32;
pub const RETIREMENT_BINDING_BYTES: usize = 32;

#[cfg(test)]
mod borrowed_result_encoding_tests {
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::*;

    fn function() -> OwnerRecord {
        let seed = b"borrowed-result-canonical-relation";
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(DeclarationId::migrate(seed, 0)),
                OwnerKind::PureFunction,
            ),
            module: ModuleId::migrate(seed, 0),
            name: Name::new("view").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                implementation_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: vec![ParameterId::migrate(seed, 0), ParameterId::migrate(seed, 1)],
                result: encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
                    .unwrap()
                    .0,
                result_borrow: Some(ParameterId::migrate(seed, 0)),
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(seed, 0),
            }),
        })
    }

    #[test]
    fn canonical_result_relation_preserves_exact_source_identity() {
        let original = function();
        let (digest, bytes) = encode_owner(&original).unwrap();
        assert_eq!(&bytes[..8], b"LKJOWN27");
        assert_eq!(
            decode_owner(&bytes, original.owner(), original.kind(), digest).unwrap(),
            original
        );
        let mut alternate = original.clone();
        let OwnerRecord::Declaration(d) = &mut alternate else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result_borrow = Some(f.parameters[1]);
        assert_ne!(encode_owner(&alternate).unwrap().0, digest);
        let OwnerRecord::Declaration(d) = &mut alternate else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result_borrow = None;
        assert_ne!(encode_owner(&alternate).unwrap().0, digest);
    }

    #[test]
    fn graph26_function_bytes_remain_literal_and_borrow_modes_cannot_downcast() {
        let mut original = function();
        original.set_encoding_for_edit(26);
        assert_eq!(
            encode_owner(&original).unwrap_err().code,
            "kernel_borrow_result_generation"
        );
        let OwnerRecord::Declaration(d) = &mut original else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result_borrow = None;
        let literal = packed::encode(
            super::super::contract::PARAMETERIZED_CONTRACT_OWNER_MAGIC,
            super::super::contract::PARAMETERIZED_CONTRACT_OWNER_ENVELOPE_DOMAIN,
            &(
                1_u32,
                d.header,
                d.module,
                &d.name,
                d.visibility,
                4_u32,
                &f.implementation_parameters,
                &f.requirement_parameters,
                &f.effect_parameters,
                &f.type_parameters,
                &f.parameters,
                f.result,
                &f.effect,
                f.body,
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let digest = OwnerObjectDigest::of(&literal);
        assert_eq!(encode_owner(&original).unwrap(), (digest, literal.clone()));
        assert_eq!(
            decode_owner(&literal, original.owner(), original.kind(), digest).unwrap(),
            original
        );
    }

    #[test]
    fn rehashed_invalid_source_relation_is_rejected_by_decoder() {
        let mut invalid = function();
        let OwnerRecord::Declaration(d) = &mut invalid else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result_borrow = Some(ParameterId::migrate(b"foreign-source", 0));
        let bytes = packed::encode(
            OWNER_MAGIC,
            OWNER_ENVELOPE_DOMAIN,
            &invalid,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let error = decode_owner(
            &bytes,
            invalid.owner(),
            invalid.kind(),
            OwnerObjectDigest::of(&bytes),
        )
        .unwrap_err();
        assert_eq!(error.code, "kernel_borrow_result");
    }

    #[test]
    fn method_result_position_is_canonical_and_graph26_method_layout_is_frozen() {
        let seed = b"borrowed-method-position-wire";
        let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let mut record = OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(DeclarationId::migrate(seed, 0)),
                OwnerKind::OwnedContract,
            ),
            module: ModuleId::migrate(seed, 0),
            name: Name::new("IndexRead").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: TypeParameterId::migrate(seed, 0),
                type_parameters: Vec::new(),
                methods: vec![OwnedMethod {
                    id: MethodId::migrate(seed, 0),
                    name: Name::new("at").unwrap(),
                    parameters: vec![
                        OwnedMethodParameter {
                            ty,
                            use_mode: ParameterUse::Borrow
                        };
                        2
                    ],
                    result: ty,
                    result_borrow: Some(0),
                    effect: FunctionEffect::Pure,
                }],
            }),
        });
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(
            decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
            record
        );
        let OwnerRecord::Declaration(d) = &mut record else {
            unreachable!()
        };
        let DeclarationPayload::OwnedContract(c) = &mut d.payload else {
            unreachable!()
        };
        c.methods[0].result_borrow = Some(1);
        assert_ne!(encode_owner(&record).unwrap().0, digest);
        record.set_encoding_for_edit(26);
        assert_eq!(
            encode_owner(&record).unwrap_err().code,
            "kernel_borrow_result_generation"
        );
        let OwnerRecord::Declaration(d) = &mut record else {
            unreachable!()
        };
        let DeclarationPayload::OwnedContract(c) = &mut d.payload else {
            unreachable!()
        };
        c.methods[0].result_borrow = None;
        let original_methods = c
            .methods
            .iter()
            .map(|m| (&m.id, &m.name, &m.parameters, m.result, &m.effect))
            .collect::<Vec<_>>();
        let literal = packed::encode(
            super::super::contract::PARAMETERIZED_CONTRACT_OWNER_MAGIC,
            super::super::contract::PARAMETERIZED_CONTRACT_OWNER_ENVELOPE_DOMAIN,
            &(
                1_u32,
                d.header,
                d.module,
                &d.name,
                d.visibility,
                8_u32,
                c.self_parameter,
                &c.type_parameters,
                original_methods,
            ),
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let digest = OwnerObjectDigest::of(&literal);
        assert_eq!(encode_owner(&record).unwrap(), (digest, literal.clone()));
        assert_eq!(
            decode_owner(&literal, record.owner(), record.kind(), digest).unwrap(),
            record
        );
    }

    #[test]
    fn borrow_call_has_ordered_children_and_requires_graph27() {
        let seed = b"borrow-call-wire";
        let call = ExpressionId::migrate(seed, 1);
        let body = ExpressionId::migrate(seed, 2);
        let mut expression = ExpressionRecord::new(
            ExpressionId::migrate(seed, 0),
            ExpressionOperation::BorrowCall {
                call,
                binding: BindingId::migrate(seed, 0),
                body,
            },
        )
        .unwrap();
        assert_eq!(
            expression.children(),
            vec![
                ExpressionChild {
                    expression: call,
                    role: ExpressionChildRole::BorrowCallInvocation,
                    ordinal: 0
                },
                ExpressionChild {
                    expression: body,
                    role: ExpressionChildRole::BorrowCallBody,
                    ordinal: 0
                },
            ]
        );
        let record = OwnerRecord::Expression(expression.clone());
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(
            decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
            record
        );
        expression.contract_version = 26;
        assert_eq!(
            encode_owner(&OwnerRecord::Expression(expression))
                .unwrap_err()
                .code,
            "kernel_borrow_result_generation"
        );
    }
}

#[cfg(test)]
mod parameterized_contract_encoding_tests {
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::*;

    #[test]
    fn original_contract_and_application_fields_round_trip_in_graph18_through25() {
        let seed = b"frozen-contract-applications";
        let declaration = DeclarationId::migrate(seed, 0);
        let owner = OwnerKey::Declaration(declaration);
        let module = ModuleId::migrate(seed, 0);
        let name = Name::new("Contract").unwrap();
        let self_parameter = TypeParameterId::migrate(seed, 0);
        let method = MethodId::migrate(seed, 0);
        let scalar = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let reference = DeclarationReference {
            package: "pkg_10000000000000000000000000000001".parse().unwrap(),
            declaration,
        };
        let methods = [OwnedMethod {
            id: method,
            name: Name::new("length").unwrap(),
            parameters: Vec::new(),
            result: scalar,
            result_borrow: None,
            effect: FunctionEffect::Pure,
        }];

        let original_methods = methods
            .iter()
            .map(|method| {
                (
                    &method.id,
                    &method.name,
                    &method.parameters,
                    &method.result,
                    &method.effect,
                )
            })
            .collect::<Vec<_>>();
        let mappings = vec![OwnedMethodImplementation {
            method,
            function: reference,
        }];
        let parameter_id = ImplementationParameterId::migrate(seed, 0);
        let parameter_name = Name::new("Witness").unwrap();
        for generation in 18..=25 {
            let header = OwnerHeader {
                contract_version: generation,
                owner,
                kind: OwnerKind::OwnedContract,
            };
            let magic: [u8; 8] = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
            let domain = format!("lkjscript.kernel.owner-envelope.v{generation}");
            // Literal original enum ordinals and nested fields are independent
            // of both current records and predecessor conversion code.
            let original_contract = packed::encode(
                magic,
                &domain,
                &(
                    1_u32,
                    header,
                    module,
                    &name,
                    DeclarationVisibility::Public,
                    8_u32,
                    self_parameter,
                    &original_methods,
                ),
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            let mut original_implementation_header = header;
            original_implementation_header.kind = OwnerKind::OwnedImplementation;
            let original_implementation = packed::encode(
                magic,
                &domain,
                &(
                    1_u32,
                    original_implementation_header,
                    module,
                    &name,
                    DeclarationVisibility::Public,
                    9_u32,
                    reference,
                    scalar,
                    &mappings,
                ),
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            let mut function_header = header;
            function_header.kind = OwnerKind::PureFunction;
            let original_parameters = vec![(parameter_id, &parameter_name, reference, scalar)];
            let original_function = packed::encode(
                magic,
                &domain,
                &(
                    1_u32,
                    function_header,
                    module,
                    &name,
                    DeclarationVisibility::Public,
                    4_u32,
                    original_parameters,
                    Vec::<RequirementParameterId>::new(),
                    Vec::<EffectParameterId>::new(),
                    Vec::<TypeParameterId>::new(),
                    Vec::<ParameterId>::new(),
                    scalar,
                    FunctionEffect::Pure,
                    ExpressionId::migrate(seed, 0),
                ),
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            for (kind, bytes) in [
                (OwnerKind::OwnedContract, original_contract),
                (OwnerKind::OwnedImplementation, original_implementation),
                (OwnerKind::PureFunction, original_function),
            ] {
                let digest = OwnerObjectDigest::of(&bytes);
                let record = decode_owner(&bytes, owner, kind, digest).unwrap();
                let OwnerRecord::Declaration(declaration) = &record else {
                    panic!("declaration");
                };
                match &declaration.payload {
                    DeclarationPayload::OwnedContract(c) => assert!(c.type_parameters.is_empty()),
                    DeclarationPayload::OwnedImplementation(i) => {
                        assert!(i.type_arguments.is_empty())
                    }
                    DeclarationPayload::Function(f) => {
                        assert!(f.implementation_parameters[0].type_arguments.is_empty())
                    }
                    _ => panic!("contract application"),
                }
                assert_eq!(encode_owner(&record).unwrap(), (digest, bytes));
            }
        }
    }

    #[test]
    fn new_contract_parameter_fields_require_the_current_envelope() {
        let seed = b"current-contract-applications";
        let declaration = DeclarationId::migrate(seed, 0);
        let owner = OwnerKey::Declaration(declaration);
        let scalar = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let mut record = OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(owner, OwnerKind::OwnedContract),
            module: ModuleId::migrate(seed, 0),
            name: Name::new("Worklist").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::OwnedContract(OwnedContract {
                self_parameter: TypeParameterId::migrate(seed, 0),
                type_parameters: vec![TypeParameterId::migrate(seed, 1)],
                methods: vec![OwnedMethod {
                    id: MethodId::migrate(seed, 0),
                    name: Name::new("length").unwrap(),
                    parameters: Vec::new(),
                    result: scalar,
                    result_borrow: None,
                    effect: FunctionEffect::Pure,
                }],
            }),
        });
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(&bytes[..8], b"LKJOWN27");
        assert_eq!(
            decode_owner(&bytes, owner, record.kind(), digest).unwrap(),
            record
        );
        for generation in 18..=25 {
            record.set_encoding_for_edit(generation);
            assert_eq!(
                encode_owner(&record).unwrap_err().code,
                "kernel_parameterized_contract_generation"
            );
            let magic: [u8; 8] = format!("LKJOWN{generation}").as_bytes().try_into().unwrap();
            let bytes = packed::encode(
                magic,
                &format!("lkjscript.kernel.owner-envelope.v{generation}"),
                &record,
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            assert!(
                decode_owner(&bytes, owner, record.kind(), OwnerObjectDigest::of(&bytes)).is_err()
            );
        }
    }

    #[test]
    fn unused_implementation_arguments_are_ordered_canonical_and_interface_type_roots() {
        let seed = b"unused-implementation-argument-roots";
        let declaration = DeclarationId::migrate(seed, 0);
        let reference = DeclarationReference {
            package: "pkg_10000000000000000000000000000001".parse().unwrap(),
            declaration,
        };
        let self_type = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
            .unwrap()
            .0;
        let argument = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let result = encode_type_object(&TypeObject::new(TypeForm::Unit).unwrap())
            .unwrap()
            .0;
        let implementation = OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration),
                OwnerKind::OwnedImplementation,
            ),
            module: ModuleId::migrate(seed, 0),
            name: Name::new("Concrete").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
                contract: reference,
                self_type,
                type_arguments: vec![argument, self_type],
                methods: vec![OwnedMethodImplementation {
                    method: MethodId::migrate(seed, 0),
                    function: reference,
                }],
            }),
        });
        let function = OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::PureFunction),
            module: ModuleId::migrate(seed, 0),
            name: Name::new("generic").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                implementation_parameters: vec![ImplementationParameter {
                    id: ImplementationParameterId::migrate(seed, 0),
                    name: Name::new("W").unwrap(),
                    contract: reference,
                    self_type,
                    type_arguments: vec![argument, self_type],
                }],
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result,
                result_borrow: None,
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(seed, 0),
            }),
        });
        for (record, expected) in [
            (implementation, vec![self_type, argument, self_type]),
            (function, vec![self_type, argument, self_type, result]),
        ] {
            assert_eq!(record.type_roots(), expected);
            let projected = PackageInterfaceRecord::project_public(&record)
                .unwrap()
                .unwrap();
            assert_eq!(projected.type_roots(), expected);
            let (digest, bytes) = encode_owner(&record).unwrap();
            assert_eq!(
                decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
                record
            );
            let mut previous = record;
            previous.set_encoding_for_edit(25);
            assert_eq!(
                encode_owner(&previous).unwrap_err().code,
                "kernel_parameterized_contract_generation"
            );
        }
    }
}

#[cfg(test)]
mod implementation_application_encoding_tests {
    use super::*;
    use crate::platform::kernel::{
        DeclarationReference, EffectRow, ExpressionOperation, ExpressionRecord,
        ImplementationOperand, RequirementOperand, RequirementReference, TypeForm,
    };
    use crate::platform::semantic_id::{DeclarationId, ExpressionId, RequirementId};

    fn references() -> (ExpressionId, DeclarationReference, RequirementOperand) {
        let seed = b"implementation-application-codec";
        let package = "pkg_10000000000000000000000000000001".parse().unwrap();
        (
            ExpressionId::migrate(seed, 0),
            DeclarationReference {
                package,
                declaration: DeclarationId::migrate(seed, 0),
            },
            RequirementReference {
                package,
                requirement: RequirementId::migrate(seed, 0),
            }
            .into(),
        )
    }

    fn envelope(generation: u16) -> ([u8; 8], String) {
        (
            format!("LKJOWN{generation}").as_bytes().try_into().unwrap(),
            format!("lkjscript.kernel.owner-envelope.v{generation}"),
        )
    }

    #[test]
    fn predecessor_implementation_calls_retain_exact_bytes_and_empty_new_operands() {
        let (id, function, _) = references();
        let type_arguments = vec![
            encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                .unwrap()
                .0,
        ];
        let implementations = vec![ImplementationOperand::Concrete {
            implementation: function,
        }];
        let arguments = vec![ExpressionId::migrate(
            b"implementation-application-codec",
            1,
        )];
        for generation in 18..=22 {
            let (magic, domain) = envelope(generation);
            // Original Graph 18–22 ordinals and fields, independent of both
            // the current expression enum and its frozen conversion.
            let original = packed::encode(
                magic,
                &domain,
                &(
                    9_u32,
                    generation,
                    id,
                    24_u32,
                    function,
                    &type_arguments,
                    &implementations,
                    &arguments,
                ),
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            let digest = OwnerObjectDigest::of(&original);
            let owner = decode_owner(
                &original,
                OwnerKey::Expression(id),
                OwnerKind::Expression,
                digest,
            )
            .unwrap();
            assert_eq!(
                owner,
                OwnerRecord::Expression(ExpressionRecord {
                    contract_version: generation,
                    id,
                    operation: ExpressionOperation::ImplementationCall {
                        requirement_arguments: Vec::new(),
                        effect_arguments: Vec::new(),
                        function,
                        type_arguments: type_arguments.clone(),
                        implementations: implementations.clone(),
                        arguments: arguments.clone(),
                    },
                })
            );
            assert_eq!(encode_owner(&owner).unwrap(), (digest, original));
        }
    }

    #[test]
    fn ordinary_call_bytes_are_preserved_in_every_predecessor_generation() {
        let (id, function, requirement) = references();
        let effect_arguments = vec![EffectRow {
            requirements: vec![requirement],
            parameters: Vec::new(),
        }];
        let type_arguments = vec![
            encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                .unwrap()
                .0,
        ];
        let arguments = vec![ExpressionId::migrate(
            b"implementation-application-codec",
            1,
        )];
        for generation in 14..=22 {
            let (magic, domain) = envelope(generation);
            let requirement_arguments = if generation == 14 {
                Vec::new()
            } else {
                vec![requirement]
            };
            let original = if generation == 14 {
                let effects = effect_arguments
                    .iter()
                    .cloned()
                    .map(super::super::wire14::EffectRow14::try_from)
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap();
                packed::encode(
                    magic,
                    &domain,
                    &(
                        9_u32,
                        generation,
                        id,
                        10_u32,
                        effects,
                        function,
                        &type_arguments,
                        &arguments,
                    ),
                    MAXIMUM_OWNER_OBJECT_BYTES,
                )
                .unwrap()
            } else {
                packed::encode(
                    magic,
                    &domain,
                    &(
                        9_u32,
                        generation,
                        id,
                        10_u32,
                        &requirement_arguments,
                        &effect_arguments,
                        function,
                        &type_arguments,
                        &arguments,
                    ),
                    MAXIMUM_OWNER_OBJECT_BYTES,
                )
                .unwrap()
            };
            let expected = OwnerRecord::Expression(ExpressionRecord {
                contract_version: generation,
                id,
                operation: ExpressionOperation::Call {
                    requirement_arguments,
                    effect_arguments: effect_arguments.clone(),
                    function,
                    type_arguments: type_arguments.clone(),
                    arguments: arguments.clone(),
                },
            });
            let digest = OwnerObjectDigest::of(&original);
            assert_eq!(encode_owner(&expected).unwrap(), (digest, original.clone()));
            assert_eq!(
                decode_owner(&original, expected.owner(), expected.kind(), digest).unwrap(),
                expected
            );
        }
    }

    #[test]
    fn new_implementation_operands_round_trip_from_graph23() {
        let (id, function, requirement) = references();
        for (requirement_arguments, effect_arguments) in [
            (vec![requirement], Vec::new()),
            (Vec::new(), vec![EffectRow::default()]),
            (
                vec![requirement],
                vec![EffectRow {
                    requirements: vec![requirement],
                    parameters: Vec::new(),
                }],
            ),
        ] {
            let expression = ExpressionRecord::new(
                id,
                ExpressionOperation::ImplementationCall {
                    requirement_arguments,
                    effect_arguments,
                    function,
                    type_arguments: Vec::new(),
                    implementations: Vec::new(),
                    arguments: Vec::new(),
                },
            )
            .unwrap();
            let current = OwnerRecord::Expression(expression.clone());
            let (digest, bytes) = encode_owner(&current).unwrap();
            assert_eq!(&bytes[..8], b"LKJOWN27");
            assert_eq!(
                decode_owner(&bytes, current.owner(), current.kind(), digest).unwrap(),
                current
            );
            let mut previous = current.clone();
            previous.set_encoding_for_edit(23);
            let (digest, bytes) = encode_owner(&previous).unwrap();
            assert_eq!(&bytes[..8], b"LKJOWN23");
            assert_eq!(
                decode_owner(&bytes, previous.owner(), previous.kind(), digest).unwrap(),
                previous
            );
            assert_eq!(
                super::super::wire22::ExpressionRecord22::try_from(expression.clone())
                    .unwrap_err()
                    .code,
                "kernel_owned_effect_generation"
            );
            for generation in 18..=22 {
                let mut predecessor = current.clone();
                predecessor.set_encoding_for_edit(generation);
                assert_eq!(
                    encode_owner(&predecessor).unwrap_err().code,
                    "kernel_owned_effect_generation"
                );
                let (magic, domain) = envelope(generation);
                let disguised =
                    packed::encode(magic, &domain, &predecessor, MAXIMUM_OWNER_OBJECT_BYTES)
                        .unwrap();
                assert!(
                    decode_owner(
                        &disguised,
                        predecessor.owner(),
                        predecessor.kind(),
                        OwnerObjectDigest::of(&disguised),
                    )
                    .is_err()
                );
            }
        }
    }
}

#[cfg(test)]
mod nominal_encoding_tests {
    use super::*;
    use crate::platform::kernel::{DeclarationReference, TypeForm};

    #[test]
    fn transfer_constraint_owners_round_trip_from_graph22() {
        use crate::platform::kernel::{
            Name, OwnerHeader, TypeParameterConstraints as C, TypeParameterRecord,
        };
        use crate::platform::semantic_id::{DeclarationId, TypeParameterId};
        let key =
            OwnerKey::TypeParameter(TypeParameterId::migrate(b"transfer-constraint-codec", 0));
        for constraint in [
            C::Transferable,
            C::CaptureSafeTransferable,
            C::OwnedTransferable,
        ] {
            let mut record = OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(key, OwnerKind::TypeParameter),
                declaration: DeclarationId::migrate(b"transfer-constraint-codec", 0),
                name: Name::new("T").unwrap(),
                constraints: constraint,
            });
            let (digest, bytes) = encode_owner(&record).unwrap();
            assert_eq!(&bytes[..8], b"LKJOWN27");
            assert_eq!(
                decode_owner(&bytes, key, OwnerKind::TypeParameter, digest).unwrap(),
                record
            );
            record.set_encoding_for_edit(22);
            let (digest, bytes) = encode_owner(&record).unwrap();
            assert_eq!(&bytes[..8], b"LKJOWN22");
            assert_eq!(
                decode_owner(&bytes, key, OwnerKind::TypeParameter, digest).unwrap(),
                record
            );
            record.set_encoding_for_edit(21);
            assert_eq!(
                encode_owner(&record).unwrap_err().code,
                "kernel_transfer_constraint_generation"
            );
            let forged = packed::encode(
                super::super::contract::PARALLEL_OWNER_MAGIC,
                super::super::contract::PARALLEL_OWNER_ENVELOPE_DOMAIN,
                &record,
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            assert_eq!(
                decode_owner(
                    &forged,
                    key,
                    OwnerKind::TypeParameter,
                    OwnerObjectDigest::of(&forged)
                )
                .unwrap_err()
                .code,
                "kernel_transfer_constraint_generation"
            );
        }
    }

    #[test]
    fn f64_type_and_literal_have_disjoint_canonical_generations() {
        use crate::platform::binary64::Binary64;
        use crate::platform::kernel::{ExpressionOperation, ExpressionRecord};
        let object = TypeObject::new(TypeForm::F64).unwrap();
        let (digest, bytes) = encode_type_object(&object).unwrap();
        assert_eq!(&bytes[..8], b"LKJF6401");
        assert_eq!(&bytes[18..20], &[1, 1]);
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), object);
        let disguised = packed::encode(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &object,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert!(decode_type_object(&disguised, TypeObjectDigest::of(&disguised)).is_err());
        for (version, tag) in [(0_u16, 1_u8), (10, 1), (1, 0), (1, 2)] {
            let malformed = packed::encode(
                super::super::contract::F64_TYPE_MAGIC,
                super::super::contract::F64_TYPE_ENVELOPE_DOMAIN,
                &(version, tag),
                MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap();
            assert!(decode_type_object(&malformed, TypeObjectDigest::of(&malformed)).is_err());
        }
        let id = crate::platform::semantic_id::ExpressionId::migrate(b"f64-canonical-owner", 0);
        let mut expression = ExpressionRecord::new(
            id,
            ExpressionOperation::F64 {
                value: Binary64::parse("nan").unwrap(),
            },
        )
        .unwrap();
        let owner = OwnerRecord::Expression(expression.clone());
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], b"LKJOWN27");
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        // Scalar meaning remains readable in each supported scalar-era envelope.
        for generation in [17, 18, 19, 20, 21, 22] {
            expression.contract_version = generation;
            let historical = OwnerRecord::Expression(expression.clone());
            let (digest, bytes) = encode_owner(&historical).unwrap();
            assert_eq!(&bytes[..8], format!("LKJOWN{generation}").as_bytes());
            assert_eq!(
                decode_owner(&bytes, historical.owner(), historical.kind(), digest).unwrap(),
                historical
            );
        }
        for (generation, magic, domain) in [
            (
                15,
                super::super::contract::REQUIREMENT_OWNER_MAGIC,
                super::super::contract::REQUIREMENT_OWNER_ENVELOPE_DOMAIN,
            ),
            (
                16,
                super::super::contract::TRANSACTION_OWNER_MAGIC,
                super::super::contract::TRANSACTION_OWNER_ENVELOPE_DOMAIN,
            ),
        ] {
            expression.contract_version = generation;
            let disguised = OwnerRecord::Expression(expression.clone());
            assert_eq!(
                encode_owner(&disguised).unwrap_err().code,
                "kernel_expression_generation"
            );
            let raw =
                packed::encode(magic, domain, &disguised, MAXIMUM_OWNER_OBJECT_BYTES).unwrap();
            assert_eq!(
                decode_owner(
                    &raw,
                    disguised.owner(),
                    disguised.kind(),
                    OwnerObjectDigest::of(&raw)
                )
                .unwrap_err()
                .code,
                "kernel_expression_generation"
            );
        }
        expression.contract_version = 17;
        expression.operation = ExpressionOperation::F64 {
            value: Binary64::parse("0").unwrap(),
        };
        let positive = encode_owner(&OwnerRecord::Expression(expression.clone())).unwrap();
        expression.operation = ExpressionOperation::F64 {
            value: Binary64::parse("-0").unwrap(),
        };
        let negative = encode_owner(&OwnerRecord::Expression(expression)).unwrap();
        assert_ne!(positive.0, negative.0);
        assert_ne!(positive.1, negative.1);
    }

    #[test]
    fn transaction_outcome_encoding_preserves_ordinary_predecessors_and_rejects_false_generation() {
        use crate::platform::kernel::{
            ExpressionOperation, ExpressionRecord, TransactionOutcomeContract,
        };
        use crate::platform::semantic_id::{BindingId, ExpressionId, RequirementId};
        let id = ExpressionId::migrate(b"transaction-outcome-generation", 0);
        for generation in [14, 15, 16, 17] {
            let mut expression = ExpressionRecord::new(id, ExpressionOperation::Unit {}).unwrap();
            expression.contract_version = generation;
            let owner = OwnerRecord::Expression(expression);
            let (digest, bytes) = encode_owner(&owner).unwrap();
            assert_eq!(&bytes[..8], format!("LKJOWN{generation}").as_bytes());
            assert_eq!(
                decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
                owner
            );
        }
        let contract = TransactionOutcomeContract::standard().unwrap();
        let body_type = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let mut expression = ExpressionRecord::new(
            id,
            ExpressionOperation::TransactionOutcome {
                requirement: super::super::RequirementReference {
                    package: contract.outcome.package,
                    requirement: RequirementId::migrate(b"transaction-outcome-generation", 0),
                }
                .into(),
                binding: BindingId::migrate(b"transaction-outcome-generation", 0),
                body: ExpressionId::migrate(b"transaction-outcome-generation", 1),
                type_argument: body_type,
                outcome: contract,
            },
        )
        .unwrap();
        expression.contract_version = 16;
        let owner = OwnerRecord::Expression(expression.clone());
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], b"LKJOWN16");
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        expression.contract_version = 15;
        let forged_owner = OwnerRecord::Expression(expression);
        assert_eq!(
            encode_owner(&forged_owner).unwrap_err().code,
            "kernel_expression_generation"
        );
        let forged = packed::encode(
            super::super::contract::REQUIREMENT_OWNER_MAGIC,
            super::super::contract::REQUIREMENT_OWNER_ENVELOPE_DOMAIN,
            &forged_owner,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )
        .unwrap();
        let error = decode_owner(
            &forged,
            forged_owner.owner(),
            forged_owner.kind(),
            OwnerObjectDigest::of(&forged),
        )
        .unwrap_err();
        assert_eq!(error.code, "kernel_expression_generation");
    }

    #[test]
    fn requirement_task_envelopes_preserve_predecessor_and_reject_malformed_extensions() {
        use crate::platform::kernel::{
            EffectRow, RequirementOperand, RequirementParameterReference, RequirementReference,
        };
        let package = "pkg_10000000000000000000000000000001".parse().unwrap();
        let result = encode_type_object(&TypeObject::new(TypeForm::Unit).unwrap())
            .unwrap()
            .0;
        let concrete = RequirementOperand::Concrete(RequirementReference {
            package,
            requirement: crate::platform::semantic_id::RequirementId::migrate(
                b"requirement-wire",
                0,
            ),
        });
        let parameter = RequirementOperand::Parameter(RequirementParameterReference {
            package,
            parameter: crate::platform::semantic_id::RequirementParameterId::migrate(
                b"requirement-wire",
                0,
            ),
        });
        for (operand, magic) in [(concrete, b"LKJTFN01"), (parameter, b"LKJTFN02")] {
            let object = TypeObject::new(TypeForm::TaskFunction {
                parameters: vec![],
                result,
                effect: EffectRow {
                    requirements: vec![operand],
                    parameters: vec![],
                },
            })
            .unwrap();
            let (digest, bytes) = encode_type_object(&object).unwrap();
            assert_eq!(&bytes[..8], magic);
            assert_eq!(
                encode_type_object(&decode_type_object(&bytes, digest).unwrap()).unwrap(),
                (digest, bytes.clone())
            );
            if operand == concrete {
                let old: TaskFunctionObject<super::super::wire14::EffectRow14> = packed::decode(
                    &bytes,
                    super::super::contract::TASK_FUNCTION_MAGIC,
                    super::super::contract::TASK_FUNCTION_ENVELOPE_DOMAIN,
                    MAXIMUM_TYPE_OBJECT_BYTES,
                )
                .unwrap();
                assert_eq!(old.contract_version, 1);
                assert_eq!(old.effect.requirements.len(), 1);
            } else {
                let task = TaskFunctionObject {
                    contract_version: 99,
                    tag: 1,
                    parameters: vec![],
                    result,
                    effect: EffectRow {
                        requirements: vec![operand],
                        parameters: vec![],
                    },
                };
                let unsupported = packed::encode(
                    super::super::contract::REQUIREMENT_TASK_FUNCTION_MAGIC,
                    super::super::contract::REQUIREMENT_TASK_FUNCTION_ENVELOPE_DOMAIN,
                    &task,
                    MAXIMUM_TYPE_OBJECT_BYTES,
                )
                .unwrap();
                let error = decode_type_object(&unsupported, TypeObjectDigest::of(&unsupported))
                    .unwrap_err();
                assert_eq!(error.code, "kernel_type_contract");
                let mut malformed = bytes.clone();
                malformed.push(0);
                assert!(decode_type_object(&malformed, TypeObjectDigest::of(&malformed)).is_err());
                let mut old = object.clone();
                old.contract_version = 1;
                assert!(encode_type_object(&old).is_err());
            }
        }
    }

    #[test]
    fn positive_application_has_one_encoding_and_ordered_nominal_identity() {
        let declaration = DeclarationReference {
            package: "pkg_10000000000000000000000000000001".parse().unwrap(),
            declaration: "decl_72b38e6cd864cb4239b329b7e337577f".parse().unwrap(),
        };
        let integer = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let text = encode_type_object(&TypeObject::new(TypeForm::Text).unwrap())
            .unwrap()
            .0;
        let application = TypeObject::new(TypeForm::Applied {
            declaration,
            arguments: vec![integer, text],
        })
        .unwrap();
        let (digest, bytes) = encode_type_object(&application).unwrap();
        assert_eq!(&bytes[..8], b"LKJTAP01");
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), application);
        assert_eq!(
            encode_type_object(&decode_type_object(&bytes, digest).unwrap()).unwrap(),
            (digest, bytes.clone())
        );
        let reversed = TypeObject::new(TypeForm::Applied {
            declaration,
            arguments: vec![text, integer],
        })
        .unwrap();
        assert_ne!(encode_type_object(&reversed).unwrap().0, digest);
        assert!(
            TypeObject::new(TypeForm::Applied {
                declaration,
                arguments: vec![]
            })
            .is_err()
        );
        for tag in [0, 2, 255] {
            let forged = packed::encode(
                super::super::contract::NOMINAL_APPLICATION_MAGIC,
                super::super::contract::NOMINAL_APPLICATION_ENVELOPE_DOMAIN,
                &NominalApplicationObject {
                    contract_version: 1,
                    tag,
                    declaration,
                    arguments: vec![integer],
                },
                MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap();
            let error = decode_type_object(&forged, TypeObjectDigest::of(&forged)).unwrap_err();
            assert_eq!(error.code, "kernel_nominal_application_tag");
        }
        let mixed = packed::encode(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &application,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert!(decode_type_object(&mixed, TypeObjectDigest::of(&mixed)).is_err());
        let empty = packed::encode(
            super::super::contract::NOMINAL_APPLICATION_MAGIC,
            super::super::contract::NOMINAL_APPLICATION_ENVELOPE_DOMAIN,
            &NominalApplicationObject {
                contract_version: 1,
                tag: 1,
                declaration,
                arguments: vec![],
            },
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert!(decode_type_object(&empty, TypeObjectDigest::of(&empty)).is_err());
    }
}

pub fn encode_owner_binding(binding: &OwnerBinding) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(OWNER_BINDING_BYTES);
    bytes.push(binding.kind.tag());
    bytes.extend_from_slice(&binding.object.bytes());
    bytes
}

#[cfg(test)]
mod borrow_encoding_tests {
    use super::*;
    use crate::platform::kernel::{
        BindingKind, BindingRecord, ExpressionOperation, ExpressionRecord, Name, OwnedChoiceArm,
        OwnerHeader, TypeForm,
    };
    use crate::platform::semantic_id::{BindingId, ExpressionId};

    #[test]
    fn lexical_borrow_tags_round_trip_and_rehashed_predecessors_reject() {
        let binding = BindingId::migrate(b"scoped-child-read-codec", 0);
        let expression = ExpressionId::migrate(b"scoped-child-read-codec", 0);
        let source = ExpressionId::migrate(b"scoped-child-read-codec", 1);
        let body = ExpressionId::migrate(b"scoped-child-read-codec", 2);
        let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let records = [
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    expression,
                    ExpressionOperation::BorrowOwnedField {
                        product_type: ty,
                        source,
                        field: Name::new("payload").unwrap(),
                        binding,
                        body,
                    },
                )
                .unwrap(),
            ),
            OwnerRecord::Expression(
                ExpressionRecord::new(
                    expression,
                    ExpressionOperation::MatchBorrowedOwned {
                        choice_type: ty,
                        source,
                        arms: vec![OwnedChoiceArm {
                            name: Name::new("accepted").unwrap(),
                            binding,
                            body,
                        }],
                    },
                )
                .unwrap(),
            ),
            OwnerRecord::Binding(BindingRecord {
                header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
                name: Name::new("view").unwrap(),
                kind: BindingKind::OwnedBorrow,
                value: None,
                declared_type: Some(ty),
            }),
        ];
        for record in records {
            let (digest, bytes) = encode_owner(&record).unwrap();
            assert_eq!(&bytes[..8], b"LKJOWN27");
            assert_eq!(
                decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
                record
            );
            let mut previous = record.clone();
            previous.set_encoding_for_edit(23);
            let code = if matches!(record, OwnerRecord::Binding(_)) {
                "kernel_borrow_binding"
            } else {
                "kernel_borrow_generation"
            };
            assert_eq!(encode_owner(&previous).unwrap_err().code, code);
            // Independent envelope rehash cannot confer predecessor authority.
            let disguised = packed::encode(
                crate::platform::kernel::contract::OWNED_EFFECT_OWNER_MAGIC,
                crate::platform::kernel::contract::OWNED_EFFECT_OWNER_ENVELOPE_DOMAIN,
                &previous,
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            assert_eq!(
                decode_owner(
                    &disguised,
                    previous.owner(),
                    previous.kind(),
                    OwnerObjectDigest::of(&disguised)
                )
                .unwrap_err()
                .code,
                code
            );
        }
    }
}

pub fn decode_owner_binding(
    bytes: &[u8],
    expected_owner: OwnerKey,
) -> Result<OwnerBinding, Diagnostic> {
    if bytes.len() != OWNER_BINDING_BYTES {
        return Err(codec_error(
            "kernel_owner_binding_length",
            "owner binding has a noncanonical byte length",
        ));
    }
    let kind = OwnerKind::ALL
        .into_iter()
        .find(|kind| kind.tag() == bytes[0])
        .ok_or_else(|| {
            codec_error(
                "kernel_owner_binding_kind",
                "owner binding contains an unknown owner-kind tag",
            )
        })?;
    if !kind.accepts_owner(expected_owner) {
        return Err(codec_error(
            "kernel_owner_binding_domain",
            "owner binding kind disagrees with its map-key identity domain",
        ));
    }
    let object = bytes[1..].try_into().map_err(|_| {
        codec_error(
            "kernel_owner_binding_length",
            "owner binding has a noncanonical digest length",
        )
    })?;
    Ok(OwnerBinding {
        kind,
        object: OwnerObjectDigest::from_bytes(object),
    })
}

pub fn encode_dependency_binding(binding: &DependencyBinding) -> Vec<u8> {
    binding.object.bytes().to_vec()
}

pub fn decode_dependency_binding(bytes: &[u8]) -> Result<DependencyBinding, Diagnostic> {
    let object = bytes.try_into().map_err(|_| {
        codec_error(
            "kernel_dependency_binding_length",
            "dependency binding has a noncanonical byte length",
        )
    })?;
    Ok(DependencyBinding {
        object: DependencyObjectDigest::from_bytes(object),
    })
}

pub fn encode_retirement_binding(binding: &RetirementBinding) -> Vec<u8> {
    binding.object.bytes().to_vec()
}

pub fn decode_retirement_binding(bytes: &[u8]) -> Result<RetirementBinding, Diagnostic> {
    let object = bytes.try_into().map_err(|_| {
        codec_error(
            "kernel_retirement_binding_length",
            "retirement binding has a noncanonical byte length",
        )
    })?;
    Ok(RetirementBinding {
        object: RetirementObjectDigest::from_bytes(object),
    })
}

pub fn encode_owner(record: &OwnerRecord) -> Result<(OwnerObjectDigest, Vec<u8>), Diagnostic> {
    record.validate_local()?;
    if record.header().contract_version == super::contract::PREDECESSOR_GRAPH_CONTRACT_VERSION {
        let wire = super::wire14::OwnerRecord14::try_from(record.clone())?;
        let bytes = packed::encode(
            super::contract::PREDECESSOR_OWNER_MAGIC,
            super::contract::PREDECESSOR_OWNER_ENVELOPE_DOMAIN,
            &wire,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        return Ok((OwnerObjectDigest::of(&bytes), bytes));
    }
    let (magic, domain) = if record.header().contract_version
        == super::contract::REQUIREMENT_GRAPH_CONTRACT_VERSION
    {
        (
            super::contract::REQUIREMENT_OWNER_MAGIC,
            super::contract::REQUIREMENT_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version
        == super::contract::TRANSACTION_GRAPH_CONTRACT_VERSION
    {
        (
            super::contract::TRANSACTION_OWNER_MAGIC,
            super::contract::TRANSACTION_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == 17 {
        (
            super::contract::SCALAR_OWNER_MAGIC,
            super::contract::SCALAR_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == 18 {
        (
            super::contract::OWNED_OWNER_MAGIC,
            super::contract::OWNED_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == 19 {
        (
            super::contract::PRODUCT_OWNER_MAGIC,
            super::contract::PRODUCT_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == 20 {
        (
            super::contract::CHOICE_OWNER_MAGIC,
            super::contract::CHOICE_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == 21 {
        (
            super::contract::PARALLEL_OWNER_MAGIC,
            super::contract::PARALLEL_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == super::contract::TRANSFER_GRAPH_CONTRACT_VERSION {
        (
            super::contract::TRANSFER_OWNER_MAGIC,
            super::contract::TRANSFER_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version
        == super::contract::OWNED_EFFECT_GRAPH_CONTRACT_VERSION
    {
        (
            super::contract::OWNED_EFFECT_OWNER_MAGIC,
            super::contract::OWNED_EFFECT_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == super::contract::BORROW_GRAPH_CONTRACT_VERSION {
        (
            super::contract::BORROW_OWNER_MAGIC,
            super::contract::BORROW_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version == super::contract::SEQUENCE_GRAPH_CONTRACT_VERSION {
        (
            super::contract::SEQUENCE_OWNER_MAGIC,
            super::contract::SEQUENCE_OWNER_ENVELOPE_DOMAIN,
        )
    } else if record.header().contract_version
        == super::contract::PARAMETERIZED_CONTRACT_GRAPH_CONTRACT_VERSION
    {
        (
            super::contract::PARAMETERIZED_CONTRACT_OWNER_MAGIC,
            super::contract::PARAMETERIZED_CONTRACT_OWNER_ENVELOPE_DOMAIN,
        )
    } else {
        (OWNER_MAGIC, OWNER_ENVELOPE_DOMAIN)
    };
    let bytes = if record.header().contract_version < 18 {
        packed::encode(
            magic,
            domain,
            &super::wire17::OwnerRecord17::try_from(record.clone())?,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?
    } else if record.header().contract_version
        < super::contract::OWNED_EFFECT_GRAPH_CONTRACT_VERSION
    {
        packed::encode(
            magic,
            domain,
            &super::wire22::OwnerRecord22::try_from(record.clone())?,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?
    } else if record.header().contract_version
        < super::contract::PARAMETERIZED_CONTRACT_GRAPH_CONTRACT_VERSION
    {
        packed::encode(
            magic,
            domain,
            &super::wire25::OwnerRecord25::try_from(record.clone())?,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?
    } else if record.header().contract_version < super::contract::GRAPH_CONTRACT_VERSION {
        packed::encode(
            magic,
            domain,
            &super::wire26::OwnerRecord26::try_from(record.clone())?,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?
    } else {
        packed::encode(magic, domain, record, MAXIMUM_OWNER_OBJECT_BYTES)?
    };
    Ok((OwnerObjectDigest::of(&bytes), bytes))
}

pub fn decode_owner(
    bytes: &[u8],
    expected_owner: OwnerKey,
    expected_kind: OwnerKind,
    expected_digest: OwnerObjectDigest,
) -> Result<OwnerRecord, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        OwnerObjectDigest::of(bytes).bytes(),
        "owner",
    )?;
    let record: OwnerRecord = if bytes.starts_with(&super::contract::PREDECESSOR_OWNER_MAGIC) {
        let wire: super::wire14::OwnerRecord14 = packed::decode(
            bytes,
            super::contract::PREDECESSOR_OWNER_MAGIC,
            super::contract::PREDECESSOR_OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        let record: OwnerRecord = wire.into();
        if record.header().contract_version != super::contract::PREDECESSOR_GRAPH_CONTRACT_VERSION {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "predecessor envelope has a foreign owner generation",
            ));
        }
        record
    } else if bytes.starts_with(&super::contract::REQUIREMENT_OWNER_MAGIC) {
        let wire: super::wire17::OwnerRecord17 = packed::decode(
            bytes,
            super::contract::REQUIREMENT_OWNER_MAGIC,
            super::contract::REQUIREMENT_OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        let record: OwnerRecord = wire.into();
        if record.header().contract_version != super::contract::REQUIREMENT_GRAPH_CONTRACT_VERSION {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "requirement envelope has a foreign owner generation",
            ));
        }
        record
    } else if bytes.starts_with(&super::contract::TRANSACTION_OWNER_MAGIC) {
        let wire: super::wire17::OwnerRecord17 = packed::decode(
            bytes,
            super::contract::TRANSACTION_OWNER_MAGIC,
            super::contract::TRANSACTION_OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        let record: OwnerRecord = wire.into();
        if record.header().contract_version != super::contract::TRANSACTION_GRAPH_CONTRACT_VERSION {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "transaction envelope has a foreign owner generation",
            ));
        }
        record
    } else if bytes.starts_with(&super::contract::SCALAR_OWNER_MAGIC) {
        let wire: super::wire17::OwnerRecord17 = packed::decode(
            bytes,
            super::contract::SCALAR_OWNER_MAGIC,
            super::contract::SCALAR_OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        let record: OwnerRecord = wire.into();
        if record.header().contract_version != 17 {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "scalar envelope has a foreign generation",
            ));
        }
        record
    } else {
        // Graphs 18–22 retain the frozen implementation-call field layout. Local
        // admission still rejects later operation/binding and constraint tags.
        let (magic, domain, generation) = if bytes.starts_with(&super::contract::OWNED_OWNER_MAGIC)
        {
            (
                super::contract::OWNED_OWNER_MAGIC,
                super::contract::OWNED_OWNER_ENVELOPE_DOMAIN,
                18,
            )
        } else if bytes.starts_with(&super::contract::PRODUCT_OWNER_MAGIC) {
            (
                super::contract::PRODUCT_OWNER_MAGIC,
                super::contract::PRODUCT_OWNER_ENVELOPE_DOMAIN,
                19,
            )
        } else if bytes.starts_with(&super::contract::CHOICE_OWNER_MAGIC) {
            (
                super::contract::CHOICE_OWNER_MAGIC,
                super::contract::CHOICE_OWNER_ENVELOPE_DOMAIN,
                20,
            )
        } else if bytes.starts_with(&super::contract::PARALLEL_OWNER_MAGIC) {
            (
                super::contract::PARALLEL_OWNER_MAGIC,
                super::contract::PARALLEL_OWNER_ENVELOPE_DOMAIN,
                21,
            )
        } else if bytes.starts_with(&super::contract::TRANSFER_OWNER_MAGIC) {
            (
                super::contract::TRANSFER_OWNER_MAGIC,
                super::contract::TRANSFER_OWNER_ENVELOPE_DOMAIN,
                super::contract::TRANSFER_GRAPH_CONTRACT_VERSION,
            )
        } else if bytes.starts_with(&super::contract::OWNED_EFFECT_OWNER_MAGIC) {
            (
                super::contract::OWNED_EFFECT_OWNER_MAGIC,
                super::contract::OWNED_EFFECT_OWNER_ENVELOPE_DOMAIN,
                super::contract::OWNED_EFFECT_GRAPH_CONTRACT_VERSION,
            )
        } else if bytes.starts_with(&super::contract::BORROW_OWNER_MAGIC) {
            (
                super::contract::BORROW_OWNER_MAGIC,
                super::contract::BORROW_OWNER_ENVELOPE_DOMAIN,
                super::contract::BORROW_GRAPH_CONTRACT_VERSION,
            )
        } else if bytes.starts_with(&super::contract::SEQUENCE_OWNER_MAGIC) {
            (
                super::contract::SEQUENCE_OWNER_MAGIC,
                super::contract::SEQUENCE_OWNER_ENVELOPE_DOMAIN,
                super::contract::SEQUENCE_GRAPH_CONTRACT_VERSION,
            )
        } else if bytes.starts_with(&super::contract::PARAMETERIZED_CONTRACT_OWNER_MAGIC) {
            (
                super::contract::PARAMETERIZED_CONTRACT_OWNER_MAGIC,
                super::contract::PARAMETERIZED_CONTRACT_OWNER_ENVELOPE_DOMAIN,
                super::contract::PARAMETERIZED_CONTRACT_GRAPH_CONTRACT_VERSION,
            )
        } else {
            (
                OWNER_MAGIC,
                OWNER_ENVELOPE_DOMAIN,
                super::contract::GRAPH_CONTRACT_VERSION,
            )
        };
        let record: OwnerRecord =
            if generation < super::contract::OWNED_EFFECT_GRAPH_CONTRACT_VERSION {
                let wire: super::wire22::OwnerRecord22 =
                    packed::decode(bytes, magic, domain, MAXIMUM_OWNER_OBJECT_BYTES)?;
                wire.into()
            } else if generation < super::contract::PARAMETERIZED_CONTRACT_GRAPH_CONTRACT_VERSION {
                let wire: super::wire25::OwnerRecord25 =
                    packed::decode(bytes, magic, domain, MAXIMUM_OWNER_OBJECT_BYTES)?;
                wire.into()
            } else if generation < super::contract::GRAPH_CONTRACT_VERSION {
                let wire: super::wire26::OwnerRecord26 =
                    packed::decode(bytes, magic, domain, MAXIMUM_OWNER_OBJECT_BYTES)?;
                wire.into()
            } else {
                packed::decode(bytes, magic, domain, MAXIMUM_OWNER_OBJECT_BYTES)?
            };
        if record.header().contract_version != generation {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "owner envelope has a foreign generation",
            ));
        }
        record
    };
    record.validate_local()?;
    if record.owner() != expected_owner || record.kind() != expected_kind {
        return Err(codec_error(
            "kernel_owner_key_mismatch",
            "owner map key or binding kind does not match the decoded owner header",
        ));
    }
    let (digest, canonical) = encode_owner(&record)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "owner",
    )?;
    Ok(record)
}

pub fn encode_type_object(object: &TypeObject) -> Result<(TypeObjectDigest, Vec<u8>), Diagnostic> {
    object.validate_local()?;
    if let super::TypeForm::OwnedSequence { item } = &object.form {
        let bytes = packed::encode(
            super::contract::OWNED_SEQUENCE_TYPE_MAGIC,
            super::contract::OWNED_SEQUENCE_TYPE_ENVELOPE_DOMAIN,
            &(object.contract_version, 1_u8, item),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if let super::TypeForm::OwnedChoice { cases } = &object.form {
        let bytes = packed::encode(
            super::contract::OWNED_CHOICE_TYPE_MAGIC,
            super::contract::OWNED_CHOICE_TYPE_ENVELOPE_DOMAIN,
            &(object.contract_version, 1_u8, cases),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if let super::TypeForm::OwnedProduct { fields } = &object.form {
        let bytes = packed::encode(
            super::contract::OWNED_PRODUCT_TYPE_MAGIC,
            super::contract::OWNED_PRODUCT_TYPE_ENVELOPE_DOMAIN,
            &(object.contract_version, 1_u8, fields),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if matches!(object.form, super::TypeForm::OwnedI64Cell) {
        let bytes = packed::encode(
            super::contract::OWNED_CELL_TYPE_MAGIC,
            super::contract::OWNED_CELL_TYPE_ENVELOPE_DOMAIN,
            &(object.contract_version, 1_u8),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if matches!(object.form, super::TypeForm::ByteBuffer) {
        let bytes = packed::encode(
            super::contract::BYTE_BUFFER_TYPE_MAGIC,
            super::contract::BYTE_BUFFER_TYPE_ENVELOPE_DOMAIN,
            &(object.contract_version, 1_u8),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if matches!(object.form, super::TypeForm::F64) {
        let bytes = packed::encode(
            super::contract::F64_TYPE_MAGIC,
            super::contract::F64_TYPE_ENVELOPE_DOMAIN,
            &(object.contract_version, 1_u8),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if let super::TypeForm::TaskFunction {
        parameters,
        result,
        effect,
    } = &object.form
    {
        let bytes = if object.contract_version == 1 {
            packed::encode(
                super::contract::TASK_FUNCTION_MAGIC,
                super::contract::TASK_FUNCTION_ENVELOPE_DOMAIN,
                &TaskFunctionObject {
                    contract_version: object.contract_version,
                    tag: 1,
                    parameters: parameters.clone(),
                    result: *result,
                    effect: super::wire14::EffectRow14::try_from(effect.clone())?,
                },
                MAXIMUM_TYPE_OBJECT_BYTES,
            )?
        } else {
            packed::encode(
                super::contract::REQUIREMENT_TASK_FUNCTION_MAGIC,
                super::contract::REQUIREMENT_TASK_FUNCTION_ENVELOPE_DOMAIN,
                &TaskFunctionObject {
                    contract_version: object.contract_version,
                    tag: 1,
                    parameters: parameters.clone(),
                    result: *result,
                    effect: effect.clone(),
                },
                MAXIMUM_TYPE_OBJECT_BYTES,
            )?
        };
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    if let super::type_object::TypeForm::Applied {
        declaration,
        arguments,
    } = &object.form
    {
        let bytes = packed::encode(
            super::contract::NOMINAL_APPLICATION_MAGIC,
            super::contract::NOMINAL_APPLICATION_ENVELOPE_DOMAIN,
            &NominalApplicationObject {
                contract_version: object.contract_version,
                tag: 1,
                declaration: *declaration,
                arguments: arguments.clone(),
            },
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        return Ok((TypeObjectDigest::of(&bytes), bytes));
    }
    let bytes = packed::encode(
        TYPE_OBJECT_MAGIC,
        TYPE_OBJECT_ENVELOPE_DOMAIN,
        object,
        MAXIMUM_TYPE_OBJECT_BYTES,
    )?;
    Ok((TypeObjectDigest::of(&bytes), bytes))
}

#[derive(bincode::Encode, bincode::Decode)]
struct NominalApplicationObject {
    contract_version: u16,
    tag: u8,
    declaration: super::reference::DeclarationReference,
    arguments: Vec<TypeObjectDigest>,
}

#[derive(bincode::Encode, bincode::Decode)]
struct TaskFunctionObject<Row> {
    contract_version: u16,
    tag: u8,
    parameters: Vec<TypeObjectDigest>,
    result: TypeObjectDigest,
    effect: Row,
}

pub fn decode_type_object(
    bytes: &[u8],
    expected_digest: TypeObjectDigest,
) -> Result<TypeObject, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        TypeObjectDigest::of(bytes).bytes(),
        "type",
    )?;
    if bytes.starts_with(&super::contract::OWNED_SEQUENCE_TYPE_MAGIC) {
        let (contract_version, tag, item): (u16, u8, TypeObjectDigest) = packed::decode(
            bytes,
            super::contract::OWNED_SEQUENCE_TYPE_MAGIC,
            super::contract::OWNED_SEQUENCE_TYPE_ENVELOPE_DOMAIN,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        if tag != 1 {
            return Err(codec_error(
                "kernel_sequence_type_tag",
                "unknown OwnedSequence type tag",
            ));
        }
        let object = TypeObject {
            contract_version,
            form: super::TypeForm::OwnedSequence { item },
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::OWNED_CHOICE_TYPE_MAGIC) {
        let (contract_version, tag, cases): (u16, u8, Vec<super::StructuralTypeField>) =
            packed::decode(
                bytes,
                super::contract::OWNED_CHOICE_TYPE_MAGIC,
                super::contract::OWNED_CHOICE_TYPE_ENVELOPE_DOMAIN,
                MAXIMUM_TYPE_OBJECT_BYTES,
            )?;
        if tag != 1 {
            return Err(codec_error(
                "kernel_choice_type_tag",
                "unknown OwnedChoice type tag",
            ));
        }
        let object = TypeObject {
            contract_version,
            form: super::TypeForm::OwnedChoice { cases },
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::OWNED_PRODUCT_TYPE_MAGIC) {
        let (contract_version, tag, fields): (u16, u8, Vec<super::StructuralTypeField>) =
            packed::decode(
                bytes,
                super::contract::OWNED_PRODUCT_TYPE_MAGIC,
                super::contract::OWNED_PRODUCT_TYPE_ENVELOPE_DOMAIN,
                MAXIMUM_TYPE_OBJECT_BYTES,
            )?;
        if tag != 1 {
            return Err(codec_error(
                "kernel_product_type_tag",
                "unknown OwnedProduct type tag",
            ));
        }
        let object = TypeObject {
            contract_version,
            form: super::TypeForm::OwnedProduct { fields },
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::OWNED_CELL_TYPE_MAGIC) {
        let (contract_version, tag): (u16, u8) = packed::decode(
            bytes,
            super::contract::OWNED_CELL_TYPE_MAGIC,
            super::contract::OWNED_CELL_TYPE_ENVELOPE_DOMAIN,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        if tag != 1 {
            return Err(codec_error(
                "kernel_cell_type_tag",
                "unknown OwnedI64Cell type tag",
            ));
        }
        let object = TypeObject {
            contract_version,
            form: super::TypeForm::OwnedI64Cell,
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::BYTE_BUFFER_TYPE_MAGIC) {
        let (contract_version, tag): (u16, u8) = packed::decode(
            bytes,
            super::contract::BYTE_BUFFER_TYPE_MAGIC,
            super::contract::BYTE_BUFFER_TYPE_ENVELOPE_DOMAIN,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        if tag != 1 {
            return Err(codec_error(
                "kernel_buffer_type_tag",
                "unknown ByteBuffer type tag",
            ));
        }
        let object = TypeObject {
            contract_version,
            form: super::TypeForm::ByteBuffer,
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::F64_TYPE_MAGIC) {
        let (contract_version, tag): (u16, u8) = packed::decode(
            bytes,
            super::contract::F64_TYPE_MAGIC,
            super::contract::F64_TYPE_ENVELOPE_DOMAIN,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        if tag != 1 {
            return Err(codec_error("kernel_f64_type_tag", "unknown F64 type tag"));
        }
        let object = TypeObject {
            contract_version,
            form: super::TypeForm::F64,
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::TASK_FUNCTION_MAGIC)
        || bytes.starts_with(&super::contract::REQUIREMENT_TASK_FUNCTION_MAGIC)
    {
        let task: TaskFunctionObject<super::EffectRow> =
            if bytes.starts_with(&super::contract::TASK_FUNCTION_MAGIC) {
                let old: TaskFunctionObject<super::wire14::EffectRow14> = packed::decode(
                    bytes,
                    super::contract::TASK_FUNCTION_MAGIC,
                    super::contract::TASK_FUNCTION_ENVELOPE_DOMAIN,
                    MAXIMUM_TYPE_OBJECT_BYTES,
                )?;
                TaskFunctionObject {
                    contract_version: old.contract_version,
                    tag: old.tag,
                    parameters: old.parameters,
                    result: old.result,
                    effect: old.effect.into(),
                }
            } else {
                packed::decode(
                    bytes,
                    super::contract::REQUIREMENT_TASK_FUNCTION_MAGIC,
                    super::contract::REQUIREMENT_TASK_FUNCTION_ENVELOPE_DOMAIN,
                    MAXIMUM_TYPE_OBJECT_BYTES,
                )?
            };
        if task.tag != 1 {
            return Err(codec_error(
                "kernel_task_function_tag",
                "unknown task callable tag",
            ));
        }
        let object = TypeObject {
            contract_version: task.contract_version,
            form: super::TypeForm::TaskFunction {
                parameters: task.parameters,
                result: task.result,
                effect: task.effect,
            },
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    if bytes.starts_with(&super::contract::NOMINAL_APPLICATION_MAGIC) {
        let application: NominalApplicationObject = packed::decode(
            bytes,
            super::contract::NOMINAL_APPLICATION_MAGIC,
            super::contract::NOMINAL_APPLICATION_ENVELOPE_DOMAIN,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )?;
        if application.tag != 1 {
            return Err(codec_error(
                "kernel_nominal_application_tag",
                "unknown nominal application tag",
            ));
        }
        let object = TypeObject {
            contract_version: application.contract_version,
            form: super::type_object::TypeForm::Applied {
                declaration: application.declaration,
                arguments: application.arguments,
            },
        };
        let (digest, canonical) = encode_type_object(&object)?;
        verify_canonical(
            bytes,
            &canonical,
            digest.bytes(),
            expected_digest.bytes(),
            "type",
        )?;
        return Ok(object);
    }
    let object: TypeObject = packed::decode(
        bytes,
        TYPE_OBJECT_MAGIC,
        TYPE_OBJECT_ENVELOPE_DOMAIN,
        MAXIMUM_TYPE_OBJECT_BYTES,
    )?;
    object.validate_local()?;
    let (digest, canonical) = encode_type_object(&object)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "type",
    )?;
    Ok(object)
}

pub fn encode_root(root: &SemanticRoot) -> Result<(SemanticRootDigest, Vec<u8>), Diagnostic> {
    root.validate_local()?;
    let bytes = packed::encode(ROOT_MAGIC, ROOT_ENVELOPE_DOMAIN, root, MAXIMUM_ROOT_BYTES)?;
    Ok((SemanticRootDigest::of(&bytes), bytes))
}

pub fn decode_root(
    bytes: &[u8],
    expected_digest: SemanticRootDigest,
) -> Result<SemanticRoot, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        SemanticRootDigest::of(bytes).bytes(),
        "root",
    )?;
    let root: SemanticRoot =
        packed::decode(bytes, ROOT_MAGIC, ROOT_ENVELOPE_DOMAIN, MAXIMUM_ROOT_BYTES)?;
    root.validate_local()?;
    let (digest, canonical) = encode_root(&root)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "root",
    )?;
    Ok(root)
}

pub fn encode_dependency(
    dependency: &DependencyRecord,
) -> Result<(DependencyObjectDigest, Vec<u8>), Diagnostic> {
    dependency.validate_local()?;
    let bytes = packed::encode(
        DEPENDENCY_MAGIC,
        DEPENDENCY_ENVELOPE_DOMAIN,
        dependency,
        MAXIMUM_DEPENDENCY_BYTES,
    )?;
    Ok((DependencyObjectDigest::of(&bytes), bytes))
}

pub fn decode_dependency(
    bytes: &[u8],
    expected_package: &PackageId,
    expected_digest: DependencyObjectDigest,
) -> Result<DependencyRecord, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        DependencyObjectDigest::of(bytes).bytes(),
        "dependency",
    )?;
    let dependency: DependencyRecord = packed::decode(
        bytes,
        DEPENDENCY_MAGIC,
        DEPENDENCY_ENVELOPE_DOMAIN,
        MAXIMUM_DEPENDENCY_BYTES,
    )?;
    dependency.validate_local()?;
    if &dependency.package != expected_package {
        return Err(codec_error(
            "kernel_dependency_key_mismatch",
            "dependency map key does not match the dependency record",
        ));
    }
    let (digest, canonical) = encode_dependency(&dependency)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "dependency",
    )?;
    Ok(dependency)
}

pub fn encode_retirement(
    retirement: &RetirementRecord,
) -> Result<(RetirementObjectDigest, Vec<u8>), Diagnostic> {
    retirement.validate_local()?;
    let bytes = packed::encode(
        RETIREMENT_MAGIC,
        RETIREMENT_ENVELOPE_DOMAIN,
        retirement,
        MAXIMUM_RETIREMENT_BYTES,
    )?;
    Ok((RetirementObjectDigest::of(&bytes), bytes))
}

pub fn decode_retirement(
    bytes: &[u8],
    expected_owner: OwnerKey,
    expected_digest: RetirementObjectDigest,
) -> Result<RetirementRecord, Diagnostic> {
    verify_digest(
        expected_digest.bytes(),
        RetirementObjectDigest::of(bytes).bytes(),
        "retirement",
    )?;
    let retirement: RetirementRecord = packed::decode(
        bytes,
        RETIREMENT_MAGIC,
        RETIREMENT_ENVELOPE_DOMAIN,
        MAXIMUM_RETIREMENT_BYTES,
    )?;
    retirement.validate_local()?;
    if retirement.owner != expected_owner {
        return Err(codec_error(
            "kernel_retirement_key_mismatch",
            "retirement map key does not match the retirement record",
        ));
    }
    let (digest, canonical) = encode_retirement(&retirement)?;
    verify_canonical(
        bytes,
        &canonical,
        digest.bytes(),
        expected_digest.bytes(),
        "retirement",
    )?;
    Ok(retirement)
}

fn verify_digest(expected: [u8; 32], actual: [u8; 32], label: &str) -> Result<(), Diagnostic> {
    if expected != actual {
        return Err(codec_error(
            "kernel_object_digest",
            format!("{label} object digest does not match its requested identity"),
        ));
    }
    Ok(())
}

fn verify_canonical(
    input: &[u8],
    canonical: &[u8],
    canonical_digest: [u8; 32],
    expected_digest: [u8; 32],
    label: &str,
) -> Result<(), Diagnostic> {
    if input != canonical || canonical_digest != expected_digest {
        return Err(codec_error(
            "kernel_noncanonical_encoding",
            format!("{label} object does not use the canonical Graph 10 encoding"),
        ));
    }
    Ok(())
}

fn codec_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

#[cfg(test)]
mod buffer_encoding_tests {
    use super::*;
    use crate::platform::kernel::TypeForm;
    #[test]
    fn byte_buffer_encoding_is_disjoint_and_rejects_disguised_or_unknown_tags() {
        let buffer = TypeObject::new(TypeForm::ByteBuffer).unwrap();
        let (digest, bytes) = encode_type_object(&buffer).unwrap();
        assert_eq!(&bytes[..8], b"LKJBUF01");
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), buffer);
        let disguised = packed::encode(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &buffer,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert!(decode_type_object(&disguised, TypeObjectDigest::of(&disguised)).is_err());
        for (version, tag) in [(0_u16, 1_u8), (10, 1), (1, 0), (1, 2)] {
            let raw = packed::encode(
                super::super::contract::BYTE_BUFFER_TYPE_MAGIC,
                super::super::contract::BYTE_BUFFER_TYPE_ENVELOPE_DOMAIN,
                &(version, tag),
                MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap();
            assert!(decode_type_object(&raw, TypeObjectDigest::of(&raw)).is_err());
        }
    }
}
