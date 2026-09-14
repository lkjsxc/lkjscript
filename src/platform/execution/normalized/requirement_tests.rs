//! Exact finite requirement vectors use the existing ordinary/effect instantiation owner.
use super::*;
use crate::platform::kernel::{
    EffectParameterRecord, EffectParameterReference, EffectRow, RequirementConstraint,
    RequirementOperand, RequirementParameterRecord, RequirementParameterReference,
    TypeParameterConstraints, TypeParameterRecord,
};
use crate::platform::semantic_id::{
    EffectParameterId, RequirementId, RequirementParameterId, TypeParameterId,
};

pub(crate) fn applications(
    count: usize,
    recursive: bool,
) -> crate::platform::kernel::KernelSnapshot {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let package = snapshot.root.package_id;
    let entry = declaration_named(&snapshot, "caller");
    let OwnerRecord::Declaration(mut generic) =
        snapshot.owners[&OwnerKey::Declaration(entry.declaration)].clone()
    else {
        panic!("entry");
    };
    let DeclarationPayload::Function(function) = &generic.payload else {
        panic!("function");
    };
    let unit = function.result;
    let first = function.effect.row().requirements[0];
    let OwnerRecord::Requirement(mut requirement) = snapshot.owners[&first.owner()].clone() else {
        panic!("requirement");
    };
    let interface = requirement.interface;
    let seed = b"requirement-vector-instantiation";
    let second_id = RequirementId::migrate(seed, 0);
    requirement.header = OwnerHeader::new(OwnerKey::Requirement(second_id), OwnerKind::Requirement);
    requirement.name = Name::new("other-exact-requirement").unwrap();
    let component = requirement.declaration;
    snapshot.owners.insert(
        OwnerKey::Requirement(second_id),
        OwnerRecord::Requirement(requirement),
    );
    let second = RequirementOperand::Concrete(crate::platform::kernel::RequirementReference {
        package,
        requirement: second_id,
    });
    let OwnerRecord::Declaration(component) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(component))
        .unwrap()
    else {
        panic!("component");
    };
    let DeclarationPayload::Component { requirements, .. } = &mut component.payload else {
        panic!("component requirements");
    };
    requirements.push(second_id);
    requirements.sort();
    let declaration = DeclarationId::migrate(seed, 0);
    let ordinary = TypeParameterId::migrate(seed, 0);
    let effect = EffectParameterId::migrate(seed, 0);
    snapshot.owners.insert(
        OwnerKey::TypeParameter(ordinary),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(ordinary), OwnerKind::TypeParameter),
            declaration,
            name: Name::new("T").unwrap(),
            constraints: TypeParameterConstraints::None,
        }),
    );
    snapshot.owners.insert(
        OwnerKey::EffectParameter(effect),
        OwnerRecord::EffectParameter(EffectParameterRecord {
            header: OwnerHeader::new(
                OwnerKey::EffectParameter(effect),
                OwnerKind::EffectParameter,
            ),
            declaration,
            name: Name::new("E").unwrap(),
        }),
    );
    let mut parameters = Vec::new();
    let mut operands = Vec::new();
    for index in 0..if recursive { 2 } else { 4 } {
        let parameter = RequirementParameterId::migrate(seed, index);
        parameters.push(parameter);
        operands.push(RequirementOperand::Parameter(
            RequirementParameterReference { package, parameter },
        ));
        snapshot.owners.insert(
            OwnerKey::RequirementParameter(parameter),
            OwnerRecord::RequirementParameter(RequirementParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::RequirementParameter(parameter),
                    OwnerKind::RequirementParameter,
                ),
                declaration,
                name: Name::new(format!("R{index}")).unwrap(),
                constraint: RequirementConstraint {
                    interface,
                    operations: vec![],
                },
            }),
        );
    }
    let generic_reference = DeclarationReference {
        package,
        declaration,
    };
    let ordinary_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::TypeParameter {
            parameter: ordinary,
        },
    );
    let integer = admit_snapshot_type(&mut snapshot, TypeForm::I64);
    let boolean = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    let mut ordinal = 0;
    let mut put = |snapshot: &mut crate::platform::kernel::KernelSnapshot, operation| {
        ordinal += 1;
        let id = ExpressionId::migrate(seed, ordinal);
        snapshot.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
        );
        id
    };
    let done = put(&mut snapshot, ExpressionOperation::Unit {});
    let stop = ParameterId::migrate(seed, 0);
    let body = if recursive {
        snapshot.owners.insert(
            OwnerKey::Parameter(stop),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(stop), OwnerKind::Parameter),
                parent: ParameterParent::Function(declaration),
                name: Name::new("stop").unwrap(),
                ty: boolean,
                use_mode: crate::platform::kernel::ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
        let condition = put(
            &mut snapshot,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(stop),
            },
        );
        let yes = put(&mut snapshot, ExpressionOperation::Bool { value: true });
        let call = put(
            &mut snapshot,
            ExpressionOperation::Call {
                function: generic_reference,
                type_arguments: vec![ordinary_type],
                effect_arguments: vec![EffectRow {
                    requirements: vec![],
                    parameters: vec![EffectParameterReference {
                        package,
                        parameter: effect,
                    }],
                }],
                requirement_arguments: vec![operands[1], operands[0]],
                arguments: vec![yes],
            },
        );
        put(
            &mut snapshot,
            ExpressionOperation::If {
                condition,
                when_true: done,
                when_false: call,
            },
        )
    } else {
        done
    };
    generic.header = OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::PureFunction);
    generic.name = Name::new("finite-requirement-library").unwrap();
    generic.visibility = DeclarationVisibility::Private;
    let DeclarationPayload::Function(function) = &mut generic.payload else {
        panic!("generic function");
    };
    function.type_parameters = vec![ordinary];
    function.effect_parameters = vec![effect];
    function.requirement_parameters = parameters;
    function.parameters = if recursive { vec![stop] } else { vec![] };
    function.effect = FunctionEffect::Pure;
    function.body = body;
    snapshot.owners.insert(
        OwnerKey::Declaration(declaration),
        OwnerRecord::Declaration(generic),
    );
    let mut values = Vec::new();
    if recursive {
        let no = put(&mut snapshot, ExpressionOperation::Bool { value: false });
        values.push(put(
            &mut snapshot,
            ExpressionOperation::Call {
                function: generic_reference,
                type_arguments: vec![integer],
                effect_arguments: vec![EffectRow::default()],
                requirement_arguments: vec![first, second],
                arguments: vec![no],
            },
        ));
    } else {
        for mask in 0..count {
            values.push(put(
                &mut snapshot,
                ExpressionOperation::FunctionValue {
                    function: generic_reference,
                    type_arguments: vec![integer],
                    effect_arguments: vec![EffectRow::default()],
                    requirement_arguments: (0..4)
                        .map(|bit| {
                            if mask & (1 << bit) == 0 {
                                first
                            } else {
                                second
                            }
                        })
                        .collect(),
                },
            ));
        }
        values.push(put(&mut snapshot, ExpressionOperation::Unit {}));
    }
    let body = put(
        &mut snapshot,
        ExpressionOperation::Sequence { items: values },
    );
    let mut acceptor = match &snapshot.owners[&OwnerKey::Declaration(entry.declaration)] {
        OwnerRecord::Declaration(owner) => owner.clone(),
        _ => panic!("acceptor template"),
    };
    let accept = DeclarationId::migrate(seed, 1);
    let callback = ParameterId::migrate(seed, 1);
    let callback_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Function {
            parameters: vec![],
            result: unit,
        },
    );
    let callback_value = put(
        &mut snapshot,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(callback),
        },
    );
    let invoke = put(
        &mut snapshot,
        ExpressionOperation::Invoke {
            callee: callback_value,
            arguments: vec![],
        },
    );
    snapshot.owners.insert(
        OwnerKey::Parameter(callback),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(callback), OwnerKind::Parameter),
            parent: ParameterParent::Function(accept),
            name: Name::new("callback").unwrap(),
            ty: callback_type,
            use_mode: crate::platform::kernel::ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    acceptor.header = OwnerHeader::new(OwnerKey::Declaration(accept), OwnerKind::PureFunction);
    acceptor.name = Name::new("accept-exact-descriptor").unwrap();
    acceptor.visibility = DeclarationVisibility::Private;
    let DeclarationPayload::Function(signature) = &mut acceptor.payload else {
        panic!("acceptor");
    };
    signature.effect = FunctionEffect::Pure;
    signature.parameters = vec![callback];
    signature.body = invoke;
    snapshot.owners.insert(
        OwnerKey::Declaration(accept),
        OwnerRecord::Declaration(acceptor),
    );
    let OwnerRecord::Declaration(entry_owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(entry.declaration))
        .unwrap()
    else {
        panic!("entry owner");
    };
    entry_owner.header.kind = OwnerKind::PureFunction;
    let DeclarationPayload::Function(function) = &mut entry_owner.payload else {
        panic!("entry function");
    };
    function.effect = FunctionEffect::Pure;
    function.body = body;
    let port_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Function {
            parameters: vec![],
            result: unit,
        },
    );
    for owner in snapshot.owners.values_mut() {
        if let OwnerRecord::Port(port) = owner
            && port.implementation == PortImplementation::Function(entry)
        {
            port.function_type = port_type;
        }
    }
    let mut reachable = BTreeSet::new();
    let mut pending = snapshot
        .owners
        .values()
        .flat_map(OwnerRecord::expression_roots)
        .collect::<Vec<_>>();
    while let Some(id) = pending.pop() {
        if reachable.insert(id) {
            let OwnerRecord::Expression(expression) = &snapshot.owners[&OwnerKey::Expression(id)]
            else {
                panic!("expression");
            };
            pending.extend(
                expression
                    .children()
                    .into_iter()
                    .map(|child| child.expression),
            );
        }
    }
    snapshot
        .owners
        .retain(|owner, _| !matches!(owner, OwnerKey::Expression(id) if !reachable.contains(id)));
    let mut types = BTreeSet::new();
    let mut pending = snapshot
        .owners
        .values()
        .flat_map(OwnerRecord::type_roots)
        .collect::<Vec<_>>();
    while let Some(ty) = pending.pop() {
        if types.insert(ty) {
            pending.extend(snapshot.types[&ty].child_types());
        }
    }
    snapshot.types.retain(|ty, _| types.contains(ty));
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

