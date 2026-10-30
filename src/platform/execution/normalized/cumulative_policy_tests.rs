//! Exact cumulative admission is checked independently by both evaluators.
use super::*;

#[test]
fn cumulative_policy_boundaries_are_exact_in_vm_and_reference() {
    let snapshot = crate::platform::compiler::tests::complete_expression_snapshot();
    let program = prepare_snapshot(&snapshot);
    let caller = declaration_named(&snapshot, "caller");
    for reference in [false, true] {
        let run = |policy| {
            let (capabilities, calls) = bind_fixture_capability(&program, u64::MAX);
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
                    .invoke(caller, vec![], Some(&capabilities), &control)
                    .map(|(value, work)| {
                        (
                            value,
                            [
                                work.expressions,
                                work.allocated_bytes,
                                work.collection_items,
                                work.capability_calls,
                            ],
                            [
                                work.expressions_saturated,
                                work.allocated_bytes_saturated,
                                work.collection_items_saturated,
                                work.capability_calls_saturated,
                            ],
                        )
                    })
            } else {
                NormalizedVm::for_test(&program, policy)
                    .invoke(caller, vec![], Some(&capabilities), &control)
                    .map(|(value, work)| {
                        (
                            value,
                            [
                                work.instructions,
                                work.allocated_bytes,
                                work.collection_items,
                                work.capability_calls,
                            ],
                            [
                                work.instructions_saturated,
                                work.allocated_bytes_saturated,
                                work.collection_items_saturated,
                                work.capability_calls_saturated,
                            ],
                        )
                    })
            };
            (result, calls.load(Ordering::Relaxed))
        };
        let (baseline, baseline_calls) = run(NormalizedRunPolicy::foreground());
        let (expected, measured, flags) = baseline.unwrap();
        assert!(measured.into_iter().all(|count| count > 0));
        assert_eq!(flags, [false; 4]);
        assert!(baseline_calls > 0);
        for (dimension, exact) in measured.into_iter().enumerate() {
            for maximum in [None, Some(0), Some(exact - 1), Some(exact), Some(u64::MAX)] {
                let mut policy = NormalizedRunPolicy::foreground();
                match dimension {
                    0 => policy.instruction_steps = maximum,
                    1 => policy.maximum_allocated_bytes = maximum,
                    2 => policy.maximum_collection_items = maximum,
                    _ => policy.maximum_capability_calls = maximum,
                }
                let (result, calls) = run(policy);
                if maximum.is_none_or(|maximum| maximum >= exact) {
                    let (value, work, flags) = result.unwrap();
                    assert_eq!(
                        value, expected,
                        "reference={reference} dimension={dimension} maximum={maximum:?}"
                    );
                    assert_eq!(work, measured);
                    assert_eq!(flags, [false; 4]);
                    assert_eq!(calls, baseline_calls);
                } else {
                    let error = result.unwrap_err();
                    assert_eq!(error.class, ExecutionFailureClass::Resource);
                    let expected_code = if reference {
                        [
                            "normalized_reference_expression_steps",
                            "normalized_reference_allocation",
                            "normalized_reference_collection_items",
                            "normalized_reference_capability_calls",
                        ]
                    } else {
                        [
                            "normalized_instruction_steps",
                            "normalized_allocation",
                            "normalized_collection_items",
                            "normalized_capability_calls",
                        ]
                    }[dimension];
                    assert_eq!(
                        error.code, expected_code,
                        "reference={reference} dimension={dimension} maximum={maximum:?}"
                    );
                    if maximum == Some(0) {
                        assert_eq!(calls, 0, "zero admission must precede adapter work");
                    }
                }
            }
        }
    }
}

#[test]
fn zero_collection_and_capability_policies_admit_pure_scalar_work() {
    let snapshot = byte_buffer_tests::author_only(
        "declarations.begin\n(units (module create zero-policy\n(function create scalar (visibility public) (effect pure) (returns I64) (body (i64 7)))))\ndeclarations.end\n",
    )
    .unwrap();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "scalar");
    let policy = NormalizedRunPolicy {
        maximum_collection_items: Some(0),
        maximum_capability_calls: Some(0),
        ..NormalizedRunPolicy::foreground()
    };
    let control = ExecutionControl::uncancelled();
    let (vm, vm_work) = NormalizedVm::for_test(&program, policy)
        .invoke(declaration, vec![], None, &control)
        .unwrap();
    let (reference, reference_work) =
        NormalizedReferenceInterpreter::new(&snapshot, &program, policy)
            .invoke(declaration, vec![], None, &control)
            .unwrap();
    assert_eq!(vm, NormalizedValue::I64(7));
    assert_eq!(reference, vm);
    assert_eq!((vm_work.collection_items, vm_work.capability_calls), (0, 0));
    assert_eq!(
        (
            reference_work.collection_items,
            reference_work.capability_calls
        ),
        (0, 0)
    );
    assert!(!vm_work.collection_items_saturated);
    assert!(!vm_work.capability_calls_saturated);
    assert!(!reference_work.collection_items_saturated);
    assert!(!reference_work.capability_calls_saturated);
}
