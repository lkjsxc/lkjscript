//! Frame elimination cannot complete, close, refund, or duplicate invocation-owned rights.
use super::*;
use crate::platform::kernel::{BindingKind, BindingRecord, OperationReference, ParameterUse};
use crate::platform::semantic_id::BindingId;

fn fixture(mode: &str) -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let handoff = mode.starts_with("handoff");
    let mut snapshot = crate::platform::kernel::tests::witness_snapshot();
    let package = snapshot.root.package_id;
    let caller = declaration_named(&snapshot, "caller");
    let OwnerRecord::Declaration(mut helper) =
        snapshot.owners[&OwnerKey::Declaration(caller.declaration)].clone()
    else {
        panic!("caller")
    };
    let DeclarationPayload::Function(ref function) = helper.payload else {
        panic!("function")
    };
    let requirement = function.effect.row().requirements[0];
    let OwnerRecord::Requirement(requirement_record) =
        &snapshot.owners[&OwnerKey::Requirement(requirement.concrete().unwrap().requirement)]
    else {
        panic!("requirement")
    };
    let interface = requirement_record.interface;
    let unit = function.result;
    let resource = admit_snapshot_type(&mut snapshot, TypeForm::CapabilityResource { interface });
    let seed = b"iteration-owned-resource";
    let relay = DeclarationId::migrate(seed, 1);
    let binding = BindingId::migrate(seed, 1);
    let final_parameter = ParameterId::migrate(seed, 1);
    let consume_parameter = ParameterId::migrate(seed, 2);
    let prefix_parameter = ParameterId::migrate(seed, 3);
    let acquire = OperationId::migrate(seed, 1);
    let observe = OperationId::migrate(seed, 2);
    let consume = OperationId::migrate(seed, 3);
    let prefix = OperationId::migrate(seed, 4);
    for (operation, name, result, parameters) in [
        (acquire, "acquire", resource, vec![]),
        (observe, "observe", unit, vec![]),
        (consume, "consume", unit, vec![consume_parameter]),
        (prefix, "prefix", unit, vec![]),
    ] {
        snapshot.owners.insert(
            OwnerKey::Operation(operation),
            OwnerRecord::Operation(OperationRecord {
                header: OwnerHeader::new(OwnerKey::Operation(operation), OwnerKind::Operation),
                declaration: interface.declaration,
                name: Name::new(name).unwrap(),
                parameters,
                result,
                idempotency: Idempotency::NonIdempotent,
                external_visibility: ExternalVisibility::Possible,
            }),
        );
    }
    snapshot.owners.insert(
        OwnerKey::Parameter(consume_parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(consume_parameter), OwnerKind::Parameter),
            parent: ParameterParent::Operation(consume),
            name: Name::new("right").unwrap(),
            ty: resource,
            use_mode: ParameterUse::Consume,
            resource_requirement: None,
        }),
    );
    let OwnerRecord::Declaration(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(interface.declaration))
        .unwrap()
    else {
        panic!("interface")
    };
    let DeclarationPayload::Interface { operations } = &mut owner.payload else {
        panic!("interface payload")
    };
    operations.extend([acquire, observe, consume, prefix]);
    operations.sort();
    let OwnerRecord::Requirement(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Requirement(
            requirement.concrete().unwrap().requirement,
        ))
        .unwrap()
    else {
        panic!("requirement")
    };
    owner.operations.extend(
        [acquire, observe, consume, prefix]
            .map(|operation| OperationReference { package, operation }),
    );
    owner.operations.sort();
    let mut ordinal = 0;
    let mut expression = |snapshot: &mut crate::platform::kernel::KernelSnapshot, operation| {
        ordinal += 1;
        let id = ExpressionId::migrate(seed, ordinal);
        snapshot.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
        );
        id
    };
    let acquired = expression(
        &mut snapshot,
        ExpressionOperation::CapabilityCall {
            requirement,
            operation: OperationReference {
                package,
                operation: acquire,
            },
            arguments: vec![],
        },
    );
    snapshot.owners.insert(
        OwnerKey::Binding(binding),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(binding), OwnerKind::Binding),
            name: Name::new("unused-or-transferred").unwrap(),
            kind: BindingKind::Let,
            value: Some(acquired),
            declared_type: Some(resource),
        }),
    );
    let observed = expression(
        &mut snapshot,
        ExpressionOperation::CapabilityCall {
            requirement,
            operation: OperationReference {
                package,
                operation: observe,
            },
            arguments: vec![],
        },
    );
    let local = expression(
        &mut snapshot,
        ExpressionOperation::Local {
            value: LocalValueReference::LexicalBinding(binding),
        },
    );
    let helper_body = if handoff {
        let handed = expression(
            &mut snapshot,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(final_parameter),
            },
        );
        let consumed = expression(
            &mut snapshot,
            ExpressionOperation::CapabilityCall {
                requirement,
                operation: OperationReference {
                    package,
                    operation: consume,
                },
                arguments: vec![handed],
            },
        );
        expression(
            &mut snapshot,
            ExpressionOperation::Sequence {
                items: vec![observed, consumed],
            },
        )
    } else {
        observed
    };
    helper.header = OwnerHeader::new(OwnerKey::Declaration(relay), OwnerKind::TaskFunction);
    helper.name = Name::new("resource-observer").unwrap();
    helper.visibility = DeclarationVisibility::Private;
    let DeclarationPayload::Function(ref mut function) = helper.payload else {
        panic!("function")
    };
    function.body = helper_body;
    function.parameters = if handoff {
        vec![prefix_parameter, final_parameter]
    } else {
        vec![]
    };
    snapshot.owners.insert(
        OwnerKey::Declaration(relay),
        OwnerRecord::Declaration(helper),
    );
    if handoff {
        snapshot.owners.insert(
            OwnerKey::Parameter(prefix_parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::Parameter(prefix_parameter),
                    OwnerKind::Parameter,
                ),
                parent: ParameterParent::Function(relay),
                name: Name::new("prefix").unwrap(),
                ty: unit,
                use_mode: ParameterUse::Unrestricted,
                resource_requirement: None,
            }),
        );
        snapshot.owners.insert(
            OwnerKey::Parameter(final_parameter),
            OwnerRecord::Parameter(ParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::Parameter(final_parameter),
                    OwnerKind::Parameter,
                ),
                parent: ParameterParent::Function(relay),
                name: Name::new("right").unwrap(),
                ty: resource,
                use_mode: ParameterUse::Consume,
                resource_requirement: Some(requirement.concrete().unwrap()),
            }),
        );
    }
    let arguments = if handoff {
        let prefix = expression(
            &mut snapshot,
            ExpressionOperation::CapabilityCall {
                requirement,
                operation: OperationReference {
                    package,
                    operation: prefix,
                },
                arguments: vec![],
            },
        );
        vec![prefix, local]
    } else {
        vec![]
    };
    let call = expression(
        &mut snapshot,
        ExpressionOperation::Call {
            requirement_arguments: Vec::new(),
            function: DeclarationReference {
                package,
                declaration: relay,
            },
            type_arguments: vec![],
            effect_arguments: vec![],
            arguments,
        },
    );
    let body = if mode == "consumed" {
        let consumed = expression(
            &mut snapshot,
            ExpressionOperation::CapabilityCall {
                requirement,
                operation: OperationReference {
                    package,
                    operation: consume,
                },
                arguments: vec![local],
            },
        );
        expression(
            &mut snapshot,
            ExpressionOperation::Sequence {
                items: vec![consumed, call],
            },
        )
    } else {
        call
    };
    let body = expression(
        &mut snapshot,
        ExpressionOperation::Let {
            bindings: vec![binding],
            body,
        },
    );
    let OwnerRecord::Declaration(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(caller.declaration))
        .unwrap()
    else {
        panic!("caller")
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        panic!("function")
    };
    function.body = body;
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
                panic!("expression")
            };
            pending.extend(record.children().into_iter().map(|child| child.expression));
        }
    }
    snapshot
        .owners
        .retain(|key, _| !matches!(key,OwnerKey::Expression(id) if !reachable.contains(id)));
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    let program = prepare_snapshot(&snapshot);
    (snapshot, program)
}

