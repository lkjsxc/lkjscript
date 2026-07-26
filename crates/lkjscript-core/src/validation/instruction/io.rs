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
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Flush => {
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::ReadByte => {
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::I64);
        }
        Op::WriteByte => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Exit => {
            expect_pop(state, Kind::I64, proto, instruction)?;
        }
        Op::WriteStr => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Stdio, proto, instruction)?;
            state.stack.push(Kind::Unit);
        }
        Op::Arg => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Arguments, proto, instruction)?;
            state.stack.push(option_kind());
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

fn expect_capability(
    state: &mut State,
    kind: crate::CapabilityKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_pop(state, Kind::Capability(kind), proto, instruction)
}
