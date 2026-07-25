use crate::canonical::{compile, evaluator, execution, forced, Scalar};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, FailureCode, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn forced_mode_executes_host_independent_allocation_and_recursion_without_fallback() {
    let unsupported = [
        (
            "host.lkjscript",
            "main/\nsig/\n->\nUnit\n/sig\nflush/\n/flush\n/main\n",
        ),
        (
            "owned-buffer.lkjscript",
            "main/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nb\nowned-buf-new/\n1\n/owned-buf-new\n/bind\nowned-buf-ref/\nborrow/\nb\n/borrow\n0\n/owned-buf-ref\n/let\n/main\n",
        ),
    ];
    for (name, source) in unsupported {
        let program = compile(source, name);
        let error = execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect_err("forced native must reject unsupported semantics");
        assert!(matches!(
            error.code(),
            FailureCode::UnsupportedType | FailureCode::UnsupportedOperation
        ));
        if name == "owned-buffer.lkjscript" {
            let diagnostic = error.to_string();
            assert!(
                diagnostic.contains("reference") || diagnostic.contains("ownership"),
                "forced owned-buffer rejection was not ownership-specific: {diagnostic}"
            );
        }
    }

    let recursion = "def/\nname/\nrecur\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nn\nI64\n/params\nif/\nlte/\nn\n0\n/lte\n0\nrecur/\n-/\nn\n1\n/-\n/recur\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nrecur/\n3\n/recur\n/main\n";
    let program = compile(recursion, "recursion.lkjscript");
    let executed = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("direct recursion is a bounded native SCC");
    assert!(
        matches!(executed.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert!(executed.stats.direct_native_calls >= 3);
    assert_eq!(executed.stats.vm_fallbacks, 0);

    let allocation = compile(
        "main/\nsig/\n->\nStr\n/sig\nempty-str/\n/empty-str\n/main\n",
        "allocation.lkjscript",
    );
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let executed = execute_forced(allocation.ssa(), &ExecutionConfig::default(), config)
        .expect("source allocation reaches generated heap dispatch");
    assert!(
        matches!(executed.outcome, ExecutionOutcome::Returned(value) if value.as_str() == Some(""))
    );
    assert_eq!(executed.stats.vm_fallbacks, 0);
    assert_eq!(executed.stats.allocations, 1);
    assert_eq!(executed.stats.collections, 1);
    assert_eq!(executed.stats.runtime_heap_attempts, 1);
    assert_eq!(executed.stats.runtime_heap_successes, 1);
    assert!(executed.stats.native_entries > 0);
}

#[test]
fn generated_buffer_result_boundaries_match_vm_and_evaluator_exactly() {
    let cases = [
        (
            "buf-slice-negative-offset.lkjscript",
            "equal-value/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n-1\n1\n/buf-slice\n/unwrap-err\nstr/\nbuf-slice offset out of range\n/str\n/equal-value",
        ),
        (
            "buf-slice-negative-length.lkjscript",
            "equal-value/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n0\n-1\n/buf-slice\n/unwrap-err\nstr/\nbuf-slice length out of range\n/str\n/equal-value",
        ),
        (
            "buf-slice-out-of-range.lkjscript",
            "equal-value/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n1\n1\n/buf-slice\n/unwrap-err\nstr/\nbuf-slice range out of bounds\n/str\n/equal-value",
        ),
    ];
    for (name, expression) in cases {
        let source = format!("main/\nsig/\n->\nBool\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);
        let expected = Scalar::Bool(true);
        assert_eq!(
            evaluator(evaluate(program.ssa(), &EvalConfig::default())),
            expected
        );
        assert_eq!(
            execution(run_chunk(program.bytecode(), &ExecutionConfig::default())),
            expected
        );
        assert_eq!(execution(forced(&source, name).outcome), expected);
    }

    let invalid_utf8 = "main/\nsig/\n->\nBool\n/sig\nvar/\nname/\nb\n/name\ntype/\nBuf\n/type\nbuf-new/\n1\n/buf-new\ndo/\nbuf-set/\nb\n0\n255\n/buf-set\nequal-value/\nunwrap-err/\nbuf-to-str/\nb\n/buf-to-str\n/unwrap-err\nstr/\nbuf-to-str: invalid UTF-8\n/str\n/equal-value\n/do\n/var\n/main\n";
    let program = compile(invalid_utf8, "buf-to-str-error.lkjscript");
    let expected = Scalar::Bool(true);
    assert_eq!(
        evaluator(evaluate(program.ssa(), &EvalConfig::default())),
        expected
    );
    assert_eq!(
        execution(run_chunk(program.bytecode(), &ExecutionConfig::default())),
        expected
    );
    assert_eq!(
        execution(forced(invalid_utf8, "buf-to-str-error.lkjscript").outcome),
        expected
    );
}
