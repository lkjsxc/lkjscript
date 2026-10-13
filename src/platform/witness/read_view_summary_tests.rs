//! Source controls for summary material, separate from semantic admission controls.

use super::*;
use crate::platform::kernel::*;
use crate::platform::semantic_id::*;

const SEED: &[u8] = b"read-view-summary-controls";

fn reference(ordinal: u64) -> DeclarationReference {
    DeclarationReference {
        package: PackageId::migrate(SEED, 0),
        declaration: DeclarationId::migrate(SEED, ordinal),
    }
}

fn scope(owner: DeclarationReference, ordinal: u64) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(owner),
        parameter: ReadScopeParameterId::migrate(SEED, ordinal),
    })
}

fn scope_parameter(ordinal: u64) -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, ordinal),
        name: Name::new(format!("source{ordinal}")).unwrap(),
    }
}

fn scalar() -> TypeObjectDigest {
    encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
        .unwrap()
        .0
}

fn function_record() -> OwnerRecord {
    let owner = OwnerKey::Declaration(reference(0).declaration);
    OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(owner, OwnerKind::PureFunction),
        module: ModuleId::migrate(SEED, 0),
        name: Name::new("refine").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![ParameterId::migrate(SEED, 0), ParameterId::migrate(SEED, 1)],
            result: CallableResult {
                ty: scalar(),
                relation: ResultRelation::Value,
            },
            read_scope_parameters: vec![scope_parameter(0), scope_parameter(1)],
            read_scope_sources: vec![ReadScopeSource {
                source: ParameterId::migrate(SEED, 0),
                scope: scope(reference(0), 0),
            }],
            effect: FunctionEffect::Pure,
            body: ExpressionId::migrate(SEED, 0),
        }),
    })
}

fn function(record: &mut OwnerRecord) -> &mut FunctionDeclaration {
    let OwnerRecord::Declaration(record) = record else {
        panic!("declaration")
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function")
    };
    function
}

fn summarize(record: &OwnerRecord) -> WorkingSummary {
    local_summary(record.owner(), record, None).unwrap()
}

#[test]
fn current_signatures_bind_unused_scope_inventory_source_order_and_owner() {
    let mut record = function_record();
    let base = summarize(&record);
    function(&mut record).read_scope_parameters[1].name = Name::new("phantom").unwrap();
    let renamed_unused = summarize(&record);
    assert_ne!(base.semantic_interface, renamed_unused.semantic_interface);
    assert_eq!(base.implementation, renamed_unused.implementation);

    function(&mut record)
        .read_scope_sources
        .push(ReadScopeSource {
            source: ParameterId::migrate(SEED, 1),
            scope: scope(reference(0), 1),
        });
    let unused_source = summarize(&record);
    assert_ne!(
        renamed_unused.semantic_interface,
        unused_source.semantic_interface
    );
    function(&mut record).read_scope_sources.reverse();
    let reversed = summarize(&record);
    assert_ne!(
        unused_source.semantic_interface,
        reversed.semantic_interface
    );
    function(&mut record).read_scope_sources[0].scope = scope(reference(1), 1);
    let other_owner = summarize(&record);
    assert_ne!(reversed.semantic_interface, other_owner.semantic_interface);
    assert_eq!(base.presentation, other_owner.presentation);
}

#[test]
fn result_carrier_commits_kind_source_and_scope_without_body_changes() {
    let mut record = function_record();
    let ordinary = summarize(&record);
    function(&mut record).result.relation = ResultRelation::ReadFrom {
        source: ParameterId::migrate(SEED, 0),
    };
    let read = summarize(&record);
    function(&mut record).result.relation = ResultRelation::ViewFrom {
        source: ParameterId::migrate(SEED, 0),
        scope: scope(reference(0), 0),
    };
    let view = summarize(&record);
    function(&mut record).result.relation = ResultRelation::ViewFrom {
        source: ParameterId::migrate(SEED, 1),
        scope: scope(reference(0), 1),
    };
    let other = summarize(&record);
    assert_ne!(ordinary.semantic_interface, read.semantic_interface);
    assert_ne!(read.semantic_interface, view.semantic_interface);
    assert_ne!(view.semantic_interface, other.semantic_interface);
    assert_eq!(ordinary.implementation, other.implementation);
    assert_eq!(ordinary.type_digest, other.type_digest);
    assert_eq!(ordinary.presentation, other.presentation);
}

