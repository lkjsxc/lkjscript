//! Physical byte ownership through native declarations and the actual VM.
use super::*;

fn fixture() -> crate::platform::kernel::KernelSnapshot {
    let temporary = tempfile::tempdir().unwrap();
    let repository = GraphRepository::create(
        &temporary.path().join("bytes"),
        &empty_normalized_snapshot(b"native-byte-buffer-reuse"),
        None,
    )
    .unwrap()
    .repository;
    let base = repository.view_current().unwrap().revision();
    let request = format!(
        r#"request base={base}
declarations.begin
(units
  (module create buffer
    (external create concat (visibility public) (implementation core.bytes.concat)
      (parameter create left (type Bytes))
      (parameter create right (type Bytes))
      (returns Bytes))))
declarations.end
"#
    );
    let decoded =
        crate::platform::control::decode_compact_change("byte reuse", request.as_bytes()).unwrap();
    let prepared = repository
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .unwrap();
    assert!(matches!(
        repository.publish(&prepared.publication).unwrap(),
        PublicationOutcome::Accepted { .. }
    ));
    repository
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value
}

fn concat(
    program: &NormalizedProgram,
    declaration: crate::platform::kernel::DeclarationReference,
    left: NormalizedValue,
    right: NormalizedValue,
) -> NormalizedValue {
    NormalizedVm::for_test(program, NormalizedRunPolicy::foreground())
        .invoke(
            declaration,
            vec![left, right],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap()
        .0
}

fn pointer(value: &NormalizedValue) -> *const u8 {
    let NormalizedValue::Bytes(bytes) = value else {
        panic!("byte result");
    };
    bytes.as_ptr()
}

#[test]
fn bytes_concat_empty_operands_retain_the_nonempty_payload() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "concat");
    let retained = NormalizedValue::bytes(vec![0, 128, 255, 17]);
    for left_empty in [true, false] {
        let empty = NormalizedValue::bytes(Vec::<u8>::new());
        let (left, right) = if left_empty {
            (empty, retained.clone())
        } else {
            (retained.clone(), empty)
        };
        let result = concat(&program, declaration, left, right);
        assert_eq!(result, retained);
        assert_eq!(
            pointer(&result),
            pointer(&retained),
            "an empty operand needs no new payload"
        );
    }
}

#[test]
fn bytes_concat_reuses_unique_buffer_capacity_after_growth() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "concat");
    let two = concat(
        &program,
        declaration,
        NormalizedValue::bytes(vec![0]),
        NormalizedValue::bytes(vec![128]),
    );
    let three = concat(
        &program,
        declaration,
        two,
        NormalizedValue::bytes(vec![255]),
    );
    let original = pointer(&three);
    let four = concat(
        &program,
        declaration,
        three,
        NormalizedValue::bytes(vec![17]),
    );
    assert_eq!(four, NormalizedValue::bytes(vec![0, 128, 255, 17]));
    assert_eq!(
        pointer(&four),
        original,
        "a unique three-byte buffer with four-byte capacity must append in place"
    );
}

#[test]
fn bytes_concat_vm_late_budget_exhaustion_cleans_owned_slots() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "concat");
    let arguments = vec![
        NormalizedValue::bytes([0, 128]),
        NormalizedValue::bytes([255]),
    ];
    let control = ExecutionControl::uncancelled();
    let (_, complete) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap();
    assert_eq!(complete.value_work.bytes.payload_bytes_copied, 3);
    let mut policy = NormalizedRunPolicy::foreground();
    policy.maximum_allocated_bytes = Some(complete.allocated_bytes - 1);
    let sink = std::sync::Mutex::new(None);
    let failure = NormalizedVm::for_test(&program, policy)
        .observing_checked(&sink)
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap_err();
    assert_eq!(failure.code, "normalized_allocation");
    let work = sink.into_inner().unwrap().unwrap();
    assert_eq!(work.value_work.bytes.concatenations, 1);
    assert_eq!(work.value_work.bytes.payload_bytes_copied, 3);
    assert_eq!(work.value_work.bytes.requested_capacity_bytes, 3);
    assert_eq!(
        (
            work.live_locals_after,
            work.live_operands_after,
            work.live_call_frames_after
        ),
        (0, 0, 0)
    );
    let mut policy = NormalizedRunPolicy::foreground();
    policy.maximum_allocated_bytes = Some(complete.allocated_bytes);
    let (result, recovered) = NormalizedVm::for_test(&program, policy)
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap();
    assert_eq!(result, NormalizedValue::bytes([0, 128, 255]));
    assert_eq!(recovered.allocated_bytes, complete.allocated_bytes);
    assert_eq!(
        arguments,
        vec![
            NormalizedValue::bytes([0, 128]),
            NormalizedValue::bytes([255])
        ]
    );
}

