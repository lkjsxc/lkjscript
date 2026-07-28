use super::*;

#[test]
fn trap_and_exit_preserve_primary_outcomes_during_emergency_resource_cleanup() {
    let mut trap = Chunk::new();
    let one = trap.add_const(Constant::I64(1));
    let zero = trap.add_const(Constant::I64(0));
    trap.main.emit_op_u16(Op::LoadConst, one.0);
    trap.main.emit_op_u16(Op::LoadConst, zero.0);
    trap.main.emit(Op::Div);
    trap.main.emit(Op::Return);
    let trap = validate(trap);
    let mut trapped_vm = Vm::new(
        &trap,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    );
    trapped_vm
        .resources
        .sys_open_read(b"/dev/null")
        .expect("open trapped VM resource");
    assert!(matches!(
        trapped_vm.run_inner(),
        ExecutionOutcome::Trapped(_)
    ));
    assert_emergency_cleanup(trapped_vm.resources.metrics());

    let mut exit = Chunk::new();
    let zero = exit.add_const(Constant::I64(0));
    exit.main.emit_op_u16(Op::LoadConst, zero.0);
    exit.main.emit(Op::Exit);
    let exit = validate(exit);
    let mut exited_vm = Vm::new(
        &exit,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    );
    exited_vm
        .resources
        .sys_open_read(b"/dev/null")
        .expect("open exited VM resource");
    assert_eq!(exited_vm.run_inner(), ExecutionOutcome::Exited(0));
    assert_emergency_cleanup(exited_vm.resources.metrics());
}

#[test]
fn runtime_teardown_failure_attaches_without_replacing_trap() {
    let mut trap = Chunk::new();
    let one = trap.add_const(Constant::I64(1));
    let zero = trap.add_const(Constant::I64(0));
    trap.main.emit_op_u16(Op::LoadConst, one.0);
    trap.main.emit_op_u16(Op::LoadConst, zero.0);
    trap.main.emit(Op::Div);
    trap.main.emit(Op::Return);
    let trap = validate(trap);
    let mut vm = Vm::new(
        &trap,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    );
    vm.resources.inject_borrowed_cleanup_failure();

    let outcome = vm.run_inner();
    assert!(matches!(outcome.primary(), ExecutionOutcome::Trapped(_)));
    let failures = outcome.cleanup_failures().expect("cleanup attachment");
    assert_eq!(failures.retained().len(), 1);
    assert_eq!(
        failures.retained()[0].subject(),
        lkjscript_core::CleanupSubject::BorrowedResource(lkjscript_core::ResourceKind::InputStream)
    );
    assert_eq!(
        failures.retained()[0].phase(),
        lkjscript_core::CleanupPhase::RuntimeTeardown
    );
}

fn assert_emergency_cleanup(metrics: crate::host_ext::ResourceMetrics) {
    assert_eq!(metrics.resources_opened(), 1);
    assert_eq!(metrics.resources_closed(), 1);
    assert_eq!(metrics.ordinary_obligations(), 1);
    assert_eq!(metrics.emergency_obligations(), 1);
    assert_eq!(metrics.cleanup_attempts(), 1);
}
