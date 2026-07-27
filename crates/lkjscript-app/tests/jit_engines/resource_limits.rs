use std::time::Duration;

use crate::canonical::{compile, execution, f64_loop, Scalar};
use lkjscript_core::{ExecutionConfig, ExecutionOutcome, ResourceLimitKind};
use lkjscript_jit::{execute_forced, execute_optimizing, FailureCode, JitConfig};

#[test]
fn native_poll_deadline_fuel_and_code_work_limits_are_bounded() {
    let program = compile(&f64_loop(), "limits.lkjscript");
    let mut deadline = ExecutionConfig::default();
    deadline.wall_time = Some(Duration::ZERO);
    let outcome = execute_forced(program.ssa(), &deadline, JitConfig::default())
        .expect("deadline is a language outcome");
    assert_eq!(execution(outcome.outcome), Scalar::Deadline);
    let optimized = execute_optimizing(program.ssa(), &deadline, JitConfig::default())
        .expect("optimizing deadline is a language outcome");
    assert_eq!(execution(optimized.outcome), Scalar::Deadline);
    assert_eq!(optimized.stats.baseline_native_entries, 0);

    let mut fuel = ExecutionConfig::default();
    fuel.instruction_fuel = 0;
    let outcome = execute_forced(program.ssa(), &fuel, JitConfig::default())
        .expect("fuel is a language outcome");
    assert_eq!(execution(outcome.outcome), Scalar::Fuel);
    let optimized = execute_optimizing(program.ssa(), &fuel, JitConfig::default())
        .expect("optimizing fuel is a language outcome");
    assert_eq!(execution(optimized.outcome), Scalar::Fuel);
    assert_eq!(optimized.stats.baseline_native_entries, 0);

    for maximum in [0, 1] {
        let mut stack = ExecutionConfig::default();
        stack.max_stack_values = maximum;
        let outcome = execute_forced(program.ssa(), &stack, JitConfig::default())
            .expect("native active-value limit is a language outcome");
        assert_eq!(
            outcome.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
        );
        assert_eq!(outcome.stats.peak_native_frame_depth, 0);
        let optimized = execute_optimizing(program.ssa(), &stack, JitConfig::default())
            .expect("optimizing active-value limit is a language outcome");
        assert_eq!(
            optimized.outcome,
            ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::StackValues)
        );
        assert!(optimized.stats.optimizing_native_entries > 0);
        assert_eq!(optimized.stats.baseline_native_entries, 0);
    }

    let mut no_frames = ExecutionConfig::default();
    no_frames.max_frames = 0;
    let optimized = execute_optimizing(program.ssa(), &no_frames, JitConfig::default())
        .expect("optimizing frame limit is a language outcome");
    assert_eq!(
        optimized.outcome,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::FrameDepth)
    );
    assert!(optimized.stats.optimizing_native_entries > 0);
    assert_eq!(optimized.stats.baseline_native_entries, 0);

    let allocation = compile(
        "main/\nsig/\ninputs/\n/inputs\noutput/\nstring\n/output\n/sig\nempty-string/\n/empty-string\n/main\n",
        "tiny-allocation-limit.lkjscript",
    );
    let mut no_allocations = ExecutionConfig::default();
    no_allocations.max_allocations = 0;
    let outcome = execute_forced(allocation.ssa(), &no_allocations, JitConfig::default())
        .expect("allocation limit is structured");
    assert!(matches!(
        outcome.outcome,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::Allocations)
    ));
    let mut tiny_heap = ExecutionConfig::default();
    tiny_heap.max_heap_bytes = 1;
    let outcome = execute_forced(allocation.ssa(), &tiny_heap, JitConfig::default())
        .expect("heap limit is structured");
    assert!(matches!(
        outcome.outcome,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
    ));

    let mut limited = JitConfig::default();
    limited.backend_limits =
        lkjscript_native::BackendLimits::new(64, 1024, 16_384, 1024, 1, 4 * 1024 * 1024, 100_000);
    let error = execute_forced(program.ssa(), &ExecutionConfig::default(), limited)
        .expect_err("code byte limit must fail forced engine");
    assert_eq!(error.code(), FailureCode::BackendVerification);
}
