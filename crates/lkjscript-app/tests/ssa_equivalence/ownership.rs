use crate::oracle::{compare_source, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome};
use lkjscript_vm::run_chunk;

#[test]
fn owned_buf_borrows_moves_and_mutation_match_evaluator_and_vm() {
    let source = "def/\nname/\npass-owned\n/name\nfn/\nsig/\nOwned\nBuf\n->\nOwned\nBuf\n/sig\nparams/\nb\nOwned/\nBuf\n/Owned\n/params\nmove/\nb\n/move\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n2\n/owned-buf-new\n/bind\ndo/\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nowned-buf-set/\nm\n1\n77\n/owned-buf-set\n/let\nlet/\nbind/\nc\npass-owned/\nmove/\nb\n/move\n/pass-owned\n/bind\nlet/\nbind/\nr\nborrow/\nc\n/borrow\n/bind\nowned-buf-ref/\nr\n1\n/owned-buf-ref\n/let\n/let\n/do\n/let\n/main\n";
    assert_eq!(
        compare_source(source, "owned-buffer.lkjscript"),
        ScalarOutcome::I64(77)
    );

    let var_source = "main/\nsig/\n->\nI64\n/sig\nvar/\nname/\nb\n/name\ntype/\nOwned\nBuf\n/type\nowned-buf-new/\n2\n/owned-buf-new\ndo/\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nowned-buf-set/\nm\n0\n91\n/owned-buf-set\n/let\nmove/\nb\n/move\nset/\nb\nowned-buf-new/\n3\n/owned-buf-new\n/set\nlet/\nbind/\nfresh\nowned-buf-new/\n1\n/owned-buf-new\n/bind\ndo/\nmove/\nb\n/move\nset/\nb\nmove/\nfresh\n/move\n/set\nowned-buf-len/\nborrow/\nb\n/borrow\n/owned-buf-len\n/do\n/let\n/do\n/var\n/main\n";
    assert_eq!(
        compare_source(var_source, "owned-var-reinit.lkjscript"),
        ScalarOutcome::I64(1)
    );

    let marked = source.to_string();
    let program = compile_source(&marked, "owned-buffer-limits.lkjscript", &Limits::default())
        .expect("compile owned buffer limits fixture");
    let eval_limits = EvalConfig {
        max_allocations: 0,
        ..EvalConfig::default()
    };
    assert!(matches!(
        evaluate(program.ssa(), &eval_limits),
        EvalOutcome::ResourceLimitExceeded(_)
    ));
    let vm_limits = ExecutionConfig {
        max_allocations: 0,
        ..ExecutionConfig::default()
    };
    assert!(matches!(
        run_chunk(program.bytecode(), &vm_limits),
        ExecutionOutcome::ResourceLimitExceeded(_)
    ));
}
