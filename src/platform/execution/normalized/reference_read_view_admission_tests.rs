//! Hand-built raw records are the oracle inputs. No producer admission seal,
//! compiler index, authored product execution, or runtime permission is reused.
use super::*;
use crate::platform::kernel::{OwnerHeader, ReadViewField, ReadViewFieldInitializer, SemanticRoot};
use crate::platform::persistent_map::{MapContentDigest, MapRoot, PageDigest};
use crate::platform::semantic_id::{
    DeclarationId, ModuleId, ParameterId, ReadScopeParameterId, RepositoryId,
};

const SEED: &[u8] = b"independent raw canonical read-view admission";

fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn decl(source: &KernelSnapshot, ordinal: u64) -> DeclarationReference {
    DeclarationReference {
        package: source.root.package_id,
        declaration: DeclarationId::migrate(SEED, ordinal),
    }
}
fn scope(binder: ReadScopeBinder, ordinal: u64) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder,
        parameter: ReadScopeParameterId::migrate(SEED, ordinal),
    })
}
fn empty() -> KernelSnapshot {
    let map = MapRoot::from_parts(
        PageDigest::from_bytes([0; 32]),
        0,
        MapContentDigest::from_bytes([0; 32]),
    );
    KernelSnapshot {
        root: SemanticRoot {
            graph_contract_version: kernel::contract::GRAPH_CONTRACT_VERSION,
            repository_id: RepositoryId::migrate(SEED, 0),
            package_id: PackageId::migrate(SEED, 0),
            package_name: name("raw-view-proof"),
            owners: map,
            dependencies: map,
            retirements: map,
        },
        owners: BTreeMap::new(),
        types: BTreeMap::new(),
        dependency_interfaces: BTreeMap::new(),
        dependency_types: BTreeMap::new(),
        representation_dependencies: BTreeMap::new(),
        blobs: BTreeMap::new(),
        dependencies: BTreeMap::new(),
        retirements: BTreeMap::new(),
    }
}
fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let ty = encode_type_object(&object).unwrap().0;
    source.types.insert(ty, object);
    ty
}
fn declaration(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
    module: u64,
    kind: OwnerKind,
    payload: DeclarationPayload,
) {
    let key = OwnerKey::Declaration(reference.declaration);
    source.owners.insert(
        key,
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(key, kind),
            module: ModuleId::migrate(SEED, module),
            name: name(&format!("declaration_{module}_{}", reference.declaration)),
            visibility: kernel::DeclarationVisibility::Private,
            payload,
        }),
    );
}
fn add_view(
    source: &mut KernelSnapshot,
    ordinal: u64,
    reference: bool,
    phantom: Option<TypeParameterConstraints>,
) -> (DeclarationReference, TypeObjectDigest) {
    let reference_decl = decl(source, ordinal);
    let scalar = intern(source, TypeForm::I64);
    let owned = intern(source, TypeForm::OwnedI64Cell);
    let mut type_parameters = Vec::new();
    let mut arguments = Vec::new();
    if let Some(constraints) = phantom {
        let parameter = TypeParameterId::migrate(SEED, ordinal);
        type_parameters.push(parameter);
        source.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(kernel::TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: reference_decl.declaration,
                name: name("Phantom"),
                constraints,
            }),
        );
        arguments.push(intern(source, TypeForm::TypeParameter { parameter }));
    }
    let read_scope = scope(ReadScopeBinder::Declaration(reference_decl), ordinal);
    let mut fields = vec![ReadViewField {
        id: FieldId::migrate(SEED, ordinal * 2),
        name: name("length"),
        ty: scalar,
        kind: ReadViewFieldKind::Ordinary,
    }];
    if reference {
        fields.push(ReadViewField {
            id: FieldId::migrate(SEED, ordinal * 2 + 1),
            name: name("source"),
            ty: owned,
            kind: ReadViewFieldKind::ReadReference { read_scope },
        });
    }
    declaration(
        source,
        reference_decl,
        0,
        OwnerKind::ReadView,
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters,
            scope_parameter: ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, ordinal),
                name: name("s"),
            },
            fields,
        }),
    );
    let ty = intern(
        source,
        TypeForm::ReadViewApplied {
            declaration: reference_decl,
            arguments,
            read_scope,
        },
    );
    (reference_decl, ty)
}
fn admit(source: &KernelSnapshot) -> Result<Proof> {
    Proof::reconstruct(&[source], &ExecutionControl::uncancelled())
}

fn type_parameter(
    source: &mut KernelSnapshot,
    owner: DeclarationReference,
    ordinal: u64,
    constraints: TypeParameterConstraints,
) -> (TypeParameterId, TypeObjectDigest) {
    let parameter = TypeParameterId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::TypeParameter(parameter),
        OwnerRecord::TypeParameter(kernel::TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: owner.declaration,
            name: name("T"),
            constraints,
        }),
    );
    let ty = intern(source, TypeForm::TypeParameter { parameter });
    (parameter, ty)
}
fn record_fields(
    source: &mut KernelSnapshot,
    owner: DeclarationReference,
    ordinal: u64,
    owned: bool,
    parameters: Vec<TypeParameterId>,
    types: &[TypeObjectDigest],
) {
    let fields: Vec<_> = types
        .iter()
        .enumerate()
        .map(|(index, ty)| {
            let field = FieldId::migrate(SEED, ordinal * 10 + index as u64);
            source.owners.insert(
                OwnerKey::Field(field),
                OwnerRecord::Field(kernel::FieldRecord {
                    header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
                    declaration: owner.declaration,
                    name: name(&format!("field_{index}")),
                    ty: *ty,
                }),
            );
            field
        })
        .collect();
    let payload = if owned {
        DeclarationPayload::OwnedRecord {
            type_parameters: parameters,
            fields,
        }
    } else {
        DeclarationPayload::Record {
            type_parameters: parameters,
            fields,
        }
    };
    declaration(
        source,
        owner,
        0,
        if owned {
            OwnerKind::OwnedRecord
        } else {
            OwnerKind::Record
        },
        payload,
    );
}
fn fifo(source: &mut KernelSnapshot, ordinal: u64) -> (DeclarationReference, TypeObjectDigest) {
    let owner = decl(source, ordinal);
    let (parameter, formal) =
        type_parameter(source, owner, ordinal, TypeParameterConstraints::Owned);
    let sequence = intern(source, TypeForm::OwnedSequence { item: formal });
    let count = intern(source, TypeForm::I64);
    record_fields(
        source,
        owner,
        ordinal,
        true,
        vec![parameter],
        &[sequence, sequence, count],
    );
    (owner, formal)
}
fn tree(source: &mut KernelSnapshot, ordinal: u64) -> (DeclarationReference, TypeObjectDigest) {
    let owner = decl(source, ordinal);
    let (parameter, formal) =
        type_parameter(source, owner, ordinal, TypeParameterConstraints::None);
    let generic = intern(
        source,
        TypeForm::Applied {
            declaration: owner,
            arguments: vec![formal],
        },
    );
    let children = intern(source, TypeForm::List { item: generic });
    let cases: Vec<_> = [formal, children]
        .into_iter()
        .enumerate()
        .map(|(index, ty)| {
            let id =
                crate::platform::semantic_id::CaseId::migrate(SEED, ordinal * 10 + index as u64);
            source.owners.insert(
                OwnerKey::Case(id),
                OwnerRecord::Case(kernel::CaseRecord {
                    header: OwnerHeader::new(OwnerKey::Case(id), OwnerKind::Case),
                    declaration: owner.declaration,
                    name: name(if index == 0 { "leaf" } else { "branch" }),
                    payload: Some(ty),
                }),
            );
            id
        })
        .collect();
    declaration(
        source,
        owner,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: vec![parameter],
            cases,
        },
    );
    (owner, generic)
}
fn expression(
    source: &mut KernelSnapshot,
    ordinal: u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(kernel::ExpressionRecord {
            contract_version: kernel::contract::GRAPH_CONTRACT_VERSION,
            id,
            operation,
        }),
    );
    id
}

struct Constructor {
    source: KernelSnapshot,
    function: DeclarationReference,
    view: DeclarationReference,
    first: ParameterId,
    second: ParameterId,
    field_expression: ExpressionId,
    body: ExpressionId,
    ty: TypeObjectDigest,
}
fn constructor(reference: bool) -> Constructor {
    let mut source = empty();
    let (view, _) = add_view(&mut source, 1, reference, None);
    let function = decl(&source, 2);
    let owned = intern(&mut source, TypeForm::OwnedI64Cell);
    let first = ParameterId::migrate(SEED, 0);
    let second = ParameterId::migrate(SEED, 1);
    for (id, label) in [(first, "selected"), (second, "alias")] {
        source.owners.insert(
            OwnerKey::Parameter(id),
            OwnerRecord::Parameter(kernel::ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                parent: ParameterParent::Function(function.declaration),
                name: name(label),
                ty: owned,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
    }
    let read_scope = scope(ReadScopeBinder::Declaration(function), 2);
    let ty = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view,
            arguments: Vec::new(),
            read_scope,
        },
    );
    let length = expression(&mut source, 0, ExpressionOperation::I64 { value: 4 });
    let field_expression = if reference {
        expression(
            &mut source,
            1,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(first),
            },
        )
    } else {
        length
    };
    let mut fields = vec![ReadViewFieldInitializer {
        field: FieldId::migrate(SEED, 2),
        value: length,
    }];
    if reference {
        fields.push(ReadViewFieldInitializer {
            field: FieldId::migrate(SEED, 3),
            value: field_expression,
        });
    }
    let body = expression(
        &mut source,
        2,
        ExpressionOperation::CreateReadView {
            declaration: view,
            type_arguments: Vec::new(),
            read_scope,
            source: LocalValueReference::FunctionParameter(first),
            fields,
        },
    );
    declaration(
        &mut source,
        function,
        0,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![first, second],
            result: CallableResult {
                ty,
                relation: ResultRelation::ViewFrom {
                    source: first,
                    scope: read_scope,
                },
            },
            read_scope_parameters: vec![ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, 2),
                name: name("s"),
            }],
            read_scope_sources: vec![
                ReadScopeSource {
                    source: first,
                    scope: read_scope,
                },
                ReadScopeSource {
                    source: second,
                    scope: read_scope,
                },
            ],
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    Constructor {
        source,
        function,
        view,
        first,
        second,
        field_expression,
        body,
        ty,
    }
}

#[test]
fn raw_nominal_layout_keeps_authored_fields_and_substitutes_exact_scope() {
    let fixture = constructor(true);
    let proof = admit(&fixture.source).unwrap();
    let layout = proof.layout(fixture.ty).unwrap();
    assert_eq!(layout.declaration, fixture.view);
    assert_eq!(
        layout
            .fields
            .iter()
            .map(|f| f.name.as_str())
            .collect::<Vec<_>>(),
        vec!["length", "source"]
    );
    assert_eq!(
        layout.fields[1].kind,
        ReadViewFieldKind::ReadReference {
            read_scope: layout.read_scope
        }
    );
    assert_eq!(proof.category(fixture.ty).unwrap(), ValueCategory::ReadView);
    assert!(proof.read_shareable(fixture.ty).unwrap());
    assert_eq!(
        proof
            .scope_sources(ReadScopeBinder::Declaration(fixture.function))
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn raw_constructor_refuses_same_type_and_same_scope_other_parameter() {
    let mut fixture = constructor(true);
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.field_expression))
        .unwrap()
    else {
        unreachable!()
    };
    expression.operation = ExpressionOperation::Local {
        value: LocalValueReference::FunctionParameter(fixture.second),
    };
    assert!(
        admit(&fixture.source)
            .unwrap_err()
            .message
            .contains("another same-typed source")
    );
}

#[test]
fn metadata_only_constructor_keeps_selected_source_and_refuses_wrong_return_anchor() {
    let mut fixture = constructor(false);
    assert!(
        admit(&fixture.source)
            .unwrap()
            .read_shareable(fixture.ty)
            .unwrap()
    );
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.body))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::CreateReadView { source, .. } = &mut expression.operation else {
        unreachable!()
    };
    *source = LocalValueReference::FunctionParameter(fixture.second);
    assert!(
        admit(&fixture.source)
            .unwrap_err()
            .message
            .contains("lineage")
    );
}

#[test]
fn hidden_unused_container_view_and_unused_foreign_scope_reject() {
    let mut source = empty();
    let (_, view) = add_view(&mut source, 1, false, None);
    intern(&mut source, TypeForm::List { item: view });
    assert!(admit(&source).is_err());
    let mut source = empty();
    let (view_decl, _) = add_view(&mut source, 1, false, None);
    let foreign = decl(&source, 99);
    intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view_decl,
            arguments: Vec::new(),
            read_scope: scope(ReadScopeBinder::Declaration(foreign), 99),
        },
    );
    assert!(admit(&source).is_err());
}

#[test]
fn metadata_only_view_still_checks_phantom_category_and_sharing() {
    let mut source = empty();
    let (view_decl, symbolic) =
        add_view(&mut source, 1, false, Some(TypeParameterConstraints::Owned));
    let (_, other_view) = add_view(&mut source, 2, false, None);
    let proof = admit(&source).unwrap();
    assert!(!proof.read_shareable(symbolic).unwrap());
    let forbidden = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view_decl,
            arguments: vec![other_view],
            read_scope: scope(ReadScopeBinder::Declaration(view_decl), 1),
        },
    );
    assert!(proof.layout_in(forbidden, &source.types).is_err());
    assert!(admit(&source).is_err());
}

#[test]
fn recursive_view_reference_and_duplicate_field_inventory_reject() {
    // Finite repeated declaration instances are not raw schema backedges. The
    // two sequence siblings also share one exact instantiated field type.
    let mut finite = empty();
    let (fifo_decl, _) = fifo(&mut finite, 100);
    let bytes = intern(&mut finite, TypeForm::ByteBuffer);
    let one = intern(
        &mut finite,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![bytes],
        },
    );
    let two = intern(
        &mut finite,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![one],
        },
    );
    let three = intern(
        &mut finite,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![two],
        },
    );
    let four = intern(
        &mut finite,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![three],
        },
    );
    let finite_proof = admit(&finite).unwrap();
    for ty in [one, two, three, four] {
        assert_eq!(
            finite_proof.prepared_category(ty).unwrap(),
            ValueCategory::Owned
        );
        assert!(finite_proof.prepared_transferable(ty).unwrap());
        assert!(finite_proof.prepared_fact(ty).unwrap().share);
    }

    // Ordinary recursive Tree<T>/List<Tree<T>> retains independent complete
    // properties. Inactive cases and callable/Secret actuals remain visible.
    let mut ordinary = empty();
    let (tree_decl, _) = tree(&mut ordinary, 101);
    let integer = intern(&mut ordinary, TypeForm::I64);
    let text = intern(&mut ordinary, TypeForm::Text);
    let secret = intern(&mut ordinary, TypeForm::Secret);
    let callable = intern(
        &mut ordinary,
        TypeForm::Function {
            parameters: Vec::new(),
            result: integer,
        },
    );
    let roots = [integer, text, secret, callable].map(|actual| {
        intern(
            &mut ordinary,
            TypeForm::Applied {
                declaration: tree_decl,
                arguments: vec![actual],
            },
        )
    });
    let ordinary_proof = admit(&ordinary).unwrap();
    for (index, ty) in roots.into_iter().enumerate() {
        assert_eq!(
            ordinary_proof.prepared_category(ty).unwrap(),
            ValueCategory::OrdinaryData
        );
        assert_eq!(ordinary_proof.prepared_metadata(ty).unwrap(), index < 2);
        assert_eq!(ordinary_proof.prepared_transferable(ty).unwrap(), index < 2);
        assert_eq!(ordinary_proof.prepared_fact(ty).unwrap().share, index < 2);
    }

    // A late poison sibling in A -> B -> List<A> must propagate through the
    // whole SCC before either independently prepared root can be published.
    let mut mutual = empty();
    let a = decl(&mutual, 102);
    let b = decl(&mutual, 103);
    let a_ty = intern(&mut mutual, TypeForm::Named { declaration: a });
    let b_ty = intern(&mut mutual, TypeForm::Named { declaration: b });
    let back = intern(&mut mutual, TypeForm::List { item: a_ty });
    let poison = intern(&mut mutual, TypeForm::Secret);
    record_fields(&mut mutual, a, 102, false, Vec::new(), &[b_ty, poison]);
    record_fields(&mut mutual, b, 103, false, Vec::new(), &[back]);
    let proof = admit(&mutual).unwrap();
    for ty in [a_ty, b_ty] {
        assert!(!proof.prepared_metadata(ty).unwrap());
        assert!(!proof.prepared_transferable(ty).unwrap());
        assert!(!proof.prepared_fact(ty).unwrap().share);
    }
    let mut constrained = mutual.clone();
    let wrapper = decl(&constrained, 113);
    let (parameter, _) = type_parameter(
        &mut constrained,
        wrapper,
        113,
        TypeParameterConstraints::Transferable,
    );
    record_fields(&mut constrained, wrapper, 113, false, vec![parameter], &[]);
    intern(
        &mut constrained,
        TypeForm::Applied {
            declaration: wrapper,
            arguments: vec![b_ty],
        },
    );
    assert!(
        admit(&constrained)
            .unwrap_err()
            .message
            .contains("obligation failed")
    );
    let late = FieldId::migrate(SEED, 1021);
    for forbidden in [
        TypeForm::OwnedI64Cell,
        TypeForm::ReadViewApplied {
            declaration: {
                let (view, _) = add_view(&mut mutual, 104, false, None);
                view
            },
            arguments: Vec::new(),
            read_scope: scope(ReadScopeBinder::Declaration(decl(&mutual, 104)), 104),
        },
    ] {
        let mut candidate = mutual.clone();
        let ty = intern(&mut candidate, forbidden);
        let OwnerRecord::Field(field) = candidate.owners.get_mut(&OwnerKey::Field(late)).unwrap()
        else {
            unreachable!()
        };
        field.ty = ty;
        assert!(admit(&candidate).is_err());
    }

    // Raw Owned schemas refuse direct, mutual, and growing recursion before
    // instantiated digests can hide a collection or phantom edge.
    for mutual_cycle in [false, true] {
        let mut source = empty();
        let a = decl(&source, 105);
        let b = decl(&source, 106);
        let a_ty = intern(&mut source, TypeForm::Named { declaration: a });
        let b_ty = intern(&mut source, TypeForm::Named { declaration: b });
        let sequence = intern(&mut source, TypeForm::OwnedSequence { item: a_ty });
        record_fields(
            &mut source,
            a,
            105,
            true,
            Vec::new(),
            &[if mutual_cycle { b_ty } else { sequence }],
        );
        record_fields(&mut source, b, 106, true, Vec::new(), &[sequence]);
        assert!(
            admit(&source)
                .unwrap_err()
                .message
                .contains("cyclic raw Owned")
        );
    }
    let mut growing = empty();
    let owner = decl(&growing, 107);
    let (parameter, formal) =
        type_parameter(&mut growing, owner, 107, TypeParameterConstraints::Owned);
    let sequence = intern(&mut growing, TypeForm::OwnedSequence { item: formal });
    let application = intern(
        &mut growing,
        TypeForm::Applied {
            declaration: owner,
            arguments: vec![sequence],
        },
    );
    record_fields(
        &mut growing,
        owner,
        107,
        true,
        vec![parameter],
        &[application],
    );
    assert!(
        admit(&growing)
            .unwrap_err()
            .message
            .contains("cyclic raw Owned")
    );
    let mut phantom = empty();
    let owner = decl(&phantom, 114);
    let erased = decl(&phantom, 115);
    let owner_ty = intern(&mut phantom, TypeForm::Named { declaration: owner });
    let (parameter, _) = type_parameter(&mut phantom, erased, 115, TypeParameterConstraints::Owned);
    record_fields(&mut phantom, erased, 115, true, vec![parameter], &[]);
    let invisible_backedge = intern(
        &mut phantom,
        TypeForm::Applied {
            declaration: erased,
            arguments: vec![owner_ty],
        },
    );
    record_fields(
        &mut phantom,
        owner,
        114,
        true,
        Vec::new(),
        &[invisible_backedge],
    );
    assert!(
        admit(&phantom)
            .unwrap_err()
            .message
            .contains("cyclic raw Owned")
    );

    let mut source = empty();
    let (view_decl, ty) = add_view(&mut source, 1, true, None);
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(view_decl.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(view) = &mut owner.payload else {
        unreachable!()
    };
    view.fields[1].ty = ty;
    assert!(admit(&source).unwrap_err().message.contains("cyclic"));
    let mut source = empty();
    let (view_decl, _) = add_view(&mut source, 1, true, None);
    let OwnerRecord::Declaration(owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(view_decl.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(view) = &mut owner.payload else {
        unreachable!()
    };
    view.fields[1].id = view.fields[0].id;
    assert!(admit(&source).unwrap_err().message.contains("duplicate"));

    // A view cannot use an ordinary SCC even when that SCC's independent
    // metadata properties are true, whether reached as metadata or a phantom.
    for phantom in [false, true] {
        let mut source = empty();
        let (tree_decl, _) = tree(&mut source, 108);
        let scalar = intern(&mut source, TypeForm::I64);
        let recursive = intern(
            &mut source,
            TypeForm::Applied {
                declaration: tree_decl,
                arguments: vec![scalar],
            },
        );
        let (view_decl, _) = add_view(
            &mut source,
            109,
            false,
            phantom.then_some(TypeParameterConstraints::Transferable),
        );
        if phantom {
            intern(
                &mut source,
                TypeForm::ReadViewApplied {
                    declaration: view_decl,
                    arguments: vec![recursive],
                    read_scope: scope(ReadScopeBinder::Declaration(view_decl), 109),
                },
            );
        } else {
            let OwnerRecord::Declaration(owner) = source
                .owners
                .get_mut(&OwnerKey::Declaration(view_decl.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::ReadView(view) = &mut owner.payload else {
                unreachable!()
            };
            view.fields[0].ty = recursive;
        }
        assert!(
            admit(&source)
                .unwrap_err()
                .message
                .contains("cyclic raw view")
        );
    }

    // Mutual view references and a growing view phantom actual are raw schema
    // cycles regardless of changing scopes or instantiated argument digests.
    for growing_cycle in [false, true] {
        let mut source = empty();
        let (a, _) = add_view(
            &mut source,
            110,
            true,
            growing_cycle.then_some(TypeParameterConstraints::Owned),
        );
        let a_scope = scope(ReadScopeBinder::Declaration(a), 110);
        let target = if growing_cycle {
            let (fifo_decl, formal) = fifo(&mut source, 111);
            // Use a's own formal, not the unrelated Fifo formal.
            let own_formal = intern(
                &mut source,
                TypeForm::TypeParameter {
                    parameter: TypeParameterId::migrate(SEED, 110),
                },
            );
            let nested = intern(
                &mut source,
                TypeForm::Applied {
                    declaration: fifo_decl,
                    arguments: vec![own_formal],
                },
            );
            assert_ne!(formal, own_formal);
            intern(
                &mut source,
                TypeForm::ReadViewApplied {
                    declaration: a,
                    arguments: vec![nested],
                    read_scope: a_scope,
                },
            )
        } else {
            let (b, _) = add_view(&mut source, 112, true, None);
            let b_scope = scope(ReadScopeBinder::Declaration(b), 112);
            let back = intern(
                &mut source,
                TypeForm::ReadViewApplied {
                    declaration: a,
                    arguments: Vec::new(),
                    read_scope: b_scope,
                },
            );
            let OwnerRecord::Declaration(owner) = source
                .owners
                .get_mut(&OwnerKey::Declaration(b.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::ReadView(view) = &mut owner.payload else {
                unreachable!()
            };
            view.fields[1].ty = back;
            intern(
                &mut source,
                TypeForm::ReadViewApplied {
                    declaration: b,
                    arguments: Vec::new(),
                    read_scope: a_scope,
                },
            )
        };
        let OwnerRecord::Declaration(owner) = source
            .owners
            .get_mut(&OwnerKey::Declaration(a.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::ReadView(view) = &mut owner.payload else {
            unreachable!()
        };
        view.fields[1].ty = target;
        assert!(
            admit(&source)
                .unwrap_err()
                .message
                .contains("cyclic raw view")
        );
    }
}

#[test]
fn readfrom_metadata_view_cannot_recover_its_custody_only_source() {
    let mut fixture = constructor(false);
    let owned = intern(&mut fixture.source, TypeForm::OwnedI64Cell);
    let projection = decl(&fixture.source, 3);
    let parameter = ParameterId::migrate(SEED, 3);
    let read_scope = scope(ReadScopeBinder::Declaration(projection), 3);
    let ty = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope,
        },
    );
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(kernel::ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(projection.declaration),
            name: name("view"),
            ty,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let body = expression(
        &mut fixture.source,
        4,
        ExpressionOperation::ReadViewDataField {
            value: LocalValueReference::FunctionParameter(parameter),
            field: kernel::ReadViewFieldReference {
                declaration: fixture.view,
                field: FieldId::migrate(SEED, 3),
            },
        },
    );
    declaration(
        &mut fixture.source,
        projection,
        0,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![parameter],
            result: CallableResult {
                ty: owned,
                relation: ResultRelation::ReadFrom { source: parameter },
            },
            read_scope_parameters: vec![ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, 3),
                name: name("s"),
            }],
            read_scope_sources: vec![ReadScopeSource {
                source: parameter,
                scope: read_scope,
            }],
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    assert!(admit(&fixture.source).is_err());
}

#[test]
fn all_path_admission_refuses_wrong_lineage_in_untaken_branch() {
    let mut fixture = constructor(true);
    let condition = expression(
        &mut fixture.source,
        10,
        ExpressionOperation::Bool { value: true },
    );
    let other_read = expression(
        &mut fixture.source,
        11,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(fixture.second),
        },
    );
    let false_body = expression(
        &mut fixture.source,
        12,
        ExpressionOperation::CreateReadView {
            declaration: fixture.view,
            type_arguments: Vec::new(),
            read_scope: scope(ReadScopeBinder::Declaration(fixture.function), 2),
            source: LocalValueReference::FunctionParameter(fixture.first),
            fields: vec![
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 2),
                    value: ExpressionId::migrate(SEED, 0),
                },
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 3),
                    value: other_read,
                },
            ],
        },
    );
    let body = expression(
        &mut fixture.source,
        13,
        ExpressionOperation::If {
            condition,
            when_true: fixture.body,
            when_false: false_body,
        },
    );
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.body = body;
    assert!(admit(&fixture.source).is_err());
}

#[test]
fn derived_nominal_objects_are_admitted_independently_and_cancellation_does_not_poison_retry() {
    let fixture = constructor(true);
    let cancelled = ExecutionControl::uncancelled();
    cancelled.cancel();
    assert_eq!(
        Proof::reconstruct(&[&fixture.source], &cancelled)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    let proof = Proof::reconstruct_with_types(
        &[&fixture.source],
        &fixture.source.types,
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!(proof.layout(fixture.ty).unwrap().ty, fixture.ty);
    let mut types = fixture.source.types.clone();
    types.get_mut(&fixture.ty).unwrap().contract_version = 999;
    assert!(
        Proof::reconstruct_with_types(&[&fixture.source], &types, &ExecutionControl::uncancelled())
            .is_err()
    );
    assert!(admit(&fixture.source).is_ok());

    let mut source = empty();
    let (fifo_decl, _) = fifo(&mut source, 120);
    let bytes = intern(&mut source, TypeForm::ByteBuffer);
    let one = intern(
        &mut source,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![bytes],
        },
    );
    let two = intern(
        &mut source,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![one],
        },
    );
    for checks in [1, 9, 31, 79] {
        let error = Proof::reconstruct(&[&source], &ExecutionControl::cancel_after_checks(checks))
            .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Cancelled);
        assert!(admit(&source).unwrap().prepared_transferable(two).unwrap());
    }
    // No declaration-only cache or stale caller context can grant another
    // declaration's formal authority, including an unused phantom occurrence.
    let mut foreign = source.clone();
    let other = decl(&foreign, 121);
    let (other_parameter, _) =
        type_parameter(&mut foreign, other, 121, TypeParameterConstraints::Owned);
    let stolen = intern(
        &mut foreign,
        TypeForm::TypeParameter {
            parameter: TypeParameterId::migrate(SEED, 120),
        },
    );
    record_fields(
        &mut foreign,
        other,
        121,
        true,
        vec![other_parameter],
        &[stolen],
    );
    assert!(admit(&foreign).unwrap_err().message.contains("foreign"));
    assert!(admit(&source).unwrap().prepared_transferable(two).unwrap());
}

#[test]
fn named_view_projection_follows_a_declared_reference_and_refuses_foreign_module_access() {
    let mut fixture = constructor(true);
    let projection = decl(&fixture.source, 3);
    let parameter = ParameterId::migrate(SEED, 3);
    let binding = BindingId::migrate(SEED, 3);
    let owned = intern(&mut fixture.source, TypeForm::OwnedI64Cell);
    let read_scope = scope(ReadScopeBinder::Declaration(projection), 3);
    let view_ty = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope,
        },
    );
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(kernel::ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(projection.declaration),
            name: name("view"),
            ty: view_ty,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    fixture.source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(kernel::BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("referent"),
            kind: BindingKind::OwnedBorrow,
            value: None,
            declared_type: Some(owned),
        }),
    );
    let leaf = expression(
        &mut fixture.source,
        20,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let body = expression(
        &mut fixture.source,
        21,
        ExpressionOperation::BorrowReadViewField {
            value: LocalValueReference::FunctionParameter(parameter),
            field: kernel::ReadViewFieldReference {
                declaration: fixture.view,
                field: FieldId::migrate(SEED, 3),
            },
            binding,
            body: leaf,
        },
    );
    declaration(
        &mut fixture.source,
        projection,
        0,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![parameter],
            result: CallableResult {
                ty: owned,
                relation: ResultRelation::ReadFrom { source: parameter },
            },
            read_scope_parameters: vec![ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, 3),
                name: name("s"),
            }],
            read_scope_sources: vec![ReadScopeSource {
                source: parameter,
                scope: read_scope,
            }],
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    assert!(
        admit(&fixture.source)
            .unwrap()
            .admits_function_flow(projection)
    );
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(projection.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    owner.module = ModuleId::migrate(SEED, 99);
    assert!(
        admit(&fixture.source)
            .unwrap_err()
            .message
            .contains("foreign module")
    );
}

