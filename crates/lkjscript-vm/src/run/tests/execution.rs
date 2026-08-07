use super::*;

#[test]
fn fuel_and_returned_values_use_structured_outcomes() {
    let chunk = validated(&[Op::Unit, Op::Return]);
    let returned = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    )
    .run();
    assert!(matches!(returned, ExecutionOutcome::Returned(value) if value.is_unit()));

    let mut config =
        ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy::conservative());
    config
        .limited_policy_mut()
        .expect("limited test policy")
        .instruction_fuel = 1;
    let exhausted = Vm::new(&chunk, crate::ExecutionInputs::default(), config).run();
    assert_eq!(
        exhausted,
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::InstructionFuel)
    );
    let unchanged = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    )
    .run();
    assert!(matches!(unchanged, ExecutionOutcome::Returned(value) if value.is_unit()));
}
#[test]
fn exit_does_not_terminate_or_contaminate_later_vms() {
    let mut exit = Chunk::new();
    let zero = exit.add_const(Constant::I64(0)).expect("add zero constant");
    exit.main.emit_op_u64(Op::LoadConst, zero.0);
    exit.main.emit(Op::Exit);
    let exit = validate_chunk(exit, ValidationPolicy::Unrestricted).expect("exit validates");
    assert_eq!(
        Vm::new(
            &exit,
            crate::ExecutionInputs::default(),
            ExecutionPolicy::unrestricted()
        )
        .run(),
        ExecutionOutcome::Exited(0)
    );

    let returned = validated(&[Op::Unit, Op::Return]);
    assert!(matches!(
        Vm::new(
            &returned,
            crate::ExecutionInputs::default(),
            ExecutionPolicy::unrestricted()
        )
        .run(),
        ExecutionOutcome::Returned(value) if value.is_unit()
    ));
}
#[test]
fn trap_does_not_contaminate_a_later_vm() {
    let mut trap = Chunk::new();
    let one = trap.add_const(Constant::I64(1)).expect("add one constant");
    let zero = trap.add_const(Constant::I64(0)).expect("add zero constant");
    trap.main.emit_op_u64(Op::LoadConst, one.0);
    trap.main.emit_op_u64(Op::LoadConst, zero.0);
    trap.main.emit(Op::Div);
    trap.main.emit(Op::Return);
    let trap = validate(trap);
    assert!(matches!(
        Vm::new(
            &trap,
            crate::ExecutionInputs::default(),
            ExecutionPolicy::unrestricted()
        )
        .run(),
        ExecutionOutcome::Trapped(_)
    ));

    let returned = validated(&[Op::Unit, Op::Return]);
    assert!(matches!(
        Vm::new(
            &returned,
            crate::ExecutionInputs::default(),
            ExecutionPolicy::unrestricted()
        )
        .run(),
        ExecutionOutcome::Returned(value) if value.is_unit()
    ));
}
#[test]
fn returned_heap_values_own_their_storage() {
    let mut chunk = Chunk::new();
    let text = chunk
        .add_const(Constant::Str("owned".into()))
        .expect("add text constant");
    chunk.main.emit_op_u64(Op::LoadConst, text.0);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    let outcome = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
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
    let size = chunk
        .add_const(Constant::I64(1))
        .expect("add size constant");
    chunk.main.emit_op_u64(Op::LoadConst, size.0);
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
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    )
    .run();
    assert!(matches!(outcome, ExecutionOutcome::Returned(value) if value.is_unit()));
}

