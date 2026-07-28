use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::SysSqliteColumnCount as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_column_count(handle);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnType as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_column_type(handle, index);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnI64 as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result =
                vm.resources
                    .sqlite_column_i64(handle, index)
                    .and_then(|value| match value {
                        Some(value) => {
                            let value = Value::from_i64(value);
                            crate::host_ext::option_some(&mut vm.arena, value)
                        }
                        None => crate::host_ext::option_none(&mut vm.arena),
                    });
            push_language_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnF64 as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result =
                vm.resources
                    .sqlite_column_f64(handle, index)
                    .and_then(|value| match value {
                        Some(value) => {
                            let value = Value::from_f64_bits(value.to_bits());
                            crate::host_ext::option_some(&mut vm.arena, value)
                        }
                        None => crate::host_ext::option_none(&mut vm.arena),
                    });
            push_language_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnText as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm
                .resources
                .sqlite_column_text(handle, index, lkjscript_core::MAX_BUFFER_BYTES)
                .and_then(|value| match value {
                    Some(value) => {
                        let value = vm.arena.alloc(HeapObj::Str(value))?;
                        crate::host_ext::option_some(&mut vm.arena, value)
                    }
                    None => crate::host_ext::option_none(&mut vm.arena),
                });
            push_language_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnBytes as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm
                .resources
                .sqlite_column_bytes(handle, index, lkjscript_core::MAX_BUFFER_BYTES)
                .and_then(|value| match value {
                    Some(value) => {
                        let value = vm.arena.alloc(HeapObj::Buf(value))?;
                        crate::host_ext::option_some(&mut vm.arena, value)
                    }
                    None => crate::host_ext::option_none(&mut vm.arena),
                });
            push_language_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteChanges as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_changes(handle);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteLastInsertRowid as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_last_insert_rowid(handle);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        x if x == Op::SysSqliteExtendedResultCode as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_extended_result_code(handle);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
            Ok(true)
        }
        _ => Ok(false),
    }
}