#[test]
fn prospective_typed_view_let_scope_is_live_for_initializer_and_descriptor_drop() {
    let mut fixture = constructor(true);
    let binding = BindingId::migrate(SEED, 30);
    let read_scope = ReadScopeArgument::Lexical {
        callable: fixture.function,
        binding,
    };
    let ty = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope,
        },
    );
    let OwnerRecord::Expression(initializer) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.body))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::CreateReadView {
        read_scope: actual, ..
    } = &mut initializer.operation
    else {
        unreachable!()
    };
    *actual = read_scope;
    fixture.source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(kernel::BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("descriptor"),
            kind: BindingKind::ReadView,
            value: Some(fixture.body),
            declared_type: Some(ty),
        }),
    );
    let drop = expression(
        &mut fixture.source,
        31,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let body = expression(
        &mut fixture.source,
        32,
        ExpressionOperation::Let {
            bindings: vec![binding],
            body: drop,
        },
    );
    let unit = intern(&mut fixture.source, TypeForm::Unit);
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.body = body;
    function.result = CallableResult {
        ty: unit,
        relation: ResultRelation::Value,
    };
    let proof = admit(&fixture.source).unwrap();
    assert!(proof.admits_function_flow(fixture.function));
    assert_eq!(
        proof.lexical_function_for_binding(fixture.function.package, binding),
        Some(fixture.function)
    );
    assert_eq!(proof.layout(ty).unwrap().read_scope, read_scope);
    let OwnerRecord::Binding(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Binding(binding))
        .unwrap()
    else {
        unreachable!()
    };
    record.kind = BindingKind::Let;
    assert!(admit(&fixture.source).is_err());
}

#[test]
fn borrowed_view_move_drop_and_direct_borrowed_return_reject() {
    for mode in 0..3 {
        let mut fixture = constructor(false);
        let callable = decl(&fixture.source, 4);
        let parameter = ParameterId::migrate(SEED, 4);
        let read_scope = scope(ReadScopeBinder::Declaration(callable), 4);
        let ty = intern(
            &mut fixture.source,
            TypeForm::ReadViewApplied {
                declaration: fixture.view,
                arguments: Vec::new(),
                read_scope,
            },
        );
        let unit = intern(&mut fixture.source, TypeForm::Unit);
        fixture.source.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(kernel::ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(callable.declaration),
                name: name("borrowed"),
                ty,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
        let value = LocalValueReference::FunctionParameter(parameter);
        let operation = match mode {
            0 => ExpressionOperation::MoveReadView { value },
            1 => ExpressionOperation::DropReadView { value },
            _ => ExpressionOperation::Local { value },
        };
        let body = expression(&mut fixture.source, 40, operation);
        declaration(
            &mut fixture.source,
            callable,
            0,
            OwnerKind::PureFunction,
            DeclarationPayload::Function(FunctionDeclaration {
                implementation_parameters: Vec::new(),
                requirement_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                type_parameters: Vec::new(),
                parameters: vec![parameter],
                result: if mode == 1 {
                    CallableResult {
                        ty: unit,
                        relation: ResultRelation::Value,
                    }
                } else {
                    CallableResult {
                        ty,
                        relation: ResultRelation::ViewFrom {
                            source: parameter,
                            scope: read_scope,
                        },
                    }
                },
                read_scope_parameters: vec![ReadScopeParameter {
                    id: ReadScopeParameterId::migrate(SEED, 4),
                    name: name("s"),
                }],
                read_scope_sources: vec![ReadScopeSource {
                    source: parameter,
                    scope: read_scope,
                }],
                effect: FunctionEffect::Pure,
                body,
            }),
        );
        assert!(admit(&fixture.source).is_err(), "mode {mode}");
    }
}

#[test]
fn runtime_queries_use_the_runtime_control_without_poisoning_admitted_source() {
    let fixture = constructor(true);
    let proof = admit(&fixture.source).unwrap();
    let control = ExecutionControl::uncancelled();
    control.cancel();
    assert_eq!(
        proof
            .layout_in_with_control(fixture.ty, &fixture.source.types, &control)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(
        proof
            .category_in_with_control(fixture.ty, &fixture.source.types, &control)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    assert_eq!(
        proof
            .read_shareable_in_with_control(fixture.ty, &fixture.source.types, &control)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    assert!(proof.layout(fixture.ty).is_ok());
}

#[test]
fn prepared_cache_follows_derived_metadata_and_nested_scope_specializations() {
    let mut source = empty();
    let (leaf, _) = add_view(
        &mut source,
        1,
        false,
        Some(TypeParameterConstraints::Transferable),
    );
    let formal = intern(
        &mut source,
        TypeForm::TypeParameter {
            parameter: TypeParameterId::migrate(SEED, 1),
        },
    );
    let symbolic_list = intern(&mut source, TypeForm::List { item: formal });
    let OwnerRecord::Declaration(leaf_owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(leaf.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(leaf_view) = &mut leaf_owner.payload else {
        unreachable!()
    };
    leaf_view.fields[0].ty = symbolic_list;
    let scalar = intern(&mut source, TypeForm::I64);
    let (middle, _) = add_view(&mut source, 2, false, None);
    let middle_scope = scope(ReadScopeBinder::Declaration(middle), 2);
    let leaf_in_middle = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: leaf,
            arguments: vec![scalar],
            read_scope: middle_scope,
        },
    );
    let OwnerRecord::Declaration(middle_owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(middle.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(middle_view) = &mut middle_owner.payload else {
        unreachable!()
    };
    middle_view.fields.push(ReadViewField {
        id: FieldId::migrate(SEED, 90),
        name: name("leaf"),
        ty: leaf_in_middle,
        kind: ReadViewFieldKind::ReadReference {
            read_scope: middle_scope,
        },
    });
    let (outer, outer_ty) = add_view(&mut source, 3, false, None);
    let outer_scope = scope(ReadScopeBinder::Declaration(outer), 3);
    let middle_in_outer = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: middle,
            arguments: Vec::new(),
            read_scope: outer_scope,
        },
    );
    let OwnerRecord::Declaration(outer_owner) = source
        .owners
        .get_mut(&OwnerKey::Declaration(outer.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::ReadView(outer_view) = &mut outer_owner.payload else {
        unreachable!()
    };
    outer_view.fields.push(ReadViewField {
        id: FieldId::migrate(SEED, 91),
        name: name("middle"),
        ty: middle_in_outer,
        kind: ReadViewFieldKind::ReadReference {
            read_scope: outer_scope,
        },
    });
    let object = TypeObject::new(TypeForm::List { item: scalar }).unwrap();
    let closed_list = encode_type_object(&object).unwrap().0;
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration: leaf,
        arguments: vec![scalar],
        read_scope: outer_scope,
    })
    .unwrap();
    let leaf_in_outer = encode_type_object(&object).unwrap().0;
    assert!(!source.types.contains_key(&closed_list));
    assert!(!source.types.contains_key(&leaf_in_outer));
    let proof =
        Proof::reconstruct_with_types(&[&source], &source.types, &ExecutionControl::uncancelled())
            .unwrap();
    assert!(proof.types().contains_key(&closed_list));
    assert!(proof.types().contains_key(&leaf_in_outer));
    assert_eq!(
        proof.prepared_layout(middle_in_outer).unwrap().fields[1].ty,
        leaf_in_outer
    );
    assert_eq!(
        proof.prepared_layout(leaf_in_outer).unwrap().fields[0].ty,
        closed_list
    );
    assert!(
        proof
            .prepared_layout(leaf_in_outer)
            .unwrap()
            .types
            .is_empty()
    );
    assert_eq!(
        proof.prepared_category(closed_list).unwrap(),
        ValueCategory::OrdinaryData
    );
    assert!(proof.prepared_metadata(closed_list).unwrap());
    assert!(proof.prepared_transferable(closed_list).unwrap());
    assert!(proof.prepared_read_shareable(outer_ty).unwrap());
    assert!(proof.prepared_read_shareable(closed_list).is_err());
}

#[test]
fn prepared_queries_are_immutable_exact_lookups_and_do_not_reuse_preparation_controls() {
    let fixture = constructor(true);
    let control = ExecutionControl::uncancelled();
    let proof =
        Proof::reconstruct_with_types(&[&fixture.source], &fixture.source.types, &control).unwrap();
    let first = proof.prepared_layout(fixture.ty).unwrap();
    let address = std::ptr::from_ref(first);
    let scalar = first.fields[0].ty;
    let owned = first.fields[1].ty;
    let original_count = proof.types().len();
    control.cancel();
    assert_eq!(
        std::ptr::from_ref(proof.prepared_layout(fixture.ty).unwrap()),
        address
    );
    assert_eq!(
        proof.prepared_category(fixture.ty).unwrap(),
        ValueCategory::ReadView
    );
    assert!(proof.prepared_metadata(scalar).unwrap());
    assert!(!proof.prepared_metadata(owned).unwrap());
    assert!(proof.prepared_read_shareable(fixture.ty).unwrap());
    assert_eq!(proof.types().len(), original_count);
    let unknown = encode_type_object(
        &TypeObject::new(TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: vec![owned],
            read_scope: scope(ReadScopeBinder::Declaration(fixture.function), 2),
        })
        .unwrap(),
    )
    .unwrap()
    .0;
    assert!(proof.prepared_layout(unknown).is_err());
    assert!(proof.prepared_category(unknown).is_err());
}

#[test]
fn complete_prepared_closure_uses_one_work_budget_and_failure_preserves_healthy_proof() {
    let fixture = constructor(true);
    let healthy = admit(&fixture.source).unwrap();
    let control = ExecutionControl::uncancelled();
    let (mut candidate, mut budget) =
        Proof::reconstruct_raw(&[&fixture.source], &control, Budget::new(&control)).unwrap();
    budget.remaining = 0;
    assert_eq!(
        candidate
            .prepare_complete_closure(budget)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Resource
    );
    assert!(candidate.prepared_layouts.is_empty());
    assert!(candidate.prepared_facts.is_empty());
    assert!(healthy.prepared_layout(fixture.ty).is_ok());
    assert!(
        admit(&fixture.source)
            .unwrap()
            .prepared_layout(fixture.ty)
            .is_ok()
    );

    // This is an accounting control for complete copied context keys. Arbitrary
    // binding IDs in a scalar proof grant no lexical read-scope authority.
    let scalar = healthy.prepared_layout(fixture.ty).unwrap().fields[0].ty;
    let empty_context = Context::declaration(fixture.function);
    let mut large_context = Context::declaration(fixture.function);
    large_context
        .lexical
        .extend((0..128).map(|ordinal| BindingId::migrate(SEED, 10_000 + ordinal)));
    let measure = |context: &Context| {
        let mut resolver = healthy.resolver_with_control(healthy.types(), &control);
        let facts = resolver.facts(scalar, context, 0).unwrap();
        assert_eq!(facts.category, ValueCategory::OrdinaryData);
        assert!(facts.metadata && facts.transfer && facts.share);
        (
            kernel::contract::MAXIMUM_VALIDATION_WORK - resolver.budget.remaining,
            resolver.budget.entries,
        )
    };
    let (empty_work, empty_entries) = measure(&empty_context);
    let (large_work, large_entries) = measure(&large_context);
    assert!(large_entries - empty_entries >= large_context.lexical.len());
    assert!(large_work - empty_work >= large_context.lexical.len());
    let mut exact = healthy.resolver_with_control(healthy.types(), &control);
    exact.budget.remaining = large_work;
    exact.budget.entries = kernel::contract::MAXIMUM_VALIDATION_WORK - large_entries;
    let facts = exact.facts(scalar, &large_context, 0).unwrap();
    assert_eq!(facts.category, ValueCategory::OrdinaryData);
    assert!(facts.metadata && facts.transfer && facts.share);
    assert_eq!(exact.budget.remaining, 0);
    assert_eq!(
        exact.budget.entries,
        kernel::contract::MAXIMUM_VALIDATION_WORK
    );
    for storage in [false, true] {
        let mut resolver = healthy.resolver_with_control(healthy.types(), &control);
        if storage {
            resolver.budget.entries =
                kernel::contract::MAXIMUM_VALIDATION_WORK - (large_entries - 1);
        } else {
            resolver.budget.remaining = large_work - 1;
        }
        assert_eq!(
            resolver.facts(scalar, &large_context, 0).unwrap_err().class,
            ExecutionFailureClass::Resource
        );
        assert!(healthy.prepared_metadata(scalar).unwrap());
        assert_eq!(measure(&large_context), (large_work, large_entries));
    }
    // The first owned key is not allocated before its complete lexical debit
    // and cancellable prefix finish. Failure leaves private facts state empty.
    let mut resolver = healthy.resolver_with_control(healthy.types(), &control);
    resolver.budget.entries =
        kernel::contract::MAXIMUM_VALIDATION_WORK - (large_context.lexical.len() - 1);
    let mut state = FactsState::default();
    assert_eq!(
        resolver
            .facts_walk(scalar, &large_context, &mut state, 0)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Resource
    );
    assert!(state.active.is_empty() && state.observed.is_empty() && state.estimates.is_empty());
    let cancelled = ExecutionControl::cancel_after_checks(32);
    let mut resolver = healthy.resolver_with_control(healthy.types(), &cancelled);
    let mut state = FactsState::default();
    assert_eq!(
        resolver
            .facts_walk(scalar, &large_context, &mut state, 0)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Cancelled
    );
    assert!(state.active.is_empty() && state.observed.is_empty() && state.estimates.is_empty());
    assert_eq!(measure(&large_context), (large_work, large_entries));

    let mut source = empty();
    let (fifo_decl, _) = fifo(&mut source, 130);
    let bytes = intern(&mut source, TypeForm::ByteBuffer);
    let one = intern(
        &mut source,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![bytes],
        },
    );
    let two = intern(
        &mut source,
        TypeForm::Applied {
            declaration: fifo_decl,
            arguments: vec![one],
        },
    );
    let healthy = admit(&source).unwrap();
    let (mut candidate, mut budget) =
        Proof::reconstruct_raw(&[&source], &control, Budget::new(&control)).unwrap();
    budget.remaining = 2;
    assert_eq!(
        candidate
            .prepare_complete_closure(budget)
            .unwrap_err()
            .class,
        ExecutionFailureClass::Resource
    );
    assert!(candidate.prepared_facts.is_empty());
    assert!(healthy.prepared_transferable(two).unwrap());

    // Ordinary growing substitution is not a finite exact-instance SCC. The
    // depth limit follows arguments, fields, bind_types and layout together.
    let mut growing = empty();
    let owner = decl(&growing, 131);
    let (parameter, formal) =
        type_parameter(&mut growing, owner, 131, TypeParameterConstraints::None);
    let nested_actual = intern(&mut growing, TypeForm::List { item: formal });
    let nested = intern(
        &mut growing,
        TypeForm::Applied {
            declaration: owner,
            arguments: vec![nested_actual],
        },
    );
    let field = intern(&mut growing, TypeForm::List { item: nested });
    record_fields(&mut growing, owner, 131, false, vec![parameter], &[field]);
    assert!(
        admit(&growing)
            .unwrap_err()
            .message
            .contains("depth boundary")
    );
    assert!(admit(&source).unwrap().prepared_transferable(two).unwrap());
}

#[test]
fn ordinary_constant_binding_has_no_named_function_scope_authority() {
    let mut source = empty();
    let constant = decl(&source, 80);
    let scalar = intern(&mut source, TypeForm::I64);
    let binding = BindingId::migrate(SEED, 80);
    let initializer = expression(&mut source, 80, ExpressionOperation::I64 { value: 7 });
    source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(kernel::BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("ordinary"),
            kind: BindingKind::Let,
            value: Some(initializer),
            declared_type: Some(scalar),
        }),
    );
    let result = expression(
        &mut source,
        81,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let body = expression(
        &mut source,
        82,
        ExpressionOperation::Let {
            bindings: vec![binding],
            body: result,
        },
    );
    declaration(
        &mut source,
        constant,
        0,
        OwnerKind::Constant,
        DeclarationPayload::Constant {
            ty: scalar,
            value: body,
        },
    );
    let proof = admit(&source).unwrap();
    assert_eq!(
        proof.lexical_owners.get(&(constant.package, binding)),
        Some(&constant)
    );
    assert_eq!(
        proof.lexical_function_for_binding(constant.package, binding),
        None
    );
    assert_eq!(
        proof.lexical_function_for_binding(constant.package, BindingId::migrate(SEED, 99)),
        None
    );
}

#[test]
fn view_borrow_and_ordinary_index_interleave_before_the_owned_suffix_only() {
    let mut fixture = constructor(false);
    let function = decl(&fixture.source, 85);
    let view_parameter = ParameterId::migrate(SEED, 85);
    let index_parameter = ParameterId::migrate(SEED, 86);
    let owner_parameter = ParameterId::migrate(SEED, 87);
    let read_scope = scope(ReadScopeBinder::Declaration(function), 85);
    let view_ty = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope,
        },
    );
    let scalar = intern(&mut fixture.source, TypeForm::I64);
    let owned = intern(&mut fixture.source, TypeForm::OwnedI64Cell);
    for (id, label, ty, use_mode) in [
        (view_parameter, "view", view_ty, ParameterUse::Borrow),
        (index_parameter, "index", scalar, ParameterUse::Unrestricted),
    ] {
        fixture.source.owners.insert(
            OwnerKey::Parameter(id),
            OwnerRecord::Parameter(kernel::ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                parent: ParameterParent::Function(function.declaration),
                name: name(label),
                ty,
                use_mode,
                resource_requirement: None,
            }),
        );
    }
    let body = expression(
        &mut fixture.source,
        85,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(index_parameter),
        },
    );
    declaration(
        &mut fixture.source,
        function,
        0,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![view_parameter, index_parameter],
            result: CallableResult {
                ty: scalar,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: vec![ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, 85),
                name: name("s"),
            }],
            read_scope_sources: vec![ReadScopeSource {
                source: view_parameter,
                scope: read_scope,
            }],
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    assert!(
        admit(&fixture.source)
            .unwrap()
            .admits_function_flow(function)
    );
    fixture.source.owners.insert(
        OwnerKey::Parameter(owner_parameter),
        OwnerRecord::Parameter(kernel::ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(owner_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(function.declaration),
            name: name("owner"),
            ty: owned,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let OwnerRecord::Declaration(owner) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.parameters.insert(0, owner_parameter);
    assert!(
        admit(&fixture.source)
            .unwrap_err()
            .message
            .contains("Owned suffix")
    );
}

struct PendingCall {
    source: KernelSnapshot,
    caller: DeclarationReference,
    helper: DeclarationReference,
    binding: BindingId,
    call: ExpressionId,
    first_argument: ExpressionId,
    second_argument: ExpressionId,
    loan_body: ExpressionId,
    view_ty: TypeObjectDigest,
    owned: TypeObjectDigest,
}

fn pending_call() -> PendingCall {
    let fixture = constructor(true);
    let mut source = fixture.source;
    let caller = decl(&source, 70);
    let helper = decl(&source, 71);
    let caller_parameter = ParameterId::migrate(SEED, 70);
    let helper_parameter = ParameterId::migrate(SEED, 71);
    let binding = BindingId::migrate(SEED, 70);
    let owned = intern(&mut source, TypeForm::OwnedI64Cell);
    let unit = intern(&mut source, TypeForm::Unit);
    for (function, parameter) in [(caller, caller_parameter), (helper, helper_parameter)] {
        source.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(kernel::ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(function.declaration),
                name: name("source"),
                ty: owned,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
    }
    let pending_scope = ReadScopeArgument::Lexical {
        callable: caller,
        binding,
    };
    let view_ty = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope: pending_scope,
        },
    );
    let first_argument = expression(
        &mut source,
        71,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(caller_parameter),
        },
    );
    let second_argument = expression(
        &mut source,
        72,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(caller_parameter),
        },
    );
    let call = expression(
        &mut source,
        70,
        ExpressionOperation::Call {
            requirement_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            function: fixture.function,
            type_arguments: Vec::new(),
            read_scope_arguments: vec![pending_scope],
            arguments: vec![first_argument, second_argument],
        },
    );
    source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(kernel::BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: name("adopted"),
            kind: BindingKind::ReadView,
            value: None,
            declared_type: Some(view_ty),
        }),
    );
    let loan_body = expression(
        &mut source,
        73,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let body = expression(
        &mut source,
        74,
        ExpressionOperation::BorrowCall {
            call,
            binding,
            body: loan_body,
        },
    );
    declaration(
        &mut source,
        caller,
        0,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![caller_parameter],
            result: CallableResult {
                ty: unit,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    let helper_body = expression(
        &mut source,
        75,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(helper_parameter),
        },
    );
    let helper_scope = scope(ReadScopeBinder::Declaration(helper), 71);
    declaration(
        &mut source,
        helper,
        0,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: vec![helper_parameter],
            result: CallableResult {
                ty: owned,
                relation: ResultRelation::ReadFrom {
                    source: helper_parameter,
                },
            },
            read_scope_parameters: vec![ReadScopeParameter {
                id: ReadScopeParameterId::migrate(SEED, 71),
                name: name("unused_scope"),
            }],
            read_scope_sources: vec![ReadScopeSource {
                source: helper_parameter,
                scope: helper_scope,
            }],
            effect: FunctionEffect::Pure,
            body: helper_body,
        }),
    );
    PendingCall {
        source,
        caller,
        helper,
        binding,
        call,
        first_argument,
        second_argument,
        loan_body,
        view_ty,
        owned,
    }
}

#[test]
fn borrow_call_pending_scope_is_direct_metadata_authority_and_is_hidden_from_arguments() {
    let mut fixture = pending_call();
    assert!(
        admit(&fixture.source)
            .unwrap()
            .admits_function_flow(fixture.caller)
    );
    let nested = expression(
        &mut fixture.source,
        76,
        ExpressionOperation::Call {
            requirement_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            function: fixture.helper,
            type_arguments: Vec::new(),
            read_scope_arguments: vec![ReadScopeArgument::Lexical {
                callable: fixture.caller,
                binding: fixture.binding,
            }],
            arguments: vec![fixture.first_argument],
        },
    );
    let OwnerRecord::Expression(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Call { arguments, .. } = &mut record.operation else {
        unreachable!()
    };
    arguments[0] = nested;
    assert!(
        admit(&fixture.source)
            .unwrap_err()
            .message
            .contains("outside its defining callable")
    );
}

#[test]
fn raw_scope_formals_callable_owners_and_loan_binding_kinds_are_independent() {
    for mutation in 0..3 {
        let mut fixture = pending_call();
        call_assert_local(&fixture.source);
        admit(&fixture.source).unwrap();
        if mutation == 0 {
            // An unused scope relationship still requires its exact formal.
            call_function_mut(&mut fixture.source, fixture.helper)
                .read_scope_parameters
                .clear();
        } else if mutation == 1 {
            // A named helper cannot acquire the caller's prospective binding.
            let ExpressionOperation::Call {
                read_scope_arguments,
                ..
            } = call_operation_mut(&mut fixture.source, fixture.call)
            else {
                unreachable!()
            };
            read_scope_arguments[0] = ReadScopeArgument::Lexical {
                callable: fixture.helper,
                binding: fixture.binding,
            };
        } else {
            let OwnerRecord::Binding(record) = fixture
                .source
                .owners
                .get_mut(&OwnerKey::Binding(fixture.binding))
                .unwrap()
            else {
                unreachable!()
            };
            record.kind = BindingKind::Let;
        }
        assert!(admit(&fixture.source).is_err(), "mutation {mutation}");
    }
}

#[test]
fn raw_owned_read_loan_establishes_exact_lexical_scope_without_view_operations() {
    let mut fixture = pending_call();
    let OwnerRecord::Declaration(helper) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.helper.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(helper) = &mut helper.payload else {
        unreachable!()
    };
    helper.read_scope_parameters.clear();
    helper.read_scope_sources.clear();
    let OwnerRecord::Expression(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Call {
        function,
        read_scope_arguments,
        arguments,
        ..
    } = &mut record.operation
    else {
        unreachable!()
    };
    *function = fixture.helper;
    read_scope_arguments.clear();
    arguments.truncate(1);
    fixture
        .source
        .owners
        .remove(&OwnerKey::Expression(fixture.second_argument));
    let OwnerRecord::Binding(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Binding(fixture.binding))
        .unwrap()
    else {
        unreachable!()
    };
    record.kind = BindingKind::OwnedBorrow;
    record.declared_type = Some(fixture.owned);
    let OwnerRecord::Expression(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.loan_body))
        .unwrap()
    else {
        unreachable!()
    };
    record.operation = ExpressionOperation::Unit {};
    call_assert_local(&fixture.source);
    let proof = admit(&fixture.source).unwrap();
    assert!(proof.prepared_layout(fixture.view_ty).is_ok());
    assert_eq!(
        proof.lexical_function_for_binding(fixture.caller.package, fixture.binding),
        Some(fixture.caller)
    );
    let unused_view = fixture.source.types.remove(&fixture.view_ty).unwrap();
    assert!(
        admit(&fixture.source).is_ok(),
        "the ordinary loan does not depend on an unused view type"
    );
    let TypeForm::ReadViewApplied {
        declaration,
        arguments,
        ..
    } = unused_view.form
    else {
        unreachable!()
    };
    let wrong_owner = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration,
            arguments,
            read_scope: ReadScopeArgument::Lexical {
                callable: fixture.helper,
                binding: fixture.binding,
            },
        },
    );
    assert_ne!(wrong_owner, fixture.view_ty);
    assert!(admit(&fixture.source).is_err());
}

// The controls below are synthetic raw canonical records. They do not claim
// authoring, maintained planner acceptance, or execution of the failed source.
fn call_parameter(ordinal: u64, position: u64) -> ParameterId {
    ParameterId::migrate(SEED, ordinal * 100 + position)
}
fn call_function(
    source: &mut KernelSnapshot,
    ordinal: u64,
    parameters: &[(TypeObjectDigest, ParameterUse)],
    result: TypeObjectDigest,
    effect: FunctionEffect,
    body: ExpressionId,
) -> DeclarationReference {
    let reference = decl(source, ordinal);
    let mut ids = Vec::new();
    for (position, (ty, use_mode)) in parameters.iter().enumerate() {
        let id = call_parameter(ordinal, position as u64);
        source.owners.insert(
            OwnerKey::Parameter(id),
            OwnerRecord::Parameter(kernel::ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(id), OwnerKind::Parameter),
                parent: ParameterParent::Function(reference.declaration),
                name: name(&format!("argument_{position}")),
                ty: *ty,
                use_mode: *use_mode,
                resource_requirement: None,
            }),
        );
        ids.push(id);
    }
    let kind = if matches!(effect, FunctionEffect::Pure) {
        OwnerKind::PureFunction
    } else {
        OwnerKind::TaskFunction
    };
    declaration(
        source,
        reference,
        0,
        kind,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: Vec::new(),
            requirement_parameters: Vec::new(),
            effect_parameters: Vec::new(),
            type_parameters: Vec::new(),
            parameters: ids,
            result: CallableResult {
                ty: result,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect,
            body,
        }),
    );
    reference
}
fn call_function_mut(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut FunctionDeclaration {
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function
}
fn call_operation_mut(source: &mut KernelSnapshot, id: ExpressionId) -> &mut ExpressionOperation {
    let OwnerRecord::Expression(record) = source.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
    else {
        unreachable!()
    };
    &mut record.operation
}
fn call_local(source: &mut KernelSnapshot, ordinal: u64, parameter: ParameterId) -> ExpressionId {
    expression(
        source,
        ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    )
}
fn call_binding(
    source: &mut KernelSnapshot,
    ordinal: u64,
    kind: BindingKind,
    ty: Option<TypeObjectDigest>,
    value: Option<ExpressionId>,
) -> BindingId {
    let id = BindingId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Binding(id),
        OwnerRecord::Binding(kernel::BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(id), OwnerKind::Binding),
            name: name(&format!("binding_{ordinal}")),
            kind,
            value,
            declared_type: ty,
        }),
    );
    id
}
fn call_effect(
    source: &mut KernelSnapshot,
    owner: DeclarationReference,
    ordinal: u64,
) -> kernel::EffectParameterReference {
    let id = crate::platform::semantic_id::EffectParameterId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::EffectParameter(id),
        OwnerRecord::EffectParameter(kernel::EffectParameterRecord {
            header: OwnerHeader::new(OwnerKey::EffectParameter(id), OwnerKind::EffectParameter),
            declaration: owner.declaration,
            name: name(&format!("E{ordinal}")),
        }),
    );
    kernel::EffectParameterReference {
        package: owner.package,
        parameter: id,
    }
}
fn call_requirement(
    source: &mut KernelSnapshot,
    owner: DeclarationReference,
    interface: DeclarationReference,
    ordinal: u64,
) -> kernel::RequirementOperand {
    let id = crate::platform::semantic_id::RequirementParameterId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::RequirementParameter(id),
        OwnerRecord::RequirementParameter(kernel::RequirementParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::RequirementParameter(id),
                OwnerKind::RequirementParameter,
            ),
            declaration: owner.declaration,
            name: name(&format!("R{ordinal}")),
            constraint: kernel::RequirementConstraint {
                interface,
                operations: Vec::new(),
            },
        }),
    );
    kernel::RequirementOperand::Parameter(kernel::RequirementParameterReference {
        package: owner.package,
        parameter: id,
    })
}
fn call_named(
    source: &mut KernelSnapshot,
    ordinal: u64,
    function: DeclarationReference,
    arguments: Vec<ExpressionId>,
) -> ExpressionId {
    expression(
        source,
        ordinal,
        ExpressionOperation::Call {
            function,
            type_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments,
        },
    )
}

