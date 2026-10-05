//! Applied schemes retain concrete callable identity through forwarding and custody.
use super::super::{byte_buffer, owned_i64_cell, owned_storage};
use super::*;

const SOURCE: &str = r#"declarations.begin
(units (module create schemes
  (external create cell (visibility private) (implementation core.cell.create)
    (parameter create value (type I64)) (returns OwnedI64Cell))
  (external create extract (visibility private) (implementation core.cell.extract)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64))
  (external create empty (visibility private) (implementation core.buffer.empty) (returns ByteBuffer))
  (external create push (visibility private) (implementation core.buffer.push)
    (parameter create octet (type I64))
    (parameter create value (type ByteBuffer) (use consume)) (returns ByteBuffer))
  (external create freeze (visibility private) (implementation core.buffer.freeze)
    (parameter create value (type ByteBuffer) (use consume)) (returns Bytes))
  (external create length (visibility private) (implementation core.buffer.length)
    (parameter create value (type ByteBuffer) (use borrow)) (returns I64))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (owned-contract create Keep (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_c1000000000000000000000000000001 keep
      (parameters (Self consume)) (returns Self)))
  (owned-contract create Read (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_c1000000000000000000000000000002 read
      (parameters (Self borrow)) (returns Self (borrow-from 0))))
  (owned-contract create TaskKeep (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (method method_c1000000000000000000000000000003 keep
      (parameters (Self consume)) (returns Self) (effect (task))))
  (function create keep (visibility public) (effect pure)
    (type-parameter create Value (constraint owned))
    (parameter create value (type Value) (use consume)) (returns Value)
    (body (local value)))
  (function create read (visibility public) (effect pure)
    (type-parameter create Value (constraint owned))
    (parameter create value (type Value) (use borrow))
    (returns Value (borrow-from value)) (body (local value)))
  (function create task-keep (visibility public) (effect (task))
    (type-parameter create Value (constraint owned))
    (parameter create value (type Value) (use consume)) (returns Value)
    (body (local value)))
  (owned-implementation create Plain (visibility public)
    (type-parameter create Item (constraint owned))
    (contract Keep) (self Item)
    (method method_c1000000000000000000000000000001 keep (types Item)))
  (owned-implementation create PlainRead (visibility public)
    (type-parameter create Item (constraint owned))
    (contract Read) (self Item)
    (method method_c1000000000000000000000000000002 read (types Item)))
  (owned-implementation create PlainTask (visibility public)
    (type-parameter create Item (constraint owned))
    (contract TaskKeep) (self Item)
    (method method_c1000000000000000000000000000003 task-keep (types Item)))
  (function create inner (visibility private) (effect pure)
    (type-parameter create B (constraint owned transferable))
    (implementation-parameter implparam_c1000000000000000000000000000001 ops Keep B)
    (parameter create value (type B) (use consume)) (returns B)
    (body (method-call parameter@inner@implparam_c1000000000000000000000000000001
      Keep method_c1000000000000000000000000000001 (local value))))
  (function create outer (visibility private) (effect pure)
    (type-parameter create A (constraint owned transferable))
    (implementation-parameter implparam_c1000000000000000000000000000002 ops Keep A)
    (parameter create value (type A) (use consume)) (returns A)
    (body (implementation-call inner (types A)
      (implementations parameter@outer@implparam_c1000000000000000000000000000002)
      (local value))))
  (function create worker (visibility private) (effect (task))
    (type-parameter create A (constraint owned transferable))
    (implementation-parameter implparam_c1000000000000000000000000000003 ops Keep A)
    (parameter create value (type A) (use consume)) (returns A)
    (body (implementation-call outer (types A)
      (implementations parameter@worker@implparam_c1000000000000000000000000000003)
      (local value))))
  (function create task-worker (visibility private) (effect (task))
    (type-parameter create A (constraint owned transferable))
    (implementation-parameter implparam_c1000000000000000000000000000004 ops TaskKeep A)
    (parameter create value (type A) (use consume)) (returns A)
    (body (method-call parameter@task-worker@implparam_c1000000000000000000000000000004
      TaskKeep method_c1000000000000000000000000000003 (local value))))
  (function create scheme-main (visibility public) (effect (task))
    (parameter create n (type I64)) (returns (record (scalar I64) (bytes Bytes) (view I64)))
    (body (let
      (binding scalar (type OwnedI64Cell) (call cell (local n)))
      (binding buffer (type ByteBuffer) (call empty))
      (binding buffer (type ByteBuffer) (call push (i64 255) (local buffer)))
      (binding view (type I64)
        (borrow-call (method-call (implementation PlainRead (types ByteBuffer))
          Read method_c1000000000000000000000000000002 (local buffer))
          (binding selected (type ByteBuffer)) (in (call length (local selected)))))
      (binding pair (type (owned-product (field left OwnedI64Cell) (field right ByteBuffer)))
        (parallel
          (implementation-call worker (types OwnedI64Cell)
            (implementations (implementation Plain (types OwnedI64Cell))) (local scalar))
          (implementation-call task-worker (types ByteBuffer)
            (implementations (implementation PlainTask (types ByteBuffer))) (local buffer))))
      (in (unpack-owned (type (owned-product (field left OwnedI64Cell) (field right ByteBuffer))) (local pair)
        (field left (binding scalar (type OwnedI64Cell)))
        (field right (binding buffer (type ByteBuffer)))
        (in (record structural
          (field scalar (call extract (local scalar)))
          (field bytes (call freeze (local buffer))) (field view (local view)))))))))
  (function create scheme-failure (visibility public) (effect pure) (returns I64)
    (body (let (binding buffer (type ByteBuffer) (call empty))
      (in (borrow-call (method-call (implementation PlainRead (types ByteBuffer))
        Read method_c1000000000000000000000000000002 (local buffer))
        (binding selected (type ByteBuffer))
        (in (call divide (call length (local selected)) (i64 0))))))))))
