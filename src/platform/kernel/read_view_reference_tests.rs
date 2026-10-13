//! Raw canonical controls for scopes/categories; no production proof is input.
use super::*;
use crate::platform::semantic_id::{
    DeclarationId, ExpressionId, FieldId, ModuleId, ReadScopeParameterId, TypeParameterId,
};

const SEED: &[u8] = b"read-view-independent-reference";
fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}
fn scope_parameter(n: u64) -> ReadScopeParameter {
    ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, n),
        name: name(&format!("R{n}")),
    }
}
fn scope(declaration: DeclarationReference, n: u64) -> ReadScopeArgument {
    ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(declaration),
        parameter: scope_parameter(n).id,
    })
}
fn insert(
    source: &mut KernelSnapshot,
    n: u64,
    kind: OwnerKind,
    payload: DeclarationPayload,
) -> DeclarationReference {
    let id = DeclarationId::migrate(SEED, n);
    let module = source
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Module(m) => match m.header.owner {
                OwnerKey::Module(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| ModuleId::migrate(SEED, 0));
    source.owners.insert(
        OwnerKey::Declaration(id),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(id), kind),
            module,
            name: name(&format!("D{n}")),
            visibility: DeclarationVisibility::Public,
            payload,
        }),
    );
    DeclarationReference {
        package: source.root.package_id,
        declaration: id,
    }
}
fn function(source: &mut KernelSnapshot, n: u64, result: TypeObjectDigest) -> DeclarationReference {
    let body = ExpressionId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(ExpressionRecord::new(body, ExpressionOperation::Unit {}).unwrap()),
    );
    insert(
        source,
        n,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            implementation_parameters: vec![],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            type_parameters: vec![],
            parameters: vec![],
            result: CallableResult {
                ty: result,
                relation: ResultRelation::Value,
            },
            read_scope_parameters: vec![scope_parameter(0), scope_parameter(1)],
            read_scope_sources: vec![],
            effect: FunctionEffect::Pure,
            body,
        }),
    )
}
fn view(source: &mut KernelSnapshot, n: u64, fields: Vec<ReadViewField>) -> DeclarationReference {
    insert(
        source,
        n,
        OwnerKind::ReadView,
        DeclarationPayload::ReadView(ReadViewDeclaration {
            type_parameters: vec![],
            scope_parameter: scope_parameter(0),
            fields,
        }),
    )
}
fn occurrence(declaration: DeclarationReference) -> Occurrence {
    Occurrence {
        parameter_scope: Some(declaration),
        context: Context::Lexical(declaration),
        consumer: (
            declaration.package,
            OwnerKey::Declaration(declaration.declaration),
        ),
    }
}
fn fixture() -> (
    KernelSnapshot,
    DeclarationReference,
    DeclarationReference,
    DeclarationReference,
    TypeObjectDigest,
) {
    let mut source = crate::platform::kernel::tests::transport_snapshot();
    let unit = intern(&mut source, TypeForm::Unit);
    let caller = function(&mut source, 100, unit);
    let callee = function(&mut source, 101, unit);
    let view = view(&mut source, 102, vec![]);
    let ty = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![],
            read_scope: scope(callee, 0),
        },
    );
    (source, caller, callee, view, ty)
}
fn environment(caller: DeclarationReference, callee: DeclarationReference) -> ReadViewEnvironment {
    let binder = ReadScopeBinder::Declaration(callee);
    ReadViewEnvironment {
        types: Environment {
            ordinary: BTreeMap::new(),
            source: Context::Lexical(callee),
            caller: occurrence(caller),
            contract_operand: None,
            witnesses: BTreeMap::new(),
        },
        source_binder: binder,
        caller_binder: ReadScopeBinder::Declaration(caller),
        scopes: [
            (
                ReadScopeReference {
                    binder,
                    parameter: scope_parameter(0).id,
                },
                scope(caller, 0),
            ),
            (
                ReadScopeReference {
                    binder,
                    parameter: scope_parameter(1).id,
                },
                scope(caller, 1),
            ),
        ]
        .into_iter()
        .collect(),
    }
}

