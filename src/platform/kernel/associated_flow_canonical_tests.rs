//! Canonical records and literal decisions, independently adapted to the oracle.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
use super::associated_flow_canonical_reference::{
    CanonicalEntry, CanonicalProgram, DefinitionIdentity,
};
use super::associated_flow_reference as model;
use super::*;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ExpressionId, ImplementationParameterId, MethodId,
    ModuleId, ParameterId, TypeParameterId,
};
use std::collections::BTreeSet;

const SEED: &[u8] = b"canonical-independent-associated-flow";

#[path = "associated_flow_canonical_family_tests.rs"]
mod families;

pub(super) struct Fixture {
    pub source: KernelSnapshot,
    pub entry: CanonicalEntry,
    pub contract: DeclarationReference,
    pub scheme: DeclarationReference,
    pub function: DeclarationReference,
    pub member: AssociatedTypeMemberId,
    pub spare: AssociatedTypeMemberId,
    pub witness: ImplementationParameterId,
    pub formal: TypeObjectDigest,
    pub projection: TypeObjectDigest,
    pub cell: TypeObjectDigest,
    pub recursive_call: ExpressionId,
}

fn name(value: &str) -> Name {
    Name::new(value).unwrap()
}
fn reference(source: &KernelSnapshot, ordinal: u64) -> DeclarationReference {
    DeclarationReference {
        package: source.root.package_id,
        declaration: DeclarationId::migrate(SEED, ordinal),
    }
}
fn ty(source: &mut KernelSnapshot, form: TypeForm) -> TypeObjectDigest {
    let object = TypeObject::new(form).unwrap();
    let digest = encode_type_object(&object).unwrap().0;
    source.types.insert(digest, object);
    digest
}
fn declaration(
    source: &mut KernelSnapshot,
    ordinal: u64,
    payload: DeclarationPayload,
) -> DeclarationReference {
    let reference = reference(source, ordinal);
    let kind = match &payload {
        DeclarationPayload::OwnedContract(_) => OwnerKind::OwnedContract,
        DeclarationPayload::OwnedImplementation(_) => OwnerKind::OwnedImplementation,
        DeclarationPayload::Function(_) => OwnerKind::PureFunction,
        _ => panic!("fixture payload"),
    };
    source.owners.insert(
        OwnerKey::Declaration(reference.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(reference.declaration), kind),
            module: ModuleId::migrate(SEED, 0),
            name: name(&format!("d{ordinal}")),
            visibility: DeclarationVisibility::Public,
            payload,
        }),
    );
    reference
}
fn formal(
    source: &mut KernelSnapshot,
    owner: DeclarationReference,
    ordinal: u64,
) -> (TypeParameterId, TypeObjectDigest) {
    let id = TypeParameterId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::TypeParameter(id),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(id), OwnerKind::TypeParameter),
            declaration: owner.declaration,
            name: name(&format!("T{ordinal}")),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    (id, ty(source, TypeForm::TypeParameter { parameter: id }))
}
fn expression(
    source: &mut KernelSnapshot,
    ordinal: u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    let id = ExpressionId::migrate(SEED, ordinal);
    source.owners.insert(
        OwnerKey::Expression(id),
        OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
    );
    id
}
fn scheme_operand(
    scheme: DeclarationReference,
    arguments: Vec<TypeObjectDigest>,
) -> ImplementationOperand {
    ImplementationOperand::Concrete {
        implementation: scheme,
        type_arguments: arguments,
        implementations: vec![],
    }
}

/// Canonically legal witness Self always has its exact ordinary Owned formal.
/// X/Y are additional scheme formals, not a zero-arity witness-only function.
pub(super) fn recurrence(expanding: bool, reset: bool, permutation: bool) -> Fixture {
    recurrence_in(PackageId::migrate(SEED, 0), expanding, reset, permutation)
}

