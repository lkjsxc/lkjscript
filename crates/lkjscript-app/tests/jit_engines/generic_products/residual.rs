use super::*;

#[test]
fn sealed_residual_generic_executes_in_all_four_tiers() {
    let source = include_str!("../../fixtures/sealed-placement.lkjscript");
    let program = compile(source, "sealed-placement.lkjscript");
    let generic = &program.ssa().program().functions[0];
    assert_eq!(generic.signature.memory_witness_parameters.len(), 1);
    assert_eq!(
        generic.signature.memory_witness_parameters[0].operations,
        [
            lkjscript_contracts::MemoryWitnessOperation::Transport,
            lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
            lkjscript_contracts::MemoryWitnessOperation::Dispose,
        ]
    );
    assert!(generic
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| matches!(
            instruction.kind,
            InstructionKind::MemoryWitnessIndependentOwner { .. }
        )));
    assert!(generic
        .blocks
        .iter()
        .flat_map(|block| &block.instructions)
        .any(|instruction| matches!(
            instruction.kind,
            InstructionKind::MemoryWitnessDispose { .. }
        )));
    assert!(program
        .memory_plan()
        .value_placements
        .iter()
        .any(|placement| {
            placement.storage == lkjscript_compiler::MemoryDomain::SealedRegion
                && placement.independent_owner_demand
        }));
    assert!(program
        .ssa()
        .program()
        .memory
        .representations
        .iter()
        .any(|representation| {
            representation.storage == lkjscript_ir::StructuralStorage::SealedRegion
        }));
    assert!(program
        .bytecode()
        .structural_representations()
        .iter()
        .any(|representation| {
            representation.storage == lkjscript_core::StructuralStorage::SealedRegion
        }));
    let (native_program, specialization) = lkjscript_ir::specialize_native_transport(program.ssa())
        .expect("residual witness ABI keeps the generic body");
    assert_eq!(specialization.functions, 0);
    assert_eq!(
        native_program.program().functions.len(),
        program.ssa().program().functions.len()
    );
    assert!(!native_program.program().functions[0]
        .signature
        .type_parameters
        .is_empty());
    reject_invalid_hidden_witness_slots(program.ssa().program());

    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::I64(42));
    assert_eq!(
        execution(run_chunk(
            program.bytecode(),
            &lkjscript_vm::ExecutionInputs::default(),
            &ExecutionConfig::default(),
        )),
        expected
    );
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("residual generic enters baseline with hidden witness ABI");
    assert_eq!(
        execution(baseline.outcome.clone()),
        expected,
        "baseline outcome: {:?}",
        baseline.outcome
    );
    assert!(baseline.stats.native_entries > 0);
    assert!(baseline.stats.direct_native_calls > 0);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    assert_sealed_native_metrics(&baseline.stats.native_structural);
    let proof = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("residual generic enters proof tier with hidden witness ABI");
    assert_eq!(execution(proof.outcome), expected);
    assert!(proof.stats.optimizing_native_entries > 0);
    assert!(proof.stats.direct_native_calls > 0);
    assert_eq!(proof.stats.vm_fallbacks, 0);
    assert_sealed_native_metrics(&proof.stats.native_structural);
}

fn assert_sealed_native_metrics(metrics: &lkjscript_jit::NativeStructuralStats) {
    assert!(metrics.sealed_publications > 0);
    assert!(metrics.zero_copy_adoptions > 0 || metrics.copied_publication_bytes > 0);
    assert!(metrics.sealed_acquisitions > 0);
    assert!(metrics.sealed_releases > 0);
    assert!(metrics.sealed_release_work > 0);
    assert!(metrics.sealed_nodes_reclaimed > 0);
    assert_eq!(metrics.live_objects, 0);
    assert_eq!(metrics.live_sealed_domains, 0);
    assert_eq!(metrics.live_sealed_owners, 0);
    assert_eq!(metrics.live_roots, 0);
    assert_eq!(metrics.live_loans, 0);
    assert_eq!(metrics.live_views, 0);
    assert_eq!(metrics.live_destinations, 0);
    assert_eq!(metrics.release_backlog, 0);
}

fn reject_invalid_hidden_witness_slots(program: &lkjscript_ir::Program) {
    let mut dropped = program.clone();
    call_instantiation(&mut dropped).memory_witnesses.pop();
    assert!(lkjscript_ir::verify(dropped).is_err());

    let mut reordered = program.clone();
    call_instantiation(&mut reordered).memory_witnesses[0].parameter = "reordered".to_owned();
    assert!(lkjscript_ir::verify(reordered).is_err());

    let mut forged = program.clone();
    let current = call_instantiation(&mut forged).memory_witnesses[0].witness;
    let replacement = forged
        .memory
        .witnesses
        .iter()
        .map(|witness| witness.id)
        .find(|witness| *witness != current)
        .expect("fixture installs a distinct forged witness candidate");
    call_instantiation(&mut forged).memory_witnesses[0].witness = replacement;
    assert!(lkjscript_ir::verify(forged).is_err());
}

fn call_instantiation(program: &mut lkjscript_ir::Program) -> &mut lkjscript_ir::GenericInstantiation {
    program
        .functions
        .iter_mut()
        .flat_map(|function| &mut function.blocks)
        .flat_map(|block| &mut block.instructions)
        .find_map(|instruction| match &mut instruction.kind {
            lkjscript_ir::InstructionKind::Call {
                instantiation: Some(instantiation),
                ..
            } if instantiation.memory_witnesses.len() == 1 => Some(instantiation),
            _ => None,
        })
        .expect("fixture direct call carries one ordered hidden witness")
}