#[test]
fn byte_vector_program_crosses_former_limit_only_under_sufficient_heap_policy() {
    let size = 1_000_001_i64;
    let mut chunk = Chunk::new();
    chunk.main.locals = 1;
    chunk.main.unique_places = 1;
    let size_constant = chunk
        .add_const(Constant::I64(size))
        .expect("add large size constant");
    chunk.main.emit_op_u64(Op::LoadConst, size_constant.0);
    chunk.main.emit(Op::ByteVectorNew);
    chunk.main.emit_op_u64(Op::StoreUniqueLocal, 0);
    chunk.main.emit_op_u64_pair(Op::ByteVectorPlaceInit, 0, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64_pair(Op::ByteVectorDropPlace, 0, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64(Op::ByteVectorPlaceEnd, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);

    let low = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_heap_bytes: usize::try_from(size - 1).expect("test size fits usize"),
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    assert_eq!(
        Vm::new(&chunk, crate::ExecutionInputs::default(), low).run(),
        ExecutionOutcome::ResourceLimitExceeded(ResourceLimitKind::HeapBytes)
    );

    let high = ExecutionPolicy::limited(lkjscript_core::LimitedExecutionPolicy {
        max_heap_bytes: usize::try_from(size * 2).expect("test size fits usize"),
        ..lkjscript_core::LimitedExecutionPolicy::conservative()
    });
    assert!(matches!(
        Vm::new(
            &chunk,
            crate::ExecutionInputs::default(),
            high
        )
        .run(),
        ExecutionOutcome::Returned(value) if value.is_unit()
    ));
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

const WIDE_TABLE_COUNT: usize = 65_537;
const WIDE_TABLE_HIGH: u64 = 65_536;

#[test]
fn wide_constant_index_validates_and_executes_without_aliasing() {
    let mut chunk = Chunk::new();
    let mut high = None;
    for value in 0..WIDE_TABLE_COUNT {
        high = Some(
            chunk
                .add_const(Constant::I64(
                    i64::try_from(value).expect("test value fits i64"),
                ))
                .expect("add distinct constant"),
        );
    }
    let high = high.expect("wide constant table is nonempty");
    assert_eq!(high.0, WIDE_TABLE_HIGH);
    chunk.main.emit_op_u64(Op::LoadConst, high.0);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    assert_eq!(
        chunk.main_instructions()[0].operand().index(),
        Some(WIDE_TABLE_COUNT - 1)
    );
    let outcome = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    )
    .run();
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(65_536)
    ));
}

#[test]
fn wide_global_index_stores_loads_and_executes() {
    let mut chunk = Chunk::new();
    let mut high = None;
    for index in 0..WIDE_TABLE_COUNT {
        high = Some(
            chunk
                .intern_global(&format!("global-{index}"))
                .expect("intern distinct global"),
        );
    }
    let high = high.expect("wide global table is nonempty");
    assert_eq!(high.0, WIDE_TABLE_HIGH);
    let value = chunk
        .add_const(Constant::I64(71))
        .expect("add global value constant");
    chunk.main.emit_op_u64(Op::LoadConst, value.0);
    chunk.main.emit_op_u64(Op::StoreGlobal, high.0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64(Op::LoadGlobal, high.0);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    let outcome = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    )
    .run();
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(71)
    ));
}

#[test]
fn wide_prototype_reference_constructs_and_calls_the_high_closure() {
    let mut chunk = Chunk::new();
    let result = chunk
        .add_const(Constant::I64(99))
        .expect("add high prototype result");
    chunk
        .protos
        .try_reserve_exact(WIDE_TABLE_COUNT)
        .expect("reserve wide prototype table");
    for index in 0..WIDE_TABLE_COUNT {
        let mut prototype = Chunk::new().main;
        prototype.name = format!("prototype-{index}");
        if index + 1 == WIDE_TABLE_COUNT {
            prototype.emit_op_u64(Op::LoadConst, result.0);
        } else {
            prototype.emit(Op::Unit);
        }
        prototype.emit(Op::Return);
        chunk.protos.push(prototype);
    }
    let constant = chunk
        .add_const(Constant::Proto(WIDE_TABLE_HIGH))
        .expect("add high prototype constant");
    chunk.main.emit_op_u64(Op::LoadConst, constant.0);
    chunk.main.emit_op_u64(Op::MakeClosure, 0);
    chunk.main.emit_op_u64(Op::Call, 0);
    chunk.main.emit(Op::Return);
    let chunk = validate(chunk);
    let outcome = Vm::new(
        &chunk,
        crate::ExecutionInputs::default(),
        ExecutionPolicy::unrestricted(),
    )
    .run();
    assert!(matches!(
        outcome,
        ExecutionOutcome::Returned(value) if value.as_i64() == Some(99)
    ));
}
