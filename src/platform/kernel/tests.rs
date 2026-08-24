//! Focused normalized-kernel contract tests.

use super::*;
use crate::platform::package::RunnerKind;
use crate::platform::persistent_map::{MapRoot, PageDigest};
use crate::platform::semantic_id::{
    AnnotationId, BindingId, CaseId, DeclarationId, DocumentationId, ExpressionId, FieldId,
    ModuleId, OperationId, ParameterId, PortId, RepositoryId, RequirementId, RevisionId, TargetId,
    TypeParameterId,
};
use std::collections::BTreeMap;

const TEST_SEED: &[u8] = b"graph-5-normalization-prototype";

#[derive(Clone, Copy)]
struct FixtureIds {
    second_module: ModuleId,
    callee: DeclarationId,
    caller: DeclarationId,
    binding_function: DeclarationId,
    field: FieldId,
    case: CaseId,
    operation: OperationId,
    parameter: ParameterId,
    binding: BindingId,
    requirement: RequirementId,
    call_expression: ExpressionId,
    test_actual: ExpressionId,
    caller_root: ExpressionId,
    record_expression: ExpressionId,
    variant_expression: ExpressionId,
    capability_expression: ExpressionId,
    parameter_expression: ExpressionId,
    binding_expression: ExpressionId,
}

fn name(value: &str) -> Name {
    Name::new(value).expect("test names are canonical")
}

fn map_root(entries: usize, marker: u8) -> MapRoot {
    MapRoot::from_parts(PageDigest::from_bytes([marker; 32]), entries as u64)
}

fn insert(owners: &mut BTreeMap<OwnerKey, OwnerRecord>, record: OwnerRecord) {
    let previous = owners.insert(record.owner(), record);
    assert!(previous.is_none());
}

fn expression(
    owners: &mut BTreeMap<OwnerKey, OwnerRecord>,
    ordinal: u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(TEST_SEED, ordinal);
    insert(
        owners,
        OwnerRecord::Expression(
            ExpressionRecord::new(id, operation).expect("test expression must be locally valid"),
        ),
    );
    id
}

