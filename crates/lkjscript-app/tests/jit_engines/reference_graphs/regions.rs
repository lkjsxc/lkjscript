use super::*;

#[test]
fn nested_region_product_and_list_graph_is_collector_free_in_all_engines() {
    let source = include_str!("../../fixtures/allocation-graph.lkjscript");
    let program = compile(source, "allocation-graph.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    let config = JitConfig::default();
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("nested graph executes through generated runtime-value sites");
    let optimized = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect("nested graph executes through optimized runtime-value sites");
    assert_eq!(expected, Scalar::I64(42));
    assert_eq!(vm, expected);
    assert_eq!(execution(native.outcome), expected);
    assert_eq!(execution(optimized.outcome), expected);
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(optimized.stats.optimizing_native_entries > 0);
    assert_eq!(
        optimized.stats.runtime_heap_attempts,
        native.stats.runtime_heap_attempts
    );
    assert_eq!(
        optimized.stats.runtime_heap_successes,
        native.stats.runtime_heap_successes
    );
    assert_eq!(native.stats.region_products.records, 4);
    assert_eq!(native.stats.region_products.fields, 6);
    assert!(native.stats.segmented_lists.live_entries >= 2);
    assert!(native.stats.segmented_lists.segment_allocations > 0);
    assert!(
        native.stats.segmented_lists.segment_allocations < native.stats.segmented_lists.prepends
    );
    assert!(native.stats.runtime_heap_attempts >= 6);
    assert_eq!(
        native.stats.runtime_heap_attempts,
        native.stats.runtime_heap_successes
    );
}
