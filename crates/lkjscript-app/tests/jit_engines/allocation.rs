use crate::canonical::{compile, evaluator, execution, forced, Scalar};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig};
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
            concat!(
                "match/\nunwrap-err/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n-1\n1\n",
                "/buf-slice\n/unwrap-err\narms/\narm/\nvariant-pattern/\ntype/\n",
                "SystemError/\n/SystemError\n/type\nvariant/\nUnsupported\n/variant\nfields/\n",
                "variant-field-pattern/\nname/\ncode\n/name\nwildcard/\n/wildcard\n",
                "/variant-field-pattern\nvariant-field-pattern/\nname/\ndetail\n/name\n",
                "wildcard/\n/wildcard\n/variant-field-pattern\n/fields\n/variant-pattern\n",
                "true\n/arm\narm/\nwildcard/\n/wildcard\nfalse\n/arm\n/arms\n/match",
            ),
        ),
        (
            "buf-slice-negative-length.lkjscript",
            "not/\nis-ok/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n0\n-1\n/buf-slice\n/is-ok\n/not",
        ),
        (
            "buf-slice-out-of-range.lkjscript",
            "not/\nis-ok/\nbuf-slice/\nbuf-new/\n1\n/buf-new\n1\n1\n/buf-slice\n/is-ok\n/not",
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
        let baseline = forced(&source, name);
        assert_eq!(execution(baseline.outcome), expected);
        assert_eq!(baseline.stats.vm_fallbacks, 0);
        let proof = execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced proof SystemError execution");
        assert_eq!(execution(proof.outcome), expected);
        assert_eq!(proof.stats.vm_fallbacks, 0);
    }

    let invalid_utf8 = concat!(
        "main/\nsig/\n->\nBool\n/sig\nvar/\nname/\nb\n/name\n",
        "type/\nBuf\n/type\nbuf-new/\n1\n/buf-new\ndo/\nbuf-set/\nb\n0\n255\n/buf-set\n",
        "match/\nunwrap-err/\nbuf-to-str/\nb\n/buf-to-str\n/unwrap-err\narms/\narm/\n",
        "variant-pattern/\ntype/\nUtf8Error/\n/Utf8Error\n/type\nvariant/\n",
        "InvalidLeadingByte\n/variant\nfields/\nvariant-field-pattern/\nname/\noffset\n/name\n",
        "i64-pattern/\n0\n/i64-pattern\n/variant-field-pattern\n/fields\n/variant-pattern\n",
        "true\n/arm\narm/\nwildcard/\n/wildcard\nfalse\n/arm\n/arms\n/match\n",
        "/do\n/var\n/main\n",
    );
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
    let baseline = forced(invalid_utf8, "buf-to-str-error.lkjscript");
    assert_eq!(execution(baseline.outcome), expected);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    assert!(baseline.stats.native_entries > 0);
    let proof = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced proof UTF-8 error execution");
    assert_eq!(execution(proof.outcome), expected);
    assert_eq!(proof.stats.vm_fallbacks, 0);
    assert!(proof.stats.native_entries > 0);
}
