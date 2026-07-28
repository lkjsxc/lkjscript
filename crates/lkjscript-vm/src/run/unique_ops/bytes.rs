use super::*;

pub(super) fn handles(op: u8) -> bool {
    matches!(
        Op::from_byte(op),
        Some(
            Op::BytesLength
                | Op::BytesByteAt
                | Op::CopyBytesSlice
                | Op::CloneBytes
                | Op::FreezeByteVector
                | Op::ThawBytes
                | Op::BytesDropPlace
                | Op::BytesPlaceEnd
                | Op::BytesPlaceInit
                | Op::BytesMove
                | Op::BytesBorrow
        )
    )
}

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: u8) -> Result<()> {
    match Op::from_byte(op).ok_or_else(|| Error::msg("unknown bytes opcode"))? {
        Op::BytesLength => {
            let value = vm.pop()?;
            let len = if let Some(bytes) = static_bytes(vm.chunk, value)? {
                bytes.len()
            } else {
                vm.unique.bytes_length(value)?
            };
            vm.push(Value::from_i64(
                i64::try_from(len).map_err(|_| Error::msg("bytes length exceeds i64"))?,
            ));
        }
        Op::BytesByteAt => {
            let index_value = vm.pop()?;
            let value = vm.pop()?;
            let raw = vm.as_i64(index_value)?;
            let index = usize::try_from(raw)
                .map_err(|_| Error::msg(format!("bytes-byte-at index {raw} is negative")))?;
            let byte = if let Some(bytes) = static_bytes(vm.chunk, value)? {
                bytes.get(index).copied().ok_or_else(|| {
                    Error::msg(format!("bytes-byte-at index {index} out of range"))
                })?
            } else {
                vm.unique.bytes_at(value, index)?
            };
            vm.push(Value::from_i64(i64::from(byte)));
        }
        Op::CloneBytes => {
            let value = vm.pop()?;
            let result = if let Some(bytes) = static_bytes(vm.chunk, value)? {
                vm.unique.clone_static(bytes)?
            } else {
                vm.unique.clone_bytes(value)?
            };
            vm.push(result);
        }
        Op::CopyBytesSlice => {
            let len_value = vm.pop()?;
            let start_value = vm.pop()?;
            let value = vm.pop()?;
            let raw_start = vm.as_i64(start_value)?;
            let raw_len = vm.as_i64(len_value)?;
            let start = usize::try_from(raw_start).map_err(|_| {
                Error::msg(format!("copy-bytes-slice start {raw_start} is negative"))
            })?;
            let len = usize::try_from(raw_len).map_err(|_| {
                Error::msg(format!("copy-bytes-slice length {raw_len} is negative"))
            })?;
            let result = if let Some(bytes) = static_bytes(vm.chunk, value)? {
                vm.unique.copy_static_range(bytes, start, len)?
            } else {
                vm.unique.copy_bytes_range(value, start, len)?
            };
            vm.push(result);
        }
        Op::FreezeByteVector => {
            let value = vm.pop()?;
            let result = vm.unique.freeze(value)?;
            vm.push(result);
        }
        Op::ThawBytes => {
            let value = vm.pop()?;
            let result = if let Some(bytes) = static_bytes(vm.chunk, value)? {
                vm.unique.thaw_static(bytes)?
            } else {
                vm.unique.thaw_dynamic(value)?
            };
            vm.push(result);
        }
        Op::BytesPlaceInit => {
            let (place, slot) = place_and_slot(vm)?;
            let value = local(vm, slot)?;
            let owner = vm.unique.validate_any_owner(value)?;
            if value.as_bytes_key().is_none() {
                return Err(Error::msg("bytes place init has wrong layout"));
            }
            let target = place_mut(vm, place)?;
            if !matches!(
                target,
                unique::RuntimePlace::Inactive
                    | unique::RuntimePlace::Active {
                        owner: None,
                        transferred: None
                    }
            ) {
                return Err(Error::msg("bytes place already initialized"));
            }
            *target = unique::RuntimePlace::Active {
                owner: Some(owner),
                transferred: None,
            };
            vm.push(Value::UNIT);
        }
        Op::BytesMove => {
            let (place, slot) = place_and_slot(vm)?;
            let value = local(vm, slot)?;
            let owner = vm.unique.ensure_any_unloaned(value)?;
            if value.as_bytes_key().is_none() {
                return Err(Error::msg("bytes move has wrong layout"));
            }
            expect_place(vm, place, owner)?;
            clear_local(vm, slot)?;
            *place_mut(vm, place)? = unique::RuntimePlace::Active {
                owner: None,
                transferred: Some(owner),
            };
            vm.push(value);
        }
        Op::BytesBorrow => {
            let slot = usize::from(vm.read_u8()?);
            let value = local(vm, slot)?;
            let owner = vm.unique.validate_any_owner(value)?;
            if value.as_bytes_key().is_none() || !current_places(vm).iter().any(|place| matches!(place, unique::RuntimePlace::Active { owner: Some(actual), .. } if *actual == owner)) { return Err(Error::msg("bytes borrow source is not current owner")); }
            let borrowed = vm.unique.borrow_bytes(value)?;
            vm.push(borrowed);
        }
        Op::BytesDropPlace => {
            let (place, slot) = place_and_slot(vm)?;
            let value = local(vm, slot)?;
            let owner = vm.unique.ensure_any_unloaned(value)?;
            let exact = current_places(vm).get(place).is_some_and(|item| matches!(item, unique::RuntimePlace::Active { owner: Some(actual), .. } | unique::RuntimePlace::Active { transferred: Some(actual), .. } if *actual == owner));
            if !exact {
                return Err(Error::msg("bytes Drop does not name exact owner"));
            }
            clear_local(vm, slot)?;
            vm.unique.drop_owner(value)?;
            *place_mut(vm, place)? = unique::RuntimePlace::Active {
                owner: None,
                transferred: None,
            };
            vm.push(Value::UNIT);
        }
        Op::BytesPlaceEnd => {
            let place = usize::from(vm.read_u8()?);
            let target = place_mut(vm, place)?;
            match *target {
                unique::RuntimePlace::Active { owner: None, .. } => {
                    *target = unique::RuntimePlace::Inactive
                }
                unique::RuntimePlace::Active { owner: Some(_), .. } => {
                    return Err(Error::msg("bytes PlaceEnd missing Drop"))
                }
                unique::RuntimePlace::Inactive => {
                    return Err(Error::msg("bytes place already ended"))
                }
            }
            vm.push(Value::UNIT);
        }
        _ => unreachable!("bytes opcode checked"),
    }
    Ok(())
}

fn static_bytes(chunk: &lkjscript_core::ValidatedChunk, value: Value) -> Result<Option<&[u8]>> {
    let Some(index) = value.as_static_bytes() else {
        return Ok(None);
    };
    match chunk.constants().get(usize::from(index)) {
        Some(lkjscript_core::Constant::StaticBytes(bytes)) => Ok(Some(bytes)),
        _ => Err(Error::msg(
            "static bytes constant index is stale or wrong-layout",
        )),
    }
}
