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
        x if x == Op::OpenRead as u8 => {
            let p = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, p)?.to_string();
            let r = vm.fds.open_read(&path)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::OpenWrite as u8 => {
            let p = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, p)?.to_string();
            let r = vm.fds.open_write(&path)?;
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
        x if x == Op::TermRaw as u8 => {
            vm.push(crate::host_term::term_raw()?);
            Ok(true)
        }
        x if x == Op::TermCooked as u8 => {
            vm.push(crate::host_term::term_cooked()?);
            Ok(true)
        }
        x if x == Op::PollByte as u8 => {
            vm.push(crate::host_term::poll_byte()?);
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
        x if x == Op::TcpListen as u8 => {
            let p = vm.pop();
            let r = vm.fds.tcp_listen(p)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::TcpAccept as u8 => {
            let fd = vm.pop();
            let r = vm.fds.tcp_accept(fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::TcpRecv as u8 => {
            let fd = vm.pop();
            let r = vm.fds.tcp_recv(&mut vm.arena, fd)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::TcpSend as u8 => {
            let data = vm.pop();
            let fd = vm.pop();
            let r = vm.fds.tcp_send(&vm.arena, fd, data)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::PathExists as u8 => {
            let p = vm.pop();
            let path = crate::host_ext::as_str(&vm.arena, p)?.to_string();
            vm.push(crate::host_ext::path_exists(&path)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}