fn prototype_snapshot() -> (KernelSnapshot, FixtureIds) {
    let package = PackageId::migrate(TEST_SEED, 0);
    let first_module = ModuleId::migrate(TEST_SEED, 0);
    let second_module = ModuleId::migrate(TEST_SEED, 1);
    let record = DeclarationId::migrate(TEST_SEED, 0);
    let variant = DeclarationId::migrate(TEST_SEED, 1);
    let interface = DeclarationId::migrate(TEST_SEED, 2);
    let component = DeclarationId::migrate(TEST_SEED, 3);
    let callee = DeclarationId::migrate(TEST_SEED, 4);
    let caller = DeclarationId::migrate(TEST_SEED, 5);
    let binding_function = DeclarationId::migrate(TEST_SEED, 6);
    let test = DeclarationId::migrate(TEST_SEED, 7);
    let external = DeclarationId::migrate(TEST_SEED, 8);
    let constant = DeclarationId::migrate(TEST_SEED, 9);
    let field = FieldId::migrate(TEST_SEED, 0);
    let case = CaseId::migrate(TEST_SEED, 0);
    let operation = OperationId::migrate(TEST_SEED, 0);
    let parameter = ParameterId::migrate(TEST_SEED, 0);
    let binding = BindingId::migrate(TEST_SEED, 0);
    let requirement = RequirementId::migrate(TEST_SEED, 0);
    let port = PortId::migrate(TEST_SEED, 0);
    let target = TargetId::migrate(TEST_SEED, 0);
    let type_parameter = TypeParameterId::migrate(TEST_SEED, 0);
    let documentation = DocumentationId::migrate(TEST_SEED, 0);
    let annotation = AnnotationId::migrate(TEST_SEED, 0);

    let mut interner = TypeObjectInterner::default();
    let unit_type = interner
        .intern(TypeForm::Unit)
        .expect("unit type must intern");
    let function_type = interner
        .intern(TypeForm::Function {
            parameters: Vec::new(),
            result: unit_type,
        })
        .expect("function type must intern");

    let mut owners = BTreeMap::new();
    insert(
        &mut owners,
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(first_module), OwnerKind::Module),
            name: name("first"),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(second_module), OwnerKind::Module),
            name: name("second"),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
            declaration: record,
            name: name("value"),
            ty: unit_type,
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(record), OwnerKind::Record),
            module: first_module,
            name: name("Payload"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Record {
                fields: vec![field],
            },
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Case(CaseRecord {
            header: OwnerHeader::new(OwnerKey::Case(case), OwnerKind::Case),
            declaration: variant,
            name: name("Ready"),
            payload: None,
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(variant), OwnerKind::Variant),
            module: first_module,
            name: name("State"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Variant { cases: vec![case] },
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Operation(OperationRecord {
            header: OwnerHeader::new(OwnerKey::Operation(operation), OwnerKind::Operation),
            declaration: interface,
            name: name("read"),
            parameters: Vec::new(),
            result: unit_type,
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(interface), OwnerKind::Interface),
            module: first_module,
            name: name("Store"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Interface {
                operations: vec![operation],
            },
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Requirement(RequirementRecord {
            header: OwnerHeader::new(OwnerKey::Requirement(requirement), OwnerKind::Requirement),
            declaration: component,
            name: name("store"),
            interface: DeclarationReference {
                package,
                declaration: interface,
            },
            operations: vec![OperationReference { package, operation }],
            limits: Vec::new(),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Port(PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component,
            name: name("run"),
            function_type,
            implementation: PortImplementation::Function(DeclarationReference {
                package,
                declaration: caller,
            }),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(component), OwnerKind::Component),
            module: first_module,
            name: name("Application"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Component {
                requirements: vec![requirement],
                ports: vec![port],
            },
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Target(TargetRecord {
            header: OwnerHeader::new(OwnerKey::Target(target), OwnerKind::Target),
            name: name("command"),
            component: DeclarationReference {
                package,
                declaration: component,
            },
            port: PortReference { package, port },
            runner: RunnerKind::Command,
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::TypeParameter(type_parameter),
                OwnerKind::TypeParameter,
            ),
            declaration: external,
            name: name("T"),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(external), OwnerKind::External),
            module: second_module,
            name: name("identity_external"),
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::External(ExternalDeclaration {
                type_parameters: vec![type_parameter],
                parameters: Vec::new(),
                result: unit_type,
                implementation: name("identity_host"),
            }),
        }),
    );
    let constant_value = expression(&mut owners, 19, ExpressionOperation::Unit);
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(constant), OwnerKind::Constant),
            module: second_module,
            name: name("unit_constant"),
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::Constant {
                ty: unit_type,
                value: constant_value,
            },
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Documentation(DocumentationRecord {
            header: OwnerHeader::new(
                OwnerKey::Documentation(documentation),
                OwnerKind::Documentation,
            ),
            owner: OwnerKey::Declaration(caller),
            class: DocumentationClass::Nonsemantic,
            content: DocumentContent::Inline("prototype caller".to_owned()),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Annotation(AnnotationRecord {
            header: OwnerHeader::new(OwnerKey::Annotation(annotation), OwnerKind::Annotation),
            owner: OwnerKey::Declaration(caller),
            class: AnnotationClass::Nonsemantic,
            key: name("review_group"),
            value: AnnotationValue::Name(name("prototype")),
        }),
    );

    let parameter_expression = expression(
        &mut owners,
        0,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    insert(
        &mut owners,
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(callee),
            name: name("input"),
            ty: unit_type,
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(callee), OwnerKind::PureFunction),
            module: first_module,
            name: name("callee"),
            visibility: DeclarationVisibility::Package,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: vec![parameter],
                result: unit_type,
                effect: FunctionEffect::Pure,
                body: parameter_expression,
            }),
        }),
    );

    let call_argument = expression(&mut owners, 1, ExpressionOperation::Unit);
    let call = expression(
        &mut owners,
        2,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package,
                declaration: callee,
            },
            type_arguments: Vec::new(),
            arguments: vec![call_argument],
        },
    );
    let record_value = expression(&mut owners, 3, ExpressionOperation::Unit);
    let record_expression = expression(
        &mut owners,
        4,
        ExpressionOperation::Record {
            nominal_type: Some(DeclarationReference {
                package,
                declaration: record,
            }),
            fields: vec![RecordExpressionField {
                selector: FieldSelector::Nominal(FieldReference { package, field }),
                value: record_value,
            }],
        },
    );
    let field_value = expression(&mut owners, 5, ExpressionOperation::Unit);
    let field_record = expression(
        &mut owners,
        6,
        ExpressionOperation::Record {
            nominal_type: Some(DeclarationReference {
                package,
                declaration: record,
            }),
            fields: vec![RecordExpressionField {
                selector: FieldSelector::Nominal(FieldReference { package, field }),
                value: field_value,
            }],
        },
    );
    let field_access = expression(
        &mut owners,
        7,
        ExpressionOperation::Field {
            value: field_record,
            selector: FieldSelector::Nominal(FieldReference { package, field }),
        },
    );
    let variant_expression = expression(
        &mut owners,
        8,
        ExpressionOperation::Variant {
            case: CaseReference { package, case },
            payload: None,
        },
    );
    let match_value = expression(
        &mut owners,
        9,
        ExpressionOperation::Variant {
            case: CaseReference { package, case },
            payload: None,
        },
    );
    let match_body = expression(&mut owners, 10, ExpressionOperation::Unit);
    let match_expression = expression(
        &mut owners,
        11,
        ExpressionOperation::Match {
            value: match_value,
            arms: vec![MatchExpressionArm {
                case: CaseReference { package, case },
                payload_binding: None,
                body: match_body,
            }],
        },
    );
    let capability_expression = expression(
        &mut owners,
        12,
        ExpressionOperation::CapabilityCall {
            requirement: RequirementReference {
                package,
                requirement,
            },
            operation: OperationReference { package, operation },
            arguments: Vec::new(),
        },
    );
    let caller_root = expression(
        &mut owners,
        13,
        ExpressionOperation::Sequence {
            items: vec![
                call,
                record_expression,
                field_access,
                variant_expression,
                match_expression,
                capability_expression,
            ],
        },
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(caller), OwnerKind::TaskFunction),
            module: first_module,
            name: name("caller"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: unit_type,
                effect: FunctionEffect::Task {
                    requirements: vec![requirement],
                },
                body: caller_root,
            }),
        }),
    );

    let binding_value = expression(&mut owners, 14, ExpressionOperation::Unit);
    let binding_expression = expression(
        &mut owners,
        15,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let let_expression = expression(
        &mut owners,
        16,
        ExpressionOperation::Let {
            bindings: vec![binding],
            body: binding_expression,
        },
    );
    insert(
        &mut owners,
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("local"),
            kind: BindingKind::Let,
            value: Some(binding_value),
            declared_type: Some(unit_type),
        }),
    );
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(
                OwnerKey::Declaration(binding_function),
                OwnerKind::PureFunction,
            ),
            module: second_module,
            name: name("with_binding"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                parameters: Vec::new(),
                result: unit_type,
                effect: FunctionEffect::Pure,
                body: let_expression,
            }),
        }),
    );

    let test_actual = expression(
        &mut owners,
        17,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package,
                declaration: binding_function,
            },
            type_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    let test_expected = expression(&mut owners, 18, ExpressionOperation::Unit);
    insert(
        &mut owners,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(test), OwnerKind::Test),
            module: second_module,
            name: name("caller_test"),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Test {
                actual: test_actual,
                expected: test_expected,
                comparison: ComparisonPolicy::Exact,
            },
        }),
    );

    let types = interner.into_objects();
    let root = SemanticRoot {
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        repository_id: RepositoryId::migrate(TEST_SEED, 0),
        package_id: package,
        package_name: name("prototype"),
        owners: map_root(owners.len(), 1),
        dependencies: map_root(0, 2),
        retirements: map_root(0, 3),
    };
    (
        KernelSnapshot {
            root,
            owners,
            types,
            blobs: BTreeMap::new(),
            dependencies: BTreeMap::new(),
            retirements: BTreeMap::new(),
        },
        FixtureIds {
            second_module,
            callee,
            caller,
            binding_function,
            field,
            case,
            operation,
            parameter,
            binding,
            requirement,
            call_expression: call,
            test_actual,
            caller_root,
            record_expression,
            variant_expression,
            capability_expression,
            parameter_expression,
            binding_expression,
        },
    )
}

