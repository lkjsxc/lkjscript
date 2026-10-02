//! A same-task transfer is neither a memory loan nor a raw-entry capability.
use super::*;
use crate::platform::kernel::*;

const FUNCTIONS: &str = r#"declarations.begin
(units (module create task-boundaries
  (function create task-ignore (visibility public) (effect (task))
    (parameter create owner (type ByteBuffer) (use consume))
    (returns Unit) (body (unit)))
  (function create task-mint (visibility public) (effect (task))
    (returns ByteBuffer) (body (call memory::empty)))
  (function create task-good (visibility public) (effect (task)) (returns Unit)
    (body (let (binding owner (type ByteBuffer) (call task-mint))
      (in (call task-ignore (local owner))))))
  (function create task-trap (visibility public) (effect (task))
    (parameter create owner (type ByteBuffer) (use consume))
    (returns I64) (body (call memory::get (i64 0) (local owner))))
  (function create task-fail (visibility public) (effect (task)) (returns I64)
    (body (let (binding owner (type ByteBuffer) (call task-mint))
      (in (call task-trap (local owner))))))))
declarations.end
"#;

#[test]
fn owned_task_rejects_unused_loans_unrestricted_order_alias_and_pure_call() {
    for (signature, expected) in [
        (
            "(parameter create owner (type ByteBuffer) (use borrow))",
            "kernel_buffer",
        ),
        (
            "(parameter create owner (type ByteBuffer))",
            "kernel_buffer",
        ),
        (
            "(type-parameter create T (constraint owned)) (parameter create owner (type T) (use borrow))",
            "kernel_buffer",
        ),
        (
            "(parameter create owner (type ByteBuffer) (use consume)) (parameter create later (type I64))",
            "kernel_buffer",
        ),
    ] {
        let invalid = format!(
            "declarations.begin\n(units (module create invalid-task (function create unused (visibility private) (effect (task)) {signature} (returns Unit) (body (unit)))))\ndeclarations.end\n"
        );
        let error = byte_buffer_tests::author(&invalid).unwrap_err();
        assert!(error.contains(expected), "{signature}: {error}");
    }
    let duplicate = FUNCTIONS.replace(
        "(in (call task-ignore (local owner)))",
        "(in (sequence (call task-ignore (local owner)) (call task-ignore (local owner))))",
    );
    assert!(
        byte_buffer_tests::author(&duplicate)
            .unwrap_err()
            .contains("kernel_buffer")
    );
    let pure = FUNCTIONS.replace(
        "(function create task-good (visibility public) (effect (task))",
        "(function create task-good (visibility public) (effect pure)",
    );
    assert!(
        byte_buffer_tests::author(&pure)
            .unwrap_err()
            .contains("kernel_type_pure_task_call")
    );
}

#[test]
fn owned_task_independent_oracle_rejects_unused_task_loan_mutations() {
    let source = byte_buffer_tests::author(FUNCTIONS).unwrap();
    assert!(memory_reference::accepts(&source));
    let target = declaration_named(&source, "task-ignore");
    let OwnerRecord::Declaration(record) =
        &source.owners[&OwnerKey::Declaration(target.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &record.payload else {
        unreachable!()
    };
    let parameter = f.parameters[0];
    for mode in [ParameterUse::Borrow, ParameterUse::Unrestricted] {
        let mut invalid = source.clone();
        let OwnerRecord::Parameter(p) = invalid
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        p.use_mode = mode;
        assert!(!memory_reference::accepts(&invalid));
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
}

#[test]
fn owned_task_raw_ingress_egress_and_traps_release_before_reuse() {
    let source = byte_buffer_tests::author(FUNCTIONS).unwrap();
    let program = prepare_snapshot(&source);
    let ignore = declaration_named(&source, "task-ignore");
    let mint = declaration_named(&source, "task-mint");
    let good = declaration_named(&source, "task-good");
    let fail = declaration_named(&source, "task-fail");
    for reference in [false, true] {
        let observed = super::super::byte_buffer::StorageObservation::start();
        let run = |entry, args| {
            let control = ExecutionControl::uncancelled();
            if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, args, None, &control)
                .map(|v| v.0)
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, args, None, &control)
                    .map(|v| v.0)
            }
        };
        for cloned in [false, true] {
            let raw = NormalizedValue::ByteBuffer(super::super::byte_buffer::ByteBuffer::empty(
                super::super::value::ValueOrigin::fresh().unwrap(),
            ));
            let argument = if cloned { raw.clone() } else { raw };
            assert!(run(ignore, vec![argument]).is_err());
        }
        assert_eq!(observed.live(), (0, 0));
        assert!(run(mint, vec![]).is_err());
        assert_eq!(observed.live(), (0, 0));
        let error = run(fail, vec![]).unwrap_err();
        assert_eq!(error.code, "normalized_buffer_index");
        assert_eq!(observed.live(), (0, 0));
        assert_eq!(run(good, vec![]).unwrap(), NormalizedValue::Unit);
        assert_eq!(observed.live(), (0, 0));
    }
}
