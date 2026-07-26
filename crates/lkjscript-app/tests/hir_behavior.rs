#![allow(clippy::expect_used)]

use lkjscript_compiler::{compile_source, ExecutableProgram};
use lkjscript_core::{Error, ExecutionConfig, ExecutionOutcome, Limits, Op, OwnedValue};
use lkjscript_vm::{run_chunk, run_chunk_with_args};

fn compile(source: &str) -> lkjscript_core::Result<ExecutableProgram> {
    let source = format!("edition/\n2\n/edition\n{source}");
    compile_source(&source, "hir-behavior.lkjscript", &Limits::default())
}

fn returned(outcome: ExecutionOutcome) -> lkjscript_core::Result<OwnedValue> {
    match outcome {
        ExecutionOutcome::Returned(value) => Ok(value),
        ExecutionOutcome::Trapped(trap) => Err(Error::msg(trap.to_string())),
        other => Err(Error::msg(other.summary())),
    }
}

fn evaluate(source: &str) -> lkjscript_core::Result<OwnedValue> {
    let program = compile(source)?;
    returned(run_chunk(program.bytecode(), &ExecutionConfig::default()))
}

fn bool_main(expression: &str) -> String {
    format!("main/\nsig/\n->\nBool\n/sig\n{expression}\n/main\n")
}

fn assert_main_bool(expression: &str, expected: bool) {
    assert_eq!(
        evaluate(&bool_main(expression))
            .expect("evaluate Bool main")
            .as_bool(),
        Some(expected)
    );
}

#[test]
fn explicit_main_returns_its_exact_body_value() {
    let source = "main/\nsig/\n->\nI64\n/sig\n42\n/main\n";
    assert_eq!(evaluate(source).expect("evaluate main").as_i64(), Some(42));
    let program = compile(source).expect("compile explicit main");
    let chunk = program.bytecode();
    assert_eq!(chunk.main().name, "main");
    assert_eq!(chunk.main().arity, 0);
    assert!(chunk.main().code.ends_with(&[Op::Return as u8]));
    assert!(chunk.global_names().is_empty());
    assert_eq!(program.ssa().program().main.raw(), 0);
}

#[test]
fn local_var_set_and_shadowing_execute_through_ssa() {
    let source = "main/\nsig/\n->\nI64\n/sig\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\n1\ndo/\nset/\nx\n2\n/set\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\n+/\nx\n1\n/+\ndo/\nset/\nx\n+/\nx\n3\n/+\n/set\nx\n/do\n/var\n/do\n/var\n/main\n";
    let program = compile(source).expect("compile local mutation");
    assert!(program
        .bytecode()
        .main()
        .code
        .contains(&(Op::StoreLocal as u8)));
    assert_eq!(
        returned(run_chunk(program.bytecode(), &ExecutionConfig::default(),))
            .expect("run local mutation")
            .as_i64(),
        Some(6)
    );

    let set_unit = "main/\nsig/\n->\nUnit\n/sig\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\n0\nset/\nx\n1\n/set\n/var\n/main\n";
    assert!(evaluate(set_unit).expect("set returns Unit").is_unit());
}

#[test]
fn function_local_var_is_per_invocation_and_typed() {
    let source = "def/\nname/\nincrement\n/name\nfn/\nsig/\nI64\n->\nI64\n/sig\nparams/\nstart\nI64\n/params\nvar/\nname/\nvalue\n/name\ntype/\nI64\n/type\nstart\ndo/\nset/\nvalue\n+/\nvalue\n1\n/+\n/set\nvalue\n/do\n/var\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\n+/\nincrement/\n4\n/increment\nincrement/\n9\n/increment\n/+\n/main\n";
    assert_eq!(
        evaluate(source).expect("function-local mutation").as_i64(),
        Some(15)
    );
}

