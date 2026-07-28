use crate::oracle::{compare_source, ScalarOutcome};
use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome};
use lkjscript_vm::run_chunk;

#[test]
fn owned_buf_borrows_moves_and_mutation_match_evaluator_and_vm() {
    let source = "def/\nname/\npass-owned\n/name\nfn/\nsig/\ninputs/\nbyte-vector\n/inputs\noutput/\nbyte-vector\n/output\n/sig\nparams/\nb\nbyte-vector\n/params\nmove/\nb\n/move\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nnew-byte-vector/\n2\n/new-byte-vector\n/bind\ndo/\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nbyte-slice-mut-set-byte/\nm\n1\n77\n/byte-slice-mut-set-byte\n/let\nlet/\nbind/\nc\npass-owned/\nmove/\nb\n/move\n/pass-owned\n/bind\nlet/\nbind/\nr\nborrow/\nc\n/borrow\n/bind\nbyte-slice-byte-at/\nr\n1\n/byte-slice-byte-at\n/let\n/let\n/do\n/let\n/main\n";
    assert_eq!(
        compare_source(source, "owned-buffer.lkjscript"),
        ScalarOutcome::I64(77)
    );

    let var_source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nb\n/name\ntype/\nbyte-vector\n/type\nnew-byte-vector/\n2\n/new-byte-vector\ndo/\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\nlet/\nbind/\nm\nborrow-mut/\nb\n/borrow-mut\n/bind\nbyte-slice-mut-set-byte/\nm\n0\n91\n/byte-slice-mut-set-byte\n/let\nmove/\nb\n/move\nset/\nb\nnew-byte-vector/\n3\n/new-byte-vector\n/set\nlet/\nbind/\nfresh\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\nmove/\nb\n/move\nset/\nb\nmove/\nfresh\n/move\n/set\nbyte-slice-length/\nborrow/\nb\n/borrow\n/byte-slice-length\n/do\n/let\n/do\n/var\n/main\n";
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
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &vm_limits
        ),
        ExecutionOutcome::ResourceLimitExceeded(_)
    ));
}

#[test]
fn implicit_owned_resource_close_runs_in_reference_vm() {
    let path = format!(
        "/tmp/lkjscript-implicit-resource-drop-{}",
        std::process::id()
    );
    std::fs::write(&path, b"resource").expect("create implicit-drop fixture");
    let source = format!(
        concat!(
            "main/\nsig/\ninputs/\ncapability/\nfile-system\n/capability\n/inputs\n",
            "output/\nunit\n/output\n/sig\nparams/\nfile-system\ncapability/\n",
            "file-system\n/capability\n/params\nlet/\nbind/\nreader\nunwrap-ok/\n",
            "open-file-reader/\nfile-system\nunwrap-ok/\nconvert-string-to-path/\n",
            "string-literal/\n{}\n/string-literal\n/convert-string-to-path\n/unwrap-ok\n",
            "/open-file-reader\n/unwrap-ok\n/bind\nunit\n/let\n/main\n"
        ),
        path
    );
    let program = compile_source(
        &source,
        "implicit-resource-drop.lkjscript",
        &Limits::default(),
    )
    .expect("compile implicit resource close");
    let outcome = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs {
            arguments: Vec::new(),
            capabilities: vec![lkjscript_core::CapabilityKind::FileSystem],
        },
        &ExecutionConfig::default(),
    );
    let _removed = std::fs::remove_file(path);
    assert!(
        matches!(outcome, ExecutionOutcome::Returned(_)),
        "{outcome:?}"
    );
}

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