#[test]
fn requirement_vectors_remain_distinct_bounded_and_simultaneously_recursive() {
    for (count, recursive, expected) in
        [(1, false, 1), (2, false, 2), (16, false, 16), (1, true, 2)]
    {
        let snapshot = applications(count, recursive);
        let started = std::time::Instant::now();
        let program = prepare_snapshot(&snapshot);
        let closed = program
            .functions
            .iter()
            .filter(|function| {
                function.declaration == declaration_named(&snapshot, "finite-requirement-library")
                    && !function.requirement_arguments.is_empty()
            })
            .collect::<Vec<_>>();
        assert_eq!(closed.len(), expected);
        let vectors = closed
            .iter()
            .map(|function| function.requirement_arguments.to_vec())
            .collect::<BTreeSet<_>>();
        assert_eq!(vectors.len(), expected);
        assert!(program.work.type_metadata_bytes < 2_000_000);
        if count == 16 {
            for after in [1, 5, program.work.type_derivation_steps / 2] {
                let mut cancelled = program.clone();
                let error = super::super::prepared_types::complete_controlled(
                    &mut cancelled,
                    &ExecutionControl::cancel_after_checks(after),
                )
                .unwrap_err();
                assert_eq!(error.class, crate::platform::DiagnosticClass::Cancelled);
                assert_eq!(error.code, "execution_cancelled");
            }
        }
        eprintln!(
            "requirements count={count} recursive={recursive} closed={expected} fixture_and_preparation_ns={} work={} metadata_bytes={}",
            started.elapsed().as_nanos(),
            program.work.type_derivation_steps,
            program.work.type_metadata_bytes
        );
        let control = ExecutionControl::uncancelled();
        let production = NormalizedVm::new(&program, Default::default())
            .invoke_root_target(&Name::new("command").unwrap(), vec![], None, &control)
            .unwrap();
        let reference =
            NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                .invoke_root_target(&Name::new("command").unwrap(), vec![], None, &control)
                .unwrap();
        assert_eq!(production.0, NormalizedValue::Unit);
        assert_eq!(reference.0, NormalizedValue::Unit);
        assert_eq!(production.1.capability_calls, 0);
        assert_eq!(reference.1.capability_calls, 0);
        if count == 2 && !recursive {
            let indices = program
                .functions
                .iter()
                .enumerate()
                .filter(|(_, function)| {
                    function.declaration
                        == declaration_named(&snapshot, "finite-requirement-library")
                        && !function.requirement_arguments.is_empty()
                })
                .collect::<Vec<_>>();
            let (index, function) = indices[0];
            let integer = program
                .types
                .iter()
                .find_map(|(digest, object)| {
                    matches!(object.form, TypeForm::I64).then_some(*digest)
                })
                .unwrap();
            let accepted = NormalizedValue::Function {
                function: super::super::value::FunctionIndex(index as u32, program.value_origin),
                type_arguments: Arc::from([integer]),
                effect_arguments: Arc::clone(&function.effect_arguments),
                requirement_arguments: Arc::clone(&function.requirement_arguments),
                bound_arguments: None,
            };
            let mut forged = accepted.clone();
            let NormalizedValue::Function {
                requirement_arguments,
                ..
            } = &mut forged
            else {
                panic!("descriptor");
            };
            *requirement_arguments = Arc::clone(&indices[1].1.requirement_arguments);
            let accept = declaration_named(&snapshot, "accept-exact-descriptor");
            // Reference callable indexes name its independently reconstructed canonical owner
            // table. Production indexes name closed prepared applications; these are not a wire
            // identity shared between the two execution owners.
            let schema =
                super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&snapshot])
                    .unwrap();
            let reference_index = schema
                .functions
                .iter()
                .position(|reference| *reference == function.declaration)
                .unwrap();
            let mut reference_accepted = accepted.clone();
            let NormalizedValue::Function {
                function: reference_function,
                ..
            } = &mut reference_accepted
            else {
                panic!("reference descriptor");
            };
            *reference_function =
                super::super::value::FunctionIndex(reference_index as u32, program.value_origin);
            let mut reference_forged = reference_accepted.clone();
            let NormalizedValue::Function {
                requirement_arguments,
                ..
            } = &mut reference_forged
            else {
                panic!("reference descriptor");
            };
            *requirement_arguments = Arc::from([]);
            for (valid, value, reference_value) in [
                (true, accepted, reference_accepted),
                (false, forged, reference_forged),
            ] {
                let production = NormalizedVm::new(&program, Default::default()).invoke(
                    accept,
                    vec![value],
                    None,
                    &control,
                );
                let reference =
                    NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                        .invoke(accept, vec![reference_value], None, &control);
                assert_eq!(
                    production.is_ok(),
                    valid,
                    "production descriptor admission: {production:?}"
                );
                assert_eq!(
                    reference.is_ok(),
                    valid,
                    "reference descriptor admission: {reference:?}"
                );
            }
        }
    }
}