#[test]
fn immutable_parameter_function_global_and_cross_function_set_are_rejected() {
    let immutable =
        "main/\nsig/\n->\nUnit\n/sig\nlet/\nbind/\nx\n1\n/bind\nset/\nx\n2\n/set\n/let\n/main\n";
    assert!(compile(immutable).is_err());

    let parameter = "def/\nname/\nf\n/name\nfn/\nsig/\nI64\n->\nUnit\n/sig\nparams/\nx\nI64\n/params\nset/\nx\n2\n/set\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nf/\n1\n/f\n/main\n";
    assert!(compile(parameter).is_err());

    let function = "def/\nname/\nf\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nunit\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nset/\nf\nunit\n/set\n/main\n";
    assert!(compile(function).is_err());

    let source_global = "def/\nname/\nx\n/name\ntype/\nI64\n/type\n0\n/def\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    assert!(compile(source_global).is_err());

    let cross_function = "def/\nname/\nmutate\n/name\nfn/\nsig/\n->\nUnit\n/sig\nparams/\n/params\nset/\nstate\n1\n/set\n/fn\n/def\nmain/\nsig/\n->\nUnit\n/sig\nvar/\nname/\nstate\n/name\ntype/\nI64\n/type\n0\nmutate/\n/mutate\n/var\n/main\n";
    assert!(compile(cross_function).is_err());
}

#[test]
fn local_var_exact_type_and_initializer_scope_are_enforced() {
    let mismatch = "main/\nsig/\n->\nUnit\n/sig\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\n1.0\nunit\n/var\n/main\n";
    assert!(compile(mismatch).is_err());

    let set_mismatch = "main/\nsig/\n->\nUnit\n/sig\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\n1\nset/\nx\n1.0\n/set\n/var\n/main\n";
    assert!(compile(set_mismatch).is_err());

    let self_initializer =
        "main/\nsig/\n->\nI64\n/sig\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\nx\nx\n/var\n/main\n";
    assert!(compile(self_initializer).is_err());

    let outer_initializer = "main/\nsig/\n->\nI64\n/sig\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\n7\nvar/\nname/\nx\n/name\ntype/\nI64\n/type\nx\nx\n/var\n/var\n/main\n";
    assert_eq!(
        evaluate(outer_initializer)
            .expect("initializer sees outer binding")
            .as_i64(),
        Some(7)
    );
}

#[test]
fn immutable_product_state_threads_through_a_local_var() {
    let source = "product/\nname/\nCounterState\n/name\nfields/\nfield/\nname/\ncount\n/name\ntype/\nI64\n/type\n/field\n/fields\n/product\ndef/\nname/\nincrement-state\n/name\nfn/\nsig/\nProduct\nCounterState\n->\nProduct\nCounterState\n/sig\nparams/\nstate\nProduct/\nCounterState\n/Product\n/params\nwith-field/\nstate\ncount\n+/\nfield/\nstate\ncount\n/field\n1\n/+\n/with-field\n/fn\n/def\nmain/\nsig/\n->\nI64\n/sig\nvar/\nname/\nstate\n/name\ntype/\nProduct\nCounterState\n/type\nproduct-value/\nCounterState\nfield/\ncount\n0\n/field\n/product-value\ndo/\nset/\nstate\nincrement-state/\nstate\n/increment-state\n/set\nset/\nstate\nincrement-state/\nstate\n/increment-state\n/set\nfield/\nstate\ncount\n/field\n/do\n/var\n/main\n";
    assert_eq!(
        evaluate(source).expect("thread product state").as_i64(),
        Some(2)
    );
}

#[test]
fn option_arguments_equality_and_products_still_cross_compiler_vm_boundary() {
    let argument = "main/\nsig/\n->\nI64\n/sig\nstr-len/\nunwrap-some/\narg/\n0\n/arg\n/unwrap-some\n/str-len\n/main\n";
    let program = compile(argument).expect("compile argument main");
    assert_eq!(
        returned(run_chunk_with_args(
            program.bytecode(),
            &["hello".into()],
            &ExecutionConfig::default(),
        ))
        .expect("argument present")
        .as_i64(),
        Some(5)
    );

    assert_main_bool(
        "equal-value/\nsome/\n42\n/some\nsome/\n42\n/some\n/equal-value",
        true,
    );
    assert_main_bool(
        "list-equal/\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\ncons/\n1\nempty-list/\nI64\n/empty-list\n/cons\n/list-equal",
        true,
    );
    assert_main_bool("f64-bits-equal/\n0.0\n-0.0\n/f64-bits-equal", false);
}

#[test]
fn removed_top_level_effects_and_legacy_do_are_compile_errors() {
    assert!(compile("do/\nunit\n/do\n").is_err());
    let with_main = "do/\nprint/\nstr/\nside effect\n/str\n/print\n/do\nmain/\nsig/\n->\nUnit\n/sig\nunit\n/main\n";
    assert!(compile(with_main).is_err());
}
