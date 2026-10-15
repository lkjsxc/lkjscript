//! Literal scoped meanings and canonical mutants; private entries supply concrete controls only.
use super::*;
use crate::platform::kernel::{ExpressionRecord, FunctionEffect, ParameterRecord};

const SCOPED_SEED: &[u8] = b"independent-associated-scoped-signatures";

#[test]
fn canonical_associated_scoped_signatures_admit_external_empty_scope_inventory_and_refuse_forged_arguments()
 {
    let source = crate::platform::execution::normalized::tests::byte_buffer_tests::author_only(
        r#"
declarations.begin
(units (module create external-scope-inventory
  (external create read (visibility public) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (function create caller (visibility public) (effect pure)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64)
    (body (call read (local value))))))
declarations.end
"#,
    )
    .unwrap();
    let external = named(&source, "read");
    let caller = named(&source, "caller");
    let OwnerRecord::Declaration(owner) =
        &source.owners[&OwnerKey::Declaration(caller.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        unreachable!()
    };
    let call = function.body;
    let control = ExecutionControl::uncancelled();
    let prove = |source: &KernelSnapshot| derive(&[source], &[], Limits::default(), &control);
    let proof = prove(&source).unwrap();
    let cell = encode_type_object(&TypeObject::new(TypeForm::OwnedI64Cell).unwrap())
        .unwrap()
        .0;
    let scalar = encode_type_object(&TypeObject::new(TypeForm::I64).unwrap())
        .unwrap()
        .0;
    for signature in [&proof.symbolic[&external], &proof.symbolic[&caller]] {
        assert_eq!(signature.parameters, vec![(cell, ParameterUse::Borrow)]);
        assert_eq!(signature.result.ty, scalar);
        assert_eq!(signature.result.relation, ResultRelation::Value);
        assert!(!signature.is_task);
        assert!(signature.read_scope_parameters.is_empty());
        assert!(signature.read_scope_sources.is_empty());
        assert!(!signature.requires_read_view_generation);
    }
    assert_eq!(
        proof.symbolic[&caller].symbolic_targets[&call].function,
        Some(external)
    );
    let concrete_caller = proof
        .applications
        .iter()
        .find(|(application, _)| application.function == caller)
        .unwrap()
        .1;
    let target = &concrete_caller.targets[&call];
    assert_eq!(target.function, external);
    assert!(target.read_scope_arguments.is_empty());
    assert_eq!(
        proof.applications[target].parameters,
        vec![(cell, ParameterUse::Borrow)]
    );
    assert_eq!(proof.applications[target].result.ty, scalar);

    // Exact empty External inventory remains distinct from view-scope authority.
    // Both the named-call actual and an explicit entry actual must refuse this
    // fabricated local/foreign formals and wrong binder kinds, rather than
    // treating an empty inventory as a wildcard. These negatives deliberately
    // supply no legitimate formal/source permission from any other owner.
    let foreign = DeclarationReference {
        package: PackageId::migrate(SCOPED_SEED, 900),
        declaration: DeclarationId::migrate(SCOPED_SEED, 900),
    };
    for binder in [
        ReadScopeBinder::Declaration(external),
        ReadScopeBinder::Declaration(caller),
        ReadScopeBinder::Declaration(foreign),
        ReadScopeBinder::ContractMethod {
            contract: external,
            method: MethodId::migrate(SCOPED_SEED, 900),
        },
    ] {
        let forged = ReadScopeArgument::Parameter(ReadScopeReference {
            binder,
            parameter: crate::platform::semantic_id::ReadScopeParameterId::migrate(
                SCOPED_SEED,
                900,
            ),
        });
        let mut changed = source.clone();
        let OwnerRecord::Expression(record) =
            changed.owners.get_mut(&OwnerKey::Expression(call)).unwrap()
        else {
            unreachable!()
        };
        let ExpressionOperation::Call {
            read_scope_arguments,
            ..
        } = &mut record.operation
        else {
            unreachable!()
        };
        read_scope_arguments.push(forged);
        assert_eq!(
            prove(&changed).unwrap_err().code,
            "reference_associated_preparation"
        );
        assert_eq!(
            derive(
                &[&source],
                &[Entry {
                    function: external,
                    types: &[],
                    implementations: &[],
                    effects: &[],
                    requirements: &[],
                    read_scope_arguments: &[forged],
                }],
                Limits::default(),
                &control
            )
            .unwrap_err()
            .code,
            "reference_associated_preparation"
        );
    }
    assert!(prove(&source).is_ok());
}

fn empty(fixture: &Fixture) -> Result<Proof, ExecutionError> {
    derive(
        &[&fixture.source],
        &[],
        Limits::default(),
        &ExecutionControl::uncancelled(),
    )
}

// Two equal-Self caller witnesses are deliberately distinct. The callee's source
// formal and projection have a different lexical scope, so raw digest equality
// cannot stand in for closure through the supplied first witness.
fn forwarded() -> (
    Fixture,
    DeclarationReference,
    ExpressionId,
    ImplementationParameterId,
) {
    let mut fixture = make_fixture();
    let callee = DeclarationReference {
        package: fixture.consumer.package,
        declaration: DeclarationId::migrate(SCOPED_SEED, 0),
    };
    let ordinary = TypeParameterId::migrate(SCOPED_SEED, 0);
    let parameter = ParameterId::migrate(SCOPED_SEED, 0);
    let witness = ImplementationParameterId::migrate(SCOPED_SEED, 0);
    let second = ImplementationParameterId::migrate(SCOPED_SEED, 1);
    let body = ExpressionId::migrate(SCOPED_SEED, 0);
    let call = ExpressionId::migrate(SCOPED_SEED, 1);
    let DeclarationPayload::Function(consumer) = payload(&mut fixture.source, fixture.consumer)
    else {
        unreachable!()
    };
    let mut second_formal = consumer.implementation_parameters[0].clone();
    second_formal.id = second;
    second_formal.name = name("equalSelfSecond");
    consumer.implementation_parameters.push(second_formal);
    let consumer_formal = consumer.type_parameters[0];
    let consumer_witness = consumer.implementation_parameters[0].id;
    let consumer_parameter = consumer.parameters[0];
    let OwnerRecord::Declaration(mut declaration) =
        fixture.source.owners[&OwnerKey::Declaration(fixture.consumer.declaration)].clone()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.type_parameters = vec![ordinary];
    function.parameters = vec![parameter];
    function.body = body;
    function.implementation_parameters.truncate(1);
    function.implementation_parameters[0].id = witness;
    let ordinary_type = add(
        &mut fixture.source,
        TypeForm::TypeParameter {
            parameter: ordinary,
        },
    );
    function.implementation_parameters[0].self_type = ordinary_type;
    let projection = add(
        &mut fixture.source,
        TypeForm::AssociatedWitnessProjection {
            scope: callee,
            parameter: witness,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    function.result.ty = projection;
    function.result.relation =
        crate::platform::kernel::ResultRelation::ReadFrom { source: parameter };
    declaration.header = OwnerHeader::new(
        OwnerKey::Declaration(callee.declaration),
        OwnerKind::PureFunction,
    );
    declaration.name = name("forwardedTarget");
    fixture.source.owners.insert(
        OwnerKey::Declaration(callee.declaration),
        OwnerRecord::Declaration(declaration),
    );
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(ordinary),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(ordinary), OwnerKind::TypeParameter),
            declaration: callee.declaration,
            name: name("V"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    fixture.source.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(callee.declaration),
            name: name("value"),
            ty: projection,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    fixture.source.owners.insert(
        OwnerKey::Expression(body),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                body,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(parameter),
                },
            )
            .unwrap(),
        ),
    );
    let ordinary_type = add(
        &mut fixture.source,
        TypeForm::TypeParameter {
            parameter: consumer_formal,
        },
    );
    let argument = ExpressionId::migrate(SCOPED_SEED, 2);
    fixture.source.owners.insert(
        OwnerKey::Expression(argument),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                argument,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(consumer_parameter),
                },
            )
            .unwrap(),
        ),
    );
    fixture.source.owners.insert(
        OwnerKey::Expression(call),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                call,
                ExpressionOperation::ImplementationCall {
                    function: callee,
                    type_arguments: vec![ordinary_type],
                    implementations: vec![ImplementationOperand::Parameter {
                        scope: fixture.consumer,
                        parameter: consumer_witness,
                    }],
                    effect_arguments: vec![],
                    requirement_arguments: vec![],
                    arguments: vec![argument],
                    read_scope_arguments: Vec::new(),
                },
            )
            .unwrap(),
        ),
    );
    let DeclarationPayload::Function(consumer) = payload(&mut fixture.source, fixture.consumer)
    else {
        unreachable!()
    };
    consumer.body = call;
    (fixture, callee, call, second)
}

