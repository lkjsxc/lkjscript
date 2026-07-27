use super::{instruction_error, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, ResourceKind, Result};

pub(super) fn apply(
    _chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    use ResourceKind::{
        Directory, FileAppender, FileReader, FileWriter, InputStream, OutputStream, TcpListener,
        TcpStream,
    };
    let op = instruction.op();
    match op {
        Op::SysTtyGet | Op::SysTtySet => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_resource(state, &[InputStream], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysPoll => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(
                state,
                &[InputStream, FileReader, TcpListener, TcpStream],
                proto,
                instruction,
            )?;
            state.stack.push(result_kind());
        }
        Op::SysIsatty => {
            expect_resource(state, &[InputStream], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysClose => {
            expect_resource(
                state,
                &[
                    OutputStream,
                    FileReader,
                    FileWriter,
                    FileAppender,
                    Directory,
                    TcpListener,
                    TcpStream,
                    ResourceKind::SqliteConnection,
                    ResourceKind::SqliteStatement,
                    ResourceKind::TerminalSession,
                ],
                proto,
                instruction,
            )?;
            state.stack.push(result_kind());
        }
        Op::SysReadByte => {
            expect_resource(
                state,
                &[InputStream, FileReader, TcpStream],
                proto,
                instruction,
            )?;
            state.stack.push(result_kind());
        }
        Op::SysAccept => {
            expect_resource(state, &[TcpListener], proto, instruction)?;
            state.stack.push(resource_result_kind(TcpStream));
        }
        Op::SysRecv => {
            expect_resource(state, &[TcpStream], proto, instruction)?;
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
            state.stack.push(resource_result_kind(TcpListener));
        }
        Op::SysOpenRead => file_open(state, proto, instruction, FileReader)?,
        Op::SysOpenWrite | Op::SysOpenCreateNew => {
            file_open(state, proto, instruction, FileWriter)?
        }
        Op::SysOpenAppend => file_open(state, proto, instruction, FileAppender)?,
        Op::SysOpenDir => file_open(state, proto, instruction, Directory)?,
        Op::SysPathExists => {
            expect_pop(state, Kind::Path, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysFsync => {
            expect_resource(
                state,
                &[FileWriter, FileAppender, Directory],
                proto,
                instruction,
            )?;
            state.stack.push(result_kind());
        }
        Op::SysWriteByte => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(
                state,
                &[OutputStream, FileWriter, FileAppender, TcpStream],
                proto,
                instruction,
            )?;
            state.stack.push(result_kind());
        }
        Op::SysBind | Op::SysListen => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(state, &[TcpListener], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysTruncate => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_resource(state, &[FileWriter, FileAppender], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysWaitMs => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::Clock, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSend => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_resource(state, &[TcpStream], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysReadInto | Op::SysWriteFrom => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Buf, proto, instruction)?;
            let kinds = if op == Op::SysReadInto {
                &[InputStream, FileReader, TcpStream][..]
            } else {
                &[OutputStream, FileWriter, FileAppender, TcpStream][..]
            };
            expect_resource(state, kinds, proto, instruction)?;
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
            expect_pop(state, Kind::Path, proto, instruction)?;
            expect_pop(state, Kind::Path, proto, instruction)?;
            expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
            state.stack.push(result_kind());
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

fn file_open(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    kind: ResourceKind,
) -> Result<()> {
    expect_pop(state, Kind::Path, proto, instruction)?;
    expect_capability(state, crate::CapabilityKind::FileSystem, proto, instruction)?;
    state.stack.push(resource_result_kind(kind));
    Ok(())
}

pub(super) fn expect_resource(
    state: &mut State,
    allowed: &[ResourceKind],
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let actual = pop(state, proto, instruction)?;
    if matches!(actual, Kind::Resource(kind) if allowed.contains(&kind)) {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            &format!("typed resource kind mismatch: got {actual}"),
        ))
    }
}

fn expect_capability(
    state: &mut State,
    kind: crate::CapabilityKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_pop(state, Kind::Capability(kind), proto, instruction)
}