fn call_interface(source: &mut KernelSnapshot, interface: DeclarationReference, ordinal: u64) {
    let operation = crate::platform::semantic_id::OperationId::migrate(SEED, ordinal);
    let unit = intern(source, TypeForm::Unit);
    source.owners.insert(
        OwnerKey::Operation(operation),
        OwnerRecord::Operation(kernel::OperationRecord {
            header: OwnerHeader::new(OwnerKey::Operation(operation), OwnerKind::Operation),
            declaration: interface.declaration,
            name: name("probe"),
            parameters: Vec::new(),
            result: unit,
            idempotency: kernel::Idempotency::Idempotent,
            external_visibility: kernel::ExternalVisibility::None,
        }),
    );
    declaration(
        source,
        interface,
        0,
        OwnerKind::Interface,
        DeclarationPayload::Interface {
            operations: vec![operation],
        },
    );
}

fn call_assert_local(source: &KernelSnapshot) {
    // These are necessary local premises for healthy synthetic raw fixtures.
    // Raw Proof still independently decides topology, types, bounds and flow.
    for (ty, object) in &source.types {
        object
            .validate_local()
            .unwrap_or_else(|error| panic!("healthy fixture type {ty}: {error:?}"));
    }
    for (owner, record) in &source.owners {
        record
            .validate_local()
            .unwrap_or_else(|error| panic!("healthy fixture owner {owner:?}: {error:?}"));
    }
}

struct RowCall {
    source: KernelSnapshot,
    caller: DeclarationReference,
    callee: DeclarationReference,
    call: ExpressionId,
    argument: ExpressionId,
    scalar: TypeObjectDigest,
    actual_callback: TypeObjectDigest,
    caller_effects: [kernel::EffectParameterReference; 2],
    callee_effects: [kernel::EffectParameterReference; 2],
    caller_requirements: [kernel::RequirementOperand; 2],
    callee_requirements: [kernel::RequirementOperand; 2],
}
fn row_call() -> RowCall {
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let caller = decl(&source, 1000);
    let callee = decl(&source, 1001);
    let interface = decl(&source, 1002);
    call_interface(&mut source, interface, 1002);
    let caller_effects = [
        call_effect(&mut source, caller, 1000),
        call_effect(&mut source, caller, 1001),
    ];
    let callee_effects = [
        call_effect(&mut source, callee, 1002),
        call_effect(&mut source, callee, 1003),
    ];
    let caller_requirements = [
        call_requirement(&mut source, caller, interface, 1000),
        call_requirement(&mut source, caller, interface, 1001),
    ];
    let callee_requirements = [
        call_requirement(&mut source, callee, interface, 1002),
        call_requirement(&mut source, callee, interface, 1003),
    ];
    let actual_callback = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: vec![scalar],
            result: scalar,
            effect: kernel::EffectRow {
                requirements: vec![caller_requirements[0]],
                parameters: vec![caller_effects[0]],
            },
        },
    );
    let template = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: vec![scalar],
            result: scalar,
            effect: kernel::EffectRow {
                requirements: vec![callee_requirements[0]],
                parameters: vec![callee_effects[0]],
            },
        },
    );
    let result = expression(&mut source, 1000, ExpressionOperation::I64 { value: 0 });
    call_function(
        &mut source,
        1001,
        &[(template, ParameterUse::Unrestricted)],
        scalar,
        FunctionEffect::Task {
            requirements: vec![callee_requirements[0]],
            effect_parameters: vec![callee_effects[0]],
        },
        result,
    );
    let argument = call_local(&mut source, 1001, call_parameter(1000, 0));
    let call = expression(
        &mut source,
        1002,
        ExpressionOperation::Call {
            function: callee,
            type_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            effect_arguments: caller_effects
                .iter()
                .map(|effect| kernel::EffectRow {
                    requirements: Vec::new(),
                    parameters: vec![*effect],
                })
                .collect(),
            requirement_arguments: caller_requirements.to_vec(),
            arguments: vec![argument],
        },
    );
    let mut ambient = caller_effects.to_vec();
    ambient.sort();
    let mut ambient_requirements = caller_requirements.to_vec();
    ambient_requirements.sort();
    call_function(
        &mut source,
        1000,
        &[(actual_callback, ParameterUse::Unrestricted)],
        scalar,
        FunctionEffect::Task {
            requirements: ambient_requirements,
            effect_parameters: ambient,
        },
        call,
    );
    for (owner, effects, requirements) in [
        (caller, caller_effects, caller_requirements),
        (callee, callee_effects, callee_requirements),
    ] {
        let function = call_function_mut(&mut source, owner);
        function.effect_parameters = effects.iter().map(|effect| effect.parameter).collect();
        function.requirement_parameters = requirements
            .iter()
            .map(|requirement| match requirement {
                kernel::RequirementOperand::Parameter(reference) => reference.parameter,
                _ => unreachable!(),
            })
            .collect();
    }
    RowCall {
        source,
        caller,
        callee,
        call,
        argument,
        scalar,
        actual_callback,
        caller_effects,
        callee_effects,
        caller_requirements,
        callee_requirements,
    }
}

#[test]
fn raw_named_calls_forward_distinct_complete_effect_and_requirement_rows() {
    let fixture = row_call();
    call_assert_local(&fixture.source);
    admit(&fixture.source).unwrap();
    assert_ne!(fixture.caller, fixture.callee);
    assert_ne!(fixture.caller_effects[0], fixture.callee_effects[0]);
    assert_ne!(
        fixture.caller_requirements[0],
        fixture.callee_requirements[0]
    );
    // Names and equal constraints cannot substitute for exact binder identity.
    for mutant in 0..8 {
        let mut changed = fixture.source.clone();
        match mutant {
            0..=5 => {
                let ExpressionOperation::Call {
                    effect_arguments,
                    requirement_arguments,
                    ..
                } = call_operation_mut(&mut changed, fixture.call)
                else {
                    unreachable!()
                };
                match mutant {
                    0 => {
                        effect_arguments.pop();
                    }
                    1 => effect_arguments.push(kernel::EffectRow::default()),
                    2 => {
                        requirement_arguments.pop();
                    }
                    3 => requirement_arguments.push(fixture.caller_requirements[0]),
                    4 => effect_arguments[0].parameters[0] = fixture.callee_effects[0],
                    _ => requirement_arguments[0] = fixture.callee_requirements[0],
                }
            }
            6 => {
                let OwnerRecord::EffectParameter(record) = changed
                    .owners
                    .get_mut(&OwnerKey::EffectParameter(
                        fixture.callee_effects[1].parameter,
                    ))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.declaration = fixture.caller.declaration;
            }
            _ => {
                let kernel::RequirementOperand::Parameter(reference) =
                    fixture.callee_requirements[1]
                else {
                    unreachable!()
                };
                let OwnerRecord::RequirementParameter(record) = changed
                    .owners
                    .get_mut(&OwnerKey::RequirementParameter(reference.parameter))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.declaration = fixture.caller.declaration;
            }
        }
        assert!(
            admit(&changed).is_err(),
            "complete row packet mutant {mutant}"
        );
    }
    for requirements in [false, true] {
        let mut changed = fixture.source.clone();
        let ExpressionOperation::Call {
            effect_arguments,
            requirement_arguments,
            ..
        } = call_operation_mut(&mut changed, fixture.call)
        else {
            unreachable!()
        };
        if requirements {
            requirement_arguments.swap(0, 1);
        } else {
            effect_arguments.swap(0, 1);
        }
        let error = admit(&changed).unwrap_err();
        assert_eq!(error.code, "reference_read_view_admission");
        assert!(
            error
                .message
                .starts_with("call argument differs from complete substituted parameter type")
        );
        for detail in [
            format!(
                "caller={}/{}",
                fixture.caller.package, fixture.caller.declaration
            ),
            format!("argument={}", fixture.argument),
            "position=0".to_owned(),
            format!("actual={}", fixture.actual_callback),
        ] {
            assert!(
                error.message.contains(&detail),
                "missing {detail}: {}",
                error.message
            );
        }
        let mut row = kernel::EffectRow {
            requirements: vec![fixture.caller_requirements[0]],
            parameters: vec![fixture.caller_effects[0]],
        };
        if requirements {
            row.requirements[0] = fixture.caller_requirements[1];
        } else {
            row.parameters[0] = fixture.caller_effects[1];
        }
        let object = TypeObject::new(TypeForm::TaskFunction {
            parameters: vec![fixture.scalar],
            result: fixture.scalar,
            effect: row,
        })
        .unwrap();
        let expected = encode_type_object(&object).unwrap().0;
        assert!(error.message.contains(&format!("expected={expected}")));
    }
    assert!(
        admit(&fixture.source).is_ok(),
        "refusal must not poison a healthy retry"
    );
}

#[test]
fn raw_effect_substitution_precedes_opaque_nested_callback_actuals() {
    let mut source = empty();
    let unit = intern(&mut source, TypeForm::Unit);
    let caller = decl(&source, 1010);
    let callee = decl(&source, 1011);
    let caller_effect = call_effect(&mut source, caller, 1010);
    let callee_effect = call_effect(&mut source, callee, 1011);
    let (formal, formal_ty) =
        type_parameter(&mut source, callee, 1010, TypeParameterConstraints::None);
    let opaque = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: Vec::new(),
            result: unit,
            effect: kernel::EffectRow {
                requirements: Vec::new(),
                parameters: vec![caller_effect],
            },
        },
    );
    let template = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: vec![formal_ty],
            result: unit,
            effect: kernel::EffectRow {
                requirements: Vec::new(),
                parameters: vec![callee_effect],
            },
        },
    );
    let expected = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: vec![opaque],
            result: unit,
            effect: kernel::EffectRow::default(),
        },
    );
    let body = expression(&mut source, 1010, ExpressionOperation::Unit {});
    call_function(
        &mut source,
        1011,
        &[(template, ParameterUse::Unrestricted)],
        unit,
        FunctionEffect::Task {
            requirements: Vec::new(),
            effect_parameters: vec![callee_effect],
        },
        body,
    );
    let argument = call_local(&mut source, 1011, call_parameter(1010, 0));
    let call = expression(
        &mut source,
        1012,
        ExpressionOperation::Call {
            function: callee,
            type_arguments: vec![opaque],
            read_scope_arguments: Vec::new(),
            effect_arguments: vec![kernel::EffectRow::default()],
            requirement_arguments: Vec::new(),
            arguments: vec![argument],
        },
    );
    call_function(
        &mut source,
        1010,
        &[(expected, ParameterUse::Unrestricted)],
        unit,
        FunctionEffect::Task {
            requirements: Vec::new(),
            effect_parameters: vec![caller_effect],
        },
        call,
    );
    call_function_mut(&mut source, caller).effect_parameters = vec![caller_effect.parameter];
    let function = call_function_mut(&mut source, callee);
    function.type_parameters = vec![formal];
    function.effect_parameters = vec![callee_effect.parameter];
    call_assert_local(&source);
    let proof = admit(&source).unwrap();
    assert_eq!(proof.types()[&opaque], source.types[&opaque]);
    assert!(
        matches!(&proof.types()[&expected].form, TypeForm::TaskFunction { effect, parameters, .. } if effect == &kernel::EffectRow::default() && parameters == &[opaque])
    );
    // An equal-shaped callback with its caller row erased is still another type.
    let erased = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: Vec::new(),
            result: unit,
            effect: kernel::EffectRow::default(),
        },
    );
    let wrong = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: vec![erased],
            result: unit,
            effect: kernel::EffectRow::default(),
        },
    );
    let OwnerRecord::Parameter(parameter) = source
        .owners
        .get_mut(&OwnerKey::Parameter(call_parameter(1010, 0)))
        .unwrap()
    else {
        unreachable!()
    };
    parameter.ty = wrong;
    assert!(
        admit(&source)
            .unwrap_err()
            .message
            .starts_with("call argument differs from complete substituted parameter type")
    );
}

fn callable_flow(
    task: bool,
) -> (
    KernelSnapshot,
    DeclarationReference,
    ExpressionId,
    ExpressionId,
    BindingId,
    TypeObjectDigest,
) {
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let boolean = intern(&mut source, TypeForm::Bool);
    let callee = decl(&source, 1021);
    let body = call_local(&mut source, 1020, call_parameter(1021, 1));
    let effect = if task {
        FunctionEffect::Task {
            requirements: Vec::new(),
            effect_parameters: Vec::new(),
        }
    } else {
        FunctionEffect::Pure
    };
    call_function(
        &mut source,
        1021,
        &[
            (scalar, ParameterUse::Unrestricted),
            (boolean, ParameterUse::Unrestricted),
        ],
        boolean,
        effect.clone(),
        body,
    );
    let function_value = expression(
        &mut source,
        1021,
        ExpressionOperation::FunctionValue {
            function: callee,
            type_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
        },
    );
    let prefix = call_local(&mut source, 1022, call_parameter(1020, 0));
    let bind = expression(
        &mut source,
        1023,
        ExpressionOperation::Bind {
            callee: function_value,
            arguments: vec![prefix],
        },
    );
    let residual = intern(
        &mut source,
        if task {
            TypeForm::TaskFunction {
                parameters: vec![boolean],
                result: boolean,
                effect: kernel::EffectRow::default(),
            }
        } else {
            TypeForm::Function {
                parameters: vec![boolean],
                result: boolean,
            }
        },
    );
    let binding = call_binding(
        &mut source,
        1020,
        BindingKind::Let,
        Some(residual),
        Some(bind),
    );
    let local = expression(
        &mut source,
        1024,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let argument = expression(&mut source, 1025, ExpressionOperation::Bool { value: true });
    let invoke = expression(
        &mut source,
        1026,
        ExpressionOperation::Invoke {
            callee: local,
            arguments: vec![argument],
        },
    );
    let body = expression(
        &mut source,
        1027,
        ExpressionOperation::Let {
            bindings: vec![binding],
            body: invoke,
        },
    );
    let caller = call_function(
        &mut source,
        1020,
        &[(scalar, ParameterUse::Unrestricted)],
        boolean,
        effect,
        body,
    );
    (source, caller, bind, invoke, binding, residual)
}

#[test]
fn raw_function_values_bind_and_invoke_preserve_residual_type_and_empty_task_kind() {
    for task in [false, true] {
        let (source, caller, bind, invoke, binding, residual) = callable_flow(task);
        call_assert_local(&source);
        let proof = admit(&source).unwrap();
        assert!(matches!(proof.types()[&residual].form, TypeForm::TaskFunction { .. }) == task);
        assert_eq!(
            proof.prepared_category(residual).unwrap(),
            ValueCategory::Callable
        );
        for mutant in 0..6 {
            let mut changed = source.clone();
            match mutant {
                0 => {
                    let ExpressionOperation::Invoke { arguments, .. } =
                        call_operation_mut(&mut changed, invoke)
                    else {
                        unreachable!()
                    };
                    arguments.clear();
                    let old_argument = ExpressionId::migrate(SEED, 1025);
                    let sequence = expression(
                        &mut changed,
                        1030,
                        ExpressionOperation::Sequence {
                            items: vec![old_argument, invoke],
                        },
                    );
                    let ExpressionOperation::Let { body, .. } =
                        call_operation_mut(&mut changed, ExpressionId::migrate(SEED, 1027))
                    else {
                        unreachable!()
                    };
                    *body = sequence;
                }
                1 => {
                    let extra = expression(
                        &mut changed,
                        1031,
                        ExpressionOperation::Bool { value: false },
                    );
                    let ExpressionOperation::Invoke { arguments, .. } =
                        call_operation_mut(&mut changed, invoke)
                    else {
                        unreachable!()
                    };
                    arguments.push(extra);
                }
                2 => {
                    *call_operation_mut(&mut changed, ExpressionId::migrate(SEED, 1025)) =
                        ExpressionOperation::I64 { value: 1 };
                }
                3 => {
                    let second = expression(
                        &mut changed,
                        1032,
                        ExpressionOperation::Bool { value: true },
                    );
                    let third =
                        expression(&mut changed, 1033, ExpressionOperation::I64 { value: 0 });
                    let ExpressionOperation::Bind { arguments, .. } =
                        call_operation_mut(&mut changed, bind)
                    else {
                        unreachable!()
                    };
                    arguments.extend([second, third]);
                }
                4 => {
                    let object = source.types[&residual].clone();
                    let form = match object.form {
                        TypeForm::Function { parameters, result } => TypeForm::TaskFunction {
                            parameters,
                            result,
                            effect: kernel::EffectRow::default(),
                        },
                        TypeForm::TaskFunction {
                            parameters, result, ..
                        } => TypeForm::Function { parameters, result },
                        _ => unreachable!(),
                    };
                    let wrong = intern(&mut changed, form);
                    let OwnerRecord::Binding(record) =
                        changed.owners.get_mut(&OwnerKey::Binding(binding)).unwrap()
                    else {
                        unreachable!()
                    };
                    record.declared_type = Some(wrong);
                }
                _ => {
                    *call_operation_mut(&mut changed, ExpressionId::migrate(SEED, 1024)) =
                        ExpressionOperation::Bool { value: false };
                }
            }
            let error = admit(&changed).unwrap_err();
            if matches!(mutant, 0 | 1 | 3) {
                assert!(error.message.contains("arity"), "{}", error.message);
            }
            if mutant == 2 {
                assert!(
                    error.message.starts_with(
                        "indirect callable argument differs from exact parameter type"
                    ),
                    "{}",
                    error.message
                );
            }
        }
        if task {
            let mut pure = source.clone();
            let OwnerRecord::Declaration(record) = pure
                .owners
                .get_mut(&OwnerKey::Declaration(caller.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            record.header.kind = OwnerKind::PureFunction;
            let DeclarationPayload::Function(function) = &mut record.payload else {
                unreachable!()
            };
            function.effect = FunctionEffect::Pure;
            let error = admit(&pure).unwrap_err();
            assert!(
                error.message.contains("pure") && error.message.contains("task"),
                "{}",
                error.message
            );
        }
        assert!(admit(&source).is_ok());
    }
    // A descriptor may mention Secret, but its instantiated ordinary type
    // packet must not import Owned, capabilities, or a scoped view.
    for memory in 0..4 {
        let mut fixture = constructor(false);
        for ordinal in [0, 2] {
            fixture
                .source
                .owners
                .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, ordinal)));
        }
        let scalar = intern(&mut fixture.source, TypeForm::I64);
        let actual = match memory {
            0 => intern(&mut fixture.source, TypeForm::Secret),
            1 => intern(&mut fixture.source, TypeForm::OwnedI64Cell),
            2 => {
                let interface = decl(&fixture.source, 1162);
                call_interface(&mut fixture.source, interface, 1162);
                intern(
                    &mut fixture.source,
                    TypeForm::CapabilityResource { interface },
                )
            }
            _ => fixture.ty,
        };
        let target = decl(&fixture.source, 1160);
        let (formal, _) = type_parameter(
            &mut fixture.source,
            target,
            1160,
            TypeParameterConstraints::None,
        );
        let target_body = expression(
            &mut fixture.source,
            1160,
            ExpressionOperation::I64 { value: 1 },
        );
        call_function(
            &mut fixture.source,
            1160,
            &[],
            scalar,
            FunctionEffect::Pure,
            target_body,
        );
        call_function_mut(&mut fixture.source, target).type_parameters = vec![formal];
        let body = expression(
            &mut fixture.source,
            1161,
            ExpressionOperation::FunctionValue {
                function: target,
                type_arguments: vec![actual],
                read_scope_arguments: Vec::new(),
                effect_arguments: Vec::new(),
                requirement_arguments: Vec::new(),
            },
        );
        let descriptor = intern(
            &mut fixture.source,
            TypeForm::Function {
                parameters: Vec::new(),
                result: scalar,
            },
        );
        let function = call_function_mut(&mut fixture.source, fixture.function);
        function.body = body;
        function.result = CallableResult {
            ty: descriptor,
            relation: ResultRelation::Value,
        };
        if memory == 0 {
            call_assert_local(&fixture.source);
            admit(&fixture.source).unwrap();
        } else {
            let error = admit(&fixture.source).unwrap_err();
            assert!(
                error
                    .message
                    .starts_with("first-class callable type actual contains memory/view/resource"),
                "memory packet {memory}: {}",
                error.message
            );
        }
    }
    assert_eq!(
        TypeObject::new(TypeForm::StructuralRecord { fields: Vec::new() })
            .unwrap_err()
            .code,
        "kernel_type_child_count"
    );
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let record_type = intern(
        &mut source,
        TypeForm::StructuralRecord {
            fields: vec![StructuralTypeField {
                name: name("value"),
                ty: scalar,
            }],
        },
    );
    let value = expression(&mut source, 1033, ExpressionOperation::I64 { value: 1 });
    let body = expression(
        &mut source,
        1032,
        ExpressionOperation::Record {
            nominal_type: None,
            type_arguments: Vec::new(),
            fields: vec![kernel::RecordExpressionField {
                selector: kernel::FieldSelector::Structural(name("value")),
                value,
            }],
        },
    );
    call_function(
        &mut source,
        1032,
        &[],
        record_type,
        FunctionEffect::Pure,
        body,
    );
    call_assert_local(&source);
    admit(&source).unwrap();
    let mut anonymous = source.clone();
    *call_operation_mut(&mut anonymous, body) = ExpressionOperation::Record {
        nominal_type: None,
        type_arguments: Vec::new(),
        fields: Vec::new(),
    };
    // Keep the previous field expression connected while the alternate body
    // independently derives the unsupported empty anonymous record type.
    let alternate = expression(
        &mut anonymous,
        1034,
        ExpressionOperation::Sequence {
            items: vec![value, body],
        },
    );
    let owner = decl(&anonymous, 1032);
    call_function_mut(&mut anonymous, owner).body = alternate;
    let error = admit(&anonymous).unwrap_err();
    assert!(
        error
            .message
            .starts_with("invalid independently substituted type"),
        "{}",
        error.message
    );
    let unit = intern(&mut source, TypeForm::Unit);
    let owner = decl(&source, 1032);
    call_function_mut(&mut source, owner).result.ty = unit;
    let error = admit(&source).unwrap_err();
    assert!(
        error
            .message
            .starts_with("function body/result canonical types differ"),
        "nonempty structural record is not Unit: {}",
        error.message
    );
}

fn capture_flow(capture: TypeForm) -> (KernelSnapshot, DeclarationReference, TypeObjectDigest) {
    let mut source = empty();
    let captured = intern(&mut source, capture);
    let boolean = intern(&mut source, TypeForm::Bool);
    let callable = intern(
        &mut source,
        TypeForm::Function {
            parameters: vec![captured, boolean],
            result: boolean,
        },
    );
    let residual = intern(
        &mut source,
        TypeForm::Function {
            parameters: vec![boolean],
            result: boolean,
        },
    );
    let callee = call_local(&mut source, 1040, call_parameter(1040, 0));
    let argument = call_local(&mut source, 1041, call_parameter(1040, 1));
    let bind = expression(
        &mut source,
        1042,
        ExpressionOperation::Bind {
            callee,
            arguments: vec![argument],
        },
    );
    let owner = call_function(
        &mut source,
        1040,
        &[
            (callable, ParameterUse::Unrestricted),
            (captured, ParameterUse::Unrestricted),
        ],
        residual,
        FunctionEffect::Pure,
        bind,
    );
    (source, owner, residual)
}

#[test]
fn raw_bind_capture_safety_is_independent_of_callable_metadata_and_transfer() {
    let (ordinary, _, _) = capture_flow(TypeForm::I64);
    call_assert_local(&ordinary);
    admit(&ordinary).unwrap();
    for capture in [
        TypeForm::Secret,
        TypeForm::Stream {
            item: encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
                .unwrap()
                .0,
        },
    ] {
        let (mut source, _, _) = capture_flow(capture);
        intern(&mut source, TypeForm::I64);
        assert!(
            admit(&source).is_err(),
            "ordinary unsafe stored capture must refuse"
        );
    }
    for task in [false, true] {
        let mut source = empty();
        let secret = intern(&mut source, TypeForm::Secret);
        let boolean = intern(&mut source, TypeForm::Bool);
        let owner = decl(&source, 1041);
        let effect = call_effect(&mut source, owner, 1040);
        let row = kernel::EffectRow {
            requirements: Vec::new(),
            parameters: vec![effect],
        };
        // Capturing this descriptor neither reads Secret nor executes its row.
        let descriptor = intern(
            &mut source,
            if task {
                TypeForm::TaskFunction {
                    parameters: vec![secret],
                    result: secret,
                    effect: row,
                }
            } else {
                TypeForm::Function {
                    parameters: vec![secret],
                    result: secret,
                }
            },
        );
        let callee_ty = intern(
            &mut source,
            TypeForm::Function {
                parameters: vec![descriptor, boolean],
                result: boolean,
            },
        );
        let residual = intern(
            &mut source,
            TypeForm::Function {
                parameters: vec![boolean],
                result: boolean,
            },
        );
        let callee = call_local(&mut source, 1043, call_parameter(1041, 0));
        let captured = call_local(&mut source, 1044, call_parameter(1041, 1));
        let bind = expression(
            &mut source,
            1045,
            ExpressionOperation::Bind {
                callee,
                arguments: vec![captured],
            },
        );
        call_function(
            &mut source,
            1041,
            &[
                (callee_ty, ParameterUse::Unrestricted),
                (descriptor, ParameterUse::Unrestricted),
            ],
            residual,
            FunctionEffect::Task {
                requirements: Vec::new(),
                effect_parameters: vec![effect],
            },
            bind,
        );
        call_function_mut(&mut source, owner).effect_parameters = vec![effect.parameter];
        call_assert_local(&source);
        let proof = admit(&source).unwrap();
        assert!(!proof.prepared_metadata(descriptor).unwrap());
        assert!(!proof.prepared_transferable(descriptor).unwrap());
        assert_eq!(proof.types()[&descriptor], source.types[&descriptor]);
        assert!(matches!(
            proof.types()[&residual].form,
            TypeForm::Function { .. }
        ));
    }
    for constraint in [
        TypeParameterConstraints::None,
        TypeParameterConstraints::CaptureSafe,
    ] {
        let mut source = empty();
        let owner = decl(&source, 1042);
        let (formal, formal_ty) = type_parameter(&mut source, owner, 1042, constraint);
        let boolean = intern(&mut source, TypeForm::Bool);
        let callable = intern(
            &mut source,
            TypeForm::Function {
                parameters: vec![formal_ty, boolean],
                result: boolean,
            },
        );
        let residual = intern(
            &mut source,
            TypeForm::Function {
                parameters: vec![boolean],
                result: boolean,
            },
        );
        let callee = call_local(&mut source, 1046, call_parameter(1042, 0));
        let captured = call_local(&mut source, 1047, call_parameter(1042, 1));
        let bind = expression(
            &mut source,
            1048,
            ExpressionOperation::Bind {
                callee,
                arguments: vec![captured],
            },
        );
        call_function(
            &mut source,
            1042,
            &[
                (callable, ParameterUse::Unrestricted),
                (formal_ty, ParameterUse::Unrestricted),
            ],
            residual,
            FunctionEffect::Pure,
            bind,
        );
        call_function_mut(&mut source, owner).type_parameters = vec![formal];
        if constraint.proves_capture_safe() {
            call_assert_local(&source);
        }
        assert_eq!(admit(&source).is_ok(), constraint.proves_capture_safe());
        if constraint.proves_capture_safe() {
            let OwnerRecord::TypeParameter(record) = source
                .owners
                .get_mut(&OwnerKey::TypeParameter(formal))
                .unwrap()
            else {
                unreachable!()
            };
            record.declaration = DeclarationId::migrate(SEED, 1043);
            assert!(
                admit(&source).is_err(),
                "a foreign CaptureSafe formal grants no capture authority"
            );
        }
    }
    // The unused actual belongs to a phantom nominal argument. The sole
    // stored field is always I64 and cannot enforce this CaptureSafe bound.
    let mut phantom = empty();
    let record = decl(&phantom, 1049);
    let (formal, _) = type_parameter(
        &mut phantom,
        record,
        1049,
        TypeParameterConstraints::CaptureSafe,
    );
    let scalar = intern(&mut phantom, TypeForm::I64);
    record_fields(&mut phantom, record, 1049, false, vec![formal], &[scalar]);
    let healthy = intern(
        &mut phantom,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![scalar],
        },
    );
    call_assert_local(&phantom);
    let proof = admit(&phantom).unwrap();
    assert_eq!(
        proof.prepared_category(healthy).unwrap(),
        ValueCategory::OrdinaryData
    );
    let secret = intern(&mut phantom, TypeForm::Secret);
    intern(
        &mut phantom,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![secret],
        },
    );
    let error = admit(&phantom).unwrap_err();
    assert!(
        error
            .message
            .starts_with("type actual violates its complete CaptureSafe constraint"),
        "{}",
        error.message
    );
}

