use super::*;
use crate::{FailureCleanupAction, FailureCleanupNode, UniqueValueKind};

const WIDE_COUNT: usize = 300;

fn encoded(value: usize) -> u64 {
    u64::try_from(value).expect("test width fits u64")
}

fn high_call_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    let prototype = chunk
        .add_const(Constant::Proto(0))
        .expect("add prototype constant");
    let mut callee = Chunk::new().main;
    callee.name = "wide-callee".into();
    callee.arity = WIDE_COUNT;
    callee.locals = WIDE_COUNT;
    callee.emit_op_u64(Op::LoadLocal, encoded(WIDE_COUNT - 1));
    callee.emit(Op::Return);
    chunk.protos.push(callee);

    for _ in 0..WIDE_COUNT {
        chunk.main.emit(Op::Unit);
    }
    chunk.main.emit_op_u64(Op::LoadConst, prototype.0);
    chunk.main.emit_op_u64(Op::MakeClosure, 0);
    chunk.main.emit_op_u64(Op::Call, encoded(WIDE_COUNT));
    chunk.main.emit(Op::Return);
    chunk
}

fn high_place_chunk() -> Chunk {
    let mut chunk = Chunk::new();
    chunk.main.locals = WIDE_COUNT;
    chunk.main.unique_places = WIDE_COUNT;
    let size = chunk
        .add_const(Constant::I64(1))
        .expect("add size constant");
    chunk.main.emit_op_u64(Op::LoadConst, size.0);
    chunk.main.emit(Op::ByteVectorNew);
    chunk
        .main
        .emit_op_u64(Op::StoreUniqueLocal, encoded(WIDE_COUNT - 1));
    chunk.main.emit_op_u64_pair(
        Op::ByteVectorPlaceInit,
        encoded(WIDE_COUNT - 1),
        encoded(WIDE_COUNT - 1),
    );
    chunk.main.emit(Op::Pop);
    chunk.main.emit_op_u64_pair(
        Op::ByteVectorDropPlace,
        encoded(WIDE_COUNT - 1),
        encoded(WIDE_COUNT - 1),
    );
    chunk.main.emit(Op::Pop);
    chunk
        .main
        .emit_op_u64(Op::ByteVectorPlaceEnd, encoded(WIDE_COUNT - 1));
    chunk.main.emit(Op::Pop);
    chunk.main.emit(Op::Unit);
    chunk.main.emit(Op::Return);
    chunk
}

#[test]
fn high_local_call_and_place_operands_validate_in_range_and_reject_equal_count() {
    let call = validate_chunk(high_call_chunk(), ValidationPolicy::Unrestricted)
        .expect("300-argument call validates");
    assert!(call
        .main_instructions()
        .iter()
        .any(|instruction| instruction.op() == Op::Call
            && instruction.operand().index() == Some(WIDE_COUNT)));

    let mut bad_local = high_call_chunk();
    bad_local.protos[0].code[1..9].copy_from_slice(&encoded(WIDE_COUNT).to_le_bytes());
    assert!(error(bad_local).contains("local index out of range"));

    let mut bad_argc = high_call_chunk();
    let validated = validate_chunk(bad_argc.clone(), ValidationPolicy::Unrestricted)
        .expect("locate validated call");
    let call_offset = validated
        .main_instructions()
        .iter()
        .find(|instruction| instruction.op() == Op::Call)
        .expect("call instruction")
        .offset();
    bad_argc.main.code[call_offset + 1..call_offset + 9]
        .copy_from_slice(&encoded(WIDE_COUNT + 1).to_le_bytes());
    assert!(error(bad_argc).contains("stack underflow"));

    validate_chunk(high_place_chunk(), ValidationPolicy::Unrestricted)
        .expect("high place/local pair validates");
    let mut bad_place = high_place_chunk();
    let validated = validate_chunk(bad_place.clone(), ValidationPolicy::Unrestricted)
        .expect("locate high place instruction");
    let place_offset = validated
        .main_instructions()
        .iter()
        .find(|instruction| instruction.op() == Op::ByteVectorPlaceInit)
        .expect("place init")
        .offset();
    bad_place.main.code[place_offset + 1..place_offset + 9]
        .copy_from_slice(&encoded(WIDE_COUNT).to_le_bytes());
    assert!(error(bad_place).contains("unique place index out of range"));
}

