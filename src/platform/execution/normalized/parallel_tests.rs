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
    let (value, work) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
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
        NormalizedVm::for_test(&program, quota)
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
                NormalizedVm::for_test(&program, policy)
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
    let source = source();
    let program = prepare_snapshot(&source);
    let buffers = super::super::byte_buffer::StorageObservation::start();
    let cells = super::super::owned_i64_cell::StorageObservation::start();
    let probe = super::super::vm::ChildProbe::start(program.value_origin);
    let (value, observed) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
        .invoke(
            declaration_named(&source, "joined"),
            vec![],
            None,
            &ExecutionControl::uncancelled(),
        )
        .unwrap();
    expected(value);
    assert_eq!(observed.parallel_scopes, 1);
    assert_eq!(observed.parallel_worker_dispatches, 1);
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
fn parallel_reuses_owned_executor_without_retaining_program_or_cancelled_control() {
    use crate::platform::runtime::structured::StructuredExecutor;
    let source = source();
    let program = Arc::new(prepare_snapshot(&source));
    let second_source =
        byte_buffer_tests::author(&INPUT.replace("(i64 -137)", "(i64 211)")).unwrap();
    let second_program = Arc::new(prepare_snapshot(&second_source));
    let mut executor = StructuredExecutor::for_test(1);
    let handle = executor.handle();
    let mut worker_thread = None;
    for (source, program, expected_right) in [
        (&source, &program, -137),
        (&second_source, &second_program, 211),
        (&source, &program, -137),
    ] {
        let vm = NormalizedVm::new(program, NormalizedRunPolicy::foreground(), &handle);
        assert_eq!(Arc::strong_count(program), 2);
        let probe = super::super::vm::ChildProbe::start(program.value_origin);
        let (value, work) = vm
            .invoke(
                declaration_named(source, "joined"),
                vec![],
                None,
                &ExecutionControl::uncancelled(),
            )
            .unwrap();
        assert_eq!(
            value,
            NormalizedValue::Record(super::super::value::NormalizedRecord::Structural {
                fields: Arc::new(vec![
                    (Name::new("left").unwrap(), NormalizedValue::I64(0)),
                    (
                        Name::new("right").unwrap(),
                        NormalizedValue::I64(expected_right)
                    ),
                ]),
            },)
        );
        assert_eq!(
            (
                work.parallel_worker_dispatches,
                work.parallel_inline_fallbacks
            ),
            (1, 0)
        );
        let observed_worker = probe
            .observed()
            .into_iter()
            .find(|(thread, _)| *thread != std::thread::current().id())
            .unwrap()
            .0;
        if let Some(worker) = worker_thread {
            assert_eq!(worker, observed_worker);
        }
        worker_thread = Some(observed_worker);
        drop(probe);
        assert_eq!(
            Arc::strong_count(program),
            2,
            "completion retained a program"
        );
        let cancelled = ExecutionControl::uncancelled();
        cancelled.cancel();
        assert_eq!(
            vm.invoke(
                declaration_named(source, "joined"),
                vec![],
                None,
                &cancelled
            )
            .unwrap_err()
            .code,
            "execution_cancelled"
        );
        drop(vm);
        assert_eq!(Arc::strong_count(program), 1);
    }
    assert_eq!(executor.observe().workers_started, 1);
    assert_eq!(executor.observe().completed_dispatches, 3);
    let stopped = executor.shutdown().unwrap();
    assert_eq!((stopped.joined_workers, stopped.remaining_workers), (1, 0));
}

