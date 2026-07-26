use crate::canonical::{compile, execution, forced, Scalar};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn i64_multiblock_loop_and_direct_call_match_vm() {
    let source = "def/\nname/\nadd-if-even\n/name\nfn/\nsig/\nI64\nI64\n->\nI64\n/sig\nparams/\nacc\nI64\ni\nI64\n/params\nif/\nequal-value/\nbit-and/\ni\n1\n/bit-and\n0\n/equal-value\n+/\nacc\ni\n/+\nacc\n/if\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nvar/\nname/\ni\n/name\ntype/\nI64\n/type\n0\nvar/\nname/\nacc\n/name\ntype/\nI64\n/type\n0\ndo/\nwhile/\nlt/\ni\n100\n/lt\ndo/\nset/\nacc\nadd-if-even/\nacc\ni\n/add-if-even\n/set\nset/\ni\n+/\ni\n1\n/+\n/set\n/do\n/while\nacc\n/do\n/var\n/var\n/main\n";
    let program = compile(source, "i64-cfg.lkjscript");
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    let native = forced(source, "i64-cfg.lkjscript");
    assert_eq!(vm, Scalar::I64(2450));
    assert_eq!(execution(native.outcome), vm);
    assert!(native.stats.direct_native_calls >= 100);
}

#[test]
fn checked_i64_traps_exit_and_explicit_trap_remain_structured() {
    for (name, expression) in [
        ("overflow.lkjscript", "+/\n9223372036854775807\n1\n/+"),
        ("divide-zero.lkjscript", "div/\n1\n0\n/div"),
        (
            "divide-overflow.lkjscript",
            "div/\n-9223372036854775808\n-1\n/div",
        ),
    ] {
        let source = format!("main/\nsig/\n->\nI64\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);
        assert_eq!(
            execution(run_chunk(
                program.bytecode(),
                &lkjscript_vm::ExecutionInputs::default(),
                &ExecutionConfig::default()
            )),
            Scalar::Trapped
        );
        assert_eq!(execution(forced(&source, name).outcome), Scalar::Trapped);
        let optimized = execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("optimizing trap remains structured");
        assert_eq!(execution(optimized.outcome), Scalar::Trapped);
        assert_eq!(optimized.stats.baseline_native_entries, 0);
    }

    for (name, operation, left, right, expected_trap) in [
        (
            "duplicate-overflow.lkjscript",
            "+",
            "9223372036854775807",
            "1",
            "checked I64 overflow",
        ),
        (
            "duplicate-division.lkjscript",
            "div",
            "1",
            "0",
            "div: I64 division by zero",
        ),
    ] {
        let expression = format!("{operation}/\nz\none\n/{operation}");
        let source = format!(
            "main/\nsig/\n->\nI64\n/sig\nlet/\nbind/\nz\n{left}\n/bind\nlet/\nbind/\none\n{right}\n/bind\nlet/\nbind/\nfirst\n{expression}\n/bind\nlet/\nbind/\nsecond\n{expression}\n/bind\nsecond\n/let\n/let\n/let\n/let\n/main\n"
        );
        let program = compile(&source, name);
        let baseline = execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("duplicate checked baseline trap");
        let optimized = execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("duplicate checked optimizing trap");
        assert!(matches!(
            baseline.outcome,
            ExecutionOutcome::Trapped(ref trap) if trap.as_str() == expected_trap
        ));
        assert!(matches!(
            optimized.outcome,
            ExecutionOutcome::Trapped(ref trap) if trap.as_str() == expected_trap
        ));
        assert_eq!(optimized.stats.baseline_native_entries, 0);
        assert_eq!(optimized.stats.vm_fallbacks, 0);
        assert_eq!(optimized.stats.checked_i64_rewrites, 1);
    }

    let exit = "main/\nsig/\n->\nUnit\n/sig\nexit/\n17\n/exit\n/main\n";
    assert_eq!(
        execution(forced(exit, "exit.lkjscript").outcome),
        Scalar::Exited(17)
    );
    let exit_program = compile(exit, "optimizing-exit.lkjscript");
    assert_eq!(
        execution(
            execute_optimizing(
                exit_program.ssa(),
                &ExecutionConfig::default(),
                JitConfig::default(),
            )
            .expect("optimizing exit remains structured")
            .outcome,
        ),
        Scalar::Exited(17)
    );
}

#[test]
fn f64_bits_ieee_comparisons_and_mixed_conversion_are_exact() {
    let cases = [
        (
            "signed-zero.lkjscript",
            "Bool",
            "f64-bits-equal/\n0.0\n-0.0\n/f64-bits-equal",
            Scalar::Bool(false),
        ),
        (
            "nan-order.lkjscript",
            "Bool",
            "lt/\ndiv/\n0.0\n0.0\n/div\n1.0\n/lt",
            Scalar::Bool(false),
        ),
        (
            "mixed.lkjscript",
            "F64",
            "+/\nf64-from-i64-rounded/\n9007199254740993\n/f64-from-i64-rounded\n0.5\n/+",
            Scalar::F64((9_007_199_254_740_993_i64 as f64 + 0.5).to_bits()),
        ),
    ];
    for (name, ty, expression, expected) in cases {
        let source = format!("main/\nsig/\n->\n{ty}\n/sig\n{expression}\n/main\n");
        let program = compile(&source, name);
        let vm = execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ));
        let native = execution(forced(&source, name).outcome);
        assert_eq!(vm, expected, "VM oracle for {name}");
        assert_eq!(native, vm, "native result for {name}");
    }
}