pub(crate) fn witness_snapshot() -> KernelSnapshot {
    prototype_snapshot().0
}

fn encoded_owner(snapshot: &KernelSnapshot, owner: OwnerKey) -> (OwnerObjectDigest, Vec<u8>) {
    encode_owner(snapshot.owners.get(&owner).expect("test owner must exist"))
        .expect("test owner must encode")
}

fn owner_digests(snapshot: &KernelSnapshot) -> BTreeMap<OwnerKey, OwnerObjectDigest> {
    snapshot
        .owners
        .keys()
        .map(|owner| (*owner, encoded_owner(snapshot, *owner).0))
        .collect()
}

#[test]
fn normalized_prototype_passes_full_oracle() {
    let (snapshot, _) = prototype_snapshot();
    let report = validate_full(&snapshot).expect("prototype must pass the full oracle");
    assert_eq!(report.owners_checked, 43);
    assert_eq!(report.type_objects_checked, 2);
    assert_eq!(report.expression_records_checked, 20);
    assert_eq!(report.relation_edges, 62);
    assert!(report.work_consumed < 1_000);
}

#[test]
fn graph_five_permits_a_structurally_empty_package() {
    let snapshot = KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(TEST_SEED, 80),
            package_id: PackageId::migrate(TEST_SEED, 80),
            package_name: name("empty"),
            owners: map_root(0, 81),
            dependencies: map_root(0, 82),
            retirements: map_root(0, 83),
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    };
    let report = validate_full(&snapshot).expect("empty package is valid Graph 5 authority");
    assert_eq!(report.owners_checked, 0);
    assert_eq!(report.relation_edges, 0);
}