declarations.end"#;

fn invoke(
    reference: bool,
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    name: &str,
    arguments: Vec<NormalizedValue>,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> Result<NormalizedValue, ExecutionError> {
    let declaration = declaration_named(source, name);
    if reference {
        let (value, work) = NormalizedReferenceInterpreter::new(source, program, policy).invoke(
            declaration,
            arguments,
            None,
            control,
        )?;
        assert_eq!(work.live_call_frames_after, 0);
        assert_eq!(work.live_handles_after, 0);
        Ok(value)
    } else {
        let (value, work) = NormalizedVm::for_test(program, policy).invoke(
            declaration,
            arguments,
            None,
            control,
        )?;
        assert_eq!(work.live_call_frames_after, 0);
        assert_eq!(work.live_handles_after, 0);
        Ok(value)
    }
}

fn expected(n: i64) -> NormalizedValue {
    NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (
                Name::new("bytes").unwrap(),
                NormalizedValue::bytes(vec![255]),
            ),
            (Name::new("scalar").unwrap(), NormalizedValue::I64(n)),
            (Name::new("view").unwrap(), NormalizedValue::I64(1)),
        ]),
    })
}

#[test]
fn applied_schemes_forward_scopes_map_generic_reads_and_return_parallel_owners() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        for n in [i64::MIN, 37, i64::MAX] {
            let cells = owned_i64_cell::StorageObservation::start();
            let buffers = byte_buffer::StorageObservation::start();
            let composites = owned_storage::StorageObservation::start();
            assert_eq!(
                invoke(
                    reference,
                    &source,
                    &program,
                    "scheme-main",
                    vec![NormalizedValue::I64(n)],
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled()
                )
                .unwrap(),
                expected(n)
            );
            assert_eq!(cells.created(), 1);
            assert_eq!(buffers.created(), 1);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
    }
    // One nominal scheme is independently applied to multiple concrete types.
    let main = SOURCE
        .replace(
            "(implementation PlainTask (types ByteBuffer))",
            "(implementation Plain (types ByteBuffer))",
        )
        .replace(
            "(implementation-call task-worker (types ByteBuffer)",
            "(implementation-call worker (types ByteBuffer)",
        );
    let source = byte_buffer_tests::author_only(&main).unwrap();
    let program = prepare_snapshot(&source);
    let worker = declaration_named(&source, "worker");
    let applications = program
        .functions
        .iter()
        .filter(|f| f.declaration == worker && !f.type_arguments.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(applications.len(), 2);
    assert_ne!(
        applications[0].type_arguments,
        applications[1].type_arguments
    );
    assert_ne!(
        applications[0].implementation_arguments[0].implementation_type_arguments,
        applications[1].implementation_arguments[0].implementation_type_arguments
    );
    for reference in [false, true] {
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                "scheme-main",
                vec![NormalizedValue::I64(19)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            expected(19)
        );
    }
}

