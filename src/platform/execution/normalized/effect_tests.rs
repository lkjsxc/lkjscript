//! Effect applications exercise both execution owners through strictly loaded artifacts.
use super::*;
use crate::platform::kernel::{
    EffectParameterRecord, EffectParameterReference, EffectRow, FieldRecord, FieldReference,
    FieldSelector, ParameterUse, RecordExpressionField, TypeParameterConstraints,
    TypeParameterRecord,
};
use crate::platform::semantic_id::{EffectParameterId, FieldId, TypeParameterId};

const SEED: &[u8] = b"explicit-effect-application-tests";

fn put_expression(
    snapshot: &mut KernelSnapshot,
    ordinal: &mut u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    *ordinal += 1;
    let id = ExpressionId::migrate(SEED, *ordinal);
    assert!(
        snapshot
            .owners
            .insert(
                OwnerKey::Expression(id),
                OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap())
            )
            .is_none()
    );
    id
}

use crate::platform::kernel::KernelSnapshot;

pub(crate) fn library_composition() -> KernelSnapshot {
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let package = snapshot.root.package_id;
    let caller = declaration_named(&snapshot, "caller");
    let (module, body, row) = match &snapshot.owners[&OwnerKey::Declaration(caller.declaration)] {
        OwnerRecord::Declaration(owner) => match &owner.payload {
            DeclarationPayload::Function(function) => {
                (owner.module, function.body, function.effect.row())
            }
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    let (requirement, operation) = snapshot
        .owners
        .values()
        .find_map(|owner| match owner {
            OwnerRecord::Expression(expression) => match expression.operation {
                ExpressionOperation::CapabilityCall {
                    requirement,
                    operation,
                    ..
                } => Some((requirement, operation)),
                _ => None,
            },
            _ => None,
        })
        .unwrap();
    let unit = snapshot
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::Unit).then_some(*ty))
        .unwrap();
    let wrapper = DeclarationId::migrate(SEED, 1);
    let factory = DeclarationId::migrate(SEED, 2);
    let callback = DeclarationId::migrate(SEED, 3);
    let payload = DeclarationId::migrate(SEED, 4);
    let stored = TypeParameterId::migrate(SEED, 4);
    let field = FieldId::migrate(SEED, 4);
    let stored_type =
        admit_snapshot_type(&mut snapshot, TypeForm::TypeParameter { parameter: stored });
    snapshot.owners.insert(
        OwnerKey::TypeParameter(stored),
        OwnerRecord::TypeParameter(TypeParameterRecord {
            header: OwnerHeader::new(OwnerKey::TypeParameter(stored), OwnerKind::TypeParameter),
            declaration: payload,
            name: Name::new("Stored").unwrap(),
            constraints: TypeParameterConstraints::None,
        }),
    );
    snapshot.owners.insert(
        OwnerKey::Declaration(payload),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(payload), OwnerKind::Record),
            module,
            name: Name::new("configured").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Record {
                type_parameters: vec![stored],
                fields: vec![field],
            },
        }),
    );
    snapshot.owners.insert(
        OwnerKey::Field(field),
        OwnerRecord::Field(FieldRecord {
            header: OwnerHeader::new(OwnerKey::Field(field), OwnerKind::Field),
            declaration: payload,
            name: Name::new("callback").unwrap(),
            ty: stored_type,
        }),
    );
    let payload = DeclarationReference {
        package,
        declaration: payload,
    };
    let field = FieldSelector::Nominal(FieldReference { package, field });
    let mut serial = 0;
    for (id, factory_mode) in [(wrapper, false), (factory, true)] {
        let ordinal = if factory_mode { 2 } else { 1 };
        let parameter = TypeParameterId::migrate(SEED, ordinal);
        let effect = EffectParameterId::migrate(SEED, ordinal);
        snapshot.owners.insert(
            OwnerKey::TypeParameter(parameter),
            OwnerRecord::TypeParameter(TypeParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::TypeParameter(parameter),
                    OwnerKind::TypeParameter,
                ),
                declaration: id,
                name: Name::new("Item").unwrap(),
                constraints: TypeParameterConstraints::CaptureSafe,
            }),
        );
        snapshot.owners.insert(
            OwnerKey::EffectParameter(effect),
            OwnerRecord::EffectParameter(EffectParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::EffectParameter(effect),
                    OwnerKind::EffectParameter,
                ),
                declaration: id,
                name: Name::new("E").unwrap(),
            }),
        );
        let symbolic = EffectRow {
            requirements: Vec::new(),
            parameters: vec![EffectParameterReference {
                package,
                parameter: effect,
            }],
        };
        let item = admit_snapshot_type(&mut snapshot, TypeForm::TypeParameter { parameter });
        let task = admit_snapshot_type(
            &mut snapshot,
            TypeForm::TaskFunction {
                parameters: vec![item],
                result: item,
                effect: symbolic.clone(),
            },
        );
        let callee_type = if factory_mode {
            admit_snapshot_type(
                &mut snapshot,
                TypeForm::TaskFunction {
                    parameters: vec![item, item],
                    result: item,
                    effect: symbolic.clone(),
                },
            )
        } else {
            task
        };
        let value = ParameterId::migrate(SEED, ordinal * 10);
        let callee = ParameterId::migrate(SEED, ordinal * 10 + 1);
        for (parameter, name, ty) in [(value, "value", item), (callee, "callback", callee_type)] {
            snapshot.owners.insert(
                OwnerKey::Parameter(parameter),
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                    parent: ParameterParent::Function(id),
                    name: Name::new(name).unwrap(),
                    ty,
                    use_mode: ParameterUse::Unrestricted,
                    resource_requirement: None,
                }),
            );
        }
        let f = put_expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(callee),
            },
        );
        let v = put_expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(value),
            },
        );
        let mut body = put_expression(
            &mut snapshot,
            &mut serial,
            if factory_mode {
                ExpressionOperation::Bind {
                    callee: f,
                    arguments: vec![v],
                }
            } else {
                ExpressionOperation::Invoke {
                    callee: f,
                    arguments: vec![v],
                }
            },
        );
        let result = if factory_mode {
            body = put_expression(
                &mut snapshot,
                &mut serial,
                ExpressionOperation::Record {
                    nominal_type: Some(payload),
                    type_arguments: vec![task],
                    fields: vec![RecordExpressionField {
                        selector: field.clone(),
                        value: body,
                    }],
                },
            );
            admit_snapshot_type(
                &mut snapshot,
                TypeForm::Applied {
                    declaration: payload,
                    arguments: vec![task],
                },
            )
        } else {
            item
        };
        let effect = if factory_mode {
            FunctionEffect::Pure
        } else {
            FunctionEffect::Task {
                requirements: Vec::new(),
                effect_parameters: symbolic.parameters,
            }
        };
        snapshot.owners.insert(
            OwnerKey::Declaration(id),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(id),
                    if factory_mode {
                        OwnerKind::PureFunction
                    } else {
                        OwnerKind::TaskFunction
                    },
                ),
                module,
                name: Name::new(if factory_mode {
                    "configure-task"
                } else {
                    "apply-task"
                })
                .unwrap(),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::Function(FunctionDeclaration {
                    type_parameters: vec![parameter],
                    effect_parameters: vec![EffectParameterId::migrate(SEED, ordinal)],
                    parameters: vec![value, callee],
                    result,
                    effect,
                    body,
                }),
            }),
        );
    }
    let params = [
        ParameterId::migrate(SEED, 30),
        ParameterId::migrate(SEED, 31),
    ];
    for (parameter, name) in params.into_iter().zip(["prefix", "item"]) {
        snapshot.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(callback),
                name: Name::new(name).unwrap(),
                ty: unit,
                use_mode: ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
    }
    let callback_body = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::CapabilityCall {
            requirement,
            operation,
            arguments: Vec::new(),
        },
    );
    snapshot.owners.insert(
        OwnerKey::Declaration(callback),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(callback), OwnerKind::TaskFunction),
            module,
            name: Name::new("consumer-task").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: Vec::new(),
                effect_parameters: Vec::new(),
                parameters: params.to_vec(),
                result: unit,
                effect: FunctionEffect::Task {
                    requirements: row.requirements.clone(),
                    effect_parameters: Vec::new(),
                },
                body: callback_body,
            }),
        }),
    );
    let reference = |id| DeclarationReference {
        package,
        declaration: id,
    };
    let prefix = put_expression(&mut snapshot, &mut serial, ExpressionOperation::Unit {});
    let target = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::FunctionValue {
            function: reference(callback),
            type_arguments: Vec::new(),
            effect_arguments: Vec::new(),
        },
    );
    let configured = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::Call {
            function: reference(factory),
            type_arguments: vec![unit],
            effect_arguments: vec![row.clone()],
            arguments: vec![prefix, target],
        },
    );
    let configured = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::Field {
            value: configured,
            selector: field,
        },
    );
    let item = put_expression(&mut snapshot, &mut serial, ExpressionOperation::Unit {});
    let application = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::Call {
            function: reference(wrapper),
            type_arguments: vec![unit],
            effect_arguments: vec![row],
            arguments: vec![item, configured],
        },
    );
    let OwnerRecord::Expression(root) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body))
        .unwrap()
    else {
        unreachable!()
    };
    let ExpressionOperation::Sequence { items } = &mut root.operation else {
        unreachable!()
    };
    items.push(application);
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