#[test]
fn declaration_rename_and_move_leave_exact_caller_bytes_unchanged() {
    let (mut snapshot, ids) = prototype_snapshot();
    let owners_before = owner_digests(&snapshot);
    let caller_before = encoded_owner(&snapshot, OwnerKey::Expression(ids.caller_root));
    let callee_before = encoded_owner(&snapshot, OwnerKey::Declaration(ids.callee));
    let Some(OwnerRecord::Declaration(callee)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(ids.callee))
    else {
        panic!("callee declaration must exist");
    };
    callee.name = name("renamed");
    callee.module = ids.second_module;
    let caller_after = encoded_owner(&snapshot, OwnerKey::Expression(ids.caller_root));
    let callee_after = encoded_owner(&snapshot, OwnerKey::Declaration(ids.callee));
    assert_eq!(caller_before, caller_after);
    assert_ne!(callee_before, callee_after);
    let owners_after = owner_digests(&snapshot);
    let changed = owners_before
        .iter()
        .filter_map(|(owner, digest)| (owners_after.get(owner) != Some(digest)).then_some(*owner))
        .collect::<Vec<_>>();
    assert_eq!(changed, vec![OwnerKey::Declaration(ids.callee)]);
    validate_full(&snapshot).expect("moved and renamed declaration remains valid");
}

#[test]
fn normalization_relation_footprints_ignore_names_and_track_only_current_ownership() {
    let (mut snapshot, ids) = prototype_snapshot();
    let before = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .expect("base relations must extract");
    let Some(OwnerRecord::Declaration(callee)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(ids.callee))
    else {
        panic!("callee declaration must exist");
    };
    callee.name = name("renamed");
    let renamed = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .expect("renamed relations must extract");
    assert_eq!(before, renamed);

    let Some(OwnerRecord::Declaration(callee)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(ids.callee))
    else {
        panic!("callee declaration must exist");
    };
    callee.module = ids.second_module;
    let moved = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .expect("moved relations must extract");
    let removed = before
        .iter()
        .filter(|edge| !moved.contains(edge))
        .collect::<Vec<_>>();
    let added = moved
        .iter()
        .filter(|edge| !before.contains(edge))
        .collect::<Vec<_>>();
    assert_eq!(removed.len(), 1);
    assert_eq!(added.len(), 1);
    assert_eq!(removed[0].kind, RelationKind::DeclarationModule);
    assert_eq!(added[0].kind, RelationKind::DeclarationModule);
}

#[test]
fn match_relations_bind_cases_for_queries_and_variant_shape_for_exhaustiveness() {
    let (snapshot, ids) = prototype_snapshot();
    let match_expression = snapshot
        .owners
        .iter()
        .find_map(|(owner, record)| match record {
            OwnerRecord::Expression(expression)
                if matches!(expression.operation, ExpressionOperation::Match { .. }) =>
            {
                Some(*owner)
            }
            _ => None,
        })
        .expect("fixture match expression");
    let OwnerRecord::Case(case) = &snapshot.owners[&OwnerKey::Case(ids.case)] else {
        panic!("fixture case must remain a case record")
    };
    let source = RelationEndpoint::Owner(ExactOwnerKey {
        package: snapshot.root.package_id,
        owner: match_expression,
    });
    let relations = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .expect("relations must extract");
    assert!(relations.iter().any(|edge| {
        edge.source == source
            && edge.kind == RelationKind::VariantMatch
            && edge.target
                == RelationEndpoint::Owner(ExactOwnerKey {
                    package: snapshot.root.package_id,
                    owner: OwnerKey::Case(ids.case),
                })
    }));
    assert!(relations.iter().any(|edge| {
        edge.source == source
            && edge.kind == RelationKind::VariantExhaustiveness
            && edge.target
                == RelationEndpoint::Owner(ExactOwnerKey {
                    package: snapshot.root.package_id,
                    owner: OwnerKey::Declaration(case.declaration),
                })
    }));
}