#[test]
fn parallel_nested_inline_scopes_share_invocation_budget_and_reclaim_owners() {
    use crate::platform::runtime::structured::StructuredExecutor;
    let source = byte_buffer_tests::author(NESTED).unwrap();
    let program = Arc::new(prepare_snapshot(&source));
    let mut executor = StructuredExecutor::for_test(0);
    let handle = executor.handle();
    let entry = declaration_named(&source, "nested-main");
    let run = |policy| {
        NormalizedVm::new(&program, policy, &handle).invoke(
            entry,
            vec![NormalizedValue::I64(3)],
            None,
            &ExecutionControl::uncancelled(),
        )
    };
    let (value, work) = run(NormalizedRunPolicy::foreground()).unwrap();
    assert_eq!(value, NormalizedValue::I64(-137));
    assert_eq!(
        (
            work.parallel_scopes,
            work.parallel_worker_dispatches,
            work.parallel_inline_fallbacks
        ),
        (3, 0, 3)
    );
    let cells = super::super::owned_i64_cell::StorageObservation::start();
    let error = run(NormalizedRunPolicy {
        instruction_steps: Some(work.instructions - 1),
        ..NormalizedRunPolicy::foreground()
    })
    .unwrap_err();
    assert_eq!(
        error.class,
        crate::platform::execution::ExecutionFailureClass::Resource
    );
    assert_eq!(cells.live(), (0, 0));
    assert_eq!(run(NormalizedRunPolicy::foreground()).unwrap().0, value);
    assert_eq!(executor.shutdown().unwrap().remaining_workers, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn stopped_resident_releases_program_while_reusable_workers_remain_owned() {
    use crate::platform::runtime::structured::StructuredExecutor;
    let input = format!(
        "{INPUT}\n{}",
        r#"declarations.begin
(units
  (module create resident-proof
    (component create command (visibility private)
      (port create main
        (type (task-function () (record (left I64) (right I64)) (row)))
        (function parallel-proof::joined))))
  (target create resident-parallel (component resident-proof::command)
    (runner command) (port resident-proof::command::main)))
declarations.end
"#
    );
    let source = byte_buffer_tests::author(&input).unwrap();
    let program = Arc::new(prepare_snapshot(&source));
    let weak = Arc::downgrade(&program);
    let mut executor = StructuredExecutor::for_test(1);
    let deployment = NormalizedPreparedDeployment::prepare(
        &program,
        Name::new("resident-parallel").unwrap(),
        Vec::new(),
        NormalizedDeploymentResourcePolicy::default(),
        &SecretCatalog::from_environment(&[]).unwrap(),
    )
    .unwrap();
    let resident = NormalizedResidentDeployment::prepare(
        Arc::clone(&program),
        deployment,
        ResidentLimits::default(),
        NormalizedRunPolicy::foreground(),
        &executor.handle(),
    )
    .unwrap();
    let receipt = resident.invoke(Vec::new()).await.unwrap();
    expected(receipt.value);
    assert_eq!(receipt.execution.parallel_worker_dispatches, 1);
    assert_eq!(executor.observe().remaining_workers, 1);
    let shutdown = resident.shutdown().await;
    assert_eq!(shutdown.remaining_tasks, 0);
    assert!(shutdown.cleanup_failures.is_empty());
    drop(resident);
    drop(program);
    assert!(
        weak.upgrade().is_none(),
        "idle workers retained the stopped program"
    );
    let stopped = executor.shutdown().unwrap();
    assert_eq!((stopped.joined_workers, stopped.remaining_workers), (1, 0));
}

#[test]
fn parallel_nested_list_map_observations_count_each_child_once_on_reused_or_inline_workers() {
    use crate::platform::runtime::structured::StructuredExecutor;
    let result = "(record (values (list I64)) (entries (map I64 I64)))";
    let pair = format!("(record (left {result}) (right {result}))");
    let input = format!(
        r#"declarations.begin
(units (module create observed-children
  (function create worker (visibility private) (effect (task)) (returns {result})
    (body (record structural
      (field values (list I64 (i64 3) (i64 5) (i64 7)))
      (field entries (map I64 I64 (entry (i64 11) (i64 13)) (entry (i64 17) (i64 19)))))))
  (function create two (visibility private) (effect (task)) (returns {pair})
    (body (parallel (call worker) (call worker))))
  (function create four (visibility public) (effect (task))
    (returns (record (left {pair}) (right {pair})))
    (body (parallel (call two) (call two))))))
declarations.end
"#
    );
    let source = byte_buffer_tests::author_only(&input).unwrap();
    let program = Arc::new(prepare_snapshot(&source));
    let entry = declaration_named(&source, "four");
    let expected =
        NormalizedReferenceInterpreter::new(&source, &program, NormalizedRunPolicy::foreground())
            .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
            .unwrap()
            .0;
    for workers in [0, 1] {
        let mut executor = StructuredExecutor::for_test(workers);
        let handle = executor.handle();
        let vm = NormalizedVm::new(&program, NormalizedRunPolicy::foreground(), &handle);
        let mut prior = None;
        for _ in 0..3 {
            let (value, work) = vm
                .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
                .unwrap();
            assert_eq!(value, expected);
            assert_eq!(work.parallel_scopes, 3);
            assert_eq!(
                work.parallel_worker_dispatches + work.parallel_inline_fallbacks,
                3
            );
            assert_eq!(work.value_work.lists.nodes_allocated, 4);
            assert_eq!(work.value_work.lists.element_handle_allocations, 12);
            assert_eq!(work.value_work.maps.nodes_allocated, 12);
            assert_eq!(work.value_work.maps.entry_handles_allocated, 8);
            let observation = (work.value_work.lists, work.value_work.maps);
            if let Some(prior) = prior {
                assert_eq!(
                    observation, prior,
                    "reused job counters leaked or merged twice"
                );
            }
            prior = Some(observation);
        }
        assert_eq!(executor.observe().workers_started, workers as u64);
        executor.shutdown().unwrap();
    }
}

#[test]
fn parallel_aggregate_instruction_allocation_and_collection_quotas_include_both_children() {
    let source = source();
    let program = prepare_snapshot(&source);
    let entry = declaration_named(&source, "joined");
    let (_, total) = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
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
        let error = NormalizedVm::for_test(&program, policy)
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
    let (value, metered) = NormalizedVm::for_test(&program, exact)
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
                NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground())
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
        let result = NormalizedVm::for_test(&program, NormalizedRunPolicy::foreground()).invoke(
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

#[path = "parallel_generic_tests.rs"]
mod generic;
