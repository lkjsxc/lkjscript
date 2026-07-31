use super::{system_types::*, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, ResourceKind, Result};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::SysTtyGet
        | Op::SysTtySet
        | Op::SysPoll
        | Op::SysIsatty
        | Op::SysClose
        | Op::ResourceDrop
        | Op::SysTtyGuardSave
        | Op::SysTtyGuardClear
        | Op::SysNowMs
        | Op::SysWaitMs
        | Op::SysRandomFill
        | Op::SysSha256 => apply_terminal(chunk, proto, instruction, state),
        Op::SysReadByte
        | Op::SysOpenRead
        | Op::SysOpenWrite
        | Op::SysOpenCreateNew
        | Op::SysOpenAppend
        | Op::SysOpenDir
        | Op::SysPathExists
        | Op::SysFsync
        | Op::SysWriteByte
        | Op::SysTruncate
        | Op::SysReadInto
        | Op::SysWriteFrom
        | Op::SysRename => apply_io(chunk, proto, instruction, state),
        Op::SysAccept | Op::SysRecv | Op::SysSocket | Op::SysBind | Op::SysListen | Op::SysSend => {
            apply_network(chunk, proto, instruction, state)
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
}

include!("terminal.rs");
include!("io.rs");
include!("network.rs");
