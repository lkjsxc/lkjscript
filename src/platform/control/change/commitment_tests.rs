//! Independent current authored-byte vectors and request commitment controls.

use super::*;
use crate::platform::canonical_schema::SCHEMA_EPOCH;
use crate::platform::change::{
    AuthoredChangeAdmission, AuthoredLiteralUpdate, AuthoredLiteralValue, CanonicalEditAdmission,
    CanonicalMapUpdateAdmission, CanonicalReadAdmission, ChangeBudget, ImpactAdmission,
    StagingAdmission, TestAdmission, ValidationAdmission, WitnessReadAdmission,
    WitnessUpdateAdmission,
};
use crate::platform::semantic_id::{MethodId, RequirementParameterId, TypeParameterId};

fn bounded_budget() -> ChangeBudget {
    ChangeBudget {
        authored: AuthoredChangeAdmission {
            maximum_operations: Some(100),
            maximum_preconditions: Some(100),
            maximum_allocated_identities: Some(100),
            maximum_type_nodes: Some(100),
        },
        canonical_edits: CanonicalEditAdmission {
            maximum_owner_edits: Some(100),
            maximum_type_edits: Some(100),
            maximum_dependency_edits: Some(100),
            maximum_retirement_edits: Some(100),
        },
        canonical_reads: CanonicalReadAdmission {
            maximum_point_reads: Some(100),
            maximum_map_pages: Some(100),
            maximum_map_entries: Some(100),
            maximum_catalog_lookups: Some(100),
            maximum_objects: Some(100),
            maximum_bytes: Some(100),
            maximum_decoded_records: Some(100),
        },
        canonical_map_update: CanonicalMapUpdateAdmission {
            maximum_pages_encoded: Some(100),
            maximum_bytes_encoded: Some(100),
        },
        witness_reads: WitnessReadAdmission {
            maximum_point_reads: Some(100),
            maximum_map_pages: Some(100),
            maximum_map_entries: Some(100),
            maximum_catalog_lookups: Some(100),
            maximum_objects: Some(100),
            maximum_bytes: Some(100),
            maximum_decoded_records: Some(100),
        },
        impact: ImpactAdmission {
            maximum_affected_owners: Some(100),
            maximum_summary_owners: Some(100),
            maximum_summary_edits: Some(100),
            maximum_ownership_steps: Some(100),
            maximum_behavior_owners: Some(100),
            maximum_relation_edges: Some(100),
            maximum_relation_fanout: Some(100),
        },
        validation: ValidationAdmission {
            maximum_owner_records: Some(100),
            maximum_ownership_entries: Some(100),
            maximum_type_objects: Some(100),
            maximum_expression_steps: Some(100),
            maximum_diagnostics: Some(100),
        },
        tests: TestAdmission {
            maximum_selected: Some(100),
            maximum_dependencies_per_test: Some(100),
            maximum_ownership_steps: Some(100),
            maximum_owners_visited: Some(100),
        },
        witness_update: WitnessUpdateAdmission {
            maximum_edits: Some(100),
            maximum_pages_encoded: Some(100),
            maximum_bytes_encoded: Some(100),
        },
        staging: StagingAdmission {
            maximum_objects: Some(100),
            maximum_bytes: Some(100),
            maximum_pages: Some(100),
        },
    }
}

fn declaration() -> DeclarationSelector {
    DeclarationSelector::Id {
        declaration: DeclarationId::from_bytes([1; 16]).unwrap(),
    }
}

fn contract(result: AuthoredType) -> AuthoredChange {
    AuthoredChange::SetFunctionContract {
        function: declaration(),
        result: AuthoredCallableResult::value(result),
        read_scope_parameters: Vec::new(),
        read_scope_sources: Vec::new(),
        effect: AuthoredFunctionEffect::Pure {},
    }
}

fn expression(operation: AuthoredExpressionOperation) -> AuthoredExpression {
    AuthoredExpression {
        symbol: None,
        operation,
    }
}

fn unhex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0);
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn literal_intent(body: &str) -> Vec<u8> {
    [
        b"LKJACHNG".as_slice(),
        &SCHEMA_EPOCH.to_le_bytes(),
        &unhex(concat!(
            "0707070707070707070707070707070707070707070707070707070707070707",
            "00000000000000000000000000000001",
        )),
        &unhex(body),
    ]
    .concat()
}

fn independent_commitment(
    intent: &[u8],
    budget: &[u8],
    options: &PublicationOptions,
) -> ChangeRequestCommitment {
    fn field(hasher: &mut blake3::Hasher, bytes: &[u8]) {
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(bytes);
    }
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.change-request-commitment.v2");
    field(&mut hasher, b"lkjscript-authored-change");
    field(&mut hasher, intent);
    field(&mut hasher, budget);
    for option in [
        options.idempotency_key.as_deref(),
        options.intent.as_deref(),
    ] {
        hasher.update(&[u8::from(option.is_some())]);
        if let Some(value) = option {
            field(&mut hasher, value.as_bytes());
        }
    }
    ChangeRequestCommitment(*hasher.finalize().as_bytes())
}

