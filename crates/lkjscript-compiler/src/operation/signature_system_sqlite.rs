use crate::operation::instantiation::function;
use crate::operation::*;
use lkjscript_core::ResourceKind;

pub(super) fn sqlite_signature(operation: Operation) -> Option<Type> {
    use lkjscript_core::CapabilityKind::Sqlite;

    let system_result =
        |success| crate::types::result_type(success, crate::types::system_error_type());
    let resource = |kind| Type::Resource(kind);
    let signature = match operation {
        Operation::SysSqliteOpen => function(
            vec![Type::Capability(Sqlite), Type::Path, Type::I64],
            system_result(resource(ResourceKind::SqliteConnection)),
        ),
        Operation::SysSqliteClose => function(
            vec![resource(ResourceKind::SqliteConnection)],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteFinalize
        | Operation::SysSqliteReset
        | Operation::SysSqliteClearBindings => function(
            vec![resource(ResourceKind::SqliteStatement)],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBusyTimeout => function(
            vec![resource(ResourceKind::SqliteConnection), Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteBindNull => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(Type::Unit),
        ),
        Operation::SysSqliteExec => function(
            vec![resource(ResourceKind::SqliteConnection), Type::Str],
            system_result(Type::Unit),
        ),
        Operation::SysSqlitePrepare => function(
            vec![resource(ResourceKind::SqliteConnection), Type::Str],
            system_result(resource(ResourceKind::SqliteStatement)),
        ),
        Operation::SysSqliteBindI64 => sqlite_bind(Type::I64),
        Operation::SysSqliteBindF64 => sqlite_bind(Type::F64),
        Operation::SysSqliteBindText => sqlite_bind(Type::Str),
        Operation::SysSqliteBindBytes => sqlite_bind(Type::ByteSlice),
        Operation::SysSqliteStep | Operation::SysSqliteColumnCount => function(
            vec![resource(ResourceKind::SqliteStatement)],
            system_result(Type::I64),
        ),
        Operation::SysSqliteChanges
        | Operation::SysSqliteLastInsertRowid
        | Operation::SysSqliteExtendedResultCode => function(
            vec![resource(ResourceKind::SqliteConnection)],
            system_result(Type::I64),
        ),
        Operation::SysSqliteColumnType => function(
            vec![resource(ResourceKind::SqliteStatement), Type::I64],
            system_result(Type::I64),
        ),
        Operation::SysSqliteColumnI64 => sqlite_column(Type::I64),
        Operation::SysSqliteColumnF64 => sqlite_column(Type::F64),
        Operation::SysSqliteColumnText => sqlite_column(Type::Str),
        Operation::SysSqliteColumnBytes => sqlite_column(Type::Bytes),
        Operation::SysSqliteBackup => function(
            vec![
                Type::Capability(Sqlite),
                resource(ResourceKind::SqliteConnection),
                Type::Path,
                Type::I64,
            ],
            system_result(Type::Unit),
        ),
        _ => return None,
    };
    Some(signature)
}

fn sqlite_bind(value: Type) -> Type {
    function(
        vec![
            Type::Resource(ResourceKind::SqliteStatement),
            Type::I64,
            value,
        ],
        crate::types::result_type(Type::Unit, crate::types::system_error_type()),
    )
}

fn sqlite_column(value: Type) -> Type {
    function(
        vec![Type::Resource(ResourceKind::SqliteStatement), Type::I64],
        crate::types::result_type(
            crate::types::option_type(value),
            crate::types::system_error_type(),
        ),
    )
}