#[test]
fn truncated_wide_operands_and_invalid_wide_jumps_fail_closed() {
    let mut truncated_constant = unit_chunk();
    truncated_constant.main.code = vec![Op::LoadConst as u8, 0, 0, 0, 0];
    assert!(error(truncated_constant).contains("truncated LoadConst operand"));

    let mut out_of_range_constant = unit_chunk();
    out_of_range_constant.main.code.clear();
    out_of_range_constant
        .main
        .emit_op_u64(Op::LoadConst, 65_536);
    assert!(error(out_of_range_constant).contains("constant index out of range"));

    let mut truncated_index = unit_chunk();
    truncated_index.main.code = vec![Op::LoadLocal as u8, 0, 0, 0, 0];
    assert!(error(truncated_index).contains("truncated LoadLocal operand"));

    let mut truncated_jump = unit_chunk();
    truncated_jump.main.code = vec![Op::Jump as u8, 0, 0, 0, 0];
    assert!(error(truncated_jump).contains("truncated Jump operand"));

    let mut truncated_pair = unit_chunk();
    truncated_pair.main.code = vec![Op::ByteVectorPlaceInit as u8; 16];
    truncated_pair.main.code[0] = Op::ByteVectorPlaceInit as u8;
    assert!(error(truncated_pair).contains("truncated ByteVectorPlaceInit operand"));

    let mut jump = Chunk::new();
    jump.main.locals = 1;
    jump.main.emit(Op::Unit);
    jump.main.emit_op_u64(Op::StoreLocal, 0);
    jump.main.emit(Op::Pop);
    jump.main.emit_op_u64(Op::Jump, 2);
    jump.main.emit(Op::Unit);
    jump.main.emit(Op::Return);
    assert!(error(jump).contains("not an instruction boundary"));

    let mut out_of_range = unit_chunk();
    out_of_range.main.code.clear();
    out_of_range.main.emit_op_u64(Op::Jump, 65_536);
    assert!(error(out_of_range).contains("out of range"));
}

#[test]
fn high_cleanup_local_and_place_metadata_are_checked_without_byte_narrowing() {
    let mut chunk = unit_chunk();
    chunk.main.locals = WIDE_COUNT;
    chunk.main.unique_places = WIDE_COUNT;
    chunk.main.failure_cleanups = vec![FailureCleanupNode {
        action: FailureCleanupAction::DropUnique {
            local: WIDE_COUNT - 1,
            place: Some(WIDE_COUNT - 1),
            kind: UniqueValueKind::ByteVector,
        },
        next: None,
    }];
    validate_chunk(chunk.clone(), ValidationPolicy::Unrestricted)
        .expect("high cleanup metadata validates in range");

    let mut bad_local = chunk.clone();
    let FailureCleanupAction::DropUnique { local, .. } =
        &mut bad_local.main.failure_cleanups[0].action
    else {
        unreachable!()
    };
    *local = WIDE_COUNT;
    assert!(error(bad_local).contains("local or place is out of range"));

    let FailureCleanupAction::DropUnique { place, .. } = &mut chunk.main.failure_cleanups[0].action
    else {
        unreachable!()
    };
    *place = Some(WIDE_COUNT);
    assert!(error(chunk).contains("local or place is out of range"));
}

#[test]
fn constant_and_global_interning_is_bit_exact_and_insertion_ordered() {
    let mut chunk = Chunk::new();
    let negative_zero = chunk
        .add_const(Constant::F64(f64::from_bits(1_u64 << 63)))
        .expect("intern negative zero");
    let positive_zero = chunk
        .add_const(Constant::F64(0.0))
        .expect("intern positive zero");
    let duplicate_negative_zero = chunk
        .add_const(Constant::F64(f64::from_bits(1_u64 << 63)))
        .expect("intern duplicate negative zero");
    let text = chunk
        .add_const(Constant::Str("owned-key".into()))
        .expect("intern string");
    let duplicate_text = chunk
        .add_const(Constant::Str("owned-key".into()))
        .expect("intern duplicate string");
    let bytes = chunk
        .add_const(Constant::StaticBytes(vec![1, 2, 3].into_boxed_slice()))
        .expect("intern bytes");
    let duplicate_bytes = chunk
        .add_const(Constant::StaticBytes(vec![1, 2, 3].into_boxed_slice()))
        .expect("intern duplicate bytes");
    assert_eq!(negative_zero.0, 0);
    assert_eq!(positive_zero.0, 1);
    assert_eq!(duplicate_negative_zero, negative_zero);
    assert_eq!(duplicate_text, text);
    assert_eq!(duplicate_bytes, bytes);
    assert_eq!(chunk.constants.len(), 4);
    chunk.constants[0] = Constant::I64(99);
    let restored_negative_zero = chunk
        .add_const(Constant::F64(f64::from_bits(1_u64 << 63)))
        .expect("reindex externally changed constants");
    assert_eq!(restored_negative_zero.0, 4);

    let first = chunk.intern_global("first").expect("intern first global");
    let second = chunk.intern_global("second").expect("intern second global");
    let duplicate = chunk
        .intern_global("first")
        .expect("intern duplicate global");
    assert_eq!((first.0, second.0, duplicate.0), (0, 1, 0));
    chunk.global_names[0] = "changed".into();
    let restored = chunk
        .intern_global("first")
        .expect("reindex externally changed globals");
    assert_eq!(restored.0, 2);
    assert_eq!(chunk.global_names, ["changed", "second", "first"]);
}

#[cfg(target_pointer_width = "32")]
#[test]
fn wide_jump_exceeding_host_usize_is_rejected_before_indexing() {
    let mut chunk = unit_chunk();
    chunk.main.code.clear();
    chunk.main.emit_op_u64(Op::Jump, u64::MAX);
    assert!(error(chunk).contains("exceeds host usize"));
}
