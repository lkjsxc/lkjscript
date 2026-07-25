use super::*;

#[test]
fn fuel_and_returned_values_use_structured_outcomes() {
    let chunk = validated(&[Op::Unit, Op::Return]);
    let returned = Vm::new(&chunk, NullJit, Vec::new(), ExecutionConfig::default()).run();
    assert!(matches!(returned, ExecutionOutcome::Returned(value) if value.is_unit()));

    let mut config = ExecutionConfig::default();
    config.instruction_fuel = 1;
    let exhausted = Vm::new(&chunk, NullJit, Vec::new(), config).run();
    assert_eq!(
        exhausted,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel)
    );
}
#[test]
fn exit_does_not_terminate_or_contaminate_later_vms() {
    let mut exit = Chunk::new();
    let zero = exit.add_const(Constant::I64(0));
    exit.main.emit_op_u16(Op::LoadConst, zero.0);
    exit.main.emit(Op::Exit);
    let exit = validate_chunk(exit, &ValidationLimits::default()).expect("exit validates");
    assert_eq!(
        Vm::new(&exit, NullJit, Vec::new(), ExecutionConfig::default()).run(),
        ExecutionOutcome::Exited(0)
    );

    let returned = validated(&[Op::Unit, Op::Return]);
    assert!(matches!(
        Vm::new(
            &returned,
            NullJit,
            Vec::new(),
            ExecutionConfig::default()
        )
        .run(),
        ExecutionOutcome::Returned(value) if value.is_unit()
    ));
}
#[test]
fn trap_does_not_contaminate_a_later_vm() {
    let mut trap = Chunk::new();
    let one = trap.add_const(Constant::I64(1));
    let zero = trap.add_const(Constant::I64(0));
    trap.main.emit_op_u16(Op::LoadConst, one.0);
    trap.main.emit_op_u16(Op::LoadConst, zero.0);
    trap.main.emit(Op::Div);
    trap.main.emit(Op::Return);
    let trap = validate(trap);
    assert!(matches!(
        Vm::new(&trap, NullJit, Vec::new(), ExecutionConfig::default()).run(),
        ExecutionOutcome::Trapped(_)
    ));

    let returned = validated(&[Op::Unit, Op::Return]);
    assert!(matches!(
        Vm::new(
            &returned,
            NullJit,
            Vec::new(),
            ExecutionConfig::default()
        )
        .run(),
        ExecutionOutcome::Returned(value) if value.is_unit()
    ));
}
#[test]
fn returned_heap_values_own_their_storage() {
    let mut chunk = Chunk::new();
    let text = chunk.add_const(Constant::Str("owned".into()));
    chunk.main.emit_op_u16(Op::LoadConst, text.0);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    let outcome = Vm::new(&chunk, NullJit, Vec::new(), ExecutionConfig::default()).run();
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_str() == Some("owned")
    ));
}
#[test]
fn sha256_opcode_returns_language_results_for_valid_and_invalid_ranges() {
    let mut valid = Chunk::new();
    let zero = valid.add_const(Constant::I64(0));
    valid.main.emit_op_u16(Op::LoadConst, zero.0);
    valid.main.emit(Op::BufNew);
    valid.main.emit_op_u16(Op::LoadConst, zero.0);
    valid.main.emit_op_u16(Op::LoadConst, zero.0);
    valid.main.emit(Op::SysSha256);
    valid.main.emit(Op::IsOk);
    valid.main.emit(Op::Return);
    let valid = validate(valid);
    assert!(matches!(
        Vm::new(&valid, NullJit, Vec::new(), ExecutionConfig::default()).run(),
        ExecutionOutcome::Returned(value) if value.as_bool() == Some(true)
    ));

    let mut invalid = Chunk::new();
    let zero = invalid.add_const(Constant::I64(0));
    let one = invalid.add_const(Constant::I64(1));
    invalid.main.emit_op_u16(Op::LoadConst, zero.0);
    invalid.main.emit(Op::BufNew);
    invalid.main.emit_op_u16(Op::LoadConst, zero.0);
    invalid.main.emit_op_u16(Op::LoadConst, one.0);
    invalid.main.emit(Op::SysSha256);
    invalid.main.emit(Op::IsOk);
    invalid.main.emit(Op::Return);
    let invalid = validate(invalid);
    assert!(matches!(
        Vm::new(&invalid, NullJit, Vec::new(), ExecutionConfig::default()).run(),
        ExecutionOutcome::Returned(value) if value.as_bool() == Some(false)
    ));
}
