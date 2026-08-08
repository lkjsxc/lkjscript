use super::*;

#[test]
fn valid_minimal_chunk_is_opaque_and_decoded() {
    let validated = validate_chunk(unit_chunk(), ValidationPolicy::Unrestricted)
        .expect("minimal chunk validates");
    assert_eq!(validated.main_instructions().len(), 2);
    assert_eq!(validated.main_instructions()[0].op(), Op::Unit);
    assert_eq!(validated.main_instructions()[1].op(), Op::Return);
}

#[test]
fn all_bytes_are_decoded_even_when_unreachable() {
    let mut chunk = unit_chunk();
    chunk.main.code = vec![Op::Unit as u8, Op::Return as u8, 145];
    assert!(error(chunk).contains("unknown or retired opcode"));

    let mut truncated = unit_chunk();
    truncated.main.code = vec![Op::Unit as u8, Op::Return as u8, Op::LoadConst as u8, 0];
    assert!(error(truncated).contains("truncated"));
}

#[test]
fn cfg_stack_local_return_and_fallthrough_are_checked() {
    let mut underflow = unit_chunk();
    underflow.main.code = vec![Op::Pop as u8, Op::Unit as u8, Op::Return as u8];
    assert!(error(underflow).contains("stack underflow"));

    let mut local = unit_chunk();
    local.main.locals = 1;
    local.main.code.clear();
    local.main.emit_op_u64(Op::LoadLocal, 0);
    local.main.emit(Op::Return);
    assert!(error(local).contains("not definitely initialized"));

    let mut fallthrough = unit_chunk();
    fallthrough.main.code = vec![Op::Unit as u8];
    assert!(error(fallthrough).contains("falls through"));

    let mut return_shape = unit_chunk();
    return_shape.main.code = vec![Op::Unit as u8, Op::Unit as u8, Op::Return as u8];
    assert!(error(return_shape).contains("exactly one"));

    let mut join = unit_chunk();
    join.main.code.clear();
    join.main.emit(Op::True);
    join.main.emit_op_u64(Op::JumpIfFalse, 21);
    join.main.emit(Op::Unit);
    join.main.emit(Op::Unit);
    join.main.emit_op_u64(Op::Jump, 31);
    join.main.emit(Op::Unit);
    join.main.emit_op_u64(Op::Jump, 31);
    join.main.emit(Op::Return);
    assert!(error(join).contains("stack depth"));
}

#[test]
fn block_entry_dataflow_handles_straight_lines_branches_and_backedges() {
    let mut straight = unit_chunk();
    straight.main.locals = 1_024;
    straight.main.code.clear();
    for _ in 0..4_096 {
        straight.main.emit(Op::Nop);
    }
    straight.main.emit(Op::Unit);
    straight.main.emit(Op::Return);
    validate_chunk(straight, ValidationPolicy::Unrestricted)
        .expect("long straight-line bytecode validates without instruction states");

    let mut diamond = unit_chunk();
    diamond.main.code.clear();
    diamond.main.emit(Op::True);
    diamond.main.emit_op_u64(Op::JumpIfFalse, 20);
    diamond.main.emit(Op::Unit);
    diamond.main.emit_op_u64(Op::Jump, 21);
    diamond.main.emit(Op::Unit);
    diamond.main.emit(Op::Return);
    validate_chunk(diamond, ValidationPolicy::Unrestricted)
        .expect("compatible branch states merge at the join block");

    let mut looped = unit_chunk();
    looped.main.code.clear();
    looped.main.emit(Op::True);
    looped.main.emit_op_u64(Op::JumpIfFalse, 19);
    looped.main.emit_op_u64(Op::Jump, 0);
    looped.main.emit(Op::Unit);
    looped.main.emit(Op::Return);
    validate_chunk(looped, ValidationPolicy::Unrestricted)
        .expect("loop backedge converges at the block entry");
}

#[test]
fn later_predecessor_change_requeues_an_already_processed_join() {
    let mut chunk = unit_chunk();
    chunk.main.locals = 1;
    chunk.main.code.clear();
    chunk.main.emit(Op::True);
    chunk.main.emit_op_u64(Op::JumpIfFalse, 19);
    chunk.main.emit_op_u64(Op::Jump, 39);
    chunk.main.emit(Op::Unit);
    chunk.main.emit_op_u64(Op::StoreLocal, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64(Op::Jump, 49);
    chunk.main.emit(Op::Nop);
    chunk.main.emit_op_u64(Op::Jump, 49);
    chunk.main.emit_op_u64(Op::LoadLocal, 0);
    chunk.main.emit(Op::Return);

    let message = error(chunk);
    assert!(message.contains("not definitely initialized"), "{message}");
}

#[test]
fn unreachable_malformed_jump_target_still_fails_closed_deterministically() {
    let diagnose = || {
        let mut chunk = unit_chunk();
        chunk.main.emit_op_u64(Op::Jump, 3);
        error(chunk)
    };
    let first = diagnose();
    assert_eq!(first, diagnose());
    assert!(
        first.contains("jump target is out of range or not an instruction boundary"),
        "{first}"
    );
}

#[test]
fn main_arity_global_initialization_and_static_operation_categories_are_checked() {
    let mut main = unit_chunk();
    main.main.arity = 1;
    main.main.locals = 1;
    assert!(error(main).contains("capability requirements"));

    let mut global = unit_chunk();
    global.global_names.push("g".into());
    global.main.code.clear();
    global.main.emit_op_u64(Op::LoadGlobal, 0);
    global.main.emit(Op::Return);
    assert!(error(global).contains("global is not definitely initialized"));

    for (operation, category) in [
        (Op::Car, "list"),
        (Op::ByteSliceLen, "byte view"),
        (Op::SysClose, "typed resource"),
        (Op::ConvertBytesToString, "immutable bytes"),
    ] {
        let mut chunk = unit_chunk();
        chunk.main.code.clear();
        chunk.main.emit(Op::Unit);
        if operation == Op::Car {
            chunk.main.emit_op_u64(operation, 0);
        } else {
            chunk.main.emit(operation);
        }
        chunk.main.emit(Op::Return);
        let message = error(chunk);
        assert!(
            message.contains(category),
            "wrong category diagnostic for {operation:?}: {message}"
        );
    }
}