fn recurrence_in(package: PackageId, expanding: bool, reset: bool, permutation: bool) -> Fixture {
    let mut source = super::tests::transport_snapshot();
    source.root.package_id = package;
    source.owners.clear();
    source.types.clear();
    source.dependency_types.clear();
    source.dependencies.clear();
    source.representation_dependencies.clear();
    source.dependency_interfaces.clear();
    let module = ModuleId::migrate(SEED, 0);
    source.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: name("canonical-flow"),
        }),
    );
    let unit = ty(&mut source, TypeForm::Unit);
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let buffer = ty(&mut source, TypeForm::ByteBuffer);
    let contract = reference(&source, 1);
    let scheme = reference(&source, 2);
    let ping = reference(&source, 3);
    let function = reference(&source, 4);
    let member = AssociatedTypeMemberId::migrate(SEED, 0);
    let spare = AssociatedTypeMemberId::migrate(SEED, 1);
    let method = MethodId::migrate(SEED, 0);
    let witness = ImplementationParameterId::migrate(SEED, 0);
    let (contract_self, contract_self_ty) = formal(&mut source, contract, 10);
    let (scheme_self, scheme_self_ty) = formal(&mut source, scheme, 20);
    let (scheme_x, scheme_x_ty) = formal(&mut source, scheme, 21);
    let (scheme_y, scheme_y_ty) = formal(&mut source, scheme, 22);
    let (ping_self, ping_self_ty) = formal(&mut source, ping, 30);
    let (function_self, function_self_ty) = formal(&mut source, function, 40);
    declaration(
        &mut source,
        1,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter: contract_self,
            type_parameters: vec![],
            methods: vec![OwnedMethod {
                id: method,
                name: name("ping"),
                parameters: vec![OwnedMethodParameter {
                    ty: contract_self_ty,
                    use_mode: ParameterUse::Borrow,
                }],
                result: unit,
                result_borrow: None,
                effect: FunctionEffect::Pure,
            }],
            associated_types: vec![
                AssociatedTypeMember {
                    id: member,
                    name: name("Out"),
                    bound: AssociatedTypeBound::Owned,
                },
                AssociatedTypeMember {
                    id: spare,
                    name: name("Spare"),
                    bound: AssociatedTypeBound::Owned,
                },
            ],
        }),
    );
    let ping_parameter = ParameterId::migrate(SEED, 0);
    source.owners.insert(
        OwnerKey::Parameter(ping_parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(ping_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(ping.declaration),
            name: name("owner"),
            ty: ping_self_ty,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let ping_body = expression(&mut source, 0, ExpressionOperation::Unit {});
    declaration(
        &mut source,
        3,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![ping_self],
            implementation_parameters: vec![],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            parameters: vec![ping_parameter],
            result: unit,
            result_borrow: None,
            effect: FunctionEffect::Pure,
            body: ping_body,
        }),
    );
    let binding = if reset {
        cell
    } else if expanding {
        ty(&mut source, TypeForm::OwnedSequence { item: scheme_x_ty })
    } else {
        scheme_x_ty
    };
    declaration(
        &mut source,
        2,
        DeclarationPayload::OwnedImplementation(OwnedImplementation {
            type_parameters: vec![scheme_self, scheme_x, scheme_y],
            implementation_parameters: vec![],
            contract,
            self_type: scheme_self_ty,
            type_arguments: vec![],
            methods: vec![OwnedMethodImplementation {
                method,
                function: ping,
                type_arguments: vec![scheme_self_ty],
                implementations: vec![],
            }],
            associated_types: vec![
                AssociatedTypeBinding {
                    member,
                    ty: binding,
                },
                AssociatedTypeBinding {
                    member: spare,
                    ty: scheme_y_ty,
                },
            ],
        }),
    );
    let projection = ty(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: function,
            parameter: witness,
            contract,
            member,
        },
    );
    let other_projection = ty(
        &mut source,
        TypeForm::AssociatedWitnessProjection {
            scope: function,
            parameter: witness,
            contract,
            member: spare,
        },
    );
    let actual = if reset {
        ty(&mut source, TypeForm::OwnedSequence { item: projection })
    } else {
        projection
    };
    let arguments = if permutation {
        vec![function_self_ty, other_projection, projection]
    } else {
        vec![function_self_ty, actual, other_projection]
    };
    let recursive_call = expression(
        &mut source,
        1,
        ExpressionOperation::ImplementationCall {
            function,
            type_arguments: vec![function_self_ty],
            effect_arguments: vec![],
            requirement_arguments: vec![],
            implementations: vec![scheme_operand(scheme, arguments)],
            arguments: vec![],
        },
    );
    declaration(
        &mut source,
        4,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![function_self],
            implementation_parameters: vec![ImplementationParameter {
                id: witness,
                name: name("w"),
                contract,
                self_type: function_self_ty,
                type_arguments: vec![],
            }],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            parameters: vec![],
            result: unit,
            result_borrow: None,
            effect: FunctionEffect::Pure,
            body: recursive_call,
        }),
    );
    Fixture {
        entry: CanonicalEntry {
            function,
            arguments: vec![cell],
            implementations: vec![scheme_operand(scheme, vec![cell, cell, buffer])],
        },
        source,
        contract,
        scheme,
        function,
        member,
        spare,
        witness,
        formal: function_self_ty,
        projection,
        cell,
        recursive_call,
    }
}

