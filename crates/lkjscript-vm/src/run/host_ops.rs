use super::*;

use lkjscript_core::Op;

pub(super) fn handles(op: u8) -> bool {
    op == Op::Print as u8
        || op == Op::Flush as u8
        || op == Op::ReadByte as u8
        || op == Op::WriteByte as u8
        || op == Op::WriteStr as u8
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::Print as u8 => {
            vm.ensure_host_deadline_support("print", false)?;
            let value = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            let provider = stdio(vm)?;
            let text = data::display::value(vm, value)?;
            vm.record_output(text.len())?;
            write_output(provider.as_ref(), text.as_bytes(), "print")?;
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::Flush as u8 => {
            vm.ensure_host_deadline_support("flush", false)?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            flush_out(stdio(vm)?.as_ref())?;
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::ReadByte as u8 => {
            vm.ensure_host_deadline_support("read-byte", false)?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            let number = read_byte(stdio(vm)?.as_ref())?;
            let value = Value::from_i64(number);
            vm.push(value);
            Ok(())
        }
        x if x == Op::WriteByte as u8 => {
            vm.ensure_host_deadline_support("write-byte", false)?;
            let value = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            let byte = vm.as_i64(value)?;
            vm.record_output(1)?;
            vm.push(write_byte(stdio(vm)?.as_ref(), byte)?);
            Ok(())
        }
        x if x == Op::WriteStr as u8 => {
            vm.ensure_host_deadline_support("write-string", false)?;
            let value = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            let text = structural_ops::copy_string(vm, value)?;
            vm.record_output(text.len())?;
            vm.push(write_str(stdio(vm)?.as_ref(), &text)?);
            Ok(())
        }
        _ => unreachable!("opcode family checked"),
    }
}

fn stdio<J: RuntimeTier>(
    vm: &Vm<'_, J>,
) -> Result<std::sync::Arc<dyn lkjscript_host::StdioProvider>> {
    vm.inputs
        .host
        .stdio
        .clone()
        .ok_or_else(|| Error::host("stdio capability has no granted provider"))
}
