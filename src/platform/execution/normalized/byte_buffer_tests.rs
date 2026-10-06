//! Ownership, independent source execution, and raw-boundary regression tests.
use super::super::prepare::NormalizedEntryPoint;
use super::*;
const INPUT: &str = include_str!("../../../../tests/fixtures/owned-byte-buffer.lkjc");
pub(crate) fn author(extra: &str) -> Result<crate::platform::kernel::KernelSnapshot, String> {
    author_only(&format!("{INPUT}\n{extra}"))
}
pub(crate) fn author_only(input: &str) -> Result<crate::platform::kernel::KernelSnapshot, String> {
    let dir = tempfile::tempdir().unwrap();
    let empty = empty_normalized_snapshot(b"owned-byte-buffer");
    let repo = GraphRepository::create(&dir.path().join("project"), &empty, None)
        .unwrap()
        .repository;
    let base = repo.view_current().unwrap().revision();
    let input = format!("request base={base}\n{input}");
    let decoded = crate::platform::control::decode_compact_change("buffer", input.as_bytes())
        .map_err(|e| format!("{e:?}"))?;
    let plan = repo
        .prepare_authored_change(&decoded.semantic, decoded.options)
        .map_err(|e| format!("{e:?}"))?;
    repo.publish(&plan.publication)
        .map_err(|e| format!("{e:?}"))?;
    Ok(repo
        .view_current()
        .unwrap()
        .reconstruct_full_oracle()
        .unwrap()
        .value)
}
#[test]
fn byte_buffer_creation_reserves_complete_owned_storage_before_allocation() {
    use super::super::byte_buffer::{ByteBuffer, StorageObservation};
    use super::super::value::ValueOrigin;
    use std::mem::size_of;
    use std::sync::Mutex;

    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let declaration = declaration_named(&source, "abandoned");
    // Independently model the token, synchronized payload, loan count,
    // admitted-program identity and certificate flag, plus two Arc counters.
    // This is admitted storage, not allocator/RSS usage.
    let expected = (size_of::<ByteBuffer>()
        + size_of::<Mutex<(Option<Vec<u8>>, usize, Option<ValueOrigin>, bool)>>()
        + 2 * size_of::<usize>()) as u64;
    let mut charges = vec![];
    for reference in [false, true] {
        let run = |maximum| {
            let storage = StorageObservation::start();
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: maximum,
                ..NormalizedRunPolicy::foreground()
            };
            let control = ExecutionControl::uncancelled();
            let (result, allocated, calls) = if reference {
                let observer = Mutex::new(None);
                let result = NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .observing_checked(&observer)
                    .invoke(declaration, vec![], None, &control)
                    .map(|pair| pair.0);
                let observed = observer.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_transactions_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                (result, observed.allocated_bytes, observed.external_calls)
            } else {
                let observer = Mutex::new(None);
                let result = NormalizedVm::for_test(&program, policy)
                    .observing_checked(&observer)
                    .invoke(declaration, vec![], None, &control)
                    .map(|pair| pair.0);
                let observed = observer.into_inner().unwrap().unwrap();
                assert_eq!(observed.live_call_frames_after, 0);
                assert_eq!(observed.live_operands_after, 0);
                assert_eq!(observed.live_transactions_after, 0);
                assert_eq!(observed.live_handles_after, 0);
                (result, observed.allocated_bytes, observed.external_calls)
            };
            assert_eq!(storage.live(), (0, 0));
            assert!(storage.created() <= 1);
            (result, allocated, calls, storage.created())
        };
        let (value, total, calls, created) = run(None);
        assert_eq!(value.unwrap(), NormalizedValue::Unit);
        assert_eq!((calls, created), (1, 1));
        assert_eq!(run(Some(total)).0.unwrap(), NormalizedValue::Unit);

        // Find creation, not merely successful completion: later stack/result
        // charges may fail after the allocation. No engine-wide magic quota.
        let (mut low, mut high) = (1, total);
        while low < high {
            let middle = low + (high - low) / 2;
            let (result, _, _, created) = run(Some(middle));
            if created == 0 {
                assert_eq!(
                    result.unwrap_err().class,
                    crate::platform::execution::ExecutionFailureClass::Resource
                );
                low = middle + 1;
            } else {
                high = middle;
            }
        }
        let (refused, before, calls, created) = run(Some(low - 1));
        assert_eq!(
            refused.unwrap_err().class,
            crate::platform::execution::ExecutionFailureClass::Resource
        );
        assert_eq!((calls, created), (1, 0));
        assert_eq!(run(Some(low)).3, 1);
        charges.push(low - before);
    }
    assert_eq!(
        charges,
        vec![expected; 2],
        "VM and source-reference creation charges"
    );
}