#[test]
fn member_and_binding_renames_leave_exact_uses_unchanged() {
    let (mut snapshot, ids) = prototype_snapshot();
    let selected = [
        OwnerKey::Expression(ids.record_expression),
        OwnerKey::Expression(ids.variant_expression),
        OwnerKey::Expression(ids.capability_expression),
        OwnerKey::Expression(ids.parameter_expression),
        OwnerKey::Expression(ids.binding_expression),
    ];
    let before = selected
        .iter()
        .map(|owner| encoded_owner(&snapshot, *owner))
        .collect::<Vec<_>>();
    for (owner, renamed) in [
        (OwnerKey::Field(ids.field), "renamed_field"),
        (OwnerKey::Case(ids.case), "RenamedCase"),
        (OwnerKey::Operation(ids.operation), "renamed_operation"),
        (OwnerKey::Parameter(ids.parameter), "renamed_parameter"),
        (OwnerKey::Binding(ids.binding), "renamed_binding"),
        (
            OwnerKey::Requirement(ids.requirement),
            "renamed_requirement",
        ),
    ] {
        match snapshot
            .owners
            .get_mut(&owner)
            .expect("renamed owner must exist")
        {
            OwnerRecord::Field(record) => record.name = name(renamed),
            OwnerRecord::Case(record) => record.name = name(renamed),
            OwnerRecord::Operation(record) => record.name = name(renamed),
            OwnerRecord::Parameter(record) => record.name = name(renamed),
            OwnerRecord::Binding(record) => record.name = name(renamed),
            OwnerRecord::Requirement(record) => record.name = name(renamed),
            _ => panic!("test selected an owner without a rename field"),
        }
    }
    let after = selected
        .iter()
        .map(|owner| encoded_owner(&snapshot, *owner))
        .collect::<Vec<_>>();
    assert_eq!(before, after);
    validate_full(&snapshot).expect("member renames remain valid");
}

#[test]
fn named_type_digest_ignores_declaration_name_and_module() {
    let (mut snapshot, ids) = prototype_snapshot();
    let object = TypeObject::new(TypeForm::Named {
        declaration: DeclarationReference {
            package: snapshot.root.package_id,
            declaration: ids.callee,
        },
    })
    .expect("named type must be valid");
    let before = encode_type_object(&object).expect("named type must encode");
    let Some(OwnerRecord::Declaration(callee)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(ids.callee))
    else {
        panic!("callee declaration must exist");
    };
    callee.name = name("elsewhere");
    callee.module = ids.second_module;
    let after = encode_type_object(&object).expect("named type must encode");
    assert_eq!(before, after);
}

#[test]
fn type_interner_deduplicates_and_requires_exact_children() {
    let mut interner = TypeObjectInterner::default();
    let unit = interner.intern(TypeForm::Unit).expect("unit must intern");
    let equal = interner.intern(TypeForm::Unit).expect("unit must reuse");
    assert_eq!(unit, equal);
    let option = interner
        .intern(TypeForm::Option { item: unit })
        .expect("known child must intern");
    assert!(interner.get(option).is_some());
    let missing = TypeObjectDigest::from_bytes([91; 32]);
    let error = interner
        .intern(TypeForm::List { item: missing })
        .expect_err("unknown child must reject");
    assert_eq!(error.code, "kernel_type_child_missing");
}

#[test]
fn package_identity_has_a_distinct_binary_domain() {
    let parameter = crate::platform::semantic_id::TypeParameterId::migrate(TEST_SEED, 0);
    let bytes = bincode::encode_to_vec(parameter, bincode::config::standard())
        .expect("type parameter must encode");
    let decoded = bincode::decode_from_slice::<PackageId, _>(&bytes, bincode::config::standard());
    assert!(decoded.is_err());
    let package = PackageId::migrate(TEST_SEED, 0);
    assert!(package.to_string().starts_with("pkg_"));
}

#[test]
fn owner_kind_and_owner_key_tags_are_unique_and_frozen() {
    let tags = OwnerKind::ALL
        .into_iter()
        .map(OwnerKind::tag)
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(tags.len(), OwnerKind::ALL.len());
    assert_eq!(OwnerKind::Module.tag(), 1);
    assert_eq!(OwnerKind::Annotation.tag(), 22);

    let same_ordinal = 7;
    let module =
        EncodedOwnerKey::new(OwnerKey::Module(ModuleId::migrate(TEST_SEED, same_ordinal))).bytes();
    let declaration = EncodedOwnerKey::new(OwnerKey::Declaration(DeclarationId::migrate(
        TEST_SEED,
        same_ordinal,
    )))
    .bytes();
    assert_eq!(module[0], 1);
    assert_eq!(declaration[0], 2);
    assert_ne!(module, declaration);
    assert_eq!(
        EncodedOwnerKey::decode(&module).expect("module key must decode"),
        OwnerKey::Module(ModuleId::migrate(TEST_SEED, same_ordinal))
    );
    assert_eq!(
        EncodedOwnerKey::decode(&declaration).expect("declaration key must decode"),
        OwnerKey::Declaration(DeclarationId::migrate(TEST_SEED, same_ordinal))
    );
    let mut foreign = module;
    foreign[0] = 255;
    assert_eq!(
        EncodedOwnerKey::decode(&foreign)
            .expect_err("foreign owner domain must reject")
            .code,
        "kernel_owner_key_domain"
    );
    let mut zero = module;
    zero[1..].fill(0);
    assert_eq!(
        EncodedOwnerKey::decode(&zero)
            .expect_err("zero owner identity must reject")
            .code,
        "kernel_owner_key_zero"
    );
}

