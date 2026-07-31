use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: lkjscript_core::Op) -> Result<()> {
    let slot = usize::from(vm.read_u8()?);
    match op {
        lkjscript_core::Op::StoreStructuralLocal => store(vm, slot),
        lkjscript_core::Op::TakeStructuralLocal => take(vm, slot),
        lkjscript_core::Op::LoadStructuralViewLocal => load_view(vm, slot),
        lkjscript_core::Op::EndStructuralBorrowLocal => end_view(vm, slot),
        lkjscript_core::Op::LoadStructuralOwnerLocal => load_owner(vm, slot),
        _ => Err(Error::msg("structural local opcode dispatch mismatch")),
    }
}

fn store<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    let value = vm.pop()?;
    {
        let structural = invocation(vm)?;
        if let Some(key) = value.as_structural_root() {
            if !structural.owners.contains_key(&key.get())
                && !structural.host_owners.contains_key(&key.get())
            {
                return Err(Error::msg(
                    "structural store received an unregistered owner",
                ));
            }
        } else if value.as_structural_view().is_some() {
            let _ = structural.view(value)?;
        } else if value.as_structural_destination().is_some() {
            let _ = structural.destination(value)?;
        } else {
            return Err(Error::msg(
                "structural store received a non-structural value",
            ));
        }
    }
    let absolute = local_index(vm, slot)?;
    let previous = vm.stack.get(absolute).copied();
    let occurrences = previous.map_or(0, |previous| {
        vm.stack
            .iter()
            .filter(|candidate| **candidate == previous)
            .count()
    });
    if let Some(previous) = previous {
        if !previous.is_invalid() && previous != value && occurrences <= 1 {
            if previous.as_structural_root().is_none()
                || values::is_host_owner(invocation(vm)?, previous)
            {
                return Err(Error::msg(
                    "structural local overwrite would leak or forge a value",
                ));
            }
            adapter::drop_registered_owner(vm, previous)?;
        }
    }
    let target = vm
        .stack
        .get_mut(absolute)
        .ok_or_else(|| Error::msg("structural local index is out of range"))?;
    if !target.is_invalid() {
        *target = value;
        return commit_handoff(vm, value);
    }
    *target = value;
    commit_handoff(vm, value)
}

fn take<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    let absolute = local_index(vm, slot)?;
    let value = vm
        .stack
        .get(absolute)
        .copied()
        .ok_or_else(|| Error::msg("structural local index is out of range"))?;
    if value.is_invalid() {
        return Err(Error::msg("structural local is moved or uninitialized"));
    }
    let structural = invocation_mut(vm)?;
    if let Some(key) = value.as_structural_root() {
        if let Some(record) = structural.owners.get_mut(&key.get()) {
            if record.taken_from.replace(absolute).is_some() {
                return Err(Error::msg("structural owner has two active handoffs"));
            }
        } else if !structural.host_owners.contains_key(&key.get()) {
            return Err(Error::msg("structural owner local is stale or forged"));
        }
    } else if let Some(word) = value.as_structural_destination() {
        let record = structural
            .destinations
            .get_mut(&word)
            .ok_or_else(|| Error::msg("structural destination local is stale or forged"))?;
        if record.taken_from.replace(absolute).is_some() {
            return Err(Error::msg("structural destination has two active handoffs"));
        }
    } else {
        return Err(Error::msg(
            "structural take expects an owner or destination",
        ));
    }
    vm.stack[absolute] = Value::INVALID;
    vm.push(value);
    Ok(())
}

fn load_view<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    let value = local(vm, slot)?;
    let _ = invocation(vm)?.view(value)?;
    vm.push(value);
    Ok(())
}

fn end_view<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    let absolute = local_index(vm, slot)?;
    let value = local(vm, slot)?;
    let (key, _) = invocation(vm)?.view(value)?;
    invocation_mut(vm)?
        .runtime
        .end_view(key)
        .map_err(map_value_error)?;
    let word = value
        .as_structural_view()
        .ok_or_else(|| Error::msg("structural view changed category during EndBorrow"))?;
    invocation_mut(vm)?.views.remove(&word);
    vm.stack[absolute] = Value::INVALID;
    vm.push(Value::UNIT);
    Ok(())
}

fn load_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    let value = local(vm, slot)?;
    let key = value
        .as_structural_root()
        .ok_or_else(|| Error::msg("structural owner local changed category"))?;
    let structural = invocation(vm)?;
    if !structural.owners.contains_key(&key.get())
        && !structural.host_owners.contains_key(&key.get())
    {
        return Err(Error::msg("structural owner local is stale or forged"));
    }
    vm.push(value);
    Ok(())
}

pub(super) fn local<J: RuntimeTier>(vm: &Vm<'_, J>, slot: usize) -> Result<Value> {
    let absolute = local_index(vm, slot)?;
    let value = vm
        .stack
        .get(absolute)
        .copied()
        .ok_or_else(|| Error::msg("structural local index is out of range"))?;
    if value.is_invalid() {
        Err(Error::msg("structural local is moved or uninitialized"))
    } else {
        Ok(value)
    }
}

pub(super) fn clear_local<J: RuntimeTier>(vm: &mut Vm<'_, J>, slot: usize) -> Result<()> {
    replace_local(vm, slot, Value::INVALID)
}

pub(super) fn replace_local<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    slot: usize,
    value: Value,
) -> Result<()> {
    let absolute = local_index(vm, slot)?;
    let target = vm
        .stack
        .get_mut(absolute)
        .ok_or_else(|| Error::msg("structural local index is out of range"))?;
    *target = value;
    Ok(())
}

fn local_index<J: RuntimeTier>(vm: &Vm<'_, J>, slot: usize) -> Result<usize> {
    vm.frames
        .last()
        .and_then(|frame| frame.locals_base.checked_add(slot))
        .ok_or_else(|| Error::msg("structural local access without a valid frame"))
}

include!("locals/handoffs.rs");
include!("locals/calls.rs");
include!("locals/returns.rs");
