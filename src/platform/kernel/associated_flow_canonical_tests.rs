//! Canonical records and literal decisions, independently adapted to the oracle.
#![allow(clippy::expect_used, clippy::panic, clippy::unwrap_used)]
use super::associated_flow_canonical_reference::{
    CanonicalEntry, CanonicalProgram, DefinitionIdentity,
};
use super::associated_flow_reference as model;
use super::*;
use crate::platform::semantic_id::{
    AssociatedTypeMemberId, DeclarationId, ExpressionId, FieldId, ImplementationParameterId,
    MethodId, ModuleId, ParameterId, TypeParameterId,
};
use std::collections::BTreeSet;

const SEED: &[u8] = b"canonical-independent-associated-flow";

#[path = "associated_flow_canonical_family_tests.rs"]
mod families;

fn repeated_method_registry(expanding: bool) -> (Fixture, usize) {
    let mut fixture = recurrence(expanding, false, false);
    let unit = ty(&mut fixture.source, TypeForm::Unit);
    let parameter = ParameterId::migrate(SEED, 60_000);
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(fixture.function.declaration),
            name: name("source"),
            ty: fixture.formal,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let local = expression(
        &mut fixture.source,
        60_000,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let method = {
        let OwnerRecord::Declaration(declaration) =
            &fixture.source.owners[&OwnerKey::Declaration(fixture.contract.declaration)]
        else {
            unreachable!()
        };
        let DeclarationPayload::OwnedContract(contract) = &declaration.payload else {
            unreachable!()
        };
        contract.methods[0].id
    };
    let sites = 128;
    let mut body = Vec::new();
    for ordinal in 0..sites {
        body.push(expression(
            &mut fixture.source,
            60_001 + ordinal as u64,
            ExpressionOperation::MethodCall {
                witness: ImplementationOperand::Parameter {
                    scope: fixture.function,
                    parameter: fixture.witness,
                },
                contract: fixture.contract,
                method,
                arguments: vec![local],

                read_scope_arguments: Vec::new(),
            },
        ));
    }
    let OwnerRecord::Expression(call) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(fixture.recursive_call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall { arguments, .. } = &mut call.operation else {
        unreachable!()
    };
    arguments.push(local);
    body.push(fixture.recursive_call);
    let body = expression(
        &mut fixture.source,
        61_000,
        ExpressionOperation::Sequence { items: body },
    );
    let OwnerRecord::Declaration(function_record) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Declaration(fixture.function.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut function_record.payload else {
        unreachable!()
    };
    function.parameters = vec![parameter];
    function.body = body;
    // Complete raw closure admission still visits every uncalled declaration.
    // They contribute no method alternative, type slot, or recursive dependency.
    for ordinal in 0..512 {
        let body = expression(
            &mut fixture.source,
            50_000 + ordinal,
            ExpressionOperation::Unit {},
        );
        let reference = declaration(
            &mut fixture.source,
            40_000 + ordinal,
            DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![],
                implementation_parameters: vec![],
                requirement_parameters: vec![],
                effect_parameters: vec![],
                parameters: vec![],
                result: crate::platform::kernel::CallableResult {
                    ty: unit,
                    relation: crate::platform::kernel::ResultRelation::Value,
                },
                effect: FunctionEffect::Pure,
                body,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
            }),
        );
        let OwnerRecord::Declaration(record) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::Declaration(reference.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        record.name = name(&format!(
            "unrelated_method_registry_{ordinal:04}_{}",
            "x".repeat(64)
        ));
    }
    (fixture, sites)
}

#[test]
fn canonical_repeated_methods_admit_large_unrelated_registry_without_relaxing_growth() {
    let maximum_bytes = 64 * 1_048_576;
    for expanding in [false, true] {
        let (fixture, sites) = repeated_method_registry(expanding);
        // Literal lower bound for the removed all-declaration scan: every site
        // cloned every declaration before filtering. Even omitting its vectors
        // and every other admission cost already exceeds the unchanged64MiB cap.
        let one_scan: usize = fixture
            .source
            .owners
            .values()
            .filter_map(|owner| match owner {
                OwnerRecord::Declaration(record) => Some(
                    2 * serde_json::to_vec(record).unwrap().len()
                        + std::mem::size_of::<DeclarationRecord>(),
                ),
                _ => None,
            })
            .sum();
        assert!(sites * one_scan > maximum_bytes);
        let mut work = 0;
        let result = associated_flow_canonical_reference::validate(
            &[&fixture.source],
            &crate::platform::execution::ExecutionControl::uncancelled(),
            &mut work,
            contract::MAXIMUM_VALIDATION_WORK,
        );
        if expanding {
            let error = result.expect_err("actual positive associated recurrence must refuse");
            assert_eq!(
                error.class,
                crate::platform::diagnostic::DiagnosticClass::Semantic
            );
            assert_eq!(error.code, "kernel_associated_reference_growth");
        } else {
            let proof = result.expect("irrelevant registry scans must not consume the growth cap");
            assert!(proof.metadata_bytes > 0 && proof.metadata_bytes <= maximum_bytes);
        }
        assert!(work > 0);
    }
}

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
                result: crate::platform::kernel::CallableResult {
                    ty: unit,
                    relation: crate::platform::kernel::ResultRelation::Value,
                },
                effect: FunctionEffect::Pure,

                read_scope_parameters: Vec::new(),
                read_scope_sources: Vec::new(),
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
            result: crate::platform::kernel::CallableResult {
                ty: unit,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: FunctionEffect::Pure,
            body: ping_body,

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
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

                read_scope_arguments: Vec::new(),
            }],
            associated_types: vec![
                AssociatedTypeBinding {
                    member,
                    value: crate::platform::kernel::AssociatedTypeBindingValue::Owned {
                        ty: binding,
                    },
                },
                AssociatedTypeBinding {
                    member: spare,
                    value: crate::platform::kernel::AssociatedTypeBindingValue::Owned {
                        ty: scheme_y_ty,
                    },
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

            read_scope_arguments: Vec::new(),
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
            result: crate::platform::kernel::CallableResult {
                ty: unit,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: FunctionEffect::Pure,
            body: recursive_call,

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
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
                implementation.associated_types[1].value = AssociatedTypeBindingValue::Owned {
                    ty: fixture.projection,
                };
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
    for generation in [32, 35] {
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
            result: crate::platform::kernel::CallableResult {
                ty: unit,
                relation: crate::platform::kernel::ResultRelation::Value,
            },
            effect: FunctionEffect::Pure,
            body,

            read_scope_parameters: Vec::new(),
            read_scope_sources: Vec::new(),
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

            read_scope_arguments: Vec::new(),
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
        bindings.push(AssociatedTypeBinding {
            member,
            value: crate::platform::kernel::AssociatedTypeBindingValue::Owned { ty: value },
        });
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
    scheme.methods[0].type_arguments[0] = scheme.associated_types[0].ty();
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
    for generation in [32, 35] {
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

/// Direct raw-canonical flow reader controls; these records do not claim whole
/// source admission, artifact loading, prepared execution, or runtime leases.
#[test]
fn canonical_view_type_profiles_remain_distinct_from_owner_generations() {
    use crate::platform::semantic_id::ReadScopeParameterId;

    let mut source = super::tests::transport_snapshot();
    source.root.graph_contract_version = contract::READ_VIEW_GRAPH_CONTRACT_VERSION;
    source.owners.clear();
    source.types.clear();
    source.dependency_types.clear();
    source.dependencies.clear();
    source.representation_dependencies.clear();
    source.dependency_interfaces.clear();
    let module = ModuleId::migrate(SEED, 70_000);
    source.owners.insert(
        OwnerKey::Module(module),
        OwnerRecord::Module(ModuleRecord {
            header: OwnerHeader::new(OwnerKey::Module(module), OwnerKind::Module),
            name: name("view-profile-flow"),
        }),
    );
    let scalar = ty(&mut source, TypeForm::I64);
    let cell = ty(&mut source, TypeForm::OwnedI64Cell);
    let view = reference(&source, 70_000);
    let callable = reference(&source, 70_001);
    let contract_ref = reference(&source, 70_002);
    let member = AssociatedTypeMemberId::migrate(SEED, 70_000);
    let method = MethodId::migrate(SEED, 70_000);
    let view_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 70_000),
        name: name("view_scope"),
    };
    let callable_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 70_001),
        name: name("caller_scope"),
    };
    let method_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 70_002),
        name: name("method_scope"),
    };
    let family_parameter = ReadScopeParameter {
        id: ReadScopeParameterId::migrate(SEED, 70_003),
        name: name("family_scope"),
    };
    let caller_scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::Declaration(callable),
        parameter: callable_parameter.id,
    });
    let method_scope = ReadScopeArgument::Parameter(ReadScopeReference {
        binder: ReadScopeBinder::ContractMethod {
            contract: contract_ref,
            method,
        },
        parameter: method_parameter.id,
    });
    let label = FieldId::migrate(SEED, 70_000);
    source.owners.insert(
        OwnerKey::Declaration(view.declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(view.declaration), OwnerKind::ReadView),
            module,
            name: name("MetadataView"),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::ReadView(ReadViewDeclaration {
                type_parameters: vec![],
                scope_parameter: view_parameter,
                fields: vec![ReadViewField {
                    id: label,
                    name: name("label"),
                    ty: scalar,
                    kind: ReadViewFieldKind::Ordinary,
                }],
            }),
        }),
    );
    let view_type = ty(
        &mut source,
        TypeForm::ReadViewApplied {
            declaration: view,
            arguments: vec![],
            read_scope: caller_scope,
        },
    );
    let signature_type = ty(
        &mut source,
        TypeForm::AssociatedReadViewMember {
            contract: contract_ref,
            member,
            read_scope: method_scope,
        },
    );
    let (self_parameter, self_type) = formal(&mut source, contract_ref, 70_000);
    declaration(
        &mut source,
        70_002,
        DeclarationPayload::OwnedContract(OwnedContract {
            self_parameter,
            type_parameters: vec![],
            associated_types: vec![AssociatedTypeMember {
                id: member,
                name: name("Out"),
                bound: AssociatedTypeBound::ReadViewFamily {
                    scope_parameter: family_parameter,
                },
            }],
            methods: vec![OwnedMethod {
                id: method,
                name: name("view"),
                parameters: vec![OwnedMethodParameter {
                    ty: self_type,
                    use_mode: ParameterUse::Borrow,
                }],
                result: CallableResult {
                    ty: signature_type,
                    relation: ResultRelation::ViewFrom {
                        source: 0,
                        scope: method_scope,
                    },
                },
                effect: FunctionEffect::Pure,
                read_scope_parameters: vec![method_parameter],
                read_scope_sources: vec![ReadScopeSource {
                    source: 0,
                    scope: method_scope,
                }],
            }],
        }),
    );
    let source_parameter = ParameterId::migrate(SEED, 70_000);
    source.owners.insert(
        OwnerKey::Parameter(source_parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(source_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(callable.declaration),
            name: name("source"),
            ty: cell,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let value = expression(&mut source, 70_000, ExpressionOperation::I64 { value: 7 });
    let body = expression(
        &mut source,
        70_001,
        ExpressionOperation::CreateReadView {
            declaration: view,
            type_arguments: vec![],
            read_scope: caller_scope,
            source: LocalValueReference::FunctionParameter(source_parameter),
            fields: vec![ReadViewFieldInitializer {
                field: label,
                value,
            }],
        },
    );
    declaration(
        &mut source,
        70_001,
        DeclarationPayload::Function(FunctionDeclaration {
            type_parameters: vec![],
            implementation_parameters: vec![],
            requirement_parameters: vec![],
            effect_parameters: vec![],
            parameters: vec![source_parameter],
            read_scope_parameters: vec![callable_parameter],
            read_scope_sources: vec![ReadScopeSource {
                source: source_parameter,
                scope: caller_scope,
            }],
            result: CallableResult {
                ty: view_type,
                relation: ResultRelation::ViewFrom {
                    source: source_parameter,
                    scope: caller_scope,
                },
            },
            effect: FunctionEffect::Pure,
            body,
        }),
    );
    // The existing declaration helper uses its fixed module coordinate.
    for owner in source.owners.values_mut() {
        if let OwnerRecord::Declaration(record) = owner {
            record.module = module;
        }
    }
    for digest in [view_type, signature_type] {
        assert_eq!(
            source.types[&digest].contract_version,
            contract::ASSOCIATED_READ_VIEW_CONTRACT_VERSION,
        );
    }
    let canonical = CanonicalProgram::read(&[&source], &[]).unwrap();
    let callable_definition = canonical
        .definitions
        .iter()
        .find_map(|(definition, identity)| {
            (identity == &DefinitionIdentity::Declaration(callable)).then_some(*definition)
        })
        .unwrap();
    assert!(
        canonical
            .program
            .functions
            .contains_key(&callable_definition)
    );
    assert_eq!(
        canonical.signature_types[&(contract_ref, signature_type)],
        associated_flow_canonical_reference::SignatureTerm::ViewMember(
            contract_ref,
            member,
            method_scope,
        ),
    );
    for generation in [33, 35] {
        for reference in [callable, contract_ref, view] {
            let mut wrong_owner = source.clone();
            let OwnerRecord::Declaration(record) = wrong_owner
                .owners
                .get_mut(&OwnerKey::Declaration(reference.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            record.header.contract_version = generation;
            assert_eq!(
                CanonicalProgram::read(&[&wrong_owner], &[]).err().unwrap(),
                model::Refusal::Scope,
            );
        }
        let mut wrong_root = source.clone();
        wrong_root.root.graph_contract_version = generation;
        assert_eq!(
            CanonicalProgram::read(&[&wrong_root], &[]).err().unwrap(),
            model::Refusal::Scope,
        );
    }
    for digest in [view_type, signature_type] {
        let mut wrong_profile = source.clone();
        wrong_profile
            .types
            .get_mut(&digest)
            .unwrap()
            .contract_version = contract::READ_VIEW_GRAPH_CONTRACT_VERSION;
        // Full canonical reading independently re-encodes retained objects
        // before either type walker; malformed object profiles refuse there.
        assert_eq!(
            CanonicalProgram::read(&[&wrong_profile], &[])
                .err()
                .unwrap(),
            model::Refusal::Inventory,
        );
    }
    let retry = CanonicalProgram::read(&[&source], &[]).unwrap();
    assert_eq!(retry.signature_types, canonical.signature_types);
}

#[test]
fn canonical_reference_flow_phase_ledger_preserves_exact_work_carry_and_cancellation() {
    let fixture = recurrence(false, false, false);
    let control = crate::platform::execution::ExecutionControl::uncancelled();
    let mut exact = 0;
    let healthy = associated_flow_canonical_reference::validate(
        &[&fixture.source],
        &control,
        &mut exact,
        contract::MAXIMUM_VALIDATION_WORK,
    )
    .unwrap();
    let mut repeated = 0;
    assert_eq!(
        associated_flow_canonical_reference::validate(
            &[&fixture.source],
            &control,
            &mut repeated,
            exact,
        )
        .unwrap(),
        healthy
    );
    assert_eq!(repeated, exact);
    let carry = 9;
    assert!(exact + carry <= contract::MAXIMUM_VALIDATION_WORK);
    let mut carried = carry;
    assert_eq!(
        associated_flow_canonical_reference::validate(
            &[&fixture.source],
            &control,
            &mut carried,
            exact + carry,
        )
        .unwrap(),
        healthy
    );
    assert_eq!(carried, exact + carry);
    let mut exhausted_work = carry;
    let exhausted = associated_flow_canonical_reference::validate(
        &[&fixture.source],
        &control,
        &mut exhausted_work,
        carry,
    )
    .unwrap_err();
    assert_eq!(
        exhausted.class,
        crate::platform::diagnostic::DiagnosticClass::Resource
    );
    assert_eq!(exhausted.code, "kernel_associated_reference_capacity");
    assert_eq!(
        exhausted.message,
        "independent associated flow exhausted reference work"
    );
    assert_eq!(exhausted_work, carry);
    assert_eq!(exhausted.notes.len(), 1);
    assert!(exhausted.notes[0].contains("phase=associated presence"));
    assert!(exhausted.notes[0].contains("entry=9 consumed=9 maximum=9"));
    assert!(exhausted.notes[0].contains("after_presence: None"));
    let cancelled = crate::platform::execution::ExecutionControl::uncancelled();
    cancelled.cancel();
    let mut cancelled_work = carry;
    let error = associated_flow_canonical_reference::validate(
        &[&fixture.source],
        &cancelled,
        &mut cancelled_work,
        exact + carry,
    )
    .unwrap_err();
    assert_eq!(
        error.class,
        crate::platform::diagnostic::DiagnosticClass::Cancelled
    );
    assert!(error.notes.is_empty());
    assert_eq!(cancelled_work, carry);
}

#[test]
fn canonical_borrowed_declarations_preserve_model_and_scope_with_less_work() {
    let checkpoint = || Ok(());
    let maximum_work = contract::MAXIMUM_VALIDATION_WORK;
    let maximum_bytes = 64 * 1_048_576;
    for expanding in [false, true] {
        let mut fixture = recurrence(expanding, false, false);
        let mut borrowed_work = 0;
        let borrowed_budget =
            model::Budget::new(&checkpoint, &mut borrowed_work, maximum_work, maximum_bytes);
        let borrowed = CanonicalProgram::read_with_budget(
            &[&fixture.source],
            std::slice::from_ref(&fixture.entry),
            &borrowed_budget,
        )
        .expect("complete borrowed translation");
        let borrowed_bytes = borrowed_budget.bytes();
        drop(borrowed_budget);
        let mut copied_work = 0;
        let copied_budget =
            model::Budget::new(&checkpoint, &mut copied_work, maximum_work, maximum_bytes);
        // This comparison really serializes and clones declarations, type
        // objects and other raw owners at their lookup sites. It isolates those
        // avoided copies, rather than reconstructing every old allocation.
        let copied = CanonicalProgram::read_with_raw_copies(
            &[&fixture.source],
            std::slice::from_ref(&fixture.entry),
            &copied_budget,
        )
        .expect("same input and allowance with real raw record copies");
        let copied_bytes = copied_budget.bytes();
        drop(copied_budget);
        assert_eq!(
            format!("{:?}", borrowed.program),
            format!("{:?}", copied.program)
        );
        assert_eq!(borrowed.definitions, copied.definitions);
        assert_eq!(borrowed.members, copied.members);
        assert_eq!(borrowed.witness_parameters, copied.witness_parameters);
        assert_eq!(borrowed.atoms, copied.atoms);
        assert_eq!(borrowed.signature_members, copied.signature_members);
        assert_eq!(borrowed.signature_types, copied.signature_types);
        assert!(borrowed_work < copied_work && borrowed_bytes < copied_bytes);
        assert_eq!(borrowed.program.entries.len(), 1);
        assert_eq!(borrowed.program.schemes.len(), 1);
        assert_eq!(borrowed.members.len(), 2);
        let proof = model::reference_classify(&borrowed.program, model::Limits::default())
            .expect("independent growth classification");
        assert_eq!(proof.finite, !expanding);
        assert_eq!(proof.positive_diagonal.is_some(), expanding);
        println!(
            "borrowed_declaration_control expanding={expanding} borrowed_work={borrowed_work} copied_work={copied_work} borrowed_bytes={borrowed_bytes} copied_bytes={copied_bytes}",
        );

        // A fresh read must still check the raw formal owner's declaration;
        // neither lookup path can carry an admission across this mutation.
        let TypeForm::TypeParameter { parameter } = &fixture.source.types[&fixture.formal].form
        else {
            unreachable!()
        };
        let OwnerRecord::TypeParameter(owner) = fixture
            .source
            .owners
            .get_mut(&OwnerKey::TypeParameter(*parameter))
            .unwrap()
        else {
            unreachable!()
        };
        owner.declaration = fixture.contract.declaration;
        for copies in [false, true] {
            let mut work = 0;
            let budget = model::Budget::new(&checkpoint, &mut work, maximum_work, maximum_bytes);
            let result = if copies {
                CanonicalProgram::read_with_raw_copies(&[&fixture.source], &[], &budget)
            } else {
                CanonicalProgram::read_with_budget(&[&fixture.source], &[], &budget)
            };
            assert!(matches!(result, Err(model::Refusal::Scope)));
        }
    }
}
