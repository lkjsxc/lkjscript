use super::*;

#[test]
fn product_elements_execute_in_all_four_tiers() {
    let source = concat!(
        "product/\nname/\nboxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\n",
        "type/\ni64\n/type\n/field\n/fields\n/product\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nfield/\n",
        "list-first/\nlist-prepend/\nproduct-value/\nboxed\nfield/\nvalue\n42\n/field\n",
        "/product-value\nempty-list/\nproduct/\nboxed\n/product\n/empty-list\n",
        "/list-prepend\n/list-first\nvalue\n/field\n/main\n",
    );
    let program = compile(source, "structural-product-list.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ));
    assert_eq!(expected, Scalar::I64(42));
    assert_eq!(vm, expected);
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("baseline product list"),
        execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("proof product list"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.segmented_lists.prepends, 1);
    }
}

#[test]
fn nested_option_lists_execute_in_all_four_tiers() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nunwrap-some/\n",
        "list-first/\nlist-first/\nlist-prepend/\nlist-prepend/\nsome/\n42\n/some\n",
        "empty-list/\noption/\ni64\n/option\n/empty-list\n/list-prepend\n",
        "empty-list/\nlist/\noption/\ni64\n/option\n/list\n/empty-list\n",
        "/list-prepend\n/list-first\n/list-first\n/unwrap-some\n/main\n",
    );
    let program = compile(source, "nested-structural-option-list.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ));
    assert_eq!(expected, Scalar::I64(42));
    assert_eq!(vm, expected);
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("baseline nested option list"),
        execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("proof nested option list"),
    ] {
        assert_eq!(execution(result.outcome), expected);
        assert_eq!(result.stats.segmented_lists.prepends, 2);
    }
}

#[test]
fn nested_structural_list_equality_is_iterative_in_all_tiers() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nbool\n/output\n/sig\nlet/\nbind/\nitems\n",
        "list-prepend/\nlist-prepend/\nsome/\n42\n/some\nempty-list/\n",
        "option/\ni64\n/option\n/empty-list\n/list-prepend\n",
        "empty-list/\nlist/\noption/\ni64\n/option\n/list\n/empty-list\n/list-prepend\n",
        "/bind\nequal-list/\nitems\nitems\n/equal-list\n/let\n/main\n",
    );
    let program = compile(source, "nested-structural-list-equality.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::Bool(true));
    let vm_outcome = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    );
    assert_eq!(execution(vm_outcome), expected);
    for result in [
        execute_forced(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("baseline nested structural equality"),
        execute_optimizing(
            program.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("proof nested structural equality"),
    ] {
        assert_eq!(execution(result.outcome), expected);
    }
}
