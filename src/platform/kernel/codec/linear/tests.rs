//! Independent current-encoding and formerly bounded owner controls.

use super::*;
use crate::platform::diagnostic::DiagnosticClass;
use crate::platform::kernel::contract::{
    GRAPH_CONTRACT_VERSION, OWNER_ENVELOPE_DOMAIN, OWNER_MAGIC,
};
use crate::platform::packed;
use crate::platform::semantic_id::*;
use crate::platform::storage::memory::MemoryPackedStore;
use crate::platform::storage::object::{
    ImmutableObjectStore, ObjectDomain, ObjectKey, StageOutcome, StoreWork,
};

const SEED: &[u8] = b"linear-owner-codec";

fn raw_owner<T: Encode>(value: &T) -> Vec<u8> {
    packed::linear::encode(OWNER_MAGIC, OWNER_ENVELOPE_DOMAIN, value, None).unwrap()
}

fn wire_round_trip(record: OwnerRecord) {
    // The unchanged derived owner encoder is an independent wire-layout oracle.
    let configuration = bincode::config::standard();
    let payload = bincode::encode_to_vec(&record, configuration).unwrap();
    let (decoded, consumed): (LinearOwner, usize) =
        bincode::decode_from_slice(&payload, configuration).unwrap();
    assert_eq!(consumed, payload.len());
    assert_eq!(decoded.0, record);
    assert_eq!(
        bincode::encode_to_vec(&decoded, configuration).unwrap(),
        payload
    );
}

fn scope() -> ReadScopeArgument {
    ReadScopeArgument::Lexical {
        callable: declaration(),
        binding: BindingId::migrate(SEED, 19),
    }
}

fn declaration() -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(SEED, 0),
        declaration: DeclarationId::migrate(SEED, 0),
    }
}

fn operand() -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: declaration(),
        type_arguments: vec![TypeObjectDigest::of(b"linear-type-one")],
        implementations: vec![ImplementationOperand::Parameter {
            scope: declaration(),
            parameter: ImplementationParameterId::migrate(SEED, 0),
        }],
    }
}

#[test]
fn sequence_owner_crosses_former_child_and_byte_limits_through_packed_storage() {
    // 25 + 17*N payload bytes: this exceeds both 100000 children and the
    // former 4 MiB payload limit, independently of the 50-byte envelope.
    const ITEMS: usize = 246_723;
    let items = (1..=ITEMS)
        .map(|ordinal| ExpressionId::migrate(SEED, u64::try_from(ordinal).unwrap()))
        .collect();
    let record = OwnerRecord::Expression(
        ExpressionRecord::new(
            ExpressionId::migrate(SEED, 0),
            ExpressionOperation::Sequence { items },
        )
        .unwrap(),
    );
    let (digest, bytes) = encode_owner(&record).unwrap();
    assert_eq!(bytes.len(), 75 + 17 * ITEMS);
    assert!(bytes.len() - 50 > 4 * 1024 * 1024);
    let key = ObjectKey::from_digest(ObjectDomain::Owner, digest.bytes());
    let mut store = MemoryPackedStore::default();
    let mut work = StoreWork::default();
    assert_eq!(
        store.stage(key, &bytes, &mut work).unwrap(),
        StageOutcome::Inserted
    );
    assert!(!store.seal_staged(64 * 1024, &mut work).unwrap().is_empty());
    let stored = store
        .read(key, key.domain.maximum_bytes(), &mut work)
        .unwrap()
        .unwrap();
    assert_eq!(stored, bytes);
    let decoded = decode_owner(&stored, record.owner(), record.kind(), digest).unwrap();
    assert_eq!(decoded, record);
    let OwnerRecord::Expression(expression) = decoded else {
        panic!("expected expression owner");
    };
    let children = expression.children();
    assert_eq!(children.len(), ITEMS);
    assert_eq!(children[0].ordinal, 0);
    assert_eq!(
        children[ITEMS - 1].ordinal,
        u32::try_from(ITEMS - 1).unwrap()
    );
    assert_eq!(
        children[ITEMS - 1].expression,
        ExpressionId::migrate(SEED, ITEMS as u64)
    );
    assert!(
        children
            .iter()
            .all(|child| child.role == ExpressionChildRole::SequenceItem)
    );
}

