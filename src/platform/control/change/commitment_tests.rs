//! Fixed request vectors for the actual normalization/commitment path.
//!
//! Originally introduced identities were checked at ae83a2fb (18), 94dad92d (19),
//! f9c2630b (20), caeb9229 (21), 0dadaed8 (22), 5847e021 (23), and 605a7cf8 (24).
//! These are designed typed historical-generation requests, not captured acceptance receipts.
//! Literal intent bytes and commitments were calculated independently of the production
//! selector/hasher, with every budget dimension fixed at 100 and both options present.

use super::*;
use crate::platform::change::{
    AuthoredChangeAdmission, AuthoredLiteralUpdate, AuthoredLiteralValue, CanonicalEditAdmission,
    CanonicalMapUpdateAdmission, CanonicalReadAdmission, ChangeBudget, ImpactAdmission,
    StagingAdmission, TestAdmission, ValidationAdmission, WitnessReadAdmission,
    WitnessUpdateAdmission,
};
use crate::platform::semantic_id::{MethodId, RequirementParameterId, TypeParameterId};

fn frozen_budget() -> ChangeBudget {
    ChangeBudget {
        authored: AuthoredChangeAdmission {
            maximum_operations: 100,
            maximum_preconditions: 100,
            maximum_allocated_identities: 100,
            maximum_type_nodes: 100,
        },
        canonical_edits: CanonicalEditAdmission {
            maximum_owner_edits: 100,
            maximum_type_edits: 100,
            maximum_dependency_edits: 100,
            maximum_retirement_edits: 100,
        },
        canonical_reads: CanonicalReadAdmission {
            maximum_point_reads: 100,
            maximum_map_pages: 100,
            maximum_map_entries: 100,
            maximum_catalog_lookups: 100,
            maximum_objects: 100,
            maximum_bytes: 100,
            maximum_decoded_records: 100,
        },
        canonical_map_update: CanonicalMapUpdateAdmission {
            maximum_pages_encoded: 100,
            maximum_bytes_encoded: 100,
        },
        witness_reads: WitnessReadAdmission {
            maximum_point_reads: 100,
            maximum_map_pages: 100,
            maximum_map_entries: 100,
            maximum_catalog_lookups: 100,
            maximum_objects: 100,
            maximum_bytes: 100,
            maximum_decoded_records: 100,
        },
        impact: ImpactAdmission {
            maximum_affected_owners: 100,
            maximum_summary_owners: 100,
            maximum_summary_edits: 100,
            maximum_ownership_steps: 100,
            maximum_behavior_owners: 100,
            maximum_relation_edges: 100,
            maximum_relation_fanout: 100,
        },
        validation: ValidationAdmission {
            maximum_owner_records: 100,
            maximum_ownership_entries: 100,
            maximum_type_objects: 100,
            maximum_expression_steps: 100,
            maximum_diagnostics: 100,
        },
        tests: TestAdmission {
            maximum_selected: 100,
            maximum_dependencies_per_test: 100,
            maximum_ownership_steps: 100,
            maximum_owners_visited: 100,
        },
        witness_update: WitnessUpdateAdmission {
            maximum_edits: 100,
            maximum_pages_encoded: 100,
            maximum_bytes_encoded: 100,
        },
        staging: StagingAdmission {
            maximum_objects: 100,
            maximum_bytes: 100,
            maximum_pages: 100,
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
        result,
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

fn assert_golden(change: AuthoredChange, magic: &[u8; 8], body: &str, expected: &str) {
    let semantic = AuthoredChangeSet {
        base: RevisionId::from_digest([7; 32]),
        preconditions: Vec::new(),
        changes: vec![change],
        budget: frozen_budget().validate().unwrap(),
    };
    let normalized = normalize_change_request(
        semantic,
        PublicationOptions {
            idempotency_key: Some("historical-commitment".to_owned()),
            intent: Some("frozen-options".to_owned()),
        },
    )
    .unwrap();
    // No magic is patched into the encoded request: the real typed operation must select it.
    let expected_intent = [
        magic.as_slice(),
        &unhex(concat!(
            "0707070707070707070707070707070707070707070707070707070707070707",
            "00000000000000000000000000000001",
        )),
        &unhex(body),
    ]
    .concat();
    assert_eq!(
        crate::platform::change::canonical_authored_intent_bytes(&normalized.semantic).unwrap(),
        expected_intent,
    );
    let expected_budget = [
        b"LKJABG01".as_slice(),
        &[0, 0, 0, 0, 0, 0, 0, 100].repeat(46),
    ]
    .concat();
    assert_eq!(
        crate::platform::change::canonical_authored_budget_bytes(normalized.semantic.budget)
            .unwrap(),
        expected_budget,
    );
    assert_eq!(normalized.request_commitment.to_string(), expected);
}

#[test]
fn historical_generations_18_through_24_keep_original_commitment_goldens() {
    assert_golden(
        AuthoredChange::SetConstant {
            constant: declaration(),
            ty: AuthoredType::I64 {},
            value: expression(AuthoredExpressionOperation::I64 { value: 42 }),
        },
        b"LKJACR18",
        "2c0101010101010101010101010101010101030003000000000000002a",
        "request_f77bce029c7a09a56790e53a53d5944be56795d4b6709203d9a0af0c09fe9676",
    );
    assert_golden(
        AuthoredChange::SetFunctionLiterals {
            function: declaration(),
            literals: vec![AuthoredLiteralUpdate {
                expression: ExpressionId::from_bytes([2; 16]).unwrap(),
                value: AuthoredLiteralValue::I64 { value: 42 },
            }],
        },
        b"LKJACR19",
        "2f010101010101010101010101010101010100000000000000010202020202020202020202020202020201000000000000002a",
        "request_22698d797bf0622fcf7bbba7b2c5a9592ad36d225424ed86afb19f695204ef89",
    );
    assert_golden(
        contract(AuthoredType::ByteBuffer {}),
        b"LKJACR20",
        "1501010101010101010101010101010101011401",
        "request_60137b8cd919eb08b80ab8f0763692132f417aafe8738e1f2d9c984547df018e",
    );
    assert_golden(
        contract(AuthoredType::OwnedI64Cell {}),
        b"LKJACR21",
        "1501010101010101010101010101010101011501",
        "request_f3c87495d55c7008ad1eca40b808ef119bf8a7a3edb3ebf49f7d4b0bb522e852",
    );
    assert_golden(
        contract(AuthoredType::OwnedProduct {
            fields: vec![AuthoredStructuralTypeField {
                name: Name::new("value").unwrap(),
                ty: AuthoredType::OwnedI64Cell {},
            }],
        }),
        b"LKJACR22",
        "150101010101010101010101010101010101160000000000000001000000000000000576616c75651501",
        "request_f0ec305efde9aeac71f82a805967adf16b8146f0ffe565a57b83ab8689cf9e9d",
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
        b"LKJACR23",
        "150101010101010101010101010101010101170000000000000002000000000000000576616c7565150000000000000005656d7074790101",
        "request_0a5055ced42d390efe1c86706572c3ddb5e70791b3c89d3f37344ce44abef19d",
    );
    assert_golden(
        AuthoredChange::SetOwnedContract {
            declaration: declaration(),
            self_type: AuthoredType::TypeParameter {
                parameter: AuthoredTypeParameterReference::Id {
                    parameter: TypeParameterId::from_bytes([3; 16]).unwrap(),
                },
            },
            methods: vec![AuthoredOwnedMethod {
                id: MethodId::from_bytes([4; 16]).unwrap(),
                name: Name::new("run").unwrap(),
                parameters: Vec::new(),
                result: AuthoredType::I64 {},
                effect: AuthoredFunctionEffect::Task {
                    requirements: Vec::new(),
                    effect_parameters: Vec::new(),
                },
            }],
        },
        b"LKJACR24",
        "570101010101010101010101010101010101080103030303030303030303030303030303000000000000000104040404040404040404040404040404000000000000000372756e0000000000000000030200000000000000000000000000000000",
        "request_9e20770ef4474c7202a8d2d24822b64a73e1b1849f2a280518d57b7d648c49a9",
    );
}

#[test]
fn earlier_generations_keep_historical_commitment_goldens() {
    assert_golden(
        contract(AuthoredType::I64 {}),
        b"LKJACR14",
        "1501010101010101010101010101010101010301",
        "request_ea816f82a586627593ccb73b59c72d76faddb0cc384c4b38cc04e1fab5dc5c36",
    );
    assert_golden(
        AuthoredChange::SetFunctionContract {
            function: declaration(),
            result: AuthoredType::I64 {},
            effect: AuthoredFunctionEffect::Task {
                effect_parameters: Vec::new(),
                requirements: vec![AuthoredRequirementReference::ParameterExact {
                    package: PackageId::from_bytes([2; 16]).unwrap(),
                    parameter: RequirementParameterId::from_bytes([3; 16]).unwrap(),
                }],
            },
        },
        b"LKJACR15",
        "150101010101010101010101010101010101030200000000000000000000000000000001040202020202020202020202020202020203030303030303030303030303030303",
        "request_fdedfc09bb3f62aecd53452c5da5059f6c308c0647f4fe88e2704deb5fa2aee0",
    );
    assert_golden(
        contract(AuthoredType::F64 {}),
        b"LKJACR17",
        "1501010101010101010101010101010101011301",
        "request_b13f2acf848f550ce59fb0fa3561929e4df8b871aac889d7d7889585eec13c3d",
    );
}

#[test]
fn current_parallel_generation_25_keeps_its_commitment_golden() {
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
        b"LKJACR25",
        "2301010101010101010101010101010101010024000b000000000000000001010202020202020202020202020202020200000000000000000000000000000000000b000000000000000001010202020202020202020202020202020200000000000000000000000000000000",
        "request_8fe7ca95ee147d46a87f38a830350911814c41cf5d3628800ada9c2f6d282abb",
    );
}

#[test]
fn unsupported_and_future_magics_keep_the_existing_current_identity_fallback() {
    for intent in [
        b"".as_slice(),
        b"LKJACR13",
        b"LKJACR26",
        b"LKJACR99",
        b"unknown",
    ] {
        assert_eq!(
            commitment_codec_identity(intent),
            AUTHORED_CHANGE_CODEC_IDENTITY
        );
    }
    assert_eq!(
        commitment_codec_identity(b"LKJACR16"),
        "lkjscript-authored-change-codec-16"
    );
    assert_eq!(
        commitment_codec_identity(b"LKJACR17"),
        "lkjscript-authored-change-codec-17"
    );
}