#[test]
fn pure_factory_returns_bound_task_and_generic_library_invokes_it_in_both_evaluators() {
    let snapshot = library_composition();
    assert_factory_invocation(&snapshot);
}

#[test]
fn generic_task_factories_can_capture_their_exact_constrained_type_parameter() {
    let mut snapshot = library_composition();
    let factory = declaration_named(&snapshot, "configure-task");
    let OwnerRecord::Declaration(declaration) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(factory.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    declaration.header.kind = OwnerKind::TaskFunction;
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Task {
        requirements: vec![],
        effect_parameters: function
            .effect_parameters
            .iter()
            .map(|id| EffectParameterReference {
                package: factory.package,
                parameter: *id,
            })
            .collect(),
    };
    let prefix = function.type_parameters[0];
    assert_factory_invocation(&snapshot);
    let OwnerRecord::TypeParameter(parameter) = snapshot
        .owners
        .get_mut(&OwnerKey::TypeParameter(prefix))
        .unwrap()
    else {
        unreachable!()
    };
    parameter.constraints = TypeParameterConstraints::None;
    assert!(
        crate::platform::kernel::validate_full(&snapshot)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "kernel_type_bind_capture")
    );
}

fn assert_factory_invocation(snapshot: &KernelSnapshot) {
    crate::platform::kernel::validate_full(snapshot).unwrap();
    let (_temporary, repository, program) = prepare_repository(snapshot);
    let canonical = repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value;
    let caller = declaration_named(snapshot, "caller");
    let schema = super::super::reference::NormalizedReferenceRead::schema(&canonical).unwrap();
    let identities = |records: &[super::super::prepare::NormalizedRecordLayout]| {
        records
            .iter()
            .map(|layout| (layout.declaration, layout.arguments.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(identities(&program.records), identities(&schema.records));
    for reference in [false, true] {
        let (capabilities, calls) = bind_fixture_capability(&program, 2);
        let result = if reference {
            super::super::reference::NormalizedReferenceInterpreter::new(
                &canonical,
                &program,
                NormalizedRunPolicy::default(),
            )
            .invoke(
                caller,
                Vec::new(),
                Some(&capabilities),
                &ExecutionControl::uncancelled(),
            )
            .map(|(value, _)| value)
        } else {
            NormalizedVm::new(&program, NormalizedRunPolicy::default())
                .invoke(
                    caller,
                    Vec::new(),
                    Some(&capabilities),
                    &ExecutionControl::uncancelled(),
                )
                .map(|(value, _)| value)
        };
        assert!(matches!(result.unwrap(), NormalizedValue::Unit));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}

#[test]
fn task_invocation_checks_activation_even_when_the_component_has_the_grant() {
    let snapshot = library_composition();
    let (_temporary, _repository, mut program) = prepare_repository(&snapshot);
    let wrapper = declaration_named(&snapshot, "apply-task");
    // The independent tier reads a separately narrowed canonical test fixture. It does not
    // consume the production mutation below or its allowance inventory.
    let mut narrowed = snapshot.clone();
    let Some(OwnerRecord::Declaration(declaration)) = narrowed
        .owners
        .get_mut(&OwnerKey::Declaration(wrapper.declaration))
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Task {
        requirements: Vec::new(),
        effect_parameters: Vec::new(),
    };
    let (capabilities, calls) = bind_fixture_capability(&program, 10);
    let error = super::super::reference::NormalizedReferenceInterpreter::new(
        &narrowed,
        &program,
        NormalizedRunPolicy::default(),
    )
    .invoke(
        declaration_named(&snapshot, "caller"),
        Vec::new(),
        Some(&capabilities),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert!(error.message.contains("allowance"), "{error:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    // A raw/test-only activation probe deliberately narrows the prepared wrapper after
    // strict loading. The valid descriptor and the selected component keep their real row.
    for function in Arc::make_mut(&mut program.functions) {
        if function.declaration == wrapper && function.effect_parameters.is_empty() {
            function.effect = FunctionEffect::Task {
                requirements: Vec::new(),
                effect_parameters: Vec::new(),
            };
            function.task_requirements = Arc::from([]);
        }
    }
    let (capabilities, calls) = bind_fixture_capability(&program, 10);
    let result = NormalizedVm::new(&program, NormalizedRunPolicy::default()).invoke(
        declaration_named(&snapshot, "caller"),
        Vec::new(),
        Some(&capabilities),
        &ExecutionControl::uncancelled(),
    );
    let error = result.unwrap_err();
    assert!(error.message.contains("allowance"), "{error:?}");
    // The caller's prior direct effect remains visible; the callback's first effect is absent.
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn effect_arity_and_task_kind_are_universal_canonical_obligations() {
    let original = library_composition();
    for omission in [true, false] {
        let mut snapshot = original.clone();
        let expression = snapshot
            .owners
            .values_mut()
            .find_map(|owner| match owner {
                OwnerRecord::Expression(expression) => match &mut expression.operation {
                    ExpressionOperation::Call {
                        effect_arguments, ..
                    } if !effect_arguments.is_empty() => Some(effect_arguments),
                    _ => None,
                },
                _ => None,
            })
            .unwrap();
        if omission {
            expression.clear();
        } else {
            expression.push(expression[0].clone());
        }
        assert!(
            crate::platform::kernel::validate_full(&snapshot)
                .unwrap_err()
                .iter()
                .any(|error| error.code == "kernel_effect_argument_count")
        );
    }
    let mut snapshot = original;
    let wrapper = declaration_named(&snapshot, "apply-task");
    let OwnerRecord::Declaration(declaration) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(wrapper.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut declaration.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Pure;
    declaration.header.kind = OwnerKind::PureFunction;
    assert!(
        crate::platform::kernel::validate_full(&snapshot)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "kernel_type_pure_task_call")
    );
}

#[test]
fn effect_parameter_scope_is_exact_and_renaming_does_not_change_applications() {
    let original = library_composition();
    let (parameter, function) = original
        .owners
        .iter()
        .find_map(|(key, record)| match (key, record) {
            (OwnerKey::EffectParameter(id), OwnerRecord::EffectParameter(record)) => {
                Some((*id, record.declaration))
            }
            _ => None,
        })
        .unwrap();
    let mut renamed = original.clone();
    let OwnerRecord::EffectParameter(record) = renamed
        .owners
        .get_mut(&OwnerKey::EffectParameter(parameter))
        .unwrap()
    else {
        unreachable!()
    };
    record.name = Name::new("CallbackEffects").unwrap();
    crate::platform::kernel::validate_full(&renamed).unwrap();
    assert_eq!(renamed.types, original.types);
    for (key, record) in &original.owners {
        if *key != OwnerKey::EffectParameter(parameter) {
            assert_eq!(renamed.owners.get(key), Some(record));
        }
    }
    assert_factory_invocation(&renamed);

    for case in ["duplicate", "foreign-owner", "foreign-argument"] {
        let mut invalid = original.clone();
        let expected = match case {
            "duplicate" => {
                let OwnerRecord::Declaration(record) = invalid
                    .owners
                    .get_mut(&OwnerKey::Declaration(function))
                    .unwrap()
                else {
                    unreachable!()
                };
                let DeclarationPayload::Function(signature) = &mut record.payload else {
                    unreachable!()
                };
                signature.effect_parameters.push(parameter);
                "kernel_owner_duplicate_child"
            }
            "foreign-owner" => {
                let caller = declaration_named(&invalid, "caller");
                let OwnerRecord::EffectParameter(record) = invalid
                    .owners
                    .get_mut(&OwnerKey::EffectParameter(parameter))
                    .unwrap()
                else {
                    unreachable!()
                };
                record.declaration = caller.declaration;
                "kernel_effect_parameter_owner"
            }
            _ => {
                let row = invalid
                    .owners
                    .values_mut()
                    .find_map(|owner| match owner {
                        OwnerRecord::Expression(expression) => match &mut expression.operation {
                            ExpressionOperation::Call {
                                effect_arguments, ..
                            } if !effect_arguments.is_empty() => Some(&mut effect_arguments[0]),
                            _ => None,
                        },
                        _ => None,
                    })
                    .unwrap();
                row.parameters = vec![EffectParameterReference {
                    package: invalid.root.package_id,
                    parameter,
                }];
                "kernel_effect_parameter_scope"
            }
        };
        let failures = crate::platform::kernel::validate_full(&invalid).unwrap_err();
        assert!(
            failures
                .iter()
                .any(|diagnostic| diagnostic.code == expected),
            "{case}: {failures:?}"
        );
    }
}

#[test]
fn recursive_effect_permutation_and_union_close_by_finite_set_identity() {
    let mut snapshot = library_composition();
    let package = snapshot.root.package_id;
    let callback = declaration_named(&snapshot, "consumer-task");
    let OwnerRecord::Declaration(owner) =
        &snapshot.owners[&OwnerKey::Declaration(callback.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &owner.payload else {
        unreachable!()
    };
    let (module, unit, concrete) = (owner.module, function.result, function.effect.row());
    let boolean = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    let recursive = DeclarationId::migrate(SEED, 20);
    let entry = DeclarationId::migrate(SEED, 21);
    let mut parameters = Vec::new();
    for (index, name) in [(0, "E"), (1, "F")] {
        let id = EffectParameterId::migrate(SEED, 20 + index);
        snapshot.owners.insert(
            OwnerKey::EffectParameter(id),
            OwnerRecord::EffectParameter(EffectParameterRecord {
                header: OwnerHeader::new(OwnerKey::EffectParameter(id), OwnerKind::EffectParameter),
                declaration: recursive,
                name: Name::new(name).unwrap(),
            }),
        );
        parameters.push(id);
    }
    let symbolic = |index| EffectRow {
        requirements: vec![],
        parameters: vec![EffectParameterReference {
            package,
            parameter: parameters[index],
        }],
    };
    let (first, second) = (symbolic(0), symbolic(1));
    let mut both = EffectRow {
        requirements: vec![],
        parameters: vec![first.parameters[0], second.parameters[0]],
    };
    both.normalize().unwrap();
    let stop = ParameterId::migrate(SEED, 200);
    snapshot.owners.insert(
        OwnerKey::Parameter(stop),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(stop), OwnerKind::Parameter),
            parent: ParameterParent::Function(recursive),
            name: Name::new("stop").unwrap(),
            ty: boolean,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    let mut ordinal = 2000;
    let mut calls = Vec::new();
    for arguments in [
        vec![second.clone(), first.clone()],
        vec![both.clone(), second],
    ] {
        let done = put_expression(
            &mut snapshot,
            &mut ordinal,
            ExpressionOperation::Bool { value: true },
        );
        calls.push(put_expression(
            &mut snapshot,
            &mut ordinal,
            ExpressionOperation::Call {
                function: DeclarationReference {
                    package,
                    declaration: recursive,
                },
                type_arguments: vec![],
                effect_arguments: arguments,
                arguments: vec![done],
            },
        ));
    }
    let otherwise = put_expression(
        &mut snapshot,
        &mut ordinal,
        ExpressionOperation::Sequence { items: calls },
    );
    let done = put_expression(&mut snapshot, &mut ordinal, ExpressionOperation::Unit {});
    let condition = put_expression(
        &mut snapshot,
        &mut ordinal,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(stop),
        },
    );
    let body = put_expression(
        &mut snapshot,
        &mut ordinal,
        ExpressionOperation::If {
            condition,
            when_true: done,
            when_false: otherwise,
        },
    );
    snapshot.owners.insert(
        OwnerKey::Declaration(recursive),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(recursive), OwnerKind::TaskFunction),
            module,
            name: Name::new("combine-effects").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![],
                effect_parameters: parameters.clone(),
                parameters: vec![stop],
                result: unit,
                effect: FunctionEffect::Task {
                    requirements: vec![],
                    effect_parameters: both.parameters,
                },
                body,
            }),
        }),
    );
    let begin = put_expression(
        &mut snapshot,
        &mut ordinal,
        ExpressionOperation::Bool { value: false },
    );
    let body = put_expression(
        &mut snapshot,
        &mut ordinal,
        ExpressionOperation::Call {
            function: DeclarationReference {
                package,
                declaration: recursive,
            },
            type_arguments: vec![],
            effect_arguments: vec![concrete.clone(), EffectRow::default()],
            arguments: vec![begin],
        },
    );
    snapshot.owners.insert(
        OwnerKey::Declaration(entry),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(entry), OwnerKind::TaskFunction),
            module,
            name: Name::new("row-entry").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![],
                effect_parameters: vec![],
                parameters: vec![],
                result: unit,
                effect: FunctionEffect::Task {
                    requirements: concrete.requirements.clone(),
                    effect_parameters: vec![],
                },
                body,
            }),
        }),
    );
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    crate::platform::kernel::validate_full(&snapshot).unwrap();
    let program = prepare_snapshot(&snapshot);
    let mut expected = BTreeSet::new();
    let mut pending = vec![(1_u8, 0_u8)];
    while let Some((e, f)) = pending.pop() {
        if expected.insert((e, f)) {
            pending.extend([(f, e), (e | f, f)]);
        }
    }
    let actual = program
        .functions
        .iter()
        .filter(|function| {
            function.declaration.declaration == recursive && function.effect_parameters.is_empty()
        })
        .map(|function| {
            assert_eq!(function.effect_arguments.len(), 2);
            let mask = |row: &EffectRow| {
                assert!(row.parameters.is_empty());
                assert!(row.requirements.is_empty() || row.requirements == concrete.requirements);
                u8::from(!row.requirements.is_empty())
            };
            (
                mask(&function.effect_arguments[0]),
                mask(&function.effect_arguments[1]),
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(actual, expected);
    assert_eq!(actual.len(), 3);
    for reference in [false, true] {
        let (capabilities, calls) = bind_fixture_capability(&program, 1);
        let entry = DeclarationReference {
            package,
            declaration: entry,
        };
        let result = if reference {
            super::super::reference::NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                Default::default(),
            )
            .invoke(
                entry,
                vec![],
                Some(&capabilities),
                &ExecutionControl::uncancelled(),
            )
            .map(|r| r.0)
        } else {
            NormalizedVm::new(&program, Default::default())
                .invoke(
                    entry,
                    vec![],
                    Some(&capabilities),
                    &ExecutionControl::uncancelled(),
                )
                .map(|r| r.0)
        };
        assert_eq!(result.unwrap(), NormalizedValue::Unit);
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }
    let mut renamed = snapshot.clone();
    let OwnerRecord::EffectParameter(parameter) = renamed
        .owners
        .get_mut(&OwnerKey::EffectParameter(parameters[0]))
        .unwrap()
    else {
        unreachable!()
    };
    parameter.name = Name::new("Renamed").unwrap();
    crate::platform::kernel::validate_full(&renamed).unwrap();
    for fault in ["foreign-owner", "duplicate-parameter", "wrong-scope"] {
        let mut invalid = snapshot.clone();
        if fault == "foreign-owner" {
            let OwnerRecord::EffectParameter(parameter) = invalid
                .owners
                .get_mut(&OwnerKey::EffectParameter(parameters[0]))
                .unwrap()
            else {
                unreachable!()
            };
            parameter.declaration = entry;
        } else {
            let OwnerRecord::Declaration(owner) = invalid
                .owners
                .get_mut(&OwnerKey::Declaration(recursive))
                .unwrap()
            else {
                unreachable!()
            };
            let DeclarationPayload::Function(function) = &mut owner.payload else {
                unreachable!()
            };
            if fault == "duplicate-parameter" {
                function.effect_parameters.push(parameters[0]);
            } else {
                function.effect_parameters.pop();
            }
        }
        assert!(
            crate::platform::kernel::validate_full(&invalid).is_err(),
            "{fault}"
        );
    }
}

fn task_input_fixture() -> (KernelSnapshot, DeclarationReference, TypeObjectDigest) {
    let mut snapshot = library_composition();
    let package = snapshot.root.package_id;
    let callback = declaration_named(&snapshot, "consumer-task");
    let OwnerRecord::Declaration(owner) =
        &snapshot.owners[&OwnerKey::Declaration(callback.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(original) = &owner.payload else {
        unreachable!()
    };
    let module = owner.module;
    let unit = original.result;
    let row = original.effect.row();
    let ty = admit_snapshot_type(
        &mut snapshot,
        TypeForm::TaskFunction {
            parameters: vec![unit],
            result: unit,
            effect: row.clone(),
        },
    );
    let accept = DeclarationId::migrate(SEED, 10);
    let parameter = ParameterId::migrate(SEED, 100);
    snapshot.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Function(accept),
            name: Name::new("callback").unwrap(),
            ty,
            use_mode: ParameterUse::Unrestricted,
            resource_requirement: None,
        }),
    );
    let mut serial = 1000;
    let callee = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        },
    );
    let argument = put_expression(&mut snapshot, &mut serial, ExpressionOperation::Unit {});
    let body = put_expression(
        &mut snapshot,
        &mut serial,
        ExpressionOperation::Invoke {
            callee,
            arguments: vec![argument],
        },
    );
    snapshot.owners.insert(
        OwnerKey::Declaration(accept),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(accept), OwnerKind::TaskFunction),
            module,
            name: Name::new("accept-task").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![],
                effect_parameters: vec![],
                parameters: vec![parameter],
                result: unit,
                effect: FunctionEffect::Task {
                    requirements: row.requirements,
                    effect_parameters: vec![],
                },
                body,
            }),
        }),
    );
    for (ordinal, name, pure) in [(11, "pure-prefix", true), (12, "empty-task-prefix", false)] {
        let declaration = DeclarationId::migrate(SEED, ordinal);
        let mut parameters = Vec::new();
        for index in 0..2 {
            let parameter = ParameterId::migrate(SEED, ordinal * 10 + index);
            parameters.push(parameter);
            snapshot.owners.insert(
                OwnerKey::Parameter(parameter),
                OwnerRecord::Parameter(ParameterRecord {
                    header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                    parent: ParameterParent::Function(declaration),
                    name: Name::new(if index == 0 { "prefix" } else { "item" }).unwrap(),
                    ty: unit,
                    use_mode: ParameterUse::Unrestricted,
                    resource_requirement: None,
                }),
            );
        }
        let body = put_expression(&mut snapshot, &mut serial, ExpressionOperation::Unit {});
        snapshot.owners.insert(
            OwnerKey::Declaration(declaration),
            OwnerRecord::Declaration(DeclarationRecord {
                header: OwnerHeader::new(
                    OwnerKey::Declaration(declaration),
                    if pure {
                        OwnerKind::PureFunction
                    } else {
                        OwnerKind::TaskFunction
                    },
                ),
                module,
                name: Name::new(name).unwrap(),
                visibility: DeclarationVisibility::Public,
                payload: DeclarationPayload::Function(FunctionDeclaration {
                    type_parameters: vec![],
                    effect_parameters: vec![],
                    parameters,
                    result: unit,
                    effect: if pure {
                        FunctionEffect::Pure
                    } else {
                        FunctionEffect::Task {
                            requirements: vec![],
                            effect_parameters: vec![],
                        }
                    },
                    body,
                }),
            }),
        );
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    (
        snapshot,
        DeclarationReference {
            package,
            declaration: accept,
        },
        ty,
    )
}

