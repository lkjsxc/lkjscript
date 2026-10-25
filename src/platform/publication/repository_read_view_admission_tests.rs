//! Literal owner inventories reserve shared publication visits before projection or store reads.
use super::*;
use crate::platform::kernel::{
    AssociatedTypeBinding, AssociatedTypeBindingValue, AssociatedTypeBound, AssociatedTypeMember,
    CallableResult, DeclarationPayload, DeclarationRecord, DeclarationReference,
    DeclarationVisibility, ExpressionOperation, ExpressionRecord, FieldReference,
    FunctionDeclaration, FunctionEffect, ImplementationOperand, ImplementationParameter,
    LocalValueReference, Name, OwnedContract, OwnedImplementation, OwnedMethod,
    OwnedMethodImplementation, OwnedMethodParameter, OwnerHeader, OwnerKey, OwnerKind, OwnerRecord,
    PackageId, ParameterUse, ReadScopeArgument, ReadScopeBinder, ReadScopeParameter,
    ReadScopeReference, ReadScopeSource, ReadViewDeclaration, ReadViewField, ReadViewFieldKind,
    ResultRelation, TypeObjectDigest,
};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, BindingId, DeclarationId, ExpressionId, FieldId,
    ImplementationParameterId, MethodId, ModuleId, ParameterId, ReadScopeParameterId,
    TypeParameterId,
};
use crate::platform::storage::memory::MemoryPackedStore;

const SEED: &[u8] = b"publication-complete-owner-root-admission";

fn ty(marker: u8) -> TypeObjectDigest {
    TypeObjectDigest::from_bytes([marker; 32])
}

fn reference(ordinal: u64) -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(SEED, 0),
        declaration: DeclarationId::migrate(SEED, ordinal),
    }
}

fn scope_parameter(ordinal: u64) -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, ordinal),
        name: Name::new(format!("scope{ordinal}")).unwrap(),
    }
}

fn scope(binder: ReadScopeBinder, ordinal: u64) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder,
        parameter: ReadScopeParameterId::migrate(SEED, ordinal),
    })
}

fn declaration(ordinal: u64, kind: OwnerKind, payload: DeclarationPayload) -> OwnerRecord {
    OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(OwnerKey::Declaration(reference(ordinal).declaration), kind),
        module: ModuleId::migrate(SEED, 0),
        name: Name::new(format!("owner{ordinal}")).unwrap(),
        visibility: DeclarationVisibility::Private,
        payload,
    })
}

fn expression(ordinal: u64, operation: ExpressionOperation) -> OwnerRecord {
    OwnerRecord::Expression(
        ExpressionRecord::new(ExpressionId::migrate(SEED, ordinal), operation).unwrap(),
    )
}

fn admission(visits: u64) -> CandidateReferenceAdmission {
    CandidateReferenceAdmission::new(CanonicalReadAdmission {
        maximum_decoded_records: Some(visits),
        ..Default::default()
    })
}

fn assert_inventory(record: &OwnerRecord, expected: &[TypeObjectDigest], visits: u64) {
    let exact = admission(visits);
    assert_eq!(
        admit_candidate_owner_type_roots(record, &exact).unwrap(),
        expected
    );
    let error = admit_candidate_owner_type_roots(record, &admission(visits - 1)).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(
        error.code,
        "publication_repository_reference_visits_exhausted"
    );
    // A complete root projection consumes the literal allowance even when the later
    // closure set will deduplicate those roots. The next actual store read is refused.
    let bytes = b"publication-next-reference";
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
    let mut store = MemoryPackedStore::default();
    store.stage(key, bytes, &mut StoreWork::default()).unwrap();
    let mut work = StoreWork::default();
    let error = read_candidate_required(&store, key.domain, key.digest.bytes(), &mut work, &exact)
        .unwrap_err();
    assert_eq!(
        error.code,
        "publication_repository_reference_visits_exhausted"
    );
    assert_eq!(work, StoreWork::default());
}