#[test]
fn view_reference_positive_category_retained_graph33_owned_and_exact34_view() {
    let (mut source, _caller, callee, _view, ty) = fixture();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let mut resolver = Resolver::new(&source);
    assert_eq!(
        resolver.read_view_category(ty, occurrence(callee)),
        Ok(ReadCategory::ReadView)
    );
    assert_eq!(resolver.owned_type(ty, occurrence(callee), 0), Ok(false));
    assert!(!Oracle(&source, Some(callee)).ordinary(ty));
    assert!(!Oracle(&source, Some(callee)).capture_safe(ty));
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(callee.declaration))
        .unwrap()
    else {
        panic!()
    };
    d.header.contract_version = 33;
    let mut resolver = Resolver::new(&source);
    assert!(resolver.current(occurrence(callee)).is_ok());
    assert_eq!(resolver.owned_type(cell, occurrence(callee), 0), Ok(true));
    assert_eq!(
        resolver.read_view_category(ty, occurrence(callee)),
        Err(Failure::Generation)
    );
}

#[test]
fn view_reference_complete_unused_scope_maps_precede_cache_and_bind_owner() {
    let (source, caller, callee, _view, ty) = fixture();
    let mut resolver = Resolver::new(&source);
    let good = environment(caller, callee);
    assert_eq!(
        source.types[&ty].contract_version,
        contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION
    );
    let output = resolver.substitute_view(ty, &good, 0).unwrap();
    assert_eq!(resolver.read_view_scope(output), Ok(Some(scope(caller, 0))));
    assert_eq!(
        resolver.object(output).unwrap().contract_version,
        contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION
    );
    let mut missing = good.clone();
    missing.scopes.remove(&ReadScopeReference {
        binder: good.source_binder,
        parameter: scope_parameter(1).id,
    });
    assert_eq!(
        resolver.substitute_view(ty, &missing, 0),
        Err(Failure::Arity)
    );
    let mut wrong = good.clone();
    wrong.scopes.insert(
        ReadScopeReference {
            binder: good.source_binder,
            parameter: scope_parameter(1).id,
        },
        scope(callee, 1),
    );
    assert_eq!(
        resolver.substitute_view(ty, &wrong, 0),
        Err(Failure::Authority)
    );
    assert_eq!(resolver.substitute_view(ty, &good, 0), Ok(output));
    // Correct object profile does not upgrade either actual owner or the root.
    for owner in [caller, callee] {
        let mut old = source.clone();
        let OwnerRecord::Declaration(record) = old
            .owners
            .get_mut(&OwnerKey::Declaration(owner.declaration))
            .unwrap()
        else {
            panic!()
        };
        record.header.contract_version = 33;
        assert_eq!(
            Resolver::new(&old).substitute_view(ty, &good, 0),
            Err(Failure::Generation)
        );
    }
    let mut old_root = source.clone();
    old_root.root.graph_contract_version = 33;
    assert_eq!(
        Resolver::new(&old_root).substitute_view(ty, &good, 0),
        Err(Failure::Generation)
    );
    // A Graph generation cannot be substituted for the type-object profile.
    let mut wrong_profile = source.clone();
    wrong_profile.types.get_mut(&ty).unwrap().contract_version =
        contract::READ_VIEW_GRAPH_CONTRACT_VERSION;
    assert_eq!(
        Resolver::new(&wrong_profile).substitute_view(ty, &good, 0),
        Err(Failure::Generation)
    );
}

#[test]
fn view_reference_source_alias_scope_does_not_merge_argument_or_sibling_paths() {
    let (mut source, caller, _, _, _) = fixture();
    let first = crate::platform::semantic_id::ParameterId::migrate(SEED, 1);
    let second = crate::platform::semantic_id::ParameterId::migrate(SEED, 2);
    let a = ReadProvenance {
        anchor: SourceAnchor {
            callable: caller,
            source: LocalValueReference::FunctionParameter(first),
        },
        path: vec![view_scopes::ReadPathStep::OwnedProductField(name("left"))],
    };
    let mut b = a.clone();
    b.anchor.source = LocalValueReference::FunctionParameter(second);
    assert!(!a.derives_from(&b));
    let mut sibling = a.clone();
    sibling.path[0] = view_scopes::ReadPathStep::OwnedProductField(name("right"));
    assert!(!a.derives_from(&sibling));
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let mut resolver = Resolver::new(&source);
    let child = a
        .extend(&mut resolver, view_scopes::ReadPathStep::OwnedItem(cell))
        .unwrap();
    assert!(child.derives_from(&a));
    assert!(!a.derives_from(&child));
    let mut relations = ScopeRelations::default();
    assert!(
        !relations
            .contains(&mut resolver, &scope(caller, 0), &scope(caller, 1))
            .unwrap()
    );
    relations
        .declare(&mut resolver, scope(caller, 0), scope(caller, 1))
        .unwrap();
    assert!(
        relations
            .contains(&mut resolver, &scope(caller, 0), &scope(caller, 1))
            .unwrap()
    );
    assert!(
        !relations
            .contains(&mut resolver, &scope(caller, 1), &scope(caller, 0))
            .unwrap()
    );
}

