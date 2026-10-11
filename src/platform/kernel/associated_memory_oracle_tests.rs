//! Canonical-input witnesses for the independent associated memory resolver.
//! Agreement fixtures supply only raw canonical records to each checker.
use super::associated::{Context, Environment, Failure, Limits, Occurrence, Resolver};
use super::*;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ImplementationParameterId, MethodId, TypeParameterId,
};

#[path = "associated_memory_closure_oracle_tests.rs"]
mod closure_tests;

const SEED: &[u8] = b"independent-associated-memory";
struct Fixture {
    source: KernelSnapshot,
    contract: DeclarationReference,
    scheme: DeclarationReference,
    other_scheme: DeclarationReference,
    source_scope: DeclarationReference,
    caller_scope: DeclarationReference,
    source_parameter: TypeParameterId,
    source_witness: ImplementationParameterId,
    caller_witness: ImplementationParameterId,
    scheme_parameter: TypeParameterId,
    member: AssociatedTypeMemberId,
    spare: AssociatedTypeMemberId,
    source_projection: TypeObjectDigest,
    caller_projection: TypeObjectDigest,
    caller_parameter_type: TypeObjectDigest,
    scheme_parameter_type: TypeObjectDigest,
    cell: TypeObjectDigest,
    buffer: TypeObjectDigest,
    scalar: TypeObjectDigest,
}
fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn intern(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}
fn insert_declaration(
    source: &mut KernelSnapshot,
    n: u64,
    kind: OwnerKind,
    payload: DeclarationPayload,
) -> DeclarationReference {
    let id = DeclarationId::migrate(SEED, n);
    let module = source
        .owners
        .values()
        .find_map(|r| match r {
            OwnerRecord::Module(m) => match m.header.owner {
                OwnerKey::Module(id) => Some(id),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
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
fn insert_parameter(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
    n: u64,
) -> (TypeParameterId, TypeObjectDigest) {
    let id = TypeParameterId::migrate(SEED, n);
    source.owners.insert(
        OwnerKey::TypeParameter(id),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
            declaration: scope.declaration,
            name: name(&format!("T{n}")),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    (
        id,
        intern(source, TypeForm::TypeParameter { parameter: id }),
    )
}
fn fixture() -> Fixture {
    let mut source = super::super::tests::transport_snapshot();
    let body = source
        .owners
        .values()
        .find_map(|r| match r {
            OwnerRecord::Expression(e) => Some(e.id),
            _ => None,
        })
        .unwrap();
    let cell = intern(&mut source, TypeForm::OwnedI64Cell);
    let buffer = intern(&mut source, TypeForm::ByteBuffer);
    let scalar = intern(&mut source, TypeForm::I64);
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let spare = AssociatedTypeMemberId::migrate(SEED, 1);
    let self_parameter = TypeParameterId::migrate(SEED, 0);
    let method = MethodId::migrate(SEED, 0);
    let contract = insert_declaration(
        &mut source,
        0,
        OwnerKind::OwnedContract,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter,
            type_parameters: vec![],
            associated_types: vec![
                AssociatedTypeMember {
                    id: member,
                    name: name("Out"),
                    bound: AssociatedTypeBound::Owned,
                },
                AssociatedTypeMember {
                    id: spare,
                    name: name("Unused"),
                    bound: AssociatedTypeBound::Owned,
                },
            ],
            methods: vec![OwnedMethod {
                id: method,
                name: name("inspect"),
                parameters: vec![],
                result: scalar,
                result_borrow: None,
                effect: FunctionEffect::Pure,
            }],
        }),
    );
    insert_parameter(&mut source, contract, 0);
    let observer = insert_declaration(
        &mut source,
        1,
        OwnerKind::PureFunction,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![],
            implementation_parameters: vec![],
            effect_parameters: vec![],
            requirement_parameters: vec![],
            parameters: vec![],
            result: scalar,
            result_borrow: None,
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    let scheme_parameter = TypeParameterId::migrate(SEED, 1);
    let scheme_parameter_type = intern(
        &mut source,
        TypeForm::TypeParameter {
            parameter: scheme_parameter,
        },
    );
    let mut bindings = vec![
        AssociatedTypeBinding {
            member,
            ty: scheme_parameter_type,
        },
        AssociatedTypeBinding {
            member: spare,
            ty: cell,
        },
    ];
    bindings.sort_by_key(|b| b.member);
    let scheme = insert_declaration(
        &mut source,
        2,
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![scheme_parameter],
            implementation_parameters: vec![],
            contract,
            self_type: cell,
            type_arguments: vec![],
            associated_types: bindings,
            methods: vec![OwnedMethodImplementation {
                method,
                function: observer,
                type_arguments: vec![],
                implementations: vec![],
            }],
        }),
    );
    insert_parameter(&mut source, scheme, 1);
    let other_parameter = TypeParameterId::migrate(SEED, 2);
    let mut other_bindings = vec![
        AssociatedTypeBinding { member, ty: buffer },
        AssociatedTypeBinding {
            member: spare,
            ty: cell,
        },
    ];
    other_bindings.sort_by_key(|b| b.member);
    let other_scheme = insert_declaration(
        &mut source,
        3,
        OwnerKind::OwnedImplementation,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![other_parameter],
            implementation_parameters: vec![],
            contract,
            self_type: cell,
            type_arguments: vec![],
            associated_types: other_bindings,
            methods: vec![OwnedMethodImplementation {
                method,
                function: observer,
                type_arguments: vec![],
                implementations: vec![],
            }],
        }),
    );
    insert_parameter(&mut source, other_scheme, 2);
    let mut scopes = Vec::new();
    for (ordinal, p_ordinal, w_ordinal) in [(4, 3, 0), (5, 4, 1)] {
        let p = TypeParameterId::migrate(SEED, p_ordinal);
        let self_type = intern(&mut source, TypeForm::TypeParameter { parameter: p });
        let witness = ImplementationParameterId::migrate(SEED, w_ordinal);
        let scope = insert_declaration(
            &mut source,
            ordinal,
            OwnerKind::PureFunction,
            DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![p],
                implementation_parameters: vec![ImplementationParameter {
                    id: witness,
                    name: name("reader"),
                    contract,
                    self_type,
                    type_arguments: vec![],
                }],
                effect_parameters: vec![],
                requirement_parameters: vec![],
                parameters: vec![],
                result: scalar,
                result_borrow: None,
                effect: FunctionEffect::Pure,
                body,
            }),
        );
        insert_parameter(&mut source, scope, p_ordinal);
        scopes.push((scope, p, self_type, witness));
    }
    let (source_scope, source_parameter, _, source_witness) = scopes[0];
    let (caller_scope, _, caller_parameter_type, caller_witness) = scopes[1];
    let source_projection = intern(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: source_scope,
            parameter: source_witness,
            contract,
            member,
        },
    );
    let caller_projection = intern(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: caller_scope,
            parameter: caller_witness,
            contract,
            member,
        },
    );
    Fixture {
        source,
        contract,
        scheme,
        other_scheme,
        source_scope,
        caller_scope,
        source_parameter,
        source_witness,
        caller_witness,
        scheme_parameter,
        member,
        spare,
        source_projection,
        caller_projection,
        caller_parameter_type,
        scheme_parameter_type,
        cell,
        buffer,
        scalar,
    }
}
fn occurrence(scope: DeclarationReference) -> Occurrence {
    Occurrence {
        parameter_scope: Some(scope),
        context: Context::Lexical(scope),
        consumer: (scope.package, OwnerKey::Declaration(scope.declaration)),
    }
}
fn forwarded(f: &Fixture) -> Environment {
    Environment {
        ordinary: [(f.source_parameter, f.caller_parameter_type)]
            .into_iter()
            .collect(),
        source: Context::Lexical(f.source_scope),
        caller: occurrence(f.caller_scope),
        contract_operand: None,
        witnesses: [(
            f.source_witness,
            ImplementationOperand::Parameter {
                scope: f.caller_scope,
                parameter: f.caller_witness,
            },
        )]
        .into_iter()
        .collect(),
    }
}
fn selected(f: &Fixture, scheme: DeclarationReference, actual: TypeObjectDigest) -> Environment {
    Environment {
        ordinary: [(f.source_parameter, f.cell)].into_iter().collect(),
        source: Context::Lexical(f.source_scope),
        caller: occurrence(f.caller_scope),
        contract_operand: None,
        witnesses: [(
            f.source_witness,
            ImplementationOperand::Concrete {
                implementation: scheme,
                type_arguments: vec![actual],
                implementations: vec![],
            },
        )]
        .into_iter()
        .collect(),
    }
}
fn implementation_mut(
    source: &mut KernelSnapshot,
    scope: DeclarationReference,
) -> &mut OwnedImplementation {
    let OwnerRecord::Declaration(r) = source
        .owners
        .get_mut(&OwnerKey::Declaration(scope.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(i) = &mut r.payload else {
        unreachable!()
    };
    i
}

#[test]
fn associated_memory_reference_resolves_forwarded_empty_and_inserted_caller_actuals() {
    let mut f = fixture();
    let mut resolver = Resolver::new(&f.source);
    let mut env = forwarded(&f);
    resolver.admit_application(&env).unwrap();
    assert_eq!(
        resolver.substitute(f.source_projection, &env, 0),
        Ok(f.caller_projection)
    );
    env.ordinary.clear();
    assert_eq!(
        resolver.substitute(f.source_projection, &env, 0),
        Ok(f.caller_projection)
    );
    assert_eq!(resolver.admit_application(&env), Err(Failure::Arity));
    let template = intern(
        &mut f.source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("item"),
                ty: f.scheme_parameter_type,
            }],
        },
    );
    implementation_mut(&mut f.source, f.scheme)
        .associated_types
        .iter_mut()
        .find(|b| b.member == f.member)
        .unwrap()
        .ty = template;
    let expected = intern(
        &mut f.source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("item"),
                ty: f.caller_projection,
            }],
        },
    );
    let outer = intern(
        &mut f.source,
        TypeForm::OwnedSequence {
            item: f.source_projection,
        },
    );
    let mut resolver = Resolver::new(&f.source);
    let env = selected(&f, f.scheme, f.caller_projection);
    resolver.admit_application(&env).unwrap();
    assert_eq!(
        resolver.substitute(f.source_projection, &env, 0),
        Ok(expected)
    );
    let result = resolver.substitute(outer, &env, 0).unwrap();
    assert_eq!(
        resolver.derived[&result].form,
        TypeForm::OwnedSequence { item: expected }
    );
    assert!(resolver.remaining() < contract::MAXIMUM_VALIDATION_WORK);
}

