//! Generic group construction with an intermediate mixed pair and exact custody.
use super::*;
use crate::platform::execution::normalized::{
    byte_buffer::StorageObservation as Buffers, owned_product::StorageObservation as Products,
    prepare::NormalizedEntryPoint, value::NormalizedRecord,
};
use crate::platform::kernel::{StructuralTypeField, TypeObject, encode_type_object};

const COMBINATOR: &str = r#"declarations.begin
(units (module create generic-group
  (function create transfer (visibility public) (effect (task))
    (type-parameter create U (constraint owned))
    (parameter create value (type U) (use consume)) (returns U)
    (body (local value)))
  (function create keep (visibility public) (effect (task))
    (type-parameter create T)
    (parameter create values (type (list T))) (returns (list T))
    (body (local values)))
  (function create join (visibility public) (effect (task))
    (type-parameter create T (constraint transferable))
    (type-parameter create U (constraint owned transferable))
    (implementation-parameter implparam_95000000000000000000000000000001 ops abstraction::Storage U)
    (parameter create values (type (list T)))
    (parameter create value (type U) (use consume))
    (returns (record (scalar I64) (values (list T))))
    (body (let
      (binding pair (type (owned-product (field left U) (field right (list T))))
        (parallel (call transfer (types U) (local value))
          (call keep (types T) (local values))))
      (in (unpack-owned (type (owned-product (field left U) (field right (list T))))
        (local pair)
        (field left (binding returned (type U)))
        (field right (binding kept (type (list T))))
        (in (record structural
          (field scalar (implementation-call abstraction::consume (types U)
            (implementations parameter@join@implparam_95000000000000000000000000000001)
            (local returned)))
          (field values (local kept)))))))))
  (function create integer-case (visibility public) (effect (task))
    (parameter create n (type I64))
    (parameter create values (type (list I64)))
    (returns (record (scalar I64) (values (list I64))))
    (body (let
      (binding value (type OwnedI64Cell) (call cell::cell-create (local n)))
      (in (implementation-call join (types I64 OwnedI64Cell)
        (implementations concrete@cell::Scalar) (local values) (local value))))))
  (function create text-case (visibility public) (effect (task))
    (parameter create values (type (list Text)))
    (returns (record (scalar I64) (values (list Text))))
    (body (let
      (binding value (type ByteBuffer) (call buffer::buffer-create (i64 197)))
      (in (implementation-call join (types Text ByteBuffer)
        (implementations concrete@buffer::Octets) (local values) (local value))))))))
declarations.end"#;

fn transfer_source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-result-library.lkjc"),
            COMBINATOR,
        ]
        .join("\n"),
    )
    .unwrap()
}

fn arguments(name: &str, n: i64) -> Vec<NormalizedValue> {
    if name == "integer-case" {
        vec![NormalizedValue::I64(n), values(name)]
    } else {
        vec![values(name)]
    }
}

fn values(name: &str) -> NormalizedValue {
    NormalizedValue::list(if name == "integer-case" {
        vec![
            NormalizedValue::I64(i64::MIN),
            NormalizedValue::I64(i64::MAX),
        ]
    } else {
        vec![
            NormalizedValue::Text(Arc::from("first")),
            NormalizedValue::Text(Arc::from("complete second payload")),
        ]
    })
    .unwrap()
}

fn expected(name: &str, n: i64) -> NormalizedValue {
    NormalizedValue::Record(NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (
                Name::new("scalar").unwrap(),
                NormalizedValue::I64(if name == "integer-case" { n } else { 1 }),
            ),
            (Name::new("values").unwrap(), values(name)),
        ]),
    })
}

