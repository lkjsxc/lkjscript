use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::ExecutionConfig;
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn nested_product_and_list_graph_matches_all_engines() {
    let source = include_str!("../fixtures/allocation-graph.lkjscript");
    let program = compile(source, "allocation-graph.lkjscript");
    let expected = evaluator(evaluate(program.ssa(), &EvalConfig::default()));
    let vm = execution(run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    ));
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("nested graph executes through generated heap sites");
    let optimized = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect("nested graph executes through optimized generated heap sites");
    assert_eq!(expected, Scalar::I64(0));
    assert_eq!(vm, expected);
    assert_eq!(execution(native.outcome), expected);
    assert_eq!(execution(optimized.outcome), expected);
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(optimized.stats.optimizing_native_entries > 0);
    assert_eq!(optimized.stats.allocations, native.stats.allocations);
    assert_eq!(
        optimized.stats.allocation_bytes_estimate,
        native.stats.allocation_bytes_estimate
    );
    assert_eq!(optimized.stats.collections, native.stats.collections);
    assert_eq!(
        optimized.stats.runtime_heap_attempts,
        native.stats.runtime_heap_attempts
    );
    assert_eq!(
        optimized.stats.runtime_heap_successes,
        native.stats.runtime_heap_successes
    );
    assert!(native.stats.allocations >= 3);
    assert!(native.stats.collections >= 3);
    assert!(native.stats.maximum_roots > 0);
    assert!(native.stats.runtime_heap_attempts >= 6);
    assert_eq!(
        native.stats.runtime_heap_attempts,
        native.stats.runtime_heap_successes
    );
    assert!(native.stats.barrier_count >= 2);
}

#[test]
fn structural_string_length_uses_generated_calls_without_collector_metadata() {
    let source = concat!(
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "string-byte-length/\nempty-string/\n/empty-string\n/string-byte-length\n/main\n",
    );
    let program = compile(source, "structural-string-length.lkjscript");
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let native = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("generated structural string execution");
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("optimizing structural string execution");
    assert_eq!(execution(native.outcome), execution(vm.clone()));
    assert_eq!(execution(optimized.outcome), execution(vm));
    for stats in [&native.stats, &optimized.stats] {
        assert_eq!(stats.vm_fallbacks, 0);
        assert_eq!(stats.collections, 0);
        assert_eq!(stats.maximum_roots, 0);
        assert_eq!(stats.runtime_heap_successes, 0);
        assert_eq!(stats.collector_runtime_invocations, 0);
        assert!(stats.structural_runtime_calls > 0);
        assert_eq!(stats.native_structural.live_roots, 0);
        assert_eq!(stats.native_structural.live_loans, 0);
        assert_eq!(stats.native_structural.live_destinations, 0);
        assert_eq!(stats.native_structural.teardown_failures, 0);
        assert!(stats.code_objects.iter().all(|object| {
            !object
                .runtime_calls
                .contains(&lkjscript_native::RuntimeCallSlot::HeapDispatch)
        }));
    }
    assert!(native.stats.native_entries > 0);
    assert!(optimized.stats.optimizing_native_entries > 0);
}

#[test]
fn recursive_structural_string_calls_teardown_every_owner() {
    let source = concat!(
        "def/\nname/\nwalk\n/name\nfn/\nsig/\ninputs/\nstring\ni64\n/inputs\n",
        "output/\ni64\n/output\n/sig\nparams/\ntext\nstring\ndepth\ni64\n/params\n",
        "if/\nless-than-or-equal/\ndepth\n0\n/less-than-or-equal\n",
        "string-byte-length/\ntext\n/string-byte-length\nwalk/\ntext\n",
        "subtract/\ndepth\n1\n/subtract\n/walk\n/if\n/fn\n/def\n",
        "main/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\n",
        "walk/\nempty-string/\n/empty-string\n4\n/walk\n/main\n",
    );
    let program = compile(source, "recursive-structural-string.lkjscript");
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let native = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("recursive generated structural execution");
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("recursive optimized structural execution");
    assert_eq!(execution(native.outcome), execution(vm.clone()));
    assert_eq!(execution(optimized.outcome), execution(vm));
    assert!(native.stats.direct_native_calls >= 4);
    assert!(optimized.stats.direct_native_calls >= 4);
    assert!(native.stats.peak_native_frame_depth >= 5);
    assert!(optimized.stats.peak_native_frame_depth >= 5);
    for stats in [&native.stats, &optimized.stats] {
        assert_eq!(stats.vm_fallbacks, 0);
        assert_eq!(stats.collector_runtime_invocations, 0);
        assert_eq!(stats.runtime_heap_successes, 0);
        assert_eq!(stats.native_structural.live_roots, 0);
        assert_eq!(stats.native_structural.live_loans, 0);
        assert_eq!(stats.native_structural.live_destinations, 0);
        assert_eq!(stats.native_structural.teardown_failures, 0);
        assert!(stats.native_structural.roots_dropped >= 5);
    }
}
