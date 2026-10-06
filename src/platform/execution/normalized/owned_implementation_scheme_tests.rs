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

const PREREQUISITES: &str = r#"
  (external create add (visibility private) (implementation core.i64.add)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (function create cell-plus (visibility public) (effect pure)
    (parameter create value (type OwnedI64Cell) (use consume)) (returns OwnedI64Cell)
    (body (call cell (call add (call extract (local value)) (i64 1)))))
  (owned-implementation create CellPlus (visibility public)
    (contract Keep) (self OwnedI64Cell)
    (method method_c1000000000000000000000000000001 cell-plus))
  (function create keep-through (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c2000000000000000000000000000001 ops Keep T)
    (parameter create value (type T) (use consume)) (returns T)
    (body (method-call parameter@keep-through@implparam_c2000000000000000000000000000001
      Keep method_c1000000000000000000000000000001 (local value))))
  (owned-implementation create Wrapper (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c2000000000000000000000000000002 ops Keep T)
    (contract Keep) (self T)
    (method method_c1000000000000000000000000000001 keep-through (types T)
      (implementations parameter@Wrapper@implparam_c2000000000000000000000000000002)))
  (function create read-through (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c2000000000000000000000000000003 ops Read T)
    (parameter create value (type T) (use borrow)) (returns T (borrow-from value))
    (body (borrow-call
      (method-call parameter@read-through@implparam_c2000000000000000000000000000003
        Read method_c1000000000000000000000000000002 (local value))
      (binding selected (type T)) (in (local selected)))))
  (owned-implementation create WrapperRead (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c2000000000000000000000000000004 ops Read T)
    (contract Read) (self T)
    (method method_c1000000000000000000000000000002 read-through (types T)
      (implementations parameter@WrapperRead@implparam_c2000000000000000000000000000004)))
  (function create task-through (visibility public) (effect (task))
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c2000000000000000000000000000005 ops TaskKeep T)
    (parameter create value (type T) (use consume)) (returns T)
    (body (method-call parameter@task-through@implparam_c2000000000000000000000000000005
      TaskKeep method_c1000000000000000000000000000003 (local value))))
  (owned-implementation create WrapperTask (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c2000000000000000000000000000006 ops TaskKeep T)
    (contract TaskKeep) (self T)
    (method method_c1000000000000000000000000000003 task-through (types T)
      (implementations parameter@WrapperTask@implparam_c2000000000000000000000000000006)))
  (function create prerequisite-main (visibility public) (effect pure)
    (returns (record (first I64) (second I64)))
    (body (let
      (binding first (type OwnedI64Cell) (call cell (i64 23)))
      (binding first (type OwnedI64Cell)
        (implementation-call outer (types OwnedI64Cell)
          (implementations (implementation Wrapper (types OwnedI64Cell)
            (implementations (implementation Wrapper (types OwnedI64Cell)
              (implementations (implementation Plain (types OwnedI64Cell))))))) (local first)))
      (binding second (type OwnedI64Cell) (call cell (i64 41)))
      (binding second (type OwnedI64Cell)
        (implementation-call outer (types OwnedI64Cell)
          (implementations (implementation Wrapper (types OwnedI64Cell)
            (implementations (implementation Wrapper (types OwnedI64Cell)
              (implementations concrete@CellPlus))))) (local second)))
      (in (record structural (field first (call extract (local first)))
        (field second (call extract (local second))))))))
"#;

pub(crate) fn prerequisite_source() -> String {
    SOURCE.replace("  (function create scheme-main", &format!("{PREREQUISITES}  (function create scheme-main"))
        .replace("(implementation PlainRead (types ByteBuffer))",
            "(implementation WrapperRead (types ByteBuffer) (implementations (implementation PlainRead (types ByteBuffer))))")
        .replace("(implementation PlainTask (types ByteBuffer))",
            "(implementation WrapperTask (types ByteBuffer) (implementations (implementation PlainTask (types ByteBuffer))))")
}

pub(crate) fn duplicate_dag_source(depth: usize) -> String {
    let mut declarations = String::from(
        r#"
  (owned-implementation create Both (visibility public)
    (type-parameter create T (constraint owned))
    (implementation-parameter implparam_c4000000000000000000000000000001 first Keep T)
    (implementation-parameter implparam_c4000000000000000000000000000002 second Keep T)
    (contract Keep) (self T)
    (method method_c1000000000000000000000000000001 keep-through (types T)
      (implementations parameter@Both@implparam_c4000000000000000000000000000001)))
"#,
    );
    for index in 0..=depth {
        let id = format!("implparam_c3{index:030x}");
        let body = if index == depth {
            format!(
                "(method-call parameter@dag-{index}@{id} Keep method_c1000000000000000000000000000001 (local value))"
            )
        } else {
            format!(
                "(implementation-call dag-{} (types T) (implementations (implementation Both (types T) (implementations parameter@dag-{index}@{id} parameter@dag-{index}@{id}))) (local value))",
                index + 1
            )
        };
        declarations.push_str(&format!(
            r#"
  (function create dag-{index} (visibility public) (effect pure)
    (type-parameter create T (constraint owned))
    (implementation-parameter {id} ops Keep T)
    (parameter create value (type T) (use consume)) (returns T)
    (body {body}))
"#
        ));
    }
    declarations.push_str(
        r#"
  (function create dag-main (visibility public) (effect pure) (returns I64)
    (body (let (binding value (type OwnedI64Cell) (call cell (i64 17)))
      (binding value (type OwnedI64Cell)
        (implementation-call dag-0 (types OwnedI64Cell)
          (implementations (implementation Plain (types OwnedI64Cell))) (local value)))
      (in (call extract (local value))))))
"#,
    );
    prerequisite_source().replace(
        "  (function create scheme-main",
        &format!("{declarations}  (function create scheme-main"),
    )
}

