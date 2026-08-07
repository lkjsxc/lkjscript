use crate::canonical::compile;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig, JitStats};
use lkjscript_native::RuntimeCallSlot;
use lkjscript_vm::run_chunk;

mod bytes;
mod conditional;
mod failure_paths;
mod preentry;
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
        &ExecutionPolicy::unrestricted(),
    );
    assert!(
        matches!(vm, ExecutionOutcome::Returned(value) if value.as_byte_vector() == Some(&[0, 91][..]))
    );
    for (proof, execution) in forced_pair(&program, &ExecutionPolicy::unrestricted()) {
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