#[test]
fn parallel_transfer_generic_intermediate_pair_is_prepared_for_each_concrete_application() {
    let source = transfer_source();
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let digest = |form| {
        encode_type_object(&TypeObject::new(form).unwrap())
            .unwrap()
            .0
    };
    for (ordinary, owned) in [
        (TypeForm::I64, TypeForm::OwnedI64Cell),
        (TypeForm::Text, TypeForm::ByteBuffer),
    ] {
        let left = digest(owned);
        let right = digest(TypeForm::List {
            item: digest(ordinary),
        });
        let pair = digest(TypeForm::OwnedProduct {
            fields: vec![
                StructuralTypeField {
                    name: Name::new("left").unwrap(),
                    ty: left,
                },
                StructuralTypeField {
                    name: Name::new("right").unwrap(),
                    ty: right,
                },
            ],
        });
        assert!(
            !source.types.contains_key(&pair),
            "pair is not a stored concrete signature root"
        );
        assert!(
            program.types.contains_key(&pair),
            "intermediate pair must be derived"
        );
    }
    for reference in [false, true] {
        for name in ["integer-case", "text-case"] {
            for n in [i64::MIN, -137, i64::MAX] {
                let cells = Cells::start();
                let buffers = Buffers::start();
                let products = Products::start();
                let entry = declaration_named(&source, name);
                let control = ExecutionControl::uncancelled();
                let result = if reference {
                    let (value, work) = NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, arguments(name, n), None, &control)
                    .unwrap();
                    assert_eq!(work.live_call_frames_after, 0);
                    assert_eq!(work.live_handles_after, 0);
                    value
                } else {
                    let (value, work) =
                        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                            .invoke(entry, arguments(name, n), None, &control)
                            .unwrap();
                    assert_eq!(work.parallel_scopes, 1);
                    assert_eq!(work.live_call_frames_after, 0);
                    assert_eq!(work.live_handles_after, 0);
                    value
                };
                assert_eq!(result, expected(name, n));
                assert_eq!(cells.created(), usize::from(name == "integer-case"));
                assert_eq!(buffers.created(), usize::from(name == "text-case"));
                assert_eq!(products.created(), 1);
                assert_eq!(cells.live(), (0, 0));
                assert_eq!(buffers.live(), (0, 0));
                assert_eq!(products.live(), (0, 0));
            }
        }
    }
}

#[test]
fn parallel_transfer_generic_pair_reservations_have_exact_quota_and_joined_cleanup() {
    use crate::platform::execution::ExecutionFailureClass;

    let source = transfer_source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "integer-case");
    for reference in [false, true] {
        let run = |policy: NormalizedRunPolicy, control: &ExecutionControl| {
            let cells = Cells::start();
            let products = Products::start();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(entry, arguments("integer-case", -137), None, control)
                    .map(|(value, work)| {
                        (
                            value,
                            work.expressions,
                            work.allocated_bytes,
                            work.collection_items,
                        )
                    })
            } else {
                NormalizedVm::for_test(&program, policy)
                    .invoke(entry, arguments("integer-case", -137), None, control)
                    .map(|(value, work)| {
                        (
                            value,
                            work.instructions,
                            work.allocated_bytes,
                            work.collection_items,
                        )
                    })
            };
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(products.live(), (0, 0));
            result
        };
        let (value, steps, bytes, items) = run(
            NormalizedRunPolicy::foreground(),
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
        assert_eq!(value, expected("integer-case", -137));
        let exact = NormalizedRunPolicy {
            instruction_steps: Some(steps),
            maximum_allocated_bytes: Some(bytes),
            maximum_collection_items: Some(items),
            ..NormalizedRunPolicy::foreground()
        };
        assert_eq!(
            run(exact, &ExecutionControl::uncancelled()).unwrap(),
            (value.clone(), steps, bytes, items)
        );
        for policy in [
            NormalizedRunPolicy {
                instruction_steps: Some(steps - 1),
                ..exact
            },
            NormalizedRunPolicy {
                maximum_allocated_bytes: Some(bytes - 1),
                ..exact
            },
            NormalizedRunPolicy {
                maximum_collection_items: Some(items - 1),
                ..exact
            },
        ] {
            assert_eq!(
                run(policy, &ExecutionControl::uncancelled())
                    .unwrap_err()
                    .class,
                ExecutionFailureClass::Resource
            );
        }
        let mut cancelled = 0;
        let mut completed = 0;
        for checks in [0, 16, 64, 128, 256, 512, 1_024, 4_096, 32_768] {
            match run(exact, &ExecutionControl::cancel_after_checks(checks)) {
                Ok((actual, ..)) => {
                    assert_eq!(actual, value);
                    completed += 1;
                }
                Err(error) => {
                    assert_eq!(error.class, ExecutionFailureClass::Cancelled);
                    cancelled += 1;
                }
            }
        }
        assert!(cancelled > 0 && completed > 0);
    }
}