fn view_owner() -> OwnerRecord {
    let read_scope = scope(ReadScopeBinder::Declaration(reference(0)), 0);
    declaration(
        0,
        OwnerKind::ReadView,
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: vec![TypeParameterId::migrate(SEED, 0)],
            scope_parameter: scope_parameter(0),
            fields: vec![
                ReadViewField {
                    id: FieldId::migrate(SEED, 0),
                    name: Name::new("ordinary").unwrap(),
                    ty: ty(1),
                    kind: ReadViewFieldKind::Ordinary,
                },
                ReadViewField {
                    id: FieldId::migrate(SEED, 1),
                    name: Name::new("unused-read").unwrap(),
                    ty: ty(2),
                    kind: ReadViewFieldKind::ReadReference { read_scope },
                },
                ReadViewField {
                    id: FieldId::migrate(SEED, 2),
                    name: Name::new("duplicate-type").unwrap(),
                    ty: ty(1),
                    kind: ReadViewFieldKind::ReadReference { read_scope },
                },
                ReadViewField {
                    id: FieldId::migrate(SEED, 3),
                    name: Name::new("unused-ordinary").unwrap(),
                    ty: ty(3),
                    kind: ReadViewFieldKind::Ordinary,
                },
            ],
        }),
    )
}

fn zero_field_creation() -> OwnerRecord {
    expression(
        0,
        ExpressionOperation::CreateReadView {
            declaration: reference(0),
            type_arguments: vec![ty(4), ty(5), ty(4)],
            read_scope: ReadScopeArgument::Lexical {
                callable: reference(1),
                binding: BindingId::migrate(SEED, 0),
            },
            source: LocalValueReference::FunctionParameter(ParameterId::migrate(SEED, 0)),
            fields: Vec::new(),
        },
    )
}

fn recursive_witness() -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: reference(8),
        type_arguments: vec![ty(8), ty(1)],
        implementations: vec![
            ImplementationOperand::Concrete {
                implementation: reference(9),
                type_arguments: vec![ty(9), ty(8)],
                implementations: vec![ImplementationOperand::Concrete {
                    implementation: reference(10),
                    type_arguments: vec![ty(10)],
                    implementations: Vec::new(),
                }],
            },
            ImplementationOperand::Parameter {
                scope: reference(2),
                parameter: ImplementationParameterId::migrate(SEED, 2),
            },
        ],
    }
}

#[test]
fn publication_roots_admit_every_unused_scoped_view_field_before_deduplication() {
    assert_inventory(&view_owner(), &[ty(1), ty(2), ty(1), ty(3)], 4);
}

#[test]
fn publication_roots_admit_unused_view_actuals_even_without_fields() {
    assert_inventory(&zero_field_creation(), &[ty(4), ty(5), ty(4)], 3);
}

#[test]
fn publication_roots_admit_family_bindings_prerequisites_and_recursive_method_actuals() {
    let record = declaration(
        2,
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: Vec::new(),
            implementation_parameters: vec![ImplementationParameter {
                id: ImplementationParameterId::migrate(SEED, 0),
                name: Name::new("unused-prerequisite").unwrap(),
                contract: reference(3),
                self_type: ty(5),
                type_arguments: vec![ty(6), ty(5)],
            }],
            contract: reference(3),
            self_type: ty(1),
            type_arguments: vec![ty(4), ty(1)],
            methods: vec![OwnedMethodImplementation {
                method: MethodId::migrate(SEED, 0),
                function: reference(4),
                type_arguments: vec![ty(2), ty(7)],
                implementations: vec![recursive_witness()],
                read_scope_arguments: vec![scope(
                    ReadScopeBinder::AssociatedBinding {
                        implementation: reference(2),
                        member: AssociatedTypeMemberId::migrate(SEED, 1),
                    },
                    1,
                )],
            }],
            associated_types: vec![
                AssociatedTypeBinding {
                    member: AssociatedTypeMemberId::migrate(SEED, 0),
                    value: AssociatedTypeBindingValue::Owned { ty: ty(2) },
                },
                AssociatedTypeBinding {
                    member: AssociatedTypeMemberId::migrate(SEED, 1),
                    value: AssociatedTypeBindingValue::ReadViewFamily {
                        scope_parameter: scope_parameter(1),
                        ty: ty(3),
                    },
                },
            ],
        }),
    );
    assert_inventory(
        &record,
        &[
            ty(1),
            ty(2),
            ty(3),
            ty(4),
            ty(1),
            ty(5),
            ty(6),
            ty(5),
            ty(2),
            ty(7),
            ty(8),
            ty(1),
            ty(9),
            ty(8),
            ty(10),
        ],
        15,
    );
}

