use super::*;

#[path = "bytes.rs"]
mod bytes_ops;
#[path = "support.rs"]
mod support;
use support::*;

pub(super) fn handles(op: u8) -> bool {
    bytes_ops::handles(op)
        || matches!(
            Op::from_byte(op),
            Some(
                Op::ByteVectorNew
                    | Op::ByteVectorPlaceInit
                    | Op::ByteVectorMove
                    | Op::ByteVectorBorrow
                    | Op::ByteVectorBorrowMut
                    | Op::StoreUniqueLocal
                    | Op::StoreViewLocal
                    | Op::TakeUniqueLocal
                    | Op::LoadViewLocal
                    | Op::ByteVectorDropPlace
                    | Op::ByteVectorPlaceEnd
                    | Op::ByteSliceLen
                    | Op::ByteSliceRef
                    | Op::ByteSliceMutSet
                    | Op::EndBorrowLocal
            )
        )
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    if bytes_ops::handles(op) {
        return bytes_ops::dispatch(vm, op);
    }
    let op = Op::from_byte(op).ok_or_else(|| Error::msg("unknown unique opcode"))?;
    match op {
        Op::ByteVectorNew => {
            let size = vm.pop()?;
            let size = vm.as_i64(size)?;
            let owner = vm.unique.allocate(size)?;
            vm.push(owner);
        }
        Op::ByteVectorPlaceInit => {
            let (place, slot) = place_and_slot(vm)?;
            let value = local(vm, slot)?;
            let owner = vm.unique.validate_owner(value)?;
            let target = place_mut(vm, place)?;
            match *target {
                unique::RuntimePlace::Inactive
                | unique::RuntimePlace::Active {
                    owner: None,
                    transferred: None,
                } => {
                    *target = unique::RuntimePlace::Active {
                        owner: Some(owner),
                        transferred: None,
                    };
                }
                unique::RuntimePlace::Active { .. } => {
                    return Err(Error::msg("VM byte-vector place is already initialized"));
                }
            }
            vm.push(Value::UNIT);
        }
        Op::ByteVectorMove => {
            let (place, slot) = place_and_slot(vm)?;
            let value = local(vm, slot)?;
            let owner = vm.unique.ensure_unloaned(value)?;
            expect_place(vm, place, owner)?;
            clear_local(vm, slot)?;
            *place_mut(vm, place)? = unique::RuntimePlace::Active {
                owner: None,
                transferred: Some(owner),
            };
            vm.push(value);
        }
        Op::ByteVectorBorrow | Op::ByteVectorBorrowMut => {
            let slot = usize::from(vm.read_u8()?);
            let owner = local(vm, slot)?;
            let word = vm.unique.validate_owner(owner)?;
            if !current_places(vm).iter().any(
                |place| matches!(place, unique::RuntimePlace::Active { owner: Some(value), .. } if *value == word),
            ) {
                return Err(Error::msg("VM borrow source is not a current whole-place owner"));
            }
            let view = vm.unique.borrow(owner, op == Op::ByteVectorBorrowMut)?;
            vm.push(view);
        }
        Op::StoreUniqueLocal => {
            let slot = usize::from(vm.read_u8()?);
            let value = vm.pop()?;
            vm.unique.validate_any_owner(value)?;
            if local(vm, slot).is_ok_and(|existing| existing.as_static_bytes().is_some()) {
                clear_local(vm, slot)?;
            }
            store_empty_local(vm, slot, value)?;
        }
        Op::StoreViewLocal => {
            let slot = usize::from(vm.read_u8()?);
            let value = vm.pop()?;
            vm.unique.validate_any_view(value)?;
            store_empty_local(vm, slot, value)?;
        }
        Op::TakeUniqueLocal => {
            let slot = usize::from(vm.read_u8()?);
            let value = local(vm, slot)?;
            vm.unique.ensure_any_unloaned(value)?;
            clear_local(vm, slot)?;
            vm.push(value);
        }
        Op::LoadViewLocal => {
            let slot = usize::from(vm.read_u8()?);
            let value = local(vm, slot)?;
            vm.unique.validate_any_view(value)?;
            vm.push(value);
        }
        Op::ByteVectorDropPlace => {
            let (place, slot) = place_and_slot(vm)?;
            let value = local(vm, slot)?;
            let owner = vm.unique.ensure_unloaned(value)?;
            let exact = current_places(vm).get(place).is_some_and(|item| {
                matches!(
                    item,
                    unique::RuntimePlace::Active { owner: Some(value), .. }
                        | unique::RuntimePlace::Active { transferred: Some(value), .. }
                        if *value == owner
                )
            });
            if !exact {
                return Err(Error::msg(
                    "VM byte-vector Drop does not name current or transferred owner",
                ));
            }
            clear_local(vm, slot)?;
            vm.unique.drop_owner(value)?;
            *place_mut(vm, place)? = unique::RuntimePlace::Active {
                owner: None,
                transferred: None,
            };
            vm.push(Value::UNIT);
        }
        Op::ByteVectorPlaceEnd => {
            let place = usize::from(vm.read_u8()?);
            let target = place_mut(vm, place)?;
            match *target {
                unique::RuntimePlace::Active { owner: None, .. } => {
                    *target = unique::RuntimePlace::Inactive;
                }
                unique::RuntimePlace::Active { owner: Some(_), .. } => {
                    return Err(Error::msg("VM byte-vector PlaceEnd is missing Drop"));
                }
                unique::RuntimePlace::Inactive => {
                    return Err(Error::msg("VM byte-vector place is already ended"));
                }
            }
            vm.push(Value::UNIT);
        }
        Op::ByteSliceLen => {
            let view = vm.pop()?;
            let len = vm.unique.len(view)?;
            vm.push(Value::from_i64(len));
        }
        Op::ByteSliceRef => {
            let index = vm.pop()?;
            let view = vm.pop()?;
            let index = vm.as_i64(index)?;
            let byte = vm.unique.byte_at(view, index)?;
            vm.push(Value::from_i64(byte));
        }
        Op::ByteSliceMutSet => {
            let byte = vm.pop()?;
            let index = vm.pop()?;
            let view = vm.pop()?;
            let byte = vm.as_i64(byte)?;
            let index = vm.as_i64(index)?;
            vm.unique.set_byte(view, index, byte)?;
            vm.push(Value::UNIT);
        }
        Op::EndBorrowLocal => {
            let slot = usize::from(vm.read_u8()?);
            let view = local(vm, slot)?;
            clear_local(vm, slot)?;
            vm.unique.end_borrow(view)?;
            vm.push(Value::UNIT);
        }
        _ => unreachable!("unique opcode family checked"),
    }
    Ok(())
}
