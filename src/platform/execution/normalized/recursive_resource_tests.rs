//! Finite recursive activations around independently observed resource effects.
use super::*;
use crate::platform::kernel::KernelSnapshot;

#[path = "recursive_resource_fuel_tests.rs"]
mod fuel;

pub(crate) fn snapshot(mode: &str, tail: bool) -> KernelSnapshot {
    let mutual = matches!(mode, "mutual" | "consume-mutual");
    let (mut snapshot, _) = fixture(match mode {
        "borrow" => "generic:borrow",
        "consume" => "generic:handoff",
        "mutual" | "consume-mutual" => "generic:borrow-reborrow",
        _ => panic!("recursive resource mode"),
    });
    let first = declaration_named(&snapshot, "resource-observer");
    let helpers = if mutual {
        vec![
            first,
            declaration_named(&snapshot, "nested-resource-observer"),
        ]
    } else {
        vec![first]
    };
    let old_calls = snapshot
        .owners
        .iter()
        .filter_map(|(key, owner)| {
            let OwnerRecord::Expression(record) = owner else {
                return None;
            };
            let ExpressionOperation::Call { function, .. } = &record.operation else {
                return None;
            };
            helpers
                .contains(function)
                .then_some((*key, record.clone(), *function))
        })
        .collect::<Vec<_>>();
    let mut serial = 0;
    for (key, mut record, function) in old_calls {
        let flag = expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::Bool {
                value: function != first,
            },
        );
        let ExpressionOperation::Call { arguments, .. } = &mut record.operation else {
            unreachable!()
        };
        arguments.insert(0, flag);
        snapshot.owners.insert(key, OwnerRecord::Expression(record));
    }
    if mode == "consume-mutual" {
        consume_mutually(&mut snapshot, &helpers);
    }
    let boolean = admit_snapshot_type(&mut snapshot, TypeForm::Bool);
    for (index, helper) in helpers.iter().enumerate() {
        let OwnerRecord::Declaration(mut owner) =
            snapshot.owners[&OwnerKey::Declaration(helper.declaration)].clone()
        else {
            panic!("resource helper");
        };
        let DeclarationPayload::Function(function) = &mut owner.payload else {
            panic!("resource function");
        };
        let flag = ParameterId::migrate(b"recursive-resource-control", index as u64);
        let next = expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::Bool {
                value: !mutual || index == 1,
            },
        );
        let mut arguments = vec![next];
        for parameter in &function.parameters {
            arguments.push(expression(
                &mut snapshot,
                &mut serial,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(*parameter),
                },
            ));
        }
        let type_arguments = function
            .type_parameters
            .iter()
            .map(|parameter| {
                admit_snapshot_type(
                    &mut snapshot,
                    TypeForm::TypeParameter {
                        parameter: *parameter,
                    },
                )
            })
            .collect();
        let callee = if mutual { helpers[1 - index] } else { *helper };
        let call = expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::Call {
                function: callee,
                type_arguments,
                effect_arguments: vec![],
                requirement_arguments: vec![],
                arguments,
            },
        );
        let recursive = if tail {
            call
        } else {
            let returned = expression(
                &mut snapshot,
                &mut serial,
                ExpressionOperation::Local {
                    value: LocalValueReference::FunctionParameter(function.parameters[0]),
                },
            );
            expression(
                &mut snapshot,
                &mut serial,
                ExpressionOperation::Sequence {
                    items: vec![call, returned],
                },
            )
        };
        let condition = expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(flag),
            },
        );
        function.body = expression(
            &mut snapshot,
            &mut serial,
            ExpressionOperation::If {
                condition,
                when_true: function.body,
                when_false: recursive,
            },
        );
        function.parameters.insert(0, flag);
        snapshot.owners.insert(
            OwnerKey::Parameter(flag),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(OwnerKey::Parameter(flag), OwnerKind::Parameter),
                parent: ParameterParent::Function(helper.declaration),
                name: Name::new("base").unwrap(),
                ty: boolean,
                use_mode: ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
        snapshot.owners.insert(
            OwnerKey::Declaration(helper.declaration),
            OwnerRecord::Declaration(owner),
        );
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

fn consume_mutually(snapshot: &mut KernelSnapshot, helpers: &[DeclarationReference]) {
    let operation = |name: &str| {
        snapshot
            .owners
            .iter()
            .find_map(|(key, owner)| {
                let (OwnerKey::Operation(id), OwnerRecord::Operation(record)) = (key, owner) else {
                    return None;
                };
                (record.name.as_str() == name).then_some(OperationReference {
                    package: snapshot.root.package_id,
                    operation: *id,
                })
            })
            .unwrap()
    };
    let borrow = operation("borrow");
    let consume = operation("consume");
    let caller_sequences = snapshot.owners.iter().filter_map(|(key, owner)| {
        let OwnerRecord::Expression(record) = owner else { return None; };
        let ExpressionOperation::Sequence { items } = &record.operation else { return None; };
        if items.len() != 2 { return None; }
        let OwnerRecord::Expression(first) = &snapshot.owners[&OwnerKey::Expression(items[0])] else {
            return None;
        };
        let OwnerRecord::Expression(last) = &snapshot.owners[&OwnerKey::Expression(items[1])] else {
            return None;
        };
        (matches!(&first.operation, ExpressionOperation::Call { function, .. } if *function == helpers[0])
            && matches!(&last.operation, ExpressionOperation::CapabilityCall { operation, .. } if *operation == consume))
            .then_some(*key)
    }).collect::<Vec<_>>();
    assert_eq!(caller_sequences.len(), 1);
    for (key, owner) in &mut snapshot.owners {
        match owner {
            OwnerRecord::Parameter(parameter) if parameter.resource_requirement.is_some() => {
                parameter.use_mode = ParameterUse::Consume;
            }
            OwnerRecord::Expression(record) => match &mut record.operation {
                ExpressionOperation::CapabilityCall { operation, .. } if *operation == borrow => {
                    *operation = consume;
                }
                ExpressionOperation::Sequence { items } if caller_sequences.contains(key) => {
                    items.truncate(1);
                }
                _ => {}
            },
            _ => {}
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
            let OwnerRecord::Expression(record) = &snapshot.owners[&OwnerKey::Expression(id)]
            else {
                panic!("expression");
            };
            pending.extend(record.children().into_iter().map(|child| child.expression));
        }
    }
    snapshot
        .owners
        .retain(|key, _| !matches!(key, OwnerKey::Expression(id) if !reachable.contains(id)));
}

fn expression(
    snapshot: &mut KernelSnapshot,
    serial: &mut u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    *serial += 1;
    let id = ExpressionId::migrate(b"recursive-resource-activation", *serial);
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

struct RecursiveScript {
    inner: ResourceScript,
    fault: &'static str,
}

impl NormalizedCapabilityAdapter for RecursiveScript {
    fn kind(&self) -> NormalizedAdapterKind {
        self.inner.kind()
    }
    fn interface(&self) -> DeclarationReference {
        self.inner.interface()
    }
    fn operations(&self) -> &BTreeSet<OperationReference> {
        self.inner.operations()
    }
    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        let result = self.inner.call(policy, arguments, resources, control)?;
        if policy.operation_name.as_str() == "observe" {
            match self.fault {
                "cancel" => control.cancel(),
                "trap" => {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Trap,
                        "recursive_resource_trap",
                        "recursive base operation failed",
                    ));
                }
                _ => {}
            }
        }
        Ok(result)
    }
}

