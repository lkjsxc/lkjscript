//! Returned owners compose through parent unpacking, quotas and joined failures.
use super::super::super::{
    byte_buffer::StorageObservation as Buffers, owned_i64_cell::StorageObservation as Cells,
    owned_product::StorageObservation as Products,
};
use super::*;
use std::sync::Arc;

fn composed_source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-result-library.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-result-workers.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-result-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap()
}

fn record(fields: Vec<(&str, NormalizedValue)>) -> NormalizedValue {
    let mut fields = fields
        .into_iter()
        .map(|(name, value)| (Name::new(name).unwrap(), value))
        .collect::<Vec<_>>();
    fields.sort_by(|(left, _), (right, _)| left.cmp(right));
    NormalizedValue::Record(super::super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(fields),
    })
}

fn expected(n: i64, accepted: bool) -> NormalizedValue {
    record(vec![
        (
            "all",
            record(vec![
                ("bytes", NormalizedValue::bytes(vec![0, 255, 128, 7, 9])),
                ("length", NormalizedValue::I64(4)),
                ("before", NormalizedValue::I64(n)),
                ("after", NormalizedValue::I64(-81)),
            ]),
        ),
        (
            "mixed-left",
            record(vec![
                ("bytes", NormalizedValue::bytes(vec![0, 255, 128, 11])),
                ("scalar", NormalizedValue::I64(n)),
            ]),
        ),
        (
            "mixed-right",
            record(vec![
                ("ordinary", NormalizedValue::I64(n)),
                ("scalar", NormalizedValue::I64(n)),
            ]),
        ),
        (
            "nested",
            record(vec![
                ("bytes", NormalizedValue::bytes(vec![0, 255, 128, 64])),
                ("scalar", NormalizedValue::I64(n)),
                (
                    "outcome",
                    record(vec![
                        ("accepted", NormalizedValue::Bool(accepted)),
                        ("value", NormalizedValue::I64(n)),
                    ]),
                ),
            ]),
        ),
    ])
}

#[test]
fn parallel_owned_results_compose_primitives_mixed_pairs_nested_groups_and_choices() {
    exercise_results(composed_source());
}

#[test]
fn parallel_generic_owned_aggregates_preserve_payloads_and_allocation_identity() {
    let source = byte_buffer_tests::author_only(
        &[
            include_str!("../../../../tests/fixtures/owned-witness-library.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-result-library.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-result-workers.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-generic-workers.lkjc"),
            include_str!("../../../../tests/fixtures/parallel-generic-consumer.lkjc"),
        ]
        .join("\n"),
    )
    .unwrap();
    exercise_results(source);
}

fn exercise_results(source: crate::platform::kernel::KernelSnapshot) {
    assert!(crate::platform::kernel::memory_reference::accepts(&source));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "main");
    for reference in [false, true] {
        for n in [i64::MIN, -137, i64::MAX] {
            for accepted in [false, true] {
                let buffers = Buffers::start();
                let cells = Cells::start();
                let products = Products::start();
                let arguments = vec![NormalizedValue::I64(n), NormalizedValue::Bool(accepted)];
                let control = ExecutionControl::uncancelled();
                let value = if reference {
                    let (value, observation) = NormalizedReferenceInterpreter::new(
                        &source,
                        &program,
                        NormalizedRunPolicy::foreground(),
                    )
                    .invoke(entry, arguments, None, &control)
                    .unwrap();
                    assert_eq!(observation.live_call_frames_after, 0);
                    assert_eq!(observation.live_handles_after, 0);
                    value
                } else {
                    let (value, observation) =
                        NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                            .invoke(entry, arguments, None, &control)
                            .unwrap();
                    assert_eq!(observation.parallel_scopes, 5);
                    assert!(observation.parallel_worker_dispatches > 0);
                    assert_eq!(observation.live_call_frames_after, 0);
                    assert_eq!(observation.live_handles_after, 0);
                    value
                };
                assert_eq!(value, expected(n, accepted));
                products.assert_transfers_preserve_allocations();
                assert_eq!(
                    (buffers.live(), cells.live(), products.live()),
                    ((0, 0), (0, 0), (0, 0))
                );
            }
        }
    }
}

#[test]
fn parallel_owned_result_wrapper_and_nested_children_charge_one_invocation_quota() {
    let source = composed_source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "main");
    for reference in [false, true] {
        let run = |policy: NormalizedRunPolicy| {
            let buffers = Buffers::start();
            let cells = Cells::start();
            let products = Products::start();
            let arguments = vec![NormalizedValue::I64(-137), NormalizedValue::Bool(false)];
            let control = ExecutionControl::uncancelled();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(entry, arguments, None, &control)
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
                    .invoke(entry, arguments, None, &control)
                    .map(|(value, work)| {
                        (
                            value,
                            work.instructions,
                            work.allocated_bytes,
                            work.collection_items,
                        )
                    })
            };
            assert_eq!(
                (buffers.live(), cells.live(), products.live()),
                ((0, 0), (0, 0), (0, 0))
            );
            result
        };
        let (value, steps, bytes, items) = run(NormalizedRunPolicy::foreground()).unwrap();
        assert_eq!(value, expected(-137, false));
        let exact = NormalizedRunPolicy {
            instruction_steps: Some(steps),
            maximum_allocated_bytes: Some(bytes),
            maximum_collection_items: Some(items),
            ..NormalizedRunPolicy::foreground()
        };
        assert_eq!(
            run(exact).unwrap(),
            (expected(-137, false), steps, bytes, items)
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
                run(policy).unwrap_err().class,
                crate::platform::execution::ExecutionFailureClass::Resource
            );
        }
    }
}

