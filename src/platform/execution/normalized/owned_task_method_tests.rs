//! Runtime ownership and exact effect observations.
use super::owned_task_method_fixture::{capabilities, controlled_capabilities, expected, source};
use super::*;

#[test]
fn owned_task_method_both_engines_preserve_values_allocations_and_exact_effect_counts() {
    let source = source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "method-main");
    for reference in [false, true] {
        for n in [i64::MIN, -257, 0, i64::MAX] {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let buffers = super::super::byte_buffer::StorageObservation::start();
            let (caps, calls) = capabilities(&program);
            let control = ExecutionControl::uncancelled();
            let args = vec![NormalizedValue::I64(n)];
            let value = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, args, Some(&caps), &control)
                .unwrap()
                .0
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, args, Some(&caps), &control)
                    .unwrap()
                    .0
            };
            expected(value, n);
            assert_eq!(calls.load(Ordering::Relaxed), 2);
            assert_eq!(cells.created(), 1);
            assert_eq!(buffers.created(), 1);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
    }
}

#[test]
fn owned_task_method_cancellation_quota_and_missing_grants_never_leak_or_replay() {
    let source = source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "method-main");
    for reference in [false, true] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let run = |policy, caps: Option<&NormalizedCapabilities>, control: &ExecutionControl| {
            let args = vec![NormalizedValue::I64(i64::MIN)];
            if reference {
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(entry, args, caps, control)
                    .map(|v| v.0)
            } else {
                NormalizedVm::new(&program, policy)
                    .invoke(entry, args, caps, control)
                    .map(|v| v.0)
            }
        };
        assert!(
            run(
                NormalizedRunPolicy::foreground(),
                None,
                &ExecutionControl::uncancelled()
            )
            .is_err()
        );
        let (caps, calls) = controlled_capabilities(&program, true);
        let error = run(
            NormalizedRunPolicy::foreground(),
            Some(&caps),
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::execution::ExecutionFailureClass::Cancelled
        );
        assert_eq!(calls.load(Ordering::Relaxed), 1);
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
        let mut observed = [false; 4];
        for checks in (0..128).step_by(8).chain([100_000]) {
            let (caps, calls) = capabilities(&program);
            match run(
                NormalizedRunPolicy::foreground(),
                Some(&caps),
                &ExecutionControl::cancel_after_checks(checks),
            ) {
                Ok(value) => {
                    expected(value, i64::MIN);
                    observed[3] = true;
                }
                Err(error) => {
                    assert_eq!(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Cancelled
                    );
                    observed[calls.load(Ordering::Relaxed) as usize] = true;
                }
            }
            assert!(calls.load(Ordering::Relaxed) <= 2);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
        assert!(observed[0] && observed[3], "{observed:?}");
        let mut outcomes = [false; 2];
        for quota in [0, 64, 1024, 16384, 65536, 1048576] {
            let (caps, calls) = capabilities(&program);
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: Some(quota),
                ..NormalizedRunPolicy::foreground()
            };
            match run(policy, Some(&caps), &ExecutionControl::uncancelled()) {
                Ok(value) => {
                    expected(value, i64::MIN);
                    outcomes[0] = true;
                }
                Err(error) => {
                    assert_eq!(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Resource
                    );
                    outcomes[1] = true;
                }
            }
            assert!(calls.load(Ordering::Relaxed) <= 2);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
        assert_eq!(outcomes, [true, true]);
    }
}