#[test]
fn owner_codec_rejects_wrong_key_and_predecessor_magic() {
    let (snapshot, _) = prototype_snapshot();
    let module = snapshot
        .owners
        .keys()
        .find_map(|owner| match owner {
            OwnerKey::Module(module) => Some(*module),
            _ => None,
        })
        .expect("prototype has a module");
    let (digest, bytes) = encoded_owner(&snapshot, OwnerKey::Module(module));
    let wrong = ModuleId::migrate(TEST_SEED, 99);
    let diagnostic = decode_owner(&bytes, OwnerKey::Module(wrong), OwnerKind::Module, digest)
        .expect_err("wrong owner key must reject");
    assert_eq!(diagnostic.code, "kernel_owner_key_mismatch");

    let mut predecessor = bytes;
    predecessor[..8].copy_from_slice(b"LKJMNG04");
    let predecessor_digest = OwnerObjectDigest::of(&predecessor);
    assert!(
        decode_owner(
            &predecessor,
            OwnerKey::Module(module),
            OwnerKind::Module,
            predecessor_digest,
        )
        .is_err()
    );
}

#[test]
fn canonical_kernel_codec_manifest_is_frozen() {
    let (snapshot, _) = prototype_snapshot();
    let mut hasher = blake3::Hasher::new_derive_key("lkjscript.kernel.codec-manifest.test.v1");
    hasher.update(contract::GRAPH_CONTRACT_IDENTITY.as_bytes());
    for (owner, record) in &snapshot.owners {
        hasher.update(&EncodedOwnerKey::new(*owner).bytes());
        let (_, bytes) = encode_owner(record).expect("owner must encode");
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    for (digest, object) in &snapshot.types {
        hasher.update(&digest.bytes());
        let (_, bytes) = encode_type_object(object).expect("type must encode");
        hasher.update(&(bytes.len() as u64).to_be_bytes());
        hasher.update(&bytes);
    }
    let (_, root) = encode_root(&snapshot.root).expect("root must encode");
    hasher.update(&(root.len() as u64).to_be_bytes());
    hasher.update(&root);
    assert_eq!(
        crate::platform::semantic_id::encode_hex(hasher.finalize().as_bytes()),
        "9d09b85eda7e5dc41d13bc80531cebb53ad1c7cddb112e139b3335d4e4f75a1b"
    );
}

#[test]
fn every_prototype_owner_and_root_round_trips_canonically() {
    let (snapshot, ids) = prototype_snapshot();
    for (owner, record) in &snapshot.owners {
        let (digest, bytes) = encode_owner(record).expect("owner must encode");
        let decoded = decode_owner(&bytes, *owner, record.kind(), digest)
            .expect("owner must decode canonically");
        assert_eq!(&decoded, record);
    }
    for (digest, object) in &snapshot.types {
        let (_, bytes) = encode_type_object(object).expect("type must encode");
        assert_eq!(
            decode_type_object(&bytes, *digest).expect("type must decode canonically"),
            *object
        );
    }
    let (root_digest, root_bytes) = encode_root(&snapshot.root).expect("root must encode");
    assert_eq!(
        decode_root(&root_bytes, root_digest).expect("root must decode canonically"),
        snapshot.root
    );

    let dependency_package = PackageId::migrate(TEST_SEED, 70);
    let dependency = DependencyRecord {
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        package: dependency_package,
        semantic_revision: RevisionId::from_digest([12; 32]),
        package_object: PackageObjectDigest::from_bytes([13; 32]),
    };
    let (dependency_digest, dependency_bytes) =
        encode_dependency(&dependency).expect("dependency must encode");
    assert_eq!(
        decode_dependency(&dependency_bytes, &dependency_package, dependency_digest,)
            .expect("dependency must decode canonically"),
        dependency
    );

    let retirement = RetirementRecord {
        graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
        owner: OwnerKey::Declaration(ids.callee),
        last_kind: OwnerKind::PureFunction,
        last_name: Some(name("callee")),
        last_parent: Some(OwnerKey::Module(ModuleId::migrate(TEST_SEED, 0))),
        last_live_revision: RevisionId::from_digest([14; 32]),
        deletion_change: ChangeDigest::from_bytes([15; 32]),
    };
    let (retirement_digest, retirement_bytes) =
        encode_retirement(&retirement).expect("retirement must encode");
    assert_eq!(
        decode_retirement(
            &retirement_bytes,
            OwnerKey::Declaration(ids.callee),
            retirement_digest,
        )
        .expect("retirement must decode canonically"),
        retirement
    );
}

#[test]
fn canonical_map_bindings_are_compact_strict_and_domain_checked() {
    let (snapshot, ids) = prototype_snapshot();
    let owner = OwnerKey::Declaration(ids.callee);
    let record = &snapshot.owners[&owner];
    let (owner_digest, _) = encode_owner(record).expect("owner must encode");
    let owner_binding = OwnerBinding {
        kind: record.kind(),
        object: owner_digest,
    };
    let owner_bytes = encode_owner_binding(&owner_binding);
    assert_eq!(owner_bytes.len(), OWNER_BINDING_BYTES);
    assert_eq!(
        decode_owner_binding(&owner_bytes, owner).expect("owner binding must decode"),
        owner_binding
    );
    assert!(decode_owner_binding(&owner_bytes, OwnerKey::Field(ids.field)).is_err());
    assert!(decode_owner_binding(&owner_bytes[..32], owner).is_err());

    let dependency = DependencyBinding {
        object: DependencyObjectDigest::from_bytes([31; 32]),
    };
    let dependency_bytes = encode_dependency_binding(&dependency);
    assert_eq!(dependency_bytes.len(), DEPENDENCY_BINDING_BYTES);
    assert_eq!(
        decode_dependency_binding(&dependency_bytes).expect("dependency binding must decode"),
        dependency
    );
    assert!(decode_dependency_binding(&dependency_bytes[..31]).is_err());

    let retirement = RetirementBinding {
        object: RetirementObjectDigest::from_bytes([32; 32]),
    };
    let retirement_bytes = encode_retirement_binding(&retirement);
    assert_eq!(retirement_bytes.len(), RETIREMENT_BINDING_BYTES);
    assert_eq!(
        decode_retirement_binding(&retirement_bytes).expect("retirement binding must decode"),
        retirement
    );
    assert!(decode_retirement_binding(&retirement_bytes[..31]).is_err());
}

#[test]
fn subtree_replacement_can_preserve_selected_expression_identity() {
    let selected = ExpressionId::migrate(TEST_SEED, 100);
    let before = ExpressionRecord::new(selected, ExpressionOperation::Unit)
        .expect("unit expression must be valid");
    let after = ExpressionRecord::new(selected, ExpressionOperation::Bool { value: true })
        .expect("bool expression must be valid");
    assert_eq!(before.id, after.id);
    assert_ne!(before.operation, after.operation);
}

#[test]
fn full_oracle_rejects_expression_cycle_and_unreachable_record() {
    let (mut snapshot, ids) = prototype_snapshot();
    let Some(OwnerRecord::Expression(root)) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(ids.caller_root))
    else {
        panic!("caller root must exist");
    };
    root.operation = ExpressionOperation::Sequence {
        items: vec![ids.caller_root],
    };
    let unreachable = ExpressionId::migrate(TEST_SEED, 999);
    insert(
        &mut snapshot.owners,
        OwnerRecord::Expression(
            ExpressionRecord::new(unreachable, ExpressionOperation::Unit)
                .expect("unreachable expression remains locally valid"),
        ),
    );
    snapshot.root.owners = map_root(snapshot.owners.len(), 1);
    let diagnostics = validate_full(&snapshot).expect_err("invalid expression graph must reject");
    let codes = diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();
    assert!(codes.contains(&"kernel_full_expression_cycle"));
    assert!(codes.contains(&"kernel_full_expression_unreachable"));
}