#[test]
fn raw_generic_match_specializes_payload_and_checks_complete_authored_case_inventory() {
    let mut source = empty();
    let variant = decl(&source, 1050);
    let (formal, formal_ty) =
        type_parameter(&mut source, variant, 1050, TypeParameterConstraints::None);
    let scalar = intern(&mut source, TypeForm::I64);
    let boolean = intern(&mut source, TypeForm::Bool);
    let payload_case = crate::platform::semantic_id::CaseId::migrate(SEED, 1050);
    let empty_case = crate::platform::semantic_id::CaseId::migrate(SEED, 1051);
    for (id, label, payload) in [
        (payload_case, "some", Some(formal_ty)),
        (empty_case, "none", None),
    ] {
        source.owners.insert(
            OwnerKey::Case(id),
            OwnerRecord::Case(kernel::CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(id), OwnerKind::Case),
                declaration: variant.declaration,
                name: name(label),
                payload,
            }),
        );
    }
    let mut cases = vec![payload_case, empty_case];
    cases.sort();
    declaration(
        &mut source,
        variant,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: vec![formal],
            cases,
        },
    );
    let instance = intern(
        &mut source,
        TypeForm::Applied {
            declaration: variant,
            arguments: vec![scalar],
        },
    );
    let identity_body = call_local(&mut source, 1050, call_parameter(1052, 0));
    let identity = call_function(
        &mut source,
        1052,
        &[(scalar, ParameterUse::Unrestricted)],
        scalar,
        FunctionEffect::Pure,
        identity_body,
    );
    let binding = call_binding(
        &mut source,
        1050,
        BindingKind::MatchPayload,
        Some(scalar),
        None,
    );
    let local = expression(
        &mut source,
        1051,
        ExpressionOperation::Local {
            value: LocalValueReference::MatchPayload(binding),
        },
    );
    let payload_body = call_named(&mut source, 1052, identity, vec![local]);
    let empty_body = expression(&mut source, 1053, ExpressionOperation::I64 { value: 0 });
    let input = call_local(&mut source, 1054, call_parameter(1051, 0));
    let matched = expression(
        &mut source,
        1055,
        ExpressionOperation::Match {
            value: input,
            arms: vec![
                kernel::MatchExpressionArm {
                    case: kernel::CaseReference {
                        package: variant.package,
                        case: payload_case,
                    },
                    payload_binding: Some(binding),
                    body: payload_body,
                },
                kernel::MatchExpressionArm {
                    case: kernel::CaseReference {
                        package: variant.package,
                        case: empty_case,
                    },
                    payload_binding: None,
                    body: empty_body,
                },
            ],
        },
    );
    let ExpressionOperation::Match { arms, .. } = call_operation_mut(&mut source, matched) else {
        unreachable!()
    };
    arms.sort_by_key(|arm| arm.case);
    let payload_position = arms
        .iter()
        .position(|arm| arm.case.case == payload_case)
        .unwrap();
    let empty_position = arms
        .iter()
        .position(|arm| arm.case.case == empty_case)
        .unwrap();
    call_function(
        &mut source,
        1051,
        &[(instance, ParameterUse::Unrestricted)],
        scalar,
        FunctionEffect::Pure,
        matched,
    );
    call_assert_local(&source);
    admit(&source).unwrap();
    let foreign = decl(&source, 1053);
    let foreign_case = crate::platform::semantic_id::CaseId::migrate(SEED, 1052);
    source.owners.insert(
        OwnerKey::Case(foreign_case),
        OwnerRecord::Case(kernel::CaseRecord {
            header: OwnerHeader::new(OwnerKey::Case(foreign_case), OwnerKind::Case),
            declaration: foreign.declaration,
            name: name("some"),
            payload: Some(scalar),
        }),
    );
    declaration(
        &mut source,
        foreign,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: Vec::new(),
            cases: vec![foreign_case],
        },
    );
    call_assert_local(&source);
    admit(&source).unwrap();
    for mutant in 0..7 {
        let mut changed = source.clone();
        if mutant == 5 {
            let OwnerRecord::Binding(record) =
                changed.owners.get_mut(&OwnerKey::Binding(binding)).unwrap()
            else {
                unreachable!()
            };
            record.declared_type = Some(boolean);
        } else {
            let ExpressionOperation::Match { arms, .. } = call_operation_mut(&mut changed, matched)
            else {
                unreachable!()
            };
            match mutant {
                0 => {
                    arms.pop();
                }
                1 => arms.push(arms[0].clone()),
                2 => arms.swap(0, 1),
                3 => arms[payload_position].case.case = foreign_case,
                4 => arms[payload_position].payload_binding = None,
                _ => arms[empty_position].payload_binding = Some(binding),
            }
        }
        let error = admit(&changed).unwrap_err();
        let expected = match mutant {
            0..=3 => "match arms change complete exact case identity/order/inventory",
            4 | 6 => "match payload binding differs from exact case payload presence",
            _ => "match payload annotation differs from substituted case type",
        };
        assert!(
            error.message.starts_with(expected),
            "generic match mutant {mutant}: {}",
            error.message
        );
    }
    assert!(admit(&source).is_ok());
}

#[test]
fn raw_parallel_constructs_exact_structural_or_owned_product_result() {
    for owned in [false, true] {
        let mut source = empty();
        let scalar = intern(&mut source, TypeForm::I64);
        let boolean = intern(&mut source, TypeForm::Bool);
        let left_ty = if owned {
            intern(&mut source, TypeForm::OwnedI64Cell)
        } else {
            scalar
        };
        let left_body = if owned {
            call_local(&mut source, 1060, call_parameter(1061, 0))
        } else {
            expression(&mut source, 1060, ExpressionOperation::I64 { value: 4 })
        };
        let task = FunctionEffect::Task {
            requirements: Vec::new(),
            effect_parameters: Vec::new(),
        };
        let left_parameters = if owned {
            vec![(left_ty, ParameterUse::Consume)]
        } else {
            Vec::new()
        };
        let left_fn = call_function(
            &mut source,
            1061,
            &left_parameters,
            left_ty,
            task.clone(),
            left_body,
        );
        let right_body = expression(&mut source, 1061, ExpressionOperation::Bool { value: true });
        let right_fn = call_function(&mut source, 1062, &[], boolean, task.clone(), right_body);
        let arguments = if owned {
            vec![call_local(&mut source, 1062, call_parameter(1060, 0))]
        } else {
            Vec::new()
        };
        let left = call_named(&mut source, 1063, left_fn, arguments);
        let right = call_named(&mut source, 1064, right_fn, Vec::new());
        let body = expression(
            &mut source,
            1065,
            ExpressionOperation::Parallel { left, right },
        );
        let fields = vec![
            StructuralTypeField {
                name: name("left"),
                ty: left_ty,
            },
            StructuralTypeField {
                name: name("right"),
                ty: boolean,
            },
        ];
        let result = intern(
            &mut source,
            if owned {
                TypeForm::OwnedProduct {
                    fields: fields.clone(),
                }
            } else {
                TypeForm::StructuralRecord {
                    fields: fields.clone(),
                }
            },
        );
        let caller = call_function(&mut source, 1060, &left_parameters, result, task, body);
        call_assert_local(&source);
        let proof = admit(&source).unwrap();
        assert_eq!(
            proof.prepared_category(result).unwrap(),
            if owned {
                ValueCategory::Owned
            } else {
                ValueCategory::OrdinaryData
            }
        );
        for mutant in 0..3 {
            let mut changed = source.clone();
            match mutant {
                0 => {
                    let wrong = intern(
                        &mut changed,
                        if owned {
                            TypeForm::StructuralRecord {
                                fields: fields.clone(),
                            }
                        } else {
                            TypeForm::OwnedProduct {
                                fields: fields.clone(),
                            }
                        },
                    );
                    call_function_mut(&mut changed, caller).result.ty = wrong;
                }
                1 => {
                    let mut wrong_fields = fields.clone();
                    wrong_fields[1].name = name("unexpected");
                    let wrong = intern(
                        &mut changed,
                        if owned {
                            TypeForm::OwnedProduct {
                                fields: wrong_fields,
                            }
                        } else {
                            TypeForm::StructuralRecord {
                                fields: wrong_fields,
                            }
                        },
                    );
                    call_function_mut(&mut changed, caller).result.ty = wrong;
                }
                _ => {
                    let OwnerRecord::Declaration(record) = changed
                        .owners
                        .get_mut(&OwnerKey::Declaration(right_fn.declaration))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    record.header.kind = OwnerKind::PureFunction;
                    let DeclarationPayload::Function(function) = &mut record.payload else {
                        unreachable!()
                    };
                    function.effect = FunctionEffect::Pure;
                }
            }
            assert!(
                admit(&changed).is_err(),
                "parallel result mutant {mutant}, owned {owned}"
            );
        }
        assert!(admit(&source).is_ok());
    }
}

#[test]
fn raw_owned_unpack_rechecks_complete_layout_source_selectors_and_annotations() {
    for nominal in [false, true] {
        let mut source = empty();
        let cell = intern(&mut source, TypeForm::OwnedI64Cell);
        let scalar = intern(&mut source, TypeForm::I64);
        let boolean = intern(&mut source, TypeForm::Bool);
        let record = decl(&source, 1070);
        let (record_formal, formal_ty) =
            type_parameter(&mut source, record, 1070, TypeParameterConstraints::Owned);
        record_fields(
            &mut source,
            record,
            1070,
            true,
            vec![record_formal],
            &[formal_ty, scalar],
        );
        let OwnerRecord::Declaration(record_owner) = source
            .owners
            .get_mut(&OwnerKey::Declaration(record.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        record_owner.visibility = kernel::DeclarationVisibility::Public;
        let DeclarationPayload::OwnedRecord { fields, .. } = &mut record_owner.payload else {
            unreachable!()
        };
        fields.sort();
        let fields = vec![
            StructuralTypeField {
                name: name("cell"),
                ty: cell,
            },
            StructuralTypeField {
                name: name("count"),
                ty: scalar,
            },
        ];
        let receiver = intern(
            &mut source,
            if nominal {
                TypeForm::Applied {
                    declaration: record,
                    arguments: vec![cell],
                }
            } else {
                TypeForm::OwnedProduct { fields }
            },
        );
        let cell_binding = call_binding(
            &mut source,
            1070,
            BindingKind::OwnedUnpack,
            Some(cell),
            None,
        );
        let count_binding = call_binding(
            &mut source,
            1071,
            BindingKind::OwnedUnpack,
            Some(scalar),
            None,
        );
        let cell_local = expression(
            &mut source,
            1070,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(cell_binding),
            },
        );
        let count_local = expression(
            &mut source,
            1071,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(count_binding),
            },
        );
        let first_field = kernel::FieldReference {
            package: record.package,
            field: FieldId::migrate(SEED, 10700),
        };
        let second_field = kernel::FieldReference {
            package: record.package,
            field: FieldId::migrate(SEED, 10701),
        };
        let packed = expression(
            &mut source,
            1072,
            if nominal {
                ExpressionOperation::PackOwnedRecord {
                    record_type: receiver,
                    fields: vec![
                        kernel::OwnedRecordExpressionField {
                            field: first_field,
                            value: cell_local,
                        },
                        kernel::OwnedRecordExpressionField {
                            field: second_field,
                            value: count_local,
                        },
                    ],
                }
            } else {
                ExpressionOperation::PackOwned {
                    product_type: receiver,
                    fields: vec![
                        kernel::OwnedProductExpressionField {
                            name: name("cell"),
                            value: cell_local,
                        },
                        kernel::OwnedProductExpressionField {
                            name: name("count"),
                            value: count_local,
                        },
                    ],
                }
            },
        );
        if let ExpressionOperation::PackOwnedRecord { fields, .. } =
            call_operation_mut(&mut source, packed)
        {
            fields.sort_by_key(|field| field.field);
        }
        let input = call_local(&mut source, 1073, call_parameter(1071, 0));
        let unpacked = expression(
            &mut source,
            1074,
            if nominal {
                ExpressionOperation::UnpackOwnedRecord {
                    record_type: receiver,
                    source: input,
                    fields: vec![
                        kernel::OwnedRecordBinding {
                            field: first_field,
                            binding: cell_binding,
                        },
                        kernel::OwnedRecordBinding {
                            field: second_field,
                            binding: count_binding,
                        },
                    ],
                    body: packed,
                }
            } else {
                ExpressionOperation::UnpackOwned {
                    product_type: receiver,
                    source: input,
                    fields: vec![
                        kernel::OwnedProductBinding {
                            name: name("cell"),
                            binding: cell_binding,
                        },
                        kernel::OwnedProductBinding {
                            name: name("count"),
                            binding: count_binding,
                        },
                    ],
                    body: packed,
                }
            },
        );
        if let ExpressionOperation::UnpackOwnedRecord { fields, .. } =
            call_operation_mut(&mut source, unpacked)
        {
            fields.sort_by_key(|field| field.field);
        }
        let owner = call_function(
            &mut source,
            1071,
            &[(receiver, ParameterUse::Consume)],
            receiver,
            FunctionEffect::Pure,
            unpacked,
        );
        call_assert_local(&source);
        admit(&source).unwrap();
        for mutant in 0..7 {
            if mutant == 6 && !nominal {
                continue;
            }
            let mut changed = source.clone();
            match mutant {
                0..=2 => match call_operation_mut(&mut changed, unpacked) {
                    ExpressionOperation::UnpackOwned { fields, .. } => match mutant {
                        0 => {
                            fields.pop();
                        }
                        1 => fields.push(fields[0].clone()),
                        _ => fields[0].name = name("absent"),
                    },
                    ExpressionOperation::UnpackOwnedRecord { fields, .. } => match mutant {
                        0 => {
                            fields.pop();
                        }
                        1 => fields.push(fields[0].clone()),
                        _ => fields[0].field.field = FieldId::migrate(SEED, 10702),
                    },
                    _ => unreachable!(),
                },
                3 => {
                    let OwnerRecord::Binding(record) = changed
                        .owners
                        .get_mut(&OwnerKey::Binding(cell_binding))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    record.declared_type = Some(boolean);
                }
                4 => {
                    let OwnerRecord::Parameter(record) = changed
                        .owners
                        .get_mut(&OwnerKey::Parameter(call_parameter(1071, 0)))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    record.ty = cell;
                }
                5 => {
                    let OwnerRecord::Binding(record) = changed
                        .owners
                        .get_mut(&OwnerKey::Binding(cell_binding))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    record.kind = BindingKind::Let;
                }
                _ => {
                    let OwnerRecord::Declaration(record) = changed
                        .owners
                        .get_mut(&OwnerKey::Declaration(owner.declaration))
                        .unwrap()
                    else {
                        unreachable!()
                    };
                    record.module = ModuleId::migrate(SEED, 1);
                }
            }
            let error = admit(&changed).unwrap_err();
            if matches!(mutant, 0 | 1) {
                assert!(
                    error
                        .message
                        .contains("unpack omits a complete ordered field inventory"),
                    "{}",
                    error.message
                );
            }
            if matches!(mutant, 2 | 3) {
                assert!(
                    error
                        .message
                        .contains("unpack field identity/order/annotation differs"),
                    "{}",
                    error.message
                );
            }
            if mutant == 4 {
                assert!(
                    error.message.contains("source")
                        && (error.message.contains("unpack") || error.message.contains("layout")),
                    "{}",
                    error.message
                );
            }
        }
        assert!(admit(&source).is_ok());
    }
}

#[test]
fn raw_transaction_outcome_keeps_complete_standard_wrapper_shape_and_body_type() {
    let mut source = empty();
    let contract = kernel::TransactionOutcomeContract::standard().unwrap();
    source.root.package_id = contract.outcome.package;
    let unit = intern(&mut source, TypeForm::Unit);
    let scalar = intern(&mut source, TypeForm::I64);
    let boolean = intern(&mut source, TypeForm::Bool);
    for (reference, label) in [
        (contract.condition_failed, "ConditionFailed"),
        (contract.conflict, "Conflict"),
    ] {
        source.owners.insert(
            OwnerKey::Case(reference.case),
            OwnerRecord::Case(kernel::CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(reference.case), OwnerKind::Case),
                declaration: contract.abort_reason.declaration,
                name: name(label),
                payload: None,
            }),
        );
    }
    declaration(
        &mut source,
        contract.abort_reason,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: Vec::new(),
            cases: vec![contract.condition_failed.case, contract.conflict.case],
        },
    );
    let reason = intern(
        &mut source,
        TypeForm::Named {
            declaration: contract.abort_reason,
        },
    );
    let (formal, formal_ty) = type_parameter(
        &mut source,
        contract.outcome,
        1080,
        TypeParameterConstraints::None,
    );
    for (reference, label, payload) in [
        (contract.committed, "Committed", formal_ty),
        (contract.aborted, "Aborted", reason),
    ] {
        source.owners.insert(
            OwnerKey::Case(reference.case),
            OwnerRecord::Case(kernel::CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(reference.case), OwnerKind::Case),
                declaration: contract.outcome.declaration,
                name: name(label),
                payload: Some(payload),
            }),
        );
    }
    declaration(
        &mut source,
        contract.outcome,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: vec![formal],
            cases: vec![contract.committed.case, contract.aborted.case],
        },
    );
    let wrapper = intern(
        &mut source,
        TypeForm::Applied {
            declaration: contract.outcome,
            arguments: vec![scalar],
        },
    );
    let interface = DeclarationReference {
        package: source.root.package_id,
        declaration: "decl_640e96fa57dee1c09557eb4bc7b53398".parse().unwrap(),
    };
    let transaction = kernel::OperationReference {
        package: source.root.package_id,
        operation: "op_1c083402875f8f088541c27751f61d22".parse().unwrap(),
    };
    declaration(
        &mut source,
        interface,
        0,
        OwnerKind::Interface,
        DeclarationPayload::Interface {
            operations: vec![transaction.operation],
        },
    );
    source.owners.insert(
        OwnerKey::Operation(transaction.operation),
        OwnerRecord::Operation(kernel::OperationRecord {
            header: OwnerHeader::new(
                OwnerKey::Operation(transaction.operation),
                OwnerKind::Operation,
            ),
            declaration: interface.declaration,
            name: name("transaction"),
            parameters: Vec::new(),
            result: unit,
            idempotency: kernel::Idempotency::NonIdempotent,
            external_visibility: kernel::ExternalVisibility::None,
        }),
    );
    let owner = decl(&source, 1080);
    let requirement = call_requirement(&mut source, owner, interface, 1080);
    let kernel::RequirementOperand::Parameter(requirement_reference) = requirement else {
        unreachable!()
    };
    let OwnerRecord::RequirementParameter(record) = source
        .owners
        .get_mut(&OwnerKey::RequirementParameter(
            requirement_reference.parameter,
        ))
        .unwrap()
    else {
        unreachable!()
    };
    record.constraint.operations = vec![transaction];
    let binding = call_binding(
        &mut source,
        1080,
        BindingKind::Transaction,
        Some(unit),
        None,
    );
    let inner = expression(&mut source, 1080, ExpressionOperation::I64 { value: 9 });
    let body = expression(
        &mut source,
        1081,
        ExpressionOperation::TransactionOutcome {
            requirement,
            binding,
            body: inner,
            outcome: contract,
            type_argument: scalar,
        },
    );
    call_function(
        &mut source,
        1080,
        &[],
        wrapper,
        FunctionEffect::Task {
            requirements: vec![requirement],
            effect_parameters: Vec::new(),
        },
        body,
    );
    call_function_mut(&mut source, owner).requirement_parameters =
        vec![requirement_reference.parameter];
    call_assert_local(&source);
    admit(&source).unwrap();
    for mutant in 0..10 {
        let mut changed = source.clone();
        match mutant {
            0 => call_function_mut(&mut changed, owner).result.ty = scalar,
            1 => {
                *call_operation_mut(&mut changed, inner) =
                    ExpressionOperation::Bool { value: false }
            }
            2 => {
                let ExpressionOperation::TransactionOutcome { type_argument, .. } =
                    call_operation_mut(&mut changed, body)
                else {
                    unreachable!()
                };
                *type_argument = boolean;
            }
            3 => {
                let ExpressionOperation::TransactionOutcome { outcome, .. } =
                    call_operation_mut(&mut changed, body)
                else {
                    unreachable!()
                };
                outcome.committed = outcome.aborted;
            }
            4 | 5 | 8 => {
                let reference = match mutant {
                    4 => contract.committed,
                    5 => contract.aborted,
                    _ => contract.condition_failed,
                };
                let OwnerRecord::Case(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Case(reference.case))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.payload = Some(if mutant == 4 { boolean } else { unit });
            }
            6 => {
                let capability = intern(&mut changed, TypeForm::CapabilityResource { interface });
                let OwnerRecord::Binding(record) =
                    changed.owners.get_mut(&OwnerKey::Binding(binding)).unwrap()
                else {
                    unreachable!()
                };
                record.declared_type = Some(capability);
            }
            7 => {
                let OwnerRecord::RequirementParameter(record) = changed
                    .owners
                    .get_mut(&OwnerKey::RequirementParameter(
                        requirement_reference.parameter,
                    ))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.constraint.operations.clear();
            }
            _ => {
                let extra = crate::platform::semantic_id::CaseId::migrate(SEED, 1080);
                changed.owners.insert(
                    OwnerKey::Case(extra),
                    OwnerRecord::Case(kernel::CaseRecord {
                        header: OwnerHeader::new(OwnerKey::Case(extra), OwnerKind::Case),
                        declaration: contract.outcome.declaration,
                        name: name("unexpected"),
                        payload: None,
                    }),
                );
                let OwnerRecord::Declaration(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Declaration(contract.outcome.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                let DeclarationPayload::Variant { cases, .. } = &mut record.payload else {
                    unreachable!()
                };
                cases.push(extra);
                cases.sort();
            }
        }
        assert!(
            admit(&changed).is_err(),
            "transaction wrapper mutant {mutant}"
        );
    }
    assert!(admit(&source).is_ok());
}

#[test]
fn raw_borrowed_owned_match_arm_keeps_function_scope_and_view_source_protection() {
    let mut fixture = constructor(true);
    // Replace the constructor body and its complete subtree, rather than leave
    // unattached syntax for the independent expression-inventory audit.
    for ordinal in 0..=2 {
        fixture
            .source
            .owners
            .remove(&OwnerKey::Expression(ExpressionId::migrate(SEED, ordinal)));
    }
    let cell = intern(&mut fixture.source, TypeForm::OwnedI64Cell);
    let unit = intern(&mut fixture.source, TypeForm::Unit);
    let choice = intern(
        &mut fixture.source,
        TypeForm::OwnedChoice {
            cases: vec![
                StructuralTypeField {
                    name: name("first"),
                    ty: cell,
                },
                StructuralTypeField {
                    name: name("second"),
                    ty: cell,
                },
            ],
        },
    );
    let OwnerRecord::Parameter(first) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Parameter(fixture.first))
        .unwrap()
    else {
        unreachable!()
    };
    first.ty = choice;
    let arm = call_binding(
        &mut fixture.source,
        1090,
        BindingKind::OwnedBorrow,
        Some(cell),
        None,
    );
    let lexical = ReadScopeArgument::Lexical {
        callable: fixture.function,
        binding: arm,
    };
    let view = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope: lexical,
        },
    );
    let source = expression(
        &mut fixture.source,
        1090,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(arm),
        },
    );
    let length = expression(
        &mut fixture.source,
        1091,
        ExpressionOperation::I64 { value: 1 },
    );
    let create = expression(
        &mut fixture.source,
        1092,
        ExpressionOperation::CreateReadView {
            declaration: fixture.view,
            type_arguments: Vec::new(),
            read_scope: lexical,
            source: LocalValueReference::LexicalBinding(arm),
            fields: vec![
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 2),
                    value: length,
                },
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 3),
                    value: source,
                },
            ],
        },
    );
    let descriptor = call_binding(
        &mut fixture.source,
        1091,
        BindingKind::ReadView,
        Some(view),
        Some(create),
    );
    let drop = expression(
        &mut fixture.source,
        1093,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(descriptor),
        },
    );
    let arm_body = expression(
        &mut fixture.source,
        1094,
        ExpressionOperation::Let {
            bindings: vec![descriptor],
            body: drop,
        },
    );
    let second_arm = call_binding(
        &mut fixture.source,
        1092,
        BindingKind::OwnedBorrow,
        Some(cell),
        None,
    );
    let second_scope = ReadScopeArgument::Lexical {
        callable: fixture.function,
        binding: second_arm,
    };
    let second_view = intern(
        &mut fixture.source,
        TypeForm::ReadViewApplied {
            declaration: fixture.view,
            arguments: Vec::new(),
            read_scope: second_scope,
        },
    );
    let second_source = expression(
        &mut fixture.source,
        1130,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(second_arm),
        },
    );
    let second_length = expression(
        &mut fixture.source,
        1131,
        ExpressionOperation::I64 { value: 1 },
    );
    let second_create = expression(
        &mut fixture.source,
        1132,
        ExpressionOperation::CreateReadView {
            declaration: fixture.view,
            type_arguments: Vec::new(),
            read_scope: second_scope,
            source: LocalValueReference::LexicalBinding(second_arm),
            fields: vec![
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 2),
                    value: second_length,
                },
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 3),
                    value: second_source,
                },
            ],
        },
    );
    let second_descriptor = call_binding(
        &mut fixture.source,
        1093,
        BindingKind::ReadView,
        Some(second_view),
        Some(second_create),
    );
    let second_drop = expression(
        &mut fixture.source,
        1133,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(second_descriptor),
        },
    );
    let second_body = expression(
        &mut fixture.source,
        1134,
        ExpressionOperation::Let {
            bindings: vec![second_descriptor],
            body: second_drop,
        },
    );
    let input = call_local(&mut fixture.source, 1095, fixture.first);
    let matched = expression(
        &mut fixture.source,
        1096,
        ExpressionOperation::MatchBorrowedOwned {
            choice_type: choice,
            source: input,
            arms: vec![
                kernel::OwnedChoiceArm {
                    name: name("first"),
                    binding: arm,
                    body: arm_body,
                },
                kernel::OwnedChoiceArm {
                    name: name("second"),
                    binding: second_arm,
                    body: second_body,
                },
            ],
        },
    );
    let function = call_function_mut(&mut fixture.source, fixture.function);
    function.body = matched;
    function.result = CallableResult {
        ty: unit,
        relation: ResultRelation::Value,
    };
    let consume_body = expression(&mut fixture.source, 1099, ExpressionOperation::Unit {});
    let consume_target = call_function(
        &mut fixture.source,
        1099,
        &[(cell, ParameterUse::Consume)],
        unit,
        FunctionEffect::Pure,
        consume_body,
    );
    call_assert_local(&fixture.source);
    let proof = admit(&fixture.source).unwrap();
    assert!(proof.admits_function_flow(fixture.function));
    assert_eq!(
        proof.lexical_function_for_binding(fixture.function.package, arm),
        Some(fixture.function)
    );
    assert_eq!(
        proof.lexical_function_for_binding(fixture.function.package, second_arm),
        Some(fixture.function)
    );
    for mutant in 0..4 {
        let mut changed = fixture.source.clone();
        match mutant {
            0 => {
                let ExpressionOperation::CreateReadView { read_scope, .. } =
                    call_operation_mut(&mut changed, create)
                else {
                    unreachable!()
                };
                *read_scope = ReadScopeArgument::Lexical {
                    callable: fixture.function,
                    binding: second_arm,
                };
            }
            1 => {
                let ExpressionOperation::CreateReadView { source, .. } =
                    call_operation_mut(&mut changed, create)
                else {
                    unreachable!()
                };
                *source = LocalValueReference::FunctionParameter(fixture.second);
            }
            2 => {
                let consumed = call_named(&mut changed, 1097, consume_target, vec![source]);
                let premature = expression(
                    &mut changed,
                    1098,
                    ExpressionOperation::Sequence {
                        items: vec![consumed, drop],
                    },
                );
                let ExpressionOperation::Let { body, .. } =
                    call_operation_mut(&mut changed, arm_body)
                else {
                    unreachable!()
                };
                *body = premature;
            }
            _ => {
                let ExpressionOperation::MatchBorrowedOwned { arms, .. } =
                    call_operation_mut(&mut changed, matched)
                else {
                    unreachable!()
                };
                arms.swap(0, 1);
            }
        }
        let error = admit(&changed).unwrap_err();
        if mutant == 2 {
            assert!(
                error
                    .message
                    .starts_with("borrowed parameter/read handle is consumed"),
                "{}",
                error.message
            );
        }
        if mutant == 3 {
            assert!(
                error
                    .message
                    .starts_with("choice path omits or duplicates inactive cases"),
                "{}",
                error.message
            );
        }
    }
    assert!(admit(&fixture.source).is_ok());
}

fn opaque_witness_implementation(
    source: &mut KernelSnapshot,
    ordinal: u64,
    contract: DeclarationReference,
    member: AssociatedTypeMemberId,
    cell: TypeObjectDigest,
    prerequisite: bool,
) -> DeclarationReference {
    let owner = decl(source, ordinal);
    let (self_parameter, self_ty) =
        type_parameter(source, owner, ordinal * 10, TypeParameterConstraints::Owned);
    let (item_parameter, item_ty) = type_parameter(
        source,
        owner,
        ordinal * 10 + 1,
        TypeParameterConstraints::None,
    );
    for (id, label) in [(self_parameter, "Self"), (item_parameter, "Payload")] {
        let OwnerRecord::TypeParameter(record) =
            source.owners.get_mut(&OwnerKey::TypeParameter(id)).unwrap()
        else {
            unreachable!()
        };
        record.name = name(label);
    }
    let target = decl(source, ordinal + 10000);
    let (target_parameter, target_ty) = type_parameter(
        source,
        target,
        ordinal * 100 + 50,
        TypeParameterConstraints::Owned,
    );
    let unit = intern(source, TypeForm::Unit);
    let body = expression(source, ordinal + 10000, ExpressionOperation::Unit {});
    call_function(
        source,
        ordinal + 10000,
        &[(target_ty, ParameterUse::Borrow)],
        unit,
        FunctionEffect::Pure,
        body,
    );
    call_function_mut(source, target).type_parameters = vec![target_parameter];
    declaration(
        source,
        owner,
        0,
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(kernel::OwnedImplementation {
            type_parameters: vec![self_parameter, item_parameter],
            contract,
            self_type: self_ty,
            type_arguments: vec![item_ty],
            implementation_parameters: if prerequisite {
                vec![kernel::ImplementationParameter {
                    id: crate::platform::semantic_id::ImplementationParameterId::migrate(
                        SEED, ordinal,
                    ),
                    name: name("inner"),
                    contract,
                    self_type: self_ty,
                    type_arguments: vec![item_ty],
                }]
            } else {
                Vec::new()
            },
            associated_types: vec![kernel::AssociatedTypeBinding {
                member,
                value: AssociatedTypeBindingValue::Owned { ty: cell },
            }],
            methods: vec![kernel::OwnedMethodImplementation {
                method: crate::platform::semantic_id::MethodId::migrate(SEED, 1100),
                function: target,
                type_arguments: vec![self_ty],
                implementations: Vec::new(),
                read_scope_arguments: Vec::new(),
            }],
        }),
    );
    owner
}

