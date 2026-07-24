//! Dispatch for string and file opcodes.

use lkjscript_core::{HeapObj, Op, Result, Value};

use crate::run::{RuntimeTier, Vm};

fn push_language_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<Value>) {
    let value = crate::host_ext::language_result(&mut vm.arena, result);
    vm.push(value);
}

fn push_i64_result<J: RuntimeTier>(vm: &mut Vm<'_, J>, result: Result<i64>) {
    let result = result.map(|number| vm.make_i64(number));
    push_language_result(vm, result);
}

fn sleep_result(milliseconds: u64) -> Result<Value> {
    lkjscript_sys::sleep_ms(milliseconds)
        .map(|()| Value::UNIT)
        .map_err(|error| lkjscript_core::Error::msg(format!("sys-wait-ms: {error}")))
}

fn wait_readable<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    handle: Value,
    operation: &str,
) -> Result<Option<lkjscript_core::Error>> {
    let Some(timeout) = vm.deadline_timeout_ms()? else {
        return Ok(None);
    };
    match vm.resources.poll_readable(handle, timeout, operation) {
        Ok(true) => Ok(None),
        Ok(false) => Err(lkjscript_core::Error::deadline(format!(
            "execution wall deadline exceeded during {operation}"
        ))),
        Err(error) => Ok(Some(error)),
    }
}

