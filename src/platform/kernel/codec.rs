//! Current schema-bound canonical codecs and hostile-input admission.
//! The packed reader proves canonical payload bytes; these owners additionally
//! bind the raw object digest, identity, kind, and local semantic invariants.

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
mod read_view_encoding_tests {
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::*;

    fn coordinates() -> (DeclarationReference, ReadScopeParameter, ReadScopeArgument) {
        let seed = b"read-view-canonical-inventory";
        let declaration = DeclarationReference {
            package: PackageId::migrate(seed, 0),
            declaration: DeclarationId::migrate(seed, 0),
        };
        let parameter = ReadScopeParameter {
            id: ReadScopeParameterId::migrate(seed, 0),
            name: Name::new("source_scope").unwrap(),
        };
        let argument = ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(declaration),
            parameter: parameter.id,
        });
        (declaration, parameter, argument)
    }

    #[test]
    fn current_type_profile_has_one_flat_canonical_encoding() {
        let unit = TypeObject::new(TypeForm::Unit).unwrap();
        let (digest, bytes) = encode_type_object(&unit).unwrap();
        let literal = packed::encode_canonical(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &(super::super::contract::TYPE_OBJECT_CONTRACT_VERSION, 0_u32),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert_eq!(bytes, literal);
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), unit);
        let invalid = TypeObject {
            contract_version: 11,
            form: TypeForm::Unit,
        };
        assert!(encode_type_object(&invalid).is_err());
        let raw = packed::encode_canonical(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &invalid,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert_eq!(
            decode_type_object(&raw, TypeObjectDigest::of(&raw))
                .unwrap_err()
                .code,
            "kernel_type_contract"
        );
        let unknown = packed::encode_canonical(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &(
                super::super::contract::TYPE_OBJECT_CONTRACT_VERSION,
                u32::MAX,
            ),
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert!(decode_type_object(&unknown, TypeObjectDigest::of(&unknown)).is_err());
    }

    #[test]
    fn complete_view_field_and_scope_inventories_are_canonical() {
        let (declaration, scope_parameter, scope) = coordinates();
        let ty = encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
            .unwrap()
            .0;
        let seed = b"read-view-canonical-inventory";
        let mut owner = OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration.declaration),
                OwnerKind::ReadView,
            ),
            module: ModuleId::migrate(seed, 0),
            name: Name::new("RangeRead").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                type_parameters: Vec::new(),
                scope_parameter,
                fields: vec![ReadViewField {
                    id: FieldId::migrate(seed, 0),
                    name: Name::new("payload").unwrap(),
                    ty,
                    kind: ReadViewFieldKind::ReadReference { read_scope: scope },
                }],
            }),
        });
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], &OWNER_MAGIC);
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        let OwnerRecord::Declaration(record) = &mut owner else {
            unreachable!()
        };
        let DeclarationPayload::ReadView(view) = &mut record.payload else {
            unreachable!()
        };
        view.fields.clear();
        assert_ne!(encode_owner(&owner).unwrap().0, digest);
        owner.set_encoding_for_edit(33);
        assert!(encode_owner(&owner).is_err());
    }

    #[test]
    fn read_view_applications_have_disjoint_scope_sensitive_type_bytes() {
        let (declaration, _, read_scope) = coordinates();
        let object = TypeObject::new(TypeForm::ReadViewApplied {
            declaration,
            arguments: Vec::new(),
            read_scope,
        })
        .unwrap();
        let (digest, bytes) = encode_type_object(&object).unwrap();
        assert_eq!(&bytes[..8], &TYPE_OBJECT_MAGIC);
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), object);
        let changed = TypeObject::new(TypeForm::ReadViewApplied {
            declaration,
            arguments: Vec::new(),
            read_scope: ReadScopeArgument::Lexical {
                callable: declaration,
                binding: BindingId::migrate(b"view-other-scope", 0),
            },
        })
        .unwrap();
        assert_ne!(encode_type_object(&changed).unwrap().0, digest);
    }

    #[test]
    fn view_result_keeps_exact_source_scope_relationship() {
        let (declaration, scope_parameter, scope) = coordinates();
        let source = ParameterId::migrate(b"read-view-canonical-inventory", 0);
        let result = encode_type_object(
            &TypeObject::new(TypeForm::ReadViewApplied {
                declaration,
                arguments: Vec::new(),
                read_scope: scope,
            })
            .unwrap(),
        )
        .unwrap()
        .0;
        let mut owner = OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration.declaration),
                OwnerKind::PureFunction,
            ),
            module: ModuleId::migrate(b"read-view-function", 0),
            name: Name::new("range").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                implementation_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: vec![source],
                result: CallableResult {
                    ty: result,
                    relation: ResultRelation::ViewFrom { source, scope },
                },
                read_scope_parameters: vec![scope_parameter],
                read_scope_sources: vec![ReadScopeSource { source, scope }],
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(b"read-view-function", 0),
            }),
        });
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
        let OwnerRecord::Declaration(record) = &mut owner else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut record.payload else {
            unreachable!()
        };
        function.read_scope_sources.clear();
        assert_ne!(encode_owner(&owner).unwrap().0, digest);
    }
}

