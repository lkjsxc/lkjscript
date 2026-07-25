use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<bool> {
    match op {
        x if x == Op::OkWrap as u8 => {
            let v = vm.pop()?;
            let __r = crate::host_ext::result_ok(&mut vm.arena, v)?;
            vm.push(__r);
            Ok(true)
        }
        x if x == Op::ErrWrap as u8 => {
            let v = vm.pop()?;
            let __r = crate::host_ext::result_err(&mut vm.arena, v)?;
            vm.push(__r);
            Ok(true)
        }
        x if x == Op::IsOk as u8 => {
            let v = vm.pop()?;
            vm.push(crate::host_ext::is_ok(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::UnwrapOk as u8 => {
            let v = vm.pop()?;
            vm.push(crate::host_ext::unwrap_ok(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::UnwrapErr as u8 => {
            let v = vm.pop()?;
            vm.push(crate::host_ext::unwrap_err(&vm.arena, v)?);
            Ok(true)
        }
        x if x == Op::SomeWrap as u8 => {
            let value = vm.pop()?;
            let wrapped = crate::host_ext::option_some(&mut vm.arena, value)?;
            vm.push(wrapped);
            Ok(true)
        }
        x if x == Op::IsSome as u8 => {
            let value = vm.pop()?;
            vm.push(crate::host_ext::is_some(&vm.arena, value)?);
            Ok(true)
        }
        x if x == Op::UnwrapSome as u8 => {
            let value = vm.pop()?;
            vm.push(crate::host_ext::unwrap_some(&vm.arena, value)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}
