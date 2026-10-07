//! A physical worker does not carry a previous owner's thread-local observations.
use super::*;
use crate::platform::runtime::structured::StructuredExecutor;

#[test]
fn parallel_handoff_scopes_list_and_map_observations_to_each_exact_job() {
    let result = "(record (values (list I64)) (entries (map I64 I64)))";
    let input = format!(
        r#"declarations.begin
(units (module create observed-handoff
  (function create worker (visibility private) (effect (task)) (returns {result})
    (body (record structural
      (field values (list I64 (i64 3) (i64 5) (i64 7)))
      (field entries (map I64 I64 (entry (i64 11) (i64 13)) (entry (i64 17) (i64 19)))))))
  (function create two (visibility public) (effect (task))
    (returns (record (left {result}) (right {result})))
    (body (parallel (call worker) (call worker))))))
declarations.end
"#
    );
    let smaller = input
        .replace("(i64 3) (i64 5) (i64 7)", "(i64 23)")
        .replace("(entry (i64 17) (i64 19))", "");
    let sources = [
        byte_buffer_tests::author_only(&input).unwrap(),
        byte_buffer_tests::author_only(&smaller).unwrap(),
    ];
    let programs = sources
        .each_ref()
        .map(|source| Arc::new(prepare_snapshot(source)));
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let mut executors = [first, second];
    for side in [0, 0, 1, 1, 0] {
        let source = &sources[side];
        let program = &programs[side];
        let entry = declaration_named(source, "two");
        let expected =
            NormalizedReferenceInterpreter::new(source, program, NormalizedRunPolicy::foreground())
                .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
                .unwrap()
                .0;
        let handle = executors[side].handle();
        let (value, work) = NormalizedVm::new(program, NormalizedRunPolicy::foreground(), &handle)
            .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
            .unwrap();
        assert_eq!(value, expected);
        assert_eq!(
            (work.parallel_scopes, work.parallel_worker_dispatches),
            (1, 1)
        );
        assert_eq!(work.parallel_inline_fallbacks, 0);
        assert_eq!(work.value_work.lists.nodes_allocated, 2);
        assert_eq!(
            work.value_work.lists.element_handle_allocations,
            [6, 2][side]
        );
        assert_eq!(work.value_work.maps.nodes_allocated, [6, 2][side]);
        assert_eq!(work.value_work.maps.entry_handles_allocated, [4, 2][side]);
    }
    assert_eq!(
        executors
            .iter()
            .map(|e| e.observe().workers_started)
            .sum::<u64>(),
        1
    );
    assert_eq!(
        executors
            .iter()
            .map(|e| e.observe().workers_received)
            .sum::<u64>(),
        2
    );
    let mut joined = 0;
    for executor in &mut executors {
        let receipt = executor.shutdown().unwrap();
        assert_eq!(receipt.remaining_workers, 0);
        joined += receipt.joined_workers;
    }
    assert_eq!(joined, 1);
}
