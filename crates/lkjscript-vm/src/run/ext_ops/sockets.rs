use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
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
        _ => Ok(false),
    }
}