#[test]
fn canonical_associated_scoped_signatures_forward_original_lexical_witness() {
    // Current canonical-reader controls are independent of artifact admission.
    let generation = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    let (fixture, callee, call, second) = forwarded();
    let mut fixture = synthetic_owned_generations(fixture, generation, generation);
    let proof = empty(&fixture).unwrap();
    assert!(proof.applications.is_empty());
    let target = &proof.symbolic[&fixture.consumer].symbolic_targets[&call];
    assert_eq!(target.function, Some(callee));
    assert_eq!(
        target.signature.parameters,
        [(fixture.projection, ParameterUse::Borrow)]
    );
    assert_eq!(target.signature.result.ty, fixture.projection);
    assert_eq!(target.signature.result.relation.source().copied(), Some(0));
    assert!(!target.signature.is_task);
    assert!(target.signature.requires_associated_generation);
    let second_projection = add(
        &mut fixture.source,
        TypeForm::AssociatedWitnessProjection {
            scope: fixture.consumer,
            parameter: second,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    implementations[0] = ImplementationOperand::Parameter {
        scope: fixture.consumer,
        parameter: second,
    };
    let other = empty(&fixture).unwrap();
    let target = &other.symbolic[&fixture.consumer].symbolic_targets[&call];
    assert_eq!(target.signature.result.ty, second_projection);
    assert_ne!(target.signature.result.ty, fixture.projection);
    assert_eq!(
        proof
            .read_function(
                &ReadContext::Symbolic(callee),
                &ExecutionControl::uncancelled()
            )
            .unwrap()
            .result
            .ty,
        proof.symbolic[&callee].result.ty
    );
    assert!(
        proof
            .read_function(
                &ReadContext::Symbolic(callee),
                &ExecutionControl::cancel_after_checks(0)
            )
            .is_err()
    );
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    expression.contract_version = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    let covered = empty(&fixture).unwrap();
    assert_eq!(
        covered.symbolic[&fixture.consumer].occurrences[&OwnerKey::Expression(call)],
        crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION
    );
    fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
        .set_encoding_for_edit(32);

    assert_eq!(
        empty(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
    fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
        .set_encoding_for_edit(35);
    assert_eq!(
        empty(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
}

#[test]
fn canonical_associated_scoped_signatures_check_every_unused_generic_mapping() {
    // Current canonical-reader controls are independent of artifact admission.
    let generation = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    let fixture = synthetic_owned_generations(make_fixture(), generation, generation);
    let proof = empty(&fixture).unwrap();
    assert!(proof.applications.is_empty());
    assert_eq!(proof.symbolic_mappings.len(), 3);
    for (index, expected) in [fixture.cell, fixture.block, fixture.cell]
        .into_iter()
        .enumerate()
    {
        let method = proof
            .contract_template(fixture.contract, &ExecutionControl::uncancelled())
            .unwrap()
            .methods[0]
            .id;
        let target = proof
            .symbolic_mapping(
                fixture.implementations[index],
                method,
                &[],
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert_eq!(target.signature.result.ty, expected);
        assert_eq!(
            target.signature.parameters,
            [(expected, ParameterUse::Borrow)]
        );
        assert_eq!(target.signature.result.relation.source().copied(), Some(0));
        assert!(!target.signature.requires_read_view_generation);
    }
    for mutant in 0..4 {
        let mut fixture = synthetic_owned_generations(make_fixture(), generation, generation);
        if mutant == 2 {
            let unit = add(&mut fixture.source, TypeForm::Unit);
            let body = ExpressionId::migrate(SCOPED_SEED, 9);
            fixture.source.owners.insert(
                OwnerKey::Expression(body),
                OwnerRecord::Expression(
                    ExpressionRecord::new(body, ExpressionOperation::Unit {}).unwrap(),
                ),
            );
            fixture
                .source
                .owners
                .get_mut(&OwnerKey::Expression(body))
                .unwrap()
                .set_encoding_for_edit(generation);
            let DeclarationPayload::OwnedContract(contract) =
                payload(&mut fixture.source, fixture.contract)
            else {
                unreachable!()
            };
            contract.methods[0].result.relation = crate::platform::kernel::ResultRelation::Value;
            contract.methods[0].result.ty = unit;
            let DeclarationPayload::Function(function) =
                payload(&mut fixture.source, fixture.reader)
            else {
                unreachable!()
            };
            function.result.relation = crate::platform::kernel::ResultRelation::Value;
            function.result.ty = unit;
            function.body = body;
            assert!(
                empty(&fixture).is_ok(),
                "identical empty pure rows are the healthy task-kind control"
            );
        }
        let DeclarationPayload::Function(function) = payload(&mut fixture.source, fixture.reader)
        else {
            unreachable!()
        };
        match mutant {
            0 => function.result.ty = fixture.block,
            1 => function.result.relation = crate::platform::kernel::ResultRelation::Value,
            2 => {
                function.result.relation = crate::platform::kernel::ResultRelation::Value;
                function.effect = FunctionEffect::Task {
                    requirements: vec![],
                    effect_parameters: vec![],
                };
            }
            3 => {
                let parameter = function.parameters[0];
                let OwnerRecord::Parameter(parameter) = fixture
                    .source
                    .owners
                    .get_mut(&OwnerKey::Parameter(parameter))
                    .unwrap()
                else {
                    unreachable!()
                };
                parameter.use_mode = ParameterUse::Consume;
            }
            _ => unreachable!(),
        }
        if mutant == 2 {
            let OwnerRecord::Declaration(reader) = fixture
                .source
                .owners
                .get_mut(&OwnerKey::Declaration(fixture.reader.declaration))
                .unwrap()
            else {
                unreachable!()
            };
            reader.header.kind = OwnerKind::TaskFunction;
        }
        assert_eq!(
            empty(&fixture).unwrap_err().code,
            "reference_associated_preparation",
            "mutant {mutant}"
        );
    }
}

fn transitive_selected_prerequisite() -> (
    Fixture,
    DeclarationReference,
    ExpressionId,
    ImplementationParameterId,
) {
    let (mut fixture, callee, call, second) = forwarded();
    let implementation = fixture.implementations[0];
    let prerequisite = ImplementationParameterId::migrate(SCOPED_SEED, 3);
    let contract_argument = TypeParameterId::migrate(SCOPED_SEED, 3);
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(contract_argument),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::TypeParameter(contract_argument),
                OwnerKind::TypeParameter,
            ),
            declaration: fixture.contract.declaration,
            name: name("Argument"),
            constraints: TypeParameterConstraints::Owned,
        }),
    );
    let DeclarationPayload::OwnedContract(contract) =
        payload(&mut fixture.source, fixture.contract)
    else {
        unreachable!()
    };
    contract.type_parameters.push(contract_argument);
    for reference in fixture.implementations {
        let DeclarationPayload::OwnedImplementation(scheme) =
            payload(&mut fixture.source, reference)
        else {
            unreachable!()
        };
        scheme.type_arguments = vec![
            scheme
                .associated_types
                .iter()
                .find(|binding| binding.member == fixture.member)
                .unwrap()
                .ty(),
        ];
    }
    for scope in [fixture.consumer, callee] {
        let DeclarationPayload::Function(function) = payload(&mut fixture.source, scope) else {
            unreachable!()
        };
        let ids = function
            .implementation_parameters
            .iter()
            .map(|formal| formal.id)
            .collect::<Vec<_>>();
        for id in ids {
            let argument = add(
                &mut fixture.source,
                TypeForm::AssociatedWitnessProjection {
                    scope,
                    parameter: id,
                    contract: fixture.contract,
                    member: fixture.member,
                },
            );
            let DeclarationPayload::Function(function) = payload(&mut fixture.source, scope) else {
                unreachable!()
            };
            function
                .implementation_parameters
                .iter_mut()
                .find(|formal| formal.id == id)
                .unwrap()
                .type_arguments = vec![argument];
        }
    }
    let DeclarationPayload::OwnedImplementation(scheme) =
        payload(&mut fixture.source, implementation)
    else {
        unreachable!()
    };
    let self_type = scheme.self_type;
    let projected = add(
        &mut fixture.source,
        TypeForm::AssociatedWitnessProjection {
            scope: implementation,
            parameter: prerequisite,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let DeclarationPayload::OwnedImplementation(scheme) =
        payload(&mut fixture.source, implementation)
    else {
        unreachable!()
    };
    scheme.implementation_parameters = vec![ImplementationParameter {
        id: prerequisite,
        name: name("originalCaller"),
        contract: fixture.contract,
        self_type,
        type_arguments: vec![projected],
    }];
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        type_arguments,
        implementations,
        ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    let original = implementations[0].clone();
    implementations[0] = ImplementationOperand::Concrete {
        implementation,
        type_arguments: type_arguments.clone(),
        implementations: vec![original],
    };
    (fixture, callee, call, second)
}

#[test]
fn canonical_associated_scoped_signatures_close_transitive_selected_prerequisites() {
    let (mut fixture, _, call, second) = transitive_selected_prerequisite();
    let implementation = fixture.implementations[0];
    let proof = empty(&fixture).unwrap();
    let SymbolicSelectionSource::Concrete { prerequisites, .. } =
        &proof.symbolic[&fixture.consumer].symbolic_targets[&call].witnesses[0].source
    else {
        unreachable!()
    };
    assert_eq!(prerequisites[0].arguments, [fixture.projection]);
    assert_eq!(
        proof.symbolic[&fixture.consumer].symbolic_targets[&call]
            .signature
            .result
            .ty,
        fixture.cell
    );
    let method = proof
        .contract_template(fixture.contract, &ExecutionControl::uncancelled())
        .unwrap()
        .methods[0]
        .id;
    assert_eq!(
        proof
            .symbolic_mapping(
                implementation,
                method,
                &[],
                &ExecutionControl::uncancelled()
            )
            .unwrap()
            .signature
            .result
            .ty,
        fixture.cell
    );
    let other_projection = add(
        &mut fixture.source,
        TypeForm::AssociatedWitnessProjection {
            scope: fixture.consumer,
            parameter: second,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete {
        implementations, ..
    } = &mut implementations[0]
    else {
        unreachable!()
    };
    implementations[0] = ImplementationOperand::Parameter {
        scope: fixture.consumer,
        parameter: second,
    };
    let other = empty(&fixture).unwrap();
    let SymbolicSelectionSource::Concrete { prerequisites, .. } =
        &other.symbolic[&fixture.consumer].symbolic_targets[&call].witnesses[0].source
    else {
        unreachable!()
    };
    assert_eq!(prerequisites[0].arguments, [other_projection]);
    assert_eq!(
        other.symbolic[&fixture.consumer].symbolic_targets[&call]
            .signature
            .result
            .ty,
        fixture.cell
    );
    assert_ne!(other_projection, fixture.projection);
    check_forward_prerequisite_equations();
}

fn check_forward_prerequisite_equations() {
    let (mut fixture, _, call, second) = transitive_selected_prerequisite();
    let implementation = fixture.implementations[0];
    let later = ImplementationParameterId::migrate(SCOPED_SEED, 4);
    let projection = add(
        &mut fixture.source,
        TypeForm::AssociatedWitnessProjection {
            scope: implementation,
            parameter: later,
            contract: fixture.contract,
            member: fixture.member,
        },
    );
    let DeclarationPayload::OwnedImplementation(scheme) =
        payload(&mut fixture.source, implementation)
    else {
        unreachable!()
    };
    let mut target = scheme.implementation_parameters[0].clone();
    target.id = later;
    target.name = name("laterPrerequisite");
    target.type_arguments = vec![projection];
    scheme.implementation_parameters[0].type_arguments = vec![projection];
    scheme.implementation_parameters.push(target);
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete {
        implementations, ..
    } = &mut implementations[0]
    else {
        unreachable!()
    };
    implementations.push(implementations[0].clone());
    for reverse in [false, true] {
        if reverse {
            let DeclarationPayload::OwnedImplementation(scheme) =
                payload(&mut fixture.source, implementation)
            else {
                unreachable!()
            };
            scheme.implementation_parameters.reverse();
        }
        let proof = empty(&fixture).unwrap();
        let selected = &proof.symbolic[&fixture.consumer].symbolic_targets[&call].witnesses[0];
        let SymbolicSelectionSource::Concrete { prerequisites, .. } = &selected.source else {
            unreachable!()
        };
        assert_eq!(prerequisites.len(), 2);
        assert_eq!(prerequisites[0].arguments, [fixture.projection]);
        assert_eq!(prerequisites[1].arguments, [fixture.projection]);
        assert_eq!(
            proof.symbolic[&fixture.consumer].symbolic_targets[&call]
                .signature
                .result
                .ty,
            fixture.cell
        );
        let types = [fixture.cell];
        let implementations = [operand(&fixture, 2), operand(&fixture, 1)];
        let entries = [Entry {
            function: fixture.consumer,
            types: &types,
            implementations: &implementations,
            effects: &[],
            requirements: &[],
            read_scope_arguments: &[],
        }];
        let concrete = derive(
            &[&fixture.source],
            &entries,
            Limits::default(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        let selected = concrete
            .selections
            .iter()
            .find(|selected| selected.implementation == implementation)
            .unwrap();
        assert_eq!(selected.prerequisites.len(), 2);
        assert_eq!(
            selected.prerequisites[0].implementation,
            fixture.implementations[2]
        );
        assert!(Arc::ptr_eq(
            &selected.prerequisites[0],
            &selected.prerequisites[1]
        ));
        assert!(concrete.applications.iter().any(|(application, function)| {
            application.function == fixture.consumer && function.result.ty == fixture.cell
        }));
    }
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        implementations, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    let ImplementationOperand::Concrete {
        implementations, ..
    } = &mut implementations[0]
    else {
        unreachable!()
    };
    // Equal Self cannot make the later formal's member equal to another caller witness's member.
    implementations[0] = ImplementationOperand::Parameter {
        scope: fixture.consumer,
        parameter: second,
    };
    assert_eq!(
        empty(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
}

#[test]
fn canonical_associated_scoped_signatures_share_work_storage_and_cancel_hits() {
    // Current canonical-reader controls are independent of artifact admission.
    let generation = crate::platform::kernel::contract::GRAPH_CONTRACT_VERSION;
    let (fixture, _, _, _) = forwarded();
    let fixture = synthetic_owned_generations(fixture, generation, generation);
    let healthy = empty(&fixture).unwrap();
    for limits in [
        Limits {
            steps: healthy.steps - 1,
            bytes: healthy.bytes,
        },
        Limits {
            steps: healthy.steps,
            bytes: healthy.bytes - 1,
        },
    ] {
        let error = derive(
            &[&fixture.source],
            &[],
            limits,
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(error.class, ExecutionFailureClass::Resource);
    }
    let exact = derive(
        &[&fixture.source],
        &[],
        Limits {
            steps: healthy.steps,
            bytes: healthy.bytes,
        },
        &ExecutionControl::uncancelled(),
    )
    .unwrap();
    assert_eq!((exact.steps, exact.bytes), (healthy.steps, healthy.bytes));
    let method = healthy
        .contract_template(fixture.contract, &ExecutionControl::uncancelled())
        .unwrap()
        .methods[0]
        .id;
    let cancelled = ExecutionControl::cancel_after_checks(1);
    healthy
        .symbolic_mapping(fixture.implementations[0], method, &[], &cancelled)
        .unwrap();
    assert!(
        healthy
            .symbolic_mapping(fixture.implementations[0], method, &[], &cancelled)
            .is_err()
    );
    for checks in [0, 1, 20, 200, healthy.steps as u64 / 2] {
        assert_eq!(
            derive(
                &[&fixture.source],
                &[],
                Limits::default(),
                &ExecutionControl::cancel_after_checks(checks)
            )
            .unwrap_err()
            .code,
            "execution_cancelled"
        );
    }
    let retry = empty(&fixture).unwrap();
    assert_eq!((retry.steps, retry.bytes), (healthy.steps, healthy.bytes));
}

#[test]
fn canonical_associated_scoped_signatures_close_rows_before_opaque_actual_insertion() {
    use crate::platform::kernel::EffectParameterRecord;
    use crate::platform::semantic_id::EffectParameterId;
    let (mut fixture, callee, call, _) = forwarded();
    let opaque = TypeParameterId::migrate(SCOPED_SEED, 7);
    fixture.source.owners.insert(
        OwnerKey::TypeParameter(opaque),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(opaque), OwnerKind::TypeParameter),
            declaration: callee.declaration,
            name: name("Opaque"),
            constraints: TypeParameterConstraints::None,
        }),
    );
    let caller_effect = EffectParameterId::migrate(SCOPED_SEED, 7);
    let callee_effect = EffectParameterId::migrate(SCOPED_SEED, 8);
    for (scope, id) in [(fixture.consumer, caller_effect), (callee, callee_effect)] {
        fixture.source.owners.insert(
            OwnerKey::EffectParameter(id),
            OwnerRecord::EffectParameter(EffectParameterRecord {
                header: OwnerHeader::new(OwnerKey::EffectParameter(id), OwnerKind::EffectParameter),
                declaration: scope.declaration,
                name: name("E"),
            }),
        );
        let DeclarationPayload::Function(function) = payload(&mut fixture.source, scope) else {
            unreachable!()
        };
        function.effect_parameters.push(id);
    }
    let caller_reference = EffectParameterReference {
        package: fixture.consumer.package,
        parameter: caller_effect,
    };
    let callee_reference = EffectParameterReference {
        package: callee.package,
        parameter: callee_effect,
    };
    let caller_row = EffectRow {
        parameters: vec![caller_reference],
        requirements: vec![],
    };
    let unit = add(&mut fixture.source, TypeForm::Unit);
    let caller_actual = add(
        &mut fixture.source,
        TypeForm::TaskFunction {
            parameters: vec![],
            result: unit,
            effect: caller_row.clone(),
        },
    );
    let ordinary = add(
        &mut fixture.source,
        TypeForm::TypeParameter { parameter: opaque },
    );
    let DeclarationPayload::Function(function) = payload(&mut fixture.source, callee) else {
        unreachable!()
    };
    let projection = function.result.ty;
    let raw = add(
        &mut fixture.source,
        TypeForm::TaskFunction {
            parameters: vec![projection, ordinary],
            result: unit,
            effect: EffectRow {
                parameters: vec![callee_reference],
                requirements: vec![],
            },
        },
    );
    let DeclarationPayload::Function(function) = payload(&mut fixture.source, callee) else {
        unreachable!()
    };
    function.type_parameters.push(opaque);
    function.result.relation = crate::platform::kernel::ResultRelation::Value;
    function.result.ty = raw;
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        type_arguments,
        effect_arguments,
        ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    type_arguments.push(caller_actual);
    effect_arguments.push(EffectRow::default());
    let proof = empty(&fixture).unwrap();
    let expected = TypeObject::new(TypeForm::TaskFunction {
        parameters: vec![fixture.projection, caller_actual],
        result: unit,
        effect: EffectRow::default(),
    })
    .unwrap();
    assert_eq!(
        proof.symbolic[&fixture.consumer].symbolic_targets[&call]
            .signature
            .result
            .ty,
        encode_type_object(&expected).unwrap().0
    );
    assert_eq!(
        proof.types[&caller_actual],
        fixture.source.types[&caller_actual]
    );
    assert!(
        matches!(&proof.types[&caller_actual].form, TypeForm::TaskFunction { effect, .. } if effect == &caller_row)
    );
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        effect_arguments, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    effect_arguments[0] = caller_row.clone();
    let forwarded_row = empty(&fixture).unwrap();
    let expected = TypeObject::new(TypeForm::TaskFunction {
        parameters: vec![fixture.projection, caller_actual],
        result: unit,
        effect: caller_row,
    })
    .unwrap();
    assert_eq!(
        forwarded_row.symbolic[&fixture.consumer].symbolic_targets[&call]
            .signature
            .result
            .ty,
        encode_type_object(&expected).unwrap().0
    );
    let OwnerRecord::Expression(expression) = fixture
        .source
        .owners
        .get_mut(&OwnerKey::Expression(call))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::ImplementationCall {
        effect_arguments, ..
    } = &mut expression.operation
    else {
        unreachable!()
    };
    effect_arguments[0].parameters = vec![callee_reference];
    assert_eq!(
        empty(&fixture).unwrap_err().code,
        "reference_associated_preparation"
    );
}
