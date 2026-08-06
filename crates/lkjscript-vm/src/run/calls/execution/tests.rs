use super::*;

fn emit_index(code: &mut Vec<u8>, op: Op, index: u64) {
    code.push(op as u8);
    code.extend_from_slice(&index.to_le_bytes());
}

#[test]
fn unique_forwarding_epilogue_with_place_ends_is_tail_position() {
    let mut code = Vec::new();
    emit_index(&mut code, Op::StoreUniqueLocal, 4);
    emit_index(&mut code, Op::ByteVectorPlaceEnd, 5);
    emit_index(&mut code, Op::StoreLocal, 6);
    code.push(Op::Pop as u8);
    emit_index(&mut code, Op::ByteVectorPlaceEnd, 4);
    emit_index(&mut code, Op::StoreLocal, 6);
    code.push(Op::Pop as u8);
    emit_index(&mut code, Op::TakeUniqueLocal, 4);
    code.push(Op::Return as u8);
    assert!(forwarding_epilogue(&code, 0));
}

#[test]
fn scalar_forwarding_epilogue_accepts_a_high_local_slot() {
    let mut code = Vec::new();
    emit_index(&mut code, Op::StoreLocal, 299);
    code.push(Op::Pop as u8);
    emit_index(&mut code, Op::LoadLocal, 299);
    code.push(Op::Return as u8);
    assert!(forwarding_epilogue(&code, 0));
}
