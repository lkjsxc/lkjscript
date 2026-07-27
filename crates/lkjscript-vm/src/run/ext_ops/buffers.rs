use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::BufFromStr as u8 => {
            let value = vm.pop()?;
            let buffer = crate::host_buf::buf_from_str(&mut vm.arena, value)?;
            vm.push(buffer);
            Ok(true)
        }
        x if x == Op::BufToStr as u8 => {
            let value = vm.pop()?;
            let result = crate::host_buf::buf_to_str(&mut vm.arena, value)?;
            let result = crate::host_ext::utf8_result(&mut vm.arena, result)?;
            vm.push(result);
            Ok(true)
        }
        x if x == Op::BufSlice as u8 => {
            let length = vm.pop()?;
            let offset = vm.pop()?;
            let length = vm.as_i64(length)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let result = crate::host_buf::buf_slice(&mut vm.arena, buffer, offset, length);
            push_language_result(vm, lkjscript_core::SystemErrorKind::Unsupported, result);
            Ok(true)
        }
        x if x == Op::SysReadInto as u8 => {
            vm.ensure_host_deadline_support("read-into", false)?;
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
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Io, result);
            Ok(true)
        }
        x if x == Op::SysRandomFill as u8 => {
            vm.ensure_host_deadline_support("fill-random", false)?;
            let requested = vm.pop()?;
            let offset = vm.pop()?;
            let requested = vm.as_i64(requested)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Entropy)?;
            let result = crate::host_buf::sys_random_fill(&mut vm.arena, buffer, offset, requested);
            push_language_result(vm, lkjscript_core::SystemErrorKind::Random, result);
            Ok(true)
        }
        x if x == Op::SysSha256 as u8 => {
            let requested = vm.pop()?;
            let offset = vm.pop()?;
            let requested = vm.as_i64(requested)?;
            let offset = vm.as_i64(offset)?;
            let buffer = vm.pop()?;
            let result = crate::host_buf::sys_sha256(&mut vm.arena, buffer, offset, requested);
            push_language_result(vm, lkjscript_core::SystemErrorKind::Unsupported, result);
            Ok(true)
        }
        x if x == Op::SysWriteFrom as u8 => {
            vm.ensure_host_deadline_support("write-from", false)?;
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
            push_i64_result(vm, lkjscript_core::SystemErrorKind::Io, result);
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
            let value = vm.make_i64(length)?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::BufRef as u8 => {
            let index = vm.pop()?;
            let buffer = vm.pop()?;
            let index = vm.as_i64(index)?;
            let byte = crate::host_buf::buf_ref(&vm.arena, buffer, index)?;
            let value = vm.make_i64(byte)?;
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
            let value = vm.make_i64(number)?;
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
        _ => Ok(false),
    }
}