#[test]
fn raw_recursive_row_permutations_leave_concrete_witness_and_prerequisite_actuals_opaque() {
    for nested in [false, true] {
        let mut source = empty();
        let scalar = intern(&mut source, TypeForm::I64);
        let cell = intern(&mut source, TypeForm::OwnedI64Cell);
        let contract = decl(&source, 1100);
        let (self_parameter, contract_self_ty) =
            type_parameter(&mut source, contract, 1100, TypeParameterConstraints::Owned);
        let (item_parameter, _) =
            type_parameter(&mut source, contract, 1101, TypeParameterConstraints::None);
        for (id, label) in [(self_parameter, "Self"), (item_parameter, "Payload")] {
            let OwnerRecord::TypeParameter(record) =
                source.owners.get_mut(&OwnerKey::TypeParameter(id)).unwrap()
            else {
                unreachable!()
            };
            record.name = name(label);
        }
        let member = AssociatedTypeMemberId::migrate(SEED, 1100);
        let unit = intern(&mut source, TypeForm::Unit);
        let harmless = kernel::OwnedMethod {
            id: crate::platform::semantic_id::MethodId::migrate(SEED, 1100),
            name: name("inspect"),
            parameters: vec![kernel::OwnedMethodParameter {
                ty: contract_self_ty,
                use_mode: ParameterUse::Borrow,
            }],
            result: CallableResult {
                ty: unit,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
            effect: FunctionEffect::Pure,
        };
        declaration(
            &mut source,
            contract,
            0,
            OwnerKind::OwnedContract,
            DeclarationPayload::OwnedContract(kernel::OwnedContract {
                self_parameter,
                type_parameters: vec![item_parameter],
                associated_types: vec![kernel::AssociatedTypeMember {
                    id: member,
                    name: name("Block"),
                    bound: AssociatedTypeBound::Owned,
                }],
                methods: vec![harmless],
            }),
        );
        let inner = opaque_witness_implementation(&mut source, 1101, contract, member, cell, false);
        let outer = opaque_witness_implementation(&mut source, 1102, contract, member, cell, true);
        let owner = decl(&source, 1103);
        let (self_formal, self_ty) =
            type_parameter(&mut source, owner, 1102, TypeParameterConstraints::Owned);
        let (item_formal, item_ty) =
            type_parameter(&mut source, owner, 1103, TypeParameterConstraints::None);
        for (id, label) in [(self_formal, "Self"), (item_formal, "Payload")] {
            let OwnerRecord::TypeParameter(record) =
                source.owners.get_mut(&OwnerKey::TypeParameter(id)).unwrap()
            else {
                unreachable!()
            };
            record.name = name(label);
        }
        let effects = [
            call_effect(&mut source, owner, 1100),
            call_effect(&mut source, owner, 1101),
        ];
        let interface = decl(&source, 1104);
        call_interface(&mut source, interface, 1104);
        let requirements = [
            call_requirement(&mut source, owner, interface, 1100),
            call_requirement(&mut source, owner, interface, 1101),
        ];
        let callback = intern(
            &mut source,
            TypeForm::TaskFunction {
                parameters: vec![scalar],
                result: scalar,
                effect: kernel::EffectRow {
                    requirements: vec![requirements[0]],
                    parameters: vec![effects[0]],
                },
            },
        );
        let forbidden_rewrites = [(0, 1), (1, 0), (1, 1)].map(|(requirement, effect)| {
            encode_type_object(
                &TypeObject::new(TypeForm::TaskFunction {
                    parameters: vec![scalar],
                    result: scalar,
                    effect: kernel::EffectRow {
                        requirements: vec![requirements[requirement]],
                        parameters: vec![effects[effect]],
                    },
                })
                .unwrap(),
            )
            .unwrap()
            .0
        });
        for forbidden in forbidden_rewrites {
            assert!(!source.types.contains_key(&forbidden));
        }
        let witness_id =
            crate::platform::semantic_id::ImplementationParameterId::migrate(SEED, 1103);
        let projection = intern(
            &mut source,
            TypeForm::AssociatedWitnessProjection {
                scope: owner,
                parameter: witness_id,
                contract,
                member,
            },
        );
        let concrete_inner = ImplementationOperand::Concrete {
            implementation: inner,
            type_arguments: vec![cell, callback],
            implementations: Vec::new(),
        };
        let witness = if nested {
            ImplementationOperand::Concrete {
                implementation: outer,
                type_arguments: vec![cell, callback],
                implementations: vec![concrete_inner],
            }
        } else {
            concrete_inner
        };
        let first = call_local(&mut source, 1100, call_parameter(1103, 1));
        let second = call_local(&mut source, 1101, call_parameter(1103, 1));
        let call = expression(
            &mut source,
            1102,
            ExpressionOperation::ImplementationCall {
                function: owner,
                type_arguments: vec![cell, callback],
                read_scope_arguments: Vec::new(),
                effect_arguments: vec![
                    kernel::EffectRow {
                        requirements: Vec::new(),
                        parameters: vec![effects[1]],
                    },
                    kernel::EffectRow {
                        requirements: Vec::new(),
                        parameters: vec![effects[0]],
                    },
                ],
                requirement_arguments: vec![requirements[1], requirements[0]],
                implementations: vec![witness],
                arguments: vec![first, second],
            },
        );
        let scalar_result = expression(&mut source, 1103, ExpressionOperation::I64 { value: 0 });
        let body = expression(
            &mut source,
            1104,
            ExpressionOperation::Sequence {
                items: vec![call, scalar_result],
            },
        );
        let mut ambient = effects.to_vec();
        ambient.sort();
        let mut ambient_requirements = requirements.to_vec();
        ambient_requirements.sort();
        call_function(
            &mut source,
            1103,
            &[
                (projection, ParameterUse::Borrow),
                (cell, ParameterUse::Borrow),
            ],
            scalar,
            FunctionEffect::Task {
                requirements: ambient_requirements,
                effect_parameters: ambient,
            },
            body,
        );
        let function = call_function_mut(&mut source, owner);
        function.type_parameters = vec![self_formal, item_formal];
        function.effect_parameters = effects.iter().map(|effect| effect.parameter).collect();
        function.requirement_parameters = requirements
            .iter()
            .map(|operand| match operand {
                kernel::RequirementOperand::Parameter(reference) => reference.parameter,
                _ => unreachable!(),
            })
            .collect();
        function.implementation_parameters = vec![kernel::ImplementationParameter {
            id: witness_id,
            name: name("storage"),
            contract,
            self_type: self_ty,
            type_arguments: vec![item_ty],
        }];
        call_assert_local(&source);
        let proof = admit(&source).unwrap();
        assert_eq!(proof.types()[&callback], source.types[&callback]);
        for forbidden in forbidden_rewrites {
            assert!(
                !proof.types().contains_key(&forbidden),
                "unused concrete actual was rebound under recursive callee row maps; nested={nested}"
            );
        }
    }
    // A second self-call supplies V2/V1 simultaneously. Its concrete member
    // must select Product<Cell,V2>, not resubstitute that caller V2 to V1.
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let contract = decl(&source, 1210);
    let (contract_self, contract_self_ty) =
        type_parameter(&mut source, contract, 1310, TypeParameterConstraints::Owned);
    let (contract_item, _) = type_parameter(
        &mut source,
        contract,
        1311,
        TypeParameterConstraints::CaptureSafeTransferable,
    );
    for (id, label) in [(contract_self, "Self"), (contract_item, "Payload")] {
        let OwnerRecord::TypeParameter(record) =
            source.owners.get_mut(&OwnerKey::TypeParameter(id)).unwrap()
        else {
            unreachable!()
        };
        record.name = name(label);
    }
    let member = AssociatedTypeMemberId::migrate(SEED, 1210);
    let unit = intern(&mut source, TypeForm::Unit);
    let harmless = kernel::OwnedMethod {
        id: crate::platform::semantic_id::MethodId::migrate(SEED, 1100),
        name: name("inspect"),
        parameters: vec![kernel::OwnedMethodParameter {
            ty: contract_self_ty,
            use_mode: ParameterUse::Borrow,
        }],
        result: CallableResult {
            ty: unit,
            relation: ResultRelation::Value,
        },
        read_scope_parameters: Vec::new(),
        read_scope_sources: Vec::new(),
        effect: FunctionEffect::Pure,
    };
    declaration(
        &mut source,
        contract,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(kernel::OwnedContract {
            self_parameter: contract_self,
            type_parameters: vec![contract_item],
            associated_types: vec![kernel::AssociatedTypeMember {
                id: member,
                name: name("Block"),
                bound: AssociatedTypeBound::Owned,
            }],
            methods: vec![harmless],
        }),
    );
    let implementation =
        opaque_witness_implementation(&mut source, 1211, contract, member, cell, false);
    let implementation_item = TypeParameterId::migrate(SEED, 12111);
    let OwnerRecord::TypeParameter(record) = source
        .owners
        .get_mut(&OwnerKey::TypeParameter(implementation_item))
        .unwrap()
    else {
        unreachable!()
    };
    record.constraints = TypeParameterConstraints::CaptureSafeTransferable;
    let implementation_item_ty = intern(
        &mut source,
        TypeForm::TypeParameter {
            parameter: implementation_item,
        },
    );
    let product = |item| TypeForm::OwnedProduct {
        fields: vec![
            StructuralTypeField {
                name: name("cell"),
                ty: cell,
            },
            StructuralTypeField {
                name: name("item"),
                ty: item,
            },
        ],
    };
    let binding_template = intern(&mut source, product(implementation_item_ty));
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(implementation.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(record) = &mut record.payload else {
        unreachable!()
    };
    record.associated_types[0].value = AssociatedTypeBindingValue::Owned {
        ty: binding_template,
    };
    let owner = decl(&source, 1212);
    let (self_formal, self_ty) =
        type_parameter(&mut source, owner, 1312, TypeParameterConstraints::Owned);
    let (first_formal, first_ty) = type_parameter(
        &mut source,
        owner,
        1313,
        TypeParameterConstraints::CaptureSafeTransferable,
    );
    let (second_formal, second_ty) = type_parameter(
        &mut source,
        owner,
        1314,
        TypeParameterConstraints::CaptureSafeTransferable,
    );
    for (id, label) in [
        (self_formal, "Self"),
        (first_formal, "V1"),
        (second_formal, "V2"),
    ] {
        let OwnerRecord::TypeParameter(record) =
            source.owners.get_mut(&OwnerKey::TypeParameter(id)).unwrap()
        else {
            unreachable!()
        };
        record.name = name(label);
    }
    let first_product = intern(&mut source, product(first_ty));
    let second_product = intern(&mut source, product(second_ty));
    let witness_id = crate::platform::semantic_id::ImplementationParameterId::migrate(SEED, 1212);
    let projection = intern(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: owner,
            parameter: witness_id,
            contract,
            member,
        },
    );
    let arguments = [1, 2, 1]
        .into_iter()
        .enumerate()
        .map(|(position, parameter)| {
            call_local(
                &mut source,
                1210 + position as u64,
                call_parameter(1212, parameter),
            )
        })
        .collect();
    let call = expression(
        &mut source,
        1213,
        ExpressionOperation::ImplementationCall {
            function: owner,
            type_arguments: vec![cell, second_ty, first_ty],
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            implementations: vec![ImplementationOperand::Concrete {
                implementation,
                type_arguments: vec![cell, second_ty],
                implementations: Vec::new(),
            }],
            arguments,
        },
    );
    let scalar_result = expression(&mut source, 1215, ExpressionOperation::I64 { value: 0 });
    let body = expression(
        &mut source,
        1214,
        ExpressionOperation::Sequence {
            items: vec![call, scalar_result],
        },
    );
    call_function(
        &mut source,
        1212,
        &[
            (projection, ParameterUse::Borrow),
            (second_product, ParameterUse::Borrow),
            (first_product, ParameterUse::Borrow),
        ],
        scalar,
        FunctionEffect::Pure,
        body,
    );
    let function = call_function_mut(&mut source, owner);
    function.type_parameters = vec![self_formal, first_formal, second_formal];
    function.implementation_parameters = vec![kernel::ImplementationParameter {
        id: witness_id,
        name: name("storage"),
        contract,
        self_type: self_ty,
        type_arguments: vec![first_ty],
    }];
    call_assert_local(&source);
    admit(&source).unwrap();
}

fn call_concrete_requirement(
    source: &mut KernelSnapshot,
    owner: DeclarationReference,
    interface: DeclarationReference,
    ordinal: u64,
    label: &str,
    maximum: u64,
) -> kernel::RequirementOperand {
    let id = crate::platform::semantic_id::RequirementId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Requirement(id),
        OwnerRecord::Requirement(kernel::RequirementRecord {
            header: OwnerHeader::new(OwnerKey::Requirement(id), OwnerKind::Requirement),
            declaration: owner.declaration,
            name: name(label),
            interface,
            operations: Vec::new(),
            limits: vec![kernel::ResourceLimit {
                name: name("storage"),
                maximum,
                unit: kernel::ResourceUnit::Bytes,
            }],
        }),
    );
    kernel::RequirementOperand::Concrete(kernel::RequirementReference {
        package: owner.package,
        requirement: id,
    })
}

#[test]
fn raw_concrete_requirement_coverage_preserves_limit_name_unit_and_maximum_direction() {
    let mut source = empty();
    let unit = intern(&mut source, TypeForm::Unit);
    let interface = decl(&source, 1110);
    call_interface(&mut source, interface, 1110);
    let caller = decl(&source, 1111);
    let callee = decl(&source, 1112);
    let available = call_concrete_requirement(&mut source, caller, interface, 1110, "data", 50);
    let required = call_concrete_requirement(&mut source, callee, interface, 1111, "data", 100);
    let inner = expression(&mut source, 1110, ExpressionOperation::Unit {});
    call_function(
        &mut source,
        1112,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![required],
            effect_parameters: Vec::new(),
        },
        inner,
    );
    let call = call_named(&mut source, 1111, callee, Vec::new());
    call_function(
        &mut source,
        1111,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![available],
            effect_parameters: Vec::new(),
        },
        call,
    );
    call_assert_local(&source);
    admit(&source).unwrap();
    let kernel::RequirementOperand::Concrete(reference) = available else {
        unreachable!()
    };
    for mutant in 0..5 {
        let mut changed = source.clone();
        let OwnerRecord::Requirement(record) = changed
            .owners
            .get_mut(&OwnerKey::Requirement(reference.requirement))
            .unwrap()
        else {
            unreachable!()
        };
        match mutant {
            0 => record.limits[0].maximum = 150,
            1 => record.limits[0].name = name("another"),
            2 => record.limits[0].unit = kernel::ResourceUnit::Items,
            3 => record.limits.clear(),
            _ => record.name = name("other-requirement"),
        }
        let error = admit(&changed).unwrap_err();
        assert!(
            error.message.contains("row")
                || error.message.contains("requirement")
                || error.message.contains("allowance"),
            "{}",
            error.message
        );
    }
    assert!(admit(&source).is_ok());
}

#[test]
fn raw_port_expression_uses_declared_callable_kind_and_row_without_extra_component_grants() {
    let mut source = empty();
    let unit = intern(&mut source, TypeForm::Unit);
    let interface = decl(&source, 1120);
    call_interface(&mut source, interface, 1120);
    let component = decl(&source, 1121);
    let callee = decl(&source, 1122);
    let component_available =
        call_concrete_requirement(&mut source, component, interface, 1120, "data", 50);
    let component_extra =
        call_concrete_requirement(&mut source, component, interface, 1121, "extra", 50);
    let callee_required =
        call_concrete_requirement(&mut source, callee, interface, 1122, "data", 100);
    let callee_body = expression(&mut source, 1120, ExpressionOperation::Unit {});
    call_function(
        &mut source,
        1122,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![callee_required],
            effect_parameters: Vec::new(),
        },
        callee_body,
    );
    let body = call_named(&mut source, 1121, callee, Vec::new());
    let descriptor_body = expression(&mut source, 1122, ExpressionOperation::Unit {});
    let descriptor_function = call_function(
        &mut source,
        1123,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![component_available],
            effect_parameters: Vec::new(),
        },
        descriptor_body,
    );
    let descriptor = expression(
        &mut source,
        1123,
        ExpressionOperation::FunctionValue {
            function: descriptor_function,
            type_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
        },
    );
    let port_body = expression(
        &mut source,
        1124,
        ExpressionOperation::Sequence {
            items: vec![body, descriptor],
        },
    );
    let signature = intern(
        &mut source,
        TypeForm::TaskFunction {
            parameters: Vec::new(),
            result: unit,
            effect: kernel::EffectRow {
                requirements: vec![component_available],
                parameters: Vec::new(),
            },
        },
    );
    let port = crate::platform::semantic_id::PortId::migrate(SEED, 1120);
    let mut requirements = [component_available, component_extra]
        .into_iter()
        .map(|operand| match operand {
            kernel::RequirementOperand::Concrete(reference) => reference.requirement,
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    requirements.sort();
    declaration(
        &mut source,
        component,
        0,
        OwnerKind::Component,
        DeclarationPayload::Component {
            requirements,
            ports: vec![port],
        },
    );
    source.owners.insert(
        OwnerKey::Port(port),
        OwnerRecord::Port(kernel::PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component.declaration,
            name: name("entry"),
            function_type: signature,
            implementation: kernel::PortImplementation::Expression(port_body),
        }),
    );
    call_assert_local(&source);
    admit(&source).unwrap();
    let mut pure = source.clone();
    let pure_signature = intern(
        &mut pure,
        TypeForm::Function {
            parameters: Vec::new(),
            result: unit,
        },
    );
    let OwnerRecord::Port(record) = pure.owners.get_mut(&OwnerKey::Port(port)).unwrap() else {
        unreachable!()
    };
    record.function_type = pure_signature;
    let error = admit(&pure).unwrap_err();
    assert!(
        error.message.contains("pure") && error.message.contains("task"),
        "{}",
        error.message
    );
    let mut widened = source.clone();
    let kernel::RequirementOperand::Concrete(reference) = callee_required else {
        unreachable!()
    };
    let OwnerRecord::Requirement(record) = widened
        .owners
        .get_mut(&OwnerKey::Requirement(reference.requirement))
        .unwrap()
    else {
        unreachable!()
    };
    record.name = name("extra");
    let error = admit(&widened).unwrap_err();
    assert!(
        error.message.contains("row")
            || error.message.contains("allowance")
            || error.message.contains("requirement"),
        "{}",
        error.message
    );
    assert!(admit(&source).is_ok());
}

