use super::*;

#[test]
fn unique_runtime_failure_paths_cleanup_and_preflight_remains_closed() {
    let trap = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
        "byte-slice-byte-at/\nborrow/\nb\n/borrow\n2\n/byte-slice-byte-at\n/let\n/main\n",
    );
    let program = compile(trap, "native-unique-trap.lkjscript");
    assert!(matches!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Trapped(_)
    ));
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted()
        ),
        ExecutionOutcome::Trapped(_)
    ));
    for (proof, execution) in forced_pair(&program, &ExecutionPolicy::unrestricted()) {
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
        assert_eq!(execution.stats.native_unique.cleanup_releases, 0);
    }

    let arithmetic_failure = compile(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\n",
            "divide/\n1\n0\n/divide\nbyte-slice-length/\nborrow/\nb\n/borrow\n",
            "/byte-slice-length\n/do\n/let\n/main\n",
        ),
        "native-unique-arithmetic-failure.lkjscript",
    );
    let eval_failure = evaluate(arithmetic_failure.ssa(), &EvalConfig::default());
    assert!(
        matches!(eval_failure, EvalOutcome::Trapped(_))
            && eval_failure.cleanup_failures().is_none()
    );
    let vm_failure = run_chunk(
        arithmetic_failure.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(
        matches!(vm_failure, ExecutionOutcome::Trapped(_))
            && vm_failure.cleanup_failures().is_none()
    );
    for (proof, execution) in forced_pair(&arithmetic_failure, &ExecutionPolicy::unrestricted()) {
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
    }

    let callee_failure = compile(
        concat!(
            "def/\nname/\nfail-owned\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\n",
            "output/\ni64\n/output\n/sig\nparams/\nb\nbyte-vector\n/params\n",
            "byte-slice-byte-at/\nborrow/\nb\n/borrow\n9\n/byte-slice-byte-at\n/fn\n/def\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "let/\nbind/\nkept\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "let/\nbind/\ndoomed\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "do/\nfail-owned/\nmove/\ndoomed\n/move\n/fail-owned\n",
            "byte-slice-length/\nborrow/\nkept\n/borrow\n/byte-slice-length\n/do\n",
            "/let\n/let\n/main\n",
        ),
        "native-unique-callee-failure.lkjscript",
    );
    let eval_failure = evaluate(callee_failure.ssa(), &EvalConfig::default());
    assert!(
        matches!(eval_failure, EvalOutcome::Trapped(_))
            && eval_failure.cleanup_failures().is_none()
    );
    let vm_failure = run_chunk(
        callee_failure.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(
        matches!(vm_failure, ExecutionOutcome::Trapped(_))
            && vm_failure.cleanup_failures().is_none()
    );
    for (proof, execution) in forced_pair(&callee_failure, &ExecutionPolicy::unrestricted()) {
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
        assert_eq!(execution.stats.native_unique.cleanup_releases, 0);
    }
    let preentry_limits = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_frames: 1,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    for (proof, execution) in forced_pair(&callee_failure, &preentry_limits) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth)
        ));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
    }

    let poll_failure = compile(
        concat!(
            "def/\nname/\ntick\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n",
            "/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\nmain/\nsig/\n",
            "inputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\n",
            "new-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\ntick/\n/tick\ntick/\n/tick\n",
            "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/do\n/let\n/main\n",
        ),
        "native-unique-poll-failure.lkjscript",
    );
    let poll_limits = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        instruction_fuel: 1,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    for (proof, execution) in forced_pair(&poll_failure, &poll_limits) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::ResourceLimitExceeded(_)
        ));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
    }

    let allocation = compile(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
            "new-byte-vector/\n1\n/new-byte-vector\n/main\n",
        ),
        "native-unique-allocation-limit.lkjscript",
    );
    let limits = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_allocations: 0,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    for (_, execution) in forced_pair(&allocation, &limits) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));
        assert_unique_metrics(&execution.stats, false);
    }

    let legacy = compile(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n",
            "empty-string/\n/empty-string\nlet/\nbind/\nb\n",
            "new-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n",
            "/let\n/do\n/main\n",
        ),
        "native-unique-legacy-reachable.lkjscript",
    );
    for result in [
        execute_forced(
            legacy.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        ),
        execute_optimizing(
            legacy.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        ),
    ] {
        assert_eq!(
            result
                .expect_err("legacy/unique group rejects preflight")
                .code(),
            FailureCode::UnsupportedType
        );
    }
}
