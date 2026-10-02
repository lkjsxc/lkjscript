//! Same-invocation task ownership: independent values, allocations and every exit.
use super::super::{byte_buffer, owned_i64_cell, owned_product};
use super::*;
use crate::platform::kernel::KernelSnapshot;

pub(super) fn input() -> String {
    [
        include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
        include_str!("../../../../tests/fixtures/owned-choices-outcome.lkjc"),
        include_str!("../../../../tests/fixtures/owned-task-library.lkjc"),
        include_str!("../../../../tests/fixtures/owned-witness-cell.lkjc"),
        include_str!("../../../../tests/fixtures/owned-witness-buffer.lkjc"),
        include_str!("../../../../tests/fixtures/owned-task-consumer.lkjc"),
    ]
    .join("\n")
}

pub(super) fn source() -> KernelSnapshot {
    let source = byte_buffer_tests::author_only(&input()).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    source
}

fn expected(value: NormalizedValue, n: i64, accepted: bool) {
    let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) =
        value
    else {
        panic!("expected a structural result");
    };
    assert_eq!(fields.len(), 4);
    let actual = fields
        .iter()
        .map(|(name, value)| (name.as_str(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        actual,
        BTreeMap::from([
            ("value", NormalizedValue::I64(n)),
            (
                "alternate",
                NormalizedValue::I64(if accepted { 99 } else { n })
            ),
            ("bytes", NormalizedValue::I64(1)),
            ("product", NormalizedValue::I64(n)),
        ])
    );
}

#[test]
fn owned_task_named_helpers_keep_exact_witness_values_and_allocations() {
    let source = source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "task-main");
    for reference in [false, true] {
        for n in [i64::MIN, -257, 0, i64::MAX] {
            for accepted in [false, true] {
                let cells = owned_i64_cell::StorageObservation::start();
                let buffers = byte_buffer::StorageObservation::start();
                let composites = owned_product::StorageObservation::start();
                let args = vec![NormalizedValue::I64(n), NormalizedValue::Bool(accepted)];
                let control = ExecutionControl::uncancelled();
                let (value, frames) = if reference {
                    let (value, work) = NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, args, None, &control)
                    .unwrap();
                    (value, work.live_call_frames_after)
                } else {
                    let (value, work) =
                        NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                            .invoke(entry, args, None, &control)
                            .unwrap();
                    assert_eq!(work.live_operands_after, 0);
                    (value, work.live_call_frames_after)
                };
                expected(value, n, accepted);
                assert_eq!(frames, 0);
                assert_eq!(cells.live(), (0, 0));
                assert_eq!(buffers.live(), (0, 0));
                assert_eq!(composites.live(), (0, 0));
                assert_eq!(cells.created(), 3);
                assert_eq!(buffers.created(), 1);
                assert_eq!(composites.created(), 4);
                composites.assert_transfers_preserve_allocations();
            }
        }
    }
}

#[test]
fn owned_task_cancel_and_quota_failures_release_all_custodians() {
    let source = source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "task-main");
    for reference in [false, true] {
        for accepted in [false, true] {
            let cells = owned_i64_cell::StorageObservation::start();
            let buffers = byte_buffer::StorageObservation::start();
            let composites = owned_product::StorageObservation::start();
            let run = |policy, control: &ExecutionControl| {
                let args = vec![
                    NormalizedValue::I64(i64::MIN),
                    NormalizedValue::Bool(accepted),
                ];
                if reference {
                    NormalizedReferenceInterpreter::new(&source, &program, policy)
                        .invoke(entry, args, None, control)
                        .map(|v| v.0)
                } else {
                    NormalizedVm::new(&program, policy)
                        .invoke(entry, args, None, control)
                        .map(|v| v.0)
                }
            };
            let mut counts = [0, 0];
            for checks in (0..4096).step_by(16).chain([100_000]) {
                match run(
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::cancel_after_checks(checks),
                ) {
                    Ok(value) => {
                        expected(value, i64::MIN, accepted);
                        counts[0] += 1;
                    }
                    Err(error) => {
                        assert_eq!(
                            error.class,
                            crate::platform::execution::ExecutionFailureClass::Cancelled
                        );
                        counts[1] += 1;
                    }
                }
                assert_eq!(cells.live(), (0, 0));
                assert_eq!(buffers.live(), (0, 0));
                assert_eq!(composites.live(), (0, 0));
            }
            assert!(counts.iter().all(|n| *n > 0));
            counts = [0, 0];
            for quota in [0, 16, 64, 256, 1024, 4096, 16384, 65536, 1048576] {
                let policy = NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(quota),
                    ..NormalizedRunPolicy::foreground()
                };
                match run(policy, &ExecutionControl::uncancelled()) {
                    Ok(value) => {
                        expected(value, i64::MIN, accepted);
                        counts[0] += 1;
                    }
                    Err(error) => {
                        assert_eq!(
                            error.class,
                            crate::platform::execution::ExecutionFailureClass::Resource
                        );
                        counts[1] += 1;
                    }
                }
                assert_eq!(cells.live(), (0, 0));
                assert_eq!(buffers.live(), (0, 0));
                assert_eq!(composites.live(), (0, 0));
            }
            assert!(counts.iter().all(|n| *n > 0));
            expected(
                run(
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled(),
                )
                .unwrap(),
                i64::MIN,
                accepted,
            );
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
    }
}
