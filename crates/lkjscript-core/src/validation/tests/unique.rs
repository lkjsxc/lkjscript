use super::*;

fn unique_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    chunk.main.locals = 3;
    chunk.main.unique_places = 1;
    let size = chunk.add_const(crate::Constant::I64(2));
    let index = chunk.add_const(crate::Constant::I64(1));
    let byte = chunk.add_const(crate::Constant::I64(77));
    chunk.main.emit_op_u16(Op::LoadConst, size.0);
    chunk.main.emit(Op::ByteVectorNew);
    chunk.main.emit_op_u8(Op::StoreUniqueLocal, 0);
    chunk.main.emit_op_u64_pair(Op::ByteVectorPlaceInit, 0, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::ByteVectorBorrowMut, 0);
    chunk.main.emit_op_u8(Op::StoreViewLocal, 1);
    chunk.main.emit_op_u8(Op::LoadViewLocal, 1);
    chunk.main.emit_op_u16(Op::LoadConst, index.0);
    chunk.main.emit_op_u16(Op::LoadConst, byte.0);
    chunk.main.emit(Op::ByteSliceMutSet);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::EndBorrowLocal, 1);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::ByteVectorBorrow, 0);
    chunk.main.emit_op_u8(Op::StoreViewLocal, 1);
    chunk.main.emit_op_u8(Op::LoadViewLocal, 1);
    chunk.main.emit(Op::ByteSliceLen);
    chunk.main.emit_op_u8(Op::StoreLocal, 2);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::EndBorrowLocal, 1);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64_pair(Op::ByteVectorDropPlace, 0, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::ByteVectorPlaceEnd, 0);
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u8(Op::LoadLocal, 2);
    chunk.main.emit(Op::Return);
    chunk
}

#[test]
fn exact_unique_family_validates_without_traced_byte_objects() {
    validate_chunk(unique_chunk(), ValidationPolicy::Unrestricted)
        .expect("exact unique byte-vector chunk validates");
}

#[test]
fn ordinary_dup_and_load_cannot_copy_unique_values() {
    let mut duplicate = Chunk::new();
    let size = duplicate.add_const(crate::Constant::I64(1));
    duplicate.main.emit_op_u16(Op::LoadConst, size.0);
    duplicate.main.emit(Op::ByteVectorNew);
    duplicate.main.emit(Op::Dup);
    duplicate.main.emit(Op::Return);
    assert!(error(duplicate).contains("Dup cannot forge"));

    let mut load = unique_chunk();
    let position = load
        .main
        .code
        .windows(9)
        .position(|bytes| bytes == index_instruction(Op::LoadViewLocal, 1))
        .expect("view load opcode");
    load.main.code[position] = Op::LoadLocal as u8;
    assert!(error(load).contains("typed local opcodes"));
}

#[test]
fn missing_end_borrow_and_drop_reject_before_execution() {
    let mut live_loan = unique_chunk();
    let end = live_loan
        .main
        .code
        .windows(9)
        .rposition(|bytes| bytes == index_instruction(Op::EndBorrowLocal, 1))
        .expect("end-borrow opcode");
    live_loan.main.code.splice(end..end + 9, [Op::Nop as u8]);
    let message = error(live_loan);
    assert!(!message.is_empty());

    let mut missing_drop = Chunk::new();
    missing_drop.main.locals = 1;
    missing_drop.main.unique_places = 1;
    let size = missing_drop.add_const(crate::Constant::I64(1));
    missing_drop.main.emit_op_u16(Op::LoadConst, size.0);
    missing_drop.main.emit(Op::ByteVectorNew);
    missing_drop.main.emit_op_u8(Op::StoreUniqueLocal, 0);
    missing_drop
        .main
        .emit_op_u64_pair(Op::ByteVectorPlaceInit, 0, 0);
    missing_drop.main.emit(Op::Pop);
    missing_drop.main.emit_op_u8(Op::ByteVectorPlaceEnd, 0);
    missing_drop.main.emit(Op::Return);
    assert!(error(missing_drop).contains("missing Drop"));
}

#[test]
fn post_move_use_overlap_and_wrong_view_type_reject() {
    let mut moved = Chunk::new();
    moved.main.locals = 2;
    moved.main.unique_places = 1;
    let size = moved.add_const(crate::Constant::I64(1));
    moved.main.emit_op_u16(Op::LoadConst, size.0);
    moved.main.emit(Op::ByteVectorNew);
    moved.main.emit_op_u8(Op::StoreUniqueLocal, 0);
    moved.main.emit_op_u64_pair(Op::ByteVectorPlaceInit, 0, 0);
    moved.main.emit(Op::Pop);
    moved.main.emit_op_u64_pair(Op::ByteVectorMove, 0, 0);
    moved.main.emit_op_u8(Op::StoreUniqueLocal, 1);
    moved.main.emit_op_u8(Op::ByteVectorBorrow, 0);
    moved.main.emit(Op::Return);
    assert!(error(moved).contains("moved, stale"));

    let mut overlap = unique_chunk();
    let first_store = overlap
        .main
        .code
        .windows(9)
        .position(|bytes| bytes == index_instruction(Op::StoreViewLocal, 1))
        .expect("first view store");
    overlap.main.code.splice(
        first_store + 9..first_store + 9,
        index_instruction(Op::ByteVectorBorrowMut, 0),
    );
    assert!(error(overlap).contains("conflicts with a live loan"));
    let mut wrong = unique_chunk();
    let borrow = wrong
        .main
        .code
        .windows(9)
        .position(|bytes| bytes == index_instruction(Op::ByteVectorBorrowMut, 0))
        .expect("mutable borrow");
    wrong.main.code[borrow] = Op::ByteVectorBorrow as u8;
    assert!(error(wrong).contains("wrong or unused view type"));
}