#[test]
fn package_dependency_relation_has_a_package_endpoint() {
    let (mut snapshot, _) = prototype_snapshot();
    let dependency = PackageId::migrate(TEST_SEED, 44);
    snapshot.dependencies.insert(
        dependency,
        DependencyRecord {
            graph_contract_version: contract::GRAPH_CONTRACT_VERSION,
            package: dependency,
            semantic_revision: RevisionId::from_digest([7; 32]),
            package_object: PackageObjectDigest::from_bytes([8; 32]),
        },
    );
    snapshot.root.dependencies = map_root(1, 2);
    let relations = extract_relations(
        snapshot.root.package_id,
        &snapshot.owners,
        &snapshot.types,
        &snapshot.dependencies,
    )
    .expect("relations must extract");
    assert!(relations.iter().any(|edge| {
        edge.source == RelationEndpoint::Package(snapshot.root.package_id)
            && edge.kind == RelationKind::PackageDependency
            && edge.target == RelationEndpoint::Package(dependency)
    }));
}

#[test]
fn full_oracle_rejects_type_arity_and_effect_violations() {
    let (mut arity, ids) = prototype_snapshot();
    let Some(OwnerRecord::Expression(call)) = arity
        .owners
        .get_mut(&OwnerKey::Expression(ids.call_expression))
    else {
        panic!("call expression must exist");
    };
    let ExpressionOperation::Call { arguments, .. } = &mut call.operation else {
        panic!("selected expression must be a call");
    };
    let removed_argument = arguments[0];
    arguments.clear();
    arity.owners.remove(&OwnerKey::Expression(removed_argument));
    arity.root.owners = map_root(arity.owners.len(), 1);
    let diagnostics = validate_full(&arity).expect_err("wrong call arity must reject");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_call_arity")
    );

    let (mut effect, ids) = prototype_snapshot();
    let Some(OwnerRecord::Expression(actual)) = effect
        .owners
        .get_mut(&OwnerKey::Expression(ids.test_actual))
    else {
        panic!("test actual expression must exist");
    };
    let ExpressionOperation::Call { function, .. } = &mut actual.operation else {
        panic!("test actual expression must be a call");
    };
    function.declaration = ids.caller;
    let diagnostics = validate_full(&effect).expect_err("pure test cannot call a task");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_pure_task_call")
    );
}