#[test]
fn parallel_transfer_generic_group_forwards_distinct_nominal_witnesses_into_children() {
    let input = r#"declarations.begin
(units (module create forwarded-group
  (function create forward-group (visibility public) (effect (task))
    (type-parameter create U (constraint owned transferable))
    (implementation-parameter implparam_96000000000000000000000000000001 left-ops abstraction::Storage U)
    (implementation-parameter implparam_96000000000000000000000000000002 right-ops abstraction::Storage U)
    (parameter create n (type I64))
    (parameter create left (type U) (use consume))
    (parameter create right (type U) (use consume))
    (returns (record (left I64) (right I64)))
    (body (let
      (binding pair (type (owned-product (field left U) (field right U)))
        (parallel
          (implementation-call generic-workers::forward (types U)
            (implementations parameter@forward-group@implparam_96000000000000000000000000000001)
            (local n) (local left))
          (implementation-call generic-workers::forward (types U)
            (implementations parameter@forward-group@implparam_96000000000000000000000000000002)
            (local n) (local right))))
      (in (unpack-owned (type (owned-product (field left U) (field right U))) (local pair)
        (field left (binding a (type U)))
        (field right (binding b (type U)))
        (in (record structural
          (field left (implementation-call abstraction::consume (types U)
            (implementations parameter@forward-group@implparam_96000000000000000000000000000001)
            (local a)))
          (field right (implementation-call abstraction::consume (types U)
            (implementations parameter@forward-group@implparam_96000000000000000000000000000002)
            (local b))))))))))
  (function create forward-main (visibility public) (effect (task))
    (parameter create n (type I64)) (returns (record (left I64) (right I64)))
    (body (let
      (binding left (type OwnedI64Cell) (call cell::cell-create (i64 0)))
      (binding right (type OwnedI64Cell) (call cell::cell-create (i64 0)))
      (in (implementation-call forward-group (types OwnedI64Cell)
        (implementations concrete@cell::Scalar concrete@generic-selection::Alternate)
        (local n) (local left) (local right))))))))
declarations.end"#;
    let source =
        byte_buffer_tests::author_only(&format!("{}\n{input}", witnessed_source())).unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "forward-main");
    for reference in [false, true] {
        for n in [i64::MIN, -137, i64::MAX] {
            let cells = Cells::start();
            let products = Products::start();
            let control = ExecutionControl::uncancelled();
            let actual = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![NormalizedValue::I64(n)], None, &control)
                .unwrap()
                .0
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![NormalizedValue::I64(n)], None, &control)
                    .unwrap()
                    .0
            };
            assert_eq!(
                actual,
                NormalizedValue::Record(NormalizedRecord::Structural {
                    fields: Arc::new(vec![
                        (Name::new("left").unwrap(), NormalizedValue::I64(n)),
                        (Name::new("right").unwrap(), NormalizedValue::I64(99)),
                    ]),
                })
            );
            assert_eq!(cells.created(), 2);
            assert_eq!(products.created(), 1);
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(products.live(), (0, 0));
        }
    }
}

#[test]
fn parallel_transfer_raw_phantom_application_checks_transfer_beside_capture_safety() {
    let input = r#"declarations.begin
(units (module create raw-transfer
  (function create phantom (visibility public) (effect pure)
    (type-parameter create T (constraint capture-safe transferable))
    (returns Unit) (body (unit)))
  (function create callback-type-root (visibility public) (effect pure)
    (parameter create callback (type (function () Unit)))
    (returns Unit) (body (unit)))))
declarations.end"#;
    let source = byte_buffer_tests::author_only(input).unwrap();
    let program = prepare_snapshot(&source);
    let declaration = declaration_named(&source, "phantom");
    let unit = program
        .types
        .iter()
        .find(|(_, ty)| matches!(ty.form, TypeForm::Unit))
        .unwrap()
        .0;
    let callback = program
        .types
        .iter()
        .find(|(_, ty)| matches!(ty.form, TypeForm::Function { .. }))
        .unwrap()
        .0;
    assert!(program.capture_safe_types.contains(callback));
    for reference in [false, true] {
        for (ty, allowed) in [(*unit, true), (*callback, false)] {
            let control = ExecutionControl::uncancelled();
            let actual = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke_instantiated(declaration, &[ty], vec![], &control)
                .map(|(value, _)| value)
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke_entry(
                        NormalizedEntryPoint::InstantiatedFunction(
                            program.function(declaration).unwrap(),
                            Arc::from([ty]),
                        ),
                        vec![],
                        None,
                        &control,
                    )
                    .map(|(value, _)| value)
            };
            if allowed {
                assert_eq!(actual.unwrap(), NormalizedValue::Unit);
            } else {
                assert!(
                    actual.is_err(),
                    "capture safety alone cannot close a transfer obligation"
                );
            }
        }
    }
}
