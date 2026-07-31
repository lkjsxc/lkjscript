use super::{
    system_types::{
        consume_resource_owner, expect_resource, structural_option_result,
        structural_resource_result, structural_value_result,
    },
    types::*,
    Kind, State,
};
use crate::{Chunk, DecodedInstruction, FunctionProto, Op, ResourceKind, Result, StructuralKind};

pub(super) fn apply(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    match instruction.op() {
        Op::SysSqliteOpen => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_leaf(
                chunk,
                state,
                proto,
                instruction,
                StructuralKind::Path,
                Kind::Path,
            )?;
            expect_pop(
                state,
                Kind::Capability(crate::CapabilityKind::Sqlite),
                proto,
                instruction,
            )?;
            state.stack.push(structural_resource_result(
                chunk,
                ResourceKind::SqliteConnection,
                proto,
                instruction,
            )?);
        }
        Op::SysSqliteClose => {
            let resource = connection(state, proto, instruction)?;
            consume_resource_owner(state, resource, proto, instruction)?;
            push_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        Op::SysSqliteChanges | Op::SysSqliteLastInsertRowid | Op::SysSqliteExtendedResultCode => {
            connection_result(chunk, state, proto, instruction, StructuralKind::I64)?;
        }
        Op::SysSqliteFinalize => {
            let resource = statement(state, proto, instruction)?;
            consume_resource_owner(state, resource, proto, instruction)?;
            push_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        Op::SysSqliteReset | Op::SysSqliteClearBindings => {
            statement_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        Op::SysSqliteStep => {
            statement_result(chunk, state, proto, instruction, StructuralKind::I64)?;
        }
        Op::SysSqliteColumnCount => {
            statement_result(chunk, state, proto, instruction, StructuralKind::I64)?;
        }
        Op::SysSqliteBindNull => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            statement_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        Op::SysSqliteBusyTimeout => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            connection_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        Op::SysSqliteExec => {
            pop_leaf(
                chunk,
                state,
                proto,
                instruction,
                StructuralKind::String,
                Kind::Str,
            )?;
            connection_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        Op::SysSqlitePrepare => {
            pop_leaf(
                chunk,
                state,
                proto,
                instruction,
                StructuralKind::String,
                Kind::Str,
            )?;
            connection(state, proto, instruction)?;
            state.stack.push(structural_resource_result(
                chunk,
                ResourceKind::SqliteStatement,
                proto,
                instruction,
            )?);
        }
        Op::SysSqliteBindI64 => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            bind_result(chunk, state, proto, instruction)?;
        }
        Op::SysSqliteBindF64 => {
            expect_pop(state, Kind::F64, proto, instruction)?;
            bind_result(chunk, state, proto, instruction)?;
        }
        Op::SysSqliteBindText => {
            pop_leaf(
                chunk,
                state,
                proto,
                instruction,
                StructuralKind::String,
                Kind::Str,
            )?;
            bind_result(chunk, state, proto, instruction)?;
        }
        Op::SysSqliteBindBytes => {
            super::unique::pop_used_view(state, false, proto, instruction)?;
            bind_result(chunk, state, proto, instruction)?;
        }
        Op::SysSqliteColumnType => {
            column_result(chunk, state, proto, instruction, StructuralKind::I64)?
        }
        Op::SysSqliteColumnI64 => {
            column_option_result(chunk, state, proto, instruction, StructuralKind::I64)?;
        }
        Op::SysSqliteColumnF64 => {
            column_option_result(chunk, state, proto, instruction, StructuralKind::F64)?;
        }
        Op::SysSqliteColumnText => {
            column_option_result(chunk, state, proto, instruction, StructuralKind::String)?;
        }
        Op::SysSqliteColumnBytes => {
            column_option_result(chunk, state, proto, instruction, StructuralKind::Bytes)?;
        }
        Op::SysSqliteBackup => {
            expect_pop(state, Kind::I64, proto, instruction)?;
            pop_leaf(
                chunk,
                state,
                proto,
                instruction,
                StructuralKind::Path,
                Kind::Path,
            )?;
            connection(state, proto, instruction)?;
            expect_pop(
                state,
                Kind::Capability(crate::CapabilityKind::Sqlite),
                proto,
                instruction,
            )?;
            push_result(chunk, state, proto, instruction, StructuralKind::Unit)?;
        }
        _ => unreachable!("opcode dispatched to wrong validation family"),
    }
    Ok(())
}

include!("helpers.rs");