#[test]
fn byte_buffer_source_and_vm_have_exact_binary_and_cleanup() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    for (name, expected) in [
        ("main", NormalizedValue::bytes(vec![0, 255, 128])),
        ("abandoned", NormalizedValue::Unit),
        ("discarded", NormalizedValue::Unit),
    ] {
        let d = declaration_named(&source, name);
        let (actual, work) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(d, vec![], None, &control)
            .unwrap();
        let (reference, reference_work) = NormalizedReferenceInterpreter::new(
            &source,
            &program,
            NormalizedRunPolicy::foreground(),
        )
        .invoke(d, vec![], None, &control)
        .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(reference, expected);
        assert_eq!(work.live_call_frames_after, 0);
        assert_eq!(work.live_operands_after, 0);
        assert_eq!(reference_work.live_call_frames_after, 0);
    }
}

#[test]
fn byte_buffer_task_locals_preserve_task_kind_in_both_evaluators() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let d = declaration_named(&source, "task-local");
    let target = Name::new("task-buffer").unwrap();
    let failure = author("declarations.begin\n(units (module create wrong-context (function create bad (visibility private) (returns Bytes) (effect pure) (body (call flows::task-local)))))\ndeclarations.end\n").unwrap_err();
    assert!(failure.contains("kernel_type_pure_task_call"), "{failure}");
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        let value = if reference {
            let engine = NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            );
            assert_eq!(
                engine.invoke(d, vec![], None, &control).unwrap().0,
                NormalizedValue::bytes(vec![])
            );
            engine
                .invoke_root_target(&target, vec![], None, &control)
                .unwrap()
                .0
        } else {
            let engine = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground());
            assert_eq!(
                engine.invoke(d, vec![], None, &control).unwrap().0,
                NormalizedValue::bytes(vec![])
            );
            engine
                .invoke_root_target(&target, vec![], None, &control)
                .unwrap()
                .0
        };
        assert_eq!(value, NormalizedValue::bytes(vec![]));
        assert_eq!(observer.live(), (0, 0));
    }
}
#[test]
fn byte_buffer_invalid_octets_and_raw_owners_reject_in_both_evaluators() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let control = ExecutionControl::uncancelled();
    for n in [-1, 256, i64::MIN, i64::MAX] {
        let d = declaration_named(&source, "invalid-push");
        assert_eq!(
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(d, vec![NormalizedValue::I64(n)], None, &control)
                .unwrap_err()
                .code,
            "normalized_buffer_octet"
        );
        assert_eq!(
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground()
            )
            .invoke(d, vec![NormalizedValue::I64(n)], None, &control)
            .unwrap_err()
            .code,
            "normalized_buffer_octet"
        );
    }
    let domain = super::super::value::ValueOrigin::fresh().unwrap();
    let raw = NormalizedValue::ByteBuffer(super::super::byte_buffer::ByteBuffer::empty(domain));
    let d = declaration_named(&source, "freeze");
    assert!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(d, vec![raw.clone()], None, &control)
            .is_err()
    );
    assert!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(d, vec![raw], None, &control)
            .is_err()
    );
    let d = declaration_named(&source, "empty");
    assert!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(d, vec![], None, &control)
            .is_err()
    );
    assert!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(d, vec![], None, &control)
            .is_err()
    );
}
#[test]
fn byte_buffer_meaning_rejects_copy_escape_generic_and_alias() {
    for body in [
        "(function create bad (visibility private) (parameter create b (type ByteBuffer) (use borrow)) (returns ByteBuffer) (effect pure) (body (local b)))",
        "(function create bad (visibility private) (parameter create b (type ByteBuffer) (use borrow)) (returns Unit) (effect pure) (body (call memory::discard (local b))))",
        "(function create bad (visibility private) (parameter create b (type ByteBuffer)) (returns Unit) (effect pure) (body (unit)))",
        "(function create bad (visibility private) (parameter create b (type ByteBuffer) (use consume)) (returns Unit) (effect pure) (body (sequence (call memory::discard (local b)) (call memory::discard (local b)))))",
        "(function create bad (visibility private) (returns (list ByteBuffer)) (effect pure) (body (list ByteBuffer)))",
        "(constant create bad (visibility private) (type ByteBuffer) (value (call memory::empty)))",
        "(record create bad (visibility private) (field create storage (type ByteBuffer)))",
        "(function create bad (visibility private) (returns (function () ByteBuffer)) (effect pure) (body (function-value memory::empty)))",
        "(function create unused (visibility private) (type-parameter create T) (returns Unit) (effect pure) (body (unit))) (function create bad (visibility private) (returns Unit) (effect pure) (body (call unused (types ByteBuffer))))",
        "(function create bad (visibility private) (parameter create b (type ByteBuffer) (use borrow)) (returns ByteBuffer (borrow-from b)) (effect (task)) (body (local b)))",
        "(record create Phantom (visibility private) (type-parameter create T) (field create tag (type I64))) (function create bad (visibility private) (returns (Phantom ByteBuffer)) (effect pure) (body (record Phantom (types ByteBuffer) (field Phantom::tag (i64 0)))))",
        "(function create alias (visibility private) (parameter create a (type ByteBuffer) (use borrow)) (parameter create b (type ByteBuffer) (use consume)) (returns Unit) (effect pure) (body (call memory::discard (local b)))) (function create bad (visibility private) (parameter create b (type ByteBuffer) (use consume)) (returns Unit) (effect pure) (body (call alias (local b) (local b))))",
    ] {
        let extra = format!(
            "declarations.begin\n(units (module create failures {body}))\ndeclarations.end\n"
        );
        let failure = author(&extra).unwrap_err();
        assert!(
            failure.contains("kernel_buffer")
                || failure.contains("kernel_borrow_result")
                || failure.contains("kernel_affine")
                || failure.contains("intrinsic_signature"),
            "wrong rejection for {body}: {failure}"
        );
    }
}

