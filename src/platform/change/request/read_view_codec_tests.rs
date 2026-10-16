//! Literal commitment controls for scope authority and complete unused inventories.
use super::*;
use crate::platform::kernel::{AssociatedTypeBound, PackageId, ReadScopeParameter};
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, ImplementationParameterId, MethodId, ReadScopeParameterId,
};

fn declaration(ordinal: u64) -> AuthoredDeclarationReference {
    AuthoredDeclarationReference::Exact {
        package: PackageId::migrate(b"view-intent", 1),
        declaration: DeclarationId::migrate(b"view-intent", ordinal),
    }
}

fn formal(ordinal: u64, name: &str) -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(b"view-intent", ordinal),
        name: Name::new(name).unwrap(),
    }
}

fn scope(ordinal: u64) -> AuthoredReadScopeArgument {
    AuthoredReadScopeArgument::Parameter {
        binder: AuthoredReadScopeBinder::Declaration(declaration(3)),
        parameter: formal(ordinal, "loan").id,
    }
}

fn source(ordinal: u64) -> AuthoredLocalReference {
    AuthoredLocalReference::FunctionParameter {
        parameter: ParameterId::migrate(b"view-intent", ordinal),
    }
}

fn count(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_be_bytes());
}

fn exact(bytes: &mut Vec<u8>, ordinal: u64) {
    bytes.push(2);
    bytes.extend_from_slice(&PackageId::migrate(b"view-intent", 1).bytes());
    bytes.extend_from_slice(&DeclarationId::migrate(b"view-intent", ordinal).bytes());
}