pub(crate) fn duplicate_dag_fixture(
    depth: usize,
) -> (crate::platform::kernel::KernelSnapshot, NormalizedProgram) {
    let source = byte_buffer_tests::author_only(&duplicate_dag_source(depth)).unwrap();
    let program = prepare_snapshot(&source);
    (source, program)
}

/// Exercise derived physical admission independently of producer layout proof capacity.
pub(crate) fn prepared_duplicate_dag(depth: usize) -> NormalizedProgram {
    let (source, mut program) = duplicate_dag_fixture(1);
    let both = declaration_named(&source, "Both");
    let template = program
        .implementation_applications
        .iter()
        .find(|node| node.implementation == both)
        .unwrap()
        .clone();
    let mut previous = template.implementations[0].clone();
    Arc::make_mut(&mut previous).identity = 0;
    Arc::make_mut(&mut previous).depth = 0;
    let mut nodes = vec![previous.clone()];
    for identity in 1..=depth {
        let mut node = template.clone();
        Arc::make_mut(&mut node).identity = u32::try_from(identity).unwrap();
        Arc::make_mut(&mut node).depth = identity;
        Arc::make_mut(&mut node).implementations = Arc::from([previous.clone(), previous]);
        nodes.push(node.clone());
        previous = node;
    }
    program.implementation_applications = nodes.into();
    program
}

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
    let (
        super::super::prepare::NormalizedFunctionBody::Code(first_code),
        super::super::prepare::NormalizedFunctionBody::Code(second_code),
    ) = (&applications[0].body, &applications[1].body)
    else {
        panic!("graph code")
    };
    assert!(Arc::ptr_eq(
        &first_code.instructions,
        &second_code.instructions
    ));
    assert_ne!(applications[0].callsites, applications[1].callsites);
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

#[test]
fn prerequisite_dags_preserve_dispatch_share_code_and_compose_reads_and_parallel_custody() {
    let source = byte_buffer_tests::author_only(&prerequisite_source()).unwrap();
    let program = prepare_snapshot(&source);
    let outer = declaration_named(&source, "outer");
    let wrapper = declaration_named(&source, "Wrapper");
    let applications = program
        .functions
        .iter()
        .filter(|function| {
            function.declaration == outer
                && function
                    .implementation_arguments
                    .first()
                    .is_some_and(|witness| witness.implementation == wrapper)
        })
        .collect::<Vec<_>>();
    assert_eq!(applications.len(), 2);
    assert_eq!(
        applications[0].type_arguments,
        applications[1].type_arguments
    );
    let a = &applications[0].implementation_arguments[0];
    let b = &applications[1].implementation_arguments[0];
    assert_eq!(a.implementation, b.implementation);
    assert_eq!(
        a.implementation_type_arguments,
        b.implementation_type_arguments
    );
    assert_eq!(
        a.implementations[0].implementation,
        b.implementations[0].implementation
    );
    assert_ne!(
        a.implementations[0].implementations[0].implementation,
        b.implementations[0].implementations[0].implementation
    );
    let (
        super::super::prepare::NormalizedFunctionBody::Code(ac),
        super::super::prepare::NormalizedFunctionBody::Code(bc),
    ) = (&applications[0].body, &applications[1].body)
    else {
        panic!("graph code")
    };
    assert!(Arc::ptr_eq(&ac.instructions, &bc.instructions));
    assert_ne!(applications[0].callsites, applications[1].callsites);
    let answer = NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (Name::new("first").unwrap(), NormalizedValue::I64(23)),
            (Name::new("second").unwrap(), NormalizedValue::I64(42)),
        ]),
    });
    for reference in [false, true] {
        let cells = owned_i64_cell::StorageObservation::start();
        let buffers = byte_buffer::StorageObservation::start();
        let composites = owned_storage::StorageObservation::start();
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                "prerequisite-main",
                Vec::new(),
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            answer
        );
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                "scheme-main",
                vec![NormalizedValue::I64(37)],
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            expected(37)
        );
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
        assert_eq!(composites.live(), (0, 0));
    }
}