#[test]
fn publication_roots_admit_complete_call_and_method_witness_actuals() {
    let call = expression(
        1,
        ExpressionOperation::ImplementationCall {
            function: reference(4),
            type_arguments: vec![ty(2), ty(2)],
            read_scope_arguments: vec![scope(ReadScopeBinder::Declaration(reference(1)), 0)],
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            implementations: vec![recursive_witness()],
            arguments: Vec::new(),
        },
    );
    assert_inventory(
        &call,
        &[ty(2), ty(2), ty(8), ty(1), ty(9), ty(8), ty(10)],
        7,
    );
    let method = expression(
        2,
        ExpressionOperation::MethodCall {
            witness: recursive_witness(),
            contract: reference(3),
            method: MethodId::migrate(SEED, 0),
            read_scope_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    assert_inventory(&method, &[ty(8), ty(1), ty(9), ty(8), ty(10)], 5);
}

#[test]
fn publication_roots_refuse_excessive_witness_depth_before_projection() {
    let mut witness = ImplementationOperand::Parameter {
        scope: reference(2),
        parameter: ImplementationParameterId::migrate(SEED, 2),
    };
    for _ in 0..crate::platform::kernel::contract::MAXIMUM_TYPE_DEPTH {
        witness = ImplementationOperand::Concrete {
            implementation: reference(8),
            type_arguments: Vec::new(),
            implementations: vec![witness],
        };
    }
    // The raw carrier lets admission check exact depth independently of semantic decoding.
    let mut record = OwnerRecord::Expression(ExpressionRecord {
        contract_version: crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION,
        id: ExpressionId::migrate(SEED, 5),
        operation: ExpressionOperation::MethodCall {
            witness,
            contract: reference(3),
            method: MethodId::migrate(SEED, 0),
            read_scope_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    });
    assert_eq!(
        admit_candidate_owner_type_roots(&record, &admission(0)).unwrap(),
        Vec::<TypeObjectDigest>::new()
    );
    let OwnerRecord::Expression(expression) = &mut record else {
        unreachable!()
    };
    let ExpressionOperation::MethodCall { witness, .. } = &mut expression.operation else {
        unreachable!()
    };
    let previous = std::mem::replace(
        witness,
        ImplementationOperand::Parameter {
            scope: reference(2),
            parameter: ImplementationParameterId::migrate(SEED, 2),
        },
    );
    *witness = ImplementationOperand::Concrete {
        implementation: reference(8),
        type_arguments: Vec::new(),
        implementations: vec![previous],
    };
    // The 257th edge must fail even when the projected root inventory would be empty.
    let error = admit_candidate_owner_type_roots(&record, &admission(u64::MAX)).unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert_eq!(
        error.code,
        "publication_repository_reference_visits_exhausted"
    );
}

#[test]
fn publication_roots_admit_nominal_owned_record_types_in_every_custody_operation() {
    let source = ExpressionId::migrate(SEED, 3);
    let body = ExpressionId::migrate(SEED, 4);
    for operation in [
        ExpressionOperation::PackOwnedRecord {
            record_type: ty(11),
            fields: Vec::new(),
        },
        ExpressionOperation::UnpackOwnedRecord {
            record_type: ty(11),
            source,
            fields: Vec::new(),
            body,
        },
        ExpressionOperation::BorrowOwnedRecordField {
            record_type: ty(11),
            source,
            field: FieldReference {
                package: reference(5).package,
                field: FieldId::migrate(SEED, 5),
            },
            binding: BindingId::migrate(SEED, 5),
            body,
        },
    ] {
        assert_inventory(&expression(3, operation), &[ty(11)], 1);
    }
}

#[test]
fn publication_roots_keep_scoped_callable_results_and_unused_prerequisites() {
    let source = ParameterId::migrate(SEED, 6);
    let read_scope = scope(ReadScopeBinder::Declaration(reference(6)), 6);
    let function = declaration(
        6,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: vec![ImplementationParameter {
                id: ImplementationParameterId::migrate(SEED, 6),
                name: Name::new("unused").unwrap(),
                contract: reference(3),
                self_type: ty(1),
                type_arguments: vec![ty(2), ty(1)],
            }],
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![source],
            result: CallableResult {
                ty: ty(3),
                relation: ResultRelation::ViewFrom {
                    source,
                    scope: read_scope,
                },
            },
            read_scope_parameters: vec![scope_parameter(6), scope_parameter(7)],
            read_scope_sources: vec![ReadScopeSource {
                source,
                scope: read_scope,
            }],
            effect: FunctionEffect::Pure,
            body: ExpressionId::migrate(SEED, 6),
        }),
    );
    assert_inventory(&function, &[ty(1), ty(2), ty(1), ty(3)], 4);
    let method_id = MethodId::migrate(SEED, 7);
    let method_scope = scope(
        ReadScopeBinder::ContractMethod {
            contract: reference(7),
            method: method_id,
        },
        7,
    );
    let contract = declaration(
        7,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: TypeParameterId::migrate(SEED, 7),
            type_parameters: Vec::new(),
            methods: vec![OwnedMethod {
                id: method_id,
                name: Name::new("read").unwrap(),
                parameters: vec![
                    OwnedMethodParameter {
                        ty: ty(1),
                        use_mode: ParameterUse::Borrow,
                    },
                    OwnedMethodParameter {
                        ty: ty(2),
                        use_mode: ParameterUse::Unrestricted,
                    },
                    OwnedMethodParameter {
                        ty: ty(1),
                        use_mode: ParameterUse::Borrow,
                    },
                ],
                result: CallableResult {
                    ty: ty(3),
                    relation: ResultRelation::ViewFrom {
                        source: 0,
                        scope: method_scope,
                    },
                },
                read_scope_parameters: vec![scope_parameter(7), scope_parameter(8)],
                read_scope_sources: vec![ReadScopeSource {
                    source: 0,
                    scope: method_scope,
                }],
                effect: FunctionEffect::Pure,
            }],
            associated_types: vec![AssociatedTypeMember {
                id: AssociatedTypeMemberId::migrate(SEED, 7),
                name: Name::new("View").unwrap(),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: scope_parameter(9),
                },
            }],
        }),
    );
    assert_inventory(&contract, &[ty(1), ty(2), ty(1), ty(3)], 4);
}

