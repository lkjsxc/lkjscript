use super::*;

pub(super) mod display;
mod equality;
mod lists;
mod stack;
#[cfg(test)]
pub(crate) use equality::list_values_equal;
pub(crate) use equality::{equal_value, f64_bits_equal, list_equal, same_object, value_equal};

pub(super) fn handles(op: u8) -> bool {
    op == Op::Nop as u8
        || op == Op::LoadConst as u8
        || op == Op::LoadLocal as u8
        || op == Op::StoreLocal as u8
        || op == Op::LoadGlobal as u8
        || op == Op::StoreGlobal as u8
        || op == Op::Cons as u8
        || op == Op::Car as u8
        || op == Op::Cdr as u8
        || op == Op::IsEmptyList as u8
        || op == Op::SameObject as u8
        || op == Op::ListEqual as u8
        || op == Op::Pop as u8
        || op == Op::Dup as u8
        || op == Op::False as u8
        || op == Op::True as u8
        || op == Op::Unit as u8
        || op == Op::EmptyList as u8
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match op {
        x if x == Op::Nop as u8 => Ok(()),
        x if x == Op::LoadConst as u8 => {
            let id = vm.read_u16()? as usize;
            let v = vm.load_const(id)?;
            vm.push(v);
            Ok(())
        }
        x if x == Op::LoadLocal as u8 => {
            let slot = vm.read_index()?;
            let base = vm
                .frames
                .last()
                .ok_or_else(|| Error::msg("LoadLocal without frame"))?
                .locals_base;
            let value = vm
                .stack
                .get(
                    base.checked_add(slot)
                        .ok_or_else(|| Error::msg("LoadLocal slot overflow"))?,
                )
                .copied()
                .ok_or_else(|| Error::msg("LoadLocal slot out of range"))?;
            if value.is_invalid() {
                return Err(Error::msg("LoadLocal read uninitialized slot"));
            }
            vm.push(value);
            Ok(())
        }
        x if x == Op::StoreLocal as u8 => {
            let slot = vm.read_index()?;
            let base = vm
                .frames
                .last()
                .ok_or_else(|| Error::msg("StoreLocal without frame"))?
                .locals_base;
            let value = vm.peek()?;
            let borrowed_resource = vm.resources.is_borrowed_handle(value)
                || vm
                    .frames
                    .last()
                    .is_some_and(|frame| frame.borrowed_resources.contains(&value));
            let resource = value.as_aggregate_adapter().is_some()
                || (value.as_resource().is_some() && !borrowed_resource);
            let target = vm
                .stack
                .get_mut(
                    base.checked_add(slot)
                        .ok_or_else(|| Error::msg("StoreLocal slot overflow"))?,
                )
                .ok_or_else(|| Error::msg("StoreLocal slot out of range"))?;
            if resource && !target.is_invalid() && *target != value {
                return Err(Error::msg(
                    "StoreLocal would overwrite a distinct live affine resource",
                ));
            }
            *target = value;
            if resource {
                let top = vm
                    .stack
                    .last_mut()
                    .ok_or_else(|| Error::msg("StoreLocal lost its resource operand"))?;
                *top = Value::UNIT;
            }
            Ok(())
        }
        x if x == Op::LoadGlobal as u8 => {
            let id = vm.read_u16()? as usize;
            let value = vm
                .globals
                .get(id)
                .copied()
                .ok_or_else(|| Error::msg("LoadGlobal slot out of range"))?;
            if value.is_invalid() {
                return Err(Error::msg("LoadGlobal read uninitialized slot"));
            }
            vm.push(value);
            Ok(())
        }
        x if x == Op::StoreGlobal as u8 => {
            let id = vm.read_u16()? as usize;
            let value = vm.peek()?;
            let target = vm
                .globals
                .get_mut(id)
                .ok_or_else(|| Error::msg("StoreGlobal slot out of range"))?;
            *target = value;
            Ok(())
        }
        x if x == Op::Cons as u8 => {
            let tail = vm.pop()?;
            let head = vm.pop()?;
            let value = vm.list_prepend(head, tail)?;
            vm.push(value);
            Ok(())
        }
        x if x == Op::Car as u8 => {
            let representation = vm.read_u16()?;
            car(
                vm,
                (representation != u16::MAX)
                    .then(|| lkjscript_core::StructuralRepresentationId::new(representation)),
            )
        }
        x if x == Op::Cdr as u8 => cdr(vm),
        x if x == Op::IsEmptyList as u8 => {
            let value = vm.pop()?;
            let empty = vm.list_is_empty(value)?;
            vm.push(Value::from_bool(empty));
            Ok(())
        }
        x if x == Op::SameObject as u8 => same_object(vm),
        x if x == Op::ListEqual as u8 => list_equal(vm),
        x if x == Op::Pop as u8 => {
            let value = vm.pop()?;
            if value.as_structural_root().is_some() {
                return crate::run::structural_ops::drop_registered_owner(vm, value);
            }
            Ok(())
        }
        x if x == Op::Dup as u8 => {
            let v = vm.peek()?;
            vm.push(v);
            Ok(())
        }
        x if x == Op::False as u8 => {
            vm.push(Value::FALSE);
            Ok(())
        }
        x if x == Op::True as u8 => {
            vm.push(Value::TRUE);
            Ok(())
        }
        x if x == Op::Unit as u8 => {
            vm.push(Value::UNIT);
            Ok(())
        }
        x if x == Op::EmptyList as u8 => {
            vm.push(Value::EMPTY_LIST);
            Ok(())
        }
        _ => unreachable!("opcode family checked"),
    }
}