#[test]
fn full_oracle_rejects_function_result_type_mismatch() {
    let (mut snapshot, ids) = prototype_snapshot();
    let Some(OwnerRecord::Expression(body)) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(ids.parameter_expression))
    else {
        panic!("callee body must exist");
    };
    body.operation = ExpressionOperation::Bool { value: true };
    let diagnostics = validate_full(&snapshot).expect_err("wrong body type must reject");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_type_root")
    );
}

#[test]
fn full_oracle_rejects_foreign_type_parameter_scope() {
    let (mut snapshot, ids) = prototype_snapshot();
    let parameter = crate::platform::semantic_id::TypeParameterId::migrate(TEST_SEED, 50);
    insert(
        &mut snapshot.owners,
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: ids.callee,
            name: name("T"),
        }),
    );
    let Some(OwnerRecord::Declaration(callee)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(ids.callee))
    else {
        panic!("callee must exist");
    };
    let DeclarationPayload::Function(callee) = &mut callee.payload else {
        panic!("callee must be a function");
    };
    callee.type_parameters.push(parameter);

    let type_object = TypeObject::new(TypeForm::TypeParameter { parameter })
        .expect("type parameter object must be locally valid");
    let (type_digest, _) =
        encode_type_object(&type_object).expect("type parameter object must encode");
    snapshot.types.insert(type_digest, type_object);
    let Some(OwnerRecord::Declaration(function)) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(ids.binding_function))
    else {
        panic!("binding function must exist");
    };
    let DeclarationPayload::Function(function) = &mut function.payload else {
        panic!("binding function must be a function");
    };
    function.result = type_digest;
    snapshot.root.owners = map_root(snapshot.owners.len(), 1);

    let diagnostics = validate_full(&snapshot).expect_err("foreign type parameter use must reject");
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "kernel_full_type_parameter_scope")
    );
}

#[test]
fn generic_call_substitutes_the_exact_type_parameter() {
    let (mut snapshot, ids) = prototype_snapshot();
    let parameter = TypeParameterId::migrate(TEST_SEED, 51);
    insert(
        &mut snapshot.owners,
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: ids.callee,
            name: name("Value"),
        }),
    );
    let type_object = TypeObject::new(TypeForm::TypeParameter { parameter })
        .expect("type parameter object must be valid");
    let (parameter_type, _) =
        encode_type_object(&type_object).expect("type parameter object must encode");
    snapshot.types.insert(parameter_type, type_object);

    let Some(OwnerRecord::Declaration(callee)) =
        snapshot.owners.get_mut(&OwnerKey::Declaration(ids.callee))
    else {
        panic!("callee must exist");
    };
    let DeclarationPayload::Function(callee) = &mut callee.payload else {
        panic!("callee must be a function");
    };
    callee.type_parameters.push(parameter);
    callee.result = parameter_type;
    let Some(OwnerRecord::Parameter(value)) =
        snapshot.owners.get_mut(&OwnerKey::Parameter(ids.parameter))
    else {
        panic!("callee parameter must exist");
    };
    value.ty = parameter_type;

    let unit_type = snapshot
        .types
        .iter()
        .find_map(|(digest, object)| matches!(object.form, TypeForm::Unit).then_some(*digest))
        .expect("prototype has unit type");
    let Some(OwnerRecord::Expression(call)) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(ids.call_expression))
    else {
        panic!("call expression must exist");
    };
    let ExpressionOperation::Call { type_arguments, .. } = &mut call.operation else {
        panic!("selected expression must be a call");
    };
    type_arguments.push(unit_type);
    snapshot.root.owners = map_root(snapshot.owners.len(), 1);
    validate_full(&snapshot).expect("exact generic substitution must validate");
}