fn adapted(fixture: &Fixture) -> CanonicalProgram {
    CanonicalProgram::read(&[&fixture.source], std::slice::from_ref(&fixture.entry)).unwrap()
}

#[test]
fn canonical_phantom_witness_growth_and_finite_alias_permutation_reset_have_literal_oracles() {
    for (expanding, reset, permutation, finite, closure_count) in [
        (true, false, false, false, None),
        (false, false, false, true, Some(1)),
        (false, false, true, true, Some(2)),
        (false, true, false, true, Some(2)),
    ] {
        let fixture = recurrence(expanding, reset, permutation);
        let adapted = adapted(&fixture);
        let proof = model::reference_classify(&adapted.program, model::Limits::default()).unwrap();
        assert_eq!(
            proof.finite, finite,
            "growth={expanding}, reset={reset}, permutation={permutation}"
        );
        assert_eq!(proof.positive_diagonal.is_some(), !finite);
        let function = adapted.definition(fixture.function).unwrap();
        assert_eq!(
            adapted.program.functions[&function].parameters, 1,
            "legal lexical Self needs its exact ordinary formal"
        );
        assert!(adapted.definitions.values().any(|identity| matches!(identity, DefinitionIdentity::MethodMapping(scheme, _) if *scheme == fixture.scheme)), "unused mapped methods retain distinct synthetic callable nodes");
        if let Some(count) = closure_count {
            assert_eq!(
                adapted
                    .program
                    .concrete_closure(model::Limits::default())
                    .unwrap()
                    .len(),
                count,
                "closure is fully exhausted, not a bounded prefix"
            );
        } else {
            assert_eq!(
                adapted
                    .program
                    .concrete_closure(model::Limits {
                        concrete_states: 8,
                        ..model::Limits::default()
                    })
                    .unwrap_err(),
                model::Refusal::Capacity("concrete states")
            );
        }
    }
}

