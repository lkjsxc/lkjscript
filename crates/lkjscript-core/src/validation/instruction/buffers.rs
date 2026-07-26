use super::{types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let op = instruction.op();
    match op {
        Op::BufNew => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Buf);
        }
        Op::BufFromStr => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Buf);
        }
        Op::BufToStr => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::PathFromStr => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::PathFromBuf => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::PathToBuf => {
            expect_pop(state, Kind::Path, proto, instruction)?;
            state.stack.push(Kind::Buf);
        }
        Op::PathToStr => {
            expect_pop(state, Kind::Path, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::BufSlice => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::BufLen | Op::BufClone => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(if op == Op::BufLen {
                Kind::I64
            } else {
                Kind::Buf
            });
        }
        Op::BufRef | Op::BufGetU32 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::BufSet | Op::BufSetU32 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::StrLen => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::StrRef => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::StrAppend => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::StrSlice => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::StrFromByte | Op::StrFromI64 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        Op::StrFromF64 => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            state.stack.push(Kind::Str);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