fn literal_scope(bytes: &mut Vec<u8>, ordinal: u64) {
    bytes.extend_from_slice(&[1, 1]); // Parameter argument; declaration binder.
    exact(bytes, 3);
    bytes.extend_from_slice(&formal(ordinal, "loan").id.bytes());
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

fn encode_scope(argument: &AuthoredReadScopeArgument) -> Vec<u8> {
    let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
    writer
        .read_scope_argument(argument, &BTreeMap::new())
        .unwrap();
    writer.finish()
}

#[test]
fn scope_arguments_commit_all_binder_coordinates_and_distinguish_lexical_authority() {
    let method = MethodId::migrate(b"view-intent", 1);
    let member = AssociatedTypeMemberId::migrate(b"view-intent", 1);
    let binders = [
        AuthoredReadScopeBinder::Declaration(declaration(3)),
        AuthoredReadScopeBinder::ContractMethod {
            contract: declaration(3),
            method,
        },
        AuthoredReadScopeBinder::AssociatedMember {
            contract: declaration(3),
            member,
        },
        AuthoredReadScopeBinder::AssociatedBinding {
            implementation: declaration(3),
            member,
        },
    ];
    let mut distinct = BTreeSet::new();
    for (index, binder) in binders.into_iter().enumerate() {
        let argument = AuthoredReadScopeArgument::Parameter {
            binder,
            parameter: formal(1, "loan").id,
        };
        let mut literal = vec![1, index as u8 + 1];
        exact(&mut literal, 3);
        if index == 1 {
            literal.extend_from_slice(&method.bytes());
        }
        if index > 1 {
            literal.extend_from_slice(&member.bytes());
        }
        literal.extend_from_slice(&formal(1, "loan").id.bytes());
        assert_eq!(encode_scope(&argument), literal);
        assert!(distinct.insert(literal));
    }
    let lexical = AuthoredReadScopeArgument::Lexical {
        callable: declaration(3),
        binding: AuthoredLocalReference::LexicalBinding {
            binding: BindingId::migrate(b"view-intent", 1),
        },
    };
    let mut literal = vec![2];
    exact(&mut literal, 3);
    literal.push(3); // Exact lexical local.
    literal.extend_from_slice(&BindingId::migrate(b"view-intent", 1).bytes());
    assert_eq!(encode_scope(&lexical), literal);
    assert!(distinct.insert(literal));
    let original = encode_scope(&scope(1));
    assert_ne!(original, encode_scope(&scope(2)));
    assert_ne!(
        original,
        encode_scope(&AuthoredReadScopeArgument::Parameter {
            binder: AuthoredReadScopeBinder::Declaration(declaration(4)),
            parameter: formal(1, "loan").id,
        })
    );
    assert_ne!(
        encode_scope(&lexical),
        encode_scope(&AuthoredReadScopeArgument::Lexical {
            callable: declaration(4),
            binding: AuthoredLocalReference::LexicalBinding {
                binding: BindingId::migrate(b"view-intent", 1)
            },
        })
    );
    assert_ne!(
        encode_scope(&lexical),
        encode_scope(&AuthoredReadScopeArgument::Lexical {
            callable: declaration(3),
            binding: AuthoredLocalReference::LexicalBinding {
                binding: BindingId::migrate(b"view-intent", 2)
            },
        })
    );
}

#[test]
fn value_result_still_commits_unused_scope_formals_sources_and_their_order() {
    let change = AuthoredChange::SetFunctionContract {
        function: DeclarationSelector::Id {
            declaration: DeclarationId::migrate(b"view-intent", 3),
        },
        result: AuthoredCallableResult::value(AuthoredType::Unit {}),
        read_scope_parameters: vec![formal(1, "loan")],
        read_scope_sources: vec![AuthoredReadScopeSource {
            source: source(1),
            scope: scope(1),
        }],
        effect: AuthoredFunctionEffect::Pure {},
    };
    let mut literal = b"LKJACHNG".to_vec();
    literal.extend_from_slice(&SCHEMA_EPOCH.to_le_bytes());
    literal.extend_from_slice(&[9; 32]);
    count(&mut literal, 0);
    count(&mut literal, 1);
    literal.extend_from_slice(&[21, 1]); // Set function contract; exact declaration selector.
    literal.extend_from_slice(&DeclarationId::migrate(b"view-intent", 3).bytes());
    literal.extend_from_slice(&[1, 1]); // Unit result; Value relation.
    count(&mut literal, 1);
    literal.extend_from_slice(&formal(1, "loan").id.bytes());
    count(&mut literal, 4);
    literal.extend_from_slice(b"loan");
    count(&mut literal, 1);
    literal.push(1); // Exact function source parameter.
    literal.extend_from_slice(&ParameterId::migrate(b"view-intent", 1).bytes());
    literal_scope(&mut literal, 1);
    literal.push(1); // Pure effect.
    assert_eq!(encode(change.clone()), literal);
    let mut mutations = vec![change.clone(); 4];
    if let AuthoredChange::SetFunctionContract {
        read_scope_parameters,
        ..
    } = &mut mutations[0]
    {
        read_scope_parameters[0].id = formal(2, "loan").id;
    }
    if let AuthoredChange::SetFunctionContract {
        read_scope_parameters,
        ..
    } = &mut mutations[1]
    {
        read_scope_parameters[0].name = Name::new("unused").unwrap();
    }
    if let AuthoredChange::SetFunctionContract {
        read_scope_sources, ..
    } = &mut mutations[2]
    {
        read_scope_sources[0].source = source(2);
    }
    if let AuthoredChange::SetFunctionContract {
        read_scope_sources, ..
    } = &mut mutations[3]
    {
        read_scope_sources[0].scope = scope(2);
    }
    for mutation in mutations {
        assert_ne!(literal, encode(mutation));
    }
    let mut ordered = change;
    let AuthoredChange::SetFunctionContract {
        read_scope_parameters,
        read_scope_sources,
        ..
    } = &mut ordered
    else {
        unreachable!()
    };
    read_scope_parameters.push(formal(2, "other"));
    read_scope_sources.push(AuthoredReadScopeSource {
        source: source(2),
        scope: scope(2),
    });
    let original = encode(ordered.clone());
    let mut reversed_formals = ordered.clone();
    let AuthoredChange::SetFunctionContract {
        read_scope_parameters,
        ..
    } = &mut reversed_formals
    else {
        unreachable!()
    };
    read_scope_parameters.reverse();
    let AuthoredChange::SetFunctionContract {
        read_scope_sources, ..
    } = &mut ordered
    else {
        unreachable!()
    };
    read_scope_sources.reverse();
    assert_ne!(original, encode(reversed_formals));
    assert_ne!(original, encode(ordered));
}

#[test]
fn result_relations_and_associated_read_view_coordinates_have_literal_layouts() {
    for relation in [
        AuthoredResultRelation::Value,
        AuthoredResultRelation::ReadFrom { source: 7_u32 },
        AuthoredResultRelation::ViewFrom {
            source: 7_u32,
            scope: scope(1),
        },
    ] {
        let mut literal = vec![3]; // I64 result.
        match &relation {
            AuthoredResultRelation::Value => literal.push(1),
            AuthoredResultRelation::ReadFrom { .. } => {
                literal.push(2);
                count(&mut literal, 7);
            }
            AuthoredResultRelation::ViewFrom { .. } => {
                literal.push(3);
                count(&mut literal, 7);
                literal_scope(&mut literal, 1);
            }
        }
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer
            .callable_result(
                &AuthoredCallableResult {
                    ty: AuthoredType::I64 {},
                    relation,
                },
                &BTreeMap::new(),
                |writer, source| writer.u64(u64::from(*source)),
            )
            .unwrap();
        assert_eq!(writer.finish(), literal);
    }
    let ty = AuthoredType::AssociatedReadViewProjection {
        scope: declaration(5),
        parameter: ImplementationParameterId::migrate(b"view-intent", 1),
        contract: declaration(3),
        member: AssociatedTypeMemberId::migrate(b"view-intent", 1),
        read_scope: scope(1),
    };
    let encode_type = |ty: &AuthoredType| {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer.authored_type(ty, &BTreeMap::new(), 1).unwrap();
        writer.finish()
    };
    let mut literal = vec![29];
    exact(&mut literal, 5);
    literal.extend_from_slice(&ImplementationParameterId::migrate(b"view-intent", 1).bytes());
    exact(&mut literal, 3);
    literal.extend_from_slice(&AssociatedTypeMemberId::migrate(b"view-intent", 1).bytes());
    literal_scope(&mut literal, 1);
    assert_eq!(encode_type(&ty), literal);
    let mut mutations = vec![ty; 5];
    if let AuthoredType::AssociatedReadViewProjection { scope, .. } = &mut mutations[0] {
        *scope = declaration(6);
    }
    if let AuthoredType::AssociatedReadViewProjection { parameter, .. } = &mut mutations[1] {
        *parameter = ImplementationParameterId::migrate(b"view-intent", 2);
    }
    if let AuthoredType::AssociatedReadViewProjection { contract, .. } = &mut mutations[2] {
        *contract = declaration(4);
    }
    if let AuthoredType::AssociatedReadViewProjection { member, .. } = &mut mutations[3] {
        *member = AssociatedTypeMemberId::migrate(b"view-intent", 2);
    }
    if let AuthoredType::AssociatedReadViewProjection { read_scope, .. } = &mut mutations[4] {
        *read_scope = scope(2);
    }
    for mutation in mutations {
        assert_ne!(literal, encode_type(&mutation));
    }
}

#[test]
fn family_bounds_and_bindings_commit_the_kind_and_full_scope_formal() {
    let member = AssociatedTypeMemberId::migrate(b"view-intent", 1);
    let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
    writer
        .associated_members(&[AuthoredAssociatedTypeMember {
            id: member,
            name: Name::new("Block").unwrap(),
            bound: AssociatedTypeBound::ReadViewFamily {
                scope_parameter: formal(1, "loan"),
            },
        }])
        .unwrap();
    let mut literal = 1_u64.to_be_bytes().to_vec();
    literal.extend_from_slice(&member.bytes());
    count(&mut literal, 5);
    literal.extend_from_slice(b"Block");
    literal.push(2); // Read-view family bound.
    literal.extend_from_slice(&formal(1, "loan").id.bytes());
    count(&mut literal, 4);
    literal.extend_from_slice(b"loan");
    assert_eq!(writer.finish(), literal);
    let encode_binding = |value| {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer
            .associated_bindings(
                &[AuthoredAssociatedTypeBinding { member, value }],
                &BTreeMap::new(),
            )
            .unwrap();
        writer.finish()
    };
    let mut literal = 1_u64.to_be_bytes().to_vec();
    literal.extend_from_slice(&member.bytes());
    literal.push(2);
    literal.extend_from_slice(&formal(1, "loan").id.bytes());
    count(&mut literal, 4);
    literal.extend_from_slice(b"loan");
    literal.push(3); // I64 template.
    assert_eq!(
        encode_binding(AuthoredAssociatedTypeBindingValue::ReadViewFamily {
            scope_parameter: formal(1, "loan"),
            ty: AuthoredType::I64 {}
        }),
        literal
    );
    for value in [
        AuthoredAssociatedTypeBindingValue::Owned {
            ty: AuthoredType::I64 {},
        },
        AuthoredAssociatedTypeBindingValue::ReadViewFamily {
            scope_parameter: formal(2, "loan"),
            ty: AuthoredType::I64 {},
        },
        AuthoredAssociatedTypeBindingValue::ReadViewFamily {
            scope_parameter: formal(1, "other"),
            ty: AuthoredType::I64 {},
        },
        AuthoredAssociatedTypeBindingValue::ReadViewFamily {
            scope_parameter: formal(1, "loan"),
            ty: AuthoredType::Bool {},
        },
    ] {
        assert_ne!(literal, encode_binding(value));
    }
}

#[test]
fn empty_view_construction_still_commits_source_and_field_references_are_qualified() {
    let encode_operation = |operation| {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer
            .expression_operation(&operation, &BTreeMap::new(), 1)
            .unwrap();
        writer.finish()
    };
    let create = |source| AuthoredExpressionOperation::CreateReadView {
        declaration: declaration(7),
        type_arguments: vec![],
        read_scope: scope(1),
        source,
        fields: vec![],
    };
    let mut literal = vec![50];
    exact(&mut literal, 7);
    count(&mut literal, 0);
    literal_scope(&mut literal, 1);
    literal.push(1);
    literal.extend_from_slice(&ParameterId::migrate(b"view-intent", 1).bytes());
    count(&mut literal, 0);
    assert_eq!(encode_operation(create(source(1))), literal);
    assert_ne!(literal, encode_operation(create(source(2))));
    let field = |owner| AuthoredExpressionOperation::ReadViewDataField {
        value: source(1),
        field: AuthoredReadViewFieldReference {
            declaration: declaration(owner),
            field: FieldId::migrate(b"view-intent", 1),
        },
    };
    assert_ne!(encode_operation(field(7)), encode_operation(field(8)));
    assert_ne!(
        encode_operation(AuthoredExpressionOperation::MoveReadView { value: source(1) }),
        encode_operation(AuthoredExpressionOperation::DropReadView { value: source(1) })
    );
}

#[test]
fn unused_view_fields_commit_identity_name_type_kind_scope_and_authored_order() {
    let field = AuthoredReadViewField {
        id: FieldId::migrate(b"view-intent", 1),
        name: Name::new("item").unwrap(),
        ty: AuthoredType::I64 {},
        kind: AuthoredReadViewFieldKind::ReadReference {
            read_scope: scope(1),
        },
    };
    let encode_fields = |fields: &[AuthoredReadViewField]| {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer.read_view_fields(fields, &BTreeMap::new()).unwrap();
        writer.finish()
    };
    let mut literal = 1_u64.to_be_bytes().to_vec();
    literal.extend_from_slice(&field.id.bytes());
    count(&mut literal, 4);
    literal.extend_from_slice(b"item");
    literal.extend_from_slice(&[3, 2]); // I64 field; typed read reference.
    literal_scope(&mut literal, 1);
    assert_eq!(encode_fields(std::slice::from_ref(&field)), literal);
    let mut mutations = vec![field.clone(); 5];
    mutations[0].id = FieldId::migrate(b"view-intent", 2);
    mutations[1].name = Name::new("other").unwrap();
    mutations[2].ty = AuthoredType::Bool {};
    mutations[3].kind = AuthoredReadViewFieldKind::Ordinary;
    mutations[4].kind = AuthoredReadViewFieldKind::ReadReference {
        read_scope: scope(2),
    };
    for mutation in &mutations {
        assert_ne!(literal, encode_fields(std::slice::from_ref(mutation)));
    }
    assert_ne!(
        encode_fields(&[field.clone(), mutations[0].clone()]),
        encode_fields(&[mutations[0].clone(), field.clone()])
    );
    let change = |fields| AuthoredChange::SetReadView {
        declaration: DeclarationSelector::Id {
            declaration: DeclarationId::migrate(b"view-intent", 7),
        },
        type_parameters: vec![],
        scope_parameter: formal(1, "loan"),
        fields,
    };
    assert_ne!(encode(change(vec![])), encode(change(vec![field])));
}

#[test]
fn all_call_and_method_mapping_scope_actuals_change_the_commitment() {
    let operations = |read_scope_arguments: Vec<AuthoredReadScopeArgument>| {
        [
            AuthoredExpressionOperation::Call {
                requirement_arguments: vec![],
                effect_arguments: vec![],
                function: declaration(7),
                type_arguments: vec![],
                read_scope_arguments: read_scope_arguments.clone(),
                arguments: vec![],
            },
            AuthoredExpressionOperation::ImplementationCall {
                requirement_arguments: vec![],
                effect_arguments: vec![],
                function: declaration(7),
                type_arguments: vec![],
                read_scope_arguments: read_scope_arguments.clone(),
                implementations: vec![],
                arguments: vec![],
            },
            AuthoredExpressionOperation::FunctionValue {
                requirement_arguments: vec![],
                effect_arguments: vec![],
                function: declaration(7),
                type_arguments: vec![],
                read_scope_arguments: read_scope_arguments.clone(),
            },
            AuthoredExpressionOperation::MethodCall {
                witness: AuthoredImplementationOperand::Parameter {
                    scope: declaration(5),
                    parameter: ImplementationParameterId::migrate(b"view-intent", 1),
                },
                contract: declaration(3),
                method: MethodId::migrate(b"view-intent", 1),
                read_scope_arguments,
                arguments: vec![],
            },
        ]
    };
    let encode_operation = |operation: &AuthoredExpressionOperation| {
        let mut writer = Writer::new(MAXIMUM_AUTHORED_CHANGE_BYTES);
        writer
            .expression_operation(operation, &BTreeMap::new(), 1)
            .unwrap();
        writer.finish()
    };
    let originals = operations(vec![scope(1), scope(2)]);
    let omitted = operations(vec![]);
    let changed = operations(vec![scope(1), scope(3)]);
    let reversed = operations(vec![scope(2), scope(1)]);
    for index in 0..4 {
        let original = encode_operation(&originals[index]);
        assert_ne!(original, encode_operation(&omitted[index]));
        assert_ne!(original, encode_operation(&changed[index]));
        assert_ne!(original, encode_operation(&reversed[index]));
    }
    let mapping = |read_scope_arguments| AuthoredChange::SetOwnedImplementation {
        declaration: DeclarationSelector::Id {
            declaration: DeclarationId::migrate(b"view-intent", 8),
        },
        contract: declaration(3),
        type_parameters: vec![],
        implementation_parameters: vec![],
        self_type: AuthoredType::OwnedI64Cell {},
        type_arguments: vec![],
        associated_types: vec![],
        methods: vec![AuthoredOwnedMethodImplementation {
            method: MethodId::migrate(b"view-intent", 1),
            function: declaration(7),
            type_arguments: vec![],
            read_scope_arguments,
            implementations: vec![],
        }],
    };
    let original = encode(mapping(vec![scope(1), scope(2)]));
    assert_ne!(original, encode(mapping(vec![])));
    assert_ne!(original, encode(mapping(vec![scope(1), scope(3)])));
    assert_ne!(original, encode(mapping(vec![scope(2), scope(1)])));
}