#[test]
fn exact_pre34_function_summary_material_remains_separate_from_current_result_shape() {
    let mut record = function_record();
    let f = function(&mut record);
    f.read_scope_parameters.clear();
    f.read_scope_sources.clear();
    f.result.relation = ResultRelation::ReadFrom {
        source: ParameterId::migrate(SEED, 0),
    };
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    d.header.contract_version = 33;
    let predecessor = summarize(&record);
    let mut literal = Material::new(OwnerKind::PureFunction);
    literal.piece(1, &DeclarationVisibility::Public).unwrap();
    literal.piece(7, &Vec::<EffectParameterId>::new()).unwrap();
    literal.raw_piece(2, &[5]);
    literal.piece(4, &Vec::<TypeParameterId>::new()).unwrap();
    literal
        .piece(
            5,
            &vec![ParameterId::migrate(SEED, 0), ParameterId::migrate(SEED, 1)],
        )
        .unwrap();
    literal.piece(6, &scalar()).unwrap();
    literal.piece(11, &ParameterId::migrate(SEED, 0)).unwrap();
    assert_eq!(
        predecessor.semantic_interface,
        literal.finish(INTERFACE_DIGEST_DOMAIN)
    );
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    d.header.contract_version = 34;
    let current = summarize(&record);
    assert_ne!(predecessor.semantic_interface, current.semantic_interface);
    assert_eq!(predecessor.implementation, current.implementation);
}

#[test]
fn inline_view_fields_bind_complete_kind_scope_and_phantom_type_inventory() {
    let owner = OwnerKey::Declaration(reference(2).declaration);
    let mut record = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(owner, OwnerKind::ReadView),
        module: ModuleId::migrate(SEED, 0),
        name: Name::new("Span").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: vec![TypeParameterId::migrate(SEED, 0)],
            scope_parameter: scope_parameter(0),
            fields: vec![ReadViewField {
                id: FieldId::migrate(SEED, 0),
                name: Name::new("payload").unwrap(),
                ty: scalar(),
                kind: ReadViewFieldKind::Ordinary,
            }],
        }),
    });
    let base = summarize(&record);
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(view) = &mut d.payload else {
        unreachable!()
    };
    view.fields[0].kind = ReadViewFieldKind::ReadReference {
        read_scope: scope(reference(2), 0),
    };
    let referenced = summarize(&record);
    assert_ne!(base.semantic_interface, referenced.semantic_interface);
    assert_eq!(base.type_digest, referenced.type_digest);
    let OwnerRecord::Declaration(d) = &mut record else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(view) = &mut d.payload else {
        unreachable!()
    };
    view.type_parameters.push(TypeParameterId::migrate(SEED, 1));
    let phantom = summarize(&record);
    assert_ne!(referenced.semantic_interface, phantom.semantic_interface);
    assert_eq!(aggregation_children(&record).unwrap().len(), 2);
    assert!(
        !aggregation_children(&record)
            .unwrap()
            .iter()
            .any(|(_, child)| matches!(child, OwnerKey::Field(_)))
    );
}

#[test]
fn scope_actuals_and_authored_view_field_order_contribute_to_expression_material() {
    let id = ExpressionId::migrate(SEED, 5);
    let call = |actuals| {
        OwnerRecord::Expression(
            ExpressionRecord::new(
                id,
                ExpressionOperation::Call {
                    function: reference(0),
                    type_arguments: Vec::new(),
                    requirement_arguments: Vec::new(),
                    effect_arguments: Vec::new(),
                    read_scope_arguments: actuals,
                    arguments: Vec::new(),
                },
            )
            .unwrap(),
        )
    };
    let first = summarize(&call(vec![scope(reference(0), 0), scope(reference(0), 1)]));
    let swapped = summarize(&call(vec![scope(reference(0), 1), scope(reference(0), 0)]));
    assert_ne!(first.implementation, swapped.implementation);
    assert_eq!(first.type_digest, swapped.type_digest);
    let mut view = ExpressionRecord::new(
        id,
        ExpressionOperation::CreateReadView {
            declaration: reference(2),
            type_arguments: vec![scalar()],
            read_scope: scope(reference(0), 0),
            source: LocalValueReference::FunctionParameter(ParameterId::migrate(SEED, 0)),
            fields: vec![
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 0),
                    value: ExpressionId::migrate(SEED, 6),
                },
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 1),
                    value: ExpressionId::migrate(SEED, 7),
                },
            ],
        },
    )
    .unwrap();
    let before = summarize(&OwnerRecord::Expression(view.clone()));
    let ExpressionOperation::CreateReadView { fields, .. } = &mut view.operation else {
        unreachable!()
    };
    fields.reverse();
    let after = summarize(&OwnerRecord::Expression(view.clone()));
    assert_ne!(before.implementation, after.implementation);
    assert_eq!(before.type_digest, after.type_digest);
    assert_eq!(
        view.children()
            .iter()
            .map(|child| child.expression)
            .collect::<Vec<_>>(),
        vec![
            ExpressionId::migrate(SEED, 7),
            ExpressionId::migrate(SEED, 6)
        ]
    );
}

