use super::*;

pub(super) fn assert_i64_all_engines(
    source: &str,
    name: &str,
    expected: i64,
    drops: u64,
    direct: bool,
) {
    let program = compile(source, name);
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(expected))
    );
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(matches!(vm, ExecutionOutcome::Returned(value) if value.as_i64() == Some(expected)));
    for (proof, execution) in forced_pair(&program, &ExecutionPolicy::unrestricted()) {
        assert!(
            matches!(execution.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(expected))
        );
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.drops, drops);
        if direct {
            assert!(execution.stats.direct_native_calls > 0);
        }
    }
}

pub(super) fn forced_pair(
    program: &lkjscript_compiler::ExecutableProgram,
    limits: &ExecutionPolicy,
) -> [(bool, lkjscript_jit::JitExecution); 2] {
    [
        (
            false,
            execute_forced(program.ssa(), limits, JitConfig::default())
                .expect("forced baseline unique execution"),
        ),
        (
            true,
            execute_optimizing(program.ssa(), limits, JitConfig::default())
                .expect("forced proof unique execution"),
        ),
    ]
}

pub(super) fn assert_returned_bytes_all_engines(source: &str, expected: &[u8], allocations: u64) {
    let program = compile(source, "native-bytes-return.lkjscript");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::ReturnedBytes(expected.to_vec()))
    );
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted()
        ),
        ExecutionOutcome::Returned(value) if value.as_bytes() == Some(expected)
    ));
    for (proof, execution) in forced_pair(&program, &ExecutionPolicy::unrestricted()) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value) if value.as_bytes() == Some(expected)
        ));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.allocations, allocations);
        assert_eq!(execution.stats.native_unique.transfers, 1);
    }
}

pub(super) fn assert_returned_vector_all_engines(source: &str, expected: &[u8], allocations: u64) {
    let program = compile(source, "native-bytes-vector-return.lkjscript");
    assert!(matches!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::ReturnedByteVector(value)) if value == expected
    ));
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted()
        ),
        ExecutionOutcome::Returned(value) if value.as_byte_vector() == Some(expected)
    ));
    for (proof, execution) in forced_pair(&program, &ExecutionPolicy::unrestricted()) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value) if value.as_byte_vector() == Some(expected)
        ));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.allocations, allocations);
    }
}

pub(super) fn assert_unique_metrics(stats: &JitStats, proof: bool) {
    assert!(stats.native_entries > 0 && stats.unique_runtime_calls > 0);
    assert_eq!(stats.runtime_heap_attempts, 0);
    assert_eq!(
        (
            stats.native_unique.live_owners,
            stats.native_unique.live_loans,
            stats.native_unique.release_backlog,
            stats.native_unique.teardown_failures
        ),
        (0, 0, 0, 0)
    );
    assert!(stats.code_objects.iter().all(|object| {
        !object
            .runtime_calls
            .contains(&RuntimeCallSlot::HeapDispatch)
            && object.wx_transition_verified
    }));
    if proof {
        assert!(stats.native_entries > 0);
    }
}