#[test]
fn view_reference_empty_metadata_share_does_not_grant_self_or_phantom_share() {
    let (mut source, caller, _, metadata, _) = fixture();
    let parameter = TypeParameterId::migrate(SEED, 10);
    source.owners.insert(
        OwnerKey::TypeParameter(parameter),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(parameter), OwnerKind::TypeParameter),
            declaration: caller.declaration,
            name: name("T"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        panic!()
    };
    f.type_parameters.push(parameter);
    let actual = intern(&mut source, TypeForm::TypeParameter { parameter });
    let metadata_ty = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: metadata,
            arguments: vec![],
            read_scope: scope(caller, 0),
        },
    );
    let generic = view(&mut source, 103, vec![]);
    let formal = TypeParameterId::migrate(SEED, 11);
    source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: generic.declaration,
            name: name("Self"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(generic.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::ReadView(v) = &mut d.payload else {
        panic!()
    };
    v.type_parameters.push(formal);
    let phantom = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: generic,
            arguments: vec![actual],
            read_scope: scope(caller, 0),
        },
    );
    assert_eq!(
        Resolver::new(&source).read_shareable_view(metadata_ty, occurrence(caller)),
        Ok(true)
    );
    assert_eq!(
        Resolver::new(&source).read_shareable_view(phantom, occurrence(caller)),
        Ok(false)
    );
    let referent = intern(&mut source, TypeForm::TypeParameter { parameter: formal });
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(generic.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::ReadView(v) = &mut d.payload else {
        panic!()
    };
    v.fields.push(ReadViewField {
        id: FieldId::migrate(SEED, 0),
        name: name("source"),
        ty: referent,
        kind: ReadViewFieldKind::ReadReference {
            read_scope: scope(generic, 0),
        },
    });
    assert_eq!(
        Resolver::new(&source).read_shareable_view(phantom, occurrence(caller)),
        Ok(false)
    );
}