#[test]
fn recursive_resource_activations_join_success_failure_and_cancellation_in_both_engines() {
    for mode in ["borrow", "consume", "mutual", "consume-mutual"] {
        for tail in [false, true] {
            let snapshot = snapshot(mode, tail);
            let program = prepare_snapshot(&snapshot);
            let target = program.root_target(&Name::new("command").unwrap()).unwrap();
            let requirement = &program.requirements
                [program.components[target.component.0 as usize].requirements[0].0 as usize];
            let operations = requirement
                .operations
                .iter()
                .map(|op| program.operations[op.0 as usize].reference)
                .collect::<BTreeSet<_>>();
            for reference in [false, true] {
                for fault in [
                    "none",
                    "quota",
                    "trap",
                    "cancel",
                    "resource",
                    "depth",
                    "borrow-cancel",
                ] {
                    if (fault == "depth" && tail)
                        || (fault == "borrow-cancel" && mode.starts_with("consume"))
                    {
                        continue;
                    }
                    let label = format!("{mode}/tail={tail}/reference={reference}/{fault}");
                    let maximum_calls = if fault == "quota" {
                        if mode.starts_with("consume") { 3 } else { 4 }
                    } else {
                        10
                    };
                    let events = Arc::new(Mutex::new(Vec::new()));
                    let capabilities = NormalizedCapabilities::bind(
                        &program,
                        target.component,
                        vec![NormalizedCapabilityGrant {
                            requirement: requirement.reference,
                            descriptor: exact_grant_descriptor(
                                requirement,
                                operations.clone(),
                                exact_grant_limits(requirement, maximum_calls),
                            ),
                            adapter: Arc::new(RecursiveScript {
                                inner: ResourceScript {
                                    interface: requirement.interface,
                                    operations: operations.clone(),
                                    events: Arc::clone(&events),
                                    exhaust: fault == "resource",
                                    fail_prefix: false,
                                    cancel_on_borrow: fault == "borrow-cancel",
                                    variant: None,
                                },
                                fault,
                            }),
                        }],
                    )
                    .unwrap();
                    let resources = NormalizedResourceScope::with_test_limit(1);
                    let control = ExecutionControl::uncancelled();
                    let policy = NormalizedRunPolicy {
                        maximum_call_depth: if fault == "depth" {
                            if mode.starts_with("consume") { 1 } else { 2 }
                        } else {
                            32
                        },
                        ..Default::default()
                    };
                    let result = if reference {
                        NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                            .invoke_root_target_scoped(
                                &Name::new("command").unwrap(),
                                vec![],
                                Some(&capabilities),
                                &resources,
                                &control,
                            )
                            .map(|(value, observation)| (value, observation.maximum_call_depth))
                    } else {
                        NormalizedVm::for_test(&program, policy)
                            .invoke_root_target_scoped(
                                &Name::new("command").unwrap(),
                                vec![],
                                Some(&capabilities),
                                &resources,
                                &control,
                            )
                            .map(|(value, observation)| (value, observation.maximum_call_depth))
                    };
                    let mut expected = vec![("acquire".into(), 0), ("prefix".into(), 1)];
                    if fault != "depth" {
                        expected.push(("observe".into(), 1));
                        if matches!(fault, "none" | "quota" | "borrow-cancel")
                            && !mode.starts_with("consume")
                        {
                            expected.push(("borrow".into(), 1));
                        }
                        if fault == "none" {
                            expected.push(("consume".into(), 1));
                        }
                    }
                    assert_eq!(*events.lock().unwrap(), expected, "{label}");
                    if fault == "none" {
                        let (value, depth) =
                            result.unwrap_or_else(|error| panic!("{label}: {error:?}"));
                        assert_eq!(value, NormalizedValue::Unit, "{label}");
                        assert!(depth <= 6, "{label}: {depth}");
                    } else {
                        let error = result.expect_err(&label);
                        if matches!(fault, "cancel" | "borrow-cancel") {
                            // Both scripted operations declare possible external visibility.
                            // Resource borrowing does not make an operation observationally pure.
                            assert_eq!(
                                error.class,
                                ExecutionFailureClass::PossibleVisibility,
                                "{label}"
                            );
                        }
                        if fault == "trap" {
                            assert_eq!(error.code, "recursive_resource_trap", "{label}");
                        }
                        if fault == "resource" {
                            assert_eq!(error.code, "normalized_resource_limit", "{label}");
                        }
                        if fault == "depth" {
                            assert!(error.code.contains("call_depth"), "{label}: {error:?}");
                        }
                    }
                    assert_eq!(resources.live_resources(), 0, "{label}");
                    resources.release_all();
                    assert_eq!(
                        *events.lock().unwrap(),
                        expected,
                        "cleanup replayed an effect: {label}"
                    );
                }
            }
        }
    }
}
