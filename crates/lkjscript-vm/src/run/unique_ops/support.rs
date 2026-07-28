use super::super::*;

pub(super) fn place_and_slot<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<(usize, usize)> {
    let packed = vm.read_u16()?;
    Ok((usize::from(packed >> 8), usize::from(packed as u8)))
}

pub(super) fn local<J: RuntimeTier>(vm: &Vm<'_, J>, slot: usize) -> Result<Value> {
    let base = vm
        .frames
        .last()
        .ok_or_else(|| Error::msg("unique local access without frame"))?
        .locals_base;
    let value = vm
        .stack
        .get(base + slot)
        .copied()
        .ok_or_else(|| Error::msg("unique local index out of range"))?;
    if value.is_invalid() {
        Err(Error::msg("unique local is moved or uninitialized"))
    } else {
        Ok(value)
    }
}

pub(super) fn clear_local<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    let base = vm
        .frames
        .last()
        .ok_or_else(|| Error::msg("unique local clear without frame"))?
        .locals_base;
    let target = vm
        .stack
        .get_mut(base + slot)
        .ok_or_else(|| Error::msg("unique local index out of range"))?;
    *target = Value::INVALID;
    Ok(())
}

pub(super) fn store_empty_local<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    slot: usize,
    value: Value,
) -> Result<()> {
    let base = vm
        .frames
        .last()
        .ok_or_else(|| Error::msg("unique local store without frame"))?
        .locals_base;
    let target = vm
        .stack
        .get_mut(base + slot)
        .ok_or_else(|| Error::msg("unique local index out of range"))?;
    if !target.is_invalid() {
        return Err(Error::msg(
            "unique local overwrite would leak or forge a value",
        ));
    }
    *target = value;
    Ok(())
}

pub(super) fn current_places<'a, J: RuntimeTier>(vm: &'a Vm<'_, J>) -> &'a [unique::RuntimePlace] {
    vm.frames
        .last()
        .map(|frame| frame.unique_places.as_slice())
        .unwrap_or_default()
}

pub(super) fn place_mut<'a, J: RuntimeTier>(
    vm: &'a mut Vm<'_, J>,
    place: usize,
) -> Result<&'a mut unique::RuntimePlace> {
    vm.frames
        .last_mut()
        .and_then(|frame| frame.unique_places.get_mut(place))
        .ok_or_else(|| Error::msg("VM byte-vector place index out of range"))
}

pub(super) fn expect_place<J: RuntimeTier>(vm: &Vm<'_, J>, place: usize, owner: u64) -> Result<()> {
    if current_places(vm).get(place)
        == Some(&unique::RuntimePlace::Active {
            owner: Some(owner),
            transferred: None,
        })
    {
        Ok(())
    } else {
        Err(Error::msg(
            "VM byte-vector place does not hold the named owner",
        ))
    }
}
