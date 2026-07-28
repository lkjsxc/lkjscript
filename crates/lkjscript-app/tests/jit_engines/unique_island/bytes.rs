use super::*;

const STATIC_READ: &str = concat!(
    "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nadd/\n",
    "bytes-length/\nbytes-literal/\n00ff10\n/bytes-literal\n/bytes-length\n",
    "bytes-byte-at/\nbytes-literal/\n00ff10\n/bytes-literal\n1\n/bytes-byte-at\n/add\n/main\n",
);
const STATIC_COPY: &str = concat!(
    "main/\nsig/\ninputs/\n/inputs\noutput/\nbytes\n/output\n/sig\n",
    "copy-bytes-slice/\nbytes-literal/\n00ff10\n/bytes-literal\n1\n2\n/copy-bytes-slice\n/main\n",
);
const STATIC_THAW: &str = concat!(
    "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
    "thaw-bytes/\nbytes-literal/\n00ff10\n/bytes-literal\n/thaw-bytes\n/main\n",
);
const DYNAMIC_CLONE: &str = concat!(
    "main/\nsig/\ninputs/\n/inputs\noutput/\nbytes\n/output\n/sig\n",
    "let/\nbind/\nv\nnew-byte-vector/\n3\n/new-byte-vector\n/bind\n",
    "let/\nbind/\nb\nfreeze-byte-vector/\nmove/\nv\n/move\n/freeze-byte-vector\n/bind\n",
    "clone-bytes/\nb\n/clone-bytes\n/let\n/let\n/main\n",
);
const DYNAMIC_THAW: &str = concat!(
    "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
    "let/\nbind/\nv\nnew-byte-vector/\n3\n/new-byte-vector\n/bind\n",
    "let/\nbind/\nb\nfreeze-byte-vector/\nmove/\nv\n/move\n/freeze-byte-vector\n/bind\n",
    "thaw-bytes/\nmove/\nb\n/move\n/thaw-bytes\n/let\n/let\n/main\n",
);
const DIRECT: &str = concat!(
    "def/\nname/\ntake\n/name\nfn/\nsig/\ninputs/\nbytes\n/inputs\noutput/\ni64\n/output\n/sig\n",
    "params/\nb\nbytes\n/params\nbytes-length/\nb\n/bytes-length\n/fn\n/def\n",
    "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
    "let/\nbind/\nv\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\n",
    "let/\nbind/\nb\nfreeze-byte-vector/\nmove/\nv\n/move\n/freeze-byte-vector\n/bind\n",
    "take/\nmove/\nb\n/move\n/take\n/let\n/let\n/main\n",
);

#[test]
fn immutable_bytes_literals_copy_and_thaw_match_all_engines() {
    let read = compile(STATIC_READ, "native-bytes-static-read.lkjscript");
    assert_eq!(
        evaluate(read.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(258))
    );
    assert!(matches!(
        run_chunk(
            read.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(258)
    ));
    for (proof, execution) in forced_pair(&read, &ExecutionConfig::default()) {
        assert!(
            matches!(execution.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(258))
        );
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.allocations, 0);
    }

    assert_returned_bytes_all_engines(STATIC_COPY, &[255, 16], 1);
    assert_returned_vector_all_engines(STATIC_THAW, &[0, 255, 16], 1);
}

#[test]
fn dynamic_clone_thaw_return_and_direct_call_preserve_exact_ownership() {
    assert_returned_bytes_all_engines(DYNAMIC_CLONE, &[0, 0, 0], 2);
    assert_returned_vector_all_engines(DYNAMIC_THAW, &[0, 0, 0], 1);

    let direct = compile(DIRECT, "native-bytes-direct.lkjscript");
    assert_eq!(
        evaluate(direct.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(2))
    );
    assert!(matches!(
        run_chunk(
            direct.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(2)
    ));
    for (proof, execution) in forced_pair(&direct, &ExecutionConfig::default()) {
        assert!(
            matches!(execution.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(2))
        );
        assert_unique_metrics(&execution.stats, proof);
        assert!(execution.stats.direct_native_calls > 0);
        assert_eq!(execution.stats.native_unique.drops, 1);
    }
}

#[test]
fn bytes_failure_paths_preserve_owners_and_cleanup_without_collectors() {
    let allocation = compile(STATIC_COPY, "native-bytes-allocation-limit.lkjscript");
    let limits = ExecutionConfig {
        max_allocations: 0,
        ..ExecutionConfig::default()
    };
    assert!(matches!(
        evaluate(
            allocation.ssa(),
            &EvalConfig {
                max_allocations: 0,
                ..EvalConfig::default()
            }
        ),
        EvalOutcome::ResourceLimitExceeded(_)
    ));
    assert!(matches!(
        run_chunk(
            allocation.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &limits
        ),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
    ));
    for (_, execution) in forced_pair(&allocation, &limits) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));
        assert_unique_metrics(&execution.stats, false);
    }

    let trap_source = DYNAMIC_CLONE.replace(
        "clone-bytes/\nb\n/clone-bytes",
        "bytes-byte-at/\nb\n9\n/bytes-byte-at",
    );
    let trap_source = trap_source.replace("output/\nbytes\n/output", "output/\ni64\n/output");
    let trap = compile(&trap_source, "native-bytes-trap.lkjscript");
    assert!(matches!(
        evaluate(trap.ssa(), &EvalConfig::default()),
        EvalOutcome::Trapped(_)
    ));
    assert!(matches!(
        run_chunk(
            trap.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Trapped(_)
    ));
    for (proof, execution) in forced_pair(&trap, &ExecutionConfig::default()) {
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_releases, 1);
    }

    let mixed = compile(
        &format!(
            "{}{}",
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n",
            concat!(
                "empty-string/\n/empty-string\nbytes-length/\nbytes-literal/\n00\n",
                "/bytes-literal\n/bytes-length\n/do\n/main\n",
            )
        ),
        "native-bytes-legacy-mix.lkjscript",
    );
    for result in [
        execute_forced(
            mixed.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        ),
        execute_optimizing(
            mixed.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        ),
    ] {
        assert_eq!(
            result.expect_err("mixed group rejects before entry").code(),
            FailureCode::UnsupportedType
        );
    }
}