#[cfg(test)]
#[path = "codec_owned_scheme_tests.rs"]
mod owned_scheme_encoding_tests;

#[cfg(test)]
#[path = "codec_generalized_sequence_tests.rs"]
mod generalized_sequence_encoding_tests;

#[cfg(test)]
#[path = "codec_nominal_owned_record_tests.rs"]
mod nominal_owned_record_encoding_tests;

#[cfg(test)]
mod shareable_encoding_tests {
    use super::*;
    use crate::platform::kernel::{
        Name, OwnerHeader, TypeParameterConstraints, TypeParameterRecord,
    };
    use crate::platform::semantic_id::{DeclarationId, TypeParameterId};

    #[test]
    fn shareable_constraints_round_trip_and_foreign_headers_reject() {
        for constraints in [
            TypeParameterConstraints::OwnedShareable,
            TypeParameterConstraints::OwnedTransferableShareable,
        ] {
            let parameter = TypeParameterId::migrate(b"shareable-owner-generation", 0);
            let mut record = OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: DeclarationId::migrate(b"shareable-owner-generation", 0),
                name: Name::new("Storage").unwrap(),
                constraints,
            });
            let (digest, bytes) = encode_owner(&record).unwrap();
            assert_eq!(&bytes[..8], &OWNER_MAGIC);
            assert_eq!(
                decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
                record
            );
            record.set_encoding_for_edit(0);
            assert_eq!(
                encode_owner(&record).unwrap_err().code,
                "kernel_owner_header"
            );
            let forged = packed::encode_canonical(
                OWNER_MAGIC,
                OWNER_ENVELOPE_DOMAIN,
                &record,
                MAXIMUM_OWNER_OBJECT_BYTES,
            )
            .unwrap();
            assert_eq!(
                decode_owner(
                    &forged,
                    record.owner(),
                    record.kind(),
                    OwnerObjectDigest::of(&forged)
                )
                .unwrap_err()
                .code,
                "kernel_owner_header"
            );
        }
    }
}

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
                result: crate::platform::kernel::CallableResult {
                    ty: encode_type_object(&TypeObject::new(TypeForm::ByteBuffer).unwrap())
                        .unwrap()
                        .0,
                    relation: crate::platform::kernel::ResultRelation::ReadFrom {
                        source: ParameterId::migrate(seed, 0),
                    },
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: FunctionEffect::Pure,
                body: ExpressionId::migrate(seed, 0),
            }),
        })
    }

    #[test]
    fn canonical_result_relation_preserves_exact_source_identity() {
        let original = function();
        let (digest, bytes) = encode_owner(&original).unwrap();
        assert_eq!(&bytes[..8], &OWNER_MAGIC);
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
        f.result.relation = crate::platform::kernel::ResultRelation::ReadFrom {
            source: f.parameters[1],
        };
        assert_ne!(encode_owner(&alternate).unwrap().0, digest);
        let OwnerRecord::Declaration(d) = &mut alternate else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result.relation = crate::platform::kernel::ResultRelation::Value;
        assert_ne!(encode_owner(&alternate).unwrap().0, digest);
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
        f.result.relation = crate::platform::kernel::ResultRelation::ReadFrom {
            source: ParameterId::migrate(b"foreign-source", 0),
        };
        let bytes = packed::encode_canonical(
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
    fn method_result_source_position_is_canonical() {
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
                    result: crate::platform::kernel::CallableResult {
                        ty,
                        relation: crate::platform::kernel::ResultRelation::ReadFrom { source: 0 },
                    },
                    read_scope_parameters: Vec::new(),
                    read_scope_sources: Vec::new(),
                    effect: FunctionEffect::Pure,
                }],
                associated_types: Vec::new(),
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
        c.methods[0].result.relation =
            crate::platform::kernel::ResultRelation::ReadFrom { source: 1 };
        assert_ne!(encode_owner(&record).unwrap().0, digest);
    }

    #[test]
    fn borrow_call_has_ordered_canonical_children() {
        let seed = b"borrow-call-wire";
        let call = ExpressionId::migrate(seed, 1);
        let body = ExpressionId::migrate(seed, 2);
        let expression = ExpressionRecord::new(
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
    }
}

#[cfg(test)]
mod parameterized_contract_encoding_tests {
    use super::*;
    use crate::platform::kernel::*;
    use crate::platform::semantic_id::*;

