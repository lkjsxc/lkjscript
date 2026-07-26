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
            let text = display_value(&vm.arena, value)?;
            vm.record_output(text.len())?;
            write_output(text.as_bytes(), "print")?;
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::Flush as u8 => {
            vm.ensure_host_deadline_support("flush", false)?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            flush_out()?;
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::ReadByte as u8 => {
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            vm.wait_for_stdin()?;
            let number = read_byte()?;
            let value = vm.make_i64(number)?;
            vm.push(value);
            Ok(())
        }
        x if x == Op::WriteByte as u8 => {
            vm.ensure_host_deadline_support("write-byte", false)?;
            let value = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            let byte = vm.as_i64(value)?;
            vm.record_output(1)?;
            vm.push(write_byte(byte)?);
            Ok(())
        }
        x if x == Op::WriteStr as u8 => {
            vm.ensure_host_deadline_support("write-str", false)?;
            let value = vm.pop()?;
            vm.require_capability(lkjscript_core::CapabilityKind::Stdio)?;
            let length = crate::host_ext::as_str(&vm.arena, value)?.len();
            vm.record_output(length)?;
            vm.push(write_str(&vm.arena, value)?);
            Ok(())
        }
        _ => unreachable!("opcode family checked"),
    }
}