#[test]
fn view_reference_all_fields_and_cumulative_capacity_precede_retention() {
    let (mut source, caller, callee, view, ty) = fixture();
    let mut baseline = Resolver::with_limits(
        &source,
        Limits {
            work: usize::MAX,
            objects: usize::MAX,
        },
    );
    baseline
        .substitute_view(ty, &environment(caller, callee), 0)
        .unwrap();
    let work = usize::MAX - baseline.remaining();
    let entries = baseline.retained_entries();
    let mut exact = Resolver::with_limits(
        &source,
        Limits {
            work,
            objects: entries,
        },
    );
    assert!(
        exact
            .substitute_view(ty, &environment(caller, callee), 0)
            .is_ok()
    );
    let mut short = Resolver::with_limits(
        &source,
        Limits {
            work: work - 1,
            objects: entries,
        },
    );
    assert_eq!(
        short.substitute_view(ty, &environment(caller, callee), 0),
        Err(Failure::Capacity)
    );
    let mut short = Resolver::with_limits(
        &source,
        Limits {
            work,
            objects: entries - 1,
        },
    );
    assert_eq!(
        short.substitute_view(ty, &environment(caller, callee), 0),
        Err(Failure::Capacity)
    );
    let bad = intern(&mut source, TypeForm::OwnedI64Cell);
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(view.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::ReadView(v) = &mut d.payload else {
        panic!()
    };
    v.fields.push(ReadViewField {
        id: FieldId::migrate(SEED, 99),
        name: name("unused"),
        ty: bad,
        kind: ReadViewFieldKind::Ordinary,
    });
    assert_eq!(
        Resolver::new(&source).admit_view_layout(view),
        Err(Failure::Category)
    );
}

#[test]
fn view_reference_exact_family_witness_and_unused_binding_kind_are_complete() {
    use crate::platform::semantic_id::{
        AssociatedTypeMemberId, ImplementationParameterId, MethodId,
    };
    let (mut source, caller, callee, first_view, _) = fixture();
    let unit = intern(&mut source, TypeForm::Unit);
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let second_view = view(&mut source, 110, vec![]);
    let method = MethodId::migrate(SEED, 0);
    let family = AssociatedTypeMemberId::migrate(SEED, 0);
    let unused = AssociatedTypeMemberId::migrate(SEED, 1);
    let self_id = TypeParameterId::migrate(SEED, 20);
    let contract = insert(
        &mut source,
        111,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: self_id,
            type_parameters: vec![],
            associated_types: vec![
                AssociatedTypeMember {
                    id: family,
                    name: name("Block"),
                    bound: AssociatedTypeBound::ReadViewFamily {
                        scope_parameter: scope_parameter(2),
                    },
                },
                AssociatedTypeMember {
                    id: unused,
                    name: name("Unused"),
                    bound: AssociatedTypeBound::Owned,
                },
            ],
            methods: vec![OwnedMethod {
                id: method,
                name: name("unused-method"),
                parameters: vec![],
                result: CallableResult {
                    ty: unit,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    source.owners.insert(
        OwnerKey::TypeParameter(self_id),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(self_id), OwnerKind::TypeParameter),
            declaration: contract.declaration,
            name: name("Self"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let target = function(&mut source, 112, unit);
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(target.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        panic!()
    };
    f.read_scope_parameters.clear();
    let mut schemes = Vec::new();
    for (n, selected) in [(113, first_view), (114, second_view)] {
        let scheme = insert(
            &mut source,
            n,
            OwnerKind::OwnedImplementation,
            DeclarationPayload::OwnedImplementation(OwnedImplementation {
                type_parameters: vec![],
                implementation_parameters: vec![],
                contract,
                self_type: cell,
                type_arguments: vec![],
                associated_types: vec![],
                methods: vec![OwnedMethodImplementation {
                    method,
                    function: target,
                    type_arguments: vec![],
                    implementations: vec![],
                    read_scope_arguments: vec![],
                }],
            }),
        );
        let binder = ReadScopeBinder::AssociatedBinding {
            implementation: scheme,
            member: family,
        };
        let ty = intern(
            &mut source,
            TypeForm::ReadViewApplied {
                declaration: selected,
                arguments: vec![],
                read_scope: ReadScopeArgument::Parameter(ReadScopeReference {
                    binder,
                    parameter: scope_parameter(2).id,
                }),
            },
        );
        let mut bindings = vec![
            AssociatedTypeBinding {
                member: family,
                value: AssociatedTypeBindingValue::ReadViewFamily {
                    scope_parameter: scope_parameter(2),
                    ty,
                },
            },
            AssociatedTypeBinding {
                member: unused,
                value: AssociatedTypeBindingValue::Owned { ty: cell },
            },
        ];
        bindings.sort_by_key(|b| b.member);
        let OwnerRecord::Declaration(d) = source
            .owners
            .get_mut(&OwnerKey::Declaration(scheme.declaration))
            .unwrap()
        else {
            panic!()
        };
        let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
            panic!()
        };
        i.associated_types = bindings;
        schemes.push(scheme);
    }
    let formal = TypeParameterId::migrate(SEED, 21);
    source.owners.insert(
        OwnerKey::TypeParameter(formal),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(formal), OwnerKind::TypeParameter),
            declaration: callee.declaration,
            name: name("T"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let self_type = intern(&mut source, TypeForm::TypeParameter { parameter: formal });
    let witness = ImplementationParameterId::migrate(SEED, 0);
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(callee.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        panic!()
    };
    f.type_parameters.push(formal);
    f.implementation_parameters.push(ImplementationParameter {
        id: witness,
        name: name("Reader"),
        contract,
        self_type,
        type_arguments: vec![],
    });
    let projection = intern(
        &mut source,
        TypeForm::AssociatedReadViewProjection {
            scope: callee,
            parameter: witness,
            contract,
            member: family,
            read_scope: scope(callee, 0),
        },
    );
    let mut outputs = Vec::new();
    for scheme in &schemes {
        let mut env = environment(caller, callee);
        env.types.ordinary.insert(formal, cell);
        env.types.witnesses.insert(
            witness,
            ImplementationOperand::Concrete {
                implementation: *scheme,
                type_arguments: vec![],
                implementations: vec![],
            },
        );
        let mut resolver = Resolver::new(&source);
        let output = resolver.substitute_view(projection, &env, 0).unwrap();
        assert_eq!(resolver.read_view_scope(output), Ok(Some(scope(caller, 0))));
        outputs.push(output);
    }
    assert_ne!(outputs[0], outputs[1]);
    let wrong_kind = intern(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: callee,
            parameter: witness,
            contract,
            member: family,
        },
    );
    assert_eq!(
        Resolver::new(&source).admit_roots(&[wrong_kind], occurrence(callee), false),
        Err(Failure::Category)
    );
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(schemes[0].declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut d.payload else {
        panic!()
    };
    i.associated_types
        .iter_mut()
        .find(|b| b.member == unused)
        .unwrap()
        .value = AssociatedTypeBindingValue::Owned { ty: unit };
    assert_eq!(
        Resolver::new(&source).admit_bindings(schemes[0]),
        Err(Failure::Category)
    );
}

#[test]
fn view_reference_lexical_authority_uses_function34_and_native_owned_loan_generations() {
    use crate::platform::semantic_id::{BindingId, ParameterId};
    let (mut source, caller, _, _, _) = fixture();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let product = intern(
        &mut source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("item"),
                ty: cell,
            }],
        },
    );
    let parameter = ParameterId::migrate(SEED, 70);
    source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader {
                contract_version: 33,
                ..OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter)
            },
            parent: ParameterParent::Function(caller.declaration),
            name: name("source"),
            ty: product,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let binding = BindingId::migrate(SEED, 70);
    source.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader {
                contract_version: 24,
                ..OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding)
            },
            kind: BindingKind::OwnedBorrow,
            name: name("child"),
            declared_type: Some(cell),
            value: None,
        }),
    );
    let local = ExpressionId::migrate(SEED, 71);
    source.owners.insert(
        OwnerKey::Expression(local),
        OwnerRecord::Expression(ExpressionRecord {
            contract_version: 24,
            id: local,
            operation: ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(parameter),
            },
        }),
    );
    let unit = ExpressionId::migrate(SEED, 72);
    source.owners.insert(
        OwnerKey::Expression(unit),
        OwnerRecord::Expression(ExpressionRecord {
            contract_version: 24,
            id: unit,
            operation: ExpressionOperation::Unit {},
        }),
    );
    let borrow = ExpressionId::migrate(SEED, 73);
    source.owners.insert(
        OwnerKey::Expression(borrow),
        OwnerRecord::Expression(ExpressionRecord {
            contract_version: 24,
            id: borrow,
            operation: ExpressionOperation::BorrowOwnedField {
                product_type: product,
                source: local,
                field: name("item"),
                binding,
                body: unit,
            },
        }),
    );
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(f) = &mut d.payload else {
        panic!()
    };
    f.parameters.push(parameter);
    f.body = borrow;
    let actual = ReadScopeArgument::Lexical {
        callable: caller,
        binding,
    };
    let binder = ReadScopeBinder::Declaration(caller);
    assert_eq!(
        Resolver::new(&source).admit_scope_argument(&actual, &binder, occurrence(caller)),
        Ok(())
    );
    let OwnerRecord::Expression(e) = source
        .owners
        .get_mut(&OwnerKey::Expression(borrow))
        .unwrap()
    else {
        panic!()
    };
    e.contract_version = 23;
    assert_eq!(
        Resolver::new(&source).admit_scope_argument(&actual, &binder, occurrence(caller)),
        Err(Failure::Generation)
    );
    let OwnerRecord::Expression(e) = source
        .owners
        .get_mut(&OwnerKey::Expression(borrow))
        .unwrap()
    else {
        panic!()
    };
    e.contract_version = 24;
    let OwnerRecord::Declaration(d) = source
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!()
    };
    d.header.contract_version = 33;
    assert_eq!(
        Resolver::new(&source).admit_scope_argument(&actual, &binder, occurrence(caller)),
        Err(Failure::Generation)
    );
}

