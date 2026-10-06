//! Synchronous task loans preserve custody without granting raw-entry capability.
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
fn owned_task_accepts_unused_loans_but_rejects_unrestricted_order_alias_and_pure_call() {
    for signature in [
        "(parameter create owner (type ByteBuffer) (use borrow))",
        "(type-parameter create T (constraint owned)) (parameter create owner (type T) (use borrow))",
    ] {
        let valid = format!(
            "declarations.begin\n(units (module create valid-task (function create unused (visibility private) (effect (task)) {signature} (returns Unit) (body (unit)))))\ndeclarations.end\n"
        );
        let source = byte_buffer_tests::author(&valid).unwrap();
        assert!(memory_reference::accepts(&source), "{signature}");
        assert!(validate_full(&source).is_ok(), "{signature}");
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source])
                .is_ok(),
            "{signature}"
        );
    }
    for (signature, expected) in [
        (
            "(parameter create owner (type ByteBuffer))",
            "kernel_buffer",
        ),
        (
            "(type-parameter create T (constraint owned)) (parameter create owner (type T))",
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
fn owned_task_independent_oracle_accepts_loans_and_rejects_escape_and_old_generations() {
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
    let mut borrowed = source.clone();
    let OwnerRecord::Parameter(p) = borrowed
        .owners
        .get_mut(&OwnerKey::Parameter(parameter))
        .unwrap()
    else {
        unreachable!()
    };
    p.use_mode = ParameterUse::Borrow;
    assert!(memory_reference::accepts(&borrowed));
    assert!(validate_full(&borrowed).is_ok());
    assert!(
        super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&borrowed]).is_ok()
    );
    for fault in [
        "unrestricted",
        "parameter-generation",
        "function-generation",
    ] {
        let mut invalid = borrowed.clone();
        let OwnerRecord::Parameter(p) = invalid
            .owners
            .get_mut(&OwnerKey::Parameter(parameter))
            .unwrap()
        else {
            unreachable!()
        };
        match fault {
            "unrestricted" => p.use_mode = ParameterUse::Unrestricted,
            "parameter-generation" => {
                p.header.contract_version = contract::SHARE_GRAPH_CONTRACT_VERSION - 1;
            }
            "function-generation" => {
                let OwnerRecord::Declaration(d) = invalid
                    .owners
                    .get_mut(&OwnerKey::Declaration(target.declaration))
                    .unwrap()
                else {
                    unreachable!()
                };
                d.header.contract_version = contract::SHARE_GRAPH_CONTRACT_VERSION - 1;
            }
            _ => unreachable!(),
        }
        assert!(!memory_reference::accepts(&invalid));
        assert!(validate_full(&invalid).is_err());
        assert!(
            super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&invalid])
                .is_err()
        );
    }
    // The loaned parameter is legal even when unused, but it must never become
    // either an owned return or an escaping task read-result.
    let source = byte_buffer_tests::author(
        "declarations.begin\n(units (module create unused-task (function create unused (visibility private) (parameter create owner (type ByteBuffer) (use borrow)) (returns Unit) (effect (task)) (body (unit)))))\ndeclarations.end\n",
    )
    .unwrap();
    let target = declaration_named(&source, "unused");
    let OwnerRecord::Declaration(d) = &source.owners[&OwnerKey::Declaration(target.declaration)]
    else {
        unreachable!()
    };
    let DeclarationPayload::Function(f) = &d.payload else {
        unreachable!()
    };
    let parameter = f.parameters[0];
    let body = f.body;
    let OwnerRecord::Parameter(p) = &source.owners[&OwnerKey::Parameter(parameter)] else {
        unreachable!()
    };
    let result = p.ty;
    for escapes_as_borrow in [false, true] {
        let mut invalid = source.clone();
        let OwnerRecord::Declaration(d) = invalid
            .owners
            .get_mut(&OwnerKey::Declaration(target.declaration))
            .unwrap()
        else {
            unreachable!()
        };
        let DeclarationPayload::Function(f) = &mut d.payload else {
            unreachable!()
        };
        f.result = result;
        f.result_borrow = escapes_as_borrow.then_some(parameter);
        let OwnerRecord::Expression(e) =
            invalid.owners.get_mut(&OwnerKey::Expression(body)).unwrap()
        else {
            unreachable!()
        };
        e.operation = ExpressionOperation::Local {
            value: LocalValueReference::FunctionParameter(parameter),
        };
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
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
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