#[test]
fn bytes_concat_vm_reservation_sweep_distinguishes_payload_and_operand_failure() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "concat");
    let arguments = vec![
        NormalizedValue::bytes([0, 128]),
        NormalizedValue::bytes([255]),
    ];
    let control = ExecutionControl::uncancelled();
    let (_, complete) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .invoke(declaration, arguments.clone(), None, &control)
        .unwrap();
    let mut before_copy = 0;
    let mut after_copy = 0;
    assert!(
        complete.allocated_bytes < 16_384,
        "bounded tiny-fixture sweep"
    );
    for maximum in 1..complete.allocated_bytes {
        let mut policy = NormalizedRunPolicy::foreground();
        policy.maximum_allocated_bytes = Some(maximum);
        let sink = std::sync::Mutex::new(None);
        let failure = NormalizedVm::for_test(&program, policy)
            .observing_checked(&sink)
            .invoke(declaration, arguments.clone(), None, &control)
            .unwrap_err();
        assert_eq!(
            failure.class,
            crate::platform::execution::ExecutionFailureClass::Resource
        );
        let work = sink.into_inner().unwrap().unwrap();
        assert_eq!(
            (
                work.live_locals_after,
                work.live_operands_after,
                work.live_call_frames_after
            ),
            (0, 0, 0)
        );
        if work.value_work.bytes.concatenations == 1 {
            assert_eq!(failure.code, "normalized_allocation");
            match work.value_work.bytes.payload_bytes_copied {
                0 => {
                    before_copy += 1;
                    assert_eq!(work.value_work.bytes.requested_capacity_bytes, 0);
                    assert_eq!(work.value_work.bytes.fresh_buffers, 0);
                }
                3 => {
                    after_copy += 1;
                    assert_eq!(work.value_work.bytes.requested_capacity_bytes, 3);
                    assert_eq!(work.value_work.bytes.fresh_buffers, 1);
                }
                copied => panic!("unexpected partial copy {copied}"),
            }
        }
    }
    assert!(before_copy > 0 && after_copy > 0);
    assert_eq!(
        arguments,
        vec![
            NormalizedValue::bytes([0, 128]),
            NormalizedValue::bytes([255])
        ]
    );
}

#[test]
fn bytes_concat_never_mutates_a_retained_prefix() {
    let snapshot = fixture();
    let program = prepare_snapshot(&snapshot);
    let declaration = declaration_named(&snapshot, "concat");
    let prefix = concat(
        &program,
        declaration,
        NormalizedValue::bytes(vec![0]),
        NormalizedValue::bytes(vec![128]),
    );
    let prefix = concat(
        &program,
        declaration,
        prefix,
        NormalizedValue::bytes(vec![255]),
    );
    let original = pointer(&prefix);
    let result = concat(
        &program,
        declaration,
        prefix.clone(),
        NormalizedValue::bytes(vec![17]),
    );
    assert_eq!(prefix, NormalizedValue::bytes(vec![0, 128, 255]));
    assert_eq!(pointer(&prefix), original);
    assert_eq!(result, NormalizedValue::bytes(vec![0, 128, 255, 17]));
    assert_ne!(
        pointer(&result),
        original,
        "retained sharing forbids in-place mutation"
    );
    let reference =
        NormalizedReferenceInterpreter::new(&snapshot, &program, NormalizedRunPolicy::foreground())
            .invoke(
                declaration,
                vec![prefix.clone(), NormalizedValue::bytes(vec![17])],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap()
            .0;
    assert_eq!(reference, result);
    assert_eq!(prefix, NormalizedValue::bytes(vec![0, 128, 255]));
}
