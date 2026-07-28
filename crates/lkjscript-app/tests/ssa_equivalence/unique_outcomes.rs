use crate::oracle::{compare_source, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome};
use lkjscript_vm::run_chunk;

#[test]
fn byte_vector_trap_early_return_and_owner_return_cleanup_match() {
    let trap_source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\nbyte-slice-byte-at/\nborrow/\nb\n/borrow\n9\n/byte-slice-byte-at\n/let\n/main\n";
    let trapped = compile_source(
        trap_source,
        "byte-vector-trap.lkjscript",
        &Limits::default(),
    )
    .expect("compile byte-vector trap fixture");
    assert!(matches!(
        evaluate(trapped.ssa(), &EvalConfig::default()),
        EvalOutcome::Trapped(_)
    ));
    assert!(matches!(
        run_chunk(
            trapped.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Trapped(_)
    ));

    let early_source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\nreturn/\n7\n/return\n/let\n/main\n";
    assert_eq!(
        compare_source(early_source, "byte-vector-early-return.lkjscript"),
        ScalarOutcome::I64(7)
    );

    let owner_source = "main/\nsig/\ninputs/\n/inputs\noutput/\nbyte-vector\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\nmove/\nb\n/move\n/let\n/main\n";
    let owner = compile_source(
        owner_source,
        "byte-vector-return.lkjscript",
        &Limits::default(),
    )
    .expect("compile returned byte-vector fixture");
    assert_eq!(
        evaluate(owner.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(lkjscript_ir::EvalValue::ReturnedByteVector(vec![0, 0]))
    );
    assert!(matches!(
        run_chunk(
            owner.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ),
        ExecutionOutcome::Returned(value) if value.as_byte_vector() == Some(&[0, 0][..])
    ));
}
