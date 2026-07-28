use crate::canonical::{compile, execution};
use lkjscript_core::ExecutionConfig;
use lkjscript_jit::{execute_optimizing, JitConfig};

#[test]
fn forced_scheduled_proof_discovery_preserves_native_output_and_proof_counts() {
    let source = include_str!("../fixtures/optimizing-loop.lkjscript");
    let program = compile(source, "scheduled-optimizing-loop.lkjscript");
    let sequential = execute_optimizing(
        program.ssa(),
        &ExecutionConfig::default(),
        JitConfig::default(),
    )
    .expect("sequential proof discovery");
    let mut config = JitConfig::default();
    config.proof_discovery_workers = 2;
    let scheduled = execute_optimizing(program.ssa(), &ExecutionConfig::default(), config)
        .expect("scheduled proof discovery");
    assert_eq!(execution(scheduled.outcome), execution(sequential.outcome));
    assert_eq!(
        scheduled.stats.optimization_certificate_records,
        sequential.stats.optimization_certificate_records
    );
    assert_eq!(
        scheduled.stats.optimization_certificate_bytes_estimate,
        sequential.stats.optimization_certificate_bytes_estimate
    );
    assert_eq!(
        scheduled.stats.optimizing_passes,
        sequential.stats.optimizing_passes
    );
    assert!(scheduled.stats.optimizing_native_entries > 0);
    assert_eq!(scheduled.stats.vm_fallbacks, 0);
}