fn unused_family_nominal_fixture() -> (
    KernelSnapshot,
    DeclarationReference,
    crate::platform::semantic_id::AssociatedTypeMemberId,
    TypeObjectDigest,
) {
    use crate::platform::semantic_id::{AssociatedTypeMemberId, MethodId};
    let (mut source, _, _, _, _) = fixture();
    let unit = intern(&mut source, TypeForm::Unit);
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let descriptor = view(&mut source, 200, vec![]);
    let family = AssociatedTypeMemberId::migrate(SEED, 200);
    let method = MethodId::migrate(SEED, 200);
    let self_id = TypeParameterId::migrate(SEED, 200);
    let contract = insert(
        &mut source,
        201,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: self_id,
            type_parameters: vec![],
            associated_types: vec![AssociatedTypeMember {
                id: family,
                name: name("UnusedView"),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: scope_parameter(2),
                },
            }],
            methods: vec![OwnedMethod {
                id: method,
                name: name("observe"),
                parameters: vec![],
                result: CallableResult {
                    ty: unit,
                    relation: ResultRelation::Value,
                },
                read_scope_parameters: vec![],
                read_scope_sources: vec![],
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    source.owners.insert(
        OwnerKey::TypeParameter(self_id),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(self_id), OwnerKind::TypeParameter),
            declaration: contract.declaration,
            name: name("UnusedSelf"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let target = function(&mut source, 202, unit);
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(target.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!()
    };
    function.read_scope_parameters.clear();
    let implementation = insert(
        &mut source,
        203,
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![],
            implementation_parameters: vec![],
            contract,
            self_type: cell,
            type_arguments: vec![],
            associated_types: vec![],
            methods: vec![OwnedMethodImplementation {
                method,
                function: target,
                type_arguments: vec![],
                implementations: vec![],
                read_scope_arguments: vec![],
            }],
        }),
    );
    let rhs = intern(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: descriptor,
            arguments: vec![],
            read_scope: ReadScopeArgument::Parameter(ReadScopeReference {
                binder: ReadScopeBinder::AssociatedBinding {
                    implementation,
                    member: family,
                },
                parameter: scope_parameter(2).id,
            }),
        },
    );
    unused_family_implementation_mut(&mut source, implementation)
        .associated_types
        .push(AssociatedTypeBinding {
            member: family,
            value: AssociatedTypeBindingValue::ReadViewFamily {
                scope_parameter: scope_parameter(2),
                ty: rhs,
            },
        });
    (source, implementation, family, rhs)
}