#[test]
fn raw_method_calls_preserve_empty_task_kind_and_exact_argument_type() {
    let mut source = empty();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let scalar = intern(&mut source, TypeForm::I64);
    let contract = decl(&source, 1140);
    let (self_parameter, self_ty) =
        type_parameter(&mut source, contract, 1140, TypeParameterConstraints::Owned);
    let method = crate::platform::semantic_id::MethodId::migrate(SEED, 1140);
    let task = FunctionEffect::Task {
        requirements: Vec::new(),
        effect_parameters: Vec::new(),
    };
    declaration(
        &mut source,
        contract,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(kernel::OwnedContract {
            self_parameter,
            type_parameters: Vec::new(),
            associated_types: Vec::new(),
            methods: vec![kernel::OwnedMethod {
                id: method,
                name: name("inspect"),
                parameters: vec![kernel::OwnedMethodParameter {
                    ty: self_ty,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: scalar,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: task.clone(),
            }],
        }),
    );
    let target_body = expression(&mut source, 1140, ExpressionOperation::I64 { value: 7 });
    let target = call_function(
        &mut source,
        1141,
        &[(cell, ParameterUse::Borrow)],
        scalar,
        task.clone(),
        target_body,
    );
    let implementation = decl(&source, 1142);
    declaration(
        &mut source,
        implementation,
        0,
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(kernel::OwnedImplementation {
            type_parameters: Vec::new(),
            implementation_parameters: Vec::new(),
            contract,
            self_type: cell,
            type_arguments: Vec::new(),
            associated_types: Vec::new(),
            methods: vec![kernel::OwnedMethodImplementation {
                method,
                function: target,
                type_arguments: Vec::new(),
                implementations: Vec::new(),
                read_scope_arguments: Vec::new(),
            }],
        }),
    );
    let argument = call_local(&mut source, 1141, call_parameter(1143, 0));
    let body = expression(
        &mut source,
        1142,
        ExpressionOperation::MethodCall {
            witness: ImplementationOperand::Concrete {
                implementation,
                type_arguments: Vec::new(),
                implementations: Vec::new(),
            },
            contract,
            method,
            read_scope_arguments: Vec::new(),
            arguments: vec![argument],
        },
    );
    let caller = call_function(
        &mut source,
        1143,
        &[(cell, ParameterUse::Borrow)],
        scalar,
        task,
        body,
    );
    call_assert_local(&source);
    admit(&source).unwrap();
    let mut pure = source.clone();
    let OwnerRecord::Declaration(record) = pure
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    record.header.kind = OwnerKind::PureFunction;
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Pure;
    let error = admit(&pure).unwrap_err();
    assert!(
        error.message.contains("pure") && error.message.contains("task"),
        "{}",
        error.message
    );
    let mut wrong_argument = source.clone();
    *call_operation_mut(&mut wrong_argument, argument) = ExpressionOperation::I64 { value: 9 };
    let error = admit(&wrong_argument).unwrap_err();
    assert!(
        error
            .message
            .starts_with("call argument differs from complete substituted parameter type"),
        "{}",
        error.message
    );
    assert!(admit(&source).is_ok());
}

#[test]
fn raw_capability_calls_require_exact_task_allowance_operation_and_parameter_type() {
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let boolean = intern(&mut source, TypeForm::Bool);
    let interface = decl(&source, 1150);
    let other_interface = decl(&source, 1151);
    let operation_id = crate::platform::semantic_id::OperationId::migrate(SEED, 1150);
    let other_operation_id = crate::platform::semantic_id::OperationId::migrate(SEED, 1151);
    let operation = kernel::OperationReference {
        package: source.root.package_id,
        operation: operation_id,
    };
    let other_operation = kernel::OperationReference {
        package: source.root.package_id,
        operation: other_operation_id,
    };
    let parameter = call_parameter(1150, 0);
    for (id, owner) in [
        (operation_id, interface),
        (other_operation_id, other_interface),
    ] {
        let parameters = if id == operation_id {
            vec![parameter]
        } else {
            Vec::new()
        };
        source.owners.insert(
            OwnerKey::Operation(id),
            OwnerRecord::Operation(kernel::OperationRecord {
                header: OwnerHeader::new(OwnerKey::Operation(id), OwnerKind::Operation),
                declaration: owner.declaration,
                name: name("read"),
                parameters,
                result: boolean,
                idempotency: kernel::Idempotency::Idempotent,
                external_visibility: kernel::ExternalVisibility::None,
            }),
        );
        declaration(
            &mut source,
            owner,
            0,
            OwnerKind::Interface,
            DeclarationPayload::Interface {
                operations: vec![id],
            },
        );
    }
    source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(kernel::ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Operation(operation_id),
            name: name("index"),
            ty: scalar,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    let caller = decl(&source, 1152);
    let requirement = call_requirement(&mut source, caller, interface, 1150);
    let kernel::RequirementOperand::Parameter(requirement_reference) = requirement else {
        unreachable!()
    };
    let OwnerRecord::RequirementParameter(record) = source
        .owners
        .get_mut(&OwnerKey::RequirementParameter(
            requirement_reference.parameter,
        ))
        .unwrap()
    else {
        unreachable!()
    };
    record.constraint.operations = vec![operation];
    let argument = expression(&mut source, 1150, ExpressionOperation::I64 { value: 1 });
    let body = expression(
        &mut source,
        1151,
        ExpressionOperation::CapabilityCall {
            requirement,
            operation,
            arguments: vec![argument],
        },
    );
    call_function(
        &mut source,
        1152,
        &[],
        boolean,
        FunctionEffect::Task {
            requirements: vec![requirement],
            effect_parameters: Vec::new(),
        },
        body,
    );
    call_function_mut(&mut source, caller).requirement_parameters =
        vec![requirement_reference.parameter];
    call_assert_local(&source);
    admit(&source).unwrap();
    for mutant in 0..5 {
        let mut changed = source.clone();
        match mutant {
            0 => {
                let OwnerRecord::Declaration(record) = changed
                    .owners
                    .get_mut(&OwnerKey::Declaration(caller.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.header.kind = OwnerKind::PureFunction;
                let DeclarationPayload::Function(function) = &mut record.payload else {
                    unreachable!()
                };
                function.effect = FunctionEffect::Pure;
            }
            1 => {
                call_function_mut(&mut changed, caller).effect = FunctionEffect::Task {
                    requirements: Vec::new(),
                    effect_parameters: Vec::new(),
                }
            }
            2 => {
                let ExpressionOperation::CapabilityCall { operation, .. } =
                    call_operation_mut(&mut changed, body)
                else {
                    unreachable!()
                };
                *operation = other_operation;
            }
            3 => {
                *call_operation_mut(&mut changed, argument) =
                    ExpressionOperation::Bool { value: false }
            }
            _ => {
                let ExpressionOperation::CapabilityCall { arguments, .. } =
                    call_operation_mut(&mut changed, body)
                else {
                    unreachable!()
                };
                arguments.clear();
                let sequence = expression(
                    &mut changed,
                    1152,
                    ExpressionOperation::Sequence {
                        items: vec![argument, body],
                    },
                );
                call_function_mut(&mut changed, caller).body = sequence;
            }
        }
        let error = admit(&changed).unwrap_err();
        let expected = match mutant {
            0 | 1 => "capability/transaction requirement is absent from exact task allowance",
            2 => "capability operation is outside exact requirement operations",
            3 => "capability argument differs from exact operation parameter type",
            _ => "capability call changes exact interface or parameter arity",
        };
        assert!(
            error.message.starts_with(expected),
            "capability mutant {mutant}: {}",
            error.message
        );
    }
    assert!(admit(&source).is_ok());
}

#[test]
fn raw_deep_expressions_keep_shallow_nominal_and_witness_depth_independent() {
    // Synthetic source syntax is deeper than the type/operand bound, while
    // every actual type and the supplied concrete witness remain shallow.
    let layers = 300;
    assert!(layers > kernel::contract::MAXIMUM_TYPE_DEPTH);
    assert!(layers + 8 < kernel::contract::MAXIMUM_EXPRESSION_DEPTH);
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let unit = intern(&mut source, TypeForm::Unit);
    let record = decl(&source, 1170);
    let (record_parameter, record_ty) =
        type_parameter(&mut source, record, 1170, TypeParameterConstraints::None);
    record_fields(
        &mut source,
        record,
        1170,
        false,
        vec![record_parameter],
        &[record_ty],
    );
    let instance = intern(
        &mut source,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![scalar],
        },
    );

    let contract = decl(&source, 1171);
    let (self_parameter, self_ty) =
        type_parameter(&mut source, contract, 1171, TypeParameterConstraints::Owned);
    let (item_parameter, _) =
        type_parameter(&mut source, contract, 1172, TypeParameterConstraints::None);
    for (id, label) in [(self_parameter, "Self"), (item_parameter, "Payload")] {
        let OwnerRecord::TypeParameter(record) =
            source.owners.get_mut(&OwnerKey::TypeParameter(id)).unwrap()
        else {
            unreachable!()
        };
        record.name = name(label);
    }
    let member = AssociatedTypeMemberId::migrate(SEED, 1171);
    declaration(
        &mut source,
        contract,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(kernel::OwnedContract {
            self_parameter,
            type_parameters: vec![item_parameter],
            associated_types: vec![kernel::AssociatedTypeMember {
                id: member,
                name: name("Block"),
                bound: AssociatedTypeBound::Owned,
            }],
            methods: vec![kernel::OwnedMethod {
                id: crate::platform::semantic_id::MethodId::migrate(SEED, 1100),
                name: name("inspect"),
                parameters: vec![kernel::OwnedMethodParameter {
                    ty: self_ty,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: unit,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    let implementation =
        opaque_witness_implementation(&mut source, 1172, contract, member, cell, false);
    let callee_body = call_local(&mut source, 1170, call_parameter(1173, 0));
    let callee = call_function(
        &mut source,
        1173,
        &[(scalar, ParameterUse::Unrestricted)],
        scalar,
        FunctionEffect::Pure,
        callee_body,
    );
    call_function_mut(&mut source, callee).implementation_parameters =
        vec![kernel::ImplementationParameter {
            id: crate::platform::semantic_id::ImplementationParameterId::migrate(SEED, 1173),
            name: name("storage"),
            contract,
            self_type: cell,
            type_arguments: vec![scalar],
        }];
    let argument = expression(&mut source, 1171, ExpressionOperation::I64 { value: 3 });
    let call = expression(
        &mut source,
        1172,
        ExpressionOperation::ImplementationCall {
            function: callee,
            type_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            implementations: vec![ImplementationOperand::Concrete {
                implementation,
                type_arguments: vec![cell, scalar],
                implementations: Vec::new(),
            }],
            arguments: vec![argument],
        },
    );
    let value = expression(&mut source, 1173, ExpressionOperation::I64 { value: 4 });
    let constructed = expression(
        &mut source,
        1174,
        ExpressionOperation::Record {
            nominal_type: Some(record),
            type_arguments: vec![scalar],
            fields: vec![kernel::RecordExpressionField {
                selector: kernel::FieldSelector::Nominal(kernel::FieldReference {
                    package: record.package,
                    field: FieldId::migrate(SEED, 11700),
                }),
                value,
            }],
        },
    );
    let mut body = expression(
        &mut source,
        1175,
        ExpressionOperation::Sequence {
            items: vec![call, constructed],
        },
    );
    for layer in 0..layers {
        body = expression(
            &mut source,
            12000 + layer as u64,
            ExpressionOperation::Sequence { items: vec![body] },
        );
    }
    call_function(&mut source, 1174, &[], instance, FunctionEffect::Pure, body);
    call_assert_local(&source);
    let proof = admit(&source).unwrap();
    assert_eq!(
        proof.prepared_category(instance).unwrap(),
        ValueCategory::OrdinaryData
    );
    assert_eq!(proof.types()[&instance], source.types[&instance]);
}

// Additional raw-source controls. Dependency/interface projections are fixtures,
// not authenticated RevisionCore proofs or execution/capability grants.
struct TransitiveMetadata {
    a: KernelSnapshot,
    b: KernelSnapshot,
    c: KernelSnapshot,
    atom: kernel::RequirementOperand,
    formal: kernel::RequirementOperand,
    interface: DeclarationReference,
    operation: kernel::OperationReference,
    other_operation: kernel::OperationReference,
    run: DeclarationReference,
    caller: DeclarationReference,
    call: ExpressionId,
    port: Option<crate::platform::semantic_id::PortId>,
    port_call: Option<ExpressionId>,
    port_extra: Option<kernel::RequirementOperand>,
    extra_operation: kernel::OperationReference,
}

fn transitive_metadata_empty(ordinal: u64) -> KernelSnapshot {
    let mut source = empty();
    source.root.package_id = PackageId::migrate(SEED, ordinal);
    source.root.package_name = name(&format!("metadata_package_{ordinal}"));
    source
}

fn transitive_metadata_public(source: &mut KernelSnapshot) {
    for record in source.owners.values_mut() {
        if let OwnerRecord::Declaration(record) = record {
            record.visibility = kernel::DeclarationVisibility::Public;
        }
    }
}

fn transitive_metadata_projection(
    supplier: &KernelSnapshot,
) -> BTreeMap<OwnerKey, kernel::PackageInterfaceRecord> {
    // Same canonical public-owner projection pattern as the maintained
    // reference_dependency_binding_tests::consumer_of fixture. No transitive
    // dependency/interface inventory is flattened into the consumer.
    let mut interface = BTreeMap::new();
    for (key, owner) in &supplier.owners {
        if matches!(owner, OwnerRecord::Declaration(record)
            if record.visibility != kernel::DeclarationVisibility::Public)
        {
            continue;
        }
        if let Some(record) = kernel::PackageInterfaceRecord::project_public(owner).unwrap() {
            interface.insert(*key, record);
        }
    }
    interface
}

fn transitive_metadata_import(
    consumer: &mut KernelSnapshot,
    supplier: &KernelSnapshot,
    marker: u8,
) {
    let selection = kernel::DependencyRecord {
        graph_contract_version: kernel::contract::GRAPH_CONTRACT_VERSION,
        package: supplier.root.package_id,
        semantic_revision: crate::platform::semantic_id::RevisionId::from_digest([marker; 32]),
        package_revision: kernel::PackageRevisionDigest::from_bytes([marker; 32]),
    };
    selection.validate_local().unwrap();
    consumer.dependency_interfaces.insert(
        selection.package_revision,
        std::sync::Arc::new(transitive_metadata_projection(supplier)),
    );
    for (digest, object) in &supplier.types {
        if let Some(previous) = consumer.dependency_types.insert(*digest, object.clone()) {
            assert_eq!(previous, *object);
        }
    }
    consumer.dependencies.insert(selection.package, selection);
}

fn transitive_metadata_assert_import(consumer: &KernelSnapshot, supplier: &KernelSnapshot) {
    let selection = &consumer.dependencies[&supplier.root.package_id];
    selection.validate_local().unwrap();
    assert_eq!(selection.package, supplier.root.package_id);
    assert_eq!(
        consumer.dependency_interfaces[&selection.package_revision].as_ref(),
        &transitive_metadata_projection(supplier)
    );
    for (digest, object) in &supplier.types {
        assert_eq!(consumer.dependency_types.get(digest), Some(object));
    }
}

fn transitive_metadata_requirement_mut(
    source: &mut KernelSnapshot,
    atom: kernel::RequirementOperand,
) -> &mut kernel::RequirementRecord {
    let kernel::RequirementOperand::Concrete(reference) = atom else {
        unreachable!()
    };
    let OwnerRecord::Requirement(record) = source
        .owners
        .get_mut(&OwnerKey::Requirement(reference.requirement))
        .unwrap()
    else {
        unreachable!()
    };
    record
}

fn transitive_metadata_formal_mut(
    source: &mut KernelSnapshot,
    formal: kernel::RequirementOperand,
) -> &mut kernel::RequirementParameterRecord {
    let kernel::RequirementOperand::Parameter(reference) = formal else {
        unreachable!()
    };
    let OwnerRecord::RequirementParameter(record) = source
        .owners
        .get_mut(&OwnerKey::RequirementParameter(reference.parameter))
        .unwrap()
    else {
        unreachable!()
    };
    record
}

fn transitive_metadata_function_port(
    source: &mut KernelSnapshot,
    component: DeclarationReference,
    ordinal: u64,
    function: DeclarationReference,
) {
    let (result, requirements) = {
        let function = call_function_mut(source, function);
        assert!(function.parameters.is_empty());
        assert!(function.requirement_parameters.is_empty());
        assert!(function.effect_parameters.is_empty());
        assert_eq!(function.result.relation, ResultRelation::Value);
        let FunctionEffect::Task {
            requirements,
            effect_parameters,
        } = &function.effect
        else {
            unreachable!()
        };
        assert!(effect_parameters.is_empty());
        (function.result.ty, requirements.clone())
    };
    let signature = intern(
        source,
        TypeForm::TaskFunction {
            parameters: Vec::new(),
            result,
            effect: kernel::EffectRow {
                requirements,
                parameters: Vec::new(),
            },
        },
    );
    let port = crate::platform::semantic_id::PortId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Port(port),
        OwnerRecord::Port(kernel::PortRecord {
            header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
            declaration: component.declaration,
            name: name("entry"),
            function_type: signature,
            implementation: kernel::PortImplementation::Function(function),
        }),
    );
}

fn transitive_metadata_assert_local_graph(source: &KernelSnapshot) {
    call_assert_local(source);
    for (key, record) in &source.owners {
        assert_eq!(record.header().owner, *key);
        match record {
            OwnerRecord::Requirement(requirement) => {
                let OwnerRecord::Declaration(owner) =
                    &source.owners[&OwnerKey::Declaration(requirement.declaration)]
                else {
                    unreachable!()
                };
                let DeclarationPayload::Component { requirements, .. } = &owner.payload else {
                    unreachable!()
                };
                let OwnerKey::Requirement(id) = key else {
                    unreachable!()
                };
                assert!(requirements.contains(id));
            }
            OwnerRecord::RequirementParameter(parameter) => {
                let OwnerRecord::Declaration(owner) =
                    &source.owners[&OwnerKey::Declaration(parameter.declaration)]
                else {
                    unreachable!()
                };
                let DeclarationPayload::Function(function) = &owner.payload else {
                    unreachable!()
                };
                let OwnerKey::RequirementParameter(id) = key else {
                    unreachable!()
                };
                assert!(function.requirement_parameters.contains(id));
            }
            OwnerRecord::Operation(operation) => {
                let OwnerRecord::Declaration(owner) =
                    &source.owners[&OwnerKey::Declaration(operation.declaration)]
                else {
                    unreachable!()
                };
                let DeclarationPayload::Interface { operations } = &owner.payload else {
                    unreachable!()
                };
                let OwnerKey::Operation(id) = key else {
                    unreachable!()
                };
                assert!(operations.contains(id));
            }
            OwnerRecord::Port(port) => {
                let OwnerRecord::Declaration(owner) =
                    &source.owners[&OwnerKey::Declaration(port.declaration)]
                else {
                    unreachable!()
                };
                let DeclarationPayload::Component { ports, .. } = &owner.payload else {
                    unreachable!()
                };
                let OwnerKey::Port(id) = key else {
                    unreachable!()
                };
                assert!(ports.contains(id));
                if let kernel::PortImplementation::Function(reference) = port.implementation {
                    assert_eq!(reference.package, source.root.package_id);
                    let OwnerRecord::Declaration(owner) =
                        &source.owners[&OwnerKey::Declaration(reference.declaration)]
                    else {
                        unreachable!()
                    };
                    let DeclarationPayload::Function(function) = &owner.payload else {
                        unreachable!()
                    };
                    let TypeForm::TaskFunction {
                        parameters,
                        result,
                        effect,
                    } = &source.types[&port.function_type].form
                    else {
                        unreachable!()
                    };
                    assert!(parameters.is_empty() && function.parameters.is_empty());
                    assert_eq!(*result, function.result.ty);
                    assert_eq!(function.result.relation, ResultRelation::Value);
                    let FunctionEffect::Task {
                        requirements,
                        effect_parameters,
                    } = &function.effect
                    else {
                        unreachable!()
                    };
                    assert_eq!(&effect.requirements, requirements);
                    assert!(effect.parameters.is_empty() && effect_parameters.is_empty());
                }
            }
            OwnerRecord::Declaration(owner) => {
                if let DeclarationPayload::Component {
                    requirements,
                    ports,
                } = &owner.payload
                {
                    assert!(!ports.is_empty());
                    for id in requirements {
                        let OwnerRecord::Requirement(record) =
                            &source.owners[&OwnerKey::Requirement(*id)]
                        else {
                            unreachable!()
                        };
                        assert_eq!(
                            record.declaration,
                            match key {
                                OwnerKey::Declaration(id) => *id,
                                _ => unreachable!(),
                            }
                        );
                    }
                    for id in ports {
                        let OwnerRecord::Port(record) = &source.owners[&OwnerKey::Port(*id)] else {
                            unreachable!()
                        };
                        assert_eq!(
                            record.declaration,
                            match key {
                                OwnerKey::Declaration(id) => *id,
                                _ => unreachable!(),
                            }
                        );
                    }
                }
                if let DeclarationPayload::Function(function) = &owner.payload {
                    assert!(matches!(
                        &source.owners[&OwnerKey::Expression(function.body)],
                        OwnerRecord::Expression(_)
                    ));
                }
            }
            _ => {}
        }
    }
}

fn transitive_metadata_fixture(with_port: bool) -> TransitiveMetadata {
    let mut b = transitive_metadata_empty(3000);
    let interface = decl(&b, 3000);
    call_interface(&mut b, interface, 3000);
    let other_interface = decl(&b, 3001);
    call_interface(&mut b, other_interface, 3001);
    let operation = kernel::OperationReference {
        package: b.root.package_id,
        operation: crate::platform::semantic_id::OperationId::migrate(SEED, 3000),
    };
    let other_operation = kernel::OperationReference {
        package: b.root.package_id,
        operation: crate::platform::semantic_id::OperationId::migrate(SEED, 3001),
    };
    transitive_metadata_public(&mut b);

    let mut a = transitive_metadata_empty(3001);
    let unit = intern(&mut a, TypeForm::Unit);
    let component = decl(&a, 3002);
    let atom = call_concrete_requirement(&mut a, component, interface, 3010, "data", 100);
    transitive_metadata_requirement_mut(&mut a, atom).operations = vec![operation];
    let kernel::RequirementOperand::Concrete(atom_reference) = atom else {
        unreachable!()
    };
    declaration(
        &mut a,
        component,
        0,
        OwnerKind::Component,
        DeclarationPayload::Component {
            requirements: vec![atom_reference.requirement],
            ports: vec![crate::platform::semantic_id::PortId::migrate(SEED, 3002)],
        },
    );
    let run = decl(&a, 3003);
    let formal = call_requirement(&mut a, run, interface, 3003);
    transitive_metadata_formal_mut(&mut a, formal)
        .constraint
        .operations = vec![operation];
    let run_body = expression(&mut a, 3003, ExpressionOperation::Unit {});
    call_function(
        &mut a,
        3003,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![formal],
            effect_parameters: Vec::new(),
        },
        run_body,
    );
    let kernel::RequirementOperand::Parameter(formal_reference) = formal else {
        unreachable!()
    };
    call_function_mut(&mut a, run).requirement_parameters = vec![formal_reference.parameter];
    let descriptor_body = expression(&mut a, 3004, ExpressionOperation::Unit {});
    let descriptor = call_function(
        &mut a,
        3004,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![atom],
            effect_parameters: Vec::new(),
        },
        descriptor_body,
    );
    transitive_metadata_function_port(&mut a, component, 3002, descriptor);
    let extra_interface = decl(&a, 3007);
    call_interface(&mut a, extra_interface, 3007);
    let extra_operation = kernel::OperationReference {
        package: a.root.package_id,
        operation: crate::platform::semantic_id::OperationId::migrate(SEED, 3007),
    };
    transitive_metadata_public(&mut a);
    transitive_metadata_import(&mut a, &b, 11);

    let mut c = transitive_metadata_empty(3002);
    let unit = intern(&mut c, TypeForm::Unit);
    let call = call_named(&mut c, 3005, run, Vec::new());
    let ExpressionOperation::Call {
        requirement_arguments,
        ..
    } = call_operation_mut(&mut c, call)
    else {
        unreachable!()
    };
    *requirement_arguments = vec![atom];
    let caller = call_function(
        &mut c,
        3005,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![atom],
            effect_parameters: Vec::new(),
        },
        call,
    );
    let (port, port_call, port_extra) = if with_port {
        let component = decl(&c, 3006);
        let extra =
            call_concrete_requirement(&mut c, component, extra_interface, 3012, "extra", 100);
        transitive_metadata_requirement_mut(&mut c, extra).operations = vec![extra_operation];
        let inner_call = call_named(&mut c, 3006, run, Vec::new());
        let ExpressionOperation::Call {
            requirement_arguments,
            ..
        } = call_operation_mut(&mut c, inner_call)
        else {
            unreachable!()
        };
        *requirement_arguments = vec![atom];
        let descriptor_value = expression(
            &mut c,
            3007,
            ExpressionOperation::FunctionValue {
                function: descriptor,
                type_arguments: Vec::new(),
                read_scope_arguments: Vec::new(),
                effect_arguments: Vec::new(),
                requirement_arguments: Vec::new(),
            },
        );
        let body = expression(
            &mut c,
            3008,
            ExpressionOperation::Sequence {
                items: vec![inner_call, descriptor_value],
            },
        );
        let signature = intern(
            &mut c,
            TypeForm::TaskFunction {
                parameters: Vec::new(),
                result: unit,
                effect: kernel::EffectRow {
                    requirements: vec![atom],
                    parameters: Vec::new(),
                },
            },
        );
        let port = crate::platform::semantic_id::PortId::migrate(SEED, 3006);
        let kernel::RequirementOperand::Concrete(extra_reference) = extra else {
            unreachable!()
        };
        declaration(
            &mut c,
            component,
            0,
            OwnerKind::Component,
            DeclarationPayload::Component {
                requirements: vec![extra_reference.requirement],
                ports: vec![port],
            },
        );
        c.owners.insert(
            OwnerKey::Port(port),
            OwnerRecord::Port(kernel::PortRecord {
                header: OwnerHeader::new(OwnerKey::Port(port), OwnerKind::Port),
                declaration: component.declaration,
                name: name("entry"),
                function_type: signature,
                implementation: kernel::PortImplementation::Expression(body),
            }),
        );
        (Some(port), Some(inner_call), Some(extra))
    } else {
        (None, None, None)
    };
    transitive_metadata_public(&mut c);
    transitive_metadata_import(&mut c, &a, 12);
    TransitiveMetadata {
        a,
        b,
        c,
        atom,
        formal,
        interface,
        operation,
        other_operation,
        run,
        caller,
        call,
        port,
        port_call,
        port_extra,
        extra_operation,
    }
}

fn transitive_metadata_assert_healthy(fixture: &TransitiveMetadata) {
    for source in [&fixture.a, &fixture.b, &fixture.c] {
        transitive_metadata_assert_local_graph(source);
        for object in source.dependency_types.values() {
            object.validate_local().unwrap();
        }
    }
    transitive_metadata_assert_import(&fixture.a, &fixture.b);
    transitive_metadata_assert_import(&fixture.c, &fixture.a);
    assert_eq!(fixture.c.dependencies.len(), 1);
    assert!(
        fixture
            .c
            .dependencies
            .contains_key(&fixture.a.root.package_id)
    );
    assert!(
        !fixture
            .c
            .dependencies
            .contains_key(&fixture.b.root.package_id)
    );
    let control = ExecutionControl::uncancelled();
    let proof = Proof::reconstruct(&[&fixture.c, &fixture.a, &fixture.b], &control).unwrap();
    assert!(!proof.dependencies[&fixture.c.root.package_id].contains(&fixture.b.root.package_id));
    Proof::reconstruct(&[&fixture.b, &fixture.a, &fixture.c], &control).unwrap();
}

fn transitive_metadata_refuses(snapshots: &[&KernelSnapshot], expected: &str) {
    for source in snapshots {
        transitive_metadata_assert_local_graph(source);
    }
    let error = Proof::reconstruct(snapshots, &ExecutionControl::uncancelled()).unwrap_err();
    assert_eq!(error.class, ExecutionFailureClass::Infrastructure);
    assert_eq!(error.code, "reference_read_view_admission");
    assert_eq!(error.message, expected);
}

#[test]
fn raw_imported_requirement_metadata_preserves_actual_owner_import_edges() {
    let fixture = transitive_metadata_fixture(false);
    transitive_metadata_assert_healthy(&fixture);

    let mut no_atom_edge = fixture.c.clone();
    no_atom_edge.dependencies.remove(&fixture.a.root.package_id);
    transitive_metadata_refuses(
        &[&no_atom_edge, &fixture.a, &fixture.b],
        "requirement atom lies outside caller dependency inventory",
    );

    let mut no_metadata_edge = fixture.a.clone();
    no_metadata_edge
        .dependencies
        .remove(&fixture.b.root.package_id);
    let mut caller_has_b = fixture.c.clone();
    transitive_metadata_import(&mut caller_has_b, &fixture.b, 13);
    transitive_metadata_import(&mut caller_has_b, &no_metadata_edge, 12);
    transitive_metadata_refuses(
        &[&caller_has_b, &no_metadata_edge, &fixture.b],
        "reference is outside its exact package dependency inventory",
    );

    let mut other_member = fixture.a.clone();
    transitive_metadata_formal_mut(&mut other_member, fixture.formal)
        .constraint
        .operations = vec![fixture.other_operation];
    let mut caller = fixture.c.clone();
    transitive_metadata_import(&mut caller, &other_member, 12);
    transitive_metadata_refuses(
        &[&caller, &other_member, &fixture.b],
        "requirement constraint operation imports another interface",
    );

    // D retains the same declaration/operation IDs, names, Unit signature and
    // limit shape as B. Package-qualified identity alone distinguishes them.
    let mut d = fixture.b.clone();
    d.root.package_id = PackageId::migrate(SEED, 3009);
    d.root.package_name = name("metadata_foreign_mirror");
    let foreign_interface = DeclarationReference {
        package: d.root.package_id,
        declaration: fixture.interface.declaration,
    };
    let foreign_operation = kernel::OperationReference {
        package: d.root.package_id,
        operation: fixture.operation.operation,
    };
    assert_ne!(foreign_interface, fixture.interface);
    assert_ne!(foreign_operation, fixture.operation);
    assert_eq!(d.owners, fixture.b.owners);
    let mut a = fixture.a.clone();
    let foreign_component = decl(&a, 3009);
    let foreign_atom = call_concrete_requirement(
        &mut a,
        foreign_component,
        foreign_interface,
        3013,
        "data",
        100,
    );
    transitive_metadata_requirement_mut(&mut a, foreign_atom).operations = vec![foreign_operation];
    let kernel::RequirementOperand::Concrete(reference) = foreign_atom else {
        unreachable!()
    };
    declaration(
        &mut a,
        foreign_component,
        0,
        OwnerKind::Component,
        DeclarationPayload::Component {
            requirements: vec![reference.requirement],
            ports: vec![crate::platform::semantic_id::PortId::migrate(SEED, 3009)],
        },
    );
    let unit = intern(&mut a, TypeForm::Unit);
    let foreign_body = expression(&mut a, 3014, ExpressionOperation::Unit {});
    let foreign_descriptor = call_function(
        &mut a,
        3014,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![foreign_atom],
            effect_parameters: Vec::new(),
        },
        foreign_body,
    );
    transitive_metadata_function_port(&mut a, foreign_component, 3009, foreign_descriptor);
    transitive_metadata_public(&mut a);
    transitive_metadata_import(&mut a, &d, 14);
    let mut c = fixture.c.clone();
    let ExpressionOperation::Call {
        requirement_arguments,
        ..
    } = call_operation_mut(&mut c, fixture.call)
    else {
        unreachable!()
    };
    *requirement_arguments = vec![foreign_atom];
    call_function_mut(&mut c, fixture.caller).effect = FunctionEffect::Task {
        requirements: vec![foreign_atom],
        effect_parameters: Vec::new(),
    };
    transitive_metadata_import(&mut c, &a, 12);

    // Prove the foreign metadata and its actual are individually admissible by
    // requiring that same exact D interface, then change only the required
    // formal constraint back to B. This reaches exact entailment, not an absent
    // edge/owner or malformed same-shaped record.
    let mut same_foreign = a.clone();
    transitive_metadata_formal_mut(&mut same_foreign, fixture.formal).constraint =
        kernel::RequirementConstraint {
            interface: foreign_interface,
            operations: vec![foreign_operation],
        };
    let mut same_c = c.clone();
    transitive_metadata_import(&mut same_c, &same_foreign, 12);
    for source in [&same_foreign, &fixture.b, &same_c, &d] {
        transitive_metadata_assert_local_graph(source);
    }
    transitive_metadata_assert_import(&same_foreign, &d);
    transitive_metadata_assert_import(&same_c, &same_foreign);
    assert!(!same_c.dependencies.contains_key(&d.root.package_id));
    Proof::reconstruct(
        &[&same_c, &same_foreign, &fixture.b, &d],
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    transitive_metadata_refuses(
        &[&c, &a, &fixture.b, &d],
        "requirement actual omits an exact interface or required operation",
    );
    assert_eq!(
        transitive_metadata_formal_mut(&mut a, fixture.formal)
            .constraint
            .interface,
        fixture.interface
    );
    assert_ne!(foreign_atom, fixture.atom);
    assert_eq!(fixture.run.package, fixture.a.root.package_id);
    transitive_metadata_assert_healthy(&fixture);
}

#[test]
fn raw_imported_port_row_uses_explicit_requirement_without_component_grants() {
    let fixture = transitive_metadata_fixture(true);
    transitive_metadata_assert_healthy(&fixture);
    let port = fixture.port.unwrap();
    let OwnerRecord::Port(record) = &fixture.c.owners[&OwnerKey::Port(port)] else {
        unreachable!()
    };
    let signature = record.function_type;
    assert!(
        matches!(&fixture.c.types[&signature].form, TypeForm::TaskFunction { effect, .. }
        if effect.requirements == vec![fixture.atom] && effect.parameters.is_empty())
    );

    let mut pure = fixture.c.clone();
    let unit = intern(&mut pure, TypeForm::Unit);
    let pure_signature = intern(
        &mut pure,
        TypeForm::Function {
            parameters: Vec::new(),
            result: unit,
        },
    );
    let OwnerRecord::Port(record) = pure.owners.get_mut(&OwnerKey::Port(port)).unwrap() else {
        unreachable!()
    };
    record.function_type = pure_signature;
    transitive_metadata_refuses(
        &[&pure, &fixture.a, &fixture.b],
        "pure caller invokes a task, including a task with an empty row",
    );

    let mut omitted = fixture.c.clone();
    let empty_row = intern(
        &mut omitted,
        TypeForm::TaskFunction {
            parameters: Vec::new(),
            result: unit,
            effect: kernel::EffectRow::default(),
        },
    );
    let OwnerRecord::Port(record) = omitted.owners.get_mut(&OwnerKey::Port(port)).unwrap() else {
        unreachable!()
    };
    record.function_type = empty_row;
    transitive_metadata_refuses(
        &[&omitted, &fixture.a, &fixture.b],
        "task invocation exceeds exact caller requirement allowance",
    );

    // The component owns this extra exact atom, but the Port row explicitly
    // imports only A's atom. Its parent component is no extra activation grant.
    let mut component_extra = fixture.c.clone();
    *call_operation_mut(&mut component_extra, fixture.port_call.unwrap()) =
        ExpressionOperation::CapabilityCall {
            requirement: fixture.port_extra.unwrap(),
            operation: fixture.extra_operation,
            arguments: Vec::new(),
        };
    transitive_metadata_refuses(
        &[&component_extra, &fixture.a, &fixture.b],
        "capability/transaction requirement is absent from exact task allowance",
    );

    let mut no_metadata_edge = fixture.a.clone();
    no_metadata_edge
        .dependencies
        .remove(&fixture.b.root.package_id);
    let mut caller_has_b = fixture.c.clone();
    transitive_metadata_import(&mut caller_has_b, &fixture.b, 13);
    transitive_metadata_import(&mut caller_has_b, &no_metadata_edge, 12);
    transitive_metadata_refuses(
        &[&caller_has_b, &no_metadata_edge, &fixture.b],
        "reference is outside its exact package dependency inventory",
    );
    transitive_metadata_assert_healthy(&fixture);
}

struct CapabilitySignatureFixture {
    source: KernelSnapshot,
    function: DeclarationReference,
    ordinary: ParameterId,
    view: Option<ParameterId>,
    owned: ParameterId,
    capability: ParameterId,
    resource: TypeObjectDigest,
    primary: kernel::RequirementReference,
    spare: kernel::RequirementReference,
    foreign: kernel::RequirementReference,
}

fn capability_signature_fixture(mode: ParameterUse, with_view: bool) -> CapabilitySignatureFixture {
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let unit = intern(&mut source, TypeForm::Unit);
    let interface = decl(&source, 4000);
    let other_interface = decl(&source, 4001);
    call_interface(&mut source, interface, 4000);
    call_interface(&mut source, other_interface, 4001);
    let resource = intern(&mut source, TypeForm::CapabilityResource { interface });
    let component = decl(&source, 4002);
    let function = decl(&source, 4003);
    let primary =
        call_concrete_requirement(&mut source, component, interface, 4000, "primary", 100);
    let spare = call_concrete_requirement(&mut source, component, interface, 4001, "spare", 100);
    let foreign = call_concrete_requirement(
        &mut source,
        component,
        other_interface,
        4002,
        "foreign",
        100,
    );
    let concrete = |operand| match operand {
        kernel::RequirementOperand::Concrete(reference) => reference,
        _ => unreachable!(),
    };
    let primary_reference = concrete(primary);
    let operation_id = crate::platform::semantic_id::OperationId::migrate(SEED, 4000);
    let operation = kernel::OperationReference {
        package: interface.package,
        operation: operation_id,
    };
    let operation_parameter = call_parameter(4000, 0);
    let OwnerRecord::Operation(record) = source
        .owners
        .get_mut(&OwnerKey::Operation(operation_id))
        .unwrap()
    else {
        unreachable!()
    };
    record.parameters = vec![operation_parameter];
    source.owners.insert(
        OwnerKey::Parameter(operation_parameter),
        OwnerRecord::Parameter(kernel::ParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::Parameter(operation_parameter),
                OwnerKind::Parameter,
            ),
            parent: ParameterParent::Operation(operation_id),
            name: name("lease"),
            ty: resource,
            use_mode: mode,
            resource_requirement: None,
        }),
    );
    let OwnerRecord::Requirement(record) = source
        .owners
        .get_mut(&OwnerKey::Requirement(primary_reference.requirement))
        .unwrap()
    else {
        unreachable!()
    };
    record.operations = vec![operation];
    let mut requirements = [primary, spare, foreign]
        .map(|operand| concrete(operand).requirement)
        .to_vec();
    requirements.sort();
    let port = crate::platform::semantic_id::PortId::migrate(SEED, 4002);
    declaration(
        &mut source,
        component,
        0,
        OwnerKind::Component,
        DeclarationPayload::Component {
            requirements,
            ports: vec![port],
        },
    );
    let descriptor_body = expression(&mut source, 4004, ExpressionOperation::Unit {});
    let descriptor = call_function(
        &mut source,
        4004,
        &[],
        unit,
        FunctionEffect::Task {
            requirements: vec![primary],
            effect_parameters: Vec::new(),
        },
        descriptor_body,
    );
    transitive_metadata_function_port(&mut source, component, 4002, descriptor);
    let read_scope = scope(ReadScopeBinder::Declaration(function), 4003);
    let mut parameters = vec![(scalar, ParameterUse::Unrestricted)];
    let view = if with_view {
        let (declaration, _) = add_view(&mut source, 4005, false, None);
        let ty = intern(
            &mut source,
            TypeForm::ReadViewApplied {
                declaration,
                arguments: Vec::new(),
                read_scope,
            },
        );
        parameters.push((ty, ParameterUse::Borrow));
        Some(call_parameter(4003, 1))
    } else {
        None
    };
    let owned = call_parameter(4003, parameters.len() as u64);
    parameters.push((cell, ParameterUse::Borrow));
    let capability = call_parameter(4003, parameters.len() as u64);
    parameters.push((resource, mode));
    let argument = call_local(&mut source, 4000, capability);
    let body = expression(
        &mut source,
        4001,
        ExpressionOperation::CapabilityCall {
            requirement: primary,
            operation,
            arguments: vec![argument],
        },
    );
    call_function(
        &mut source,
        4003,
        &parameters,
        unit,
        FunctionEffect::Task {
            requirements: vec![primary],
            effect_parameters: Vec::new(),
        },
        body,
    );
    let OwnerRecord::Parameter(record) = source
        .owners
        .get_mut(&OwnerKey::Parameter(capability))
        .unwrap()
    else {
        unreachable!()
    };
    record.resource_requirement = Some(primary_reference);
    if let Some(view) = view {
        let function = call_function_mut(&mut source, function);
        function.read_scope_parameters = vec![ReadScopeParameter {
            id: ReadScopeParameterId::migrate(SEED, 4003),
            name: name("s"),
        }];
        function.read_scope_sources = vec![ReadScopeSource {
            source: view,
            scope: read_scope,
        }];
    }
    CapabilitySignatureFixture {
        source,
        function,
        ordinary: call_parameter(4003, 0),
        view,
        owned,
        capability,
        resource,
        primary: primary_reference,
        spare: concrete(spare),
        foreign: concrete(foreign),
    }
}

#[test]
fn raw_named_task_capability_parameters_require_exact_ambient_authority() {
    // Synthetic raw admission checks metadata/category and exact authority.
    // The independent affine admission remains responsible for resource flow.
    for mode in [ParameterUse::Borrow, ParameterUse::Consume] {
        for with_view in [false, true] {
            let fixture = capability_signature_fixture(mode, with_view);
            transitive_metadata_assert_local_graph(&fixture.source);
            let proof = admit(&fixture.source).unwrap();
            assert_eq!(
                proof.prepared_category(fixture.resource).unwrap(),
                ValueCategory::Capability
            );
            for mutant in 0..7 {
                let mut changed = fixture.source.clone();
                match mutant {
                    0 => {
                        let OwnerRecord::Parameter(record) = changed
                            .owners
                            .get_mut(&OwnerKey::Parameter(fixture.capability))
                            .unwrap()
                        else {
                            unreachable!()
                        };
                        record.use_mode = ParameterUse::Unrestricted;
                    }
                    1 => {
                        let OwnerRecord::Declaration(record) = changed
                            .owners
                            .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
                            .unwrap()
                        else {
                            unreachable!()
                        };
                        record.header.kind = OwnerKind::PureFunction;
                        let DeclarationPayload::Function(function) = &mut record.payload else {
                            unreachable!()
                        };
                        function.effect = FunctionEffect::Pure;
                    }
                    2 => {
                        let OwnerRecord::Parameter(record) = changed
                            .owners
                            .get_mut(&OwnerKey::Parameter(fixture.capability))
                            .unwrap()
                        else {
                            unreachable!()
                        };
                        record.resource_requirement = None;
                    }
                    3 => {
                        call_function_mut(&mut changed, fixture.function).effect =
                            FunctionEffect::Task {
                                requirements: vec![kernel::RequirementOperand::Concrete(
                                    fixture.spare,
                                )],
                                effect_parameters: Vec::new(),
                            };
                    }
                    4 => {
                        let OwnerRecord::Parameter(record) = changed
                            .owners
                            .get_mut(&OwnerKey::Parameter(fixture.capability))
                            .unwrap()
                        else {
                            unreachable!()
                        };
                        record.resource_requirement = Some(fixture.foreign);
                        let mut requirements = vec![
                            kernel::RequirementOperand::Concrete(fixture.primary),
                            kernel::RequirementOperand::Concrete(fixture.foreign),
                        ];
                        requirements.sort();
                        call_function_mut(&mut changed, fixture.function).effect =
                            FunctionEffect::Task {
                                requirements,
                                effect_parameters: Vec::new(),
                            };
                    }
                    5 => {
                        let OwnerRecord::Parameter(record) = changed
                            .owners
                            .get_mut(&OwnerKey::Parameter(fixture.ordinary))
                            .unwrap()
                        else {
                            unreachable!()
                        };
                        record.resource_requirement = Some(fixture.primary);
                    }
                    _ => {
                        let interface = decl(&changed, 4000);
                        let formal =
                            call_requirement(&mut changed, fixture.function, interface, 4003);
                        let kernel::RequirementOperand::Parameter(reference) = formal else {
                            unreachable!()
                        };
                        call_function_mut(&mut changed, fixture.function).requirement_parameters =
                            vec![reference.parameter];
                    }
                }
                let error = admit(&changed).unwrap_err();
                let expected = match mutant {
                    0 => "capability parameter must borrow or consume",
                    1 => "capability parameter requires exact named task function",
                    2 => "capability parameter lacks exact concrete requirement binding",
                    3 => "capability parameter binding is absent from exact function effect",
                    4 => "capability parameter interface differs from exact requirement",
                    5 => "nonresource parameter binds a resource requirement",
                    _ => "resource-bearing function cannot declare requirement formals",
                };
                assert!(
                    error.message.starts_with(expected),
                    "mode={mode:?}, view={with_view}, mutant={mutant}: {}",
                    error.message
                );
            }
            assert!(admit(&fixture.source).is_ok());
        }
    }
}

#[test]
fn raw_capability_signature_suffix_and_result_boundaries_remain_exact() {
    let fixture = capability_signature_fixture(ParameterUse::Consume, true);
    transitive_metadata_assert_local_graph(&fixture.source);
    admit(&fixture.source).unwrap();
    let view = fixture.view.unwrap();
    for mutant in 0..5 {
        let mut changed = fixture.source.clone();
        call_function_mut(&mut changed, fixture.function).parameters = match mutant {
            0 => vec![fixture.ordinary, view, fixture.capability, fixture.owned],
            1 => vec![view, fixture.owned, fixture.capability, fixture.ordinary],
            2 => vec![fixture.ordinary, fixture.owned, fixture.capability, view],
            3 => vec![fixture.ordinary, fixture.owned, view, fixture.capability],
            _ => vec![fixture.capability, fixture.ordinary, view, fixture.owned],
        };
        let error = admit(&changed).unwrap_err();
        let expected = match mutant {
            0 => "Owned signature parameter follows the capability suffix",
            1 | 4 => "signature ordinary/affine parameter order or use changed",
            2 => "view signature parameter follows the capability suffix",
            _ => "view signature parameter follows the Owned suffix",
        };
        assert!(
            error.message.starts_with(expected),
            "suffix mutant={mutant}: {}",
            error.message
        );
    }
    for borrowed in [false, true] {
        let mut changed = fixture.source.clone();
        let cell = intern(&mut changed, TypeForm::OwnedI64Cell);
        let body = expression(&mut changed, 4006, ExpressionOperation::Unit {});
        let returned = call_function(
            &mut changed,
            4006,
            &[(cell, ParameterUse::Borrow)],
            fixture.resource,
            FunctionEffect::Pure,
            body,
        );
        if borrowed {
            call_function_mut(&mut changed, returned).result.relation = ResultRelation::ReadFrom {
                source: call_parameter(4006, 0),
            };
        }
        let error = admit(&changed).unwrap_err();
        assert!(
            error
                .message
                .starts_with("function result contains a capability resource"),
            "borrowed={borrowed}: {}",
            error.message
        );
    }
    for nominal in [false, true] {
        for unsafe_callable in [false, true] {
            let mut source = fixture.source.clone();
            let secret = intern(&mut source, TypeForm::Secret);
            let unit = intern(&mut source, TypeForm::Unit);
            let actual = intern(
                &mut source,
                TypeForm::Function {
                    parameters: vec![if unsafe_callable {
                        fixture.resource
                    } else {
                        secret
                    }],
                    result: unit,
                },
            );
            if nominal {
                let record = decl(&source, 4010);
                let (formal, _) =
                    type_parameter(&mut source, record, 4010, TypeParameterConstraints::None);
                record_fields(&mut source, record, 4010, false, vec![formal], &[unit]);
                intern(
                    &mut source,
                    TypeForm::Applied {
                        declaration: record,
                        arguments: vec![actual],
                    },
                );
            } else {
                let target = decl(&source, 4012);
                let (formal, _) =
                    type_parameter(&mut source, target, 4012, TypeParameterConstraints::None);
                let body = expression(&mut source, 4012, ExpressionOperation::Unit {});
                call_function(&mut source, 4012, &[], unit, FunctionEffect::Pure, body);
                call_function_mut(&mut source, target).type_parameters = vec![formal];
                let body = expression(
                    &mut source,
                    4013,
                    ExpressionOperation::Call {
                        function: target,
                        type_arguments: vec![actual],
                        read_scope_arguments: Vec::new(),
                        effect_arguments: Vec::new(),
                        requirement_arguments: Vec::new(),
                        arguments: Vec::new(),
                    },
                );
                call_function(&mut source, 4013, &[], unit, FunctionEffect::Pure, body);
            }
            if unsafe_callable {
                let error = admit(&source).unwrap_err();
                assert!(
                    error.message.starts_with(
                        "forbidden canonical container contains a capability resource"
                    ),
                    "nominal={nominal}: {}",
                    error.message
                );
            } else {
                transitive_metadata_assert_local_graph(&source);
                admit(&source).unwrap();
            }
        }
    }
    // T remains unused by the stored field. Secret is a legal ordinary actual;
    // the enclosing ordinary nominal capability container remains forbidden.
    let mut source = fixture.source.clone();
    let unit = intern(&mut source, TypeForm::Unit);
    let secret = intern(&mut source, TypeForm::Secret);
    let record = decl(&source, 4014);
    let (formal, _) = type_parameter(&mut source, record, 4014, TypeParameterConstraints::None);
    record_fields(&mut source, record, 4014, false, vec![formal], &[unit]);
    intern(
        &mut source,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![secret],
        },
    );
    transitive_metadata_assert_local_graph(&source);
    admit(&source).unwrap();
    intern(
        &mut source,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![fixture.resource],
        },
    );
    let error = admit(&source).unwrap_err();
    assert!(
        error
            .message
            .starts_with("forbidden canonical container contains a capability resource"),
        "{}",
        error.message
    );
    let mut owned = fixture.source.clone();
    let cell = intern(&mut owned, TypeForm::OwnedI64Cell);
    let record = decl(&owned, 4015);
    let (formal, _) = type_parameter(&mut owned, record, 4015, TypeParameterConstraints::Owned);
    record_fields(&mut owned, record, 4015, true, vec![formal], &[cell]);
    intern(
        &mut owned,
        TypeForm::Applied {
            declaration: record,
            arguments: vec![cell],
        },
    );
    transitive_metadata_assert_local_graph(&owned);
    admit(&owned).unwrap();
    // A bare named resource variant used only as an unused generic type actual
    // is a different canonical context from a forbidden container around it.
    let mut metadata = fixture.source.clone();
    let unit = intern(&mut metadata, TypeForm::Unit);
    let variant = decl(&metadata, 4020);
    let absent = crate::platform::semantic_id::CaseId::migrate(SEED, 4020);
    let present = crate::platform::semantic_id::CaseId::migrate(SEED, 4021);
    for (id, label, payload) in [
        (absent, "absent", None),
        (present, "present", Some(fixture.resource)),
    ] {
        metadata.owners.insert(
            OwnerKey::Case(id),
            OwnerRecord::Case(kernel::CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(id), OwnerKind::Case),
                declaration: variant.declaration,
                name: name(label),
                payload,
            }),
        );
    }
    let mut cases = vec![absent, present];
    cases.sort();
    declaration(
        &mut metadata,
        variant,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: Vec::new(),
            cases,
        },
    );
    let actual = intern(
        &mut metadata,
        TypeForm::Named {
            declaration: variant,
        },
    );
    let target = decl(&metadata, 4021);
    let (formal, _) = type_parameter(&mut metadata, target, 4021, TypeParameterConstraints::None);
    let body = expression(&mut metadata, 4021, ExpressionOperation::Unit {});
    call_function(&mut metadata, 4021, &[], unit, FunctionEffect::Pure, body);
    call_function_mut(&mut metadata, target).type_parameters = vec![formal];
    let body = expression(
        &mut metadata,
        4022,
        ExpressionOperation::Call {
            function: target,
            type_arguments: vec![actual],
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: Vec::new(),
        },
    );
    call_function(&mut metadata, 4022, &[], unit, FunctionEffect::Pure, body);
    transitive_metadata_assert_local_graph(&metadata);
    let proof = admit(&metadata).unwrap();
    assert_eq!(
        proof.prepared_category(actual).unwrap(),
        ValueCategory::OrdinaryData
    );
    assert!(!proof.prepared_metadata(actual).unwrap());
    // This ledger-only query must visit a shared ordinary DAG in bounded work;
    // it establishes no execution or resource-flow authority.
    let mut dag = fixture.source.clone();
    let mut previous = intern(&mut dag, TypeForm::I64);
    let mut roots = Vec::new();
    for level in 0..24 {
        previous = intern(
            &mut dag,
            TypeForm::StructuralRecord {
                fields: vec![
                    StructuralTypeField {
                        name: name("left"),
                        ty: previous,
                    },
                    StructuralTypeField {
                        name: name("right"),
                        ty: previous,
                    },
                ],
            },
        );
        roots.push(previous);
        if level == 7 {
            transitive_metadata_assert_local_graph(&dag);
            admit(&dag).unwrap();
        }
    }
    transitive_metadata_assert_local_graph(&dag);
    let late = intern(
        &mut dag,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: name("left"),
                    ty: previous,
                },
                StructuralTypeField {
                    name: name("right"),
                    ty: fixture.resource,
                },
            ],
        },
    );
    let control = ExecutionControl::uncancelled();
    let proof = admit(&fixture.source).unwrap();
    let measure = |ty, levels: usize| {
        let limit = 64 * (levels + 2);
        let mut budget = Budget::new(&control);
        budget.remaining = limit;
        let mut resolver = proof.resolver_with_budget(&dag.types, budget);
        let free = resolver.resource_free(ty, &Context::default(), 0).unwrap();
        (
            free,
            limit - resolver.budget.remaining,
            resolver.budget.entries,
        )
    };
    let (small_free, small_work, small_storage) = measure(roots[11], 12);
    let (large_free, large_work, large_storage) = measure(roots[23], 24);
    assert!(small_free && large_free);
    assert!(large_work <= 3 * small_work + 64);
    assert!(large_storage <= 3 * small_storage + 64);
    assert!(!measure(late, 25).0);
    assert!(measure(roots[23], 24).0);
    assert!(admit(&fixture.source).is_ok());
}

