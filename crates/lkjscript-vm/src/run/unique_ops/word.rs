use super::*;

pub(super) fn handles(op: u8) -> bool {
    matches!(
        Op::from_byte(op),
        Some(Op::ByteSliceReadU32Le | Op::ByteSliceMutWriteU32Le)
    )
}

pub(super) fn dispatch(vm: &mut Vm<'_>, op: u8) -> Result<()> {
    match Op::from_byte(op).ok_or_else(|| Error::msg("unknown byte-word opcode"))? {
        Op::ByteSliceReadU32Le => {
            let index = vm.pop()?;
            let view = vm.pop()?;
            let index = vm.as_i64(index)?;
            let word = if super::super::structural_ops::is_byte_view(vm, view) {
                super::super::structural_ops::read_u32_little_endian(vm, view, index)?
            } else {
                vm.unique.read_u32_little_endian(view, index)?
            };
            vm.push(Value::from_i64(word));
        }
        Op::ByteSliceMutWriteU32Le => {
            let word = vm.pop()?;
            let index = vm.pop()?;
            let view = vm.pop()?;
            let word = vm.as_i64(word)?;
            let index = vm.as_i64(index)?;
            vm.unique.write_u32_little_endian(view, index, word)?;
            vm.push(Value::UNIT);
        }
        _ => unreachable!("byte-word opcode family checked"),
    }
    Ok(())
}