#[test]
fn canonical_adapter_admits_all_unused_bindings_and_refuses_exact_authority_mutants() {
    let control = recurrence(false, false, false);
    adapted(&control);
    for mutation in 0..5 {
        let mut fixture = recurrence(false, false, false);
        let expected = match mutation {
            0 => {
                let OwnerRecord::Declaration(record) = fixture
                    .source
                    .owners
                    .get_mut(&OwnerKey::Declaration(fixture.scheme.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                let DeclarationPayload::OwnedImplementation(implementation) = &mut record.payload
                else {
                    unreachable!()
                };
                implementation.associated_types[1].ty = fixture.projection;
                model::Refusal::ProjectionInBinding
            }
            1 => {
                let wrong = ty(
                    &mut fixture.source,
                    TypeForm::AssociatedWitnessProjection {
                        scope: fixture.scheme,
                        parameter: fixture.witness,
                        contract: fixture.contract,
                        member: fixture.member,
                    },
                );
                let OwnerRecord::Expression(record) = fixture
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(fixture.recursive_call))
                    .unwrap()
                else {
                    unreachable!()
                };
                let ExpressionOperation::ImplementationCall {
                    implementations, ..
                } = &mut record.operation
                else {
                    unreachable!()
                };
                let ImplementationOperand::Concrete { type_arguments, .. } =
                    &mut implementations[0]
                else {
                    unreachable!()
                };
                type_arguments[1] = wrong;
                model::Refusal::Scope
            }
            2 => {
                let OwnerRecord::Expression(record) = fixture
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(fixture.recursive_call))
                    .unwrap()
                else {
                    unreachable!()
                };
                let ExpressionOperation::ImplementationCall {
                    implementations, ..
                } = &mut record.operation
                else {
                    unreachable!()
                };
                let ImplementationOperand::Concrete { type_arguments, .. } =
                    &mut implementations[0]
                else {
                    unreachable!()
                };
                type_arguments.pop();
                model::Refusal::Arity
            }
            3 => {
                let wrong = ty(
                    &mut fixture.source,
                    TypeForm::AssociatedWitnessProjection {
                        scope: fixture.function,
                        parameter: fixture.witness,
                        contract: fixture.contract,
                        member: AssociatedTypeMemberId::migrate(SEED, 999),
                    },
                );
                let OwnerRecord::Expression(record) = fixture
                    .source
                    .owners
                    .get_mut(&OwnerKey::Expression(fixture.recursive_call))
                    .unwrap()
                else {
                    unreachable!()
                };
                let ExpressionOperation::ImplementationCall {
                    implementations, ..
                } = &mut record.operation
                else {
                    unreachable!()
                };
                let ImplementationOperand::Concrete { type_arguments, .. } =
                    &mut implementations[0]
                else {
                    unreachable!()
                };
                type_arguments[1] = wrong;
                model::Refusal::ForeignMember
            }
            _ => {
                let OwnerRecord::Declaration(record) = fixture
                    .source
                    .owners
                    .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                let DeclarationPayload::Function(function) = &mut record.payload else {
                    unreachable!()
                };
                function.type_parameters.clear();
                model::Refusal::Scope
            }
        };
        assert_eq!(
            CanonicalProgram::read(&[&fixture.source], std::slice::from_ref(&fixture.entry))
                .err()
                .unwrap(),
            expected,
            "mutation={mutation}"
        );
    }
    for generation in [32, 34] {
        let mut fixture = recurrence(false, false, false);
        let OwnerRecord::Expression(record) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::Expression(fixture.recursive_call))
            .unwrap()
        else {
            unreachable!()
        };
        record.contract_version = generation;
        assert_eq!(
            CanonicalProgram::read(&[&fixture.source], std::slice::from_ref(&fixture.entry))
                .err()
                .unwrap(),
            model::Refusal::Scope,
            "projection consumer generation={generation}"
        );
    }
}

#[test]
fn canonical_adapter_keeps_untaken_syntax_and_unused_function_value_actuals() {
    let mut fixture = recurrence(true, false, false);
    let condition = expression(
        &mut fixture.source,
        100,
        ExpressionOperation::Bool { value: true },
    );
    let plain = expression(&mut fixture.source, 101, ExpressionOperation::Unit {});
    let conditional = expression(
        &mut fixture.source,
        102,
        ExpressionOperation::If {
            condition,
            when_true: plain,
            when_false: fixture.recursive_call,
        },
    );
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function.body = conditional;
    let untaken = adapted(&fixture);
    assert!(
        !model::reference_classify(&untaken.program, model::Limits::default())
            .unwrap()
            .finite
    );
    assert!(
        untaken.program.functions[&untaken.definition(fixture.function).unwrap()]
            .calls
            .iter()
            .any(|call| call.untaken)
    );

    // A discarded function value still admits its phantom type actual. Its
    // exact target has no lexical witnesses and one ordinary Owned formal.
    let target = reference(&fixture.source, 50);
    let (parameter, _) = formal(&mut fixture.source, target, 500);
    let unit = ty(&mut fixture.source, TypeForm::Unit);
    let body = expression(&mut fixture.source, 103, ExpressionOperation::Unit {});
    declaration(
        &mut fixture.source,
        50,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![parameter],
            implementation_parameters: vec![],
            effect_parameters: vec![],
            requirement_parameters: vec![],
            parameters: vec![],
            result: unit,
            result_borrow: None,
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    let value = expression(
        &mut fixture.source,
        104,
        ExpressionOperation::FunctionValue {
            function: target,
            type_arguments: vec![fixture.projection],
            effect_arguments: vec![],
            requirement_arguments: vec![],
        },
    );
    let sequence = expression(
        &mut fixture.source,
        105,
        ExpressionOperation::Sequence {
            items: vec![value, conditional],
        },
    );
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        unreachable!()
    };
    function.body = sequence;
    let adapted = adapted(&fixture);
    assert!(
        adapted.program.functions[&adapted.definition(fixture.function).unwrap()]
            .calls
            .iter()
            .any(|call| call.kind == model::CallKind::FunctionValue
                && matches!(call.arguments.first(), Some(model::Term::Projection { .. })))
    );
}

