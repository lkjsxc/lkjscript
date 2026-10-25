//! Independent raw readback controls. These never admit or publish a read-view candidate.
use super::*;
use crate::platform::kernel::{
    ReadScopeArgument, ReadScopeBinder, ReadScopeReference, ReadViewField, ReadViewFieldKind,
    TypeObject,
};
use crate::platform::publication::GraphRepository;
use crate::platform::semantic_id::{AssociatedTypeMemberId, ImplementationParameterId, MethodId};

fn declaration_ref(package_byte: u8, declaration_byte: u8) -> k::DeclarationReference {
    k::DeclarationReference {
        package: PackageId::from_bytes([package_byte; 16]).unwrap(),
        declaration: DeclarationId::from_bytes([declaration_byte; 16]).unwrap(),
    }
}
fn scope_parameter(byte: u8, name: &str) -> k::ReadScopeParameter {
    k::ReadScopeParameter {
        id: ReadScopeParameterId::from_bytes([byte; 16]).unwrap(),
        name: Name::new(name).unwrap(),
    }
}
fn insert_type(
    types: &mut BTreeMap<k::TypeObjectDigest, TypeObject>,
    form: TypeForm,
) -> k::TypeObjectDigest {
    let ty = TypeObject::new(form).unwrap();
    let digest = k::encode_type_object(&ty).unwrap().0;
    types.insert(digest, ty);
    digest
}

#[test]
fn raw_read_view_readback_keeps_every_scope_binder_and_family_coordinate() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("ordinary"),
        &k::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let view = created.repository.view_current().unwrap();
    let contract = declaration_ref(1, 2);
    let implementation = declaration_ref(3, 4);
    let callable = declaration_ref(5, 6);
    let member = AssociatedTypeMemberId::from_bytes([7; 16]).unwrap();
    let formal = scope_parameter(8, "window");
    let binding = BindingId::from_bytes([9; 16]).unwrap();
    let method = MethodId::from_bytes([10; 16]).unwrap();
    let scopes = vec![
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::Declaration(callable),
            parameter: formal.id,
        }),
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::ContractMethod { contract, method },
            parameter: formal.id,
        }),
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::AssociatedMember { contract, member },
            parameter: formal.id,
        }),
        ReadScopeArgument::Parameter(ReadScopeReference {
            binder: ReadScopeBinder::AssociatedBinding {
                implementation,
                member,
            },
            parameter: formal.id,
        }),
        ReadScopeArgument::Lexical { callable, binding },
    ];
    let authored_scopes = vec![
        AuthoredReadScopeArgument::Parameter {
            binder: AuthoredReadScopeBinder::Declaration(declaration(callable)),
            parameter: formal.id,
        },
        AuthoredReadScopeArgument::Parameter {
            binder: AuthoredReadScopeBinder::ContractMethod {
                contract: declaration(contract),
                method,
            },
            parameter: formal.id,
        },
        AuthoredReadScopeArgument::Parameter {
            binder: AuthoredReadScopeBinder::AssociatedMember {
                contract: declaration(contract),
                member,
            },
            parameter: formal.id,
        },
        AuthoredReadScopeArgument::Parameter {
            binder: AuthoredReadScopeBinder::AssociatedBinding {
                implementation: declaration(implementation),
                member,
            },
            parameter: formal.id,
        },
        AuthoredReadScopeArgument::Lexical {
            callable: declaration(callable),
            binding: AuthoredLocalReference::Symbol {
                symbol: format!("$b_{binding}"),
            },
        },
    ];
    let mut types = BTreeMap::new();
    let projected = insert_type(
        &mut types,
        TypeForm::AssociatedReadViewProjection {
            scope: callable,
            parameter: ImplementationParameterId::from_bytes([11; 16]).unwrap(),
            contract,
            member,
            read_scope: scopes[4],
        },
    );
    let members = vec![k::AssociatedTypeMember {
        id: member,
        name: Name::new("Window").unwrap(),
        bound: k::AssociatedTypeBound::ReadViewFamily {
            scope_parameter: formal.clone(),
        },
    }];
    let bindings = vec![k::AssociatedTypeBinding {
        member,
        value: k::AssociatedTypeBindingValue::ReadViewFamily {
            scope_parameter: formal.clone(),
            ty: projected,
        },
    }];
    let mut reader = Reader::for_raw_lowered_candidate(
        &view,
        ExecutionControl::uncancelled(),
        BTreeMap::new(),
        types,
    );
    assert_eq!(reader.read_scopes(scopes).unwrap(), authored_scopes);
    assert_eq!(
        reader.associated_members(&members).unwrap(),
        vec![AuthoredAssociatedTypeMember {
            id: member,
            name: Name::new("Window").unwrap(),
            bound: k::AssociatedTypeBound::ReadViewFamily {
                scope_parameter: formal.clone()
            }
        }]
    );
    assert_eq!(
        reader.associated_bindings(&bindings).unwrap(),
        vec![AuthoredAssociatedTypeBinding {
            member,
            value: AuthoredAssociatedTypeBindingValue::ReadViewFamily {
                scope_parameter: formal,
                ty: AuthoredType::AssociatedReadViewProjection {
                    scope: declaration(callable),
                    parameter: ImplementationParameterId::from_bytes([11; 16]).unwrap(),
                    contract: declaration(contract),
                    member,
                    read_scope: authored_scopes[4].clone()
                }
            }
        }]
    );
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}

