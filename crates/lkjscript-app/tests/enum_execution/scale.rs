use super::*;

fn structural_scale_source() -> String {
    concat!(
        "enum/\nname/\ntree\n/name\nvariants/\n",
        "variant/\nname/\nleaf\n/name\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/variant-field\n",
        "/fields\n/variant\nvariant/\nname/\nbranch\n/name\nfields/\n",
        "variant-field/\nname/\nleft\n/name\ntype/\ntree/\n/tree\n/type\n/variant-field\n",
        "variant-field/\nname/\nright\n/name\ntype/\ntree/\n/tree\n/type\n/variant-field\n",
        "/fields\n/variant\n/variants\n/enum\n",
        "def/\nname/\nbuild-tree\n/name\nfn/\nsig/\ninputs/\ni64\n/inputs\n",
        "output/\ntree/\n/tree\n/output\n/sig\nparams/\ndepth\ni64\n/params\nif/\n",
        "equal-value/\ndepth\n0\n/equal-value\n",
        "variant-value/\ntype/\ntree/\n/tree\n/type\nvariant/\nleaf\n/variant\nfields/\n",
        "variant-field/\nname/\nvalue\n/name\n0\n/variant-field\n/fields\n/variant-value\n",
        "variant-value/\ntype/\ntree/\n/tree\n/type\nvariant/\nbranch\n/variant\nfields/\n",
        "variant-field/\nname/\nleft\n/name\nbuild-tree/\nsubtract/\ndepth\n1\n/subtract\n",
        "/build-tree\n/variant-field\nvariant-field/\nname/\nright\n/name\nbuild-tree/\n",
        "subtract/\ndepth\n1\n/subtract\n/build-tree\n/variant-field\n",
        "/fields\n/variant-value\n/if\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\n",
        "bind/\nvalue\nbuild-tree/\n14\n/build-tree\n/bind\n7\n/let\n/main\n",
    )
    .into()
}

fn borrowed_string_call_source() -> &'static str {
    concat!(
        "def/\nname/\nstring-size\n/name\nfn/\nsig/\ninputs/\nstring\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\nvalue\nstring\n/params\n",
        "string-byte-length/\nvalue\n/string-byte-length\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nlet/\n",
        "bind/\ncaller-value\nstring-literal/\nabc\n/string-literal\n/bind\n",
        "string-size/\ncaller-value\n/string-size\n/let\n/main\n",
    )
}

#[test]
fn immutable_direct_call_has_no_prebackend_copy_and_all_tiers_agree() {
    let program = compile_source(
        borrowed_string_call_source(),
        "borrowed-string-call.lkjscript",
    )
    .expect("compile borrowed string call");
    let copies = program
        .ssa()
        .program()
        .functions
        .iter()
        .flat_map(|function| &function.blocks)
        .flat_map(|block| &block.instructions)
        .filter(|instruction| {
            matches!(
                instruction.kind,
                lkjscript_ir::InstructionKind::StructuralCopy { .. }
            )
        })
        .count();
    assert_eq!(copies, 0);
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(3))
    );
    let ExecutionOutcome::Returned(value) = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ) else {
        panic!("VM must borrow the string call argument")
    };
    assert_eq!(value.as_i64(), Some(3));
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline executes the string call"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof executes the string call"),
    ] {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(ref value) if value.as_i64() == Some(3)
        ));
        assert_eq!(execution.stats.vm_fallbacks, 0);
        assert_eq!(execution.stats.native_structural.live_roots, 0);
        assert_eq!(execution.stats.native_structural.release_backlog, 0);
    }
}

#[test]
fn thirty_two_thousand_node_value_builds_and_releases_on_all_tiers() {
    let program = compile_source(&structural_scale_source(), "structural-scale.lkjscript")
        .expect("compile structural scale source");
    assert_eq!(
        evaluate(program.ssa(), &EvalConfig::default()),
        EvalOutcome::Returned(EvalValue::I64(7))
    );
    let ExecutionOutcome::Returned(value) = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ) else {
        panic!("VM must build and release the structural tree")
    };
    assert_eq!(value.as_i64(), Some(7));
    for execution in [
        execute_forced(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("baseline builds structural scale tree"),
        execute_optimizing(
            program.ssa(),
            &ExecutionConfig::default(),
            JitConfig::default(),
        )
        .expect("proof builds structural scale tree"),
    ] {
        assert!(matches!(
            execution.outcome,
            ExecutionOutcome::Returned(ref value) if value.as_i64() == Some(7)
        ));
        assert!(execution.stats.native_entries > 0);
        assert_eq!(execution.stats.vm_fallbacks, 0);
        assert_eq!(execution.stats.native_structural.live_roots, 0);
        assert_eq!(execution.stats.native_structural.live_loans, 0);
        assert_eq!(execution.stats.native_structural.live_destinations, 0);
        assert_eq!(execution.stats.native_structural.release_backlog, 0);
        assert_eq!(execution.stats.native_structural.teardown_failures, 0);
    }
}