fn unused_family_implementation_mut(
    source: &mut KernelSnapshot,
    reference: DeclarationReference,
) -> &mut OwnedImplementation {
    let OwnerRecord::Declaration(record) = source
        .owners
        .get_mut(&OwnerKey::Declaration(reference.declaration))
        .unwrap()
    else {
        panic!()
    };
    let DeclarationPayload::OwnedImplementation(implementation) = &mut record.payload else {
        panic!()
    };
    implementation
}

fn unused_family_nominal_frontier(
    source: &KernelSnapshot,
    reference: DeclarationReference,
) -> Vec<crate::platform::diagnostic::Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut work = 0;
    crate::platform::kernel::infer::validate_expression_roots(
        source,
        [OwnerKey::Declaration(reference.declaration)],
        &mut diagnostics,
        &mut work,
    );
    diagnostics
}

#[test]
fn production_nominal_frontier_admits_unused_view_family_in_its_exact_binding_role() {
    let (source, implementation, _, rhs) = unused_family_nominal_fixture();
    assert_eq!(
        Resolver::new(&source).admit_bindings(implementation),
        Ok(())
    );
    assert_eq!(
        Resolver::new(&source).read_view_category(rhs, occurrence(implementation)),
        Ok(ReadCategory::ReadView)
    );
    assert_eq!(
        unused_family_nominal_frontier(&source, implementation),
        vec![]
    );
}

