use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn forced_mode_executes_host_independent_allocation_and_recursion_without_fallback() {
    let source = concat!(
        "main/\nsig/\ninputs/\ncapability/\nstdio\n/capability\n/inputs\noutput/\nunit\n/output\n/sig\n",
        "params/\nstdio\ncapability/\nstdio\n/capability\n/params\n",
        "flush/\nstdio\n/flush\n/main\n"
    );
    let program = compile(source, "host.lkjscript");
    let error = execute_forced(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect_err("forced native must reject unsupported semantics");
    assert!(matches!(
        error.code(),
        FailureCode::UnsupportedType | FailureCode::UnsupportedOperation
    ));

    let recursion = "def/\nname/\nrecur\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nn\ni64\n/params\nif/\nless-than-or-equal/\nn\n0\n/less-than-or-equal\n0\nrecur/\nsubtract/\nn\n1\n/subtract\n/recur\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nrecur/\n3\n/recur\n/main\n";
    let program = compile(recursion, "recursion.lkjscript");
    let executed = execute_forced(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("direct recursion is a bounded native SCC");
    assert!(
        matches!(executed.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert!(executed.stats.direct_native_calls >= 3);
    assert_eq!(executed.stats.vm_fallbacks, 0);

    let allocation = compile(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\nempty-string/\n/empty-string\n/main\n",
        "allocation.lkjscript",
    );
    let executed = execute_forced(
        allocation.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("source allocation reaches generated structural dispatch");
    assert!(
        matches!(executed.outcome, ExecutionOutcome::Returned(value) if value.as_str() == Some(""))
    );
    assert_eq!(executed.stats.vm_fallbacks, 0);
    assert_eq!(executed.stats.runtime_heap_attempts, 0);
    assert_eq!(executed.stats.runtime_heap_successes, 0);
    assert!(executed.stats.structural_runtime_calls > 0);
    assert_eq!(executed.stats.native_structural.live_roots, 0);
    assert!(executed.stats.native_entries > 0);
}

#[test]
fn bytes_conversion_results_match_evaluator_vm_and_forced_native_fails_closed() {
    let success = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "let/\nbind/\nb\nconvert-string-to-bytes/\nempty-string/\n/empty-string\n",
        "/convert-string-to-bytes\n/bind\nstring-byte-length/\nunwrap-ok/\n",
        "convert-bytes-to-string/\nb\n/convert-bytes-to-string\n/unwrap-ok\n/string-byte-length\n/let\n/main\n",
    );
    let success_program = compile(success, "bytes-to-string-success.lkjscript");
    let expected = Scalar::I64(0);
    assert_eq!(
        evaluator(evaluate(success_program.ssa(), &EvalConfig::default())),
        expected
    );
    assert_eq!(
        execution(run_chunk(
            success_program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        )),
        expected
    );
    assert_eq!(
        execute_forced(
            success_program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect_err("mixed structural string and unique bytes reject before native entry")
        .code(),
        FailureCode::UnsupportedType
    );

    let invalid_utf8 = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n",
        "let/\nbind/\nv\nnew-byte-vector/\n1\n/new-byte-vector\n/bind\ndo/\n",
        "byte-slice-mut-set-byte/\nborrow-mut/\nv\n/borrow-mut\n0\n255\n/byte-slice-mut-set-byte\n",
        "let/\nbind/\nb\nfreeze-byte-vector/\nmove/\nv\n/move\n/freeze-byte-vector\n/bind\n",
        "not/\nis-ok/\nconvert-bytes-to-string/\nb\n/convert-bytes-to-string\n/is-ok\n/not\n",
        "/let\n/do\n/let\n/main\n",
    );
    let program = compile(invalid_utf8, "bytes-to-string-error.lkjscript");
    let expected = Scalar::Bool(true);
    assert_eq!(
        evaluator(evaluate(program.ssa(), &EvalConfig::default())),
        expected
    );
    assert_eq!(
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        )),
        expected
    );
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        ),
        execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        ),
    ] {
        assert_eq!(
            result
                .expect_err("mixed structural string and unique bytes reject before native entry")
                .code(),
            FailureCode::UnsupportedType
        );
    }
}