fn assert_golden(change: AuthoredChange, current_body: &str) {
    let budget = bounded_budget().validate().unwrap();
    let options = PublicationOptions {
        idempotency_key: Some("current-commitment".to_owned()),
        intent: Some("current-options".to_owned()),
    };
    let expected_budget = [
        b"LKJABDG2".as_slice(),
        &[2, 0],
        &[1, 0, 0, 0, 0, 0, 0, 0, 100].repeat(46),
    ]
    .concat();
    assert_eq!(
        crate::platform::change::canonical_authored_budget_bytes(budget).unwrap(),
        expected_budget
    );
    let semantic = AuthoredChangeSet {
        base: RevisionId::from_digest([7; 32]),
        preconditions: Vec::new(),
        changes: vec![change],
        budget,
    };
    let normalized = normalize_change_request(semantic, options.clone()).unwrap();
    let current_intent = literal_intent(current_body);
    assert_eq!(
        crate::platform::change::canonical_authored_intent_bytes(&normalized.semantic).unwrap(),
        current_intent,
    );
    assert_eq!(
        normalized.request_commitment,
        independent_commitment(&current_intent, &expected_budget, &options),
    );
}

#[test]
fn literal_and_owned_contracts_commit_complete_current_bytes() {
    assert_golden(
        AuthoredChange::SetConstant {
            constant: declaration(),
            ty: AuthoredType::I64 {},
            value: expression(AuthoredExpressionOperation::I64 { value: 42 }),
        },
        "2c0101010101010101010101010101010101030003000000000000002a",
    );
    assert_golden(
        AuthoredChange::SetFunctionLiterals {
            function: declaration(),
            literals: vec![AuthoredLiteralUpdate {
                expression: ExpressionId::from_bytes([2; 16]).unwrap(),
                value: AuthoredLiteralValue::I64 { value: 42 },
            }],
        },
        "2f010101010101010101010101010101010100000000000000010202020202020202020202020202020201000000000000002a",
    );
    assert_golden(
        contract(AuthoredType::ByteBuffer {}),
        "15010101010101010101010101010101010114010000000000000000000000000000000001",
    );
    assert_golden(
        contract(AuthoredType::OwnedI64Cell {}),
        "15010101010101010101010101010101010115010000000000000000000000000000000001",
    );
    assert_golden(
        contract(AuthoredType::OwnedProduct {
            fields: vec![AuthoredStructuralTypeField {
                name: Name::new("value").unwrap(),
                ty: AuthoredType::OwnedI64Cell {},
            }],
        }),
        "150101010101010101010101010101010101160000000000000001000000000000000576616c756515010000000000000000000000000000000001",
    );
    assert_golden(
        contract(AuthoredType::OwnedChoice {
            cases: vec![
                AuthoredStructuralTypeField {
                    name: Name::new("value").unwrap(),
                    ty: AuthoredType::OwnedI64Cell {},
                },
                AuthoredStructuralTypeField {
                    name: Name::new("empty").unwrap(),
                    ty: AuthoredType::Unit {},
                },
            ],
        }),
        "150101010101010101010101010101010101170000000000000002000000000000000576616c7565150000000000000005656d70747901010000000000000000000000000000000001",
    );
    assert_golden(
        AuthoredChange::SetOwnedContract {
            associated_types: Vec::new(),
            declaration: declaration(),
            type_parameters: Vec::new(),
            self_type: AuthoredType::TypeParameter {
                parameter: AuthoredTypeParameterReference::Id {
                    parameter: TypeParameterId::from_bytes([3; 16]).unwrap(),
                },
            },
            methods: vec![AuthoredOwnedMethod {
                id: MethodId::from_bytes([4; 16]).unwrap(),
                name: Name::new("run").unwrap(),
                parameters: Vec::new(),
                result: AuthoredCallableResult::value(AuthoredType::I64 {}),
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: AuthoredFunctionEffect::Task {
                    requirements: Vec::new(),
                    effect_parameters: Vec::new(),
                },
            }],
        },
        "6601010101010101010101010101010101010801030303030303030303030303030303030000000000000000000000000000000104040404040404040404040404040404000000000000000372756e000000000000000003010000000000000000000000000000000002000000000000000000000000000000000000000000000000",
    );
}

