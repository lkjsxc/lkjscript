use super::*;

pub(super) fn dispatch(vm: &mut Vm<'_>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::StrLen as u8 => {
            let value = vm.pop()?;
            let text = crate::run::structural_ops::copy_string(vm, value)?;
            let length =
                i64::try_from(text.len()).map_err(|_| Error::msg("string length exceeds I64"))?;
            vm.push(Value::from_i64(length));
            Ok(true)
        }
        x if x == Op::StrRef as u8 => {
            let index = vm.pop()?;
            let index = vm.as_i64(index)?;
            let string = vm.pop()?;
            let index = usize::try_from(index)
                .map_err(|_| Error::msg("string byte index is out of range"))?;
            let text = crate::run::structural_ops::copy_string(vm, string)?;
            let byte = text
                .as_bytes()
                .get(index)
                .copied()
                .ok_or_else(|| Error::msg("string byte index is out of bounds"))?;
            vm.push(Value::from_i64(i64::from(byte)));
            Ok(true)
        }
        x if x == Op::StrAppend as u8 => {
            let right = vm.pop()?;
            let left = vm.pop()?;
            let mut output = crate::run::structural_ops::copy_string(vm, left)?;
            output.push_str(&crate::run::structural_ops::copy_string(vm, right)?);
            let value = crate::run::structural_ops::publish_string(vm, output)?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrSlice as u8 => {
            let end = vm.pop()?;
            let end = usize::try_from(vm.as_i64(end)?)
                .map_err(|_| Error::msg("string slice end is out of range"))?;
            let start = vm.pop()?;
            let start = usize::try_from(vm.as_i64(start)?)
                .map_err(|_| Error::msg("string slice start is out of range"))?;
            let string = vm.pop()?;
            let text = crate::run::structural_ops::copy_string(vm, string)?;
            let slice = text
                .get(start..end)
                .ok_or_else(|| Error::msg("string slice is out of bounds or splits UTF-8"))?;
            let value = crate::run::structural_ops::publish_string(vm, slice.to_owned())?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrFromI64 as u8 => {
            let number = vm.pop()?;
            let number = vm.as_i64(number)?;
            let value = crate::run::structural_ops::publish_string(vm, number.to_string())?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrFromF64 as u8 => {
            let number = vm
                .pop()?
                .as_f64()
                .ok_or_else(|| Error::msg("string-from-f64 expects F64"))?;
            let value = crate::run::structural_ops::publish_string(vm, number.to_string())?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrFromByte as u8 => {
            let byte = vm.pop()?;
            let byte = u8::try_from(vm.as_i64(byte)?)
                .map_err(|_| Error::msg("string-from-byte value is out of range"))?;
            let value =
                crate::run::structural_ops::publish_string(vm, String::from(char::from(byte)))?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::EmptyStr as u8 => {
            let value = crate::run::structural_ops::publish_string(vm, String::new())?;
            vm.push(value);
            Ok(true)
        }
        _ => Ok(false),
    }
}
