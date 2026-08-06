use super::types::{expect_pop, pop};
use super::unique::support::error;
use super::{Kind, OwnerIdentity, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

mod places;
use places::*;

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::BytesLength => {
            pop_bytes(state, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::BytesByteAt => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_bytes(state, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::CopyBytesSlice => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_bytes(state, proto, instruction)?;
            state.stack.push(Kind::Bytes(new_owner(instruction)?));
        }
        Op::CloneBytes => {
            pop_bytes(state, proto, instruction)?;
            state.stack.push(Kind::Bytes(new_owner(instruction)?));
        }
        Op::FreezeByteVector => {
            let Kind::ByteVector(owner) = pop(state, proto, instruction)? else {
                return Err(error(proto, instruction, "freeze expects byte-vector"));
            };
            reject_owner_views(state, owner, proto, instruction)?;
            state.stack.push(Kind::Bytes(owner));
        }
        Op::ThawBytes => {
            let value = pop_bytes(state, proto, instruction)?;
            let owner = match value {
                Kind::Bytes(owner) => owner,
                Kind::StaticBytes => new_owner(instruction)?,
                Kind::BytesBorrow { .. } => {
                    return Err(error(
                        proto,
                        instruction,
                        "thaw cannot consume a bytes borrow",
                    ))
                }
                _ => unreachable!("bytes kind checked"),
            };
            state.stack.push(Kind::ByteVector(owner));
        }
        Op::BytesPlaceInit => place_init(proto, instruction, state)?,
        Op::BytesMove => move_owner(proto, instruction, state)?,
        Op::BytesBorrow => borrow(proto, instruction, state)?,
        Op::BytesDropPlace => drop_owner(proto, instruction, state)?,
        Op::BytesPlaceEnd => place_end(proto, instruction, state)?,
        _ => unreachable!("bytes validation opcode family checked"),
    }
    Ok(())
}

fn pop_bytes(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    let value = pop(state, proto, instruction)?;
    if matches!(
        value,
        Kind::StaticBytes | Kind::Bytes(_) | Kind::BytesBorrow { .. }
    ) {
        Ok(value)
    } else {
        Err(error(proto, instruction, "operation expects exact bytes"))
    }
}

pub(super) const fn new_owner(instruction: DecodedInstruction) -> Result<OwnerIdentity> {
    Ok(OwnerIdentity::instruction(instruction.offset(), 1))
}
