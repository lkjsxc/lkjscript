use crate::oracle::{compare_source, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome};
use lkjscript_vm::run_chunk;

#[test]
fn byte_vector_borrows_moves_and_mutation_match_evaluator_and_vm() {
    let source = "def/\nname/\npass-owned\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\noutput/\nbyte-vector\n/output\n/sig\nparams/\nb\nbyte-vector\n/params\nmove/\nb\n/move\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\ndo/\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nbyte-slice-mut-set-byte/\nm\n1\n77\n/byte-slice-mut-set-byte\n/let\nlet/\nbind/\nc\npass-owned/\nmove/\nb\n/move\n/pass-owned\n/bind\nlet/\nbind/\nr\nborrow/\nc\n/borrow\n/bind\nbyte-slice-byte-at/\nr\n1\n/byte-slice-byte-at\n/let\n/let\n/do\n/let\n/main\n";
    assert_eq!(
        compare_source(source, "byte-vector-owner.lkjscript"),
        ScalarOutcome::I64(77)
    );

    let var_source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nb\n/name\ntype/\nbyte-vector\n/type\nnew-byte-vector/\n2\n/new-byte-vector\ndo/\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nbyte-slice-mut-set-byte/\nm\n0\n91\n/byte-slice-mut-set-byte\n/let\nmove/\nb\n/move\nset/\nb\nnew-byte-vector/\n3\n/new-byte-vector\n/set\nlet/\nbind/\nfresh\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nmove/\nb\n/move\nset/\nb\nmove/\nfresh\n/move\n/set\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/do\n/let\n/do\n/var\n/main\n";
    assert_eq!(
        compare_source(var_source, "owned-var-reinit.lkjscript"),
        ScalarOutcome::I64(1)
    );

    let marked = source.to_string();
    let program = compile_source(&marked, "byte-vector-limits.lkjscript")
        .expect("compile byte-vector limits fixture");
    let eval_limits = EvalConfig {
        max_allocations: 0,
        ..EvalConfig::default()
    };
    assert!(matches!(
        evaluate(program.ssa(), &eval_limits),
        EvalOutcome::ResourceLimitExceeded(_)
    ));
    let vm_limits = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_allocations: 0,
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &vm_limits
        ),
        ExecutionOutcome::ResourceLimitExceeded(_)
    ));
}
