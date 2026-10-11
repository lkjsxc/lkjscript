//! Ordinary metadata crosses exact storage witnesses without acquiring owner custody.
use super::super::owned_storage;
use super::*;

pub(crate) const SOURCE: &str = r#"declarations.begin
(units (module create mixed-storage
  (owned-contract create Indexed (visibility public)
    (self Self) (type-parameter create Self (constraint owned))
    (type-parameter create Item (constraint transferable))
    (method method_a5100000000000000000000000000001 empty (parameters) (returns Self))
    (method method_a5100000000000000000000000000002 push
      (parameters (Item unrestricted) (Self consume)) (returns Self))
    (method method_a5100000000000000000000000000003 get
      (parameters (I64 unrestricted) (Self borrow)) (returns Item))
    (method method_a5100000000000000000000000000004 replace
      (parameters (I64 unrestricted) (Item unrestricted) (Self consume))
      (returns (owned-product (field rest Self) (field value Item)))))
  (function create empty (visibility public) (effect pure)
    (type-parameter create T (constraint transferable)) (returns (owned-sequence T))
    (body (sequence-empty (type (owned-sequence T)))))
  (function create push (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create value (type T))
    (parameter create items (type (owned-sequence T)) (use consume))
    (returns (owned-sequence T))
    (body (sequence-push (type (owned-sequence T)) (local value) (local items))))
  (function create get (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create index (type I64))
    (parameter create items (type (owned-sequence T)) (use borrow)) (returns T)
    (body (sequence-get (type (owned-sequence T)) (local items) (index (local index)))))
  (function create replace (visibility public) (effect pure)
    (type-parameter create T (constraint transferable))
    (parameter create index (type I64)) (parameter create value (type T))
    (parameter create items (type (owned-sequence T)) (use consume))
    (returns (owned-product (field rest (owned-sequence T)) (field value T)))
    (body (sequence-replace (type (owned-sequence T)) (index (local index))
      (local value) (local items))))
  (owned-implementation create Flat (visibility public)
    (type-parameter create T (constraint transferable))
    (contract Indexed) (self (owned-sequence T)) (types T)
    (method method_a5100000000000000000000000000001 empty (types T))
    (method method_a5100000000000000000000000000002 push (types T))
    (method method_a5100000000000000000000000000003 get (types T))
    (method method_a5100000000000000000000000000004 replace (types T)))
  (function create worker (visibility public) (effect (task))
    (type-parameter create Q (constraint owned transferable))
    (type-parameter create T (constraint transferable))
    (implementation-parameter implparam_a5100000000000000000000000000001 ops Indexed Q (types T))
    (parameter create first (type T)) (parameter create second (type T))
    (returns (record (before T) (current T) (removed T)))
    (body (let
      (binding items (type Q) (method-call parameter@worker@implparam_a5100000000000000000000000000001
        Indexed method_a5100000000000000000000000000001))
      (binding items (type Q) (method-call parameter@worker@implparam_a5100000000000000000000000000001
        Indexed method_a5100000000000000000000000000002 (local first) (local items)))
      (binding before (type T) (method-call parameter@worker@implparam_a5100000000000000000000000000001
        Indexed method_a5100000000000000000000000000003 (i64 0) (local items)))
      (binding outcome (type (owned-product (field rest Q) (field value T)))
        (method-call parameter@worker@implparam_a5100000000000000000000000000001
          Indexed method_a5100000000000000000000000000004 (i64 0) (local second) (local items)))
      (in (unpack-owned (type (owned-product (field rest Q) (field value T))) (local outcome)
        (field rest (binding items (type Q))) (field value (binding removed (type T)))
        (in (let (binding current (type T)
          (method-call parameter@worker@implparam_a5100000000000000000000000000001
            Indexed method_a5100000000000000000000000000003 (i64 0) (local items)))
          (in (record structural (field before (local before)) (field current (local current)) (field removed (local removed)))))))))))
  (function create main (visibility public) (effect (task))
    (returns (record
      (left (record (before I64) (current I64) (removed I64)))
      (right (record (before Bool) (current Bool) (removed Bool)))))
    (body (parallel
      (implementation-call worker (types (owned-sequence I64) I64)
        (implementations (implementation Flat (types I64))) (i64 17) (i64 23))
      (implementation-call worker (types (owned-sequence Bool) Bool)
        (implementations (implementation Flat (types Bool))) (bool true) (bool false)))))))
declarations.end"#;

pub(crate) fn prepared_mixed() -> NormalizedProgram {
    prepare_snapshot(&byte_buffer_tests::author_only(SOURCE).unwrap())
}

fn record(fields: &[(&str, NormalizedValue)]) -> NormalizedValue {
    NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(
            fields
                .iter()
                .map(|(name, value)| (Name::new(*name).unwrap(), value.clone()))
                .collect(),
        ),
    })
}

fn expected() -> NormalizedValue {
    record(&[
        (
            "left",
            record(&[
                ("before", NormalizedValue::I64(17)),
                ("current", NormalizedValue::I64(23)),
                ("removed", NormalizedValue::I64(17)),
            ]),
        ),
        (
            "right",
            record(&[
                ("before", NormalizedValue::Bool(true)),
                ("current", NormalizedValue::Bool(false)),
                ("removed", NormalizedValue::Bool(true)),
            ]),
        ),
    ])
}

