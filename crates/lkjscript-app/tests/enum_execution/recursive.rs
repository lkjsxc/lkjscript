use super::*;

fn tree_source() -> &'static str {
    concat!(
        "enum/\nname/\ntree\n/name\nforall/\nt\n/forall\nvariants/\n",
        "variant/\nname/\nleaf\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\nt\n/type\n/variant-field\n/fields\n/variant\n",
        "variant/\nname/\nbranch\n/name\nfields/\n",
        "variant-field/\nname/\nleft\n/name\ntype/\ntree/\nt\n/tree\n/type\n/variant-field\n",
        "variant-field/\nname/\nright\n/name\ntype/\ntree/\nt\n/tree\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ntree/\ni64\n/tree\n/output\n/sig\n",
        "variant-value/\ntype/\ntree/\ni64\n/tree\n/type\nvariant/\nbranch\n/variant\nfields/\n",
        "variant-field/\nname/\nleft\n/name\nvariant-value/\ntype/\ntree/\ni64\n/tree\n/type\n",
        "variant/\nleaf\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n1\n",
        "/variant-field\n/fields\n/variant-value\n/variant-field\n",
        "variant-field/\nname/\nright\n/name\nvariant-value/\ntype/\ntree/\ni64\n/tree\n/type\n",
        "variant/\nleaf\n/variant\nfields/\nvariant-field/\nname/\nvalue\n/name\n2\n",
        "/variant-field\n/fields\n/variant-value\n/variant-field\n",
        "/fields\n/variant-value\n/main\n",
    )
}

fn match_source() -> &'static str {
    concat!(
        "enum/\nname/\nchain\n/name\nvariants/\n",
        "variant/\nname/\nleaf\n/name\nfields/\nvariant-field/\nname/\nvalue\n/name\n",
        "type/\ni64\n/type\n/variant-field\n/fields\n/variant\n",
        "variant/\nname/\nbranch\n/name\nfields/\nvariant-field/\nname/\nnext\n/name\n",
        "type/\nchain/\n/chain\n/type\n/variant-field\n/fields\n/variant\n/variants\n/enum\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nmatch/\n",
        "variant-value/\ntype/\nchain/\n/chain\n/type\nvariant/\nbranch\n/variant\n",
        "fields/\nvariant-field/\nname/\nnext\n/name\n",
        "variant-value/\ntype/\nchain/\n/chain\n/type\nvariant/\nleaf\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n41\n/variant-field\n/fields\n/variant-value\n",
        "/variant-field\n/fields\n/variant-value\narms/\n",
        "arm/\nvariant-pattern/\ntype/\nchain/\n/chain\n/type\nvariant/\nleaf\n/variant\n",
        "fields/\nvariant-field-pattern/\nname/\nvalue\n/name\nwildcard/\n/wildcard\n",
        "/variant-field-pattern\n/fields\n/variant-pattern\n0\n/arm\n",
        "arm/\nvariant-pattern/\ntype/\nchain/\n/chain\n/type\nvariant/\nbranch\n/variant\n",
        "fields/\nvariant-field-pattern/\nname/\nnext\n/name\nwildcard/\n/wildcard\n",
        "/variant-field-pattern\n/fields\n/variant-pattern\n42\n/arm\n",
        "/arms\n/match\n/main\n",
    )
}

#[test]
fn finite_generic_recursive_tree_is_structural_on_evaluator_vm_and_native_tiers() {
    let compiled = compile_source(tree_source(), "recursive-tree.lkjscript")
        .expect("compile finite recursive tree");
    let evaluated = evaluator_owned(&compiled);
    assert_eq!(evaluated.snapshot_object_count(), 5);
    let ExecutionOutcome::Returned(vm) = run_chunk(
        compiled.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) else {
        panic!("VM returns recursive tree")
    };
    assert_eq!(vm.snapshot_object_count(), 5);
    for execution in [
        execute_forced(
            compiled.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("baseline returns recursive tree"),
        execute_optimizing(
            compiled.ssa(),
            &ExecutionPolicy::unrestricted(),
            JitConfig::default(),
        )
        .expect("proof returns recursive tree"),
    ] {
        let ExecutionOutcome::Returned(value) = execution.outcome else {
            panic!("native helper returns recursive tree")
        };
        assert_eq!(value.snapshot_object_count(), 5);
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.runtime_heap_successes, 0);
        assert_eq!(execution.stats.native_structural.live_roots, 0);
        assert_eq!(execution.stats.native_structural.live_destinations, 0);
    }
}

#[test]
fn recursive_branch_match_runs_on_evaluator_and_vm() {
    let compiled = compile_source(match_source(), "recursive-tree-match.lkjscript")
        .expect("compile recursive branch match");
    assert_eq!(
        evaluate(compiled.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(42)),
    );
    let ExecutionOutcome::Returned(vm) = run_chunk(
        compiled.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionPolicy::unrestricted(),
    ) else {
        panic!("VM returns recursive field projection")
    };
    assert_eq!(vm.as_i64(), Some(42));
}
