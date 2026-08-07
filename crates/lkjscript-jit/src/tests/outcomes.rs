use super::*;

#[test]
fn verified_ssa_trap_and_structured_outcome_reach_native_status() {
    let trap = terminal_program(
        Terminator::Trap {
            value: ValueId::new(0),
        },
        EffectSet::MAY_TRAP,
    );
    let executed = execute_forced(
        &trap,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("execute explicit native trap");
    assert!(matches!(
        executed.outcome,
        ExecutionOutcome::Trapped(trap) if trap.as_str() == "exact native trap"
    ));

    let deadline = terminal_program(
        Terminator::Outcome {
            outcome: StructuredOutcome::DeadlineExceeded,
            detail: None,
        },
        EffectSet::PURE,
    );
    let executed = execute_forced(
        &deadline,
        &ExecutionPolicy::unrestricted(),
        JitConfig::default(),
    )
    .expect("execute native structured outcome");
    assert_eq!(executed.outcome, ExecutionOutcome::DeadlineExceeded);
}