#[test]
fn byte_buffer_synchronous_task_borrows_retain_the_owner() {
    let source = author(
        r#"declarations.begin
(units (module create borrowed-tasks
  (function create inspect (visibility private) (effect (task))
    (parameter create b (type ByteBuffer) (use borrow))
    (returns I64) (body (call memory::length (local b))))
  (function create synchronous-task-read (visibility public) (effect (task))
    (returns Bytes)
    (body (let
      (binding empty (type ByteBuffer) (call memory::empty))
      (binding filled (type ByteBuffer) (call memory::push (i64 17) (local empty)))
      (binding count (type I64) (call inspect (local filled)))
      (in (call memory::freeze (local filled))))))))
declarations.end
"#,
    )
    .unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "synchronous-task-read");
    for reference in [false, true] {
        let storage = super::super::byte_buffer::StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        let value = if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke(entry, vec![], None, &control)
            .unwrap()
            .0
        } else {
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke(entry, vec![], None, &control)
                .unwrap()
                .0
        };
        assert_eq!(value, NormalizedValue::bytes(vec![17]));
        assert_eq!(storage.live(), (0, 0));
    }
}

#[test]
fn byte_buffer_failures_cancel_and_quotas_drop_all_storage_and_loans() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let main = declaration_named(&source, "main");
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        let mut cancellations = 0;
        for checks in 0..200 {
            let control = ExecutionControl::cancel_after_checks(checks);
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(main, vec![], None, &control)
                .map(|_| ())
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(main, vec![], None, &control)
                    .map(|_| ())
            };
            if let Err(e) = result {
                assert_eq!(
                    e.class,
                    crate::platform::execution::ExecutionFailureClass::Cancelled
                );
                cancellations += 1;
            }
            assert_eq!(
                observer.live(),
                (0, 0),
                "cancel checks={checks} reference={reference}"
            );
        }
        assert!(cancellations > 0);
        let mut refusals = 0;
        let mut successes = 0;
        for quota in [0, 16, 64, 128, 256, 512, 1024, 2048, 4096, 1048576] {
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: Some(quota),
                ..NormalizedRunPolicy::foreground()
            };
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(main, vec![], None, &control)
                    .map(|_| ())
            } else {
                NormalizedVm::for_test(&program, policy)
                    .invoke(main, vec![], None, &control)
                    .map(|_| ())
            };
            match result {
                Ok(()) => successes += 1,
                Err(e) => {
                    assert_eq!(
                        e.class,
                        crate::platform::execution::ExecutionFailureClass::Resource
                    );
                    refusals += 1;
                }
            }
            assert_eq!(
                observer.live(),
                (0, 0),
                "quota={quota} reference={reference}"
            );
        }
        assert!(
            refusals > 0 && successes > 0,
            "both refused and completed quota paths must execute"
        );
    }
}

