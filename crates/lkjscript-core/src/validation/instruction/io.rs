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
        Op::Print => {
            let _value = pop(state, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Flush => state.stack.push(Kind::Unit),
        Op::ReadByte => state.stack.push(Kind::I64),
        Op::WriteByte => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Exit => {
            expect_pop(state, Kind::I64, proto, instruction)?;
        }
        Op::WriteStr => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Arg => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Option);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
