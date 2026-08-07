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
            let result = execution_policy(send_string(vm, handle, data.as_bytes()))?;
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Network, result);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn send_string<J: RuntimeTier>(vm: &mut Vm<'_, J>, handle: Value, data: &[u8]) -> Result<i64> {
    if data.len() > vm.remaining_output_capacity()? {
        return Err(Error::resource(
            lkjscript_core::ResourceLimitKind::OutputBytes,
            "send-string output policy exhausted",
        ));
    }
    let interruption = vm.interruption()?;
    let mut sent = 0_usize;
    while sent < data.len() {
        interruption.check()?;
        let count = vm.resources.sys_send_chunk(handle, &data[sent..])?;
        if count == 0 {
            return Err(Error::msg("sys-send: zero-byte progress"));
        }
        sent = sent
            .checked_add(count)
            .ok_or_else(|| Error::msg("sys-send count out of range"))?;
        vm.record_output(count)?;
    }
    i64::try_from(sent).map_err(|_| Error::msg("sys-send count out of range"))
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::*;
    use crate::run::NoTier;
    use std::io::{Read, Write};

    fn chunk() -> lkjscript_core::ValidatedChunk {
        let mut chunk = lkjscript_core::Chunk::new();
        chunk.main.emit(lkjscript_core::Op::Unit);
        chunk.main.emit(lkjscript_core::Op::Return);
        lkjscript_core::validate_chunk(chunk, lkjscript_core::ValidationPolicy::Unrestricted)
            .expect("socket bulk test chunk validates")
    }

    #[test]
    fn local_socket_send_and_receive_requests_cross_former_bulk_limit() -> Result<()> {
        let len = 64_usize * 1024 + 1;
        let len_i64 = i64::try_from(len).expect("test length fits i64");
        let probe = std::net::TcpListener::bind(("127.0.0.1", 0))
            .map_err(|error| Error::host(error.to_string()))?;
        let port = probe
            .local_addr()
            .map_err(|error| Error::host(error.to_string()))?
            .port();
        drop(probe);

        let chunk = chunk();
        let mut vm = Vm::new(
            &chunk,
            NoTier,
            crate::ExecutionInputs::default(),
            lkjscript_core::ExecutionConfig {
                max_heap_bytes: len * 4,
                max_output_bytes: len * 2,
                ..lkjscript_core::ExecutionConfig::default()
            },
        );
        let listener = vm.resources.sys_socket()?;
        vm.resources.sys_bind(listener, i64::from(port))?;
        vm.resources.sys_listen(listener, 1)?;

        let client = std::thread::spawn(move || -> std::io::Result<Vec<u8>> {
            let mut stream = std::net::TcpStream::connect(("127.0.0.1", port))?;
            let input = vec![0x31; len];
            stream.write_all(&input)?;
            stream.shutdown(std::net::Shutdown::Write)?;
            let mut output = vec![0_u8; len];
            stream.read_exact(&mut output)?;
            Ok(output)
        });
        let stream = vm.resources.sys_accept(listener)?;
        let owner = vm.unique.allocate(len_i64)?;
        let view = vm.unique.borrow(owner, true)?;
        let mut received = 0_usize;
        while received < len {
            let count = crate::host_bytes::read_into(&mut vm, stream, view)?;
            if count == 0 {
                return Err(Error::msg("socket bulk fixture reached early EOF"));
            }
            received = received
                .checked_add(usize::try_from(count).map_err(|_| Error::msg("negative receive"))?)
                .ok_or_else(|| Error::msg("socket receive count overflow"))?;
        }
        assert_eq!(received, len);
        let output = vec![0x52; len];
        assert_eq!(send_string(&mut vm, stream, &output)?, len_i64);
        assert_eq!(
            client
                .join()
                .map_err(|_| Error::host("socket bulk client panicked"))?
                .map_err(|error| Error::host(error.to_string()))?,
            output
        );
        vm.unique.end_borrow(view)?;
        vm.unique.drop_owner(owner)?;
        vm.resources.close(stream)?;
        vm.resources.close(listener)?;
        vm.unique.verify_empty()?;
        Ok(())
    }
}