fn invoke(
    reference: bool,
    source: &crate::platform::kernel::KernelSnapshot,
    program: &NormalizedProgram,
    policy: NormalizedRunPolicy,
    control: &ExecutionControl,
) -> Result<NormalizedValue, ExecutionError> {
    let main = declaration_named(source, "main");
    if reference {
        NormalizedReferenceInterpreter::new(source, program, policy)
            .invoke(main, vec![], None, control)
            .map(|(value, work)| {
                assert_eq!(work.live_handles_after, 0);
                assert_eq!(work.live_call_frames_after, 0);
                value
            })
    } else {
        NormalizedVm::for_test(program, policy)
            .invoke(main, vec![], None, control)
            .map(|(value, work)| {
                assert_eq!(work.live_handles_after, 0);
                assert_eq!(work.live_call_frames_after, 0);
                value
            })
    }
}

#[test]
fn mixed_storage_witnesses_preserve_retained_and_displaced_data_across_joined_calls() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    assert!(!program.implementation_applications.is_empty());
    for application in program.implementation_applications.iter() {
        assert_eq!(
            application
                .implementation_type_parameter_constraints
                .as_ref(),
            &[crate::platform::kernel::TypeParameterConstraints::Transferable]
        );
        assert_eq!(
            application.contract_type_parameter_constraints.as_ref(),
            &[crate::platform::kernel::TypeParameterConstraints::Transferable]
        );
    }
    for reference in [false, true] {
        let storage = owned_storage::StorageObservation::start();
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            expected()
        );
        assert_eq!(storage.live(), (0, 0));
    }
}

#[test]
fn mixed_storage_witnesses_join_cleanup_after_refusal_and_cancellation() {
    let source = byte_buffer_tests::author_only(SOURCE).unwrap();
    let program = prepare_snapshot(&source);
    for reference in [false, true] {
        let storage = owned_storage::StorageObservation::start();
        for checks in [0, 32, 128, 1024, 8192, 131072] {
            match invoke(
                reference,
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::cancel_after_checks(checks),
            ) {
                Ok(value) => assert_eq!(value, expected()),
                Err(error) => assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Cancelled
                ),
            }
            assert_eq!(storage.live(), (0, 0));
        }
        for quota in [0, 256, 4096, 1048576] {
            match invoke(
                reference,
                &source,
                &program,
                NormalizedRunPolicy {
                    maximum_allocated_bytes: Some(quota),
                    ..NormalizedRunPolicy::foreground()
                },
                &ExecutionControl::uncancelled(),
            ) {
                Ok(value) => assert_eq!(value, expected()),
                Err(error) => assert_eq!(
                    error.class,
                    crate::platform::execution::ExecutionFailureClass::Resource
                ),
            }
            assert_eq!(storage.live(), (0, 0));
        }
        assert_eq!(
            invoke(
                reference,
                &source,
                &program,
                NormalizedRunPolicy::foreground(),
                &ExecutionControl::uncancelled()
            )
            .unwrap(),
            expected()
        );
        assert_eq!(storage.live(), (0, 0));
    }
}

#[test]
fn mixed_storage_reads_retain_nested_and_nominal_ordinary_values_after_storage_destruction() {
    for nominal in [false, true] {
        let item_type = if nominal { "Payload" } else { "(list I64)" };
        let list = |a, b| format!("(list I64 (i64 {a}) (i64 {b}))");
        let literal = |a, b| {
            if nominal {
                format!("(record Payload (field Payload::values {}))", list(a, b))
            } else {
                list(a, b)
            }
        };
        let authored = SOURCE
            .replace("(module create mixed-storage", if nominal {
                "(module create mixed-storage (record create Payload (visibility public) (field create values (type (list I64))))"
            } else { "(module create mixed-storage" })
            .replace("(before I64) (current I64) (removed I64)",
                &format!("(before {item_type}) (current {item_type}) (removed {item_type})"))
            .replace("(types (owned-sequence I64) I64)",
                &format!("(types (owned-sequence {item_type}) {item_type})"))
            .replace("(implementation Flat (types I64))", &format!("(implementation Flat (types {item_type}))"))
            .replace("(i64 17) (i64 23)", &format!("{} {}", literal(17, 19), literal(23, 29)))
            .replace("(constraint transferable)", "(constraint capture-safe transferable)");
        let source = byte_buffer_tests::author_only(&authored).unwrap();
        let program = prepare_snapshot(&source);
        let item = |a, b| {
            let values =
                NormalizedValue::list(vec![NormalizedValue::I64(a), NormalizedValue::I64(b)])
                    .unwrap();
            if nominal {
                let declaration = declaration_named(&source, "Payload");
                let index = program
                    .records
                    .iter()
                    .position(|layout| layout.declaration == declaration)
                    .unwrap();
                NormalizedValue::Record(super::super::value::NormalizedRecord::Nominal {
                    layout: super::super::value::RecordLayoutIndex(
                        u32::try_from(index).unwrap(),
                        program.value_origin,
                    ),
                    fields: Arc::new(vec![values]),
                })
            } else {
                values
            }
        };
        let expected = record(&[
            (
                "left",
                record(&[
                    ("before", item(17, 19)),
                    ("current", item(23, 29)),
                    ("removed", item(17, 19)),
                ]),
            ),
            (
                "right",
                record(&[
                    ("before", NormalizedValue::Bool(true)),
                    ("current", NormalizedValue::Bool(false)),
                    ("removed", NormalizedValue::Bool(true)),
                ]),
            ),
        ]);
        for reference in [false, true] {
            let storage = owned_storage::StorageObservation::start();
            assert_eq!(
                invoke(
                    reference,
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                    &ExecutionControl::uncancelled()
                )
                .unwrap(),
                expected
            );
            assert_eq!(storage.live(), (0, 0));
        }
    }
}