#[test]
fn task_descriptor_raw_boundaries_reject_kind_row_origin_arity_and_retained_resource() {
    let (snapshot, accept, task_type) = task_input_fixture();
    let program = prepare_snapshot(&snapshot);
    let foreign = prepare_snapshot(&snapshot);
    let callback = declaration_named(&snapshot, "consumer-task");
    let index = program.function(callback).unwrap();
    let make = |function,
                types: Vec<TypeObjectDigest>,
                effects: Vec<EffectRow>,
                prefix: Option<Vec<NormalizedValue>>| NormalizedValue::Function {
        function,
        type_arguments: types.into(),
        effect_arguments: effects.into(),
        bound_arguments: prefix.map(Arc::new),
    };
    let valid = make(index, vec![], vec![], Some(vec![NormalizedValue::Unit]));
    let scope = super::super::resource::NormalizedResourceScope::new().unwrap();
    let requirement = &program.requirements[0];
    let resource = scope
        .reserve_queue_lease(requirement.reference, requirement.interface)
        .unwrap()
        .commit(crate::platform::queue::JobLease {
            job_id: "task-prefix".into(),
            attempt_id: "owned".into(),
            worker_id: "test".into(),
            payload: vec![],
            attempt_number: 1,
            lease_until_milliseconds: 1,
        })
        .unwrap();
    let cases = [
        ("valid", valid.clone(), true),
        ("unbound-arity", make(index, vec![], vec![], None), false),
        (
            "excess-prefix",
            make(index, vec![], vec![], Some(vec![NormalizedValue::Unit; 3])),
            false,
        ),
        (
            "prefix-type",
            make(index, vec![], vec![], Some(vec![NormalizedValue::I64(7)])),
            false,
        ),
        (
            "effect-arity",
            make(
                index,
                vec![],
                vec![EffectRow::default()],
                Some(vec![NormalizedValue::Unit]),
            ),
            false,
        ),
        (
            "type-arity",
            make(
                index,
                vec![task_type],
                vec![],
                Some(vec![NormalizedValue::Unit]),
            ),
            false,
        ),
        (
            "foreign-preparation",
            make(
                foreign.function(callback).unwrap(),
                vec![],
                vec![],
                Some(vec![NormalizedValue::Unit]),
            ),
            false,
        ),
        (
            "pure-kind",
            make(
                program
                    .function(declaration_named(&snapshot, "pure-prefix"))
                    .unwrap(),
                vec![],
                vec![],
                Some(vec![NormalizedValue::Unit]),
            ),
            false,
        ),
        (
            "wrong-row",
            make(
                program
                    .function(declaration_named(&snapshot, "empty-task-prefix"))
                    .unwrap(),
                vec![],
                vec![],
                Some(vec![NormalizedValue::Unit]),
            ),
            false,
        ),
        (
            "live-prefix",
            make(
                index,
                vec![],
                vec![],
                Some(vec![NormalizedValue::Resource(resource)]),
            ),
            false,
        ),
    ];
    for reference in [false, true] {
        for (label, value, accepted) in &cases {
            let (capabilities, calls) = bind_fixture_capability(&program, 1);
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                super::super::reference::NormalizedReferenceInterpreter::new(
                    &snapshot,
                    &program,
                    NormalizedRunPolicy::default(),
                )
                .invoke(accept, vec![value.clone()], Some(&capabilities), &control)
                .map(|(value, _)| value)
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::default())
                    .invoke(accept, vec![value.clone()], Some(&capabilities), &control)
                    .map(|(value, _)| value)
            };
            assert_eq!(
                result.is_ok(),
                *accepted,
                "{label}, reference={reference}: {result:?}"
            );
            assert_eq!(
                calls.load(Ordering::SeqCst),
                u64::from(*accepted),
                "{label}"
            );
            assert_eq!(
                scope.live_resources(),
                1,
                "raw rejection must preserve its caller's retained resource"
            );
        }
    }
    assert!(!valid.is_durable());
    assert!(
        super::super::codec::encode_typed(
            &program,
            &valid,
            task_type,
            crate::platform::json::JsonLimits::default()
        )
        .is_err()
    );
    scope.release_all();
    assert_eq!(scope.live_resources(), 0);
}