    #[test]
    fn new_contract_parameter_fields_require_the_current_envelope() {
        let seed = b"current-contract-applications";
        let declaration = DeclarationId::migrate(seed, 0);
        let owner = OwnerKey::Declaration(declaration);
        let scalar = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let record = OwnerRecord::Declaration(DeclarationRecord {
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
                    result: crate::platform::kernel::CallableResult {
                        ty: scalar,
                        relation: crate::platform::kernel::ResultRelation::Value,
                    },
                    read_scope_parameters: Vec::new(),
                    read_scope_sources: Vec::new(),
                    effect: FunctionEffect::Pure,
                }],
                associated_types: Vec::new(),
            }),
        });
        let (digest, bytes) = encode_owner(&record).unwrap();
        assert_eq!(&bytes[..8], &OWNER_MAGIC);
        assert_eq!(
            decode_owner(&bytes, owner, record.kind(), digest).unwrap(),
            record
        );
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
                implementation_parameters: Vec::new(),
                type_parameters: Vec::new(),
                contract: reference,
                self_type,
                type_arguments: vec![argument, self_type],
                methods: vec![OwnedMethodImplementation {
                    read_scope_arguments: Vec::new(),
                    implementations: Vec::new(),
                    type_arguments: Vec::new(),
                    method: MethodId::migrate(seed, 0),
                    function: reference,
                }],
                associated_types: Vec::new(),
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
                result: crate::platform::kernel::CallableResult {
                    ty: result,
                    relation: crate::platform::kernel::ResultRelation::Value,
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
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

    #[test]
    fn implementation_effect_operands_round_trip() {
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
                    read_scope_arguments: Vec::new(),
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
            assert_eq!(&bytes[..8], &OWNER_MAGIC);
            assert_eq!(
                decode_owner(&bytes, current.owner(), current.kind(), digest).unwrap(),
                current
            );
        }
    }
}

#[cfg(test)]
mod nominal_encoding_tests {
    use super::*;
    use crate::platform::kernel::{DeclarationReference, TypeForm};

