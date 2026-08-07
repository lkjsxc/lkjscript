use super::*;

#[test]
fn multiple_dynamic_owner_parameters_lower_and_execute_in_source_order() {
    let source = concat!(
        "def/\nname/\nconsume\n/name\nfn/\nforall/\nt\nu\n/forall\nsig/\ninputs/\n",
        "t\nt\nu\nu\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\n",
        "first-t\nt\nsecond-t\nt\nfirst-u\nu\nsecond-u\nu\n/params\ndo/\n",
        "first-t\nsecond-t\nfirst-u\nsecond-u\n42\n/do\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "consume/\nstring-literal/\na\n/string-literal\nstring-literal/\nb\n/string-literal\n",
        "string-literal/\nc\n/string-literal\nstring-literal/\nd\n/string-literal\n",
        "/consume\n/main\n",
    );
    let program = compile(source, "multiple-dynamic-owners.lkjscript");

    let hir_signature = &program.memory_plan().functions[0].signature;
    assert_eq!(hir_signature.witness_parameters.len(), 2);
    assert_eq!(hir_signature.witness_parameters[0].parameter, "t");
    assert_eq!(hir_signature.witness_parameters[1].parameter, "u");
    assert!(hir_signature.witness_parameters.iter().all(|requirement| {
        requirement.operations.contains(
            &lkjscript_compiler::memory_plan::MemoryWitnessOperation::IndependentOwner,
        ) && requirement
            .operations
            .contains(&lkjscript_compiler::memory_plan::MemoryWitnessOperation::Dispose)
    }));

    let generic = &program.ssa().program().functions[0];
    let independent_parameters: Vec<_> = generic
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .filter_map(|instruction| match &instruction.kind {
            InstructionKind::MemoryWitnessIndependentOwner { parameter, .. } => {
                Some(parameter.as_str())
            }
            _ => None,
        })
        .collect();
    assert_eq!(independent_parameters, ["t", "t", "u", "u"]);

    let outcome = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    assert_eq!(
        execution(outcome.clone()),
        Scalar::I64(42),
        "VM outcome: {outcome:?}"
    );
}
