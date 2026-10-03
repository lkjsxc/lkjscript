//! Nonterminating tail calls still charge work and join invocation resources.
use super::*;

#[test]
fn recursive_resource_tail_cycles_exhaust_work_without_replaying_effects() {
    for mode in ["borrow", "consume"] {
        let mut snapshot = snapshot(mode, true);
        let helper = declaration_named(&snapshot, "resource-observer");
        let flags = snapshot
            .owners
            .values()
            .filter_map(|owner| {
                let OwnerRecord::Expression(record) = owner else {
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
                (*function == helper).then(|| arguments[0])
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(flags.len(), 2);
        for flag in flags {
            let OwnerRecord::Expression(record) = snapshot
                .owners
                .get_mut(&OwnerKey::Expression(flag))
                .unwrap()
            else {
                panic!("call flag");
            };
            assert!(matches!(record.operation, ExpressionOperation::Bool { .. }));
            record.operation = ExpressionOperation::Bool { value: false };
        }
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
            let label = format!("{mode}/reference={reference}");
            let events = Arc::new(Mutex::new(Vec::new()));
            let capabilities = NormalizedCapabilities::bind(
                &program,
                target.component,
                vec![NormalizedCapabilityGrant {
                    requirement: requirement.reference,
                    descriptor: exact_grant_descriptor(
                        requirement,
                        operations.clone(),
                        exact_grant_limits(requirement, 10),
                    ),
                    adapter: Arc::new(ResourceScript {
                        interface: requirement.interface,
                        operations: operations.clone(),
                        events: Arc::clone(&events),
                        exhaust: false,
                        fail_prefix: false,
                        cancel_on_borrow: false,
                        variant: None,
                    }),
                }],
            )
            .unwrap();
            let resources = NormalizedResourceScope::with_test_limit(1);
            let control = ExecutionControl::uncancelled();
            let policy = NormalizedRunPolicy {
                instruction_steps: Some(256),
                maximum_call_depth: 4,
                ..Default::default()
            };
            let error = if reference {
                NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                    .invoke_root_target_scoped(
                        &Name::new("command").unwrap(),
                        vec![],
                        Some(&capabilities),
                        &resources,
                        &control,
                    )
                    .expect_err("bounded expression work must stop recursive borrowing/transfer")
            } else {
                NormalizedVm::for_test(&program, policy)
                    .invoke_root_target_scoped(
                        &Name::new("command").unwrap(),
                        vec![],
                        Some(&capabilities),
                        &resources,
                        &control,
                    )
                    .expect_err("bounded instruction work must stop recursive borrowing/transfer")
            };
            assert_eq!(
                error.code,
                if reference {
                    "normalized_reference_expression_steps"
                } else {
                    "normalized_instruction_steps"
                },
                "{label}"
            );
            let expected = vec![("acquire".into(), 0), ("prefix".into(), 1)];
            assert_eq!(*events.lock().unwrap(), expected, "{label}");
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