#[test]
fn applied_schemes_release_loans_and_owners_on_trap_cancellation_and_quota_refusal() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let cells = owned_i64_cell::StorageObservation::start();
        let buffers = byte_buffer::StorageObservation::start();
        let composites = owned_storage::StorageObservation::start();
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                "scheme-failure",
                vec![],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
            crate::platform::execution::ExecutionFailureClass::Trap
        );
        assert_eq!(buffers.live(), (0, 0));
        let mut cancelled = 0;
        let mut completed = 0;
        for checks in [0, 32, 128, 512, 2048, 8192, 131072] {
            match invoke(
                reference,
                &source,
                &program,
                "scheme-main",
                vec![NormalizedValue::I64(41)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::cancel_after_checks(checks),
            ) {
                Ok(value) => {
                    assert_eq!(value, expected(41));
                    completed += 1;
                }
                Err(error) => {
                    assert_eq!(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Cancelled
                    );
                    cancelled += 1;
                }
            }
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
        assert!(cancelled > 0 && completed > 0);
        for maximum in [0, 256, 1024] {
            let policy = NormalizedRunPolicy {
                maximum_allocated_bytes: Some(maximum),
                ..NormalizedRunPolicy::foreground()
            };
            assert_eq!(
                invoke(
                    reference,
                    &source,
                    &program,
                    "scheme-main",
                    vec![NormalizedValue::I64(41)],
                    policy,
                    &ExecutionControl::uncancelled()
                )
                .unwrap_err()
                .class,
                crate::platform::execution::ExecutionFailureClass::Resource
            );
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
            assert_eq!(composites.live(), (0, 0));
        }
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                "scheme-main",
                vec![NormalizedValue::I64(41)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            expected(41)
        );
    }
}

#[test]
fn parallel_scheme_application_cannot_retarget_a_specialized_callable_type() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    let worker = declaration_named(&source, "worker");
    let (index, instance) = program
        .functions
        .iter()
        .enumerate()
        .find(|(_, f)| f.declaration == worker && !f.type_arguments.is_empty())
        .unwrap();
    let target = super::super::value::FunctionIndex(index as u32, program.value_origin);
    let control = ExecutionControl::uncancelled();
    let wrong = crate::platform::kernel::encode_type_object(
        &crate::platform::kernel::TypeObject::new(crate::platform::kernel::TypeForm::ByteBuffer)
            .unwrap(),
    )
    .unwrap()
    .0;
    assert_ne!(instance.type_arguments.as_ref(), &[wrong]);
    assert!(
        super::super::vm::transfer::TaskApplication::bind(
            &program,
            target,
            Arc::from([wrong]),
            &control,
            &mut |_| Ok(())
        )
        .is_err()
    );
    assert!(
        super::super::vm::transfer::TaskApplication::bind(
            &program,
            target,
            Arc::clone(&instance.type_arguments),
            &control,
            &mut |_| Ok(())
        )
        .is_ok()
    );
}

#[test]
fn applied_scheme_phantom_arguments_keep_distinct_identity_at_equal_self_and_callable_types() {
    let probe = r#"
  (function create phantom-main (visibility public) (effect pure)
    (returns (record (first I64) (second I64)))
    (body (let
      (binding first (type OwnedI64Cell) (call cell (i64 23)))
      (binding first (type OwnedI64Cell)
        (implementation-call outer (types OwnedI64Cell)
          (implementations (implementation Plain (types OwnedI64Cell ByteBuffer))) (local first)))
      (binding second (type OwnedI64Cell) (call cell (i64 41)))
      (binding second (type OwnedI64Cell)
        (implementation-call outer (types OwnedI64Cell)
          (implementations (implementation Plain (types OwnedI64Cell OwnedI64Cell))) (local second)))
      (in (record structural (field first (call extract (local first)))
        (field second (call extract (local second))))))))
"#;
    let input = SOURCE
        .replace(
            "(contract Keep) (self Item)",
            "(type-parameter create Phantom (constraint owned)) (contract Keep) (self Item)",
        )
        .replace(
            "(implementation Plain (types OwnedI64Cell))",
            "(implementation Plain (types OwnedI64Cell ByteBuffer))",
        )
        .replace(
            "  (function create scheme-main",
            &format!("{probe}  (function create scheme-main"),
        );
    let source = byte_buffer_tests::author_only(&input).unwrap();
    let program = prepare_snapshot(&source);
    let outer = declaration_named(&source, "outer");
    let applications = program
        .functions
        .iter()
        .filter(|f| f.declaration == outer && !f.type_arguments.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(applications.len(), 2);
    assert_eq!(
        applications[0].type_arguments,
        applications[1].type_arguments
    );
    let first = &applications[0].implementation_arguments[0];
    let second = &applications[1].implementation_arguments[0];
    assert_eq!(first.implementation, second.implementation);
    assert_eq!(first.self_type, second.self_type);
    assert_eq!(first.type_arguments, second.type_arguments);
    assert_ne!(
        first.implementation_type_arguments,
        second.implementation_type_arguments
    );
    let expected = NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (Name::new("first").unwrap(), NormalizedValue::I64(23)),
            (Name::new("second").unwrap(), NormalizedValue::I64(41)),
        ]),
    });
    for reference in [false, true] {
        let cells = owned_i64_cell::StorageObservation::start();
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                "phantom-main",
                vec![],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            expected
        );
        assert_eq!(cells.created(), 2);
        assert_eq!(cells.live(), (0, 0));
    }
}
