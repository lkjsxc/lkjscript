use super::*;

fn owner_list_source(output: &str, operation: &str) -> String {
    format!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\n{output}\n/output\n/sig\n{operation}/\n\
         list-prepend/\nappend-string/\nstring-literal/\nval\n/string-literal\n\
         string-literal/\nue\n/string-literal\n/append-string\n\
         empty-list/\nstring\n/empty-list\n/list-prepend\n/{operation}\n/main\n"
    )
}

#[test]
fn structural_string_elements_are_owned_by_the_list_region() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n",
        "equal-list/\nlist-prepend/\nappend-string/\nstring-literal/\nval\n/string-literal\n",
        "string-literal/\nue\n/string-literal\n/append-string\n",
        "empty-list/\nstring\n/empty-list\n/list-prepend\n",
        "list-prepend/\nappend-string/\nstring-literal/\nvalue\n/string-literal\n",
        "string-literal/\n\n/string-literal\n/append-string\n",
        "empty-list/\nstring\n/empty-list\n/list-prepend\n/equal-list\n/main\n",
    );
    let program = compile(source, "structural-string-list.lkjscript");
    let evaluated = evaluate(program.ssa(), &EvalConfig::default());
    assert!(
        !matches!(evaluated, lkjscript_ir::EvalOutcome::Trapped(_)),
        "{evaluated:?}"
    );
    let expected = evaluator(evaluated);
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    assert_eq!(expected, Scalar::Bool(true));
    assert_eq!(vm, expected);
}

#[test]
fn list_first_clones_a_dynamic_structural_owner() {
    let source = owner_list_source("string", "list-first");
    let program = compile(&source, "structural-string-list-first.lkjscript");
    let evaluated = evaluate(program.ssa(), &EvalConfig::default());
    assert!(matches!(
        evaluated,
        lkjscript_ir::EvalOutcome::Returned(lkjscript_ir::EvalValue::ReturnedOwned(_))
    ));
    let lkjscript_ir::EvalOutcome::Returned(lkjscript_ir::EvalValue::ReturnedOwned(value)) =
        evaluated
    else {
        return;
    };
    assert!(matches!(
        value.as_structural().map(|value| &value.payload),
        Some(lkjscript_core::SemanticPayload::String(bytes)) if bytes == b"value"
    ));
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ),
        lkjscript_core::ExecutionOutcome::Returned(_)
    ));
}
