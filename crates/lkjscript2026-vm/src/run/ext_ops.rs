//! Dispatch for string and file opcodes.

use lkjscript2026_core::{HeapObj, JitHook, Op, Result, Value};

use crate::run::Vm;

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
        x if x == Op::StrFromByte as u8 => {
            let b = vm.pop();
            let r = crate::host_ext::str_from_byte(&mut vm.arena, b)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysOpenRead as u8 => {
            let p = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, p)?.to_string();
            let r = vm.fds.sys_open_read(&path)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysOpenWrite as u8 => {
            let p = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, p)?.to_string();
            let r = vm.fds.sys_open_write(&path)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::CloseFd as u8 => {
            let fd = vm.pop();
            let r = vm.fds.close(fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::ReadByteFd as u8 => {
            let fd = vm.pop();
            let r = vm.fds.read_byte(fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::WriteByteFd as u8 => {
            let b = vm.pop();
            let fd = vm.pop();
            let r = vm.fds.write_byte(fd, b)?;
            vm.push(r);
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
        x if x == Op::SysIoctl as u8 => {
            let buf = vm.pop();
            let req = vm.pop();
            let fd = vm.pop();
            let r = crate::host_buf::sys_ioctl(&mut vm.arena, fd, req, buf)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysPoll as u8 => {
            let ms = vm.pop();
            let fd = vm.pop();
            let r = crate::host_buf::sys_poll(fd, ms)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StdinFd as u8 => {
            vm.push(crate::host_buf::stdin_fd());
            Ok(true)
        }
        x if x == Op::Isatty as u8 => {
            let fd = vm.pop();
            let r = crate::host_buf::isatty(fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::TtyGuardSave as u8 => {
            let buf = vm.pop();
            let r = crate::host_buf::tty_guard_save(&vm.arena, buf)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::TtyGuardClear as u8 => {
            vm.push(crate::host_buf::tty_guard_clear());
            Ok(true)
        }
        x if x == Op::NowMs as u8 => {
            vm.push(crate::host_term::now_ms()?);
            Ok(true)
        }
        x if x == Op::WaitMs as u8 => {
            let v = vm.pop();
            vm.push(crate::host_term::wait_ms(v)?);
            Ok(true)
        }
        x if x == Op::SysSocket as u8 => {
            let r = vm.fds.sys_socket()?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysBind as u8 => {
            let port = vm.pop();
            let fd = vm.pop();
            let r = vm.fds.sys_bind(fd, port)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysListen as u8 => {
            let backlog = vm.pop();
            let fd = vm.pop();
            let r = vm.fds.sys_listen(fd, backlog)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysAccept as u8 => {
            let fd = vm.pop();
            let r = vm.fds.sys_accept(fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysRecv as u8 => {
            let fd = vm.pop();
            let r = vm.fds.sys_recv(&mut vm.arena, fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysSend as u8 => {
            let data = vm.pop();
            let fd = vm.pop();
            let r = vm.fds.sys_send(&vm.arena, fd, data)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::SysPathExists as u8 => {
            let p = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, p)?.to_string();
            vm.push(crate::host_ext::FdTable::sys_path_exists(&path)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}
