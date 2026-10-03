//! Test-only multi-parameter extension of the controlled ownership workload.
use super::*;
use crate::platform::kernel::KernelSnapshot;

pub(crate) fn snapshot(nested: bool) -> KernelSnapshot {
    extend(type_generic_borrowed_snapshot(nested))
}

fn extend(mut snapshot: KernelSnapshot) -> KernelSnapshot {
    let seed = b"resource-suffix-workload";
    let helpers = snapshot
        .owners
        .iter()
        .filter_map(|(key, record)| {
            let OwnerRecord::Declaration(record) = record else {
                return None;
            };
            let DeclarationPayload::Function(function) = &record.payload else {
                return None;
            };
            let parameter = function.parameters.last()?;
            let Some(OwnerRecord::Parameter(record)) =
                snapshot.owners.get(&OwnerKey::Parameter(*parameter))
            else {
                return None;
            };
            record
                .resource_requirement
                .map(|_| (*key, function.clone(), record.clone()))
        })
        .collect::<Vec<_>>();
    let mut ordinal = 0;
    let mut declarations = BTreeSet::new();
    for (index, (key, mut function, mut parameter)) in helpers.into_iter().enumerate() {
        let OwnerKey::Declaration(declaration) = key else {
            unreachable!()
        };
        declarations.insert(declaration);
        let extra = ParameterId::migrate(seed, index as u64);
        parameter.header = OwnerHeader::new(OwnerKey::Parameter(extra), OwnerKind::Parameter);
        parameter.name = Name::new("extra-view").unwrap();
        assert_eq!(parameter.use_mode, ParameterUse::Borrow);
        let requirement = parameter.resource_requirement.unwrap();
        assert!(
            snapshot
                .owners
                .insert(
                    OwnerKey::Parameter(extra),
                    OwnerRecord::Parameter(parameter)
                )
                .is_none()
        );
        function.parameters.push(extra);
        let local = expression(
            &mut snapshot,
            &mut ordinal,
            ExpressionOperation::Local {
                value: LocalValueReference::FunctionParameter(extra),
            },
        );
        let package = snapshot.root.package_id;
        let borrowed = expression(
            &mut snapshot,
            &mut ordinal,
            ExpressionOperation::CapabilityCall {
                requirement: requirement.into(),
                operation: OperationReference {
                    package,
                    operation: OperationId::migrate(b"iteration-owned-resource", 5),
                },
                arguments: vec![local],
            },
        );
        function.body = expression(
            &mut snapshot,
            &mut ordinal,
            ExpressionOperation::Sequence {
                items: vec![borrowed, function.body],
            },
        );
        let OwnerRecord::Declaration(record) = snapshot.owners.get_mut(&key).unwrap() else {
            unreachable!()
        };
        record.payload = DeclarationPayload::Function(function);
    }
    let calls = snapshot
        .owners
        .iter()
        .filter_map(|(key, record)| {
            let OwnerRecord::Expression(record) = record else {
                return None;
            };
            let ExpressionOperation::Call {
                function,
                arguments,
                ..
            } = &record.operation
            else {
                return None;
            };
            declarations
                .contains(&function.declaration)
                .then(|| (*key, *arguments.last().unwrap()))
        })
        .collect::<Vec<_>>();
    for (key, last) in calls {
        let OwnerRecord::Expression(record) = &snapshot.owners[&OwnerKey::Expression(last)] else {
            unreachable!()
        };
        let ExpressionOperation::Local { value } = record.operation else {
            panic!("resource local")
        };
        let extra = expression(
            &mut snapshot,
            &mut ordinal,
            ExpressionOperation::Local { value },
        );
        let OwnerRecord::Expression(record) = snapshot.owners.get_mut(&key).unwrap() else {
            unreachable!()
        };
        let ExpressionOperation::Call { arguments, .. } = &mut record.operation else {
            unreachable!()
        };
        arguments.push(extra);
    }
    snapshot.root.owners = MapRoot::from_parts(
        snapshot.root.owners.page(),
        snapshot.owners.len() as u64,
        snapshot.root.owners.content(),
    );
    snapshot
}

fn expression(
    snapshot: &mut KernelSnapshot,
    ordinal: &mut u64,
    operation: ExpressionOperation,
) -> ExpressionId {
    *ordinal += 1;
    let id = ExpressionId::migrate(b"resource-suffix-workload", *ordinal);
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

#[test]
fn resource_suffix_both_evaluators_preserve_effect_order_and_join_failures() {
    for mode in ["borrow", "borrow-reborrow", "borrow-prefix-failure"] {
        let (original, _) = fixture(&format!("generic:{mode}"));
        let snapshot = extend(original);
        let program = prepare_snapshot(&snapshot);
        let target = program.root_target(&Name::new("command").unwrap()).unwrap();
        let req = &program.requirements
            [program.components[target.component.0 as usize].requirements[0].0 as usize];
        let operations = req
            .operations
            .iter()
            .map(|op| program.operations[op.0 as usize].reference)
            .collect::<BTreeSet<_>>();
        for reference in [false, true] {
            for (maximum_calls, cancelled) in [(3, false), (5, false), (16, false), (16, true)] {
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
                            fail_prefix: mode == "borrow-prefix-failure",
                            cancel_on_borrow: cancelled,
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
                        .map(|(value, _)| value)
                } else {
                    NormalizedVm::for_test(&program, Default::default())
                        .invoke_root_target_scoped(
                            &Name::new("command").unwrap(),
                            vec![],
                            Some(&capabilities),
                            &resources,
                            &control,
                        )
                        .map(|(value, _)| value)
                };
                let mut expected = vec![("acquire".into(), 0), ("prefix".into(), 1)];
                if mode != "borrow-prefix-failure" {
                    expected.push(("borrow".into(), 1));
                    if !cancelled {
                        expected.push(("observe".into(), 1));
                        if mode == "borrow-reborrow" {
                            expected.push(("borrow".into(), 1));
                        }
                        expected.push(("borrow".into(), 1));
                        expected.push(("consume".into(), 1));
                    }
                }
                let succeeds = mode != "borrow-prefix-failure"
                    && !cancelled
                    && maximum_calls as usize >= expected.len();
                expected.truncate(maximum_calls as usize);
                assert_eq!(
                    *events.lock().unwrap(),
                    expected,
                    "{mode}/{reference}/{maximum_calls}/{cancelled}"
                );
                assert_eq!(
                    result.is_ok(),
                    succeeds,
                    "{mode}/{reference}/{maximum_calls}/{cancelled}: {result:?}"
                );
                if let Ok(value) = result {
                    assert_eq!(value, NormalizedValue::Unit);
                }
                assert_eq!(resources.live_resources(), 0);
                resources.release_all();
                assert_eq!(
                    *events.lock().unwrap(),
                    expected,
                    "cleanup must not replay effects"
                );
            }
        }
    }
}
