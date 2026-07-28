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
            vm.ensure_host_deadline_support("open-sqlite", false)?;
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
            vm.ensure_host_deadline_support("close-sqlite", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_close(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBusyTimeout as u8 => {
            vm.ensure_host_deadline_support("set-sqlite-busy-timeout", false)?;
            let milliseconds = vm.pop()?;
            let milliseconds = vm.as_i64(milliseconds)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_busy_timeout(handle, milliseconds);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteExec as u8 => {
            vm.ensure_host_deadline_support("execute-sqlite", false)?;
            let sql = vm.pop()?;
            let sql = crate::host_ext::as_str(&vm.arena, sql)?.to_string();
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_exec(handle, &sql);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqlitePrepare as u8 => {
            vm.ensure_host_deadline_support("prepare-sqlite", false)?;
            let sql = vm.pop()?;
            let sql = crate::host_ext::as_str(&vm.arena, sql)?.to_string();
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_prepare(handle, &sql);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteFinalize as u8 => {
            vm.ensure_host_deadline_support("finalize-sqlite-statement", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_finalize(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteReset as u8 => {
            vm.ensure_host_deadline_support("reset-sqlite-statement", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_reset(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteClearBindings as u8 => {
            vm.ensure_host_deadline_support("clear-sqlite-bindings", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_clear_bindings(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindNull as u8 => {
            vm.ensure_host_deadline_support("bind-sqlite-null", false)?;
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_null(handle, index);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindI64 as u8 => {
            vm.ensure_host_deadline_support("bind-sqlite-i64", false)?;
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
            vm.ensure_host_deadline_support("bind-sqlite-f64", false)?;
            let value = vm.pop()?;
            let value = vm
                .as_f64(value)
                .map_err(|_| lkjscript_core::Error::msg("sys-sqlite-bind-f64: expected F64"))?;
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_bind_f64(handle, index, value);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBindText as u8 => {
            vm.ensure_host_deadline_support("bind-sqlite-string", false)?;
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
            vm.ensure_host_deadline_support("bind-sqlite-bytes", false)?;
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
            vm.ensure_host_deadline_support("step-sqlite", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_step(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBackup as u8 => {
            vm.ensure_host_deadline_support("backup-sqlite", false)?;
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
