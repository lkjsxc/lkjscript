//! Exact admitted programs and invocation resources remain private across custody moves.
use super::*;
use crate::platform::runtime::structured::StructuredExecutor;

fn pair_value(right: i64) -> NormalizedValue {
    NormalizedValue::Record(super::super::super::value::NormalizedRecord::Structural {
        fields: Arc::new(vec![
            (Name::new("left").unwrap(), NormalizedValue::I64(0)),
            (Name::new("right").unwrap(), NormalizedValue::I64(right)),
        ]),
    })
}

#[test]
fn parallel_handoff_keeps_exact_programs_quotas_cancellation_and_reclamation_private() {
    let sources = [
        source(),
        byte_buffer_tests::author(&INPUT.replace("(i64 -137)", "(i64 211)")).unwrap(),
    ];
    let programs = [
        Arc::new(prepare_snapshot(&sources[0])),
        Arc::new(prepare_snapshot(&sources[1])),
    ];
    assert_ne!(programs[0].value_origin, programs[1].value_origin);
    let weak = programs.each_ref().map(Arc::downgrade);
    let first = StructuredExecutor::for_test(1);
    let second = first.sharing_capacity_for_test();
    let mut executors = [first, second];
    let mut worker = None;
    for index in 0..4 {
        let side = index % 2;
        let source = &sources[side];
        let program = &programs[side];
        let expected = pair_value([-137, 211][side]);
        let entry = declaration_named(source, "joined");
        let handle = executors[side].handle();
        let vm = NormalizedVm::new(program, NormalizedRunPolicy::foreground(), &handle);
        let cells = super::super::super::owned_i64_cell::StorageObservation::start();
        let buffers = super::super::super::byte_buffer::StorageObservation::start();
        let before = executors[side].observe();
        let cancelled = ExecutionControl::uncancelled();
        cancelled.cancel();
        assert_eq!(
            vm.invoke(entry, vec![], None, &cancelled).unwrap_err().code,
            "execution_cancelled"
        );
        assert_eq!(executors[side].observe(), before);
        let probe = super::super::super::vm::ChildProbe::start(program.value_origin);
        let fresh = ExecutionControl::uncancelled();
        let (value, work) = vm.invoke(entry, vec![], None, &fresh).unwrap();
        assert_eq!(value, expected);
        assert!(!fresh.is_cancelled());
        assert_eq!(
            (
                work.parallel_worker_dispatches,
                work.parallel_inline_fallbacks
            ),
            (1, 0)
        );
        assert_eq!(
            (work.live_call_frames_after, work.live_handles_after),
            (0, 0)
        );
        let actual_thread = probe
            .observed()
            .into_iter()
            .find(|(thread, _)| *thread != std::thread::current().id())
            .unwrap()
            .0;
        assert_eq!(*worker.get_or_insert(actual_thread), actual_thread);
        drop(probe);
        let limited = NormalizedVm::new(
            program,
            NormalizedRunPolicy {
                instruction_steps: Some(work.instructions - 1),
                ..NormalizedRunPolicy::foreground()
            },
            &handle,
        );
        assert_eq!(
            limited
                .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
                .unwrap_err()
                .class,
            crate::platform::execution::ExecutionFailureClass::Resource
        );
        drop(limited);
        assert_eq!(cells.live(), (0, 0));
        assert_eq!(buffers.live(), (0, 0));
        assert_eq!(executors[side].observe().active_dispatches, 0);
        let reference =
            NormalizedReferenceInterpreter::new(source, program, NormalizedRunPolicy::foreground())
                .invoke(entry, vec![], None, &ExecutionControl::uncancelled())
                .unwrap()
                .0;
        assert_eq!(reference, expected);
        drop(vm);
        assert_eq!(Arc::strong_count(program), 1);
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
        3
    );
    assert_eq!(
        executors
            .iter()
            .map(|e| e.observe().workers_handed_off)
            .sum::<u64>(),
        3
    );
    drop(programs);
    assert!(weak.iter().all(|program| program.upgrade().is_none()));
    let mut joins = 0;
    for executor in &mut executors {
        let stopped = executor.shutdown().unwrap();
        assert_eq!(
            (stopped.active_dispatches, stopped.remaining_workers),
            (0, 0)
        );
        assert_eq!(
            stopped.workers_started + stopped.workers_received,
            stopped.joined_workers + stopped.workers_handed_off
        );
        joins += stopped.joined_workers;
    }
    assert_eq!(joins, 1);
}
