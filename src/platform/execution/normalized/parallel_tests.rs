//! Language child execution, exact outcomes, aggregate budgets and joined cleanup.
use super::*;

#[path = "parallel_owned_result_tests.rs"]
mod owned_results;

const INPUT: &str = r#"declarations.begin
(units (module create parallel-proof
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create cell-read (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (function create left-worker (visibility public) (effect (task))
    (parameter create value (type ByteBuffer) (use consume))
    (returns I64) (body (call memory::length (local value))))
  (function create right-worker (visibility public) (effect (task))
    (parameter create value (type OwnedI64Cell) (use consume))
    (returns I64) (body (call cell-read (local value))))
  (function create joined (visibility public) (effect (task))
    (returns (record (left I64) (right I64)))
    (body (let
      (binding bytes (type ByteBuffer) (call memory::empty))
      (binding cell (type OwnedI64Cell) (call new-cell (i64 -137)))
      (in (parallel (call left-worker (local bytes)) (call right-worker (local cell)))))))))
declarations.end
"#;

fn source() -> crate::platform::kernel::KernelSnapshot {
    byte_buffer_tests::author(INPUT).unwrap()
}

const NESTED: &str = r#"declarations.begin
(units (module create nested-proof
  (external create new-cell (visibility private) (implementation core.cell.create)
    (parameter create n (type I64)) (returns OwnedI64Cell))
  (external create read-cell (visibility private) (implementation core.cell.read)
    (parameter create value (type OwnedI64Cell) (use borrow)) (returns I64))
  (external create subtract (visibility private) (implementation core.i64.subtract)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns I64))
  (external create less (visibility private) (implementation core.i64.less)
    (parameter create a (type I64)) (parameter create b (type I64)) (returns Bool))
  (function create ordinary-worker (visibility private) (effect (task))
    (parameter create n (type I64)) (returns I64) (body (local n)))
  (function create recurse (visibility private) (effect (task))
    (parameter create n (type I64))
    (parameter create value (type OwnedI64Cell) (use consume)) (returns I64)
    (body (if (call less (local n) (i64 1)) (call read-cell (local value))
      (let (binding pair (type (record (left I64) (right I64)))
        (parallel (call recurse (call subtract (local n) (i64 1)) (local value))
          (call ordinary-worker (local n))))
        (in (field (local pair) (name left)))))))
  (function create nested-main (visibility public) (effect (task))
    (parameter create depth (type I64)) (returns I64)
    (body (let (binding value (type OwnedI64Cell) (call new-cell (i64 -137)))
      (in (call recurse (local depth) (local value))))))))
declarations.end
"#;

#[test]
fn parallel_nested_language_scopes_share_quotas_and_preserve_depth_limits() {
    let source = byte_buffer_tests::author(NESTED).unwrap();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "nested-main");
    let (value, work) = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
        .invoke(
            entry,
            vec![NormalizedValue::I64(3)],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    assert_eq!(value, NormalizedValue::I64(-137));
    assert_eq!(work.parallel_scopes, 3);
    assert!(work.maximum_call_depth >= 4);
    let quota = NormalizedRunPolicy {
        instruction_steps: Some(work.instructions - 1),
        ..NormalizedRunPolicy::foreground()
    };
    assert_eq!(
        NormalizedVm::new(&program, quota)
            .invoke(
                entry,
                vec![NormalizedValue::I64(3)],
                None,
                &ExecutionControl::uncancelled()
            )
            .unwrap_err()
            .class,
        crate::platform::execution::ExecutionFailureClass::Resource
    );
    for reference in [false, true] {
        for (depth, policy) in [
            (
                3,
                NormalizedRunPolicy {
                    maximum_call_depth: 2,
                    ..NormalizedRunPolicy::foreground()
                },
            ),
            (
                super::super::parallel::MAXIMUM_STRUCTURED_DEPTH as i64 + 1,
                NormalizedRunPolicy::foreground(),
            ),
        ] {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let result = if reference {
                NormalizedReferenceInterpreter::new(&source, &program, policy)
                    .invoke(
                        entry,
                        vec![NormalizedValue::I64(depth)],
                        None,
                        &ExecutionControl::uncancelled(),
                    )
                    .map(|v| v.0)
            } else {
                NormalizedVm::new(&program, policy)
                    .invoke(
                        entry,
                        vec![NormalizedValue::I64(depth)],
                        None,
                        &ExecutionControl::uncancelled(),
                    )
                    .map(|v| v.0)
            };
            let error = result.unwrap_err();
            assert_eq!(
                error.class,
                crate::platform::execution::ExecutionFailureClass::Resource
            );
            assert!(
                matches!(
                    error.code.as_str(),
                    "normalized_call_depth"
                        | "normalized_reference_call_depth"
                        | "normalized_parallel_depth"
                ),
                "{error:?}"
            );
            assert_eq!(cells.live(), (0, 0));
        }
    }
}

