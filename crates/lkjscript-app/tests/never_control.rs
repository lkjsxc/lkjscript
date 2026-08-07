#![allow(clippy::expect_used, clippy::panic)]

use lkjscript_compiler::compile_source;
use lkjscript_core::{ExecutionOutcome, ExecutionPolicy};
use lkjscript_ir::{evaluate, EvalConfig, EvalOutcome, EvalValue};
use lkjscript_vm::run_chunk;

const E2: &str = "";

fn main_source(result: &str, body: &str) -> String {
    format!("{E2}main/\nsig/\ninputs/\n/inputs\noutput/\n{result}\n/output\n/sig\n{body}\n/main\n")
}

fn assert_i64_evaluator_vm(source: &str, expected: i64) {
    let program = compile_source(source, "never-control.lkjscript")
        .expect("compile canonical control source");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(expected)),
    );
    let ExecutionOutcome::Returned(vm) = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) else {
        panic!("reference VM did not return")
    };
    assert_eq!(vm.as_i64(), Some(expected));
}

#[test]
fn early_return_and_divergent_branch_join_match_evaluator_and_vm() {
    let source = format!(
        "{E2}def/\nname/\nchoose\n/name\nfn/\nsig/\ninputs/\nbool\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nflag\nbool\n/params\ndo/\nif/\nflag\nreturn/\n41\n/return\nunit\n/if\n42\n/do\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nchoose/\ntrue\n/choose\n/main\n"
    );
    assert_i64_evaluator_vm(&source, 41);
    let joined = main_source(
        "i64",
        "if/\nfalse\ntrap/\nstring-literal/\nunselected\n/string-literal\n/trap\n7\n/if",
    );
    assert_i64_evaluator_vm(&joined, 7);
    let matched = main_source(
        "i64",
        "match/\nfalse\narms/\narm/\nbool-pattern/\ntrue\n/bool-pattern\ntrap/\nstring-literal/\nunselected\n/string-literal\n/trap\n/arm\narm/\nbool-pattern/\nfalse\n/bool-pattern\n11\n/arm\n/arms\n/match",
    );
    assert_i64_evaluator_vm(&matched, 11);
}

#[test]
fn typed_loop_break_and_unit_while_break_match_evaluator_and_vm() {
    assert_i64_evaluator_vm(
        &main_source("i64", "loop/\ntype/\ni64\n/type\nbreak/\n42\n/break\n/loop"),
        42,
    );
    assert_i64_evaluator_vm(
        &main_source(
            "i64",
            "do/\nwhile/\ntrue\nbreak/\nunit\n/break\n/while\n9\n/do",
        ),
        9,
    );
}

#[test]
fn nested_continue_targets_nearest_loop() {
    let body = "loop/\ntype/\ni64\n/type\nvar/\nname/\nj\n/name\ntype/\ni64\n/type\n0\ndo/\nwhile/\nless-than/\nj\n3\n/less-than\nset/\nj\nadd/\nj\n1\n/add\n/set\ncontinue/\n/continue\n/while\nbreak/\n2\n/break\n/do\n/var\n/loop";
    assert_i64_evaluator_vm(&main_source("i64", body), 2);
}

#[test]
fn value_trap_is_exact_in_evaluator_and_vm() {
    let source = format!(
        "{E2}def/\nname/\nfail\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nn\ni64\n/params\ntrap/\nformat-i64/\nbit-or/\nn\n0\n/bit-or\n/format-i64\n/trap\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfail/\n7\n/fail\n/main\n"
    );
    let program = compile_source(&source, "trap-control.lkjscript").expect("compile value trap");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Trapped("7".into()),
    );
    let outcome = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert!(matches!(
        &outcome,
        ExecutionOutcome::Trapped(trap) if trap.as_str() == "7"
    ));
}

#[test]
fn exit_is_structured_in_evaluator_and_vm() {
    let source = main_source(
        "i64",
        "let/\nbind/\ntext\nformat-i64/\n9\n/format-i64\n/bind\nexit/\n23\n/exit\n/let",
    );
    let program = compile_source(&source, "exit-control.lkjscript").expect("compile exit control");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Exited(23),
    );
    assert_eq!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted()
        ),
        ExecutionOutcome::Exited(23),
    );
}

#[test]
fn never_storage_and_illegal_control_are_rejected() {
    let invalid = [
        main_source("never", "trap/\nstring-literal/\nx\n/string-literal\n/trap"),
        main_source("i64", "let/\nbind/\nx\nreturn/\n1\n/return\n/bind\nx\n/let"),
        main_source("i64", "break/\n1\n/break"),
        main_source("i64", "do/\nreturn/\n1\n/return\n2\n/do"),
        main_source("i64", "while/\ntrue\nbreak/\n1\n/break\n/while"),
        main_source("i64", "var/\nname/\nx\n/name\ntype/\nnever\n/type\nunit\n1\n/var"),
        main_source("list never", "empty-list/\nnever\n/empty-list"),
        format!("{E2}product/\nname/\nbad\n/name\nfields/\nfield/\nname/\nx\n/name\ntype/\nnever\n/type\n/field\n/fields\n/product\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n1\n/main\n"),
        format!("{E2}def/\nname/\nbad\n/name\nfn/\nsig/\ninputs/\nnever\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nx\nnever\n/params\n1\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n1\n/main\n"),
        format!("{E2}enum/\nname/\nbad\n/name\nvariants/\nvariant/\nname/\nonly\n/name\nfields/\nvariant-field/\nname/\nx\n/name\ntype/\nnever\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n1\n/main\n"),
        format!("{E2}enum/\nname/\nboxed\n/name\nforall/\nt\n/forall\nvariants/\nvariant/\nname/\nempty\n/name\nfields/\n/fields\n/variant\n/variants\n/enum\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\nvariant-value/\ntype/\nboxed/\nnever\n/boxed\n/type\nvariant/\nempty\n/variant\nfields/\n/fields\n/variant-value\n1\n/do\n/main\n"),
    ];
    for source in invalid {
        assert!(compile_source(&source, "invalid-never.lkjscript").is_err());
    }
}
