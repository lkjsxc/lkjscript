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
        &snapshot.owners[&OwnerKey::Requirement(requirement.requirement)]
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
        .get_mut(&OwnerKey::Requirement(requirement.requirement))
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
                resource_requirement: Some(requirement),
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
            "acquire" => Ok(NormalizedValue::Resource(
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
            )),
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