    #[test]
    fn transfer_constraint_owners_round_trip() {
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
            let record = OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(key, OwnerKind::TypeParameter),
                declaration: DeclarationId::migrate(b"transfer-constraint-codec", 0),
                name: Name::new("T").unwrap(),
                constraints: constraint,
            });
            let (digest, bytes) = encode_owner(&record).unwrap();
            assert_eq!(&bytes[..8], &OWNER_MAGIC);
            assert_eq!(
                decode_owner(&bytes, key, OwnerKind::TypeParameter, digest).unwrap(),
                record
            );
        }
    }

    #[test]
    fn f64_type_and_literal_preserve_distinct_canonical_values() {
        use crate::platform::binary64::Binary64;
        use crate::platform::kernel::{ExpressionOperation, ExpressionRecord};
        let object = TypeObject::new(TypeForm::F64).unwrap();
        let (digest, bytes) = encode_type_object(&object).unwrap();
        assert_eq!(&bytes[..8], &TYPE_OBJECT_MAGIC);
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), object);
        assert_ne!(
            digest,
            encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                .unwrap()
                .0
        );
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
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
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
    fn transaction_outcome_contract_round_trips() {
        use crate::platform::kernel::{
            ExpressionOperation, ExpressionRecord, TransactionOutcomeContract,
        };
        use crate::platform::semantic_id::{BindingId, ExpressionId, RequirementId};
        let id = ExpressionId::migrate(b"transaction-outcome-generation", 0);
        let contract = TransactionOutcomeContract::standard().unwrap();
        let body_type = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
            .unwrap()
            .0;
        let expression = ExpressionRecord::new(
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
        let owner = OwnerRecord::Expression(expression.clone());
        let (digest, bytes) = encode_owner(&owner).unwrap();
        assert_eq!(&bytes[..8], &OWNER_MAGIC);
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
    }

    #[test]
    fn task_type_preserves_requirement_operand_authority() {
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
        for operand in [concrete, parameter] {
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
            assert_eq!(&bytes[..8], &TYPE_OBJECT_MAGIC);
            assert_eq!(
                encode_type_object(&decode_type_object(&bytes, digest).unwrap()).unwrap(),
                (digest, bytes.clone())
            );
            let mut invalid = object.clone();
            invalid.contract_version = 0;
            assert!(encode_type_object(&invalid).is_err());
            let malformed = packed::encode_canonical(
                TYPE_OBJECT_MAGIC,
                TYPE_OBJECT_ENVELOPE_DOMAIN,
                &invalid,
                MAXIMUM_TYPE_OBJECT_BYTES,
            )
            .unwrap();
            assert_eq!(
                decode_type_object(&malformed, TypeObjectDigest::of(&malformed))
                    .unwrap_err()
                    .code,
                "kernel_type_contract"
            );
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
        assert_eq!(&bytes[..8], &TYPE_OBJECT_MAGIC);
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
        let invalid = TypeObject {
            contract_version: super::super::contract::TYPE_OBJECT_CONTRACT_VERSION,
            form: TypeForm::Applied {
                declaration,
                arguments: vec![],
            },
        };
        let empty = packed::encode_canonical(
            TYPE_OBJECT_MAGIC,
            TYPE_OBJECT_ENVELOPE_DOMAIN,
            &invalid,
            MAXIMUM_TYPE_OBJECT_BYTES,
        )
        .unwrap();
        assert_eq!(
            decode_type_object(&empty, TypeObjectDigest::of(&empty))
                .unwrap_err()
                .code,
            "kernel_type_child_count"
        );
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
    fn lexical_borrow_records_round_trip() {
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
            assert_eq!(&bytes[..8], &OWNER_MAGIC);
            assert_eq!(
                decode_owner(&bytes, record.owner(), record.kind(), digest).unwrap(),
                record
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
    let bytes = packed::encode_canonical(
        OWNER_MAGIC,
        OWNER_ENVELOPE_DOMAIN,
        record,
        MAXIMUM_OWNER_OBJECT_BYTES,
    )?;
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
    let record: OwnerRecord = packed::decode_canonical(
        bytes,
        OWNER_MAGIC,
        OWNER_ENVELOPE_DOMAIN,
        MAXIMUM_OWNER_OBJECT_BYTES,
    )?;
    record.validate_local()?;
    if record.owner() != expected_owner || record.kind() != expected_kind {
        return Err(codec_error(
            "kernel_owner_key_mismatch",
            "owner map key or binding kind does not match the decoded owner header",
        ));
    }
    Ok(record)
}

pub fn encode_type_object(object: &TypeObject) -> Result<(TypeObjectDigest, Vec<u8>), Diagnostic> {
    object.validate_local()?;
    let bytes = packed::encode_canonical(
        TYPE_OBJECT_MAGIC,
        TYPE_OBJECT_ENVELOPE_DOMAIN,
        object,
        MAXIMUM_TYPE_OBJECT_BYTES,
    )?;
    Ok((TypeObjectDigest::of(&bytes), bytes))
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
    let object: TypeObject = packed::decode_canonical(
        bytes,
        TYPE_OBJECT_MAGIC,
        TYPE_OBJECT_ENVELOPE_DOMAIN,
        MAXIMUM_TYPE_OBJECT_BYTES,
    )?;
    object.validate_local()?;
    Ok(object)
}

pub fn encode_root(root: &SemanticRoot) -> Result<(SemanticRootDigest, Vec<u8>), Diagnostic> {
    root.validate_local()?;
    let bytes =
        packed::encode_canonical(ROOT_MAGIC, ROOT_ENVELOPE_DOMAIN, root, MAXIMUM_ROOT_BYTES)?;
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
        packed::decode_canonical(bytes, ROOT_MAGIC, ROOT_ENVELOPE_DOMAIN, MAXIMUM_ROOT_BYTES)?;
    root.validate_local()?;
    Ok(root)
}

pub fn encode_dependency(
    dependency: &DependencyRecord,
) -> Result<(DependencyObjectDigest, Vec<u8>), Diagnostic> {
    dependency.validate_local()?;
    let bytes = packed::encode_canonical(
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
    let dependency: DependencyRecord = packed::decode_canonical(
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
    Ok(dependency)
}

pub fn encode_retirement(
    retirement: &RetirementRecord,
) -> Result<(RetirementObjectDigest, Vec<u8>), Diagnostic> {
    retirement.validate_local()?;
    let bytes = packed::encode_canonical(
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
    let retirement: RetirementRecord = packed::decode_canonical(
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

fn codec_error(code: &str, message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(DiagnosticClass::Corrupt, code, message)
}

#[cfg(test)]
mod buffer_encoding_tests {
    use super::*;
    use crate::platform::kernel::TypeForm;
    #[test]
    fn byte_buffer_encoding_is_distinct_and_requires_current_framing() {
        let buffer = TypeObject::new(TypeForm::ByteBuffer).unwrap();
        let (digest, bytes) = encode_type_object(&buffer).unwrap();
        assert_eq!(&bytes[..8], &TYPE_OBJECT_MAGIC);
        assert_eq!(decode_type_object(&bytes, digest).unwrap(), buffer);
        assert_ne!(
            digest,
            encode_type_object(&TypeObject::new(TypeForm::Bytes).unwrap())
                .unwrap()
                .0
        );
        let unframed = bincode::encode_to_vec(&buffer, bincode::config::standard()).unwrap();
        assert!(decode_type_object(&unframed, TypeObjectDigest::of(&unframed)).is_err());
    }
}
