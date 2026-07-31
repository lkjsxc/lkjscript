use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match Op::from_byte(op) {
        Some(Op::ConvertStringToBytes) => {
            let value = vm.pop()?;
            let text = crate::run::structural_ops::copy_string(vm, value)?;
            let value = vm.unique.allocate_bytes(text.into_bytes())?;
            vm.push(value);
            Ok(true)
        }
        Some(Op::ConvertBytesToString) => {
            let value = vm.pop()?;
            let bytes = exact_bytes(vm, value)?;
            let result = lkjscript_core::validate_utf8(&bytes)
                .map(|text| crate::run::structural_ops::HostValue::String(text.to_owned()));
            let result = crate::run::structural_ops::publish_utf8_result(vm, result)?;
            vm.push(result);
            Ok(true)
        }
        Some(Op::SysReadInto) => {
            vm.ensure_host_deadline_support("read-into", false)?;
            let view = vm.pop()?;
            let resource = vm.pop()?;
            let result =
                crate::host_bytes::read_into(&mut vm.unique, &vm.resources, resource, view);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Io, result);
            Ok(true)
        }
        Some(Op::SysWriteFrom) => {
            vm.ensure_host_deadline_support("write-from", false)?;
            let view = vm.pop()?;
            let resource = vm.pop()?;
            let result =
                crate::host_bytes::write_from(&mut vm.unique, &vm.resources, resource, view);
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Io, result);
            Ok(true)
        }
        Some(Op::SysRandomFill) => {
            vm.ensure_host_deadline_support("fill-random", false)?;
            let view = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Entropy)?;
            let result = crate::host_bytes::fill_random(&mut vm.unique, view);
            push_runtime_result(
                vm,
                lkjscript_core::SystemErrorKind::Random,
                crate::run::structural_ops::HostValueType::Unit,
                result,
            );
            Ok(true)
        }
        Some(Op::SysSha256) => {
            let view = vm.pop()?;
            let digest = crate::host_bytes::sha256(&mut vm.unique, view)?;
            vm.push(digest);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn exact_bytes<J: RuntimeTier>(vm: &mut Vm<'_, J>, value: Value) -> Result<Vec<u8>> {
    if let Some(index) = value.as_static_bytes() {
        return match vm.chunk.constants().get(usize::from(index)) {
            Some(lkjscript_core::Constant::StaticBytes(bytes)) => Ok(bytes.to_vec()),
            _ => Err(Error::msg("stale static bytes constant")),
        };
    }
    vm.unique.copy_bytes(value)
}