#[test]
fn byte_buffer_independent_kernel_oracle_and_reference_reject_mutated_source() {
    use crate::platform::kernel::*;
    let source = author("").unwrap();
    assert!(memory_reference::accepts(&source));
    let buffer = source
        .types
        .iter()
        .find_map(|(id, t)| matches!(t.form, TypeForm::ByteBuffer).then_some(*id))
        .unwrap();
    for name in ["consumer", "forward", "get", "transformer"] {
        let mut invalid = source.clone();
        let d = declaration_named(&source, name);
        let OwnerRecord::Declaration(record) = invalid
            .owners
            .get_mut(&OwnerKey::Declaration(d.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let parameters = match &mut record.payload {
            DeclarationPayload::Function(f) => f.parameters.clone(),
            DeclarationPayload::External(f) => f.parameters.clone(),
            _ => unreachable!(),
        };
        let parameter=parameters.iter().copied().find(|p|matches!(source.owners.get(&OwnerKey::Parameter(*p)),Some(OwnerRecord::Parameter(p))if p.ty==buffer)).unwrap();
        let OwnerRecord::Parameter(p) = invalid
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        p.use_mode = ParameterUse::Unrestricted;
        assert!(!memory_reference::accepts(&invalid));
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
}

#[test]
fn byte_buffer_long_owned_builder_borrowed_reader_and_both_owner_choices() {
    let source = author("").unwrap();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        let control = ExecutionControl::uncancelled();
        for (name, args, expected) in [
            (
                "long-bytes",
                vec![],
                NormalizedValue::bytes(vec![12; 16384]),
            ),
            ("long-read", vec![], NormalizedValue::I64(196608)),
            (
                "choice",
                vec![NormalizedValue::Bool(true)],
                NormalizedValue::bytes(vec![0]),
            ),
            (
                "choice",
                vec![NormalizedValue::Bool(false)],
                NormalizedValue::bytes(vec![255]),
            ),
        ] {
            let d = declaration_named(&source, name);
            if reference {
                let (value, work) = NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(d, args, None, &control)
                .unwrap();
                assert_eq!(value, expected);
                assert!(work.maximum_call_depth < 16);
                if name.starts_with("long") {
                    assert!(work.tail_transfers >= 16383);
                }
            } else {
                let (value, work) =
                    NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                        .invoke(d, args, None, &control)
                        .unwrap();
                assert_eq!(value, expected);
                assert!(work.maximum_call_depth < 16);
                if name.starts_with("long") {
                    assert!(work.tail_transfers >= 16383);
                }
            }
            assert_eq!(observer.live(), (0, 0));
        }
        let test = declaration_named(&source, "transient-test");
        if reference {
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .invoke_test(test, None, &control)
            .unwrap();
        } else {
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .invoke_test(test, None, &control)
                .unwrap();
        }
        assert_eq!(observer.live(), (0, 0));
    }
}

#[test]
fn byte_buffer_all_executable_roots_and_branch_maybe_moves_reject() {
    let repeated = "(let (binding b (type ByteBuffer) (call memory::empty)) (in (sequence (call memory::discard (local b)) (call memory::freeze (local b)))))";
    let direct_uses = [
        format!("(constant create bad (visibility private) (type Bytes) (value {repeated}))"),
        format!(
            "(test create bad (visibility private) (actual {repeated}) (expected (let (binding x (type ByteBuffer) (call memory::empty)) (in (call memory::freeze (local x))))))"
        ),
        format!(
            "(test create bad (visibility private) (actual (let (binding x (type ByteBuffer) (call memory::empty)) (in (call memory::freeze (local x))))) (expected {repeated}))"
        ),
        format!(
            "(component create bad (visibility private) (port create p (type (function () Bytes)) (value {repeated})))"
        ),
    ];
    for body in direct_uses {
        let failure = author(&format!(
            "declarations.begin\n(units (module create failures {body}))\ndeclarations.end"
        ))
        .unwrap_err();
        assert!(failure.contains("kernel_buffer_ownership"), "{failure}");
    }
    let extra = "declarations.begin\n(units (module create failures (function create maybe-moved (visibility private) (parameter create flag (type Bool)) (parameter create a (type ByteBuffer) (use consume)) (parameter create b (type ByteBuffer) (use consume)) (returns Bytes) (effect pure) (body (let (binding selected (type ByteBuffer) (if (local flag) (local a) (local b))) (in (call memory::freeze (local a))))))))\ndeclarations.end";
    assert!(
        author(extra)
            .unwrap_err()
            .contains("kernel_buffer_ownership")
    );
}

#[test]
fn byte_buffer_raw_unused_generic_and_descriptor_instantiations_reject() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let buffer = source
        .types
        .iter()
        .find_map(|(id, t)| {
            matches!(t.form, crate::platform::kernel::TypeForm::ByteBuffer).then_some(*id)
        })
        .unwrap();
    let declaration = declaration_named(&source, "unused-type");
    let function = program.function(declaration).unwrap();
    let control = ExecutionControl::uncancelled();
    assert!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke_entry(
                NormalizedEntryPoint::InstantiatedFunction(
                    function,
                    std::sync::Arc::from([buffer])
                ),
                vec![],
                None,
                &control
            )
            .is_err()
    );
    assert!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke_instantiated(declaration, &[buffer], vec![], &control)
            .is_err()
    );
    let raw = NormalizedValue::Function {
        function,
        type_arguments: std::sync::Arc::from([buffer]),
        effect_arguments: std::sync::Arc::from([]),
        requirement_arguments: std::sync::Arc::from([]),
        bound_arguments: None,
    };
    let callback = declaration_named(&source, "callback");
    assert!(
        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
            .invoke(callback, vec![raw.clone()], None, &control)
            .is_err()
    );
    assert!(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(callback, vec![raw], None, &control)
            .is_err()
    );
}

