use super::*;

fn push_language_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<Value>) {
    super::push_language_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
}

fn push_i64_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<i64>) {
    super::push_i64_result(vm, lkjscript_core::SystemErrorKind::Sqlite, result);
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::SysSqliteOpen as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-open", false)?;
            let flags = vm.pop()?;
            let flags = vm.as_i64(flags)?;
            let path = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Sqlite)?;
            let path = crate::host_ext::as_path(&vm.arena, path)?;
            let result = vm.resources.sqlite_open(path, flags);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteClose as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-close", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_close(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBusyTimeout as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-busy-timeout", false)?;
            let milliseconds = vm.pop()?;
            let milliseconds = vm.as_i64(milliseconds)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_busy_timeout(handle, milliseconds);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteExec as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-exec", false)?;
            let sql = vm.pop()?;
            let sql = crate::host_ext::as_str(&vm.arena, sql)?.to_string();
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_exec(handle, &sql);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqlitePrepare as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-prepare", false)?;
            let sql = vm.pop()?;
            let sql = crate::host_ext::as_str(&vm.arena, sql)?.to_string();
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_prepare(handle, &sql);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteFinalize as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-finalize", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_finalize(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteReset as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-reset", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_reset(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteClearBindings as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-clear-bindings", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_clear_bindings(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindNull as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-bind-null", false)?;
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_null(handle, index);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindI64 as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-bind-i64", false)?;
            let value = vm.pop()?;
            let value = vm.as_i64(value)?;
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_i64(handle, index, value);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindF64 as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-bind-f64", false)?;
            let value = vm.pop()?;
            let value = match vm.arena.get(value)? {
                HeapObj::Float(value) => *value,
                _ => {
                    return Err(lkjscript_core::Error::msg(
                        "sys-sqlite-bind-f64: expected F64",
                    ))
                }
            };
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_f64(handle, index, value);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindText as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-bind-text", false)?;
            let value = vm.pop()?;
            let value = crate::host_ext::as_str(&vm.arena, value)?.to_string();
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_text(handle, index, &value);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindBytes as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-bind-bytes", false)?;
            let value = vm.pop()?;
            let value = crate::host_buf::as_buf(&vm.arena, value)?.to_vec();
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_bytes(handle, index, &value);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteStep as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-step", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_step(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBackup as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-backup", false)?;
            let flags = vm.pop()?;
            let flags = vm.as_i64(flags)?;
            let path = vm.pop()?;
            let handle = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Sqlite)?;
            let path = crate::host_ext::as_path(&vm.arena, path)?;
            let result = vm.resources.sqlite_backup(handle, path, flags);
            push_language_result(vm, result);
            Ok(true)
        }
        _ => Ok(false),
    }
}
