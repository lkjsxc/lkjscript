//! Cancellation and explicit quotas keep whole-operation failure semantics.
use super::*;

#[test]
fn map_entries_quota_refusal_cancellation_and_recovery_leave_no_partial_result() {
    let (mut program, mut snapshot) = fixture();
    let mut schema = NormalizedReferenceSchema::reconstruct([&snapshot]).unwrap();
    let integer = internal_type(&mut program, &mut schema, TypeForm::I64);
    let list = internal_type(&mut program, &mut schema, TypeForm::List { item: integer });
    let option = internal_type(&mut program, &mut schema, TypeForm::Option { item: list });
    let ty = internal_type(&mut program, &mut schema, TypeForm::Option { item: option });
    snapshot.types = schema.types.clone();
    let reader = boundary_reader(&snapshot, &schema);
    let child = NormalizedValue::Option(Some(Box::new(NormalizedValue::Option(Some(Box::new(
        NormalizedValue::list(vec![NormalizedValue::I64(17); 4]).unwrap(),
    ))))));
    for reference in [false, true] {
        for count in [0, 1, 8] {
            let original = map((0..count).map(|key| (NormalizedMapKey::I64(key), child.clone())));
            let run = |policy, control: &ExecutionControl| {
                invoke(
                    &program,
                    &reader,
                    reference,
                    "core.map.entries",
                    &[integer, ty],
                    vec![original.clone()],
                    policy,
                    control,
                    None,
                )
            };
            let complete = run(
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled(),
            );
            assert!(complete.value.is_ok());
            // Explicit quota dimensions must be positive, even for zero work.
            // A zero-valued policy is rejected before the execution observer.
            let (bytes, items) = (complete.bytes.max(1), complete.items.max(1));
            if count > 0 {
                assert!(complete.bytes > 1 && complete.items > 1);
            }
            let mut limits = vec![(bytes, items, true)];
            if bytes > 1 {
                limits.push((bytes - 1, items, false));
            }
            if items > 1 {
                limits.push((bytes, items - 1, false));
            }
            for (bytes, items, success) in limits {
                let bounded = run(
                    NormalizedRunPolicy {
                        maximum_allocated_bytes: Some(bytes),
                        maximum_collection_items: Some(items),
                        ..NormalizedRunPolicy::foreground()
                    },
                    &ExecutionControl::uncancelled(),
                );
                assert_eq!(
                    bounded.value.is_ok(),
                    success,
                    "reference={reference}/count={count}"
                );
                if !success {
                    assert_eq!(
                        bounded.value.unwrap_err().class,
                        crate::platform::execution::ExecutionFailureClass::Resource
                    );
                }
            }
            let mut finished = false;
            let mut during_projection = false;
            // The first poll precedes observer creation; cover it separately.
            for checks in 1..4096 {
                let observed = run(
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::cancel_after_checks(checks),
                );
                if let Ok(value) = observed.value {
                    assert_eq!(&value, complete.value.as_ref().unwrap());
                    finished = true;
                    break;
                }
                assert_eq!(observed.value.unwrap_err().code, "execution_cancelled");
                during_projection |= observed.work.input_admission_nodes
                    == complete.work.input_admission_nodes
                    && observed.work.maps.node_visits > count as u64;
            }
            assert!(
                finished,
                "cancellation sweep must reach a complete execution"
            );
            if count > 0 {
                assert!(
                    during_projection,
                    "must cancel after complete ingress, during projection"
                );
            }
            assert_eq!(
                run(
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled()
                )
                .value
                .unwrap(),
                complete.value.unwrap()
            );
            let NormalizedValue::Map(retained) = original else {
                unreachable!()
            };
            assert_eq!(retained.len(), count as usize);
            assert!(retained.iter().all(|(_, value)| value == &child));
        }
    }
}

#[test]
fn map_entries_pre_cancelled_invocation_rejects_before_observation_in_both_evaluators() {
    let (program, snapshot) = fixture();
    let integer = scalar_type(&program, TypeForm::I64);
    let (function, declaration) = external(&program, "core.map.entries", 2, 1);
    let types = [integer, integer];
    let raw = map([(NormalizedMapKey::I64(11), NormalizedValue::I64(17))]);
    let control = ExecutionControl::cancel_after_checks(0);
    let production = std::sync::Mutex::new(None);
    let result = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .observing_checked(&production)
        .invoke_entry(
            NormalizedEntryPoint::InstantiatedFunction(function, Arc::from(types)),
            vec![raw.clone()],
            None,
            &control,
        );
    assert_eq!(result.unwrap_err().code, "execution_cancelled");
    assert!(production.into_inner().unwrap().is_none());
    let reference = std::sync::Mutex::new(None);
    let result = NormalizedReferenceInterpreter::from_reader(
        &snapshot,
        &program,
        NormalizedRunPolicy::foreground(),
    )
    .observing_checked(&reference)
    .invoke_instantiated(declaration, &types, vec![raw.clone()], &control);
    assert_eq!(result.unwrap_err().code, "execution_cancelled");
    assert!(reference.into_inner().unwrap().is_none());
    assert_eq!(
        raw,
        map([(NormalizedMapKey::I64(11), NormalizedValue::I64(17))])
    );
}