struct ResourceScript {
    interface: DeclarationReference,
    operations: BTreeSet<OperationReference>,
    events: Arc<Mutex<Vec<(String, usize)>>>,
    exhaust: bool,
    fail_prefix: bool,
    cancel_on_borrow: bool,
    variant: Option<(super::super::value::VariantLayoutIndex, u32)>,
}
impl NormalizedCapabilityAdapter for ResourceScript {
    fn kind(&self) -> NormalizedAdapterKind {
        NormalizedAdapterKind::Configuration
    }
    fn interface(&self) -> DeclarationReference {
        self.interface
    }
    fn operations(&self) -> &BTreeSet<OperationReference> {
        &self.operations
    }
    fn call(
        &self,
        policy: &NormalizedCallPolicy,
        arguments: Vec<NormalizedValue>,
        resources: &NormalizedResourceScope,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        control.check()?;
        self.events.lock().unwrap().push((
            policy.operation_name.to_string(),
            resources.live_resources(),
        ));
        match policy.operation_name.as_str() {
            "prefix" => {
                if self.fail_prefix {
                    return Err(ExecutionError::new(
                        ExecutionFailureClass::Trap,
                        "iteration_prefix_failure",
                        "unrestricted argument failed before final consume",
                    ));
                }
                Ok(NormalizedValue::Unit)
            }
            "acquire" => {
                let value = NormalizedValue::Resource(
                    resources
                        .reserve_queue_lease(policy.grant_requirement, self.interface)?
                        .commit(crate::platform::queue::JobLease {
                            job_id: "owned-job".into(),
                            attempt_id: "owned-attempt".into(),
                            worker_id: "owned-worker".into(),
                            payload: vec![],
                            attempt_number: 1,
                            lease_until_milliseconds: 10,
                        })?,
                );
                Ok(match self.variant {
                    Some((layout, case)) => NormalizedValue::Variant {
                        layout,
                        case,
                        payload: Some(Box::new(value)),
                    },
                    None => value,
                })
            }
            "borrow" => {
                let [NormalizedValue::Resource(right)] = arguments.as_slice() else {
                    panic!("borrow resource");
                };
                let lease = resources.borrow_queue_lease(
                    policy.grant_requirement,
                    self.interface,
                    *right,
                )?;
                assert_eq!(lease.job_id, "owned-job");
                if self.cancel_on_borrow {
                    control.cancel();
                }
                Ok(NormalizedValue::Unit)
            }
            "observe" => {
                if self.exhaust {
                    resources.reserve_queue_lease(policy.grant_requirement, self.interface)?;
                }
                Ok(NormalizedValue::Unit)
            }
            "consume" => {
                let [NormalizedValue::Resource(right)] = arguments.as_slice() else {
                    panic!("resource argument")
                };
                resources.consume_queue_lease(policy.grant_requirement, self.interface, *right)?;
                Ok(NormalizedValue::Unit)
            }
            _ => panic!("unexpected resource operation"),
        }
    }
}