#[test]
fn canonical_adapter_preserves_constructor_names_and_exact_scoped_reverse_identities() {
    let mut fixture = recurrence(false, false, false);
    let named_a = ty(
        &mut fixture.source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("a"),
                ty: fixture.cell,
            }],
        },
    );
    let named_b = ty(
        &mut fixture.source,
        TypeForm::OwnedProduct {
            fields: vec![StructuralTypeField {
                name: name("b"),
                ty: fixture.cell,
            }],
        },
    );
    let OwnerRecord::Expression(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.recursive_call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut record.operation
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete { type_arguments, .. } = &mut implementations[0] else {
        unreachable!()
    };
    type_arguments[1] = named_a;
    type_arguments[2] = named_b;
    let adapted = adapted(&fixture);
    let atom_names: BTreeSet<_> = adapted
        .atoms
        .values()
        .filter(|atom| atom.kind == "owned_product")
        .map(|atom| atom.fields.clone())
        .collect();
    assert_eq!(
        atom_names,
        BTreeSet::from([vec!["a".to_owned()], vec!["b".to_owned()]])
    );
    assert_eq!(
        adapted.definitions.values().collect::<BTreeSet<_>>().len(),
        adapted.definitions.len()
    );
    assert_eq!(
        adapted.members.values().collect::<BTreeSet<_>>().len(),
        adapted.members.len()
    );
    assert_eq!(adapted.witness_parameters.len(), 1);
}

#[test]
fn canonical_adapter_names_identical_raw_ids_in_distinct_package_scopes_injectively() {
    let first = recurrence_in(PackageId::migrate(SEED, 10), false, false, false);
    let second = recurrence_in(PackageId::migrate(SEED, 11), false, false, false);
    assert_eq!(first.function.declaration, second.function.declaration);
    assert_eq!(first.member, second.member);
    assert_eq!(first.witness, second.witness);
    let adapted = CanonicalProgram::read(
        &[&first.source, &second.source],
        &[first.entry.clone(), second.entry.clone()],
    )
    .unwrap();
    assert_ne!(
        adapted.definition(first.function).unwrap(),
        adapted.definition(second.function).unwrap()
    );
    assert_eq!(adapted.members.values().collect::<BTreeSet<_>>().len(), 4);
    assert!(
        model::reference_classify(&adapted.program, model::Limits::default())
            .unwrap()
            .finite
    );
    assert_eq!(
        adapted
            .program
            .concrete_closure(model::Limits::default())
            .unwrap()
            .len(),
        2
    );
}

fn extra_binding_atoms(fixture: &mut Fixture, count: usize) {
    let mut members = Vec::new();
    let mut bindings = Vec::new();
    for ordinal in 0..count {
        let member = AssociatedTypeMemberId::migrate(SEED, 1000 + ordinal as u64);
        let value = ty(
            &mut fixture.source,
            TypeForm::OwnedProduct {
                fields: vec![StructuralTypeField {
                    name: name(&format!("field{ordinal}")),
                    ty: fixture.cell,
                }],
            },
        );
        members.push(AssociatedTypeMember {
            id: member,
            name: name(&format!("Member{ordinal}")),
            bound: AssociatedTypeBound::Owned,
        });
        bindings.push(AssociatedTypeBinding { member, ty: value });
    }
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
        unreachable!()
    };
    contract.associated_types.extend(members);
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.scheme.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(scheme) = &mut record.payload else {
        unreachable!()
    };
    scheme.associated_types.extend(bindings);
}

