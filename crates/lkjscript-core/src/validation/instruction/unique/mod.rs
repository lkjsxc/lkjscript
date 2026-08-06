use super::{Kind, OwnerIdentity, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

mod owners;
mod release;
pub(super) mod support;
mod views;
pub(super) use views::pop_used_view;

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
        | Op::ByteSliceReadU32Le
        | Op::ByteSliceMutWriteU32Le
        | Op::EndBorrowLocal => views::apply(chunk, proto, instruction, state),
        _ => unreachable!("opcode dispatched to wrong unique validation family"),
    }
}
