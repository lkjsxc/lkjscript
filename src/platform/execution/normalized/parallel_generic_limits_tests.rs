//! Exact aggregate limits include generic metadata and returned custody.
use super::*;
use crate::platform::execution::ExecutionFailureClass;
use crate::platform::execution::normalized::owned_product::StorageObservation as Products;

#[test]
fn parallel_generic_application_shares_exact_quota_and_cleans_cancelled_owners() {
    let source = byte_buffer_tests::author_only(&witnessed_source()).unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "owned");
    for reference in [false, true] {
        let run = |policy: NormalizedRunPolicy, control: &ExecutionControl| {
            let cells = Cells::start();
            let products = Products::start();
            let arguments = vec![NormalizedValue::I64(-137)];
            let result = if reference {
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(entry, arguments, None, control)
                    .map(|(value, work)| {
                        (
                            value,
                            work.expressions,
                            work.allocated_bytes,
                            work.collection_items,
                        )
                    })
            } else {
                NormalizedVm::for_test(&program, policy)
                    .invoke(entry, arguments, None, control)
                    .map(|(value, work)| {
                        (
                            value,
                            work.instructions,
                            work.allocated_bytes,
                            work.collection_items,
                        )
                    })
            };
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(products.live(), (0, 0));
            // This fixture transfers cells; its joined product is created only in the parent.
            if result.is_ok() {
                assert_eq!(cells.created(), 2);
            }
            result
        };
        let (value, steps, bytes, items) = run(
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        let exact = NormalizedRunPolicy {
            instruction_steps: Some(steps),
            maximum_allocated_bytes: Some(bytes),
            maximum_collection_items: Some(items),
            ..NormalizedRunPolicy::foreground()
        };
        assert_eq!(
            run(exact, &ExecutionControl::uncancelled()).unwrap(),
            (value.clone(), steps, bytes, items)
        );
        for policy in [
            NormalizedRunPolicy {
                instruction_steps: Some(steps - 1),
                ..exact
            },
            NormalizedRunPolicy {
                maximum_allocated_bytes: Some(bytes - 1),
                ..exact
            },
            NormalizedRunPolicy {
                maximum_collection_items: Some(items - 1),
                ..exact
            },
        ] {
            assert_eq!(
                run(policy, &ExecutionControl::uncancelled())
                    .unwrap_err()
                    .class,
                ExecutionFailureClass::Resource
            );
        }
        let mut rejected = 0;
        let mut completed = 0;
        for checks in [0, 16, 32, 64, 128, 256, 512, 1_024, 4_096, 32_768] {
            match run(exact, &ExecutionControl::cancel_after_checks(checks)) {
                Ok((actual, ..)) => {
                    assert_eq!(actual, value);
                    completed += 1;
                }
                Err(error) => {
                    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                    rejected += 1;
                }
            }
        }
        assert!(rejected > 0 && completed > 0);
    }
}
