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

#[test]
fn parallel_nested_non_tail_witness_calls_count_ancestor_frames() {
    // The helper makes no ordinary call that could independently mask a missing
    // ancestor-depth check. Its let-bound application cannot become a tail call.
    let source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create witness-depth
  (external create new (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (owned-contract create Marker (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_89000000000000000000000000000001 marker
      (parameters) (returns I64)))
  (function create marker (visibility public) (returns I64) (effect pure) (body (i64 7)))
  (owned-implementation create Cell (visibility public) (contract Marker) (self OwnedI64Cell)
    (method method_89000000000000000000000000000001 marker))
  (function create helper (visibility public) (effect (task))
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_89000000000000000000000000000001 ops Marker T)
    (parameter create value (type T) (use consume)) (returns I64) (body (i64 7)))
  (function create leaf (visibility public) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64)
    (body (let (binding result (type I64)
      (implementation-call helper (types OwnedI64Cell)
        (implementations concrete@Cell) (local value))) (in (local result)))))
  (function create idle (visibility public) (effect (task)) (returns I64) (body (i64 0)))
  (function create nested (visibility public) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns (record (left I64) (right I64)))
    (body (parallel (call leaf (local value)) (call idle))))
  (function create root (visibility public) (effect (task))
    (returns (record (left (record (left I64) (right I64))) (right I64)))
    (body (let (binding value (type OwnedI64Cell) (call new (i64 42)))
      (in (parallel (call nested (local value)) (call idle))))))))
declarations.end
"#,
    )
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "root");
    let record = |left, right| {
        NormalizedValue::Record(
            crate::platform::execution::normalized::value::NormalizedRecord::Structural {
                fields: Arc::new(vec![
                    (Name::new("left").unwrap(), left),
                    (Name::new("right").unwrap(), right),
                ]),
            },
        )
    };
    let expected = record(
        record(NormalizedValue::I64(7), NormalizedValue::I64(0)),
        NormalizedValue::I64(0),
    );
    for reference in [false, true] {
        for maximum_call_depth in [3, 4] {
            let cells = Cells::start();
            let control = ExecutionControl::uncancelled();
            let policy = NormalizedRunPolicy {
                maximum_call_depth,
                ..NormalizedRunPolicy::foreground()
            };
            let result = if reference {
                let observed = Mutex::new(None);
                let result = NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .observing_checked(&observed)
                    .invoke(entry, vec![], None, &control);
                let observed = observed.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                result.map(|(value, work)| (value, work.maximum_call_depth))
            } else {
                let observed = Mutex::new(None);
                let result = NormalizedVm::for_test(&program, policy)
                    .observing_checked(&observed)
                    .invoke(entry, vec![], None, &control);
                let observed = observed.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_operands_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                result.map(|(value, work)| (value, work.maximum_call_depth))
            };
            if maximum_call_depth == 3 {
                let error = result.expect_err("root, nested, leaf and helper require four frames");
                assert_eq!(error.class, ExecutionFailureClass::Resource);
                assert!(error.code.ends_with("call_depth"), "{error:?}");
            } else {
                assert_eq!(result.unwrap(), (expected.clone(), 4));
            }
            assert_eq!(cells.created(), 1);
            assert_eq!(cells.live(), (0, 0));
        }
    }
}