struct InspectHost<'a> {
    observer: &'a super::super::byte_buffer::StorageObservation,
    cancel: bool,
    live: (usize, usize),
    forge: bool,
}
impl InspectHost<'_> {
    fn answer(&self, control: &ExecutionControl) -> Result<NormalizedValue, ExecutionError> {
        let live = self.observer.live();
        assert_eq!(live.0, self.live.0);
        assert!(live.1 >= self.live.1);
        if self.cancel {
            control.cancel();
        }
        if self.forge {
            return Ok(NormalizedValue::ByteBuffer(
                super::super::byte_buffer::ByteBuffer::empty(
                    super::super::value::ValueOrigin::fresh().unwrap(),
                ),
            ));
        }
        Ok(NormalizedValue::I64(0))
    }
}
impl super::super::vm::NormalizedHost for InspectHost<'_> {
    fn call(
        &self,
        _: &NormalizedProgram,
        _: &super::super::prepare::NormalizedFunction,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}
impl super::super::reference::NormalizedReferenceHost for InspectHost<'_> {
    fn call(
        &self,
        _: &dyn super::super::value_schema::NormalizedValueSchema,
        _: &super::super::reference::ReferenceSignature,
        _: &crate::platform::kernel::ImplementationName,
        _: &[TypeObjectDigest],
        _: Vec<NormalizedValue>,
        control: &ExecutionControl,
    ) -> Result<NormalizedValue, ExecutionError> {
        self.answer(control)
    }
}
#[test]
fn byte_buffer_lexical_drop_precedes_continuation_and_cancel_during_live_loan_cleans_up() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        for (name, cancel, live) in [
            ("scope-cleanup", false, (0, 0)),
            ("cancel-loan", true, (1, 1)),
        ] {
            let host = InspectHost {
                observer: &observer,
                cancel,
                live,
                forge: false,
            };
            let d = declaration_named(&source, name);
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                let sink = std::sync::Mutex::new(None);
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .observing(&sink, &host)
                .invoke(d, vec![], None, &control)
                .map(|_| ())
            } else {
                let sink = std::sync::Mutex::new(None);
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .observing(&sink, &host)
                    .invoke(d, vec![], None, &control)
                    .map(|_| ())
            };
            if cancel {
                assert_eq!(
                    result.unwrap_err().class,
                    crate::platform::execution::ExecutionFailureClass::Cancelled
                );
            } else {
                result.unwrap();
            }
            assert_eq!(observer.live(), (0, 0));
        }
    }
}