#[test]
fn canonical_adapter_constructor_atoms_above_legacy_capacity_remain_injective() {
    let control = recurrence(false, false, false);
    let baseline = adapted(&control).atoms.len();
    let available = usize::from(u8::MAX) + 2 - baseline;
    let mut exact = recurrence(false, false, false);
    extra_binding_atoms(&mut exact, available);
    let admitted = adapted(&exact);
    assert_eq!(admitted.atoms.len(), usize::from(u8::MAX) + 2);
    assert_eq!(
        admitted.atoms.values().collect::<BTreeSet<_>>().len(),
        admitted.atoms.len()
    );
    assert!(admitted.atoms.keys().any(|id| *id > usize::from(u8::MAX)));
    let mut larger = recurrence(false, false, false);
    extra_binding_atoms(&mut larger, available + 1);
    assert_eq!(adapted(&larger).atoms.len(), admitted.atoms.len() + 1);
    assert!(matches!(
        control.source.types[&control.formal].form,
        TypeForm::TypeParameter { .. }
    ));
    let OwnerRecord::Declaration(record) =
        &control.source.owners[&OwnerKey::Declaration(control.scheme.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(scheme) = &record.payload else {
        unreachable!()
    };
    assert!(
        scheme
            .associated_types
            .iter()
            .any(|binding| binding.member == control.spare)
    );
}

#[test]
fn canonical_contract_signature_members_remain_abstract_and_scope_checked_when_unused() {
    use super::associated_flow_canonical_reference::SignatureTerm;
    let mut fixture = recurrence(false, false, false);
    let member_type = ty(
        &mut fixture.source,
        TypeForm::AssociatedContractMember {
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
        unreachable!()
    };
    contract.methods[0].parameters[0].ty = member_type;
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.scheme.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedImplementation(scheme) = &mut record.payload else {
        unreachable!()
    };
    scheme.methods[0].type_arguments[0] = scheme.associated_types[0].ty;
    let control = adapted(&fixture);
    assert_eq!(
        control.signature_members,
        BTreeSet::from([(fixture.contract, fixture.member)])
    );
    assert_eq!(
        control.signature_types[&(fixture.contract, member_type)],
        SignatureTerm::Member(fixture.contract, fixture.member)
    );
    assert!(
        model::reference_classify(&control.program, model::Limits::default())
            .unwrap()
            .finite
    );
    for generation in [32, 34] {
        let mut source = fixture.source.clone();
        let OwnerRecord::Declaration(record) = source
            .owners
            .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        record.header.contract_version = generation;
        assert_eq!(
            CanonicalProgram::read(&[&source], std::slice::from_ref(&fixture.entry))
                .err()
                .unwrap(),
            model::Refusal::Scope,
            "contract-member consumer generation={generation}"
        );
    }
    let OwnerRecord::Declaration(record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.contract.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::OwnedContract(contract) = &mut record.payload else {
        unreachable!()
    };
    contract.methods[0].parameters[0].ty = fixture.projection;
    assert_eq!(
        CanonicalProgram::read(&[&fixture.source], std::slice::from_ref(&fixture.entry))
            .err()
            .unwrap(),
        model::Refusal::Scope
    );
}

#[test]
fn canonical_associated_growth_agrees_with_eager_and_demanded_production_decisions() {
    use crate::platform::diagnostic::DiagnosticClass;
    for (expanding, reset, permutation, expected_finite) in [
        (true, false, false, false),
        (false, false, false, true),
        (false, false, true, true),
        (false, true, false, true),
    ] {
        let fixture = recurrence(expanding, reset, permutation);
        let adapted = adapted(&fixture);
        let independent =
            model::reference_classify(&adapted.program, model::Limits::default()).unwrap();
        assert_eq!(independent.finite, expected_finite);
        for eager in [false, true] {
            let mut work = 0;
            let result = if eager {
                callable_flow::validate_snapshot_callable_closure_eager(
                    &[&fixture.source],
                    &|| Ok(()),
                    &mut work,
                    contract::MAXIMUM_VALIDATION_WORK,
                )
            } else {
                callable_flow::validate_snapshot_callable_closure(
                    &[&fixture.source],
                    &|| Ok(()),
                    &mut work,
                    contract::MAXIMUM_VALIDATION_WORK,
                )
            };
            match (expected_finite, result) {
                (true, Ok(_)) => {}
                (false, Err(diagnostic)) => {
                    assert_eq!(diagnostic.class, DiagnosticClass::Semantic);
                    assert_eq!(diagnostic.code, "kernel_callable_expansion");
                }
                (_, unexpected) => panic!(
                    "eager={eager}, growth={expanding}, reset={reset}, permutation={permutation}: {unexpected:?}"
                ),
            }
            // Producer slot/edge counts deliberately are not the model's counts.
            assert!(work > 0);
        }
    }
}

#[test]
fn canonical_associated_flow_capacity_cancellation_and_healthy_retry_keep_distinct_classes() {
    use crate::platform::diagnostic::{Diagnostic, DiagnosticClass};
    use std::cell::Cell;
    let fixture = recurrence(false, false, true);
    assert!(
        model::reference_classify(&adapted(&fixture).program, model::Limits::default())
            .unwrap()
            .finite
    );
    for eager in [false, true] {
        let checkpoints = Cell::new(0usize);
        let count = || {
            checkpoints.set(checkpoints.get() + 1);
            Ok(())
        };
        let mut work = 0;
        let expected = callable_flow::validate_snapshot_callable_closure_with_limits(
            &[&fixture.source],
            &count,
            &mut work,
            contract::MAXIMUM_VALIDATION_WORK,
            64 * 1_048_576,
            eager,
        )
        .unwrap();
        let complete_work = work;
        let complete_checkpoints = checkpoints.get();
        assert!(complete_work > 1 && expected.metadata_bytes > 1 && complete_checkpoints > 1);
        for (maximum_work, maximum_metadata_bytes) in [
            (complete_work - 1, expected.metadata_bytes),
            (complete_work, expected.metadata_bytes - 1),
        ] {
            let failure = callable_flow::validate_snapshot_callable_closure_with_limits(
                &[&fixture.source],
                &|| Ok(()),
                &mut 0,
                maximum_work,
                maximum_metadata_bytes,
                eager,
            )
            .unwrap_err();
            assert_eq!(failure.class, DiagnosticClass::Resource);
        }
        checkpoints.set(0);
        let cancel = || {
            let next = checkpoints.get() + 1;
            checkpoints.set(next);
            if next == complete_checkpoints - 1 {
                Err(Diagnostic::new(
                    DiagnosticClass::Cancelled,
                    "canonical_flow_cancelled",
                    "late owning flow cancellation",
                ))
            } else {
                Ok(())
            }
        };
        let failure = callable_flow::validate_snapshot_callable_closure_with_limits(
            &[&fixture.source],
            &cancel,
            &mut 0,
            complete_work,
            expected.metadata_bytes,
            eager,
        )
        .unwrap_err();
        assert_eq!(failure.class, DiagnosticClass::Cancelled);
        assert_eq!(failure.code, "canonical_flow_cancelled");
        let mut retry_work = 0;
        let retry = callable_flow::validate_snapshot_callable_closure_with_limits(
            &[&fixture.source],
            &|| Ok(()),
            &mut retry_work,
            complete_work,
            expected.metadata_bytes,
            eager,
        )
        .unwrap();
        assert_eq!(retry_work, complete_work);
        assert_eq!(
            (
                retry.functions,
                retry.applications,
                retry.slots,
                retry.edges,
                retry.metadata_bytes
            ),
            (
                expected.functions,
                expected.applications,
                expected.slots,
                expected.edges,
                expected.metadata_bytes
            )
        );
    }
}
