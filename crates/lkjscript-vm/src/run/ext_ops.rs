//! Dispatch for string and file opcodes.

use lkjscript_core::{HeapObj, JitHook, Op, Result, Value};

use crate::run::Vm;

fn push_language_result<J: JitHook>(vm: &mut Vm<'_, J>, result: Result<Value>) {
    let value = crate::host_ext::language_result(&mut vm.arena, result);
    vm.push(value);
}

fn push_i64_result<J: JitHook>(vm: &mut Vm<'_, J>, result: Result<i64>) {
    let result = result.map(|number| vm.make_i64(number));
    push_language_result(vm, result);
}

pub fn dispatch_ext<J: JitHook>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
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
        x if x == Op::SysOpenRead as u8 => {
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_read(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenWrite as u8 => {
            let path = vm.pop()?;
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_write(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysClose as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.close(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysReadByte as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.read_byte(handle);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWriteByte as u8 => {
            let byte = vm.pop()?;
            let handle = vm.pop()?;
            let byte = vm.as_i64(byte)?;
            let result = vm.resources.write_byte(handle, byte);
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
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_tty_get(&mut vm.arena, &vm.resources, handle, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtySet as u8 => {
            let buffer = vm.pop()?;
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_tty_set(&vm.arena, &vm.resources, handle, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPoll as u8 => {
            let timeout = vm.pop()?;
            let handle = vm.pop()?;
            let timeout = vm.as_i64(timeout)?;
            let result = crate::host_buf::sys_poll(&vm.resources, handle, timeout);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::StdinHandle as u8 => {
            vm.push(crate::host_buf::stdin_handle());
            Ok(true)
        }
        x if x == Op::SysIsatty as u8 => {
            let handle = vm.pop()?;
            let result = crate::host_buf::sys_isatty(&vm.resources, handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardSave as u8 => {
            let buffer = vm.pop()?;
            let result = crate::host_buf::sys_tty_guard_save(&vm.arena, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardClear as u8 => {
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
            let result = match vm.as_i64(duration) {
                Ok(milliseconds) => u64::try_from(milliseconds)
                    .map_err(|_| lkjscript_core::Error::msg("sys-wait-ms: duration out of range"))
                    .and_then(|milliseconds| {
                        lkjscript_sys::sleep_ms(milliseconds)
                            .map(|()| Value::UNIT)
                            .map_err(|error| {
                                lkjscript_core::Error::msg(format!("sys-wait-ms: {error}"))
                            })
                    }),
                Err(_) => Err(lkjscript_core::Error::msg(
                    "sys-wait-ms: expected I64 duration",
                )),
            };
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSocket as u8 => {
            let result = vm.resources.sys_socket();
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysBind as u8 => {
            let port = vm.pop()?;
            let handle = vm.pop()?;
            let port = vm.as_i64(port)?;
            let result = vm.resources.sys_bind(handle, port);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysListen as u8 => {
            let backlog = vm.pop()?;
            let handle = vm.pop()?;
            let backlog = vm.as_i64(backlog)?;
            let result = vm.resources.sys_listen(handle, backlog);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysAccept as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sys_accept(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysRecv as u8 => {
            let handle = vm.pop()?;
            let result = vm.resources.sys_recv(&mut vm.arena, handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSend as u8 => {
            let data = vm.pop()?;
            let handle = vm.pop()?;
            let result = vm.resources.sys_send(&vm.arena, handle, data);
            push_i64_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPathExists as u8 => {
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
