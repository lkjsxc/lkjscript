use crate::canonical::compile;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, ResourceLimitKind};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome};
use lkjscript_jit::{execute_forced, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn evaluator_vm_and_native_buffer_results_share_tiny_resource_boundaries() {
    let cases = [
        (
            "buf-to-str-success-limits.lkjscript",
            "Result\nStr\nUtf8Error",
            "buf-to-str/\nbuf-new/\n0\n/buf-new\n/buf-to-str",
        ),
        (
            "buf-to-str-error-limits.lkjscript",
            "Result\nStr\nUtf8Error",
            "var/\nname/\nb\n/name\ntype/\nBuf\n/type\nbuf-new/\n1\n/buf-new\ndo/\nbuf-set/\nb\n0\n255\n/buf-set\nbuf-to-str/\nb\n/buf-to-str\n/do\n/var",
        ),
        (
            "buf-slice-success-limits.lkjscript",
            "Result\nBuf\nSystemError",
            "buf-slice/\nbuf-new/\n1\n/buf-new\n0\n1\n/buf-slice",
        ),
        (
            "buf-slice-error-limits.lkjscript",
            "Result\nBuf\nSystemError",
            "buf-slice/\nbuf-new/\n1\n/buf-new\n-1\n1\n/buf-slice",
        ),
    ];
    for (name, return_type, expression) in cases {
        let source = format!("main/\nsig/\n->\n{return_type}\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);

        let eval_allocations = EvalConfig {
            max_allocations: 2,
            ..EvalConfig::default()
        };
        assert!(matches!(
            evaluate(program.ssa(), &eval_allocations),
            EvalOutcome::ResourceLimitExceeded(ref kind) if kind == "allocations"
        ));
        let allocation_limits = ExecutionConfig {
            max_allocations: 2,
            ..ExecutionConfig::default()
        };
        assert!(matches!(
            run_chunk(
                program.bytecode(),
                &lkjscript_vm::ExecutionInputs::default(),
                &allocation_limits
            ),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));
        assert!(matches!(
            execute_forced(program.ssa(), &allocation_limits, JitConfig::default())
                .expect("native allocation limit is structured")
                .outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
        ));

        let eval_heap = EvalConfig {
            max_heap_bytes: 1,
            ..EvalConfig::default()
        };
        assert!(matches!(
            evaluate(program.ssa(), &eval_heap),
            EvalOutcome::ResourceLimitExceeded(ref kind) if kind == "heap bytes"
        ));
        let heap_limits = ExecutionConfig {
            max_heap_bytes: 1,
            ..ExecutionConfig::default()
        };
        assert!(matches!(
            run_chunk(
                program.bytecode(),
                &lkjscript_vm::ExecutionInputs::default(),
                &heap_limits
            ),
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
        ));
        assert!(matches!(
            execute_forced(program.ssa(), &heap_limits, JitConfig::default())
                .expect("native heap limit is structured")
                .outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
        ));
    }
}
