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
        Op::SysTtyGet | Op::SysTtySet => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysPoll => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysIsatty | Op::SysClose | Op::SysReadByte | Op::SysAccept | Op::SysRecv => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysTtyGuardSave => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysTtyGuardClear | Op::SysNowMs | Op::SysSocket => state.stack.push(Kind::Result),
        Op::SysOpenRead
        | Op::SysOpenWrite
        | Op::SysOpenAppend
        | Op::SysOpenCreateNew
        | Op::SysOpenDir
        | Op::SysPathExists => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysFsync => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysWriteByte | Op::SysBind | Op::SysListen | Op::SysTruncate => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysWaitMs => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysSend => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysReadInto | Op::SysWriteFrom => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysRandomFill | Op::SysSha256 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        Op::SysRename => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(Kind::Result);
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