#[test]
fn owner_declared_lengths_and_framing_fail_without_count_based_reservation() {
    let expression = ExpressionId::migrate(SEED, 0);
    let module = ModuleId::migrate(SEED, 0);
    let module_header = OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module);
    let samples = [
        (
            raw_owner(&(9_u32, GRAPH_CONTRACT_VERSION, expression, 9_u32, u64::MAX)),
            OwnerKey::Expression(expression),
            OwnerKind::Expression,
        ),
        (
            // A complete nested effect-row header followed by an impossible
            // requirement inventory; the outer call vector has one element.
            raw_owner(&(
                9_u32,
                GRAPH_CONTRACT_VERSION,
                expression,
                10_u32,
                0_usize,
                1_usize,
                u64::MAX,
            )),
            OwnerKey::Expression(expression),
            OwnerKind::Expression,
        ),
        (
            raw_owner(&(0_u32, module_header, u64::MAX)),
            OwnerKey::Module(module),
            OwnerKind::Module,
        ),
        (
            raw_owner(&(0_u32, module_header, 1_usize, [0xff_u8])),
            OwnerKey::Module(module),
            OwnerKind::Module,
        ),
        (
            raw_owner(&(
                9_u32,
                GRAPH_CONTRACT_VERSION,
                expression,
                9_u32,
                2_usize,
                expression,
            )),
            OwnerKey::Expression(expression),
            OwnerKind::Expression,
        ),
    ];
    for (bytes, key, kind) in samples {
        let error = decode_owner(&bytes, key, kind, OwnerObjectDigest::of(&bytes)).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Corrupt);
        assert_eq!(error.code, "packed_decode");
    }
    let empty = raw_owner(&(9_u32, GRAPH_CONTRACT_VERSION, expression, 9_u32, 0_usize));
    assert_eq!(
        decode_owner(
            &empty,
            OwnerKey::Expression(expression),
            OwnerKind::Expression,
            OwnerObjectDigest::of(&empty),
        )
        .unwrap_err()
        .code,
        "kernel_expression_child_count"
    );
    let mut overflow = empty;
    overflow[10..18].copy_from_slice(&u64::MAX.to_le_bytes());
    let error = decode_owner(
        &overflow,
        OwnerKey::Expression(expression),
        OwnerKind::Expression,
        OwnerObjectDigest::of(&overflow),
    )
    .unwrap_err();
    assert_eq!(error.class, DiagnosticClass::Resource);
    assert!(matches!(
        error.code.as_str(),
        "packed_object_length" | "packed_payload_length"
    ));
}

