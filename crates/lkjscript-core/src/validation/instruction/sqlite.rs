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
        Op::SysSqliteOpen => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteClose
        | Op::SysSqliteFinalize
        | Op::SysSqliteReset
        | Op::SysSqliteClearBindings
        | Op::SysSqliteBindNull
        | Op::SysSqliteStep
        | Op::SysSqliteColumnCount
        | Op::SysSqliteChanges
        | Op::SysSqliteLastInsertRowid
        | Op::SysSqliteExtendedResultCode => {
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBusyTimeout => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteExec | Op::SysSqlitePrepare => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindI64 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindF64 => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindText => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindBytes => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteColumnType
        | Op::SysSqliteColumnI64
        | Op::SysSqliteColumnF64
        | Op::SysSqliteColumnText
        | Op::SysSqliteColumnBytes => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBackup => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::Handle, proto, instruction)?;
            state.stack.push(result_kind());
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}
