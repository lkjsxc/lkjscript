use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::SysSocket as u8 => {
            vm.ensure_host_deadline_support("open-tcp-socket", false)?;
            vm.require_capability(lkjscript_core::CapabilityKind::Network)?;
            let result = vm.resources.sys_socket();
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Network,
                crate::run::structural_ops::HostValueType::Resource(
                    lkjscript_core::ResourceKind::TcpListener,
                ),
                result,
            );
            Ok(true)
        }
        x if x == Op::SysBind as u8 => {
            vm.ensure_host_deadline_support("bind-tcp", false)?;
            let port = vm.pop()?;
            let handle = vm.pop()?;
            let port = vm.as_i64(port)?;
            let result = vm.resources.sys_bind(handle, port);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Network,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysListen as u8 => {
            vm.ensure_host_deadline_support("listen-tcp", false)?;
            let backlog = vm.pop()?;
            let handle = vm.pop()?;
            let backlog = vm.as_i64(backlog)?;
            let result = vm.resources.sys_listen(handle, backlog);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Network,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysAccept as u8 => {
            let handle = vm.pop()?;
            let success = crate::run::structural_ops::HostValueType::Resource(
                lkjscript_core::ResourceKind::TcpStream,
            );
            if let Some(error) = wait_readable(vm, handle, "accept-tcp")? {
                push_runtime_result(
                    vm,
                    lkjscript_core::SystemErrorKind::Network,
                    success,
                    Err(error),
                );
                return Ok(true);
            }
            let result = vm.resources.sys_accept(handle);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Network,
                success,
                result,
            );
            Ok(true)
        }
        x if x == Op::SysRecv as u8 => {
            let handle = vm.pop()?;
            if let Some(error) = wait_readable(vm, handle, "receive-string")? {
                push_language_result(
                    vm,
                    lkjscript_core::SystemErrorKind::Network,
                    crate::run::structural_ops::HostValueType::String,
                    Err(error),
                );
                return Ok(true);
            }
            match vm.resources.sys_recv(handle) {
                Ok(text) => push_language_result(
                    vm,
                    lkjscript_core::SystemErrorKind::Network,
                    crate::run::structural_ops::HostValueType::String,
                    Ok(crate::run::structural_ops::HostValue::String(text)),
                ),
                Err(crate::host_ext::SocketReceiveError::Network(error)) => {
                    push_language_result(
                        vm,
                        lkjscript_core::SystemErrorKind::Network,
                        crate::run::structural_ops::HostValueType::String,
                        Err(error),
                    );
                }
                Err(crate::host_ext::SocketReceiveError::Utf8(error)) => {
                    let result = crate::run::structural_ops::publish_system_utf8_result(vm, error)?;
                    vm.push(result);
                }
            }
            Ok(true)
        }
        x if x == Op::SysSend as u8 => {
            vm.ensure_host_deadline_support("send-string", false)?;
            let data = vm.pop()?;
            let handle = vm.pop()?;
            let data = crate::run::structural_ops::copy_string(vm, data)?;
            let result = vm.resources.sys_send(handle, &data);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Network, result);
            Ok(true)
        }
        _ => Ok(false),
    }
}