#[test]
fn task_tail_transfer_preserves_unused_consumed_and_final_handoff_resource_lifetimes() {
    for mode in ["unused", "consumed", "handoff", "handoff-failure"] {
        let (snapshot, program) = fixture(mode);
        for reference in [false, true] {
            for exhaust in [false, true] {
                let target = program.root_target(&Name::new("command").unwrap()).unwrap();
                let req = &program.requirements
                    [program.components[target.component.0 as usize].requirements[0].0 as usize];
                let operations = req
                    .operations
                    .iter()
                    .map(|op| program.operations[op.0 as usize].reference)
                    .collect::<BTreeSet<_>>();
                let events = Arc::new(Mutex::new(Vec::new()));
                let capabilities = NormalizedCapabilities::bind(
                    &program,
                    target.component,
                    vec![NormalizedCapabilityGrant {
                        requirement: req.reference,
                        descriptor: exact_grant_descriptor(
                            req,
                            operations.clone(),
                            exact_grant_limits(req, 10),
                        ),
                        adapter: Arc::new(ResourceScript {
                            interface: req.interface,
                            operations,
                            events: Arc::clone(&events),
                            exhaust,
                            fail_prefix: mode == "handoff-failure",
                            cancel_on_borrow: false,
                            variant: None,
                        }),
                    }],
                )
                .unwrap();
                let resources = NormalizedResourceScope::with_test_limit(1);
                let control = ExecutionControl::uncancelled();
                let result = if reference {
                    NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                        .invoke_root_target_scoped(
                            &Name::new("command").unwrap(),
                            vec![],
                            Some(&capabilities),
                            &resources,
                            &control,
                        )
                        .map(|(v, w)| (v, w.maximum_call_depth))
                } else {
                    NormalizedVm::new(&program, Default::default())
                        .invoke_root_target_scoped(
                            &Name::new("command").unwrap(),
                            vec![],
                            Some(&capabilities),
                            &resources,
                            &control,
                        )
                        .map(|(v, w)| (v, w.maximum_call_depth))
                };
                let expected = if mode == "consumed" {
                    vec![
                        ("acquire".into(), 0),
                        ("consume".into(), 1),
                        ("observe".into(), 0),
                    ]
                } else if mode == "handoff-failure" {
                    vec![("acquire".into(), 0), ("prefix".into(), 1)]
                } else if mode == "handoff" && !exhaust {
                    vec![
                        ("acquire".into(), 0),
                        ("prefix".into(), 1),
                        ("observe".into(), 1),
                        ("consume".into(), 1),
                    ]
                } else if mode == "handoff" {
                    vec![
                        ("acquire".into(), 0),
                        ("prefix".into(), 1),
                        ("observe".into(), 1),
                    ]
                } else {
                    vec![("acquire".into(), 0), ("observe".into(), 1)]
                };
                assert_eq!(
                    *events.lock().unwrap(),
                    expected,
                    "{mode}/{reference}/{exhaust}"
                );
                if mode == "handoff-failure" {
                    assert_eq!(result.unwrap_err().code, "iteration_prefix_failure");
                    assert_eq!(resources.live_resources(), 0);
                } else if exhaust && mode != "consumed" {
                    assert_eq!(result.unwrap_err().code, "normalized_resource_limit");
                    assert_eq!(resources.live_resources(), 0);
                } else {
                    let (value, depth) = result.unwrap();
                    assert_eq!(value, NormalizedValue::Unit);
                    assert!(depth <= 2);
                    assert_eq!(resources.live_resources(), usize::from(mode == "unused"));
                }
                resources.release_all();
                assert_eq!(resources.live_resources(), 0);
                assert_eq!(
                    *events.lock().unwrap(),
                    expected,
                    "owner cleanup must not perform a queue completion or failure"
                );
            }
        }
    }
}

