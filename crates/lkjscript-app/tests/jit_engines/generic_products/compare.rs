use super::*;

#[test]
fn residual_compare_executes_in_all_four_tiers() {
    let source = include_str!("../../fixtures/residual-compare.lkjscript");
    let program = compile(source, "residual-compare.lkjscript");
    let generic = &program.ssa().program().functions[0];
    assert_eq!(generic.signature.memory_witness_parameters.len(), 1);
    assert_eq!(
        generic.signature.memory_witness_parameters[0].operations,
        [
            lkjscript_contracts::MemoryWitnessOperation::Transport,
            lkjscript_contracts::MemoryWitnessOperation::Compare,
        ]
    );
    assert!(generic
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| matches!(
            instruction.kind,
            InstructionKind::MemoryWitnessCompare { .. }
        )));
    assert!(!generic
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| matches!(
            instruction.kind,
            InstructionKind::MemoryWitnessIndependentOwner { .. }
                | InstructionKind::MemoryWitnessDispose { .. }
        )));
    let prototype = &program.bytecode().protos()[0];
    assert_eq!(prototype.memory_witness_parameters.len(), 1);
    assert!(prototype.code.contains(&(lkjscript_core::Op::MemoryWitnessCompare as u8)));

    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::I64(41));
    assert_eq!(
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionPolicy::unrestricted(),
        )),
        expected
    );
    let mut malformed_requirement = program.ssa().program().clone();
    malformed_requirement.functions[0].signature.memory_witness_parameters[0]
        .operations
        .retain(|operation| *operation != lkjscript_contracts::MemoryWitnessOperation::Compare);
    assert!(lkjscript_ir::verify(malformed_requirement).is_err());

    let mut malformed_witness = program.ssa().program().clone();
    let witness = malformed_witness
        .memory
        .witnesses
        .iter_mut()
        .find(|witness| {
            witness
                .facts
                .operations
                .contains(&lkjscript_contracts::MemoryWitnessOperation::Compare)
        })
        .expect("fixture installs a compare witness");
    witness
        .facts
        .operations
        .retain(|operation| *operation != lkjscript_contracts::MemoryWitnessOperation::Compare);
    assert!(lkjscript_ir::verify(malformed_witness).is_err());

    if cfg!(miri) {
        return;
    }
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("residual compare enters baseline native code");
    assert_eq!(execution(baseline.outcome), expected);
    assert!(baseline.stats.native_entries > 0);
    assert!(baseline.stats.direct_native_calls > 0);
    assert_eq!(baseline.stats.vm_fallbacks, 0);

    let proof = execute_optimizing(
        program.ssa(),
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("residual compare enters proof native code");
    assert_eq!(execution(proof.outcome), expected);
    assert!(proof.stats.optimizing_native_entries > 0);
    assert!(proof.stats.direct_native_calls > 0);
    assert_eq!(proof.stats.vm_fallbacks, 0);
}
