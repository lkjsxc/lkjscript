use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_vm::run_chunk;

const STATIC: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nbytes-byte-at/\nbytes-literal/\n00ff10\n/bytes-literal\n1\n/bytes-byte-at\n/main\n";
const STATIC_LOCAL: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nb\nbytes-literal/\n00ff10\n/bytes-literal\n/bind\nadd/\nbytes-length/\nb\n/bytes-length\nbytes-byte-at/\nb\n1\n/bytes-byte-at\n/add\n/let\n/main\n";

const DYNAMIC: &str = "main/\nsig/\ninputs/\n/inputs\noutput/\nbytes\n/output\n/sig\nlet/\nbind/\nv\nnew-byte-vector/\n3\n/new-byte-vector\n/bind\nlet/\nbind/\nb\nfreeze-byte-vector/\nmove/\nv\n/move\n/freeze-byte-vector\n/bind\nclone-bytes/\nb\n/clone-bytes\n/let\n/let\n/main\n";

#[test]
fn immutable_bytes_static_and_dynamic_match_evaluator_and_vm() {
    let static_program = compile_source(STATIC, "static-bytes.lkjscript", &Limits::default())
        .expect("compile static bytes");
    assert_eq!(
        evaluate(static_program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(255))
    );
    assert!(matches!(
        run_chunk(
            static_program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(255)
    ));

    let local = compile_source(
        STATIC_LOCAL,
        "static-bytes-local.lkjscript",
        &Limits::default(),
    )
    .expect("compile copyable static bytes local");
    assert_eq!(
        evaluate(local.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(258))
    );
    assert!(matches!(
        run_chunk(local.bytecode(), &lkjscript_vm::ExecutionInputs::default(), &ExecutionConfig::default()),
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(258)
    ));

    let dynamic_program = compile_source(DYNAMIC, "dynamic-bytes.lkjscript", &Limits::default())
        .expect("compile dynamic bytes");
    assert_eq!(
        evaluate(dynamic_program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::ReturnedBytes(vec![0, 0, 0]))
    );
    assert!(matches!(
        run_chunk(
            dynamic_program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Returned(value) if value.as_bytes() == Some(&[0, 0, 0][..])
    ));
}

#[test]
fn bytes_slice_copy_thaw_and_allocation_failure_match() {
    let slice = STATIC
        .replace("output/\ni64\n/output", "output/\nbytes\n/output")
        .replace(
            "bytes-byte-at/\nbytes-literal/\n00ff10\n/bytes-literal\n1\n/bytes-byte-at",
            "copy-bytes-slice/\nbytes-literal/\n00ff10\n/bytes-literal\n1\n2\n/copy-bytes-slice",
        );
    let program = compile_source(&slice, "bytes-slice.lkjscript", &Limits::default())
        .expect("compile bytes slice copy");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::ReturnedBytes(vec![255, 16]))
    );
    assert!(matches!(
        run_chunk(program.bytecode(), &lkjscript_vm::ExecutionInputs::default(), &ExecutionConfig::default()),
        ExecutionOutcome::Returned(value) if value.as_bytes() == Some(&[255, 16][..])
    ));

    let thaw = slice
        .replace("output/\nbytes\n/output", "output/\nbyte-vector\n/output")
        .replace("copy-bytes-slice/", "thaw-bytes/")
        .replace("\n1\n2\n/copy-bytes-slice", "\n/thaw-bytes");
    let program = compile_source(&thaw, "bytes-thaw.lkjscript", &Limits::default())
        .expect("compile static bytes thaw");
    assert!(matches!(
        run_chunk(program.bytecode(), &lkjscript_vm::ExecutionInputs::default(), &ExecutionConfig::default()),
        ExecutionOutcome::Returned(value) if value.as_byte_vector() == Some(&[0, 255, 16][..])
    ));

    let program = compile_source(DYNAMIC, "bytes-allocation.lkjscript", &Limits::default())
        .expect("compile bytes allocation failure");
    assert!(matches!(
        evaluate(
            program.ssa(),
            &EvalConfig {
                max_allocations: 0,
                ..EvalConfig::default()
            }
        ),
        EvalOutcome::ResourceLimitExceeded(_)
    ));
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig {
                max_allocations: 0,
                ..ExecutionConfig::default()
            }
        ),
        ExecutionOutcome::ResourceLimitExceeded(_)
    ));
}

#[test]
fn bytes_literal_and_range_failures_are_pre_effect_and_deterministic() {
    for payload in ["0", "FF", "0g", "00 11"] {
        let source = STATIC.replace("00ff10", payload);
        assert!(compile_source(&source, "bad-bytes.lkjscript", &Limits::default()).is_err());
    }
    let source = STATIC.replace("1\n/bytes-byte-at", "9\n/bytes-byte-at");
    let program = compile_source(&source, "bytes-bounds.lkjscript", &Limits::default())
        .expect("compile bounds trap");
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
}