pub fn dispatch_ext<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::StrLen as u8 => {
            let v = vm.pop()?;
            let number = crate::host_ext::str_len(&vm.arena, v)?;
            let value = vm.make_i64(number);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrRef as u8 => {
            let index = vm.pop()?;
            let string = vm.pop()?;
            let index = vm.as_i64(index)?;
            let number = crate::host_ext::str_ref(&vm.arena, string, index)?;
            let value = vm.make_i64(number);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrAppend as u8 => {
            let b = vm.pop()?;
            let a = vm.pop()?;
            let r = crate::host_ext::str_append(&mut vm.arena, a, b)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrSlice as u8 => {
            let end = vm.pop()?;
            let start = vm.pop()?;
            let string = vm.pop()?;
            let start = vm.as_i64(start)?;
            let end = vm.as_i64(end)?;
            let r = crate::host_ext::str_slice(&mut vm.arena, string, start, end)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromI64 as u8 => {
            let value = vm.pop()?;
            let number = vm.as_i64(value)?;
            let string = crate::host_ext::str_from_i64(&mut vm.arena, number);
            vm.push(string);
            Ok(true)
        }
        x if x == Op::StrFromF64 as u8 => {
            let n = vm.pop()?;
            let r = crate::host_ext::str_from_f64(&mut vm.arena, n)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromByte as u8 => {
            let value = vm.pop()?;
            let byte = vm.as_i64(value)?;
            let r = crate::host_ext::str_from_byte(&mut vm.arena, byte)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufFromStr as u8 => {
            let value = vm.pop()?;
            let buffer = crate::host_buf::buf_from_str(&mut vm.arena, value)?;
            vm.push(buffer);
            Ok(true)
        }
        x if x == Op::BufToStr as u8 => {
            let value = vm.pop()?;
            let result = crate::host_buf::buf_to_str(&mut vm.arena, value);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::BufSlice as u8 => {
            let length = vm.pop()?;
            let offset = vm.pop()?;
            let length = vm.as_i64(length)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let result = crate::host_buf::buf_slice(&mut vm.arena, buffer, offset, length);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenRead as u8 => {
            vm.ensure_host_deadline_support("sys-open-read", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_read(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenWrite as u8 => {
            vm.ensure_host_deadline_support("sys-open-write", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_write(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenAppend as u8 => {
            vm.ensure_host_deadline_support("sys-open-append", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_append(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenCreateNew as u8 => {
            vm.ensure_host_deadline_support("sys-open-create-new", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_create_new(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenDir as u8 => {
            vm.ensure_host_deadline_support("sys-open-dir", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_dir(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysFsync as u8 => {
            vm.ensure_host_deadline_support("sys-fsync", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_fsync(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTruncate as u8 => {
            vm.ensure_host_deadline_support("sys-truncate", false)?;
            let length = vm.pop()?;
            let length = vm.as_i64(length)?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_truncate(handle, length);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysRename as u8 => {
            vm.ensure_host_deadline_support("sys-rename", false)?;
            let to = vm.pop()?;
            let from = vm.pop()?;
            let from = crate::host_ext::as_str(&vm.arena, from)?.to_string();
            let to = crate::host_ext::as_str(&vm.arena, to)?.to_string();
            let result = crate::host_ext::ResourceTable::sys_rename(&from, &to);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysClose as u8 => {
            vm.ensure_host_deadline_support("sys-close", false)?;
            let handle = vm.pop()?;
            let result = vm.resources.close(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysReadByte as u8 => {
            let handle = vm.pop()?;
            if let Some(error) = wait_readable(vm, handle, "sys-read-byte")? {
                push_i64_result(vm, Err(error));
                return Ok(true);
            }
            let result = vm.resources.read_byte(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWriteByte as u8 => {
            vm.ensure_host_deadline_support("sys-write-byte", false)?;
            let byte = vm.pop()?;
            let handle = vm.pop()?;
            let byte = vm.as_i64(byte)?;
            let result = vm.resources.write_byte(handle, byte);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysReadInto as u8 => {
            vm.ensure_host_deadline_support("sys-read-into", false)?;
            let requested = vm.pop()?;
            let offset = vm.pop()?;
            let requested = vm.as_i64(requested)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_read_into(
                &mut vm.arena,
                &vm.resources,
                handle,
                buffer,
                offset,
                requested,
            );
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysRandomFill as u8 => {
            vm.ensure_host_deadline_support("sys-random-fill", false)?;
            let requested = vm.pop()?;
            let offset = vm.pop()?;
            let requested = vm.as_i64(requested)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let result = crate::host_buf::sys_random_fill(&mut vm.arena, buffer, offset, requested);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSha256 as u8 => {
            let requested = vm.pop()?;
            let offset = vm.pop()?;
            let requested = vm.as_i64(requested)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let result = crate::host_buf::sys_sha256(&mut vm.arena, buffer, offset, requested);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWriteFrom as u8 => {
            vm.ensure_host_deadline_support("sys-write-from", false)?;
            let requested = vm.pop()?;
            let offset = vm.pop()?;
            let requested = vm.as_i64(requested)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_write_from(
                &vm.arena,
                &vm.resources,
                handle,
                buffer,
                offset,
                requested,
            );
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteOpen as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-open", false)?;
            let flags = vm.pop()?;
            let flags = vm.as_i64(flags)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sqlite_open(&path, flags);
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
        x if x == Op::SysSqliteColumnCount as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_column_count(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnType as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_column_type(handle, index);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnI64 as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm
                .resources
                .sqlite_column_i64(handle, index)
                .map(|value| match value {
                    Some(value) => {
                        let value = vm.make_i64(value);
                        crate::host_ext::option_some(&mut vm.arena, value)
                    }
                    None => Value::NONE,
                });
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnF64 as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm
                .resources
                .sqlite_column_f64(handle, index)
                .map(|value| match value {
                    Some(value) => {
                        let value = vm.arena.alloc(HeapObj::Float(value));
                        crate::host_ext::option_some(&mut vm.arena, value)
                    }
                    None => Value::NONE,
                });
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnText as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm
                .resources
                .sqlite_column_text(handle, index, lkjscript_core::MAX_BUFFER_BYTES)
                .map(|value| match value {
                    Some(value) => {
                        let value = vm.arena.alloc(HeapObj::Str(value));
                        crate::host_ext::option_some(&mut vm.arena, value)
                    }
                    None => Value::NONE,
                });
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteColumnBytes as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let handle = vm.pop()?;
            let result = vm
                .resources
                .sqlite_column_bytes(handle, index, lkjscript_core::MAX_BUFFER_BYTES)
                .map(|value| match value {
                    Some(value) => {
                        let value = vm.arena.alloc(HeapObj::Buf(value));
                        crate::host_ext::option_some(&mut vm.arena, value)
                    }
                    None => Value::NONE,
                });
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteChanges as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_changes(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteLastInsertRowid as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_last_insert_rowid(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteExtendedResultCode as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_extended_result_code(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSqliteBackup as u8 => {
            vm.ensure_host_deadline_support("sys-sqlite-backup", false)?;
            let flags = vm.pop()?;
            let flags = vm.as_i64(flags)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let handle = vm.pop()?;
            let result = vm.resources.sqlite_backup(handle, &path, flags);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::Arg as u8 => {
            let value = vm.pop()?;
            let index = vm.as_i64(value)?;
            let index = usize::try_from(index).ok();
            if index.is_none_or(|index| index >= vm.args.len()) {
                vm.push(Value::NONE);
            } else if let Some(index) = index {
                let string = vm.arena.alloc(HeapObj::Str(vm.args[index].clone()));
                let value = crate::host_ext::option_some(&mut vm.arena, string);
                vm.push(value);
            }
            Ok(true)
        }
        x if x == Op::Argc as u8 => {
            let count = i64::try_from(vm.args.len())
                .map_err(|_| lkjscript_core::Error::msg("argc out of range"))?;
            let value = vm.make_i64(count);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::EmptyStr as u8 => {
            let v = vm.arena.alloc(HeapObj::Str(String::new()));
            vm.push(v);
            Ok(true)
        }
        x if x == Op::BufNew as u8 => {
            let value = vm.pop()?;
            let size = vm.as_i64(value)?;
            let r = crate::host_buf::buf_new(&mut vm.arena, size)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufLen as u8 => {
            let v = vm.pop()?;
            let length = crate::host_buf::buf_len(&vm.arena, v)?;
            let value = vm.make_i64(length);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::BufRef as u8 => {
            let index = vm.pop()?;
            let buffer = vm.pop()?;
            let index = vm.as_i64(index)?;
            let byte = crate::host_buf::buf_ref(&vm.arena, buffer, index)?;
            let value = vm.make_i64(byte);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::BufSet as u8 => {
            let byte = vm.pop()?;
            let index = vm.pop()?;
            let buffer = vm.pop()?;
            let byte = vm.as_i64(byte)?;
            let index = vm.as_i64(index)?;
            let r = crate::host_buf::buf_set(&mut vm.arena, buffer, index, byte)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufGetU32 as u8 => {
            let index = vm.pop()?;
            let buffer = vm.pop()?;
            let index = vm.as_i64(index)?;
            let number = crate::host_buf::buf_get_u32(&vm.arena, buffer, index)?;
            let value = vm.make_i64(number);
            vm.push(value);
            Ok(true)
        }
        x if x == Op::BufSetU32 as u8 => {
            let number = vm.pop()?;
            let index = vm.pop()?;
            let buffer = vm.pop()?;
            let number = vm.as_i64(number)?;
            let index = vm.as_i64(index)?;
            let r = crate::host_buf::buf_set_u32(&mut vm.arena, buffer, index, number)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufClone as u8 => {
            let v = vm.pop()?;
            let r = crate::host_buf::buf_clone(&mut vm.arena, v)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysTtyGet as u8 => {
            vm.ensure_host_deadline_support("sys-tty-get", false)?;
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_tty_get(&mut vm.arena, &vm.resources, handle, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtySet as u8 => {
            vm.ensure_host_deadline_support("sys-tty-set", false)?;
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_tty_set(&vm.arena, &vm.resources, handle, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPoll as u8 => {
            let timeout = vm.pop()?;
            let handle = vm.pop()?;
            let requested = vm.as_i64(timeout)?;
            let mut timeout = requested;
            let mut deadline_limited = false;
            if let Some(remaining) = vm.remaining_wall_time()? {
                let remaining_ms = remaining.as_millis().max(1);
                let remaining_ms = i64::try_from(remaining_ms).unwrap_or(i64::MAX);
                if timeout > remaining_ms {
                    timeout = remaining_ms;
                    deadline_limited = true;
                }
            }
            let result = crate::host_buf::sys_poll(&vm.resources, handle, timeout);
            if deadline_limited && matches!(result, Ok(0)) {
                return Err(lkjscript_core::Error::deadline(
                    "execution wall deadline exceeded during sys-poll",
                ));
            }
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::StdinHandle as u8 => {
            vm.push(crate::host_buf::stdin_handle());
            Ok(true)
        }
        x if x == Op::SysIsatty as u8 => {
            vm.ensure_host_deadline_support("sys-isatty", false)?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_isatty(&vm.resources, handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardSave as u8 => {
            vm.ensure_host_deadline_support("sys-tty-guard-save", false)?;
            let buffer = vm.pop()?;
            let result = crate::host_buf::sys_tty_guard_save(&vm.arena, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardClear as u8 => {
            vm.ensure_host_deadline_support("sys-tty-guard-clear", false)?;
            let result = crate::host_buf::sys_tty_guard_clear();
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysNowMs as u8 => {
            let result = lkjscript_sys::now_ms_monotonic()
                .map_err(|error| lkjscript_core::Error::msg(format!("sys-now-ms: {error}")));
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWaitMs as u8 => {
            let duration = vm.pop()?;
            let milliseconds = vm
                .as_i64(duration)
                .map_err(|_| lkjscript_core::Error::msg("sys-wait-ms: expected I64 duration"));
            let milliseconds = milliseconds.and_then(|milliseconds| {
                u64::try_from(milliseconds)
                    .map_err(|_| lkjscript_core::Error::msg("sys-wait-ms: duration out of range"))
            });
            let result = match milliseconds {
                Ok(milliseconds) => {
                    if let Some(remaining) = vm.remaining_wall_time()? {
                        let requested = std::time::Duration::from_millis(milliseconds);
                        if requested > remaining {
                            let sleep_ms = u64::try_from(remaining.as_millis()).unwrap_or(u64::MAX);
                            match lkjscript_sys::sleep_ms(sleep_ms) {
                                Ok(()) => {
                                    return Err(lkjscript_core::Error::deadline(
                                        "execution wall deadline exceeded during sys-wait-ms",
                                    ));
                                }
                                Err(error) => {
                                    Err(lkjscript_core::Error::msg(format!("sys-wait-ms: {error}")))
                                }
                            }
                        } else {
                            sleep_result(milliseconds)
                        }
                    } else {
                        sleep_result(milliseconds)
                    }
                }
                Err(error) => Err(error),
            };
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSocket as u8 => {
            vm.ensure_host_deadline_support("sys-socket", false)?;
            let result = vm.resources.sys_socket();
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysBind as u8 => {
            vm.ensure_host_deadline_support("sys-bind", false)?;
            let port = vm.pop()?;
            let handle = vm.pop()?;
            let port = vm.as_i64(port)?;
            let result = vm.resources.sys_bind(handle, port);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysListen as u8 => {
            vm.ensure_host_deadline_support("sys-listen", false)?;
            let backlog = vm.pop()?;
            let handle = vm.pop()?;
            let backlog = vm.as_i64(backlog)?;
            let result = vm.resources.sys_listen(handle, backlog);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysAccept as u8 => {
            let handle = vm.pop()?;
            if let Some(error) = wait_readable(vm, handle, "sys-accept")? {
                push_language_result(vm, Err(error));
                return Ok(true);
            }
            let result = vm.resources.sys_accept(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysRecv as u8 => {
            let handle = vm.pop()?;
            if let Some(error) = wait_readable(vm, handle, "sys-recv")? {
                push_language_result(vm, Err(error));
                return Ok(true);
            }
            let result = vm.resources.sys_recv(&mut vm.arena, handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSend as u8 => {
            vm.ensure_host_deadline_support("sys-send", false)?;
            let data = vm.pop()?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_send(&vm.arena, handle, data);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPathExists as u8 => {
            vm.ensure_host_deadline_support("sys-path-exists", false)?;
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = crate::host_ext::ResourceTable::sys_path_exists(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::OkWrap as u8 => {
            let v = vm.pop()?;
            let __r = crate::host_ext::result_ok(&mut vm.arena, v);
            vm.push(__r);
            Ok(true)
        }
        x if x == Op::ErrWrap as u8 => {
            let v = vm.pop()?;
            let __r = crate::host_ext::result_err(&mut vm.arena, v);
            vm.push(__r);
            Ok(true)
        }
        x if x == Op::IsOk as u8 => {
            let v = vm.pop()?;
            vm.push(crate::host_ext::is_ok(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::UnwrapOk as u8 => {
            let v = vm.pop()?;
            vm.push(crate::host_ext::unwrap_ok(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::UnwrapErr as u8 => {
            let v = vm.pop()?;
            vm.push(crate::host_ext::unwrap_err(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::SomeWrap as u8 => {
            let value = vm.pop()?;
            let wrapped = crate::host_ext::option_some(&mut vm.arena, value);
            vm.push(wrapped);
            Ok(true)
        }
        x if x == Op::IsSome as u8 => {
            let value = vm.pop()?;
            vm.push(crate::host_ext::is_some(&vm.arena, value)?);
            Ok(true)
        }
        x if x == Op::UnwrapSome as u8 => {
            let value = vm.pop()?;
            vm.push(crate::host_ext::unwrap_some(&vm.arena, value)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}
