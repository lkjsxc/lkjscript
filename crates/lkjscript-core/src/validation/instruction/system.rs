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
            state.stack.push(result_kind());
        }
        Op::SysPoll => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysIsatty | Op::SysClose | Op::SysReadByte | Op::SysAccept | Op::SysRecv => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysTtyGuardSave => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Terminal, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysTtyGuardClear => {
            expect_capability(state, crate::CapabilityKind::Terminal, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysNowMs => {
            expect_capability(state, crate::CapabilityKind::Clock, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSocket => {
            expect_capability(state, crate::CapabilityKind::Network, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysOpenRead
        | Op::SysOpenWrite
        | Op::SysOpenAppend
        | Op::SysOpenCreateNew
        | Op::SysOpenDir
        | Op::SysPathExists => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysFsync => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysWriteByte | Op::SysBind | Op::SysListen | Op::SysTruncate => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysWaitMs => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Clock, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSend => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysReadInto | Op::SysWriteFrom => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysRandomFill => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Entropy, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSha256 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysRename => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
            state.stack.push(result_kind());
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