struct MixedBorrowedChoiceFixture {
    source: KernelSnapshot,
    function: DeclarationReference,
    matching: ExpressionId,
    ordinary_binding: BindingId,
    owned_binding: BindingId,
    ordinary_body: ExpressionId,
    owned_body: ExpressionId,
    source_expression: ExpressionId,
    choice: TypeObjectDigest,
    scalar: TypeObjectDigest,
    cell: TypeObjectDigest,
}

fn mixed_borrowed_choice_fixture(source_use: ParameterUse) -> MixedBorrowedChoiceFixture {
    let mut source = empty();
    let scalar = intern(&mut source, TypeForm::I64);
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let choice = intern(
        &mut source,
        TypeForm::OwnedChoice {
            cases: vec![
                StructuralTypeField {
                    name: name("accepted"),
                    ty: scalar,
                },
                StructuralTypeField {
                    name: name("rejected"),
                    ty: cell,
                },
            ],
        },
    );
    let read_body = expression(&mut source, 5101, ExpressionOperation::I64 { value: 19 });
    let reader = call_function(
        &mut source,
        5101,
        &[(cell, ParameterUse::Borrow)],
        scalar,
        FunctionEffect::Pure,
        read_body,
    );
    let ordinary_binding = call_binding(
        &mut source,
        5100,
        BindingKind::OwnedBorrow,
        Some(scalar),
        None,
    );
    let owned_binding = call_binding(
        &mut source,
        5101,
        BindingKind::OwnedBorrow,
        Some(cell),
        None,
    );
    let ordinary_body = expression(
        &mut source,
        5102,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(ordinary_binding),
        },
    );
    let child = expression(
        &mut source,
        5103,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(owned_binding),
        },
    );
    let owned_body = call_named(&mut source, 5104, reader, vec![child]);
    let source_expression = call_local(&mut source, 5105, call_parameter(5100, 0));
    let matching = expression(
        &mut source,
        5106,
        ExpressionOperation::MatchBorrowedOwned {
            choice_type: choice,
            source: source_expression,
            arms: vec![
                kernel::OwnedChoiceArm {
                    name: name("accepted"),
                    binding: ordinary_binding,
                    body: ordinary_body,
                },
                kernel::OwnedChoiceArm {
                    name: name("rejected"),
                    binding: owned_binding,
                    body: owned_body,
                },
            ],
        },
    );
    let function = call_function(
        &mut source,
        5100,
        &[(choice, source_use)],
        scalar,
        FunctionEffect::Pure,
        matching,
    );
    MixedBorrowedChoiceFixture {
        source,
        function,
        matching,
        ordinary_binding,
        owned_binding,
        ordinary_body,
        owned_body,
        source_expression,
        choice,
        scalar,
        cell,
    }
}

#[test]
fn raw_borrowed_choice_copies_ordinary_payloads_and_keeps_source_frozen_in_every_arm() {
    // Both parameter modes retain the complete source guard in every arm.
    // These raw controls do not execute the VM's custody/fuel fixture.
    for source_use in [ParameterUse::Borrow, ParameterUse::Consume] {
        let fixture = mixed_borrowed_choice_fixture(source_use);
        call_assert_local(&fixture.source);
        let proof = admit(&fixture.source).unwrap();
        assert!(!proof.admits_function_flow(fixture.function));
        for binding in [fixture.ordinary_binding, fixture.owned_binding] {
            assert_eq!(
                proof.lexical_function_for_binding(fixture.function.package, binding),
                Some(fixture.function),
            );
        }
        assert_eq!(
            proof.prepared_category(fixture.choice).unwrap(),
            ValueCategory::Owned
        );
        assert_eq!(
            proof.prepared_category(fixture.scalar).unwrap(),
            ValueCategory::OrdinaryData
        );

        for fault in 0..4 {
            let mut changed = fixture.source.clone();
            let OwnerRecord::Binding(binding) = changed
                .owners
                .get_mut(&OwnerKey::Binding(fixture.ordinary_binding))
                .unwrap()
            else {
                unreachable!()
            };
            let expected = match fault {
                0 => {
                    binding.value = Some(fixture.ordinary_body);
                    "choice payload binding has an initializer"
                }
                1 => {
                    binding.declared_type = Some(fixture.cell);
                    "choice payload annotation counterfeits an OwnedChild path"
                }
                2 => {
                    binding.kind = BindingKind::OwnedChoicePayload;
                    "binding kind does not match its lexical operation"
                }
                _ => {
                    let ExpressionOperation::MatchBorrowedOwned { arms, .. } =
                        call_operation_mut(&mut changed, fixture.matching)
                    else {
                        unreachable!()
                    };
                    arms.swap(0, 1);
                    "choice path omits or duplicates inactive cases"
                }
            };
            let error = admit(&changed).unwrap_err();
            assert!(
                error.message.starts_with(expected),
                "source-use={source_use:?}, fault={fault}: {}",
                error.message
            );
        }

        // The ordinary result is a copied I64, but the read of the complete
        // choice still spans this arm and rejects consumption of its source.
        let mut changed = fixture.source.clone();
        let body = expression(&mut changed, 5110, ExpressionOperation::I64 { value: 0 });
        let consume_choice = call_function(
            &mut changed,
            5110,
            &[(fixture.choice, ParameterUse::Consume)],
            fixture.scalar,
            FunctionEffect::Pure,
            body,
        );
        *call_operation_mut(&mut changed, fixture.ordinary_body) = ExpressionOperation::Call {
            function: consume_choice,
            type_arguments: Vec::new(),
            read_scope_arguments: Vec::new(),
            effect_arguments: Vec::new(),
            requirement_arguments: Vec::new(),
            arguments: vec![fixture.source_expression],
        };
        let expected = if source_use == ParameterUse::Consume {
            "source consume/mutation occurs under a retained read/view guard"
        } else {
            "borrowed parameter/read handle is consumed"
        };
        assert!(admit(&changed).unwrap_err().message.starts_with(expected));

        let mut changed = fixture.source.clone();
        let body = expression(&mut changed, 5111, ExpressionOperation::I64 { value: 0 });
        let consume_cell = call_function(
            &mut changed,
            5111,
            &[(fixture.cell, ParameterUse::Consume)],
            fixture.scalar,
            FunctionEffect::Pure,
            body,
        );
        let ExpressionOperation::Call { function, .. } =
            call_operation_mut(&mut changed, fixture.owned_body)
        else {
            unreachable!()
        };
        *function = consume_cell;
        assert!(
            admit(&changed)
                .unwrap_err()
                .message
                .starts_with("borrowed parameter/read handle is consumed",)
        );

        // ReadFrom of the complete choice remains tied to its declared
        // parameter; an ordinary arm does not erase a different lineage.
        let mut retained = fixture.source.clone();
        let target = call_function_mut(&mut retained, fixture.function);
        target.result = CallableResult {
            ty: fixture.choice,
            relation: ResultRelation::ReadFrom {
                source: call_parameter(5100, 0),
            },
        };
        let OwnerRecord::Parameter(parameter) = retained
            .owners
            .get_mut(&OwnerKey::Parameter(call_parameter(5100, 0)))
            .unwrap()
        else {
            unreachable!()
        };
        parameter.use_mode = ParameterUse::Borrow;
        *call_operation_mut(&mut retained, fixture.ordinary_body) = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(call_parameter(5100, 0)),
        };
        let ExpressionOperation::Call { arguments, .. } =
            call_operation_mut(&mut retained, fixture.owned_body)
        else {
            unreachable!()
        };
        let child = arguments[0];
        retained.owners.remove(&OwnerKey::Expression(child));
        *call_operation_mut(&mut retained, fixture.owned_body) = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(call_parameter(5100, 0)),
        };
        call_assert_local(&retained);
        admit(&retained).unwrap();
        let other = call_parameter(5100, 1);
        retained.owners.insert(
            OwnerKey::Parameter(other),
            OwnerRecord::Parameter(kernel::ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(other), OwnerKind::Parameter),
                parent: ParameterParent::Function(fixture.function.declaration),
                name: name("other"),
                ty: fixture.choice,
                use_mode: ParameterUse::Borrow,
                resource_requirement: None,
            }),
        );
        call_function_mut(&mut retained, fixture.function)
            .parameters
            .push(other);
        *call_operation_mut(&mut retained, fixture.ordinary_body) = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(other),
        };
        assert!(
            admit(&retained)
                .unwrap_err()
                .message
                .starts_with("inactive match arm changes borrowed source lineage",)
        );
    }
}

struct AffineConditionalFixture {
    source: KernelSnapshot,
    function: DeclarationReference,
    consumer: DeclarationReference,
    body: ExpressionId,
    first: ParameterId,
    second: ParameterId,
}

fn affine_cell_consumer(source: &mut KernelSnapshot) -> DeclarationReference {
    let cell = intern(source, TypeForm::OwnedI64Cell);
    let unit = intern(source, TypeForm::Unit);
    let body = expression(source, 6100, ExpressionOperation::Unit {});
    call_function(
        source,
        6100,
        &[(cell, ParameterUse::Consume)],
        unit,
        FunctionEffect::Pure,
        body,
    )
}

fn affine_conditional_fixture() -> AffineConditionalFixture {
    let mut source = empty();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let unit = intern(&mut source, TypeForm::Unit);
    let consumer = affine_cell_consumer(&mut source);
    let first = call_parameter(6000, 0);
    let second = call_parameter(6000, 1);
    let condition = expression(&mut source, 6000, ExpressionOperation::Bool { value: true });
    let first_argument = call_local(&mut source, 6001, first);
    let first_body = call_named(&mut source, 6002, consumer, vec![first_argument]);
    let second_argument = call_local(&mut source, 6003, second);
    let second_body = call_named(&mut source, 6004, consumer, vec![second_argument]);
    let body = expression(
        &mut source,
        6005,
        ExpressionOperation::If {
            condition,
            when_true: first_body,
            when_false: second_body,
        },
    );
    let function = call_function(
        &mut source,
        6000,
        &[(cell, ParameterUse::Consume), (cell, ParameterUse::Consume)],
        unit,
        FunctionEffect::Pure,
        body,
    );
    AffineConditionalFixture {
        source,
        function,
        consumer,
        body,
        first,
        second,
    }
}

#[test]
fn raw_conditional_joins_intersect_owned_liveness_without_changing_loan_lineage() {
    let fixture = affine_conditional_fixture();
    transitive_metadata_assert_local_graph(&fixture.source);
    admit(&fixture.source).unwrap();
    for source in [fixture.first, fixture.second] {
        let mut changed = fixture.source.clone();
        let argument = call_local(&mut changed, 6006, source);
        let reused = call_named(&mut changed, 6007, fixture.consumer, vec![argument]);
        let body = expression(
            &mut changed,
            6008,
            ExpressionOperation::Sequence {
                items: vec![fixture.body, reused],
            },
        );
        call_function_mut(&mut changed, fixture.function).body = body;
        let error = admit(&changed).unwrap_err();
        assert!(
            error
                .message
                .starts_with("view/Owned operation uses a dead or foreign lexical local"),
            "{}",
            error.message
        );
    }
    let mut loan = constructor(true);
    transitive_metadata_assert_local_graph(&loan.source);
    admit(&loan.source).unwrap();
    let length = expression(
        &mut loan.source,
        6009,
        ExpressionOperation::I64 { value: 1 },
    );
    let reference = call_local(&mut loan.source, 6010, loan.second);
    let TypeForm::ReadViewApplied { read_scope, .. } = loan.source.types[&loan.ty].form else {
        unreachable!()
    };
    let other = expression(
        &mut loan.source,
        6011,
        ExpressionOperation::CreateReadView {
            declaration: loan.view,
            type_arguments: Vec::new(),
            read_scope,
            source: LocalValueReference::FunctionParameter(loan.second),
            fields: vec![
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 2),
                    value: length,
                },
                ReadViewFieldInitializer {
                    field: FieldId::migrate(SEED, 3),
                    value: reference,
                },
            ],
        },
    );
    let condition = expression(
        &mut loan.source,
        6012,
        ExpressionOperation::Bool { value: true },
    );
    let body = expression(
        &mut loan.source,
        6013,
        ExpressionOperation::If {
            condition,
            when_true: loan.body,
            when_false: other,
        },
    );
    call_function_mut(&mut loan.source, loan.function).body = body;
    let error = admit(&loan.source).unwrap_err();
    assert!(
        error
            .message
            .starts_with("untaken branch changes selected source lineage/scope"),
        "{}",
        error.message
    );
}

#[test]
fn raw_owned_child_results_join_only_common_source_scope_permissions() {
    for choice in [false, true] {
        let mut source = empty();
        let cell = intern(&mut source, TypeForm::OwnedI64Cell);
        let fields = ["left", "right"]
            .into_iter()
            .map(|label| StructuralTypeField {
                name: name(label),
                ty: cell,
            })
            .collect();
        let input = intern(
            &mut source,
            if choice {
                TypeForm::OwnedChoice { cases: fields }
            } else {
                TypeForm::OwnedProduct { fields }
            },
        );
        let owner = call_parameter(6200, 0);
        let other = call_parameter(6200, 1);
        let mut children = Vec::new();
        let mut paths = Vec::new();
        for (ordinal, label) in [(6201, "left"), (6202, "right")] {
            let binding = call_binding(
                &mut source,
                ordinal,
                BindingKind::OwnedBorrow,
                Some(cell),
                None,
            );
            let body = expression(
                &mut source,
                ordinal,
                ExpressionOperation::Local {
                    value: LocalValueReference::LexicalBinding(binding),
                },
            );
            children.push(body);
            if choice {
                paths.push(kernel::OwnedChoiceArm {
                    name: name(label),
                    binding,
                    body,
                });
            } else {
                let value = call_local(&mut source, ordinal + 10, owner);
                let path = expression(
                    &mut source,
                    ordinal + 20,
                    ExpressionOperation::BorrowOwnedField {
                        product_type: input,
                        source: value,
                        field: name(label),
                        binding,
                        body,
                    },
                );
                paths.push(kernel::OwnedChoiceArm {
                    name: name(label),
                    binding,
                    body: path,
                });
            }
        }
        let root = if choice {
            let value = call_local(&mut source, 6203, owner);
            expression(
                &mut source,
                6204,
                ExpressionOperation::MatchBorrowedOwned {
                    choice_type: input,
                    source: value,
                    arms: paths,
                },
            )
        } else {
            let condition =
                expression(&mut source, 6203, ExpressionOperation::Bool { value: true });
            let joined = expression(
                &mut source,
                6204,
                ExpressionOperation::If {
                    condition,
                    when_true: children[0],
                    when_false: paths[1].body,
                },
            );
            let ExpressionOperation::BorrowOwnedField { body, .. } =
                call_operation_mut(&mut source, paths[0].body)
            else {
                unreachable!()
            };
            // Like fifo-front: one result is reached through an enclosing
            // child loan, the other through its sibling from the same owner.
            *body = joined;
            paths[0].body
        };
        let function = call_function(
            &mut source,
            6200,
            &[(input, ParameterUse::Borrow), (cell, ParameterUse::Borrow)],
            cell,
            FunctionEffect::Pure,
            root,
        );
        call_function_mut(&mut source, function).result.relation =
            ResultRelation::ReadFrom { source: owner };
        transitive_metadata_assert_local_graph(&source);
        admit(&source).unwrap();

        // Discharging a lexical permission never merges different source roots,
        // including the inactive conditional path or inactive choice arm.
        *call_operation_mut(&mut source, children[1]) = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(other),
        };
        let error = admit(&source).unwrap_err();
        assert!(
            error.message.starts_with(if choice {
                "inactive match arm changes borrowed source lineage"
            } else {
                "untaken branch changes selected source lineage/scope"
            }),
            "choice={choice}: {}",
            error.message,
        );
    }
}

