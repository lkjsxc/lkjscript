//! Ordinary request lowering retains complete inline view inventories without extra owners.
use super::*;
use crate::platform::change::{
    AuthoredChange, AuthoredChangeSet, ChangeBudget, PrimitiveEdit, lower_authored_changes,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{ModuleId, ReadScopeParameterId};

fn proposal(
    view: &crate::platform::publication::RepositoryView,
    module: ModuleId,
) -> AuthoredChangeSet {
    let scope_parameter = crate::platform::kernel::ReadScopeParameter {
        id: ReadScopeParameterId::from_bytes([1; 16]).unwrap(),
        name: Name::new("source").unwrap(),
    };
    let read_scope = AuthoredReadScopeArgument::Parameter {
        binder: AuthoredReadScopeBinder::Declaration(AuthoredDeclarationReference::Local {
            declaration: DeclarationSelector::Symbol {
                symbol: "$window".to_owned(),
            },
        }),
        parameter: scope_parameter.id,
    };
    AuthoredChangeSet {
        base: view.revision(),
        preconditions: Vec::new(),
        budget: ChangeBudget::default(),
        changes: vec![AuthoredChange::CreateReadView {
            symbol: "$window".to_owned(),
            module: ModuleSelector::Id { module },
            name: Name::new("Window").unwrap(),
            visibility: DeclarationVisibility::Public,
            type_parameters: Vec::new(),
            scope_parameter,
            fields: vec![
                AuthoredReadViewField {
                    id: FieldId::from_bytes([3; 16]).unwrap(),
                    name: Name::new("storage").unwrap(),
                    ty: AuthoredType::OwnedSequence {
                        item: Box::new(AuthoredType::I64 {}),
                    },
                    kind: AuthoredReadViewFieldKind::ReadReference { read_scope },
                },
                AuthoredReadViewField {
                    id: FieldId::from_bytes([2; 16]).unwrap(),
                    name: Name::new("length").unwrap(),
                    ty: AuthoredType::I64 {},
                    kind: AuthoredReadViewFieldKind::Ordinary,
                },
            ],
        }],
    }
}

#[test]
fn authored_read_view_create_and_set_retain_ordered_fields_and_replace_empty_inventory() {
    let temporary = tempfile::tempdir().unwrap();
    let base = crate::platform::kernel::tests::witness_snapshot();
    let module = base
        .owners
        .keys()
        .find_map(|o| match o {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    let created = GraphRepository::create(&temporary.path().join("ordinary"), &base, None).unwrap();
    let view = created.repository.view_current().unwrap();
    let mut request = proposal(&view, module);
    let lowered = lower_authored_changes(&view, &view, &request).unwrap();
    assert!(!lowered.edits.iter().any(|edit| matches!(
        edit,
        PrimitiveEdit::InsertOwner {
            record: OwnerRecord::Field(_)
        }
    )));
    let owner = lowered.allocated["$window"];
    let declaration = lowered
        .edits
        .iter()
        .find_map(|edit| match edit {
            PrimitiveEdit::InsertOwner {
                record: OwnerRecord::Declaration(d),
            } if d.header.owner == owner => Some(d),
            _ => None,
        })
        .unwrap();
    let DeclarationPayload::ReadView(window) = &declaration.payload else {
        panic!("view payload")
    };
    assert_eq!(
        window
            .fields
            .iter()
            .map(|f| (f.id, f.name.as_str()))
            .collect::<Vec<_>>(),
        [
            (FieldId::from_bytes([3; 16]).unwrap(), "storage"),
            (FieldId::from_bytes([2; 16]).unwrap(), "length")
        ]
    );
    assert_eq!(
        window.scope_parameter.id,
        ReadScopeParameterId::from_bytes([1; 16]).unwrap()
    );
    let crate::platform::kernel::ReadViewFieldKind::ReadReference { read_scope } =
        window.fields[0].kind
    else {
        panic!("source relationship")
    };
    assert_eq!(
        read_scope,
        crate::platform::kernel::ReadScopeArgument::Parameter(
            crate::platform::kernel::ReadScopeReference {
                binder: crate::platform::kernel::ReadScopeBinder::Declaration(
                    DeclarationReference {
                        package: view.package(),
                        declaration: match owner {
                            OwnerKey::Declaration(id) => id,
                            _ => unreachable!(),
                        }
                    }
                ),
                parameter: window.scope_parameter.id
            }
        )
    );
    request.changes.push(AuthoredChange::SetReadView {
        declaration: DeclarationSelector::Symbol {
            symbol: "$window".to_owned(),
        },
        type_parameters: Vec::new(),
        scope_parameter: crate::platform::kernel::ReadScopeParameter {
            id: window.scope_parameter.id,
            name: Name::new("renamed").unwrap(),
        },
        fields: Vec::new(),
    });
    let replaced = lower_authored_changes(&view, &view, &request).unwrap();
    let result = replaced
        .edits
        .iter()
        .find_map(|edit| match edit {
            PrimitiveEdit::InsertOwner {
                record: OwnerRecord::Declaration(d),
            } => match &d.payload {
                DeclarationPayload::ReadView(v) => Some(v),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert!(result.fields.is_empty());
    assert!(result.type_parameters.is_empty());
    assert_eq!(result.scope_parameter.name.as_str(), "renamed");
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}

#[test]
fn authored_read_view_inline_field_identity_and_name_duplicates_refuse_without_publication() {
    let temporary = tempfile::tempdir().unwrap();
    let base = crate::platform::kernel::tests::witness_snapshot();
    let module = base
        .owners
        .keys()
        .find_map(|o| match o {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    let created = GraphRepository::create(&temporary.path().join("ordinary"), &base, None).unwrap();
    let view = created.repository.view_current().unwrap();
    for duplicate_id in [true, false] {
        let mut request = proposal(&view, module);
        let AuthoredChange::CreateReadView { fields, .. } = &mut request.changes[0] else {
            unreachable!()
        };
        if duplicate_id {
            fields[1].id = fields[0].id;
        } else {
            fields[1].name = fields[0].name.clone();
        }
        let error = lower_authored_changes(&view, &view, &request).unwrap_err();
        assert_eq!(error.class, DiagnosticClass::Semantic);
        assert_eq!(error.code, "change_authored_read_view_field_duplicate");
        assert_eq!(
            created.repository.view_current().unwrap().revision(),
            view.revision()
        );
    }
    assert!(lower_authored_changes(&view, &view, &proposal(&view, module)).is_ok());
}

#[test]
fn authored_read_view_let_uses_positive_annotation_and_keeps_unused_scope_inventory() {
    let temporary = tempfile::tempdir().unwrap();
    let base = crate::platform::kernel::tests::witness_snapshot();
    let module = base
        .owners
        .keys()
        .find_map(|o| match o {
            OwnerKey::Module(id) => Some(*id),
            _ => None,
        })
        .unwrap();
    let created = GraphRepository::create(&temporary.path().join("ordinary"), &base, None).unwrap();
    let view = created.repository.view_current().unwrap();
    let mut request = proposal(&view, module);
    let AuthoredChange::CreateReadView { fields, .. } = &mut request.changes[0] else {
        unreachable!()
    };
    fields.clear();
    let scope = crate::platform::kernel::ReadScopeParameter {
        id: ReadScopeParameterId::from_bytes([4; 16]).unwrap(),
        name: Name::new("source").unwrap(),
    };
    let read_scope = AuthoredReadScopeArgument::Parameter {
        binder: AuthoredReadScopeBinder::Declaration(AuthoredDeclarationReference::Local {
            declaration: DeclarationSelector::Symbol {
                symbol: "$reader".to_owned(),
            },
        }),
        parameter: scope.id,
    };
    let view_type = AuthoredType::ReadViewApplied {
        declaration: AuthoredDeclarationReference::Local {
            declaration: DeclarationSelector::Symbol {
                symbol: "$window".to_owned(),
            },
        },
        arguments: Vec::new(),
        read_scope: read_scope.clone(),
    };
    request.changes.push(AuthoredChange::CreateFunction {
        symbol: "$reader".to_owned(),
        module: ModuleSelector::Id { module },
        name: Name::new("observe").unwrap(),
        visibility: DeclarationVisibility::Private,
        type_parameters: Vec::new(),
        parameters: vec![AuthoredParameter {
            symbol: "$source".to_owned(),
            name: Name::new("source").unwrap(),
            ty: AuthoredType::ByteBuffer {},
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }],
        result: AuthoredCallableResult::value(AuthoredType::Unit {}),
        read_scope_parameters: vec![
            scope,
            crate::platform::kernel::ReadScopeParameter {
                id: ReadScopeParameterId::from_bytes([5; 16]).unwrap(),
                name: Name::new("unused").unwrap(),
            },
        ],
        read_scope_sources: vec![AuthoredReadScopeSource {
            source: AuthoredLocalReference::Symbol {
                symbol: "$source".to_owned(),
            },
            scope: read_scope.clone(),
        }],
        effect: AuthoredFunctionEffect::Pure {},
        body: AuthoredExpression {
            symbol: None,
            operation: AuthoredExpressionOperation::Let {
                bindings: vec![AuthoredLetBinding {
                    symbol: "$loan".to_owned(),
                    name: Name::new("loan").unwrap(),
                    declared_type: Some(view_type),
                    value: AuthoredExpression {
                        symbol: None,
                        operation: AuthoredExpressionOperation::CreateReadView {
                            declaration: AuthoredDeclarationReference::Local {
                                declaration: DeclarationSelector::Symbol {
                                    symbol: "$window".to_owned(),
                                },
                            },
                            type_arguments: Vec::new(),
                            read_scope,
                            source: AuthoredLocalReference::Symbol {
                                symbol: "$source".to_owned(),
                            },
                            fields: Vec::new(),
                        },
                    },
                }],
                body: Box::new(AuthoredExpression {
                    symbol: None,
                    operation: AuthoredExpressionOperation::DropReadView {
                        value: AuthoredLocalReference::Symbol {
                            symbol: "$loan".to_owned(),
                        },
                    },
                }),
            },
        },
    });
    let lowered = lower_authored_changes(&view, &view, &request).unwrap();
    let binding = lowered
        .edits
        .iter()
        .find_map(|e| match e {
            PrimitiveEdit::InsertOwner {
                record: OwnerRecord::Binding(b),
            } if b.header.owner == lowered.allocated["$loan"] => Some(b),
            _ => None,
        })
        .unwrap();
    assert_eq!(binding.kind, crate::platform::kernel::BindingKind::ReadView);
    assert!(binding.value.is_some());
    assert!(binding.declared_type.is_some());
    let function = lowered
        .edits
        .iter()
        .find_map(|e| match e {
            PrimitiveEdit::InsertOwner {
                record: OwnerRecord::Declaration(d),
            } if d.header.owner == lowered.allocated["$reader"] => match &d.payload {
                DeclarationPayload::Function(f) => Some(f),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    assert_eq!(
        function
            .read_scope_parameters
            .iter()
            .map(|p| (p.id, p.name.as_str()))
            .collect::<Vec<_>>(),
        [
            (ReadScopeParameterId::from_bytes([4; 16]).unwrap(), "source"),
            (ReadScopeParameterId::from_bytes([5; 16]).unwrap(), "unused")
        ]
    );
    assert_eq!(function.read_scope_sources.len(), 1);
    assert!(matches!(
        function.result.relation,
        crate::platform::kernel::ResultRelation::Value
    ));
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}
