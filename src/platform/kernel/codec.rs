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
mod nominal_encoding_tests {
    use super::*;
    use crate::platform::kernel::{DeclarationReference, TypeForm};

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
        assert_eq!(&bytes[..8], b"LKJOWN17");
        assert_eq!(
            decode_owner(&bytes, owner.owner(), owner.kind(), digest).unwrap(),
            owner
        );
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
    } else {
        (OWNER_MAGIC, OWNER_ENVELOPE_DOMAIN)
    };
    let bytes = packed::encode(magic, domain, record, MAXIMUM_OWNER_OBJECT_BYTES)?;
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
        let record: OwnerRecord = packed::decode(
            bytes,
            super::contract::REQUIREMENT_OWNER_MAGIC,
            super::contract::REQUIREMENT_OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        if record.header().contract_version != super::contract::REQUIREMENT_GRAPH_CONTRACT_VERSION {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "requirement envelope has a foreign owner generation",
            ));
        }
        record
    } else if bytes.starts_with(&super::contract::TRANSACTION_OWNER_MAGIC) {
        let record: OwnerRecord = packed::decode(
            bytes,
            super::contract::TRANSACTION_OWNER_MAGIC,
            super::contract::TRANSACTION_OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?;
        if record.header().contract_version != super::contract::TRANSACTION_GRAPH_CONTRACT_VERSION {
            return Err(codec_error(
                "kernel_owner_encoding_generation",
                "transaction envelope has a foreign owner generation",
            ));
        }
        record
    } else {
        packed::decode(
            bytes,
            OWNER_MAGIC,
            OWNER_ENVELOPE_DOMAIN,
            MAXIMUM_OWNER_OBJECT_BYTES,
        )?
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
