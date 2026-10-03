//! Value and accounting evidence independent of the private slice descriptor.
use super::*;

#[test]
fn byte_ranges_and_copies_agree_through_checked_and_explicit_core_hosts() {
    use crate::platform::execution::normalized::reference::CoreNormalizedReferenceHost;
    use crate::platform::execution::normalized::vm::CoreNormalizedHost;
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let control = ExecutionControl::uncancelled();
    let root = NormalizedValue::bytes((0..=255).collect::<Vec<u8>>());
    let NormalizedValue::Bytes(payload) = &root else {
        unreachable!()
    };
    let window = NormalizedValue::Bytes(
        payload
            .clone()
            .slice(100, 110, &control, &mut |_| Ok(()))
            .unwrap(),
    );
    for (name, arguments, expected) in [
        (
            "slice",
            vec![
                root.clone(),
                NormalizedValue::I64(127),
                NormalizedValue::I64(130),
            ],
            NormalizedValue::bytes([127, 128, 129]),
        ),
        (
            "slice",
            vec![
                window.clone(),
                NormalizedValue::I64(2),
                NormalizedValue::I64(5),
            ],
            NormalizedValue::bytes([102, 103, 104]),
        ),
        ("copy", vec![root.clone()], root.clone()),
        (
            "copy",
            vec![window.clone()],
            NormalizedValue::bytes((100..110).collect::<Vec<u8>>()),
        ),
    ] {
        let declaration = declaration_named(&snapshot, name);
        for explicit in [false, true] {
            let vm_observer = std::sync::Mutex::new(None);
            let reference_observer = std::sync::Mutex::new(None);
            let vm = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground());
            let vm = if explicit {
                vm.observing(&vm_observer, &CoreNormalizedHost)
            } else {
                vm
            };
            let reference = NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                NormalizedRunPolicy::foreground(),
            );
            let reference = if explicit {
                reference.observing(&reference_observer, &CoreNormalizedReferenceHost)
            } else {
                reference
            };
            let actual = vm
                .invoke(declaration, arguments.clone(), None, &control)
                .unwrap()
                .0;
            let independent = reference
                .invoke(declaration, arguments.clone(), None, &control)
                .unwrap()
                .0;
            assert_eq!(actual, expected);
            assert_eq!(independent, expected);
            if name == "copy" {
                let NormalizedValue::Bytes(input) = &arguments[0] else {
                    unreachable!()
                };
                let NormalizedValue::Bytes(output) = &actual else {
                    unreachable!()
                };
                assert_ne!(input.as_ptr(), output.as_ptr());
            }
        }
    }
    assert_eq!(root, NormalizedValue::bytes((0..=255).collect::<Vec<u8>>()));
}

fn observed(
    snapshot: &crate::platform::kernel::KernelSnapshot,
    name: &str,
    arguments: Vec<NormalizedValue>,
    maximum: Option<u64>,
    reference: bool,
) -> (Result<NormalizedValue, ExecutionError>, u64, u64) {
    let program = prepare_snapshot(snapshot);
    let declaration = declaration_named(snapshot, name);
    let mut policy = NormalizedRunPolicy::foreground();
    policy.maximum_allocated_bytes = maximum;
    let control = ExecutionControl::uncancelled();
    if reference {
        let observer = std::sync::Mutex::new(None);
        let result = NormalizedReferenceInterpreter::new(snapshot, &program, policy)
            .observing_checked(&observer)
            .invoke(declaration, arguments, None, &control)
            .map(|pair| pair.0);
        let observation = observer.into_inner().unwrap().unwrap();
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_transactions_after, 0);
        assert_eq!(observation.live_handles_after, 0);
        (
            result,
            observation.allocated_bytes,
            observation.external_calls,
        )
    } else {
        let observer = std::sync::Mutex::new(None);
        let result = NormalizedVm::for_test(&program, policy)
            .observing_checked(&observer)
            .invoke(declaration, arguments, None, &control)
            .map(|pair| pair.0);
        let observation = observer.into_inner().unwrap().unwrap();
        assert_eq!(observation.live_call_frames_after, 0);
        assert_eq!(observation.live_transactions_after, 0);
        assert_eq!(observation.live_handles_after, 0);
        (
            result,
            observation.allocated_bytes,
            observation.external_calls,
        )
    }
}

#[test]
fn byte_range_intrinsic_budget_boundaries_retain_joined_cleanup() {
    let snapshot = fixture();
    let input = NormalizedValue::bytes(vec![255; 2048]);
    for reference in [false, true] {
        for (name, arguments, expected) in [
            (
                "slice",
                vec![
                    input.clone(),
                    NormalizedValue::I64(1),
                    NormalizedValue::I64(2047),
                ],
                NormalizedValue::bytes(vec![255; 2046]),
            ),
            ("copy", vec![input.clone()], input.clone()),
        ] {
            let (result, total, calls) =
                observed(&snapshot, name, arguments.clone(), None, reference);
            assert_eq!(result.unwrap(), expected);
            assert_eq!(calls, 1);
            let (result, charged, calls) =
                observed(&snapshot, name, arguments.clone(), Some(total), reference);
            assert_eq!(result.unwrap(), expected);
            assert_eq!((charged, calls), (total, 1));
            let (result, charged, calls) =
                observed(&snapshot, name, arguments, Some(total - 1), reference);
            let error = result.unwrap_err();
            assert!(error.code.contains("allocation"), "{}", error.code);
            assert_eq!(
                calls, 1,
                "the failure must reach the intrinsic, not input admission"
            );
            assert!(charged < total);
        }
    }
    let (_, full, _) = observed(
        &snapshot,
        "slice",
        vec![
            input.clone(),
            NormalizedValue::I64(0),
            NormalizedValue::I64(2048),
        ],
        None,
        false,
    );
    let (_, part, _) = observed(
        &snapshot,
        "slice",
        vec![
            input.clone(),
            NormalizedValue::I64(1),
            NormalizedValue::I64(2047),
        ],
        None,
        false,
    );
    assert_eq!(
        part - full,
        crate::platform::execution::normalized::bytes::BytePayload::SLICE_DESCRIPTOR_BYTES
    );
    assert_eq!(input, NormalizedValue::bytes(vec![255; 2048]));
}

#[test]
fn byte_range_pre_cancelled_calls_return_no_value_in_either_evaluator() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let control = ExecutionControl::uncancelled();
    control.cancel();
    for (name, arguments) in [
        (
            "slice",
            vec![
                NormalizedValue::bytes([1, 2, 3]),
                NormalizedValue::I64(0),
                NormalizedValue::I64(3),
            ],
        ),
        ("copy", vec![NormalizedValue::bytes([1, 2, 3])]),
    ] {
        let declaration = declaration_named(&snapshot, name);
        assert!(
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(declaration, arguments.clone(), None, &control)
                .is_err()
        );
        assert!(
            NormalizedReferenceInterpreter::new(
                &snapshot,
                &program,
                NormalizedRunPolicy::foreground()
            )
            .invoke(declaration, arguments, None, &control)
            .is_err()
        );
    }
}
