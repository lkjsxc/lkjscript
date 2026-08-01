use lkjscript_core::ExecutionConfig;
use lkjscript_ir::{evaluate, EvalConfig};
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
    let program = lkjscript_compiler::compile_path(&entry, &lkjscript_core::Limits::default())
        .expect("compile locked nested history workload");
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