#[test]
fn requirement_minimum_constraints_reject_unused_and_untaken_named_arguments() {
    for untaken in [false, true] {
        let mut snapshot = applications(2, false);
        let declaration = declaration_named(&snapshot, "finite-requirement-library");
        let OwnerRecord::Declaration(record) =
            &snapshot.owners[&OwnerKey::Declaration(declaration.declaration)]
        else {
            panic!("generic");
        };
        let DeclarationPayload::Function(function) = &record.payload else {
            panic!("generic signature");
        };
        let formal = function.requirement_parameters[0];
        let second = RequirementId::migrate(b"requirement-vector-instantiation", 0);
        let OwnerRecord::Requirement(actual) = snapshot
            .owners
            .get_mut(&OwnerKey::Requirement(second))
            .unwrap()
        else {
            panic!("actual");
        };
        let operation = actual.operations[0];
        actual.operations.clear();
        let OwnerRecord::RequirementParameter(parameter) = snapshot
            .owners
            .get_mut(&OwnerKey::RequirementParameter(formal))
            .unwrap()
        else {
            panic!("formal");
        };
        parameter.constraint.operations = vec![operation];
        if untaken {
            let caller = declaration_named(&snapshot, "caller");
            let OwnerRecord::Declaration(record) =
                &snapshot.owners[&OwnerKey::Declaration(caller.declaration)]
            else {
                panic!("caller");
            };
            let DeclarationPayload::Function(function) = &record.payload else {
                panic!("caller signature");
            };
            let root = function.body;
            let seed = b"untaken-requirement-application";
            let condition = ExpressionId::migrate(seed, 0);
            let done = ExpressionId::migrate(seed, 1);
            let hidden = ExpressionId::migrate(seed, 2);
            snapshot.owners.insert(
                OwnerKey::Expression(condition),
                OwnerRecord::Expression(
                    ExpressionRecord::new(condition, ExpressionOperation::Bool { value: true })
                        .unwrap(),
                ),
            );
            snapshot.owners.insert(
                OwnerKey::Expression(done),
                OwnerRecord::Expression(
                    ExpressionRecord::new(done, ExpressionOperation::Unit {}).unwrap(),
                ),
            );
            snapshot.owners.insert(
                OwnerKey::Expression(hidden),
                OwnerRecord::Expression(
                    ExpressionRecord::new(
                        hidden,
                        ExpressionOperation::If {
                            condition,
                            when_true: done,
                            when_false: root,
                        },
                    )
                    .unwrap(),
                ),
            );
            let OwnerRecord::Declaration(record) = snapshot
                .owners
                .get_mut(&OwnerKey::Declaration(caller.declaration))
                .unwrap()
            else {
                panic!("caller");
            };
            let DeclarationPayload::Function(function) = &mut record.payload else {
                panic!("caller signature");
            };
            function.body = hidden;
            snapshot.root.owners = MapRoot::from_parts(
                snapshot.root.owners.page(),
                snapshot.owners.len() as u64,
                snapshot.root.owners.content(),
            );
        }
        let errors = crate::platform::kernel::validate_full(&snapshot).unwrap_err();
        assert!(
            errors
                .iter()
                .any(|error| error.code == "kernel_requirement_argument_constraint"),
            "{untaken}: {errors:?}"
        );
    }
}