#[test]
fn empty_and_inactive_task_containers_are_transient_but_not_serializable() {
    let (mut snapshot, accept, task) = task_input_fixture();
    let package = snapshot.root.package_id;
    let OwnerRecord::Declaration(owner) =
        &snapshot.owners[&OwnerKey::Declaration(accept.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(signature) = &owner.payload else {
        unreachable!()
    };
    let (module, unit) = (owner.module, signature.result);
    let list = admit_snapshot_type(&mut snapshot, TypeForm::List { item: task });
    let declaration = DeclarationId::migrate(SEED, 30);
    let mut cases = Vec::new();
    for (ordinal, name, payload) in [(30, "absent", None), (31, "present", Some(task))] {
        let id = crate::platform::semantic_id::CaseId::migrate(SEED, ordinal);
        snapshot.owners.insert(
            OwnerKey::Case(id),
            OwnerRecord::Case(crate::platform::kernel::CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(id), OwnerKind::Case),
                declaration,
                name: Name::new(name).unwrap(),
                payload,
            }),
        );
        cases.push(id);
    }
    cases.sort();
    snapshot.owners.insert(
        OwnerKey::Declaration(declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Variant),
            module,
            name: Name::new("task-container").unwrap(),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Variant {
                type_parameters: vec![],
                cases,
            },
        }),
    );
    let nominal = admit_snapshot_type(
        &mut snapshot,
        TypeForm::Named {
            declaration: DeclarationReference {
                package,
                declaration,
            },
        },
    );
    let discard = DeclarationId::migrate(SEED, 31);
    let mut parameters = Vec::new();
    for (ordinal, name, ty) in [(300, "empty", list), (301, "inactive", nominal)] {
        let parameter = ParameterId::migrate(SEED, ordinal);
        snapshot.owners.insert(
            OwnerKey::Parameter(parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
                parent: ParameterParent::Function(discard),
                name: Name::new(name).unwrap(),
                ty,
                use_mode: ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
        parameters.push(parameter);
    }
    let mut ordinal = 3000;
    let body = put_expression(&mut snapshot, &mut ordinal, ExpressionOperation::Unit {});
    snapshot.owners.insert(
        OwnerKey::Declaration(discard),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(discard), OwnerKind::PureFunction),
            module,
            name: Name::new("discard-task-containers").unwrap(),
            visibility: DeclarationVisibility::Private,
            payload: DeclarationPayload::Function(FunctionDeclaration {
                type_parameters: vec![],
                effect_parameters: vec![],
                parameters,
                result: unit,
                effect: FunctionEffect::Pure,
                body,
            }),
        }),
    );
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    let layout = program.variant_instances[&nominal];
    let case = program.variants[layout.0 as usize]
        .cases
        .iter()
        .position(|case| case.name.as_str() == "absent")
        .unwrap();
    let empty = NormalizedValue::list(vec![]).unwrap();
    let inactive = NormalizedValue::Variant {
        layout,
        case: case as u32,
        payload: None,
    };
    let discard = DeclarationReference {
        package,
        declaration: discard,
    };
    for reference in [false, true] {
        let values = vec![empty.clone(), inactive.clone()];
        let result = if reference {
            super::super::reference::NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                Default::default(),
            )
            .invoke(discard, values, None, &ExecutionControl::uncancelled())
            .map(|r| r.0)
        } else {
            NormalizedVm::new(&program, Default::default())
                .invoke(discard, values, None, &ExecutionControl::uncancelled())
                .map(|r| r.0)
        };
        assert_eq!(result.unwrap(), NormalizedValue::Unit);
    }
    for (ty, value, json) in [
        (list, empty, serde_json::json!([])),
        (nominal, inactive, serde_json::json!({"case":"absent"})),
    ] {
        assert!(
            super::super::codec::encode_typed(&program, &value, ty, Default::default()).is_err()
        );
        assert!(
            super::super::codec::decode_value(&program, &json, ty, Default::default()).is_err()
        );
        assert!(super::super::data_codec::encode_typed(&program, &value, ty).is_err());
    }
}

