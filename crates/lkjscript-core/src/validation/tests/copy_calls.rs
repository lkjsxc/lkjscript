use super::*;

#[test]
fn scalar_call_metadata_rejects_wrong_argument_kind() {
    let mut chunk = unit_chunk();
    let prototype = chunk.add_const(Constant::Proto(0));
    let mut proto = Chunk::new().main;
    proto.name = "i64-parameter".into();
    proto.arity = 1;
    proto.locals = 1;
    proto.parameter_copy_kinds = vec![Some(crate::StructuralKind::I64)];
    proto.code = index_instruction(Op::LoadLocal, 0);
    proto.code.push(Op::Return as u8);
    chunk.protos.push(proto);
    chunk.main.code.clear();
    chunk.main.emit(Op::Unit);
    chunk.main.emit_op_u16(Op::LoadConst, prototype.0);
    chunk.main.emit(Op::MakeClosure);
    chunk.main.emit_u16(0);
    chunk.main.emit_op_u8(Op::Call, 1);
    chunk.main.emit(Op::Return);
    assert!(error(chunk).contains("copy call argument does not match"));
}

#[test]
fn scalar_return_metadata_rejects_wrong_result_kind() {
    let mut chunk = unit_chunk();
    let mut proto = Chunk::new().main;
    proto.name = "i64-return".into();
    proto.return_copy_kind = Some(crate::StructuralKind::I64);
    proto.code = vec![Op::Unit as u8, Op::Return as u8];
    chunk.protos.push(proto);
    assert!(error(chunk).contains("copy return does not match"));
}