/// Turn the existing resource workload into two ordinary requirement-generic helpers. The
/// concrete entry closes both applications, without changing the deterministic adapter.
pub(crate) fn requirement_snapshot(cross_operand: bool) -> crate::platform::kernel::KernelSnapshot {
    use crate::platform::kernel::{
        RequirementConstraint, RequirementOperand, RequirementParameterRecord,
        RequirementParameterReference,
    };
    use crate::platform::semantic_id::RequirementParameterId;
    let (mut snapshot, _) = fixture("consumed");
    add_requirement_match_and_borrow(&mut snapshot);
    let package = snapshot.root.package_id;
    let entry = declaration_named(&snapshot, "caller");
    let relay = declaration_named(&snapshot, "resource-observer");
    let generic = DeclarationId::migrate(b"requirement-local-resource", 1);
    let OwnerRecord::Declaration(mut library) =
        snapshot.owners[&OwnerKey::Declaration(entry.declaration)].clone()
    else {
        panic!("entry");
    };
    let DeclarationPayload::Function(function) = &library.payload else {
        panic!("function");
    };
    let concrete = function.effect.row().requirements[0];
    let OwnerRecord::Requirement(requirement) = snapshot.owners[&concrete.owner()].clone() else {
        panic!("requirement");
    };
    library.header = OwnerHeader::new(OwnerKey::Declaration(generic), OwnerKind::TaskFunction);
    library.name = Name::new("locally-consumed-library").unwrap();
    library.visibility = DeclarationVisibility::Private;
    snapshot.owners.insert(
        OwnerKey::Declaration(generic),
        OwnerRecord::Declaration(library),
    );
    let mut formals = BTreeMap::new();
    for (index, function_id) in [generic, relay.declaration].into_iter().enumerate() {
        let parameter =
            RequirementParameterId::migrate(b"requirement-local-resource", index as u64);
        let operand =
            RequirementOperand::Parameter(RequirementParameterReference { package, parameter });
        snapshot.owners.insert(
            OwnerKey::RequirementParameter(parameter),
            OwnerRecord::RequirementParameter(RequirementParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::RequirementParameter(parameter),
                    OwnerKind::RequirementParameter,
                ),
                declaration: function_id,
                name: Name::new("R").unwrap(),
                constraint: RequirementConstraint {
                    interface: requirement.interface,
                    operations: requirement.operations.clone(),
                },
            }),
        );
        let OwnerRecord::Declaration(owner) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(function_id))
            .unwrap()
        else {
            panic!("generic function");
        };
        let DeclarationPayload::Function(function) = &mut owner.payload else {
            panic!("function");
        };
        function.requirement_parameters = vec![parameter];
        function.effect = FunctionEffect::Task {
            requirements: vec![operand],
            effect_parameters: vec![],
        };
        let mut pending = vec![function.body];
        while let Some(id) = pending.pop() {
            let OwnerRecord::Expression(expression) = &snapshot.owners[&OwnerKey::Expression(id)]
            else {
                panic!("expression");
            };
            if let ExpressionOperation::Let { bindings, .. } = &expression.operation {
                for binding in bindings {
                    let OwnerRecord::Binding(binding) =
                        &snapshot.owners[&OwnerKey::Binding(*binding)]
                    else {
                        panic!("let binding");
                    };
                    pending.extend(binding.value);
                }
            }
            let OwnerRecord::Expression(expression) =
                snapshot.owners.get_mut(&OwnerKey::Expression(id)).unwrap()
            else {
                panic!("expression");
            };
            pending.extend(
                expression
                    .children()
                    .into_iter()
                    .map(|child| child.expression),
            );
            match &mut expression.operation {
                ExpressionOperation::CapabilityCall { requirement, .. } => *requirement = operand,
                ExpressionOperation::Call {
                    function,
                    requirement_arguments,
                    ..
                } if *function == relay => *requirement_arguments = vec![operand],
                _ => {}
            }
        }
        formals.insert(function_id, operand);
    }
    let entry_expression = ExpressionId::migrate(b"requirement-local-resource", 1);
    let mut arguments = vec![concrete];
    if cross_operand {
        // The generic proof must reject R1 acquisition / R2 consumption even when this entry
        // deliberately supplies the same concrete requirement for both parameters.
        let parameter = RequirementParameterId::migrate(b"requirement-local-resource", 2);
        let second =
            RequirementOperand::Parameter(RequirementParameterReference { package, parameter });
        snapshot.owners.insert(
            OwnerKey::RequirementParameter(parameter),
            OwnerRecord::RequirementParameter(RequirementParameterRecord {
                header: OwnerHeader::new(
                    OwnerKey::RequirementParameter(parameter),
                    OwnerKind::RequirementParameter,
                ),
                declaration: generic,
                name: Name::new("R2").unwrap(),
                constraint: RequirementConstraint {
                    interface: requirement.interface,
                    operations: requirement.operations.clone(),
                },
            }),
        );
        let OwnerRecord::Declaration(owner) = snapshot
            .owners
            .get_mut(&OwnerKey::Declaration(generic))
            .unwrap()
        else {
            panic!("generic");
        };
        let DeclarationPayload::Function(function) = &mut owner.payload else {
            panic!("function");
        };
        function.requirement_parameters.push(parameter);
        let mut row = vec![formals[&generic], second];
        row.sort();
        function.effect = FunctionEffect::Task {
            requirements: row,
            effect_parameters: vec![],
        };
        let consume = snapshot
            .owners
            .values()
            .find_map(|record| match record {
                OwnerRecord::Operation(operation) if operation.name.as_str() == "consume" => {
                    Some(OperationReference {
                        package,
                        operation: match operation.header.owner {
                            OwnerKey::Operation(id) => id,
                            _ => panic!("operation"),
                        },
                    })
                }
                _ => None,
            })
            .unwrap();
        for owner in snapshot.owners.values_mut() {
            if let OwnerRecord::Expression(expression) = owner
                && let ExpressionOperation::CapabilityCall {
                    requirement,
                    operation,
                    ..
                } = &mut expression.operation
                && *operation == consume
            {
                *requirement = second;
            }
        }
        arguments.push(concrete);
    }
    snapshot.owners.insert(
        OwnerKey::Expression(entry_expression),
        OwnerRecord::Expression(
            ExpressionRecord::new(
                entry_expression,
                ExpressionOperation::Call {
                    function: DeclarationReference {
                        package,
                        declaration: generic,
                    },
                    type_arguments: vec![],
                    effect_arguments: vec![],
                    requirement_arguments: arguments,
                    arguments: vec![],
                },
            )
            .unwrap(),
        ),
    );
    let OwnerRecord::Declaration(owner) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(entry.declaration))
        .unwrap()
    else {
        panic!("entry");
    };
    let DeclarationPayload::Function(function) = &mut owner.payload else {
        panic!("function");
    };
    function.body = entry_expression;
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

