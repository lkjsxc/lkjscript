use crate::canonical::compile;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig, JitStats};
use lkjscript_native::RuntimeCallSlot;
use lkjscript_vm::run_chunk;

mod bytes;
mod support;
mod word;
use support::*;

#[test]
fn exact_byte_vector_mutation_move_drop_and_return_execute_in_both_forced_tiers() {
    let mutation = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\ndo/\n",
        "let/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\n",
        "byte-slice-mut-set-byte/\nm\n0\n65\n/byte-slice-mut-set-byte\n/let\n",
        "byte-slice-byte-at/\nborrow/\nb\n/borrow\n0\n/byte-slice-byte-at\n/do\n/let\n/main\n",
    );
    assert_i64_all_engines(mutation, "native-unique-mutation.lkjscript", 65, 1, false);

    let move_call = concat!(
        "def/\nname/\ntake\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\nb\nbyte-vector\n/params\n",
        "byte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n3\n/new-byte-vector\n/bind\n",
        "take/\nmove/\nb\n/move\n/take\n/let\n/main\n",
    );
    assert_i64_all_engines(move_call, "native-unique-move.lkjscript", 3, 1, true);

    let returned = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
        "let/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\ndo/\n",
        "let/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\n",
        "byte-slice-mut-set-byte/\nm\n1\n91\n/byte-slice-mut-set-byte\n/let\n",
        "move/\nb\n/move\n/do\n/let\n/main\n",
    );
    let program = compile(returned, "native-unique-return.lkjscript");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::ReturnedByteVector(vec![0, 91]))
    );
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert!(
        matches!(vm, ExecutionOutcome::Returned(value) if value.as_byte_vector() == Some(&[0, 91][..]))
    );
    for (proof, execution) in forced_pair(&program, &ExecutionConfig::default()) {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(value)
                if value.as_byte_vector() == Some(&[0, 91][..])
        ));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.transfers, 1);
        assert_eq!(execution.stats.native_unique.drops, 0);
    }
}

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
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Trapped(_)
    ));
    for (proof, execution) in forced_pair(&program, &ExecutionConfig::default()) {
        assert!(matches!(execution.outcome, ExecutionOutcome::Trapped(_)));
        assert_unique_metrics(&execution.stats, proof);
        assert_eq!(execution.stats.native_unique.cleanup_releases, 1);
    }

    let allocation = compile(
        concat!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\n",
            "new-byte-vector/\n1\n/new-byte-vector\n/main\n",
        ),
        "native-unique-allocation-limit.lkjscript",
    );
    let limits = ExecutionConfig {
        max_allocations: 0,
        ..ExecutionConfig::default()
    };
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
            &ExecutionConfig::default(),
            JitConfig::default(),
        ),
        execute_optimizing(
            legacy.ssa(),
            &ExecutionConfig::default(),
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