#[test]
fn prerequisite_reads_clean_up_on_trap_cancellation_and_allocation_refusal() {
    let source = byte_buffer_tests::author_only(&prerequisite_source()).unwrap();
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
                Vec::new(),
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
            crate::platform::execution::ExecutionFailureClass::Trap
        );
        let mut cancelled = 0;
        let mut completed = 0;
        for checks in [0, 32, 128, 512, 2048, 8192, 524288] {
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
fn forged_nested_prerequisites_and_shared_callsite_tables_reject_without_leaking_owners() {
    let source = byte_buffer_tests::author_only(&prerequisite_source()).unwrap();
    let program = prepare_snapshot(&source);
    let outer = declaration_named(&source, "outer");
    let wrapper = declaration_named(&source, "Wrapper");
    let read = declaration_named(&source, "Read");
    for fault in 0..4 {
        let mut forged = program.clone();
        let mut changed = 0;
        for function in Arc::make_mut(&mut forged.functions) {
            if function.declaration != outer
                || !function
                    .implementation_arguments
                    .first()
                    .is_some_and(|witness| witness.implementation == wrapper)
            {
                continue;
            }
            changed += 1;
            match fault {
                0 => {
                    Arc::make_mut(&mut Arc::make_mut(&mut function.implementation_arguments)[0])
                        .implementations = Arc::from([])
                }
                1 => {
                    let witness = Arc::make_mut(
                        &mut Arc::make_mut(&mut function.implementation_arguments)[0],
                    );
                    Arc::make_mut(&mut Arc::make_mut(&mut witness.implementations)[0]).contract =
                        read;
                }
                2 => function.callsites = Arc::from([]),
                _ => {
                    let entry = &mut Arc::make_mut(&mut function.callsites)[0];
                    entry.1 = super::super::prepare::NormalizedCallSite::Parallel {
                        left: super::super::value::FunctionIndex(0, program.value_origin),
                        left_types: Arc::from([]),
                        right: super::super::value::FunctionIndex(0, program.value_origin),
                        right_types: Arc::from([]),
                    };
                }
            }
        }
        assert_eq!(changed, 2);
        let cells = owned_i64_cell::StorageObservation::start();
        let error = invoke(
            false,
            &source,
            &forged,
            "prerequisite-main",
            Vec::new(),
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap_err();
        assert!(matches!(
            error.class,
            crate::platform::execution::ExecutionFailureClass::Trap
                | crate::platform::execution::ExecutionFailureClass::Infrastructure
        ));
        assert_eq!(cells.live(), (0, 0));
    }
}

#[test]
fn duplicated_prerequisite_edges_preserve_exact_shared_dag_dispatch() {
    // Source, concrete preparation and execution compose at the same full depth.
    // This was already supported before whole-node sharing.
    let source = byte_buffer_tests::author_only(&duplicate_dag_source(24)).unwrap();
    let program = prepare_snapshot(&source);
    let both = declaration_named(&source, "Both");
    let nodes = program
        .implementation_applications
        .iter()
        .filter(|node| node.implementation == both)
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 24);
    for node in nodes {
        assert_eq!(node.implementations.len(), 2);
        assert_eq!(
            node.implementations[0].identity,
            node.implementations[1].identity
        );
        assert!(Arc::ptr_eq(
            &node.implementations[0].implementations,
            &node.implementations[1].implementations
        ));
    }
    let entry = declaration_named(&source, "dag-main");
    let cells = owned_i64_cell::StorageObservation::start();
    let (value, work) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .invoke(entry, Vec::new(), None, &ExecutionControl::uncancelled())
        .unwrap();
    assert_eq!(value, NormalizedValue::I64(17));
    assert!(work.instructions < 4096);
    assert!(work.allocated_bytes < 2 * 1024 * 1024);
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(
        invoke(
            true,
            &source,
            &program,
            "dag-main",
            Vec::new(),
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled()
        )
        .unwrap(),
        NormalizedValue::I64(17)
    );
    assert_eq!(cells.live(), (0, 0));
}

#[test]
fn repeated_intern_identity_cannot_hide_a_forged_descendant() {
    let source = byte_buffer_tests::author_only(&duplicate_dag_source(4)).unwrap();
    let program = prepare_snapshot(&source);
    let both = declaration_named(&source, "Both");
    let read = declaration_named(&source, "Read");
    let mut forged = program.clone();
    let function = Arc::make_mut(&mut forged.functions)
        .iter_mut()
        .find(|function| {
            function.declaration == declaration_named(&source, "dag-4")
                && function
                    .implementation_arguments
                    .first()
                    .is_some_and(|w| w.implementation == both)
        })
        .unwrap();
    let root = Arc::make_mut(&mut Arc::make_mut(&mut function.implementation_arguments)[0]);
    // The sibling retains the same intern ID; its descendant has private mutated storage.
    let children = Arc::make_mut(&mut root.implementations);
    assert_eq!(children[0].identity, children[1].identity);
    let changed = Arc::make_mut(&mut children[1]);
    Arc::make_mut(&mut Arc::make_mut(&mut changed.implementations)[0]).contract = read;
    let cells = owned_i64_cell::StorageObservation::start();
    assert!(
        invoke(
            false,
            &source,
            &forged,
            "dag-main",
            Vec::new(),
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled()
        )
        .is_err()
    );
    assert_eq!(cells.live(), (0, 0));
}
