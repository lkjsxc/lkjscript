use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::ExecutionConfig;
use lkjscript_ir::{evaluate, EvalConfig, InstructionKind};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn locked_generic_history_transports_4096_nested_lists_in_all_tiers() {
    std::thread::Builder::new()
        .name("generic-history".into())
        .stack_size(64 * 1024 * 1024)
        .spawn(run_workload)
        .expect("spawn bounded deep-history stack")
        .join()
        .expect("generic history workload");
}

fn run_workload() {
    let entry = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../src/examples/polymorphic-transport/history-workload.lkjscript");
    let program =
        lkjscript_compiler::compile_path(&entry).expect("compile locked nested history workload");
    let mut eval_config = EvalConfig::default();
    eval_config.max_frames = 8_192;
    let evaluated = evaluate(program.ssa(), &eval_config);
    assert_eq!(
        evaluated,
        lkjscript_ir::EvalOutcome::Returned(lkjscript_ir::EvalValue::I64(8_390_656))
    );

    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("baseline nested generic history");
    assert!(baseline.stats.direct_native_calls > 0);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    assert!(baseline.stats.segmented_lists.prepends > 0);
    let proof = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("proof nested generic history");
    assert!(proof.stats.optimizing_native_entries > 0);
    assert_eq!(proof.stats.vm_fallbacks, 0);
    assert!(proof.stats.segmented_lists.prepends > 0);
    for outcome in [vm, baseline.outcome, proof.outcome] {
        assert_history(outcome);
    }
}

fn assert_history(outcome: lkjscript_core::ExecutionOutcome) {
    let value = outcome.returned().expect("history execution returns");
    assert_eq!(value.as_i64(), Some(8_390_656));
    let wire = lkjscript_core::encode_execution_outcome(&outcome, 1_000_000)
        .expect("encode nested generic history");
    assert_eq!(
        lkjscript_core::decode_execution_outcome(&wire, 1_000_000)
            .expect("decode nested generic history"),
        outcome
    );
}

#[test]
fn multiple_transport_instances_are_canonical_deduplicated_and_native() {
    let source = concat!(
        "product/\nname/\nboxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "def/\nname/\nkeep\n/name\nfn/\nforall/\nt\n/forall\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n",
        "keep/\nproduct-value/\nboxed\nfield/\nvalue\n11\n/field\n/product-value\n/keep\n",
        "keep/\n42\n/keep\nkeep/\n7\n/keep\n/do\n/main\n",
    );
    let program = compile(source, "multiple-generic-instances.lkjscript");
    let (specialized, stats) = lkjscript_ir::specialize_native_transport(program.ssa())
        .expect("specialize two exact transport instances");
    assert_eq!(stats.functions, 2);
    assert_eq!(stats.calls, 3);

    let functions = &specialized.program().functions;
    assert_eq!(functions.len(), program.ssa().program().functions.len() + 1);
    assert_eq!(
        functions[0].signature.parameters,
        [lkjscript_ir::SsaType::I64]
    );
    assert_eq!(
        functions[2].signature.parameters,
        [lkjscript_ir::SsaType::Product(
            lkjscript_ir::ProductId::new(0)
        )]
    );
    let targets = functions[1].blocks[0]
        .instructions
        .iter()
        .filter_map(|instruction| match &instruction.kind {
            InstructionKind::Call {
                target: lkjscript_ir::CallTarget::Direct(target),
                instantiation,
                ..
            } => {
                assert!(instantiation.is_none());
                Some(*target)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(targets.len(), 3);
    assert_eq!(targets[0], lkjscript_ir::FunctionId::new(2));
    assert_eq!(targets[1], lkjscript_ir::FunctionId::new(0));
    assert_eq!(targets[2], targets[1]);

    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    assert_eq!(expected, Scalar::I64(7));
    assert_eq!(
        evaluator(evaluate(&specialized, &EvalConfig::default())),
        expected
    );
    let config = JitConfig::default();
    let baseline = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("multiple generic instances enter baseline native code");
    assert_eq!(execution(baseline.outcome), expected);
    assert!(baseline.stats.direct_native_calls >= 3);
    assert_eq!(baseline.stats.vm_fallbacks, 0);
    let proof = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect("multiple generic instances enter proof native code");
    assert_eq!(execution(proof.outcome), expected);
    assert!(proof.stats.direct_native_calls >= 3);
    assert!(proof.stats.optimizing_native_entries > 0);
    assert_eq!(proof.stats.vm_fallbacks, 0);
}

#[test]
fn transport_specialization_rejects_witness_mismatch() {
    let source = concat!(
        "product/\nname/\nboxed\n/name\nfields/\nfield/\nname/\nvalue\n/name\ntype/\ni64\n/type\n/field\n/fields\n/product\n",
        "def/\nname/\nkeep\n/name\nfn/\nforall/\nt\n/forall\nsig/\ninputs/\nt\n/inputs\noutput/\nt\n/output\n/sig\n",
        "params/\nvalue\nt\n/params\nvalue\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\ndo/\n",
        "keep/\nproduct-value/\nboxed\nfield/\nvalue\n11\n/field\n/product-value\n/keep\nkeep/\n7\n/keep\n/do\n/main\n",
    );
    let valid = compile(source, "generic-witness-mismatch.lkjscript");
    let mut forged = valid.ssa().program().clone();
    let calls = forged.functions[1].blocks[0]
        .instructions
        .iter()
        .filter_map(|instruction| match &instruction.kind {
            InstructionKind::Call {
                instantiation: Some(instantiation),
                ..
            } => Some(instantiation.memory_witnesses[0].witness),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(calls.len(), 2);
    let first = forged.functions[1].blocks[0]
        .instructions
        .iter_mut()
        .find_map(|instruction| match &mut instruction.kind {
            InstructionKind::Call {
                instantiation: Some(instantiation),
                ..
            } => Some(instantiation),
            _ => None,
        })
        .expect("first generic call");
    first.memory_witnesses[0].witness = calls[1];
    let error = lkjscript_ir::verify(forged).expect_err("mismatched exact witness must reject");
    assert!(error
        .to_string()
        .contains("SSA generic call memory witness does not match type or operations"));
}
