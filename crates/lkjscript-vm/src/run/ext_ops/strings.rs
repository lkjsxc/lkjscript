use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::StrLen as u8 => {
            let v = vm.pop()?;
            let number = crate::host_ext::str_len(&vm.arena, v)?;
            let value = vm.make_i64(number)?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrRef as u8 => {
            let index = vm.pop()?;
            let string = vm.pop()?;
            let index = vm.as_i64(index)?;
            let number = crate::host_ext::str_ref(&vm.arena, string, index)?;
            let value = vm.make_i64(number)?;
            vm.push(value);
            Ok(true)
        }
        x if x == Op::StrAppend as u8 => {
            let b = vm.pop()?;
            let a = vm.pop()?;
            let r = crate::host_ext::str_append(&mut vm.arena, a, b)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrSlice as u8 => {
            let end = vm.pop()?;
            let start = vm.pop()?;
            let string = vm.pop()?;
            let start = vm.as_i64(start)?;
            let end = vm.as_i64(end)?;
            let r = crate::host_ext::str_slice(&mut vm.arena, string, start, end)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromI64 as u8 => {
            let value = vm.pop()?;
            let number = vm.as_i64(value)?;
            let string = crate::host_ext::str_from_i64(&mut vm.arena, number)?;
            vm.push(string);
            Ok(true)
        }
        x if x == Op::StrFromF64 as u8 => {
            let n = vm.pop()?;
            let r = crate::host_ext::str_from_f64(&mut vm.arena, n)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::StrFromByte as u8 => {
            let value = vm.pop()?;
            let byte = vm.as_i64(value)?;
            let r = crate::host_ext::str_from_byte(&mut vm.arena, byte)?;
            vm.push(r);
            Ok(true)
        }
        x if x == Op::EmptyStr as u8 => {
            let v = vm.arena.alloc(HeapObj::Str(String::new()))?;
            vm.push(v);
            Ok(true)
        }
        _ => Ok(false),
    }
}