#[test]
fn byte_buffer_raw_adapter_results_and_borrowed_or_stale_ingress_reject() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        let host = InspectHost {
            observer: &observer,
            cancel: false,
            live: (0, 0),
            forge: true,
        };
        let control = ExecutionControl::uncancelled();
        let d = declaration_named(&source, "scope-cleanup");
        let result = if reference {
            let sink = std::sync::Mutex::new(None);
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .observing(&sink, &host)
            .invoke(d, vec![], None, &control)
            .map(|_| ())
        } else {
            let sink = std::sync::Mutex::new(None);
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .observing(&sink, &host)
                .invoke(d, vec![], None, &control)
                .map(|_| ())
        };
        assert!(result.is_err());
        assert_eq!(observer.live(), (0, 0));
        for stale in [false, true] {
            let owner = super::super::byte_buffer::ByteBuffer::empty(
                super::super::value::ValueOrigin::fresh().unwrap(),
            );
            let raw = if stale {
                owner.clone()
            } else {
                owner.borrow().unwrap()
            };
            let retained = if stale {
                drop(owner);
                None
            } else {
                Some(owner)
            };
            let d = declaration_named(&source, "freeze");
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(d, vec![NormalizedValue::ByteBuffer(raw)], None, &control)
                .map(|_| ())
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(d, vec![NormalizedValue::ByteBuffer(raw)], None, &control)
                    .map(|_| ())
            };
            assert!(result.is_err());
            drop(retained);
            assert_eq!(observer.live(), (0, 0));
        }
    }
}

