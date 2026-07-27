use super::{system::expect_resource, types::*, Kind, State};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, ResourceKind, Result};

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
            expect_pop(state, Kind::Path, proto, instruction)?;
            expect_pop(
                state,
                Kind::Capability(crate::CapabilityKind::Sqlite),
                proto,
                instruction,
            )?;
            state
                .stack
                .push(resource_result_kind(ResourceKind::SqliteConnection));
        }
        Op::SysSqliteClose
        | Op::SysSqliteChanges
        | Op::SysSqliteLastInsertRowid
        | Op::SysSqliteExtendedResultCode => {
            expect_resource(state, &[ResourceKind::SqliteConnection], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteFinalize
        | Op::SysSqliteReset
        | Op::SysSqliteClearBindings
        | Op::SysSqliteStep
        | Op::SysSqliteColumnCount => {
            expect_resource(state, &[ResourceKind::SqliteStatement], proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindNull => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBusyTimeout => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            connection(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteExec => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            connection(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqlitePrepare => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            connection(state, proto, instruction)?;
            state
                .stack
                .push(resource_result_kind(ResourceKind::SqliteStatement));
        }
        Op::SysSqliteBindI64 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindF64 => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindText => {
            expect_pop(state, Kind::Str, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBindBytes => {
            expect_pop(state, Kind::Buf, proto, instruction)?;
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteColumnType
        | Op::SysSqliteColumnI64
        | Op::SysSqliteColumnF64
        | Op::SysSqliteColumnText
        | Op::SysSqliteColumnBytes => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement(state, proto, instruction)?;
            state.stack.push(result_kind());
        }
        Op::SysSqliteBackup => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            expect_pop(state, Kind::Path, proto, instruction)?;
            connection(state, proto, instruction)?;
            expect_pop(
                state,
                Kind::Capability(crate::CapabilityKind::Sqlite),
                proto,
                instruction,
            )?;
            state.stack.push(result_kind());
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

fn connection(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_resource(state, &[ResourceKind::SqliteConnection], proto, instruction)
}

fn statement(
    state: &mut State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    expect_resource(state, &[ResourceKind::SqliteStatement], proto, instruction)
}