#[test]
fn parallel_owned_result_cancellation_joins_and_releases_every_parent_created_owner() {
    let source = composed_source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "main");
    for reference in [false, true] {
        let mut failed = 0;
        let mut completed = 0;
        for checks in [0, 64, 128, 256, 512, 1_024, 4_096, 32_768] {
            let buffers = Buffers::start();
            let cells = Cells::start();
            let products = Products::start();
            let arguments = vec![NormalizedValue::I64(-137), NormalizedValue::Bool(false)];
            let control = ExecutionControl::cancel_after_checks(checks);
            let result = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, arguments, None, &control)
                .map(|r| r.0)
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, arguments, None, &control)
                    .map(|r| r.0)
            };
            match result {
                Ok(value) => {
                    completed += 1;
                    assert_eq!(value, expected(-137, false));
                }
                Err(error) => {
                    failed += 1;
                    assert_eq!(
                        error.class,
                        crate::platform::execution::ExecutionFailureClass::Cancelled
                    );
                }
            }
            assert_eq!(
                (buffers.live(), cells.live(), products.live()),
                ((0, 0), (0, 0), (0, 0))
            );
        }
        assert!(failed > 0 && completed > 0);
    }
}

const FAILURE: &str = r#"declarations.begin
(units (module create result-failure
  (external create cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create divide (visibility private) (implementation core.i64.divide)
    (parameter create left (type I64)) (parameter create right (type I64)) (returns I64))
  (function create give (visibility private) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (body (local value)))
  (function create fail (visibility private) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (body (sequence (call divide (i64 1) (i64 0)) (local value))))
  (function create main (visibility public) (effect (task)) (returns Unit)
    (body (let
      (binding a (type OwnedI64Cell) (call cell (i64 17)))
      (binding b (type OwnedI64Cell) (call cell (i64 29)))
      (binding pair (type (owned-product (field left OwnedI64Cell) (field right OwnedI64Cell)))
        (parallel (call give (local a)) (call fail (local b))))
      (in (unit)))))))
declarations.end"#;

#[test]
fn parallel_owned_result_trap_in_either_lane_preserves_original_failure_and_cleanup() {
    for failing_left in [false, true] {
        let input = if failing_left {
            FAILURE.replace(
                "(call give (local a)) (call fail (local b))",
                "(call fail (local a)) (call give (local b))",
            )
        } else {
            FAILURE.to_owned()
        };
        let source = byte_buffer_tests::author_only(&input).unwrap();
        let program = prepare_snapshot(&source);
        let entry = declaration_named(&source, "main");
        for reference in [false, true] {
            let cells = Cells::start();
            let products = Products::start();
            let control = ExecutionControl::uncancelled();
            let error = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![], None, &control)
                .unwrap_err()
            } else {
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![], None, &control)
                    .unwrap_err()
            };
            assert_eq!(
                error.class,
                crate::platform::execution::ExecutionFailureClass::Trap
            );
            assert_eq!(
                error.code,
                if reference {
                    "reference_integer_division"
                } else {
                    "normalized_integer_division"
                }
            );
            assert_eq!((cells.live(), products.live()), ((0, 0), (0, 0)));
        }
    }
}

#[test]
fn parallel_owned_result_pair_is_derived_without_an_authored_type_annotation() {
    use crate::platform::kernel::{StructuralTypeField, TypeObject, encode_type_object};
    let mut source = byte_buffer_tests::author_only(
        r#"declarations.begin
(units (module create inferred-result
  (external create cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (function create give (visibility private) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns OwnedI64Cell) (body (local value)))
  (function create main (visibility public) (effect (task)) (returns I64)
    (body (let
      (binding a (type OwnedI64Cell) (call cell (i64 17)))
      (binding b (type OwnedI64Cell) (call cell (i64 29)))
      (in (sequence
        (parallel (call give (local a)) (call give (local b)))
        (i64 41))))))))
declarations.end"#,
    )
    .unwrap();
    let cell = source
        .types
        .iter()
        .find_map(|(ty, object)| matches!(object.form, TypeForm::OwnedI64Cell).then_some(*ty))
        .unwrap();
    let pair = TypeObject::new(TypeForm::OwnedProduct {
        fields: vec![
            StructuralTypeField {
                name: Name::new("left").unwrap(),
                ty: cell,
            },
            StructuralTypeField {
                name: Name::new("right").unwrap(),
                ty: cell,
            },
        ],
    })
    .unwrap();
    let pair_type = encode_type_object(&pair).unwrap().0;
    // No authored return or binding refers to this type. Canonical execution
    // must reconstruct its meaning without copying the compiler's inferred table.
    source.types.remove(&pair_type);
    source.dependency_types.remove(&pair_type);
    assert!(!source.types.contains_key(&pair_type));
    let schema =
        super::super::super::reference_schema::NormalizedReferenceSchema::reconstruct([&source])
            .unwrap();
    assert_eq!(schema.types.get(&pair_type), Some(&pair));
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "main");
    for reference in [false, true] {
        let cells = Cells::start();
        let products = Products::start();
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
        assert_eq!(value, NormalizedValue::I64(41));
        assert_eq!(cells.created(), 2);
        assert_eq!((cells.live(), products.live()), ((0, 0), (0, 0)));
    }
    let cancelled = ExecutionControl::cancel_after_checks(0);
    assert_eq!(
        super::super::super::reference_schema::NormalizedReferenceSchema::reconstruct_with_control(
            [&source],
            &cancelled
        )
        .unwrap_err()
        .class,
        crate::platform::execution::ExecutionFailureClass::Cancelled,
    );
}
