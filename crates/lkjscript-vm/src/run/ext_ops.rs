//! Dispatch for string and file opcodes.

use lkjscript_core::{HeapObj, JitHook, Op, Result, Value};

use crate::run::Vm;

fn push_language_result<J: JitHook>(vm: &mut Vm<'_, J>, result: Result<Value>) {
    let value = crate::host_ext::language_result(&mut vm.arena, result);
    vm.push(value);
}

pub fn dispatch_ext<J: JitHook>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::StrLen as u8 => {
            let v = vm.pop();
            let r = crate::host_ext::str_len(&vm.arena, v)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrRef as u8 => {
            let i = vm.pop();
            let s = vm.pop();
            let r = crate::host_ext::str_ref(&vm.arena, s, i)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrAppend as u8 => {
            let b = vm.pop();
            let a = vm.pop();
            let r = crate::host_ext::str_append(&mut vm.arena, a, b)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrSlice as u8 => {
            let en = vm.pop();
            let st = vm.pop();
            let s = vm.pop();
            let r = crate::host_ext::str_slice(&mut vm.arena, s, st, en)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromI64 as u8 => {
            let n = vm.pop();
            let r = crate::host_ext::str_from_i64(&mut vm.arena, n)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromF64 as u8 => {
            let n = vm.pop();
            let r = crate::host_ext::str_from_f64(&mut vm.arena, n)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromByte as u8 => {
            let b = vm.pop();
            let r = crate::host_ext::str_from_byte(&mut vm.arena, b)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysOpenRead as u8 => {
            let path = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_read(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysOpenWrite as u8 => {
            let path = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = vm.resources.sys_open_write(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysClose as u8 => {
            let handle = vm.pop();
            let result = vm.resources.close(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysReadByte as u8 => {
            let handle = vm.pop();
            let result = vm.resources.read_byte(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWriteByte as u8 => {
            let byte = vm.pop();
            let handle = vm.pop();
            let result = vm.resources.write_byte(handle, byte);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::Arg as u8 => {
            let i = vm.pop().as_int().unwrap_or(-1);
            if i < 0 || i as usize >= vm.args.len() {
                vm.push(Value::NIL);
            } else {
                let s = vm.args[i as usize].clone();
                let v = vm.arena.alloc(HeapObj::Str(s));
                vm.push(v);
            }
            Ok(true)
        }
        x if x == Op::Argc as u8 => {
            vm.push(Value::from_int(vm.args.len() as i64));
            Ok(true)
        }
        x if x == Op::EmptyStr as u8 => {
            let v = vm.arena.alloc(HeapObj::Str(String::new()));
            vm.push(v);
            Ok(true)
        }
        x if x == Op::BufNew as u8 => {
            let n = vm.pop();
            let r = crate::host_buf::buf_new(&mut vm.arena, n)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufLen as u8 => {
            let v = vm.pop();
            let r = crate::host_buf::buf_len(&vm.arena, v)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufRef as u8 => {
            let i = vm.pop();
            let v = vm.pop();
            let r = crate::host_buf::buf_ref(&vm.arena, v, i)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufSet as u8 => {
            let b = vm.pop();
            let i = vm.pop();
            let v = vm.pop();
            let r = crate::host_buf::buf_set(&mut vm.arena, v, i, b)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufGetU32 as u8 => {
            let i = vm.pop();
            let v = vm.pop();
            let r = crate::host_buf::buf_get_u32(&vm.arena, v, i)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufSetU32 as u8 => {
            let n = vm.pop();
            let i = vm.pop();
            let v = vm.pop();
            let r = crate::host_buf::buf_set_u32(&mut vm.arena, v, i, n)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::BufClone as u8 => {
            let v = vm.pop();
            let r = crate::host_buf::buf_clone(&mut vm.arena, v)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysTtyGet as u8 => {
            let buffer = vm.pop();
            let handle = vm.pop();
            let result = crate::host_buf::sys_tty_get(&mut vm.arena, &vm.resources, handle, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtySet as u8 => {
            let buffer = vm.pop();
            let handle = vm.pop();
            let result = crate::host_buf::sys_tty_set(&vm.arena, &vm.resources, handle, buffer);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPoll as u8 => {
            let timeout = vm.pop();
            let handle = vm.pop();
            let result = crate::host_buf::sys_poll(&vm.resources, handle, timeout);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::StdinHandle as u8 => {
            vm.push(crate::host_buf::stdin_handle());
            Ok(true)
        }
        x if x == Op::SysIsatty as u8 => {
            let handle = vm.pop();
            let result = crate::host_buf::sys_isatty(&vm.resources, handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysTtyGuardSave as u8 => {
            let buffer = vm.pop();
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
                .map(Value::from_int)
                .map_err(|error| lkjscript_core::Error::msg(format!("sys-now-ms: {error}")));
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysWaitMs as u8 => {
            let duration = vm.pop();
            let result = match duration.as_int() {
                Some(milliseconds) if milliseconds >= 0 => {
                    lkjscript_sys::sleep_ms(milliseconds as u64)
                        .map(|()| Value::NIL)
                        .map_err(|error| {
                            lkjscript_core::Error::msg(format!("sys-wait-ms: {error}"))
                        })
                }
                Some(_) => Err(lkjscript_core::Error::msg(
                    "sys-wait-ms: duration out of range",
                )),
                None => Err(lkjscript_core::Error::msg(
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
            let port = vm.pop();
            let handle = vm.pop();
            let result = vm.resources.sys_bind(handle, port);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysListen as u8 => {
            let backlog = vm.pop();
            let handle = vm.pop();
            let result = vm.resources.sys_listen(handle, backlog);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysAccept as u8 => {
            let handle = vm.pop();
            let result = vm.resources.sys_accept(handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysRecv as u8 => {
            let handle = vm.pop();
            let result = vm.resources.sys_recv(&mut vm.arena, handle);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysSend as u8 => {
            let data = vm.pop();
            let handle = vm.pop();
            let result = vm.resources.sys_send(&vm.arena, handle, data);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::SysPathExists as u8 => {
            let path = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, path)?.to_string();
            let result = crate::host_ext::ResourceTable::sys_path_exists(&path);
            push_language_result(vm, result);
            Ok(true)
        }
        x if x == Op::OkWrap as u8 => {
            let v = vm.pop();
            let __r = crate::host_ext::result_ok(&mut vm.arena, v);
            vm.push(__r);
            Ok(true)
        }
        x if x == Op::ErrWrap as u8 => {
            let v = vm.pop();
            let __r = crate::host_ext::result_err(&mut vm.arena, v);
            vm.push(__r);
            Ok(true)
        }
        x if x == Op::IsOk as u8 => {
            let v = vm.pop();
            vm.push(crate::host_ext::is_ok(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::UnwrapOk as u8 => {
            let v = vm.pop();
            vm.push(crate::host_ext::unwrap_ok(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::UnwrapErr as u8 => {
            let v = vm.pop();
            vm.push(crate::host_ext::unwrap_err(&vm.arena, v)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}