#[test]
fn progressive_owner_decoder_preserves_all_expression_operation_encodings() {
    let first = ExpressionId::migrate(SEED, 1);
    let second = ExpressionId::migrate(SEED, 2);
    let third = ExpressionId::migrate(SEED, 3);
    let binding = BindingId::migrate(SEED, 4);
    let ty = TypeObjectDigest::of(b"linear-expression-type");
    let other = TypeObjectDigest::of(b"linear-other-type");
    let name = Name::new("value").unwrap();
    let field = FieldReference {
        package: declaration().package,
        field: FieldId::migrate(SEED, 0),
    };
    let case = CaseReference {
        package: declaration().package,
        case: CaseId::migrate(SEED, 0),
    };
    let requirement: RequirementOperand = RequirementReference {
        package: declaration().package,
        requirement: RequirementId::migrate(SEED, 0),
    }
    .into();
    let local = LocalValueReference::LexicalBinding(binding);
    let row = EffectRow {
        requirements: vec![requirement],
        parameters: Vec::new(),
    };
    let operations = [
        ExpressionOperation::Unit {},
        ExpressionOperation::Bool { value: true },
        ExpressionOperation::I64 { value: -42 },
        ExpressionOperation::Text {
            value: TextValue::Inline {
                text: "literal".to_owned(),
            },
        },
        ExpressionOperation::StaticText {
            value: TextValue::Blob {
                digest: BlobObjectDigest::of(b"blob"),
                bytes: 4,
            },
        },
        ExpressionOperation::Local { value: local },
        ExpressionOperation::Constant {
            declaration: declaration(),
        },
        ExpressionOperation::If {
            condition: first,
            when_true: second,
            when_false: third,
        },
        ExpressionOperation::Let {
            bindings: vec![binding],
            body: second,
        },
        ExpressionOperation::Sequence {
            items: vec![first, second, third],
        },
        ExpressionOperation::Call {
            requirement_arguments: vec![requirement],
            effect_arguments: vec![row.clone()],
            function: declaration(),
            type_arguments: vec![ty, other],
            read_scope_arguments: vec![scope()],
            arguments: vec![first, second],
        },
        ExpressionOperation::FunctionValue {
            requirement_arguments: vec![requirement],
            effect_arguments: vec![row.clone()],
            function: declaration(),
            type_arguments: vec![other, ty],
            read_scope_arguments: vec![scope()],
        },
        ExpressionOperation::Invoke {
            callee: first,
            arguments: vec![second, third],
        },
        ExpressionOperation::Record {
            nominal_type: None,
            type_arguments: Vec::new(),
            fields: vec![RecordExpressionField {
                selector: FieldSelector::Structural(name.clone()),
                value: first,
            }],
        },
        ExpressionOperation::Variant {
            case,
            type_arguments: vec![ty],
            payload: Some(first),
        },
        ExpressionOperation::Field {
            value: first,
            selector: FieldSelector::Nominal(field),
        },
        ExpressionOperation::List {
            item_type: ty,
            items: vec![first, second],
        },
        ExpressionOperation::Map {
            key_type: ty,
            value_type: other,
            entries: vec![MapExpressionEntry {
                key: first,
                value: second,
            }],
        },
        ExpressionOperation::Match {
            value: first,
            arms: vec![MatchExpressionArm {
                case,
                payload_binding: Some(binding),
                body: second,
            }],
        },
        ExpressionOperation::CapabilityCall {
            requirement,
            operation: OperationReference {
                package: declaration().package,
                operation: OperationId::migrate(SEED, 0),
            },
            arguments: vec![first, second],
        },
        ExpressionOperation::Transaction {
            requirement,
            binding,
            body: first,
        },
        ExpressionOperation::Bind {
            callee: first,
            arguments: vec![second, third],
        },
        ExpressionOperation::TransactionOutcome {
            requirement,
            binding,
            body: first,
            outcome: TransactionOutcomeContract::standard().unwrap(),
            type_argument: ty,
        },
        ExpressionOperation::F64 {
            value: crate::platform::binary64::Binary64::from_float(3.5),
        },
        ExpressionOperation::ImplementationCall {
            requirement_arguments: vec![requirement],
            effect_arguments: vec![row],
            function: declaration(),
            type_arguments: vec![ty, other],
            read_scope_arguments: vec![scope()],
            implementations: vec![operand()],
            arguments: vec![first, second],
        },
        ExpressionOperation::MethodCall {
            witness: operand(),
            contract: declaration(),
            method: MethodId::migrate(SEED, 0),
            read_scope_arguments: vec![scope()],
            arguments: vec![first, second],
        },
        ExpressionOperation::PackOwned {
            product_type: ty,
            fields: vec![OwnedProductExpressionField {
                name: name.clone(),
                value: first,
            }],
        },
        ExpressionOperation::UnpackOwned {
            product_type: ty,
            source: first,
            fields: vec![OwnedProductBinding {
                name: name.clone(),
                binding,
            }],
            body: second,
        },
        ExpressionOperation::ChooseOwned {
            choice_type: ty,
            case: name.clone(),
            value: first,
        },
        ExpressionOperation::MatchOwned {
            choice_type: ty,
            source: first,
            arms: vec![OwnedChoiceArm {
                name: name.clone(),
                binding,
                body: second,
            }],
        },
        ExpressionOperation::Parallel {
            left: first,
            right: second,
        },
        ExpressionOperation::BorrowOwnedField {
            product_type: ty,
            source: first,
            field: name.clone(),
            binding,
            body: second,
        },
        ExpressionOperation::MatchBorrowedOwned {
            choice_type: ty,
            source: first,
            arms: vec![OwnedChoiceArm {
                name,
                binding,
                body: second,
            }],
        },
        ExpressionOperation::SequenceEmpty { sequence_type: ty },
        ExpressionOperation::SequenceLength {
            sequence_type: ty,
            source: first,
        },
        ExpressionOperation::SequencePush {
            sequence_type: ty,
            value: first,
            source: second,
        },
        ExpressionOperation::SequencePop {
            sequence_type: ty,
            result_type: other,
            source: first,
        },
        ExpressionOperation::BorrowOwnedItem {
            sequence_type: ty,
            source: first,
            index: second,
            binding,
            body: third,
        },
        ExpressionOperation::BorrowCall {
            call: first,
            binding,
            body: second,
        },
        ExpressionOperation::SequenceGet {
            sequence_type: ty,
            source: first,
            index: second,
        },
        ExpressionOperation::SequenceReplace {
            sequence_type: ty,
            result_type: other,
            index: first,
            value: second,
            source: third,
        },
        ExpressionOperation::PackOwnedRecord {
            record_type: ty,
            fields: vec![OwnedRecordExpressionField {
                field,
                value: first,
            }],
        },
        ExpressionOperation::UnpackOwnedRecord {
            record_type: ty,
            source: first,
            fields: vec![OwnedRecordBinding { field, binding }],
            body: second,
        },
        ExpressionOperation::BorrowOwnedRecordField {
            record_type: ty,
            source: first,
            field,
            binding,
            body: second,
        },
        ExpressionOperation::CreateReadView {
            declaration: declaration(),
            type_arguments: vec![ty],
            read_scope: scope(),
            source: local,
            fields: vec![ReadViewFieldInitializer {
                field: field.field,
                value: first,
            }],
        },
        ExpressionOperation::BorrowReadView {
            value: local,
            binding,
            body: first,
        },
        ExpressionOperation::ReborrowReadView {
            value: local,
            read_scope: scope(),
        },
        ExpressionOperation::MoveReadView { value: local },
        ExpressionOperation::DropReadView { value: local },
        ExpressionOperation::ReadViewDataField {
            value: local,
            field: ReadViewFieldReference {
                declaration: declaration(),
                field: field.field,
            },
        },
        ExpressionOperation::BorrowReadViewField {
            value: local,
            field: ReadViewFieldReference {
                declaration: declaration(),
                field: field.field,
            },
            binding,
            body: second,
        },
        ExpressionOperation::Region {
            operation: RegionExpression::With {
                binding,
                capacity: first,
                body: second,
            },
        },
    ];
    assert_eq!(operations.len(), 52);
    for operation in operations {
        wire_round_trip(OwnerRecord::Expression(ExpressionRecord {
            contract_version: GRAPH_CONTRACT_VERSION,
            id: ExpressionId::migrate(SEED, 0),
            operation,
        }));
    }
}

