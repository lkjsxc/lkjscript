use crate::canonical::{compile, execution, f64_loop, Scalar};
use lkjscript_core::ExecutionConfig;
use lkjscript_jit::{execute_forced, execute_optimizing, JitConfig, JitStats};
use lkjscript_native::RuntimeCallSlot;

#[test]
fn forced_scalar_groups_have_structurally_zero_collector_interaction() {
    let source = scalar_island_source();
    let program = compile(&source, "collector-free-scalar.lkjscript");
    let baseline = execute_forced(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced baseline scalar island");
    let proof = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("forced proof scalar island");

    assert_eq!(execution(baseline.outcome), Scalar::Bool(true));
    assert_eq!(execution(proof.outcome), Scalar::Bool(true));
    assert_collector_zero(&baseline.stats);
    assert_collector_zero(&proof.stats);
    assert!(baseline.stats.baseline_native_entries > 0);
    assert_eq!(baseline.stats.optimizing_native_entries, 0);
    assert_eq!(proof.stats.baseline_native_entries, 0);
    assert!(proof.stats.optimizing_native_entries > 0);
    assert!(proof
        .stats
        .code_objects
        .iter()
        .all(|object| object.optimization_certificate.is_some()));
}

fn assert_collector_zero(stats: &JitStats) {
    assert!(stats.native_entries > 0);
    assert!(stats.direct_native_calls > 0);
    assert!(stats.poll_calls > 0);
    assert_eq!(stats.vm_fallbacks, 0);
    assert_eq!(stats.vm_to_native_transitions, 0);
    assert_eq!(stats.native_to_vm_transitions, 0);
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.allocation_bytes_estimate, 0);
    assert_eq!(stats.collections, 0);
    assert_eq!(stats.peak_live_heap_bytes_estimate, 0);
    assert_eq!(stats.maximum_roots, 0);
    assert_eq!(stats.runtime_heap_attempts, 0);
    assert_eq!(stats.runtime_heap_successes, 0);
    assert_eq!(stats.barrier_count, 0);
    assert_eq!(stats.collector_runtime_invocations, 0);
    assert_eq!(stats.resource_runtime_calls, 0);
    assert!(stats.code_objects.iter().all(|object| {
        object.safepoint_count == 0
            && object.exact_scalar_stack_maps
            && !object
                .runtime_calls
                .contains(&RuntimeCallSlot::HeapDispatch)
            && !object
                .runtime_calls
                .contains(&RuntimeCallSlot::CollectReference)
            && !object
                .runtime_calls
                .contains(&RuntimeCallSlot::PublishSafepoint)
    }));
}

fn scalar_island_source() -> String {
    let source = f64_loop();
    let (definitions, main) = source
        .rsplit_once("main/\n")
        .expect("canonical scalar fixture contains main");
    let main = main.replacen("output/\nf64\n/output", "output/\nbool\n/output", 1);
    let main = main.replacen(
        "acc\n/do\n/var\n/var\n/main",
        "less-than/\nacc\n1000.0\n/less-than\n/do\n/var\n/var\n/main",
        1,
    );
    format!("{definitions}main/\n{main}")
}
