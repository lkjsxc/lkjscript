use super::*;

fn owner_list_source(output: &str, operation: &str) -> String {
    format!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\n{output}\n/output\n/sig\n{operation}/\n\
         list-prepend/\nformat-i64/\n42\n/format-i64\n\
         empty-list/\nstring\n/empty-list\n/list-prepend\n/{operation}\n/main\n"
    )
}

#[test]
fn static_string_lists_execute_without_native_fallback() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n",
        "equal-list/\nlist-prepend/\nstring-literal/\nowner\n/string-literal\n",
        "empty-list/\nstring\n/empty-list\n/list-prepend\n",
        "list-prepend/\nstring-literal/\nowner\n/string-literal\n",
        "empty-list/\nstring\n/empty-list\n/list-prepend\n/equal-list\n/main\n",
    );
    let program = compile(source, "static-string-owner-list.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    assert_eq!(expected, Scalar::Bool(true));
    assert_eq!(vm, expected);
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline static string list"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof static string list"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.segmented_lists.prepends, 2);
    }
}

#[test]
fn option_owners_execute_in_all_four_tiers() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n",
        "equal-list/\nlist-prepend/\nsome/\n42\n/some\nempty-list/\n",
        "option/\ni64\n/option\n/empty-list\n/list-prepend\n",
        "list-prepend/\nsome/\n42\n/some\nempty-list/\n",
        "option/\ni64\n/option\n/empty-list\n/list-prepend\n/equal-list\n/main\n",
    );
    let program = compile(source, "structural-option-list.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    assert_eq!(expected, Scalar::Bool(true));
    assert_eq!(vm, expected);
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline option list"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof option list"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.segmented_lists.prepends, 2);
        assert_eq!(result.stats.native_structural.live_roots, 0);
    }
}

#[test]
fn structural_string_elements_are_owned_by_the_list_region() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\n",
        "equal-list/\nlist-prepend/\nformat-i64/\n42\n/format-i64\n",
        "empty-list/\nstring\n/empty-list\n/list-prepend\n",
        "list-prepend/\nformat-i64/\n42\n/format-i64\n",
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
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline dynamic string list"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof dynamic string list"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.segmented_lists.prepends, 2);
        assert_eq!(result.stats.native_structural.live_roots, 0);
    }
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
        Some(lkjscript_core::SemanticPayload::String(bytes)) if bytes == b"42"
    ));
    assert!(matches!(
        run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        ),
        lkjscript_core::ExecutionOutcome::Returned(_)
    ));
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline dynamic string list-first"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof dynamic string list-first"),
    ] {
        assert!(matches!(
            result.outcome,
            lkjscript_core::ExecutionOutcome::Returned(_)
        ));
        let lkjscript_core::ExecutionOutcome::Returned(value) = &result.outcome else {
            continue;
        };
        assert!(matches!(
            value.as_structural().map(|value| &value.payload),
            Some(lkjscript_core::SemanticPayload::String(bytes)) if bytes == b"42"
        ));
        assert_eq!(result.stats.vm_fallbacks, 0);
        assert_eq!(result.stats.segmented_lists.prepends, 1);
        assert_eq!(result.stats.native_structural.live_roots, 0);
    }
}
