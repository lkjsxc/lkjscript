use super::{instruction_error, types::*, Kind, OwnerIdentity, State};
use crate::{Chunk, DecodedInstruction, Error, FunctionProto, Op, Result, StructuralSliceExt};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::ConvertStringToBytes => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            state.stack.push(Kind::Bytes(new_owner(instruction)?));
        }
        Op::ConvertBytesToString => {
            pop_bytes(state, proto, instruction)?;
            state.stack.push(result_owner(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        Op::PathFromStr => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            state.stack.push(result_owner(
                chunk,
                crate::StructuralKind::Path,
                proto,
                instruction,
            )?);
        }
        Op::PathFromBytes => {
            pop_bytes(state, proto, instruction)?;
            state.stack.push(result_owner(
                chunk,
                crate::StructuralKind::Path,
                proto,
                instruction,
            )?);
        }
        Op::PathToBytes => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::Path,
                Kind::Path,
                proto,
                instruction,
            )?;
            state.stack.push(Kind::Bytes(new_owner(instruction)?));
        }
        Op::PathToStr => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::Path,
                Kind::Path,
                proto,
                instruction,
            )?;
            state.stack.push(result_owner(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        Op::StrLen => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            state.stack.push(Kind::I64);
        }
        Op::StrRef => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            state.stack.push(Kind::I64);
        }
        Op::StrAppend => {
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            state.stack.push(direct_owner(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        Op::StrSlice => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_structural_leaf(
                chunk,
                state,
                crate::StructuralKind::String,
                Kind::Str,
                proto,
                instruction,
            )?;
            state.stack.push(direct_owner(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        Op::StrFromByte | Op::StrFromI64 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(direct_owner(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        Op::StrFromF64 => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            state.stack.push(direct_owner(
                chunk,
                crate::StructuralKind::String,
                proto,
                instruction,
            )?);
        }
        _ => unreachable!("opcode dispatched to wrong byte-data validation family"),
    }
    Ok(())
}

include!("helpers.rs");
include!("result.rs");