fn expected(value: NormalizedValue) {
    let NormalizedValue::Record(super::super::value::NormalizedRecord::Structural { fields }) =
        value
    else {
        panic!("expected pair");
    };
    assert_eq!(
        fields
            .iter()
            .map(|(name, value)| (name.as_str(), value.clone()))
            .collect::<Vec<_>>(),
        vec![
            ("left", NormalizedValue::I64(0)),
            ("right", NormalizedValue::I64(-137))
        ]
    );
}

#[test]
fn parallel_graph_children_overlap_with_distinct_origins_and_join_owned_cleanup() {
    let _lane = super::super::parallel::isolated_test_lane();
    let source = source();
    let program = prepare_snapshot(&source);
    let buffers = super::super::byte_buffer::StorageObservation::start();
    let cells = super::super::owned_i64_cell::StorageObservation::start();
    let probe = super::super::vm::ChildProbe::start(program.value_origin);
    let (value, observed) = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
        .invoke(
            declaration_named(&source, "joined"),
            vec![],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    expected(value);
    assert_eq!(observed.parallel_scopes, 1);
    assert_eq!(observed.parallel_workers_spawned, 1);
    let children = probe.observed();
    assert_eq!(children.len(), 2);
    assert_ne!(children[0].0, children[1].0);
    assert_ne!(children[0].1, children[1].1);
    assert_eq!(buffers.created(), 1);
    assert_eq!(cells.created(), 1);
    assert_eq!(buffers.live(), (0, 0));
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(observed.live_call_frames_after, 0);
    assert_eq!(observed.live_handles_after, 0);
    drop(probe);
    expected(
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(
                declaration_named(&source, "joined"),
                vec![],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap()
            .0,
    );
}

#[test]
fn parallel_aggregate_instruction_allocation_and_collection_quotas_include_both_children() {
    let source = source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "joined");
    let (_, total) = NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
        .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
        .unwrap();
    for policy in [
        NormalizedRunPolicy {
            instruction_steps: Some(total.instructions - 1),
            ..NormalizedRunPolicy::foreground()
        },
        NormalizedRunPolicy {
            maximum_allocated_bytes: Some(total.allocated_bytes - 1),
            ..NormalizedRunPolicy::foreground()
        },
        NormalizedRunPolicy {
            maximum_collection_items: Some(total.collection_items - 1),
            ..NormalizedRunPolicy::foreground()
        },
    ] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let error = NormalizedVm::new(&program, policy)
            .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
            .unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::execution::ExecutionFailureClass::Resource
        );
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
    }
    let exact = NormalizedRunPolicy {
        instruction_steps: Some(total.instructions),
        maximum_allocated_bytes: Some(total.allocated_bytes),
        maximum_collection_items: Some(total.collection_items),
        ..NormalizedRunPolicy::foreground()
    };
    let (value, metered) = NormalizedVm::new(&program, exact)
        .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
        .unwrap();
    expected(value);
    assert_eq!(
        (
            metered.instructions,
            metered.allocated_bytes,
            metered.collection_items
        ),
        (
            total.instructions,
            total.allocated_bytes,
            total.collection_items
        ),
    );
    let (_, total) =
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
            .unwrap();
    for policy in [
        NormalizedRunPolicy {
            instruction_steps: Some(total.expressions - 1),
            ..NormalizedRunPolicy::foreground()
        },
        NormalizedRunPolicy {
            maximum_allocated_bytes: Some(total.allocated_bytes - 1),
            ..NormalizedRunPolicy::foreground()
        },
        NormalizedRunPolicy {
            maximum_collection_items: Some(total.collection_items - 1),
            ..NormalizedRunPolicy::foreground()
        },
    ] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let error = NormalizedReferenceInterpreter::new(&source, &program, policy)
            .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
            .unwrap_err();
        assert_eq!(
            error.class,
            crate::platform::execution::ExecutionFailureClass::Resource
        );
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
    }
    let (value, metered) = NormalizedReferenceInterpreter::new(
        &source,
        &program,
        NormalizedRunPolicy {
            instruction_steps: Some(total.expressions),
            maximum_allocated_bytes: Some(total.allocated_bytes),
            maximum_collection_items: Some(total.collection_items),
            ..NormalizedRunPolicy::foreground()
        },
    )
    .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
    .unwrap();
    expected(value);
    assert_eq!(
        (
            metered.expressions,
            metered.allocated_bytes,
            metered.collection_items
        ),
        (
            total.expressions,
            total.allocated_bytes,
            total.collection_items
        ),
    );
}