#[test]
fn requirement_resource_helpers_close_exact_operands_and_join_both_execution_owners() {
    let snapshot = requirement_snapshot(false);
    let program = prepare_snapshot(&snapshot);
    let target = program.root_target(&Name::new("command").unwrap()).unwrap();
    let req = &program.requirements
        [program.components[target.component.0 as usize].requirements[0].0 as usize];
    let operations = req
        .operations
        .iter()
        .map(|op| program.operations[op.0 as usize].reference)
        .collect::<BTreeSet<_>>();
    let variant = program
        .variants
        .iter()
        .enumerate()
        .find_map(|(index, layout)| {
            layout
                .cases
                .iter()
                .enumerate()
                .find(|(_, case)| case.name.as_str() == "resource-live")
                .map(|(case, _)| {
                    (
                        super::super::value::VariantLayoutIndex(index as u32, program.value_origin),
                        case as u32,
                    )
                })
        })
        .unwrap();
    for reference in [false, true] {
        for (maximum_calls, cancelled) in [(2, false), (3, false), (10, false), (10, true)] {
            let events = Arc::new(Mutex::new(Vec::new()));
            let capabilities = NormalizedCapabilities::bind(
                &program,
                target.component,
                vec![NormalizedCapabilityGrant {
                    requirement: req.reference,
                    descriptor: exact_grant_descriptor(
                        req,
                        operations.clone(),
                        exact_grant_limits(req, maximum_calls),
                    ),
                    adapter: Arc::new(ResourceScript {
                        interface: req.interface,
                        operations: operations.clone(),
                        events: Arc::clone(&events),
                        exhaust: false,
                        fail_prefix: false,
                        cancel_on_borrow: cancelled,
                        variant: Some(variant),
                    }),
                }],
            )
            .unwrap();
            let resources = NormalizedResourceScope::with_test_limit(1);
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&snapshot, &program, Default::default())
                    .invoke_root_target_scoped(
                        &Name::new("command").unwrap(),
                        vec![],
                        Some(&capabilities),
                        &resources,
                        &control,
                    )
                    .map(|(value, _)| value)
            } else {
                NormalizedVm::new(&program, Default::default())
                    .invoke_root_target_scoped(
                        &Name::new("command").unwrap(),
                        vec![],
                        Some(&capabilities),
                        &resources,
                        &control,
                    )
                    .map(|(value, _)| value)
            };
            if cancelled {
                assert_eq!(result.unwrap_err().class, ExecutionFailureClass::Cancelled);
                assert_eq!(
                    *events.lock().unwrap(),
                    vec![("acquire".into(), 0), ("borrow".into(), 1)]
                );
            } else if maximum_calls < 4 {
                assert_eq!(
                    result.unwrap_err().code,
                    if reference {
                        "normalized_reference_grant_calls"
                    } else {
                        "normalized_grant_calls"
                    }
                );
                let mut expected = vec![("acquire".into(), 0), ("borrow".into(), 1)];
                if maximum_calls == 3 {
                    expected.push(("consume".into(), 1));
                }
                assert_eq!(*events.lock().unwrap(), expected);
            } else {
                assert_eq!(result.unwrap(), NormalizedValue::Unit);
                assert_eq!(
                    *events.lock().unwrap(),
                    vec![
                        ("acquire".into(), 0),
                        ("borrow".into(), 1),
                        ("consume".into(), 1),
                        ("observe".into(), 0)
                    ]
                );
            }
            assert_eq!(resources.live_resources(), 0);
        }
    }
}

