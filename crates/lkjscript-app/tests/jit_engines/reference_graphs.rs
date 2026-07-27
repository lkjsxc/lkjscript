use crate::canonical::{compile, evaluator, execution, Scalar};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome};
use lkjscript_ir::{evaluate, EvalConfig};
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig};
use lkjscript_vm::run_chunk;

#[test]
fn nested_product_option_result_list_string_buffer_graph_matches_all_engines() {
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
    assert_eq!(expected, Scalar::I64(1));
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
    assert!(native.stats.allocations >= 7);
    assert!(native.stats.collections >= 7);
    assert!(native.stats.maximum_roots > 0);
    assert!(native.stats.runtime_heap_attempts >= 14);
    assert_eq!(
        native.stats.runtime_heap_attempts,
        native.stats.runtime_heap_successes
    );
    assert!(native.stats.barrier_count >= 4);
}

#[test]
fn forced_collection_sees_live_reference_in_recursive_caller_and_callee_frames() {
    let source = "def/\nname/\nwalk\n/name\nfn/\nsig/\ninputs/\nstring\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\ntext\nstring\ndepth\ni64\n/params\nif/\nless-than-or-equal/\ndepth\n0\n/less-than-or-equal\nstring-byte-length/\ntext\n/string-byte-length\nwalk/\ntext\nsubtract/\ndepth\n1\n/subtract\n/walk\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\nwalk/\nempty-string/\n/empty-string\n4\n/walk\n/main\n";
    let program = compile(source, "recursive-roots.lkjscript");
    let vm = run_chunk(
        program.bytecode(),
        &lkjscript_vm::ExecutionInputs::default(),
        &ExecutionConfig::default(),
    );
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("recursive generated reference execution");
    let mut optimizing_config = JitConfig::default();
    optimizing_config.force_gc_before_allocation = true;
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        optimizing_config,
    )
    .expect("optimizing recursive generated reference execution");
    assert_eq!(execution(native.outcome), execution(vm.clone()));
    assert_eq!(execution(optimized.outcome), execution(vm));
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(native.stats.native_entries >= 6);
    assert!(optimized.stats.optimizing_native_entries >= 6);
    assert!(native.stats.peak_native_frame_depth >= 6);
    assert!(optimized.stats.peak_native_frame_depth >= 6);
    assert!(native.stats.collections >= 2);
    assert!(optimized.stats.collections >= 2);
    assert!(native.stats.maximum_roots >= 5);
    assert!(optimized.stats.maximum_roots >= 5);
    assert!(native
        .stats
        .code_objects
        .iter()
        .any(|object| !object.exact_scalar_stack_maps));
    assert!(optimized
        .stats
        .code_objects
        .iter()
        .any(|object| !object.exact_scalar_stack_maps));

    let mutual = "def/\nname/\neven\n/name\nfn/\nsig/\ninputs/\nstring\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\ntext\nstring\ndepth\ni64\n/params\nif/\nless-than-or-equal/\ndepth\n0\n/less-than-or-equal\nstring-byte-length/\ntext\n/string-byte-length\nodd/\ntext\nsubtract/\ndepth\n1\n/subtract\n/odd\n/if\n/fn\n/def\ndef/\nname/\nodd\n/name\nfn/\nsig/\ninputs/\nstring\ni64\n/inputs\noutput/\ni64\n/output\n/sig\nparams/\ntext\nstring\ndepth\ni64\n/params\nif/\nless-than-or-equal/\ndepth\n0\n/less-than-or-equal\nstring-byte-length/\ntext\n/string-byte-length\neven/\ntext\nsubtract/\ndepth\n1\n/subtract\n/even\n/if\n/fn\n/def\nmain/\nsig/\ninputs/\n/inputs\noutput/\ni64\n/output\n/sig\neven/\nempty-string/\n/empty-string\n5\n/even\n/main\n";
    let program = compile(mutual, "mutual-recursive-roots.lkjscript");
    let mut config = JitConfig::default();
    config.force_gc_before_allocation = true;
    let native = execute_forced(program.ssa(), &ExecutionConfig::default(), config)
        .expect("mutual recursive SCC with live reference executes natively");
    let mut optimizing_config = JitConfig::default();
    optimizing_config.force_gc_before_allocation = true;
    let optimized = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        optimizing_config,
    )
    .expect("mutual recursive SCC with live reference optimizes natively");
    assert!(
        matches!(native.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert!(
        matches!(optimized.outcome, ExecutionOutcome::Returned(value) if value.as_i64() == Some(0))
    );
    assert_eq!(native.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.vm_fallbacks, 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);
    assert!(native.stats.peak_native_frame_depth >= 7);
    assert!(optimized.stats.peak_native_frame_depth >= 7);
    assert!(native.stats.maximum_roots >= 6);
    assert!(optimized.stats.maximum_roots >= 6);
}