#[test]
fn parallel_failure_keeps_original_trap_and_cancellation_joins_all_owners() {
    let _lane = super::super::parallel::isolated_test_lane();
    let trapping_input = INPUT.replace(
        "(body (call memory::length (local value)))",
        "(body (call memory::get (i64 0) (local value)))",
    );
    // Cover both a caller-child trap and a spawned-worker trap with the same owners.
    for input in [
        trapping_input.clone(),
        trapping_input.replace(
            "(parallel (call left-worker (local bytes)) (call right-worker (local cell)))",
            "(parallel (call right-worker (local cell)) (call left-worker (local bytes)))",
        ),
    ] {
        let source = byte_buffer_tests::author(&input).unwrap();
        let program = prepare_snapshot(&source);
        let entry = declaration_named(&source, "joined");
        for reference in [false, true] {
            let cells = super::super::owned_i64_cell::StorageObservation::start();
            let buffers = super::super::byte_buffer::StorageObservation::start();
            let error = if reference {
                NormalizedReferenceInterpreter::new(
                    &source,
                    &program,
                    NormalizedRunPolicy::foreground(),
                )
                .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
                .unwrap_err()
            } else {
                NormalizedVm::new(&program, NormalizedRunPolicy::foreground())
                    .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
                    .unwrap_err()
            };
            assert_eq!(
                error.class,
                crate::platform::execution::ExecutionFailureClass::Trap
            );
            assert_eq!(error.code, "normalized_buffer_index");
            assert_eq!(cells.live(), (0, 0));
            assert_eq!(buffers.live(), (0, 0));
        }
    }
    let source = self::source();
    let program = prepare_snapshot(&source);
    for checks in [0, 16, 64, 128, 256, 512, 1024, 4096] {
        let cells = super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::byte_buffer::StorageObservation::start();
        let result = NormalizedVm::new(&program, NormalizedRunPolicy::foreground()).invoke(
            declaration_named(&source, "joined"),
            vec![],
            None,
            &ExecutionControl::cancel_after_checks(checks),
        );
        match result {
            Ok((value, _)) => expected(value),
            Err(error) => assert_eq!(
                error.class,
                crate::platform::execution::ExecutionFailureClass::Cancelled
            ),
        }
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
    }
}