#[test]
fn progressive_owner_decoder_preserves_nested_contract_and_http_encodings() {
    let ty = TypeObjectDigest::of(b"linear-nested-type");
    let parameter = ParameterId::migrate(SEED, 1);
    let scope_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 0),
        name: Name::new("scope").unwrap(),
    };
    let implementation_parameter = ImplementationParameter {
        id: ImplementationParameterId::migrate(SEED, 0),
        name: Name::new("witness").unwrap(),
        contract: declaration(),
        self_type: ty,
        type_arguments: vec![ty],
    };
    let method = MethodId::migrate(SEED, 0);
    let associated = AssociatedTypeMemberId::migrate(SEED, 0);
    let payloads = [
        DeclarationPayload::External(ExternalDeclaration {
            type_parameters: vec![TypeParameterId::migrate(SEED, 0)],
            parameters: vec![parameter],
            result: ty,
            implementation: ImplementationName::new("core.bytes.len").unwrap(),
        }),
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: vec![implementation_parameter.clone()],
            requirement_parameters: vec![RequirementParameterId::migrate(SEED, 0)],
            effect_parameters: vec![EffectParameterId::migrate(SEED, 0)],
            type_parameters: vec![TypeParameterId::migrate(SEED, 0)],
            parameters: vec![parameter],
            result: CallableResult {
                ty,
                relation: ResultRelation::ViewFrom {
                    source: parameter,
                    scope: scope(),
                },
            },
            read_scope_parameters: vec![scope_parameter.clone()],
            read_scope_sources: vec![ReadScopeSource {
                source: parameter,
                scope: scope(),
            }],
            effect: FunctionEffect::Task {
                requirements: Vec::new(),
                effect_parameters: Vec::new(),
            },
            body: ExpressionId::migrate(SEED, 1),
        }),
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: TypeParameterId::migrate(SEED, 0),
            type_parameters: vec![TypeParameterId::migrate(SEED, 1)],
            methods: vec![OwnedMethod {
                id: method,
                name: Name::new("read").unwrap(),
                parameters: vec![OwnedMethodParameter {
                    ty,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty,
                    relation: ResultRelation::ReadFrom { source: 0 },
                },
                read_scope_parameters: vec![scope_parameter.clone()],
                read_scope_sources: vec![ReadScopeSource {
                    source: 0,
                    scope: scope(),
                }],
                effect: FunctionEffect::Pure,
            }],
            associated_types: vec![AssociatedTypeMember {
                id: associated,
                name: Name::new("View").unwrap(),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: scope_parameter.clone(),
                },
            }],
        }),
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![TypeParameterId::migrate(SEED, 0)],
            implementation_parameters: vec![implementation_parameter],
            contract: declaration(),
            self_type: ty,
            type_arguments: vec![ty],
            methods: vec![OwnedMethodImplementation {
                method,
                function: declaration(),
                type_arguments: vec![ty],
                implementations: vec![operand()],
                read_scope_arguments: vec![scope()],
            }],
            associated_types: vec![AssociatedTypeBinding {
                member: associated,
                value: AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter: scope_parameter.clone(),
                    ty,
                },
            }],
        }),
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: vec![TypeParameterId::migrate(SEED, 0)],
            scope_parameter,
            fields: vec![ReadViewField {
                id: FieldId::migrate(SEED, 0),
                name: Name::new("field").unwrap(),
                ty,
                kind: ReadViewFieldKind::ReadReference {
                    read_scope: scope(),
                },
            }],
        }),
    ];
    for payload in payloads {
        wire_round_trip(OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(declaration().declaration),
                OwnerKind::PureFunction,
            ),
            module: ModuleId::migrate(SEED, 0),
            name: Name::new("declaration").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload,
        }));
    }
    for selector in [
        HttpRouteSelector::exact("/exact").unwrap(),
        HttpRouteSelector::parse_pattern("/records/{record}").unwrap(),
    ] {
        wire_round_trip(OwnerRecord::HttpRoute(HttpRouteRecord {
            header: OwnerHeader::new(
                OwnerKey::HttpRoute(HttpRouteId::migrate(SEED, 0)),
                OwnerKind::HttpRoute,
            ),
            target: TargetId::migrate(SEED, 0),
            method: "GET".to_owned(),
            selector,
            port: PortReference {
                package: declaration().package,
                port: PortId::migrate(SEED, 0),
            },
        }));
    }
}
