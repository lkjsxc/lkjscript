#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, Limits};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

const E2: &str = "";

fn main_source(result: &str, body: &str) -> String {
    format!("{E2}main/\nsig/\n->\n{result}\n/sig\n{body}\n/main\n")
}

fn assert_i64_all(source: &str, expected: i64) {
    let program = compile_source(source, "never-control.lkjscript", &Limits::default())
        .expect("compile canonical control source");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(expected)),
    );
    let ExecutionOutcome::Returned(vm) = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ) else {
        panic!("reference VM did not return")
    };
    assert_eq!(vm.as_i64(), Some(expected));
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced baseline control"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced proof control"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("generated control did not return")
        };
        assert_eq!(value.as_i64(), Some(expected));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}

#[test]
fn early_return_and_divergent_branch_join_cross_all_engines() {
    let source = format!(
        "{E2}def/\nname/\nchoose\n/name\nfn/\nsig/\nBool\n->\nI64\n/sig\nparams/\nflag\nBool\n/params\ndo/\nif/\nflag\nreturn/\n41\n/return\nunit\n/if\n42\n/do\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nchoose/\ntrue\n/choose\n/main\n"
    );
    assert_i64_all(&source, 41);
    let joined = main_source(
        "I64",
        "if/\nfalse\ntrap/\nstr/\nunselected\n/str\n/trap\n7\n/if",
    );
    assert_i64_all(&joined, 7);
    let matched = main_source(
        "I64",
        "match/\nfalse\narms/\narm/\nbool-pattern/\ntrue\n/bool-pattern\ntrap/\nstr/\nunselected\n/str\n/trap\n/arm\narm/\nbool-pattern/\nfalse\n/bool-pattern\n11\n/arm\n/arms\n/match",
    );
    assert_i64_all(&matched, 11);
}

#[test]
fn typed_loop_break_and_unit_while_break_cross_all_engines() {
    assert_i64_all(
        &main_source("I64", "loop/\ntype/\nI64\n/type\nbreak/\n42\n/break\n/loop"),
        42,
    );
    assert_i64_all(
        &main_source(
            "I64",
            "do/\nwhile/\ntrue\nbreak/\nunit\n/break\n/while\n9\n/do",
        ),
        9,
    );
}

#[test]
fn nested_continue_targets_nearest_loop() {
    let body = "loop/\ntype/\nI64\n/type\nvar/\nname/\nj\n/name\ntype/\nI64\n/type\n0\ndo/\nwhile/\nlt/\nj\n3\n/lt\nset/\nj\n+/\nj\n1\n/+\n/set\ncontinue/\n/continue\n/while\nbreak/\n2\n/break\n/do\n/var\n/loop";
    assert_i64_all(&main_source("I64", body), 2);
}

#[test]
fn value_trap_is_exact_in_all_engines() {
    let source = format!(
        "{E2}def/\nname/\nfail\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nn\nI64\n/params\ntrap/\nstr-from-i64/\nbit-or/\nn\n0\n/bit-or\n/str-from-i64\n/trap\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nfail/\n7\n/fail\n/main\n"
    );
    let program = compile_source(&source, "trap-control.lkjscript", &Limits::default())
        .expect("compile value trap");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Trapped("7".into()),
    );
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced baseline trap");
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced proof trap");
    assert!(optimized.stats.optimization_certificate_records > 0);
    assert!(optimized.stats.optimization_checker_passes > 0);
    assert!(optimized.stats.optimization_reconstruction_passes > 0);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    let outcomes = [
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ),
        baseline.outcome,
        optimized.outcome,
    ];
    for outcome in outcomes {
        assert!(matches!(
            outcome,
            ExecutionOutcome::Trapped(trap) if trap.as_str() == "7"
        ));
    }
}

#[test]
fn exit_is_structured_in_all_engines() {
    let source = main_source("I64", "exit/\n23\n/exit");
    let program = compile_source(&source, "exit-control.lkjscript", &Limits::default())
        .expect("compile exit control");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Exited(23),
    );
    assert_eq!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default()
        ),
        ExecutionOutcome::Exited(23),
    );
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced baseline exit"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("forced proof exit"),
    ] {
        assert_eq!(execution.outcome, ExecutionOutcome::Exited(23));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
    }
}

#[test]
fn never_storage_and_illegal_control_are_rejected() {
    let invalid = [
        main_source("Never", "trap/\nstr/\nx\n/str\n/trap"),
        main_source("I64", "let/\nbind/\nx\nreturn/\n1\n/return\n/bind\nx\n/let"),
        main_source("I64", "break/\n1\n/break"),
        main_source("I64", "do/\nreturn/\n1\n/return\n2\n/do"),
        main_source("I64", "while/\ntrue\nbreak/\n1\n/break\n/while"),
        main_source("I64", "var/\nname/\nx\n/name\ntype/\nNever\n/type\nunit\n1\n/var"),
        main_source("List Never", "empty-list/\nNever\n/empty-list"),
        format!("{E2}product/\nname/\nBad\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\nNever\n/type\n/field\n/fields\n/product\nmain/\nsig/\n->\nI64\n/sig\n1\n/main\n"),
        format!("{E2}def/\nname/\nbad\n/name\nfn/\nsig/\nNever\n->\nI64\n/sig\nparams/\nx\nNever\n/params\n1\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\n1\n/main\n"),
        format!("{E2}enum/\nname/\nBad\n/name\nvariants/\nvariant/\nname/\nOnly\n/name\nfields/\nvariant-field/\nname/\nx\n/name\ntype/\nNever\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\nmain/\nsig/\n->\nI64\n/sig\n1\n/main\n"),
        format!("{E2}enum/\nname/\nBoxed\n/name\nforall/\nT\n/forall\nvariants/\nvariant/\nname/\nEmpty\n/name\nfields/\n/fields\n/variant\n/variants\n/enum\nmain/\nsig/\n->\nI64\n/sig\ndo/\nvariant-value/\ntype/\nBoxed/\nNever\n/Boxed\n/type\nvariant/\nEmpty\n/variant\nfields/\n/fields\n/variant-value\n1\n/do\n/main\n"),
    ];
    for source in invalid {
        assert!(compile_source(&source, "invalid-never.lkjscript", &Limits::default()).is_err());
    }
}