#[test]
fn byte_buffer_independent_oracle_rejects_repeated_direct_discard() {
    use crate::platform::kernel::*;
    let source = author(
        r#"declarations.begin
(units (module create oracle-discard
  (function create parameter-owner (visibility private)
    (parameter create b (type ByteBuffer) (use consume)) (returns Unit) (effect pure)
    (body (sequence (local b) (unit) (unit))))
  (function create lexical-owner (visibility private) (returns Unit) (effect pure)
    (body (let (binding b (type ByteBuffer) (call memory::empty))
      (in (sequence (local b) (unit) (unit))))))))
declarations.end"#,
    )
    .unwrap();
    assert!(memory_reference::accepts(&source));
    for name in ["parameter-owner", "lexical-owner"] {
        let mut invalid = source.clone();
        let d = declaration_named(&source, name);
        let OwnerRecord::Declaration(record) =
            &source.owners[&OwnerKey::Declaration(d.declaration)]
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &record.payload else {
            unreachable!()
        };
        let OwnerRecord::Expression(root) = &source.owners[&OwnerKey::Expression(f.body)] else {
            unreachable!()
        };
        let sequence = if let ExpressionOperation::Let { body, .. } = &root.operation {
            let OwnerRecord::Expression(body) = &source.owners[&OwnerKey::Expression(*body)] else {
                unreachable!()
            };
            &body.operation
        } else {
            &root.operation
        };
        let ExpressionOperation::Sequence { items } = sequence else {
            unreachable!()
        };
        let OwnerRecord::Expression(first) = &source.owners[&OwnerKey::Expression(items[0])] else {
            unreachable!()
        };
        let OwnerRecord::Expression(second) = invalid
            .owners
            .get_mut(&OwnerKey::Expression(items[1]))
            .unwrap()
        else {
            unreachable!()
        };
        second.operation = first.operation.clone();
        let diagnostics = validate_full(&invalid).unwrap_err();
        assert!(format!("{diagnostics:?}").contains("kernel_buffer_ownership"));
        assert!(
            !memory_reference::accepts(&invalid),
            "the independent oracle lost the moved {name} identity"
        );
    }
}

#[test]
fn byte_buffer_discarded_sequence_temporary_is_gone_before_its_successor() {
    let source = author(
        r#"declarations.begin
(units (module create sequence-drop
  (external create probe (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create temporary-drop (visibility private) (returns I64) (effect pure)
    (body (let
      (binding observed (sequence (call memory::empty) (call probe (i64 0) (i64 0))))
      (in (local observed)))))))
declarations.end"#,
    )
    .unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        let host = InspectHost {
            observer: &observer,
            cancel: false,
            live: (0, 0),
            forge: false,
        };
        let control = ExecutionControl::uncancelled();
        let d = declaration_named(&source, "temporary-drop");
        println!("sequence-discard reference={reference}");
        let value = if reference {
            let sink = std::sync::Mutex::new(None);
            NormalizedReferenceInterpreter::new(
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
            )
            .observing(&sink, &host)
            .invoke(d, vec![], None, &control)
            .unwrap()
            .0
        } else {
            let sink = std::sync::Mutex::new(None);
            NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                .observing(&sink, &host)
                .invoke(d, vec![], None, &control)
                .unwrap()
                .0
        };
        assert_eq!(value, NormalizedValue::I64(0));
        assert_eq!(observer.live(), (0, 0));
    }
}

#[test]
fn byte_buffer_invalid_indices_trap_and_drop_storage_in_both_evaluators() {
    let source = author("").unwrap();
    let program = prepare_snapshot(&source);
    let d = declaration_named(&source, "invalid-index");
    let control = ExecutionControl::uncancelled();
    for reference in [false, true] {
        let observer = super::super::byte_buffer::StorageObservation::start();
        for index in [-1, 1, i64::MIN, i64::MAX] {
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(d, vec![NormalizedValue::I64(index)], None, &control)
                .map(|_| ())
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(d, vec![NormalizedValue::I64(index)], None, &control)
                    .map(|_| ())
            };
            assert_eq!(result.unwrap_err().code, "normalized_buffer_index");
            assert_eq!(observer.live(), (0, 0));
        }
    }
}
