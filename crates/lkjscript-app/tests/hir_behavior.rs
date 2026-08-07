#![allow(clippy::expect_used)]

use lkjscript_compiler::{compile_source, ExecutableProgram};
use lkjscript_core::{Error, ExecutionOutcome, ExecutionPolicy, Op, OwnedValue};
use lkjscript_vm::run_chunk;

fn compile(source: &str) -> lkjscript_core::Result<ExecutableProgram> {
    let source = source.to_string();
    compile_source(&source, "hir-behavior.lkjscript")
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
    returned(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ))
}

fn bool_main(expression: &str) -> String {
    format!("main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n{expression}\n/main\n")
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
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n42\n/main\n";
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
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n1\ndo/\nset/\nx\n2\n/set\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\nadd/\nx\n1\n/add\ndo/\nset/\nx\nadd/\nx\n3\n/add\n/set\nx\n/do\n/var\n/do\n/var\n/main\n";
    let program = compile(source).expect("compile local mutation");
    assert!(program
        .bytecode()
        .main()
        .code
        .contains(&(Op::StoreLocal as u8)));
    assert_eq!(
        returned(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        ))
        .expect("run local mutation")
        .as_i64(),
        Some(6)
    );

    let set_unit = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n0\nset/\nx\n1\n/set\n/var\n/main\n";
    assert!(evaluate(set_unit).expect("set returns Unit").is_unit());
}

#[test]
fn function_local_var_is_per_invocation_and_typed() {
    let source = "def/\nname/\nincrement\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\nstart\ni64\n/params\nvar/\nname/\nvalue\n/name\ntype/\ni64\n/type\nstart\ndo/\nset/\nvalue\nadd/\nvalue\n1\n/add\n/set\nvalue\n/do\n/var\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nadd/\nincrement/\n4\n/increment\nincrement/\n9\n/increment\n/add\n/main\n";
    assert_eq!(
        evaluate(source).expect("function-local mutation").as_i64(),
        Some(15)
    );
}

#[test]
fn immutable_parameter_function_global_and_cross_function_set_are_rejected() {
    let immutable =
        "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nlet/\nbind/\nx\n1\n/bind\nset/\nx\n2\n/set\n/let\n/main\n";
    assert!(compile(immutable).is_err());

    let parameter = "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\nx\ni64\n/params\nset/\nx\n2\n/set\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nf/\n1\n/f\n/main\n";
    assert!(compile(parameter).is_err());

    let function = "def/\nname/\nf\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\nunit\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nset/\nf\nunit\n/set\n/main\n";
    assert!(compile(function).is_err());

    let source_global = "def/\nname/\nx\n/name\ntype/\ni64\n/type\n0\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    assert!(compile(source_global).is_err());

    let cross_function = "def/\nname/\nmutate\n/name\nfn/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nparams/\n/params\nset/\nstate\n1\n/set\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nvar/\nname/\nstate\n/name\ntype/\ni64\n/type\n0\nmutate/\n/mutate\n/var\n/main\n";
    assert!(compile(cross_function).is_err());
}

#[test]
fn local_var_exact_type_and_initializer_scope_are_enforced() {
    let mismatch = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n1.0\nunit\n/var\n/main\n";
    assert!(compile(mismatch).is_err());

    let set_mismatch = "main/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n1\nset/\nx\n1.0\n/set\n/var\n/main\n";
    assert!(compile(set_mismatch).is_err());

    let self_initializer =
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\nx\nx\n/var\n/main\n";
    assert!(compile(self_initializer).is_err());

    let outer_initializer = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\n7\nvar/\nname/\nx\n/name\ntype/\ni64\n/type\nx\nx\n/var\n/var\n/main\n";
    assert_eq!(
        evaluate(outer_initializer)
            .expect("initializer sees outer binding")
            .as_i64(),
        Some(7)
    );
}

#[test]
fn immutable_product_state_threads_through_a_local_var() {
    let source = "product/\nname/\ncounter-state\n/name\nfields/\nfield/\nname/\ncount\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\ndef/\nname/\nincrement-state\n/name\nfn/\nsig/\ninputs/\nproduct/\ncounter-state\n/product\n/inputs\noutput/\nproduct/\ncounter-state\n/product\n/output\n/sig\nparams/\nstate\nproduct/\ncounter-state\n/product\n/params\nwith-field/\nstate\ncount\nadd/\nfield/\nstate\ncount\n/field\n1\n/add\n/with-field\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nstate\n/name\ntype/\nproduct\ncounter-state\n/type\nproduct-value/\ncounter-state\nfield/\ncount\n0\n/field\n/product-value\ndo/\nset/\nstate\nincrement-state/\nstate\n/increment-state\n/set\nset/\nstate\nincrement-state/\nstate\n/increment-state\n/set\nfield/\nstate\ncount\n/field\n/do\n/var\n/main\n";
    assert_eq!(
        evaluate(source).expect("thread product state").as_i64(),
        Some(2)
    );
}

#[test]
fn structural_string_loop_locals_teardown_every_owner() {
    let source = "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nvar/\nname/\nn\n/name\ntype/\ni64\n/type\n0\ndo/\nwhile/\nless-than/\nn\n3\n/less-than\nlet/\nbind/\ntext\nformat-i64/\nn\n/format-i64\n/bind\ndo/\nstring-byte-length/\ntext\n/string-byte-length\nset/\nn\nadd/\nn\n1\n/add\n/set\n/do\n/let\n/while\nn\n/do\n/var\n/main\n";
    let program = compile(source).expect("compile structural string loop");
    assert!(program
        .bytecode()
        .main()
        .code
        .contains(&(Op::StoreStructuralLocal as u8)));
    assert_eq!(
        evaluate(source)
            .expect("run structural string loop")
            .as_i64(),
        Some(3)
    );
}

#[test]
fn option_arguments_equality_and_products_still_cross_compiler_vm_boundary() {
    let argument = concat!(
        "main/\nsig/\ninputs/\ncapability/\narguments\n/capability\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "params/\narguments\ncapability/\narguments\n/capability\n/params\n",
        "string-byte-length/\nunwrap-some/\nargument-at/\narguments\n0\n/argument-at\n/unwrap-some\n/string-byte-length\n/main\n"
    );
    let program = compile(argument).expect("compile argument main");
    assert_eq!(
        returned(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs {
                arguments: vec!["hello".into()],
                capabilities: vec![lkjscript_core::CapabilityKind::Arguments],
                host: lkjscript_host::HostEnvironment::default(),
            },
            &ExecutionPolicy::unrestricted(),
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
        "equal-list/\nlist-prepend/\n1\nempty-list/\ni64\n/empty-list\n/list-prepend\nlist-prepend/\n1\nempty-list/\ni64\n/empty-list\n/list-prepend\n/equal-list",
        true,
    );
    assert_main_bool("equal-f64-bits/\n0.0\n-0.0\n/equal-f64-bits", false);
}

#[test]
fn removed_top_level_effects_and_legacy_do_are_compile_errors() {
    assert!(compile("do/\nunit\n/do\n").is_err());
    let with_main = "do/\nprint/\nstring-literal/\nside effect\n/string-literal\n/print\n/do\nmain/\nsig/\ninputs/\n/inputs\noutput/\nunit\n/output\n/sig\nunit\n/main\n";
    assert!(compile(with_main).is_err());
}