#[test]
fn production_nominal_frontier_rejects_foreign_family_identity_and_ordinary_role_aliases() {
    use crate::platform::semantic_id::{AssociatedTypeMemberId, ImplementationParameterId};
    for attack in 0..11 {
        let (mut source, implementation, family, rhs) = unused_family_nominal_fixture();
        if attack < 3 || attack == 9 {
            if attack == 0 {
                unused_family_implementation_mut(&mut source, implementation).associated_types[0]
                    .member = AssociatedTypeMemberId::migrate(SEED, 999);
            } else {
                let TypeForm::ReadViewApplied {
                    declaration,
                    mut arguments,
                    mut read_scope,
                } = source.types[&rhs].form.clone()
                else {
                    panic!()
                };
                let ReadScopeArgument::Parameter(reference) = &mut read_scope else {
                    panic!()
                };
                if attack == 9 {
                    arguments.push(intern(&mut source, TypeForm::Unit));
                } else if attack == 1 {
                    reference.binder = ReadScopeBinder::AssociatedBinding {
                        implementation,
                        member: AssociatedTypeMemberId::migrate(SEED, 999),
                    };
                } else {
                    reference.parameter = scope_parameter(999).id;
                }
                let bad = intern(
                    &mut source,
                    TypeForm::ReadViewApplied {
                        declaration,
                        arguments,
                        read_scope,
                    },
                );
                let AssociatedTypeBindingValue::ReadViewFamily { ty, .. } =
                    &mut unused_family_implementation_mut(&mut source, implementation)
                        .associated_types[0]
                        .value
                else {
                    panic!()
                };
                *ty = bad;
            }
            assert!(
                Resolver::new(&source)
                    .admit_bindings(implementation)
                    .is_err()
            );
        } else {
            // The exact same digest remains valid in its binding. Every ordinary
            // physical role must independently refuse the positive view category.
            assert_eq!(
                Resolver::new(&source).admit_bindings(implementation),
                Ok(())
            );
            assert_eq!(
                Resolver::new(&source).read_view_category(rhs, occurrence(implementation)),
                Ok(ReadCategory::ReadView)
            );
            let cell = intern(&mut source, TypeForm::OwnedI64Cell);
            let ordinary_rhs = if attack == 10 {
                let formal = TypeParameterId::migrate(SEED, 999);
                let phantom = insert(
                    &mut source,
                    999,
                    OwnerKind::Record,
                    DeclarationPayload::Record {
                        type_parameters: vec![formal],
                        fields: vec![],
                    },
                );
                source.owners.insert(
                    OwnerKey::TypeParameter(formal),
                    OwnerRecord::TypeParameter(TypeParameterRecord {
                        header: OwnerHeader::new(
                            OwnerKey::TypeParameter(formal),
                            OwnerKind::TypeParameter,
                        ),
                        declaration: phantom.declaration,
                        name: name("UnusedPhantom"),
                        constraints: TypeParameterConstraints::default(),
                    }),
                );
                intern(
                    &mut source,
                    TypeForm::Applied {
                        declaration: phantom,
                        arguments: vec![rhs],
                    },
                )
            } else {
                rhs
            };
            let contract = unused_family_implementation_mut(&mut source, implementation).contract;
            let record = unused_family_implementation_mut(&mut source, implementation);
            match attack {
                3 => record.self_type = rhs,
                4 => record.type_arguments.push(rhs),
                5 => record
                    .implementation_parameters
                    .push(ImplementationParameter {
                        id: ImplementationParameterId::migrate(SEED, 999),
                        name: name("UnusedPrerequisite"),
                        contract,
                        self_type: rhs,
                        type_arguments: vec![],
                    }),
                6 => record.methods[0].type_arguments.push(rhs),
                7 => record.methods[0]
                    .implementations
                    .push(ImplementationOperand::Concrete {
                        implementation,
                        type_arguments: vec![],
                        implementations: vec![ImplementationOperand::Concrete {
                            implementation,
                            type_arguments: vec![rhs],
                            implementations: vec![],
                        }],
                    }),
                8 => record
                    .implementation_parameters
                    .push(ImplementationParameter {
                        id: ImplementationParameterId::migrate(SEED, 999),
                        name: name("UnusedPrerequisiteActual"),
                        contract,
                        self_type: cell,
                        type_arguments: vec![rhs],
                    }),
                10 => record.type_arguments.push(ordinary_rhs),
                _ => unreachable!(),
            }
            assert_eq!(record.associated_types[0].member, family);
            assert_eq!(record.associated_types[0].ty(), rhs);
            assert_eq!(
                Resolver::new(&source).admit_bindings(implementation),
                Ok(())
            );
        }
        let diagnostics = unused_family_nominal_frontier(&source, implementation);
        assert!(!diagnostics.is_empty(), "physical role attack {attack}");
        if (3..9).contains(&attack) {
            assert!(
                diagnostics.iter().any(|diagnostic| {
                    diagnostic.message
                        == "lexical scope authority requires an exact named graph function"
                }),
                "ordinary root did not retain its exact context: {attack}: {diagnostics:?}"
            );
        }
        assert!(
            diagnostics.iter().any(|diagnostic| {
                diagnostic.code == "kernel_read_view"
                    || diagnostic.code == "kernel_owned_associated"
            }),
            "physical role attack {attack}: {diagnostics:?}"
        );
    }
}
