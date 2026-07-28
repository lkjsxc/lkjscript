use super::{Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

mod owners;
mod release;
mod support;
mod views;

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::ByteVectorNew
        | Op::ByteVectorPlaceInit
        | Op::ByteVectorMove
        | Op::ByteVectorBorrow
        | Op::ByteVectorBorrowMut
        | Op::StoreUniqueLocal
        | Op::TakeUniqueLocal => owners::apply(chunk, proto, instruction, state),
        Op::ByteVectorDropPlace | Op::ByteVectorPlaceEnd => {
            release::apply(chunk, proto, instruction, state)
        }
        Op::StoreViewLocal
        | Op::LoadViewLocal
        | Op::ByteSliceLen
        | Op::ByteSliceRef
        | Op::ByteSliceMutSet
        | Op::EndBorrowLocal => views::apply(chunk, proto, instruction, state),
        _ => unreachable!("opcode dispatched to wrong unique validation family"),
    }
}