#[test]
fn associated_memory_reference_retains_exact_witnesses_and_owned_only_category() {
    let f = fixture();
    let mut resolver = Resolver::new(&f.source);
    for schemes in [[f.scheme, f.other_scheme], [f.other_scheme, f.scheme]] {
        for scheme in schemes {
            let env = selected(&f, scheme, f.cell);
            resolver.admit_application(&env).unwrap();
            assert_eq!(
                resolver.substitute(f.source_projection, &env, 0),
                Ok(if scheme == f.scheme { f.cell } else { f.buffer })
            );
        }
    }
    assert!(
        resolver
            .owned_type(f.caller_projection, occurrence(f.caller_scope), 0)
            .unwrap()
    );
    resolver
        .constraints(
            f.caller_projection,
            TypeParameterConstraints::Owned,
            occurrence(f.caller_scope),
            0,
        )
        .unwrap();
    for bounds in [
        TypeParameterConstraints::OwnedTransferable,
        TypeParameterConstraints::OwnedShareable,
        TypeParameterConstraints::CaptureSafeTransferable,
    ] {
        assert_eq!(
            resolver.constraints(f.caller_projection, bounds, occurrence(f.caller_scope), 0),
            Err(Failure::Category)
        );
    }
    resolver
        .constraints(
            f.cell,
            TypeParameterConstraints::OwnedTransferableShareable,
            occurrence(f.caller_scope),
            0,
        )
        .unwrap();
}