#[test]
fn scalar_and_task_contracts_commit_complete_current_bytes() {
    assert_golden(
        contract(AuthoredType::I64 {}),
        "15010101010101010101010101010101010103010000000000000000000000000000000001",
    );
    assert_golden(
        AuthoredChange::SetFunctionContract {
            function: declaration(),
            result: AuthoredCallableResult::value(AuthoredType::I64 {}),
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect: AuthoredFunctionEffect::Task {
                effect_parameters: Vec::new(),
                requirements: vec![AuthoredRequirementReference::ParameterExact {
                    package: PackageId::from_bytes([2; 16]).unwrap(),
                    parameter: RequirementParameterId::from_bytes([3; 16]).unwrap(),
                }],
            },
        },
        "1501010101010101010101010101010101010301000000000000000000000000000000000200000000000000000000000000000001040202020202020202020202020202020203030303030303030303030303030303",
    );
    assert_golden(
        contract(AuthoredType::F64 {}),
        "15010101010101010101010101010101010113010000000000000000000000000000000001",
    );
}

#[test]
fn parallel_intent_commits_both_complete_calls() {
    let child = || {
        expression(AuthoredExpressionOperation::Call {
            function: AuthoredDeclarationReference::Local {
                declaration: DeclarationSelector::Id {
                    declaration: DeclarationId::from_bytes([2; 16]).unwrap(),
                },
            },
            type_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            arguments: Vec::new(),
        })
    };
    assert_golden(
        AuthoredChange::ReplaceFunctionBody {
            function: declaration(),
            body: expression(AuthoredExpressionOperation::Parallel {
                left: Box::new(child()),
                right: Box::new(child()),
            }),
        },
        "2301010101010101010101010101010101010024003900000000000000000000000000000000010102020202020202020202020202020202000000000000000000000000000000000000000000000000003900000000000000000000000000000000010102020202020202020202020202020202000000000000000000000000000000000000000000000000",
    );
}

#[test]
fn implementation_authority_and_empty_application_commit_complete_current_bytes() {
    let local = |byte| AuthoredDeclarationReference::Local {
        declaration: DeclarationSelector::Id {
            declaration: DeclarationId::from_bytes([byte; 16]).unwrap(),
        },
    };
    let request = |effect_arguments, requirement_arguments| AuthoredChangeSet {
        base: RevisionId::from_digest([7; 32]),
        preconditions: Vec::new(),
        changes: vec![AuthoredChange::ReplaceFunctionBody {
            function: declaration(),
            body: expression(AuthoredExpressionOperation::ImplementationCall {
                function: local(2),
                type_arguments: Vec::new(),
                effect_arguments,
                requirement_arguments,
                read_scope_arguments: Vec::new(),
                implementations: vec![AuthoredImplementationOperand::Concrete {
                    implementation: local(3),
                    type_arguments: Vec::new(),
                    implementations: Vec::new(),
                }],
                arguments: vec![expression(AuthoredExpressionOperation::I64 { value: 42 })],
            }),
        }],
        budget: bounded_budget(),
    };
    let empty = request(Vec::new(), Vec::new());
    let current_empty = literal_intent(
        "230101010101010101010101010101010101003a000000000000000000000000000000000101020202020202020202020202020202020000000000000000000000000000000000000000000000010101010303030303030303030303030303030300000000000000010003000000000000002a",
    );
    assert_eq!(
        crate::platform::change::canonical_authored_intent_bytes(&empty).unwrap(),
        current_empty
    );

    let authority = request(
        vec![AuthoredEffectRow::default()],
        vec![AuthoredRequirementReference::Exact {
            package: PackageId::from_bytes([4; 16]).unwrap(),
            requirement: RequirementId::from_bytes([5; 16]).unwrap(),
        }],
    );
    let current_authority = literal_intent(
        "230101010101010101010101010101010101003a00000000000000010104040404040404040404040404040404050505050505050505050505050505050000000000000001000000000000000000000000000000000101020202020202020202020202020202020000000000000000000000000000000000000000000000010101010303030303030303030303030303030300000000000000010003000000000000002a",
    );
    assert_eq!(
        crate::platform::change::canonical_authored_intent_bytes(&authority).unwrap(),
        current_authority
    );
    let options = PublicationOptions {
        idempotency_key: Some("current-commitment".to_owned()),
        intent: Some("current-options".to_owned()),
    };
    let budget =
        crate::platform::change::canonical_authored_budget_bytes(bounded_budget()).unwrap();
    for (request, intent) in [(empty, current_empty), (authority, current_authority)] {
        let normalized = normalize_change_request(request, options.clone()).unwrap();
        assert_eq!(
            normalized.request_commitment,
            independent_commitment(&intent, &budget, &options)
        );
    }
    for request in [
        request(vec![AuthoredEffectRow::default()], Vec::new()),
        request(
            Vec::new(),
            vec![AuthoredRequirementReference::Exact {
                package: PackageId::from_bytes([4; 16]).unwrap(),
                requirement: RequirementId::from_bytes([5; 16]).unwrap(),
            }],
        ),
    ] {
        assert_eq!(
            &crate::platform::change::canonical_authored_intent_bytes(&request).unwrap()[..8],
            b"LKJACHNG",
        );
    }
}