#[test]
fn a_shared_canonical_grant_does_not_widen_the_declared_operation_allowance() {
    use crate::platform::semantic_id::{OperationId, RequirementId};
    let (mut snapshot, accept, _) = task_input_fixture();
    let wide = match &snapshot.owners[&OwnerKey::Declaration(accept.declaration)] {
        OwnerRecord::Declaration(owner) => match &owner.payload {
            DeclarationPayload::Function(f) => f.effect.row().requirements[0],
            _ => unreachable!(),
        },
        _ => unreachable!(),
    };
    let OwnerRecord::Requirement(original) =
        &snapshot.owners[&OwnerKey::Requirement(wide.requirement)]
    else {
        unreachable!()
    };
    let mut narrow = original.clone();
    let operation = OperationId::migrate(SEED, 90);
    let OwnerRecord::Operation(original_operation) =
        &snapshot.owners[&OwnerKey::Operation(original.operations[0].operation)]
    else {
        unreachable!()
    };
    let mut added = original_operation.clone();
    added.header = OwnerHeader::new(OwnerKey::Operation(operation), OwnerKind::Operation);
    added.name = Name::new("second-operation").unwrap();
    snapshot.owners.insert(
        OwnerKey::Operation(operation),
        OwnerRecord::Operation(added),
    );
    let OwnerRecord::Declaration(interface) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(narrow.interface.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Interface { operations } = &mut interface.payload else {
        unreachable!()
    };
    operations.push(operation);
    operations.sort();
    let OwnerRecord::Requirement(available) = snapshot
        .owners
        .get_mut(&OwnerKey::Requirement(wide.requirement))
        .unwrap()
    else {
        unreachable!()
    };
    available
        .operations
        .push(crate::platform::kernel::OperationReference {
            package: wide.package,
            operation,
        });
    available.operations.sort();
    let alias = RequirementId::migrate(SEED, 90);
    narrow.header = OwnerHeader::new(OwnerKey::Requirement(alias), OwnerKind::Requirement);
    narrow.declaration = accept.declaration;
    snapshot.owners.insert(
        OwnerKey::Requirement(alias),
        OwnerRecord::Requirement(narrow),
    );
    let narrow = crate::platform::kernel::RequirementReference {
        package: wide.package,
        requirement: alias,
    };
    let OwnerRecord::Declaration(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(accept.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &mut owner.payload else {
        unreachable!()
    };
    let FunctionEffect::Task { requirements, .. } = &mut f.effect else {
        unreachable!()
    };
    requirements.push(narrow);
    requirements.sort();
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let mut program = prepare_snapshot(&snapshot);
    let wide_index = program
        .requirements
        .iter()
        .position(|r| r.reference == wide)
        .unwrap();
    let narrow_index = program
        .requirements
        .iter()
        .position(|r| r.reference == narrow)
        .unwrap();
    let (capabilities, _) = bind_fixture_capability(&program, 1);
    assert_eq!(
        capabilities
            .canonical_requirement_exact(&program, wide)
            .unwrap(),
        capabilities
            .canonical_requirement_exact(&program, narrow)
            .unwrap()
    );
    assert!(!super::super::capability::equivalent_requirement(
        &program.requirements[wide_index],
        &program.requirements[narrow_index]
    ));
    let descriptor = NormalizedValue::Function {
        function: program
            .function(declaration_named(&snapshot, "consumer-task"))
            .unwrap(),
        type_arguments: Arc::from([]),
        effect_arguments: Arc::from([]),
        bound_arguments: Some(Arc::new(vec![NormalizedValue::Unit])),
    };
    // Canonical rejection and the two raw activation probes independently narrow the allowance.
    let mut narrowed = snapshot.clone();
    let OwnerRecord::Declaration(owner) = narrowed
        .owners
        .get_mut(&OwnerKey::Declaration(accept.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &mut owner.payload else {
        unreachable!()
    };
    f.effect = FunctionEffect::Task {
        requirements: vec![narrow],
        effect_parameters: vec![],
    };
    assert!(
        crate::platform::kernel::validate_full(&narrowed)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "kernel_type_task_requirement"),
        "narrow calling context must fail universal validation"
    );
    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let error = super::super::reference::NormalizedReferenceInterpreter::new(
        &narrowed,
        &program,
        NormalizedRunPolicy::default(),
    )
    .invoke(
        accept,
        vec![descriptor.clone()],
        Some(&capabilities),
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert!(error.message.contains("allowance"), "{error:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    let function = program.function(accept).unwrap();
    let target = &mut Arc::make_mut(&mut program.functions)[function.0 as usize];
    target.effect = FunctionEffect::Task {
        requirements: vec![narrow],
        effect_parameters: vec![],
    };
    target.task_requirements =
        Arc::from([super::super::value::RequirementIndex(narrow_index as u32)]);
    let (capabilities, calls) = bind_fixture_capability(&program, 1);
    let error = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .invoke(
            accept,
            vec![descriptor],
            Some(&capabilities),
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
    assert!(error.message.contains("allowance"), "{error:?}");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[test]
fn concrete_requirement_coverage_preserves_package_interface_operations_and_limits() {
    let snapshot = library_composition();
    let program = prepare_snapshot(&snapshot);
    let required = program.requirements[0].clone();
    let OwnerRecord::Requirement(owner) =
        &snapshot.owners[&OwnerKey::Requirement(required.reference.requirement)]
    else {
        unreachable!()
    };
    for (case, expected) in [
        ("same", true),
        ("narrower-limit", true),
        ("foreign-package", false),
        ("same-spelling-foreign-interface", false),
        ("missing-operation", false),
        ("wider-limit", false),
        ("wrong-unit", false),
        ("missing-limit", false),
    ] {
        let mut available = required.clone();
        let mut candidate = owner.clone();
        let mut required_owner = owner.clone();
        let limit = crate::platform::kernel::ResourceLimit {
            name: Name::new("maximum_calls").unwrap(),
            maximum: 1000,
            unit: crate::platform::kernel::ResourceUnit::Calls,
        };
        candidate.limits = vec![limit.clone()];
        required_owner.limits = vec![limit.clone()];
        available.limits = Arc::from([limit]);
        let mut required = required.clone();
        required.limits = available.limits.clone();
        match case {
            "same" => {}
            "narrower-limit" => {
                candidate.limits[0].maximum = 999;
                Arc::make_mut(&mut available.limits)[0].maximum = 999;
            }
            "foreign-package" => {
                available.reference.package = crate::platform::kernel::PackageId::migrate(SEED, 99);
            }
            "same-spelling-foreign-interface" => {
                available.interface.package = crate::platform::kernel::PackageId::migrate(SEED, 99);
                candidate.interface = available.interface;
            }
            "missing-operation" => {
                candidate.operations.clear();
                available.operations = Arc::from([]);
            }
            "wider-limit" => {
                candidate.limits[0].maximum = 1001;
                Arc::make_mut(&mut available.limits)[0].maximum = 1001;
            }
            "wrong-unit" => {
                candidate.limits[0].unit = crate::platform::kernel::ResourceUnit::Bytes;
                Arc::make_mut(&mut available.limits)[0].unit =
                    crate::platform::kernel::ResourceUnit::Bytes;
            }
            "missing-limit" => {
                candidate.limits.clear();
                available.limits = Arc::from([]);
            }
            _ => unreachable!(),
        }
        assert_eq!(
            crate::platform::kernel::requirement_is_covered_by(
                required.reference.package,
                &required_owner,
                available.reference.package,
                &candidate
            ),
            expected,
            "canonical {case}"
        );
        assert_eq!(
            super::super::capability::equivalent_requirement(&required, &available),
            expected,
            "runtime {case}"
        );
    }
}

#[test]
fn an_empty_task_row_requires_task_context_in_both_evaluators() {
    let (mut snapshot, accept, old_type) = task_input_fixture();
    let TypeForm::TaskFunction {
        parameters, result, ..
    } = snapshot.types[&old_type].form.clone()
    else {
        unreachable!()
    };
    let task_type = admit_snapshot_type(
        &mut snapshot,
        TypeForm::TaskFunction {
            parameters,
            result,
            effect: EffectRow::default(),
        },
    );
    let OwnerRecord::Declaration(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(accept.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Task {
        requirements: vec![],
        effect_parameters: vec![],
    };
    let parameter = function.parameters[0];
    let OwnerRecord::Parameter(parameter) = snapshot
        .owners
        .get_mut(&OwnerKey::Parameter(parameter))
        .unwrap()
    else {
        unreachable!()
    };
    parameter.ty = task_type;
    snapshot.types.remove(&old_type);
    let mut program = prepare_snapshot(&snapshot);
    let value = NormalizedValue::Function {
        function: program
            .function(declaration_named(&snapshot, "empty-task-prefix"))
            .unwrap(),
        type_arguments: Arc::from([]),
        effect_arguments: Arc::from([]),
        bound_arguments: Some(Arc::new(vec![NormalizedValue::Unit])),
    };
    assert!(
        NormalizedVm::new(&program, NormalizedRunPolicy::default())
            .invoke(
                accept,
                vec![value.clone()],
                None,
                &ExecutionControl::uncancelled()
            )
            .is_ok()
    );
    assert!(
        super::super::reference::NormalizedReferenceInterpreter::new(
            &snapshot,
            &program,
            NormalizedRunPolicy::default()
        )
        .invoke(
            accept,
            vec![value.clone()],
            None,
            &ExecutionControl::uncancelled()
        )
        .is_ok()
    );
    let OwnerRecord::Declaration(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(accept.declaration))
        .unwrap()
    else {
        unreachable!()
    };
    owner.header.kind = OwnerKind::PureFunction;
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        unreachable!()
    };
    function.effect = FunctionEffect::Pure;
    assert!(
        crate::platform::kernel::validate_full(&snapshot)
            .unwrap_err()
            .iter()
            .any(|error| error.code == "kernel_type_pure_task_call")
    );
    let error = super::super::reference::NormalizedReferenceInterpreter::new(
        &snapshot,
        &program,
        NormalizedRunPolicy::default(),
    )
    .invoke(
        accept,
        vec![value.clone()],
        None,
        &ExecutionControl::uncancelled(),
    )
    .unwrap_err();
    assert!(error.message.contains("task context"), "{error:?}");
    let index = program.function(accept).unwrap();
    let target = &mut Arc::make_mut(&mut program.functions)[index.0 as usize];
    target.effect = FunctionEffect::Pure;
    target.pure_graph = true;
    let error = NormalizedVm::new(&program, NormalizedRunPolicy::default())
        .invoke(accept, vec![value], None, &ExecutionControl::uncancelled())
        .unwrap_err();
    assert!(error.message.contains("task calling context"), "{error:?}");
}