#[test]
fn associated_memory_reference_rejects_occurrence_and_unused_binding_mutants() {
    let f = fixture();
    let env = forwarded(&f);
    let mut wrong = env.clone();
    wrong.source = Context::Lexical(f.caller_scope);
    assert_eq!(
        Resolver::new(&f.source).substitute(f.source_projection, &wrong, 0),
        Err(Failure::Arity)
    );
    let mut old = f.source.clone();
    let OwnerRecord::Declaration(r) = old
        .owners
        .get_mut(&OwnerKey::Declaration(f.caller_scope.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    r.header.contract_version = 32;
    assert_eq!(
        Resolver::new(&old).substitute(f.source_projection, &env, 0),
        Err(Failure::Generation)
    );
    let mut absent = f.source.clone();
    implementation_mut(&mut absent, f.scheme)
        .associated_types
        .retain(|b| b.member != f.spare);
    assert_eq!(
        Resolver::new(&absent).admit_bindings(f.scheme),
        Err(Failure::Inventory)
    );
    let mut duplicate = f.source.clone();
    let i = implementation_mut(&mut duplicate, f.scheme);
    i.associated_types.push(i.associated_types[0].clone());
    assert_eq!(
        Resolver::new(&duplicate).admit_bindings(f.scheme),
        Err(Failure::Inventory)
    );
    let mut hidden = f.source.clone();
    implementation_mut(&mut hidden, f.scheme)
        .associated_types
        .iter_mut()
        .find(|b| b.member == f.spare)
        .unwrap()
        .ty = f.caller_projection;
    assert_eq!(
        Resolver::new(&hidden).admit_bindings(f.scheme),
        Err(Failure::BindingProjection)
    );
    let env = selected(&f, f.other_scheme, f.scalar);
    assert_eq!(
        Resolver::new(&f.source).substitute(f.source_projection, &env, 0),
        Err(Failure::Category)
    );
    let mut foreign = f.source.clone();
    let OwnerRecord::TypeParameter(p) = foreign
        .owners
        .get_mut(&OwnerKey::TypeParameter(f.scheme_parameter))
        .unwrap()
    else {
        unreachable!()
    };
    p.declaration = f.caller_scope.declaration;
    assert_eq!(
        Resolver::new(&foreign).admit_bindings(f.scheme),
        Err(Failure::Authority)
    );
}

#[test]
fn associated_memory_reference_checks_unused_prerequisites_and_capacity_recovery() {
    let mut f = fixture();
    let prerequisite = ImplementationParameterId::migrate(SEED, 2);
    implementation_mut(&mut f.source, f.scheme)
        .implementation_parameters
        .push(ImplementationParameter {
            id: prerequisite,
            name: name("unused"),
            contract: f.contract,
            self_type: f.scheme_parameter_type,
            type_arguments: vec![],
        });
    let mut env = selected(&f, f.scheme, f.cell);
    let ImplementationOperand::Concrete {
        implementations, ..
    } = env.witnesses.get_mut(&f.source_witness).unwrap()
    else {
        unreachable!()
    };
    implementations.push(ImplementationOperand::Concrete {
        implementation: f.other_scheme,
        type_arguments: vec![f.cell],
        implementations: vec![],
    });
    Resolver::new(&f.source).admit_application(&env).unwrap();
    let mut broken = env.clone();
    let ImplementationOperand::Concrete {
        implementations, ..
    } = broken.witnesses.get_mut(&f.source_witness).unwrap()
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete { type_arguments, .. } = &mut implementations[0] else {
        unreachable!()
    };
    type_arguments[0] = f.scalar;
    assert_eq!(
        Resolver::new(&f.source).substitute(f.source_projection, &broken, 0),
        Err(Failure::Category)
    );
    assert_eq!(
        Resolver::with_limits(
            &f.source,
            Limits {
                work: 1,
                objects: 1
            }
        )
        .substitute(f.source_projection, &env, 0),
        Err(Failure::Capacity)
    );
    assert_eq!(
        Resolver::new(&f.source).substitute(f.source_projection, &env, 0),
        Ok(f.cell)
    );
}

#[test]
fn associated_memory_reference_agrees_on_projected_calls_and_exact_borrowed_source() {
    use super::super::memory::associated_tests::{FlowKind, check_caller, fixture as flow_fixture};
    for kind in [FlowKind::Direct, FlowKind::Witness, FlowKind::Method] {
        let mut f = flow_fixture(kind);
        check_caller(&f).unwrap();
        assert!(accepts(&f.source));
        let oracle = Oracle(&f.source, Some(f.caller));
        let invocation = oracle.associated_invocation(f.call).unwrap();
        assert_eq!(invocation.result_borrow, Some(0));
        assert_eq!(invocation.parameters[0].1, invocation.result);
        // Both parameters have the same projected type, but changing the
        // promised result root still fails the independent Rights proof.
        let OwnerRecord::Declaration(record) = f
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(f.caller.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(function) = &mut record.payload else {
            unreachable!()
        };
        function.result_borrow = Some(f.second);
        assert!(check_caller(&f).is_err());
        assert!(!accepts(&f.source));
    }
}

#[test]
fn associated_memory_reference_agrees_on_child_generations_and_absent_transport_rights() {
    use super::super::memory::associated_tests::{FlowKind, check_caller, fixture as flow_fixture};
    for kind in [FlowKind::Direct, FlowKind::Witness, FlowKind::Method] {
        for generation in [32, 34] {
            for target in 0..6 {
                let mut f = flow_fixture(kind);
                let key = match target {
                    0 => OwnerKey::Declaration(f.caller.declaration),
                    1 => OwnerKey::Parameter(f.first),
                    2 => OwnerKey::Binding(f.binding),
                    3 => OwnerKey::Expression(f.local),
                    4 => OwnerKey::Expression(f.call),
                    _ => OwnerKey::Expression(f.borrow),
                };
                f.source
                    .owners
                    .get_mut(&key)
                    .unwrap()
                    .set_encoding_for_edit(generation);
                assert!(check_caller(&f).is_err());
                assert!(
                    !accepts(&f.source),
                    "consumer {target} at generation {generation} acquired authority from its parent"
                );
            }
        }
        let mut future_root = flow_fixture(kind);
        future_root.source.root.graph_contract_version = 34;
        assert_eq!(
            Resolver::new(&future_root.source).admit_canonical_closure(),
            Err(Failure::Generation)
        );
    }
    for bounds in [
        TypeParameterConstraints::OwnedTransferable,
        TypeParameterConstraints::OwnedShareable,
    ] {
        let mut f = flow_fixture(FlowKind::Direct);
        let formal = f
            .source
            .owners
            .values_mut()
            .find_map(|owner| match owner {
                OwnerRecord::TypeParameter(p) if p.declaration == f.callee.declaration => Some(p),
                _ => None,
            })
            .unwrap();
        formal.constraints = bounds;
        assert!(check_caller(&f).is_err());
        assert!(!accepts(&f.source));
    }
}