#[test]
fn associated_family_and_unused_method_scope_mapping_are_never_erased() {
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let method_id = MethodId::migrate(SEED, 0);
    let method_scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::ContractMethod {
            contract: reference(3),
            method: method_id,
        },
        parameter: ReadScopeParameterId::migrate(SEED, 0),
    });
    let owner = OwnerKey::Declaration(reference(3).declaration);
    let mut contract = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(owner, OwnerKind::OwnedContract),
        module: ModuleId::migrate(SEED, 0),
        name: Name::new("RangeRead").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: TypeParameterId::migrate(SEED, 0),
            type_parameters: Vec::new(),
            methods: vec![OwnedMethod {
                id: method_id,
                name: Name::new("range").unwrap(),
                parameters: vec![OwnedMethodParameter {
                    ty: scalar(),
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: scalar(),
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![scope_parameter(0)],
                read_scope_sources: vec![ReadScopeSource {
                    source: 0,
                    scope: method_scope,
                }],
                effect: FunctionEffect::Pure,
            }],
            associated_types: vec![AssociatedTypeMember {
                id: member,
                name: Name::new("View").unwrap(),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: scope_parameter(2),
                },
            }],
        }),
    });
    let base = summarize(&contract);
    let OwnerRecord::Declaration(d) = &mut contract else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(c) = &mut d.payload else {
        unreachable!()
    };
    c.methods[0].result.relation = ResultRelation::ViewFrom {
        source: 0,
        scope: method_scope,
    };
    let returning = summarize(&contract);
    assert_ne!(base.semantic_interface, returning.semantic_interface);
    assert_eq!(base.implementation, returning.implementation);
    let OwnerRecord::Declaration(d) = &mut contract else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(c) = &mut d.payload else {
        unreachable!()
    };
    c.associated_types[0].bound = AssociatedTypeBound::ReadViewFamily {
        scope_parameter: scope_parameter(3),
    };
    assert_ne!(
        returning.semantic_interface,
        summarize(&contract).semantic_interface
    );

    let owner = OwnerKey::Declaration(reference(4).declaration);
    let mut implementation = OwnerRecord::Declaration(DeclarationRecord {
        header: OwnerHeader::new(owner, OwnerKind::OwnedImplementation),
        module: ModuleId::migrate(SEED, 0),
        name: Name::new("Flat").unwrap(),
        visibility: DeclarationVisibility::Public,
        payload: DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: Vec::new(),
            implementation_parameters: Vec::new(),
            contract: reference(3),
            self_type: scalar(),
            type_arguments: Vec::new(),
            methods: vec![OwnedMethodImplementation {
                method: method_id,
                function: reference(0),
                type_arguments: Vec::new(),
                implementations: Vec::new(),
                read_scope_arguments: vec![scope(reference(4), 0)],
            }],
            associated_types: vec![AssociatedTypeBinding {
                member,
                value: AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter: scope_parameter(2),
                    ty: scalar(),
                },
            }],
        }),
    });
    let selected = summarize(&implementation);
    let OwnerRecord::Declaration(d) = &mut implementation else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.methods[0].read_scope_arguments[0] = scope(reference(5), 0);
    let swapped = summarize(&implementation);
    assert_ne!(selected.semantic_interface, swapped.semantic_interface);
    assert_ne!(selected.implementation, swapped.implementation);
    assert_eq!(selected.type_digest, swapped.type_digest);
    let OwnerRecord::Declaration(d) = &mut implementation else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        unreachable!()
    };
    i.associated_types[0].value = AssociatedTypeBindingValue::ReadViewFamily {
        scope_parameter: scope_parameter(3),
        ty: scalar(),
    };
    let binder = summarize(&implementation);
    assert_ne!(swapped.semantic_interface, binder.semantic_interface);
    assert_ne!(swapped.implementation, binder.implementation);
}

#[test]
fn owning_view_initializer_expression_is_bound_in_implementation_summary() {
    let id = BindingId::migrate(SEED, 200);
    let ty = encode_type_object(
        &TypeObject::new(TypeForm::ReadViewApplied {
            declaration: reference(2),
            arguments: Vec::new(),
            read_scope: scope(reference(0), 0),
        })
        .unwrap(),
    )
    .unwrap()
    .0;
    let mut binding = BindingRecord {
        header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
        name: Name::new("view").unwrap(),
        kind: BindingKind::ReadView,
        value: Some(ExpressionId::migrate(SEED, 200)),
        declared_type: Some(ty),
    };
    let before = summarize(&OwnerRecord::Binding(binding.clone()));
    binding.value = Some(ExpressionId::migrate(SEED, 201));
    let after = summarize(&OwnerRecord::Binding(binding));
    assert_ne!(before.implementation, after.implementation);
    assert_eq!(before.semantic_interface, after.semantic_interface);
    assert_eq!(before.type_digest, after.type_digest);
}