fn add_requirement_match_and_borrow(snapshot: &mut crate::platform::kernel::KernelSnapshot) {
    use crate::platform::kernel::{CaseRecord, CaseReference, MatchExpressionArm};
    use crate::platform::semantic_id::CaseId;
    let seed = b"requirement-matched-local-resource";
    let caller = declaration_named(snapshot, "caller");
    let package = snapshot.root.package_id;
    let OwnerRecord::Declaration(owner) =
        &snapshot.owners[&OwnerKey::Declaration(caller.declaration)]
    else {
        panic!("caller");
    };
    let module = owner.module;
    let DeclarationPayload::Function(function) = &owner.payload else {
        panic!("function");
    };
    let root = function.body;
    let unit = function.result;
    let requirement = function.effect.row().requirements[0];
    let OwnerRecord::Requirement(record) = &snapshot.owners[&requirement.owner()] else {
        panic!("requirement");
    };
    let interface = record.interface;
    let OwnerRecord::Expression(expression) = &snapshot.owners[&OwnerKey::Expression(root)] else {
        panic!("let");
    };
    let ExpressionOperation::Let { bindings, body } = &expression.operation else {
        panic!("let body");
    };
    let binding = bindings[0];
    let body = *body;
    let OwnerRecord::Binding(record) = &snapshot.owners[&OwnerKey::Binding(binding)] else {
        panic!("binding");
    };
    let resource = record.declared_type.unwrap();
    let acquire = record.value.unwrap();
    let declaration = DeclarationId::migrate(seed, 0);
    let absent = CaseId::migrate(seed, 0);
    let live = CaseId::migrate(seed, 1);
    snapshot.owners.insert(
        OwnerKey::Declaration(declaration),
        OwnerRecord::Declaration(DeclarationRecord {
            header: OwnerHeader::new(OwnerKey::Declaration(declaration), OwnerKind::Variant),
            module,
            name: Name::new("ResourceChoice").unwrap(),
            visibility: DeclarationVisibility::Public,
            payload: DeclarationPayload::Variant {
                type_parameters: vec![],
                cases: vec![absent, live],
            },
        }),
    );
    for (case, name, payload) in [
        (absent, "resource-absent", None),
        (live, "resource-live", Some(resource)),
    ] {
        snapshot.owners.insert(
            OwnerKey::Case(case),
            OwnerRecord::Case(CaseRecord {
                header: OwnerHeader::new(OwnerKey::Case(case), OwnerKind::Case),
                declaration,
                name: Name::new(name).unwrap(),
                payload,
            }),
        );
    }
    let variant = admit_snapshot_type(
        snapshot,
        TypeForm::Named {
            declaration: DeclarationReference {
                package,
                declaration,
            },
        },
    );
    let OwnerRecord::Expression(acquisition) = &snapshot.owners[&OwnerKey::Expression(acquire)]
    else {
        panic!("acquire");
    };
    let ExpressionOperation::CapabilityCall { operation, .. } = acquisition.operation else {
        panic!("acquire operation");
    };
    let OwnerRecord::Operation(operation) = snapshot
        .owners
        .get_mut(&OwnerKey::Operation(operation.operation))
        .unwrap()
    else {
        panic!("operation");
    };
    operation.result = variant;
    let OwnerRecord::Binding(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Binding(binding))
        .unwrap()
    else {
        panic!("binding");
    };
    record.declared_type = Some(variant);
    let payload = BindingId::migrate(seed, 0);
    snapshot.owners.insert(
        OwnerKey::Binding(payload),
        OwnerRecord::Binding(BindingRecord {
            header: OwnerHeader::new(OwnerKey::Binding(payload), OwnerKind::Binding),
            name: Name::new("matched-right").unwrap(),
            kind: BindingKind::MatchPayload,
            value: None,
            declared_type: Some(resource),
        }),
    );
    for record in snapshot.owners.values_mut() {
        if let OwnerRecord::Expression(expression) = record
            && let ExpressionOperation::Local { value } = &mut expression.operation
            && *value == LocalValueReference::LexicalBinding(binding)
        {
            *value = LocalValueReference::MatchPayload(payload);
        }
    }
    let borrowed = OperationId::migrate(seed, 0);
    let parameter = ParameterId::migrate(seed, 0);
    snapshot.owners.insert(
        OwnerKey::Operation(borrowed),
        OwnerRecord::Operation(OperationRecord {
            header: OwnerHeader::new(OwnerKey::Operation(borrowed), OwnerKind::Operation),
            declaration: interface.declaration,
            name: Name::new("borrow").unwrap(),
            parameters: vec![parameter],
            result: unit,
            idempotency: Idempotency::Idempotent,
            external_visibility: ExternalVisibility::None,
        }),
    );
    snapshot.owners.insert(
        OwnerKey::Parameter(parameter),
        OwnerRecord::Parameter(ParameterRecord {
            header: OwnerHeader::new(OwnerKey::Parameter(parameter), OwnerKind::Parameter),
            parent: ParameterParent::Operation(borrowed),
            name: Name::new("right").unwrap(),
            ty: resource,
            use_mode: ParameterUse::Borrow,
            resource_requirement: None,
        }),
    );
    let OwnerRecord::Declaration(record) = snapshot
        .owners
        .get_mut(&OwnerKey::Declaration(interface.declaration))
        .unwrap()
    else {
        panic!("interface");
    };
    let DeclarationPayload::Interface { operations } = &mut record.payload else {
        panic!("interface operations");
    };
    operations.push(borrowed);
    operations.sort();
    let OwnerRecord::Requirement(record) = snapshot.owners.get_mut(&requirement.owner()).unwrap()
    else {
        panic!("requirement");
    };
    record.operations.push(OperationReference {
        package,
        operation: borrowed,
    });
    record.operations.sort();
    let input = ExpressionId::migrate(seed, 0);
    let borrow_value = ExpressionId::migrate(seed, 1);
    let borrow = ExpressionId::migrate(seed, 2);
    let absent_value = ExpressionId::migrate(seed, 3);
    let matched = ExpressionId::migrate(seed, 4);
    for (id, operation) in [
        (
            input,
            ExpressionOperation::Local {
                value: LocalValueReference::LexicalBinding(binding),
            },
        ),
        (
            borrow_value,
            ExpressionOperation::Local {
                value: LocalValueReference::MatchPayload(payload),
            },
        ),
        (
            borrow,
            ExpressionOperation::CapabilityCall {
                requirement,
                operation: OperationReference {
                    package,
                    operation: borrowed,
                },
                arguments: vec![borrow_value],
            },
        ),
        (absent_value, ExpressionOperation::Unit {}),
        (
            matched,
            ExpressionOperation::Match {
                value: input,
                arms: vec![
                    MatchExpressionArm {
                        case: CaseReference {
                            package,
                            case: absent,
                        },
                        payload_binding: None,
                        body: absent_value,
                    },
                    MatchExpressionArm {
                        case: CaseReference {
                            package,
                            case: live,
                        },
                        payload_binding: Some(payload),
                        body,
                    },
                ],
            },
        ),
    ] {
        snapshot.owners.insert(
            OwnerKey::Expression(id),
            OwnerRecord::Expression(ExpressionRecord::new(id, operation).unwrap()),
        );
    }
    let OwnerRecord::Expression(expression) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(body))
        .unwrap()
    else {
        panic!("sequence");
    };
    let ExpressionOperation::Sequence { items } = &mut expression.operation else {
        panic!("sequence body");
    };
    items.insert(0, borrow);
    let OwnerRecord::Expression(expression) = snapshot
        .owners
        .get_mut(&OwnerKey::Expression(root))
        .unwrap()
    else {
        panic!("root");
    };
    let ExpressionOperation::Let { body, .. } = &mut expression.operation else {
        panic!("root let");
    };
    *body = matched;
}