#[test]
fn raw_read_view_readback_reserves_complete_fields_and_cancels_empty_inventory_before_growth() {
    let temporary = tempfile::tempdir().unwrap();
    let created = GraphRepository::create(
        &temporary.path().join("ordinary"),
        &k::tests::witness_snapshot(),
        None,
    )
    .unwrap();
    let view = created.repository.view_current().unwrap();
    let mut types = BTreeMap::new();
    let ty = insert_type(&mut types, TypeForm::I64);
    let field = ReadViewField {
        id: FieldId::from_bytes([1; 16]).unwrap(),
        name: Name::new("length").unwrap(),
        ty,
        kind: ReadViewFieldKind::Ordinary,
    };
    for available in [1, 2] {
        let mut reader = Reader::for_raw_lowered_candidate(
            &view,
            ExecutionControl::uncancelled(),
            BTreeMap::new(),
            types.clone(),
        );
        reader.type_nodes = crate::platform::change::budget::BudgetCounter::exact(u64::MAX - available);
        assert_eq!(
            reader.view_fields(vec![field.clone()]).unwrap(),
            vec![AuthoredReadViewField {
                id: field.id,
                name: field.name.clone(),
                ty: AuthoredType::I64 {},
                kind: AuthoredReadViewFieldKind::Ordinary
            }]
        );
        assert_eq!(reader.type_nodes.consumed, u64::MAX);
        assert_eq!(reader.type_nodes.saturated, available < 2);
    }
    let mut cancelled = Reader::for_raw_lowered_candidate(
        &view,
        ExecutionControl::cancel_after_checks(0),
        BTreeMap::new(),
        types,
    );
    assert_eq!(
        cancelled.read_scopes(Vec::new()).unwrap_err().class,
        DiagnosticClass::Cancelled
    );
    let mut healthy = Reader::new(&view, ExecutionControl::uncancelled());
    assert_eq!(
        healthy.read_scopes(Vec::new()).unwrap(),
        Vec::<AuthoredReadScopeArgument>::new()
    );
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}

#[test]
fn raw_read_view_draft_reentry_preserves_scope_formal_inline_field_order_and_complete_set_inventory()
 {
    let temporary = tempfile::tempdir().unwrap();
    let base = k::tests::witness_snapshot();
    let created = GraphRepository::create(&temporary.path().join("ordinary"), &base, None).unwrap();
    let view = created.repository.view_current().unwrap();
    let module = base
        .owners
        .values()
        .find_map(|o| match o {
            OwnerRecord::Module(m) => Some(m.header.owner),
            _ => None,
        })
        .unwrap();
    let OwnerKey::Module(module) = module else {
        unreachable!()
    };
    let id = DeclarationId::from_bytes([21; 16]).unwrap();
    let owner = OwnerKey::Declaration(id);
    let formal = scope_parameter(22, "source");
    let read_scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(k::DeclarationReference {
            package: view.package(),
            declaration: id,
        }),
        parameter: formal.id,
    });
    let mut types = BTreeMap::new();
    let length = insert_type(&mut types, TypeForm::I64);
    let storage = insert_type(&mut types, TypeForm::OwnedSequence { item: length });
    let record = OwnerRecord::Declaration(k::DeclarationRecord {
        header: k::OwnerHeader::new(owner, k::OwnerKind::ReadView),
        module,
        name: Name::new("Window").unwrap(),
        visibility: k::DeclarationVisibility::Private,
        payload: k::DeclarationPayload::ReadView(k::ReadViewDeclaration {
            type_parameters: Vec::new(),
            scope_parameter: formal.clone(),
            fields: vec![
                ReadViewField {
                    id: FieldId::from_bytes([24; 16]).unwrap(),
                    name: Name::new("items").unwrap(),
                    ty: storage,
                    kind: ReadViewFieldKind::ReadReference { read_scope },
                },
                ReadViewField {
                    id: FieldId::from_bytes([23; 16]).unwrap(),
                    name: Name::new("length").unwrap(),
                    ty: length,
                    kind: ReadViewFieldKind::Ordinary,
                },
            ],
        }),
    });
    let owners = BTreeMap::from([(owner, record)]);
    let rendered = super::super::draft::render_raw_lowered_candidate(
        &view,
        &[owner.into()],
        Some(4 * 1_048_576),
        ExecutionControl::uncancelled(),
        owners.clone(),
        types.clone(),
    )
    .unwrap();
    let mut reader =
        Reader::for_raw_lowered_candidate(&view, ExecutionControl::uncancelled(), owners, types);
    let parsed = super::super::decode_parsed_change(
        super::super::input::parse("raw-window-draft.lkjc", &rendered).unwrap(),
        Some(&mut reader),
    )
    .unwrap();
    let AuthoredChange::SetReadView {
        declaration: selected,
        type_parameters,
        scope_parameter,
        fields,
    } = parsed
        .semantic
        .changes
        .iter()
        .find(|c| matches!(c, AuthoredChange::SetReadView { .. }))
        .unwrap()
    else {
        unreachable!()
    };
    assert_eq!(selected, &DeclarationSelector::Id { declaration: id });
    assert!(type_parameters.is_empty());
    assert_eq!(scope_parameter, &formal);
    assert_eq!(
        fields
            .iter()
            .map(|f| (f.id, f.name.as_str()))
            .collect::<Vec<_>>(),
        [
            (FieldId::from_bytes([24; 16]).unwrap(), "items"),
            (FieldId::from_bytes([23; 16]).unwrap(), "length")
        ]
    );
    assert!(matches!(
        fields[0].kind,
        AuthoredReadViewFieldKind::ReadReference { .. }
    ));
    assert_eq!(fields[1].kind, AuthoredReadViewFieldKind::Ordinary);
    assert_eq!(
        created.repository.view_current().unwrap().revision(),
        view.revision()
    );
}
