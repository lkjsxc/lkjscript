use crate::{RuntimeOp, SsaType};
use lkjscript_contracts::{CapabilityKind, ResourceKind};

pub(super) fn sqlite_signature(
    operation: RuntimeOp,
    parameters: &[SsaType],
    result: &SsaType,
) -> Option<bool> {
    let resource = SsaType::Resource;
    let exact = |expected: &[SsaType], result_type: &SsaType| {
        parameters == expected && result == result_type
    };
    let valid = match operation {
        RuntimeOp::SysSqliteOpen => exact(
            &[
                SsaType::Capability(CapabilityKind::Sqlite),
                SsaType::Path,
                SsaType::I64,
            ],
            &super::system_result(resource(ResourceKind::SqliteConnection)),
        ),
        RuntimeOp::SysSqliteClose => {
            sqlite_one(ResourceKind::SqliteConnection, result, SsaType::Unit)
        }
        RuntimeOp::SysSqliteFinalize
        | RuntimeOp::SysSqliteReset
        | RuntimeOp::SysSqliteClearBindings => {
            sqlite_one(ResourceKind::SqliteStatement, result, SsaType::Unit)
        }
        RuntimeOp::SysSqliteBusyTimeout => sqlite_tail(
            ResourceKind::SqliteConnection,
            &[SsaType::I64],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteBindNull => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqliteExec => sqlite_tail(
            ResourceKind::SqliteConnection,
            &[SsaType::Str],
            parameters,
            result,
            SsaType::Unit,
        ),
        RuntimeOp::SysSqlitePrepare => sqlite_tail(
            ResourceKind::SqliteConnection,
            &[SsaType::Str],
            parameters,
            result,
            resource(ResourceKind::SqliteStatement),
        ),
        RuntimeOp::SysSqliteBindI64 => sqlite_bind(SsaType::I64, parameters, result),
        RuntimeOp::SysSqliteBindF64 => sqlite_bind(SsaType::F64, parameters, result),
        RuntimeOp::SysSqliteBindText => sqlite_bind(SsaType::Str, parameters, result),
        RuntimeOp::SysSqliteBindBytes => sqlite_bind(SsaType::Buf, parameters, result),
        RuntimeOp::SysSqliteStep | RuntimeOp::SysSqliteColumnCount => {
            sqlite_one(ResourceKind::SqliteStatement, result, SsaType::I64)
        }
        RuntimeOp::SysSqliteChanges
        | RuntimeOp::SysSqliteLastInsertRowid
        | RuntimeOp::SysSqliteExtendedResultCode => {
            sqlite_one(ResourceKind::SqliteConnection, result, SsaType::I64)
        }
        RuntimeOp::SysSqliteColumnType => sqlite_tail(
            ResourceKind::SqliteStatement,
            &[SsaType::I64],
            parameters,
            result,
            SsaType::I64,
        ),
        RuntimeOp::SysSqliteColumnI64 => sqlite_column(SsaType::I64, parameters, result),
        RuntimeOp::SysSqliteColumnF64 => sqlite_column(SsaType::F64, parameters, result),
        RuntimeOp::SysSqliteColumnText => sqlite_column(SsaType::Str, parameters, result),
        RuntimeOp::SysSqliteColumnBytes => sqlite_column(SsaType::Buf, parameters, result),
        RuntimeOp::SysSqliteBackup => exact(
            &[
                SsaType::Capability(CapabilityKind::Sqlite),
                resource(ResourceKind::SqliteConnection),
                SsaType::Path,
                SsaType::I64,
            ],
            &super::system_result(SsaType::Unit),
        ),
        _ => return None,
    };
    Some(valid)
}

fn sqlite_bind(value: SsaType, parameters: &[SsaType], result: &SsaType) -> bool {
    sqlite_tail(
        ResourceKind::SqliteStatement,
        &[SsaType::I64, value],
        parameters,
        result,
        SsaType::Unit,
    )
}

fn sqlite_column(value: SsaType, parameters: &[SsaType], result: &SsaType) -> bool {
    sqlite_tail(
        ResourceKind::SqliteStatement,
        &[SsaType::I64],
        parameters,
        result,
        crate::prelude_contract::option(value),
    )
}

fn sqlite_one(kind: ResourceKind, result: &SsaType, output: SsaType) -> bool {
    sqlite_tail(kind, &[], &[SsaType::Resource(kind)], result, output)
}

fn sqlite_tail(
    kind: ResourceKind,
    tail: &[SsaType],
    parameters: &[SsaType],
    result: &SsaType,
    output: SsaType,
) -> bool {
    parameters.first() == Some(&SsaType::Resource(kind))
        && parameters.get(1..) == Some(tail)
        && result == &super::system_result(output)
}
