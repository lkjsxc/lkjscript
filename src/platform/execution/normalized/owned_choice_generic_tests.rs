//! Generic choice payloads compose with products without copying their owned storage.
use super::*;

#[test]
fn owned_choice_closed_self_witnesses_recursive_transfer_and_reborrow() {
    let literal = include_str!("../../../../tests/fixtures/owned-choices-witness.lkjc");
    let source = byte_buffer_tests::author_only(literal).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let composites = super::super::owned_product::StorageObservation::start();
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        for n in [i64::MIN, i64::MAX] {
            for accepted in [false, true] {
                let args = vec![NormalizedValue::I64(n), NormalizedValue::Bool(accepted)];
                let entry = declaration_named(&source, "selected");
                let control = ExecutionControl::uncancelled();
                let value = if reference {
                    NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, args, None, &control)
                    .unwrap()
                    .0
                } else {
                    NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                        .invoke(entry, args, None, &control)
                        .unwrap()
                        .0
                };
                let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
                    fields,
                }) = value
                else {
                    panic!("ordinary choice witness result");
                };
                assert_eq!(
                    fields.as_ref(),
                    &[
                        (Name::new("alternate").unwrap(), NormalizedValue::I64(99)),
                        (Name::new("scalar").unwrap(), NormalizedValue::I64(n)),
                        (Name::new("skipped").unwrap(), NormalizedValue::I64(7)),
                    ]
                );
                assert_eq!(composites.live(), (0, 0));
                assert_eq!(cells.live(), (0, 0));
            }
        }
    }
    let wrong_kind = literal.replace(
        "create alternate (visibility public) (effect pure)",
        "create alternate (visibility public) (effect (task))",
    );
    assert!(
        byte_buffer_tests::author_only(&wrong_kind)
            .unwrap_err()
            .contains("kernel_")
    );
}
const INPUT: &str = include_str!("../../../../tests/fixtures/owned-choices.lkjc");

#[test]
fn owned_choice_generics_preserve_nested_allocations_and_independent_arm_scope() {
    let source = byte_buffer_tests::author_only(INPUT).unwrap();
    crate::platform::kernel::validate_full(&source).unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    let mut examples = vec![
        ("accepted", vec![], NormalizedValue::I64(42)),
        ("abandoned", vec![], NormalizedValue::Unit),
    ];
    for n in [i64::MIN, -128, 0, 128, i64::MAX] {
        for inner in [false, true] {
            for outer in [false, true] {
                examples.push((
                    "main",
                    vec![
                        NormalizedValue::I64(n),
                        NormalizedValue::Bool(inner),
                        NormalizedValue::Bool(outer),
                    ],
                    NormalizedValue::I64(n),
                ));
            }
        }
    }
    for n in [0, 127, 128, 255] {
        for left in [false, true] {
            examples.push((
                "bytes",
                vec![NormalizedValue::I64(n), NormalizedValue::Bool(left)],
                NormalizedValue::bytes(vec![n as u8]),
            ));
        }
    }
    for reference in [false, true] {
        for (name, arguments, expected) in &examples {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let bytes = super::super::byte_buffer::StorageObservation::start();
            let composites = super::super::owned_product::StorageObservation::start();
            let entry = declaration_named(&source, name);
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, arguments.clone(), None, &control)
                .unwrap()
                .0
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, arguments.clone(), None, &control)
                    .unwrap()
                    .0
            };
            assert_eq!(&result, expected, "{name} reference={reference}");
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(bytes.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
            if *name != "abandoned" {
                composites.assert_transfers_preserve_allocations();
            }
            assert_eq!(
                composites.created(),
                if *name == "main" || *name == "abandoned" {
                    3
                } else {
                    1
                }
            );
        }
    }
}

#[test]
fn owned_choice_quota_failure_releases_nested_payloads_in_both_engines() {
    use std::sync::Mutex;
    let source = byte_buffer_tests::author_only(INPUT).unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "abandoned");
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let run = |limit| {
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: limit,
                ..NormalizedRunPolicy::foreground()
            };
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let composites = super::super::owned_product::StorageObservation::start();
            let observer = Mutex::new(None);
            let (result, allocated) = if reference {
                let result = NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .observing_checked(&observer)
                    .invoke(entry, vec![], None, &control);
                let observed = observer.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_transactions_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                (result.map(|pair| pair.0), observed.allocated_bytes)
            } else {
                let observer = Mutex::new(None);
                let result = NormalizedVm::new(&program, policy)
                    .observing_checked(&observer)
                    .invoke(entry, vec![], None, &control);
                let observed = observer.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_operands_after, 0);
                assert_eq!(observed.live_transactions_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                (result.map(|pair| pair.0), observed.allocated_bytes)
            };
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
            (result, allocated, composites.created())
        };
        let (success, total, created) = run(None);
        assert_eq!(success.unwrap(), NormalizedValue::Unit);
        assert_eq!(created, 3);
        assert_eq!(run(Some(total)).0.unwrap(), NormalizedValue::Unit);
        let mut stages = std::collections::BTreeSet::new();
        // Sweep every charged byte: failures before, during and after each composite.
        for limit in 1..total {
            let (result, _, created) = run(Some(limit));
            assert_eq!(
                result.unwrap_err().class,
                crate::platform::execution::ExecutionFailureClass::Resource
            );
            stages.insert(created);
        }
        assert_eq!(stages, [0, 1, 2, 3].into_iter().collect());
    }
}
