use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: lkjscript_core::Op) -> Result<()> {
    match op {
        lkjscript_core::Op::StructuralPlaceInit => place_init(vm),
        lkjscript_core::Op::StructuralMove => move_owner(vm),
        lkjscript_core::Op::StructuralDropPlace => drop_owner(vm),
        lkjscript_core::Op::StructuralPlaceEnd => place_end(vm),
        _ => Err(Error::msg("structural place opcode dispatch mismatch")),
    }
}

fn place_init<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let (place, slot) = place_and_slot(vm)?;
    let value = locals::local(vm, slot)?;
    let (mut key, record) = invocation(vm)?.owner(value)?;
    let mode = vm
        .chunk
        .structural_representations()
        .get(record.representation.index())
        .and_then(|representation| {
            vm.chunk
                .structural_types()
                .get(representation.type_id.index())
        })
        .map(|ty| ty.mode);
    let duplicate = current_places(vm)
        .iter()
        .enumerate()
        .find_map(|(index, item)| {
            (index != place
                && matches!(
                    item,
                    unique::RuntimePlace::Active { owner: Some(owner), .. }
                        if *owner == key.get()
                ))
            .then_some(index)
        });
    if duplicate.is_some() {
        if !matches!(
            mode,
            Some(
                lkjscript_core::StructuralTypeMode::Copy
                    | lkjscript_core::StructuralTypeMode::Immutable
            )
        ) {
            return Err(Error::msg(
                "structural owner is already bound to another place",
            ));
        }
        let copied = invocation_mut(vm)?
            .runtime
            .clone_owned(key, record.value_type)
            .map_err(map_value_error)?;
        let copied_value =
            invocation_mut(vm)?.register_owner(copied, record.representation, record.value_type)?;
        locals::replace_local(vm, slot, copied_value)?;
        key = copied;
    }
    let target = place_mut(vm, place)?;
    match *target {
        unique::RuntimePlace::Inactive
        | unique::RuntimePlace::Active {
            owner: None,
            transferred: None,
        } => {
            *target = unique::RuntimePlace::Active {
                owner: Some(key.get()),
                transferred: None,
            };
        }
        unique::RuntimePlace::Active { .. } => {
            return Err(Error::msg("structural place is already initialized"));
        }
    }
    vm.push(Value::UNIT);
    Ok(())
}

fn move_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let (place, slot) = place_and_slot(vm)?;
    let value = locals::local(vm, slot)?;
    let (key, record) = invocation(vm)?.owner(value)?;
    require_place(vm, place, key.get())?;
    let next = invocation_mut(vm)?
        .runtime
        .move_owned(key, record.value_type)
        .map_err(map_value_error)?;
    locals::clear_local(vm, slot)?;
    let removed = invocation_mut(vm)?
        .owners
        .remove(&key.get())
        .ok_or_else(|| Error::msg("moved structural owner disappeared from registry"))?;
    let next_value =
        invocation_mut(vm)?.register_owner(next, removed.representation, removed.value_type)?;
    *place_mut(vm, place)? = unique::RuntimePlace::Active {
        owner: None,
        transferred: Some(next.get()),
    };
    vm.push(next_value);
    Ok(())
}

fn drop_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let (place, slot) = place_and_slot(vm)?;
    let value = locals::local(vm, slot)?;
    let (key, record) = invocation(vm)?.owner(value)?;
    let exact = current_places(vm).get(place).is_some_and(|item| {
        matches!(
            item,
            unique::RuntimePlace::Active { owner: Some(owner), .. }
                | unique::RuntimePlace::Active { transferred: Some(owner), .. }
                if *owner == key.get()
        )
    });
    if !exact {
        return Err(Error::msg(
            "structural Drop does not name the current or transferred place owner",
        ));
    }
    invocation_mut(vm)?
        .runtime
        .drop_owned(key, record.value_type)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.owners.remove(&key.get());
    locals::clear_local(vm, slot)?;
    *place_mut(vm, place)? = unique::RuntimePlace::Active {
        owner: None,
        transferred: None,
    };
    vm.push(Value::UNIT);
    Ok(())
}

fn place_end<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let place = usize::from(vm.read_u8()?);
    let target = place_mut(vm, place)?;
    match *target {
        unique::RuntimePlace::Active { owner: None, .. } => {
            *target = unique::RuntimePlace::Inactive;
        }
        unique::RuntimePlace::Active { owner: Some(_), .. } => {
            return Err(Error::msg("structural PlaceEnd is missing Drop or Move"));
        }
        unique::RuntimePlace::Inactive => {
            return Err(Error::msg("structural place is already ended"));
        }
    }
    vm.push(Value::UNIT);
    Ok(())
}

include!("locals/place_state.rs");
