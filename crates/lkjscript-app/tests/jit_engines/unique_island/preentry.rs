use super::*;

#[test]
fn native_poll_failure_cleans_not_yet_transferred_arguments() {
    let program = compile(
        concat!(
            "def/\nname/\nconsume\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nb\nbyte-vector\n/params\nunit\n/fn\n/def\n",
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
            "let/\nbind/\nkept\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "let/\nbind/\na\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\n",
            "let/\nbind/\nb\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\n",
            "consume/\nmove/\na\n/move\n/consume\nconsume/\nmove/\nb\n/move\n/consume\n",
            "byte-slice-length/\nborrow/\nkept\n/borrow\n/byte-slice-length\n/do\n",
            "/let\n/let\n/let\n/main\n",
        ),
        "native-poll-unentered-cleanup.lkjscript",
    );
    let mut evaluator_completed = false;
    let mut vm_completed = false;
    for fuel in 0..256 {
        let evaluated = evaluate(
            program.ssa(),
            &EvalConfig {
                fuel,
                ..EvalConfig::default()
            },
        );
        if matches!(evaluated, EvalOutcome::Returned(_)) {
            evaluator_completed = true;
        }
        assert!(evaluated.cleanup_failures().is_none(), "{evaluated:?}");
        let executed = run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
                instruction_fuel: fuel,
                ..lkjscript_core::LimitedExecutionPolicy::conservative()
            }),
        );
        if matches!(executed, ExecutionOutcome::Returned(_)) {
            vm_completed = true;
        }
        assert!(
            executed.cleanup_failures().is_none(),
            "fuel={fuel} {executed:?}"
        );
        if evaluator_completed && vm_completed {
            break;
        }
    }
    assert!(evaluator_completed && vm_completed);

    let limits = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        instruction_fuel: 1,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    for (proof, execution) in forced_pair(&program, &limits) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel)
        ));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_attempts, 0);
    }
}