#[test]
fn raw_three_arm_matches_keep_the_join_from_the_first_consuming_arm() {
    for owned in [false, true] {
        let mut source = empty();
        let cell = intern(&mut source, TypeForm::OwnedI64Cell);
        let unit = intern(&mut source, TypeForm::Unit);
        let consumer = affine_cell_consumer(&mut source);
        let function = decl(&source, 6011);
        let owner = call_parameter(6011, 1);
        let value = call_parameter(6011, 0);
        let input = if owned {
            intern(
                &mut source,
                TypeForm::OwnedChoice {
                    cases: ["a", "b", "c"]
                        .into_iter()
                        .map(|label| StructuralTypeField {
                            name: name(label),
                            ty: if label == "a" { cell } else { unit },
                        })
                        .collect(),
                },
            )
        } else {
            let variant = decl(&source, 6010);
            let mut cases = Vec::new();
            for ordinal in 6010..6013 {
                let case = crate::platform::semantic_id::CaseId::migrate(SEED, ordinal);
                source.owners.insert(
                    OwnerKey::Case(case),
                    OwnerRecord::Case(kernel::CaseRecord {
                        header: OwnerHeader::new(OwnerKey::Case(case), OwnerKind::Case),
                        declaration: variant.declaration,
                        name: name(&format!("case_{ordinal}")),
                        payload: None,
                    }),
                );
                cases.push(case);
            }
            cases.sort();
            declaration(
                &mut source,
                variant,
                0,
                OwnerKind::Variant,
                DeclarationPayload::Variant {
                    type_parameters: Vec::new(),
                    cases,
                },
            );
            intern(
                &mut source,
                TypeForm::Named {
                    declaration: variant,
                },
            )
        };
        let source_expression = call_local(&mut source, 6010, value);
        let argument = call_local(&mut source, 6011, owner);
        let first = call_named(&mut source, 6012, consumer, vec![argument]);
        let second = expression(&mut source, 6013, ExpressionOperation::Unit {});
        let third = expression(&mut source, 6014, ExpressionOperation::Unit {});
        let bodies = [first, second, third];
        let operation = if owned {
            let arms = ["a", "b", "c"]
                .into_iter()
                .enumerate()
                .map(|(position, label)| {
                    let binding = call_binding(
                        &mut source,
                        6010 + position as u64,
                        BindingKind::OwnedChoicePayload,
                        Some(if label == "a" { cell } else { unit }),
                        None,
                    );
                    kernel::OwnedChoiceArm {
                        name: name(label),
                        binding,
                        body: bodies[position],
                    }
                })
                .collect();
            ExpressionOperation::MatchOwned {
                choice_type: input,
                source: source_expression,
                arms,
            }
        } else {
            let OwnerRecord::Declaration(record) =
                &source.owners[&OwnerKey::Declaration(DeclarationId::migrate(SEED, 6010))]
            else {
                unreachable!()
            };
            let DeclarationPayload::Variant { cases, .. } = &record.payload else {
                unreachable!()
            };
            let arms = cases
                .iter()
                .enumerate()
                .map(|(position, case)| kernel::MatchExpressionArm {
                    case: kernel::CaseReference {
                        package: function.package,
                        case: *case,
                    },
                    payload_binding: None,
                    body: bodies[position],
                })
                .collect();
            ExpressionOperation::Match {
                value: source_expression,
                arms,
            }
        };
        let matching = expression(&mut source, 6015, operation);
        call_function(
            &mut source,
            6011,
            &[
                (
                    input,
                    if owned {
                        ParameterUse::Consume
                    } else {
                        ParameterUse::Unrestricted
                    },
                ),
                (cell, ParameterUse::Consume),
            ],
            unit,
            FunctionEffect::Pure,
            matching,
        );
        transitive_metadata_assert_local_graph(&source);
        admit(&source).unwrap();
        let argument = call_local(&mut source, 6016, owner);
        let reused = call_named(&mut source, 6017, consumer, vec![argument]);
        let body = expression(
            &mut source,
            6018,
            ExpressionOperation::Sequence {
                items: vec![matching, reused],
            },
        );
        call_function_mut(&mut source, function).body = body;
        let error = admit(&source).unwrap_err();
        assert!(
            error
                .message
                .starts_with("view/Owned operation uses a dead or foreign lexical local"),
            "owned={owned}: {}",
            error.message
        );
    }
}

#[test]
fn raw_discarded_owned_values_consume_their_source_and_scalars_remain_copyable() {
    let mut source = empty();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let unit = intern(&mut source, TypeForm::Unit);
    let consumer = affine_cell_consumer(&mut source);
    let owner = call_parameter(6020, 0);
    let discarded = call_local(&mut source, 6020, owner);
    let end = expression(&mut source, 6021, ExpressionOperation::Unit {});
    let body = expression(
        &mut source,
        6022,
        ExpressionOperation::Sequence {
            items: vec![discarded, end],
        },
    );
    let function = call_function(
        &mut source,
        6020,
        &[(cell, ParameterUse::Consume)],
        unit,
        FunctionEffect::Pure,
        body,
    );
    transitive_metadata_assert_local_graph(&source);
    admit(&source).unwrap();
    let mut reused = source.clone();
    let argument = call_local(&mut reused, 6023, owner);
    let consumed = call_named(&mut reused, 6024, consumer, vec![argument]);
    let continuation = expression(
        &mut reused,
        6025,
        ExpressionOperation::Sequence {
            items: vec![body, consumed],
        },
    );
    call_function_mut(&mut reused, function).body = continuation;
    let error = admit(&reused).unwrap_err();
    assert!(
        error
            .message
            .starts_with("view/Owned operation uses a dead or foreign lexical local"),
        "{}",
        error.message
    );
    let mut borrowed = source.clone();
    let OwnerRecord::Parameter(record) = borrowed
        .owners
        .get_mut(&OwnerKey::Parameter(owner))
        .unwrap()
    else {
        unreachable!()
    };
    record.use_mode = ParameterUse::Borrow;
    let error = admit(&borrowed).unwrap_err();
    assert!(
        error
            .message
            .starts_with("borrowed parameter/read handle is consumed"),
        "{}",
        error.message
    );
    for mode in [ParameterUse::Borrow, ParameterUse::Consume] {
        let mut copied = source.clone();
        let OwnerRecord::Parameter(record) =
            copied.owners.get_mut(&OwnerKey::Parameter(owner)).unwrap()
        else {
            unreachable!()
        };
        record.use_mode = mode;
        let binding = call_binding(&mut copied, 6026, BindingKind::Let, None, Some(discarded));
        *call_operation_mut(&mut copied, body) = ExpressionOperation::Let {
            bindings: vec![binding],
            body: end,
        };
        let error = admit(&copied).unwrap_err();
        assert!(
            error
                .message
                .starts_with("affine Owned local requires consume/borrow/return use"),
            "mode={mode:?}: {}",
            error.message
        );
    }
    let mut scalar = empty();
    let ty = intern(&mut scalar, TypeForm::I64);
    let unit = intern(&mut scalar, TypeForm::Unit);
    let value = call_local(&mut scalar, 6020, call_parameter(6020, 0));
    let repeated = call_local(&mut scalar, 6021, call_parameter(6020, 0));
    let end = expression(&mut scalar, 6022, ExpressionOperation::Unit {});
    let body = expression(
        &mut scalar,
        6023,
        ExpressionOperation::Sequence {
            items: vec![value, repeated, end],
        },
    );
    call_function(
        &mut scalar,
        6020,
        &[(ty, ParameterUse::Unrestricted)],
        unit,
        FunctionEffect::Pure,
        body,
    );
    transitive_metadata_assert_local_graph(&scalar);
    admit(&scalar).unwrap();
    let mut guarded = constructor(true);
    guarded.source.types.remove(&guarded.ty);
    let unit = intern(&mut guarded.source, TypeForm::Unit);
    let descriptor = BindingId::migrate(SEED, 6030);
    let lexical = ReadScopeArgument::Lexical {
        callable: guarded.function,
        binding: descriptor,
    };
    let view = intern(
        &mut guarded.source,
        TypeForm::ReadViewApplied {
            declaration: guarded.view,
            arguments: Vec::new(),
            read_scope: lexical,
        },
    );
    let ExpressionOperation::CreateReadView { read_scope, .. } =
        call_operation_mut(&mut guarded.source, guarded.body)
    else {
        unreachable!()
    };
    *read_scope = lexical;
    call_binding(
        &mut guarded.source,
        6030,
        BindingKind::ReadView,
        Some(view),
        Some(guarded.body),
    );
    for parameter in [guarded.first, guarded.second] {
        let OwnerRecord::Parameter(record) = guarded
            .source
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        record.use_mode = ParameterUse::Consume;
    }
    let drop = expression(
        &mut guarded.source,
        6030,
        ExpressionOperation::DropReadView {
            value: LocalValueReference::LexicalBinding(descriptor),
        },
    );
    let condition = expression(
        &mut guarded.source,
        6031,
        ExpressionOperation::Bool { value: true },
    );
    let joined = expression(
        &mut guarded.source,
        6032,
        ExpressionOperation::If {
            condition,
            when_true: drop,
            when_false: drop,
        },
    );
    let consumer = affine_cell_consumer(&mut guarded.source);
    let parent = call_local(&mut guarded.source, 6033, guarded.first);
    let consumed = call_named(&mut guarded.source, 6034, consumer, vec![parent]);
    let continuation = expression(
        &mut guarded.source,
        6035,
        ExpressionOperation::Sequence {
            items: vec![joined, consumed],
        },
    );
    let body = expression(
        &mut guarded.source,
        6036,
        ExpressionOperation::Let {
            bindings: vec![descriptor],
            body: continuation,
        },
    );
    let function = call_function_mut(&mut guarded.source, guarded.function);
    function.read_scope_parameters.clear();
    function.read_scope_sources.clear();
    function.result = CallableResult {
        ty: unit,
        relation: ResultRelation::Value,
    };
    function.body = body;
    transitive_metadata_assert_local_graph(&guarded.source);
    admit(&guarded.source).unwrap();
    // Releasing the guard in just one branch cannot make its source available
    // to the continuation while the other branch still retains the view.
    let inactive = expression(&mut guarded.source, 6037, ExpressionOperation::Unit {});
    let ExpressionOperation::If { when_false, .. } =
        call_operation_mut(&mut guarded.source, joined)
    else {
        unreachable!()
    };
    *when_false = inactive;
    let error = admit(&guarded.source).unwrap_err();
    assert!(
        error
            .message
            .starts_with("untaken branch changes affine liveness/source guards"),
        "{}",
        error.message
    );
}

#[test]
fn raw_prepared_foreign_view_layout_keeps_carrier_and_scope_authority_separate() {
    let mut consumer = transitive_metadata_empty(7100);
    let contract = decl(&consumer, 7100);
    let (self_parameter, self_type) = type_parameter(
        &mut consumer,
        contract,
        7100,
        TypeParameterConstraints::Owned,
    );
    let unit = intern(&mut consumer, TypeForm::Unit);
    let method = crate::platform::semantic_id::MethodId::migrate(SEED, 7100);
    let binder = ReadScopeBinder::ContractMethod { contract, method };
    let actual_scope = scope(binder, 7100);
    declaration(
        &mut consumer,
        contract,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(kernel::OwnedContract {
            self_parameter,
            type_parameters: Vec::new(),
            associated_types: Vec::new(),
            methods: vec![kernel::OwnedMethod {
                id: method,
                name: name("inspect"),
                parameters: vec![kernel::OwnedMethodParameter {
                    ty: self_type,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: unit,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![ReadScopeParameter {
                    id: ReadScopeParameterId::migrate(SEED, 7100),
                    name: name("s"),
                }],
                read_scope_sources: vec![ReadScopeSource {
                    source: 0,
                    scope: actual_scope,
                }],
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    let scalar = intern(&mut consumer, TypeForm::I64);
    let owned = intern(&mut consumer, TypeForm::OwnedI64Cell);
    let secret = intern(&mut consumer, TypeForm::Secret);
    transitive_metadata_public(&mut consumer);
    let mut supplier = transitive_metadata_empty(7101);
    let (view, _) = add_view(
        &mut supplier,
        7101,
        true,
        Some(TypeParameterConstraints::Transferable),
    );
    transitive_metadata_import(&mut supplier, &consumer, 41);
    transitive_metadata_assert_import(&supplier, &consumer);
    assert!(
        !consumer
            .dependencies
            .contains_key(&supplier.root.package_id)
    );
    for source in [&consumer, &supplier] {
        transitive_metadata_assert_local_graph(source);
    }
    let object = TypeObject::new(TypeForm::ReadViewApplied {
        declaration: view,
        arguments: vec![scalar],
        read_scope: actual_scope,
    })
    .unwrap();
    let derived = encode_type_object(&object).unwrap().0;
    let additional = BTreeMap::from([(derived, object.clone())]);
    let control = ExecutionControl::uncancelled();
    let healthy =
        Proof::reconstruct_with_types(&[&consumer, &supplier], &additional, &control).unwrap();
    assert_eq!(healthy.prepared_layout(derived).unwrap().declaration, view);
    assert_eq!(
        healthy.prepared_layout(derived).unwrap().read_scope,
        actual_scope
    );
    assert_eq!(
        healthy.prepared_category(derived).unwrap(),
        ValueCategory::ReadView
    );
    assert!(healthy.prepared_read_shareable(derived).unwrap());
    assert!(!healthy.dependencies[&consumer.root.package_id].contains(&supplier.root.package_id));

    // The same bytes cannot acquire source naming authority from preparation.
    let mut authored = consumer.clone();
    authored.types.insert(derived, object.clone());
    let OwnerRecord::Declaration(record) = authored
        .owners
        .get_mut(&OwnerKey::Declaration(contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract_record) = &mut record.payload else {
        unreachable!()
    };
    contract_record.methods[0].result = CallableResult {
        ty: derived,
        relation: ResultRelation::ViewFrom {
            source: 0,
            scope: actual_scope,
        },
    };
    let mut coherent_supplier = supplier.clone();
    transitive_metadata_import(&mut coherent_supplier, &authored, 42);
    transitive_metadata_refuses(
        &[&authored, &coherent_supplier],
        "reference is outside its exact package dependency inventory",
    );
    let missing_formal = TypeObject::new(TypeForm::TypeParameter {
        parameter: TypeParameterId::migrate(SEED, 7199),
    })
    .unwrap();
    let missing_formal_digest = encode_type_object(&missing_formal).unwrap().0;
    for (fault, read_scope, argument) in [
        ("missing-scope", scope(binder, 7199), scalar),
        (
            "missing-method",
            scope(
                ReadScopeBinder::ContractMethod {
                    contract,
                    method: crate::platform::semantic_id::MethodId::migrate(SEED, 7199),
                },
                7100,
            ),
            scalar,
        ),
        ("phantom-owned-category", actual_scope, owned),
        ("phantom-transfer", actual_scope, secret),
        (
            "unbound-phantom-formal",
            actual_scope,
            missing_formal_digest,
        ),
    ] {
        let changed = TypeObject::new(TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![argument],
            read_scope,
        })
        .unwrap();
        let ty = encode_type_object(&changed).unwrap().0;
        let mut raw = BTreeMap::from([(ty, changed)]);
        if argument == missing_formal_digest {
            raw.insert(missing_formal_digest, missing_formal.clone());
        }
        assert!(
            Proof::reconstruct_with_types(&[&consumer, &supplier], &raw, &control).is_err(),
            "{fault}"
        );
    }
    assert!(healthy.prepared_layout(derived).is_ok());
    Proof::reconstruct_with_types(&[&supplier, &consumer], &additional, &control).unwrap();
}

#[test]
fn raw_nominal_field_metadata_keeps_defining_and_actual_import_edges_separate() {
    let mut metadata = transitive_metadata_empty(7200);
    let metadata_record = decl(&metadata, 7200);
    let scalar = intern(&mut metadata, TypeForm::I64);
    record_fields(
        &mut metadata,
        metadata_record,
        7200,
        false,
        Vec::new(),
        &[scalar],
    );
    let metadata_type = intern(
        &mut metadata,
        TypeForm::Named {
            declaration: metadata_record,
        },
    );
    transitive_metadata_public(&mut metadata);
    let mut scheme = transitive_metadata_empty(7201);
    let used = decl(&scheme, 7201);
    let (used_formal, used_type) =
        type_parameter(&mut scheme, used, 7201, TypeParameterConstraints::None);
    record_fields(
        &mut scheme,
        used,
        7201,
        false,
        vec![used_formal],
        &[used_type, metadata_type],
    );
    // Canonical record child IDs are ordered by identity, not migration ordinal.
    {
        let OwnerRecord::Declaration(record) = scheme
            .owners
            .get_mut(&OwnerKey::Declaration(used.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Record { fields, .. } = &mut record.payload else {
            unreachable!()
        };
        fields.sort();
    }
    let phantom = decl(&scheme, 7202);
    let (phantom_formal, _) =
        type_parameter(&mut scheme, phantom, 7202, TypeParameterConstraints::None);
    record_fields(
        &mut scheme,
        phantom,
        7202,
        false,
        vec![phantom_formal],
        &[metadata_type],
    );
    let variant = decl(&scheme, 7203);
    let case = crate::platform::semantic_id::CaseId::migrate(SEED, 7203);
    scheme.owners.insert(
        OwnerKey::Case(case),
        OwnerRecord::Case(kernel::CaseRecord {
            header: OwnerHeader::new(OwnerKey::Case(case), OwnerKind::Case),
            declaration: variant.declaration,
            name: name("metadata"),
            payload: Some(metadata_type),
        }),
    );
    declaration(
        &mut scheme,
        variant,
        0,
        OwnerKind::Variant,
        DeclarationPayload::Variant {
            type_parameters: Vec::new(),
            cases: vec![case],
        },
    );
    let variant_type = intern(
        &mut scheme,
        TypeForm::Named {
            declaration: variant,
        },
    );
    transitive_metadata_public(&mut scheme);
    transitive_metadata_import(&mut scheme, &metadata, 43);
    let mut caller = transitive_metadata_empty(7202);
    let local = decl(&caller, 7204);
    record_fields(&mut caller, local, 7204, false, Vec::new(), &[scalar]);
    let caller_actual = intern(&mut caller, TypeForm::Named { declaration: local });
    let used_instance = intern(
        &mut caller,
        TypeForm::Applied {
            declaration: used,
            arguments: vec![caller_actual],
        },
    );
    let phantom_instance = intern(
        &mut caller,
        TypeForm::Applied {
            declaration: phantom,
            arguments: vec![caller_actual],
        },
    );
    let unit = intern(&mut caller, TypeForm::Unit);
    let body = expression(&mut caller, 7205, ExpressionOperation::Unit {});
    let function = call_function(
        &mut caller,
        7205,
        &[
            (used_instance, ParameterUse::Unrestricted),
            (phantom_instance, ParameterUse::Unrestricted),
            (variant_type, ParameterUse::Unrestricted),
        ],
        unit,
        FunctionEffect::Pure,
        body,
    );
    transitive_metadata_public(&mut caller);
    transitive_metadata_import(&mut caller, &scheme, 44);
    assert!(!caller.dependencies.contains_key(&metadata.root.package_id));
    for source in [&caller, &scheme, &metadata] {
        transitive_metadata_assert_local_graph(source);
    }
    transitive_metadata_assert_import(&scheme, &metadata);
    transitive_metadata_assert_import(&caller, &scheme);
    let control = ExecutionControl::uncancelled();
    let proof = Proof::reconstruct(&[&caller, &scheme, &metadata], &control).unwrap();
    let context = Context::declaration(function);
    let mut resolver = proof.resolver_with_control(proof.types(), &control);
    for ty in [used_instance, phantom_instance, variant_type] {
        let facts = resolver.facts(ty, &context, 0).unwrap();
        assert_eq!(facts.category, ValueCategory::OrdinaryData);
        assert!(facts.metadata && facts.transfer && facts.share);
        assert!(proof.prepared_transferable(ty).unwrap());
        assert!(
            resolver
                .capture_safe(ty, &context, &mut BTreeSet::new(), 0)
                .unwrap()
        );
        assert!(resolver.memory_free(ty, &context, 0).unwrap());
        assert!(resolver.resource_free(ty, &context, 0).unwrap());
    }
    // One resource walk reaches the same exact Metadata digest first through
    // a lawful defining field, then as a caller phantom actual lacking its edge.
    let mut query = caller.clone();
    let forbidden_actual = intern(
        &mut query,
        TypeForm::Applied {
            declaration: phantom,
            arguments: vec![metadata_type],
        },
    );
    let combined = intern(
        &mut query,
        TypeForm::StructuralRecord {
            fields: vec![
                StructuralTypeField {
                    name: name("actual"),
                    ty: forbidden_actual,
                },
                StructuralTypeField {
                    name: name("field"),
                    ty: used_instance,
                },
            ],
        },
    );
    let mut query_resolver = proof.resolver_with_control(&query.types, &control);
    assert_eq!(
        query_resolver
            .resource_free(combined, &context, 0)
            .unwrap_err()
            .message,
        "reference is outside its exact package dependency inventory"
    );
    assert!(
        query_resolver
            .capture_safe(forbidden_actual, &context, &mut BTreeSet::new(), 0)
            .is_err()
    );
    assert!(query_resolver.facts(forbidden_actual, &context, 0).is_err());
    for declaration in [used, phantom] {
        let mut wrong_actual = caller.clone();
        let instance = intern(
            &mut wrong_actual,
            TypeForm::Applied {
                declaration,
                arguments: vec![metadata_type],
            },
        );
        let parameter = call_parameter(7205, 0);
        let OwnerRecord::Parameter(record) = wrong_actual
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        record.ty = instance;
        transitive_metadata_refuses(
            &[&wrong_actual, &scheme, &metadata],
            "reference is outside its exact package dependency inventory",
        );
        // Same digest in a caller actual and the defining raw field is not one
        // permission. The caller edge must not repair the missing defining edge.
        transitive_metadata_import(&mut wrong_actual, &metadata, 45);
        let mut no_defining_edge = scheme.clone();
        no_defining_edge
            .dependencies
            .remove(&metadata.root.package_id);
        transitive_metadata_import(&mut wrong_actual, &no_defining_edge, 46);
        assert_eq!(
            wrong_actual.types[&instance].form,
            TypeForm::Applied {
                declaration,
                arguments: vec![metadata_type],
            }
        );
        transitive_metadata_refuses(
            &[&wrong_actual, &no_defining_edge, &metadata],
            "reference is outside its exact package dependency inventory",
        );
        // Restore the exact defining edge as well; neither refusal poisons reuse.
        transitive_metadata_import(&mut wrong_actual, &scheme, 44);
        Proof::reconstruct(&[&wrong_actual, &scheme, &metadata], &control).unwrap();
    }
    Proof::reconstruct(&[&metadata, &scheme, &caller], &control).unwrap();
}

#[test]
fn raw_lexical_associated_projection_preparation_keeps_witness_and_loan_authority_exact() {
    // A real raw named function establishes this BorrowCall loan. Derived type
    // preparation may use that exact ownership proof, never an invented seal.
    let mut fixture = pending_call();
    let contract = decl(&fixture.source, 1400);
    let (self_parameter, self_ty) = type_parameter(
        &mut fixture.source,
        contract,
        1400,
        TypeParameterConstraints::Owned,
    );
    let OwnerRecord::TypeParameter(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::TypeParameter(self_parameter))
        .unwrap()
    else {
        unreachable!()
    };
    record.name = name("Self");
    let member = AssociatedTypeMemberId::migrate(SEED, 1400);
    let unit = intern(&mut fixture.source, TypeForm::Unit);
    declaration(
        &mut fixture.source,
        contract,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(kernel::OwnedContract {
            self_parameter,
            type_parameters: Vec::new(),
            associated_types: vec![kernel::AssociatedTypeMember {
                id: member,
                name: name("View"),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: ReadScopeParameter {
                        id: ReadScopeParameterId::migrate(SEED, 1400),
                        name: name("s"),
                    },
                },
            }],
            methods: vec![kernel::OwnedMethod {
                id: crate::platform::semantic_id::MethodId::migrate(SEED, 1400),
                name: name("inspect"),
                parameters: vec![kernel::OwnedMethodParameter {
                    ty: self_ty,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: unit,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    // Match the canonical lexical witness premise: Self is an exact caller
    // Owned formal, even though this witness is unused by the concrete loan.
    let (caller_self_parameter, caller_self_ty) = type_parameter(
        &mut fixture.source,
        fixture.caller,
        1401,
        TypeParameterConstraints::Owned,
    );
    call_function_mut(&mut fixture.source, fixture.caller)
        .type_parameters
        .push(caller_self_parameter);
    let witness = crate::platform::semantic_id::ImplementationParameterId::migrate(SEED, 1400);
    call_function_mut(&mut fixture.source, fixture.caller)
        .implementation_parameters
        .push(kernel::ImplementationParameter {
            id: witness,
            name: name("reader"),
            contract,
            self_type: caller_self_ty,
            type_arguments: Vec::new(),
        });
    call_assert_local(&fixture.source);
    let control = ExecutionControl::uncancelled();
    let raw = Proof::reconstruct(&[&fixture.source], &control).unwrap();
    assert!(raw.admits_function_flow(fixture.caller));
    assert_eq!(
        raw.lexical_function_for_binding(fixture.caller.package, fixture.binding),
        Some(fixture.caller)
    );
    let lexical = ReadScopeArgument::Lexical {
        callable: fixture.caller,
        binding: fixture.binding,
    };
    let projected = TypeObject::new(TypeForm::AssociatedReadViewProjection {
        scope: fixture.caller,
        parameter: witness,
        contract,
        member,
        read_scope: lexical,
    })
    .unwrap();
    let projected_ty = encode_type_object(&projected).unwrap().0;
    assert!(!fixture.source.types.contains_key(&projected_ty));
    let derived = BTreeMap::from([(projected_ty, projected.clone())]);
    let proof = Proof::reconstruct_with_types(&[&fixture.source], &derived, &control).unwrap();
    assert_eq!(
        proof.prepared_category(projected_ty).unwrap(),
        ValueCategory::ReadView
    );
    assert!(!proof.prepared_metadata(projected_ty).unwrap());
    assert!(!proof.prepared_transferable(projected_ty).unwrap());
    // An abstract witness still carries no concrete ReadShareable permission.
    assert!(!proof.prepared_read_shareable(projected_ty).unwrap());
    assert!(proof.prepared_layout(projected_ty).is_err());
    assert!(proof.admits_function_flow(fixture.caller));
    {
        let mut resolver = proof.resolver_with_control(&proof.types, &control);
        let context = resolver.property_context_for(projected_ty).unwrap();
        assert_eq!(context.owner, Some(fixture.caller));
        assert_eq!(
            context.binder,
            Some(ReadScopeBinder::Declaration(fixture.caller))
        );
        assert!(context.structural_properties);
        assert_eq!(context.lexical, BTreeSet::from([fixture.binding]));
        resolver.facts(projected_ty, &context, 0).unwrap();
        // Querying the identical prepared digest under another physical owner
        // must not inherit the healthy query's witness or lexical authority.
        let wrong_owner = Context::declaration(fixture.helper);
        assert!(
            resolver
                .facts(projected_ty, &wrong_owner, 0)
                .unwrap_err()
                .message
                .contains("associated projection imports another witness binder")
        );
        let outside_loan = Context::declaration(fixture.caller);
        assert!(
            resolver
                .facts(projected_ty, &outside_loan, 0)
                .unwrap_err()
                .message
                .contains("lexical scope is outside its defining callable and loan body")
        );
    }
    for fault in [
        "witness-scope",
        "witness-parameter",
        "lexical-callable",
        "loan-binding",
        "member",
        "abstract-member",
    ] {
        let mut form = projected.form.clone();
        let expected = match fault {
            "witness-scope" => {
                let TypeForm::AssociatedReadViewProjection { scope, .. } = &mut form else {
                    unreachable!()
                };
                *scope = fixture.helper;
                "associated projection imports another witness binder"
            }
            "witness-parameter" => {
                let TypeForm::AssociatedReadViewProjection { parameter, .. } = &mut form else {
                    unreachable!()
                };
                *parameter =
                    crate::platform::semantic_id::ImplementationParameterId::migrate(SEED, 1401);
                "associated projection changes exact witness/member authority"
            }
            "lexical-callable" => {
                let TypeForm::AssociatedReadViewProjection { read_scope, .. } = &mut form else {
                    unreachable!()
                };
                *read_scope = ReadScopeArgument::Lexical {
                    callable: fixture.helper,
                    binding: fixture.binding,
                };
                "nominal instance has an unproved lexical callable/loan binding"
            }
            "loan-binding" => {
                let TypeForm::AssociatedReadViewProjection { read_scope, .. } = &mut form else {
                    unreachable!()
                };
                *read_scope = ReadScopeArgument::Lexical {
                    callable: fixture.caller,
                    binding: BindingId::migrate(SEED, 1401),
                };
                "nominal instance has an unproved lexical callable/loan binding"
            }
            "member" => {
                let TypeForm::AssociatedReadViewProjection { member, .. } = &mut form else {
                    unreachable!()
                };
                *member = AssociatedTypeMemberId::migrate(SEED, 1401);
                "associated member absent from exact contract"
            }
            "abstract-member" => {
                form = TypeForm::AssociatedReadViewMember {
                    contract,
                    member,
                    read_scope: lexical,
                };
                "abstract view occurrence escapes its exact contract method"
            }
            _ => unreachable!(),
        };
        let object = TypeObject::new(form).unwrap();
        object.validate_local().unwrap();
        let ty = encode_type_object(&object).unwrap().0;
        let hostile = BTreeMap::from([(ty, object)]);
        let error =
            Proof::reconstruct_with_types(&[&fixture.source], &hostile, &control).unwrap_err();
        assert!(
            error.message.contains(expected),
            "{fault}: {}",
            error.message
        );
    }
    let mut missing_witness = fixture.source.clone();
    call_function_mut(&mut missing_witness, fixture.caller)
        .implementation_parameters
        .clear();
    call_assert_local(&missing_witness);
    assert!(
        Proof::reconstruct_with_types(&[&missing_witness], &derived, &control)
            .unwrap_err()
            .message
            .contains("associated projection changes exact witness/member authority")
    );
    assert!(Proof::reconstruct_with_types(&[&fixture.source], &derived, &control).is_ok());
}
