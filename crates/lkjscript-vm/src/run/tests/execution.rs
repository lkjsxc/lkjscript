use super::*;

#[test]
fn fuel_and_returned_values_use_structured_outcomes() {
    let chunk = validated(&[Op::Unit, Op::Return]);
    let returned = Vm::new(
        &chunk,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    )
    .run();
    assert!(matches!(returned, ExecutionOutcome::Returned(value) if value.is_unit()));

    let mut config = ExecutionConfig::default();
    config.instruction_fuel = 1;
    let exhausted = Vm::new(&chunk, NullJit, crate::ExecutionInputs::default(), config).run();
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
    let exit = validate_chunk(exit, ValidationPolicy::Unrestricted).expect("exit validates");
    assert_eq!(
        Vm::new(
            &exit,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default()
        )
        .run(),
        ExecutionOutcome::Exited(0)
    );

    let returned = validated(&[Op::Unit, Op::Return]);
    assert!(matches!(
        Vm::new(
            &returned,
            NullJit,
            crate::ExecutionInputs::default(),
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
        Vm::new(
            &trap,
            NullJit,
            crate::ExecutionInputs::default(),
            ExecutionConfig::default()
        )
        .run(),
        ExecutionOutcome::Trapped(_)
    ));

    let returned = validated(&[Op::Unit, Op::Return]);
    assert!(matches!(
        Vm::new(
            &returned,
            NullJit,
            crate::ExecutionInputs::default(),
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
    let outcome = Vm::new(
        &chunk,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    )
    .run();
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_str() == Some("owned")
    ));
}
#[test]
fn high_unique_local_and_place_execute_without_byte_narrowing() {
    let mut chunk = Chunk::new();
    chunk.main.locals = 300;
    chunk.main.unique_places = 300;
    let size = chunk.add_const(Constant::I64(1));
    chunk.main.emit_op_u16(Op::LoadConst, size.0);
    chunk.main.emit(Op::ByteVectorNew);
    chunk.main.emit_op_u64(Op::StoreUniqueLocal, 299);
    chunk
        .main
        .emit_op_u64_pair(Op::ByteVectorPlaceInit, 299, 299);
    chunk.main.emit(Op::Pop);
    chunk
        .main
        .emit_op_u64_pair(Op::ByteVectorDropPlace, 299, 299);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64(Op::ByteVectorPlaceEnd, 299);
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    let outcome = Vm::new(
        &chunk,
        NullJit,
        crate::ExecutionInputs::default(),
        ExecutionConfig::default(),
    )
    .run();
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.is_unit()));
}

#[test]
fn removed_buffer_opcode_bytes_are_rejected_before_execution() {
    for removed in [66_u8, 67, 68, 69, 72, 73, 85, 191] {
        let mut chunk = Chunk::new();
        chunk.main.code = vec![removed, Op::Return as u8];
        assert!(lkjscript_core::validate_chunk(
            chunk,
            lkjscript_core::ValidationPolicy::Unrestricted
        )
        .is_err());
    }
}