#[test]
fn raw_pure_descriptor_cannot_omit_unused_requirement_arguments() {
    let mut snapshot = applications(1, false);
    let library = declaration_named(&snapshot, "finite-requirement-library");
    let OwnerRecord::Declaration(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(library.declaration))
        .unwrap()
    else {
        panic!("library")
    };
    let DeclarationPayload::Function(function) = &mut record.payload else {
        panic!("function")
    };
    let effects = std::mem::take(&mut function.effect_parameters);
    for effect in effects {
        snapshot.owners.remove(&OwnerKey::EffectParameter(effect));
    }
    for owner in snapshot.owners.values_mut() {
        if let OwnerRecord::Expression(expression) = owner
            && let ExpressionOperation::FunctionValue {
                function,
                effect_arguments,
                ..
            } = &mut expression.operation
            && *function == library
        {
            effect_arguments.clear();
        }
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    let index = program
        .functions
        .iter()
        .position(|function| {
            function.declaration == library && !function.requirement_parameters.is_empty()
        })
        .unwrap();
    let integer = program
        .types
        .iter()
        .find_map(|(id, ty)| matches!(ty.form, TypeForm::I64).then_some(*id))
        .unwrap();
    let value = NormalizedValue::Function {
        function: super::super::value::FunctionIndex(index as u32, program.value_origin),
        type_arguments: Arc::from([integer]),
        effect_arguments: Arc::from([]),
        requirement_arguments: Arc::from([]),
        bound_arguments: None,
    };
    let accept = declaration_named(&snapshot, "accept-exact-descriptor");
    let result = NormalizedVm::new(&program, Default::default()).invoke(
        accept,
        vec![value],
        None,
        &ExecutionControl::uncancelled(),
    );
    assert!(
        result.is_err(),
        "unclosed pure descriptor was admitted: {result:?}"
    );
}
