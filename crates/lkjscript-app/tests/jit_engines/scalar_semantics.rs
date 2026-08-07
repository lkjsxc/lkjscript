use crate::canonical::{compile, execution, forced, Scalar};
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn i64_multiblock_loop_and_direct_call_match_vm() {
    let source = "def/\nname/\nadd-if-even\n/name\nfn/\nsig/\ninputs/\ni64\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nacc\ni64\ni\ni64\n/params\nif/\nequal-value/\nbit-and/\ni\n1\n/bit-and\n0\n/equal-value\nadd/\nacc\ni\n/add\nacc\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\ni\n/name\ntype/\ni64\n/type\n0\nvar/\nname/\nacc\n/name\ntype/\ni64\n/type\n0\ndo/\nwhile/\nless-than/\ni\n100\n/less-than\ndo/\nset/\nacc\nadd-if-even/\nacc\ni\n/add-if-even\n/set\nset/\ni\nadd/\ni\n1\n/add\n/set\n/do\n/while\nacc\n/do\n/var\n/var\n/main\n";
    let program = compile(source, "i64-cfg.lkjscript");
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ));
    let native = forced(source, "i64-cfg.lkjscript");
    assert_eq!(vm, Scalar::I64(2450));
    assert_eq!(execution(native.outcome), vm);
    assert!(native.stats.direct_native_calls >= 100);
}

#[test]
fn checked_i64_traps_exit_and_explicit_trap_remain_structured() {
    for (name, expression) in [
        ("overflow.lkjscript", "add/\n9223372036854775807\n1\n/add"),
        ("divide-zero.lkjscript", "divide/\n1\n0\n/divide"),
        (
            "divide-overflow.lkjscript",
            "divide/\n-9223372036854775808\n-1\n/divide",
        ),
    ] {
        let source = format!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n{expression}\n/main\n"
        );
        let program = compile(&source, name);
        assert_eq!(
            execution(run_chunk(
                program.bytecode(),
                &lkjscript_vm::ExecutionInputs::default(),
                &ExecutionPolicy::unrestricted()
            )),
            Scalar::Trapped
        );
        assert_eq!(execution(forced(&source, name).outcome), Scalar::Trapped);
        let optimized = execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("optimizing trap remains structured");
        assert_eq!(execution(optimized.outcome), Scalar::Trapped);
    }

    for (name, operation, left, right, expected_trap) in [
        (
            "duplicate-overflow.lkjscript",
            "add",
            "9223372036854775807",
            "1",
            "checked I64 overflow",
        ),
        (
            "duplicate-division.lkjscript",
            "divide",
            "1",
            "0",
            "div: I64 division by zero",
        ),
    ] {
        let expression = format!("{operation}/\nz\none\n/{operation}");
        let source = format!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\nbind/\nz\n{left}\n/bind\nlet/\nbind/\none\n{right}\n/bind\nlet/\nbind/\nfirst\n{expression}\n/bind\nlet/\nbind/\nsecond\n{expression}\n/bind\nsecond\n/let\n/let\n/let\n/let\n/main\n"
        );
        let program = compile(&source, name);
        let baseline = execute_forced(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("duplicate checked baseline trap");
        let optimized = execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
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
        assert_eq!(optimized.stats.checked_i64_rewrites, 1);
    }

    let exit =
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nexit/\n17\n/exit\n/main\n";
    assert_eq!(
        execution(forced(exit, "exit.lkjscript").outcome),
        Scalar::Exited(17)
    );
    let exit_program = compile(exit, "optimizing-exit.lkjscript");
    assert_eq!(
        execution(
            execute_optimizing(
                exit_program.ssa(),
                &ExecutionPolicy::unrestricted(),
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
            "bool",
            "equal-f64-bits/\n0.0\n-0.0\n/equal-f64-bits",
            Scalar::Bool(false),
        ),
        (
            "nan-order.lkjscript",
            "bool",
            "less-than/\ndivide/\n0.0\n0.0\n/divide\n1.0\n/less-than",
            Scalar::Bool(false),
        ),
        (
            "mixed.lkjscript",
            "f64",
            "add/\nconvert-i64-to-f64-rounded/\n9007199254740993\n/convert-i64-to-f64-rounded\n0.5\n/add",
            Scalar::F64((9_007_199_254_740_993_i64 as f64 + 0.5).to_bits()),
        ),
    ];
    for (name, ty, expression, expected) in cases {
        let source = format!(
            "main/\nsig/\ninputs/\n/inputs\noutput/\n{ty}\n/output\n/sig\n{expression}\n/main\n"
        );
        let program = compile(&source, name);
        let vm = execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ));
        let forced = forced(&source, name);
        if name == "mixed.lkjscript" {
            assert_eq!(forced.stats.runtime_heap_attempts, 0);
            assert!(forced.stats.code_objects.iter().all(|object| !object
                .runtime_calls
                .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatch)));
        }
        let native = execution(forced.outcome);
        assert_eq!(vm, expected, "VM oracle for {name}");
        assert_eq!(native, vm, "native result for {name}");
    }
}
