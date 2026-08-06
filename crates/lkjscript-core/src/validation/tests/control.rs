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
fn main_arity_global_initialization_and_static_operation_categories_are_checked() {
    let mut main = unit_chunk();
    main.main.arity = 1;
    main.main.locals = 1;
    assert!(error(main).contains("capability requirements"));

    let mut global = unit_chunk();
    global.global_names.push("g".into());
    global.main.code = vec![Op::LoadGlobal as u8, 0, 0, Op::Return as u8];
    assert!(error(global).contains("global is not definitely initialized"));

    for (operation, category) in [
        (Op::Car, "list"),
        (Op::ByteSliceLen, "byte view"),
        (Op::SysClose, "typed resource"),
        (Op::ConvertBytesToString, "immutable bytes"),
    ] {
        let mut chunk = unit_chunk();
        chunk.main.code = if operation == Op::Car {
            vec![
                Op::Unit as u8,
                operation as u8,
                u8::MAX,
                u8::MAX,
                Op::Return as u8,
            ]
        } else {
            vec![Op::Unit as u8, operation as u8, Op::Return as u8]
        };
        let message = error(chunk);
        assert!(
            message.contains(category),
            "wrong category diagnostic for {operation:?}: {message}"
        );
    }
}