#[test]
fn publication_root_inventories_share_exact_allowance_with_the_next_store_read() {
    let bytes = b"shared-publication-reference";
    let key = ObjectKey::for_bytes(ObjectDomain::Blob, bytes);
    let mut store = MemoryPackedStore::default();
    store.stage(key, bytes, &mut StoreWork::default()).unwrap();
    for (visits, read_succeeds) in [(8, true), (7, false)] {
        let admission = admission(visits);
        assert_eq!(
            admit_candidate_owner_type_roots(&view_owner(), &admission).unwrap(),
            vec![ty(1), ty(2), ty(1), ty(3)]
        );
        assert_eq!(
            admit_candidate_owner_type_roots(&zero_field_creation(), &admission).unwrap(),
            vec![ty(4), ty(5), ty(4)]
        );
        let mut work = StoreWork::default();
        let result = read_candidate_required(
            &store,
            key.domain,
            key.digest.bytes(),
            &mut work,
            &admission,
        );
        if read_succeeds {
            assert_eq!(result.unwrap(), bytes);
            assert_eq!(work.catalog_lookups, 1);
            assert_eq!(work.objects_read, 1);
            assert_eq!(work.bytes_read, bytes.len() as u64);
            let before = work;
            assert_eq!(
                read_candidate_required(
                    &store,
                    key.domain,
                    key.digest.bytes(),
                    &mut work,
                    &admission
                )
                .unwrap_err()
                .code,
                "publication_repository_reference_visits_exhausted"
            );
            assert_eq!(work, before);
        } else {
            assert_eq!(
                result.unwrap_err().code,
                "publication_repository_reference_visits_exhausted"
            );
            assert_eq!(work, StoreWork::default());
        }
    }
    // One short for the second owner is also refused by the same shared admission,
    // before a partial root vector can become a successful projection.
    let admission = admission(6);
    assert_eq!(
        admit_candidate_owner_type_roots(&view_owner(), &admission).unwrap(),
        vec![ty(1), ty(2), ty(1), ty(3)]
    );
    assert_eq!(
        admit_candidate_owner_type_roots(&zero_field_creation(), &admission)
            .unwrap_err()
            .code,
        "publication_repository_reference_visits_exhausted"
    );
}
