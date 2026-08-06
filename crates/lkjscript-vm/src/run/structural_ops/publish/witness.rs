use super::*;

pub(super) fn compare<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let parameter = u64::try_from(vm.read_index()?)
        .map_err(|_| Error::msg("memory compare witness parameter exceeds u64"))?;
    let binding = vm
        .frames
        .last()
        .and_then(|frame| {
            frame
                .memory_witnesses
                .iter()
                .find(|binding| binding.parameter == parameter)
        })
        .ok_or_else(|| Error::msg("memory compare witness is missing"))?;
    let witness = vm
        .chunk
        .memory_witnesses()
        .get(
            usize::try_from(binding.witness)
                .map_err(|_| Error::msg("memory compare witness slot exceeds host usize"))?,
        )
        .ok_or_else(|| Error::msg("memory compare witness slot is stale"))?;
    if witness
        .facts
        .operations
        .binary_search(&lkjscript_core::MemoryWitnessOperation::Compare)
        .is_err()
    {
        return Err(Error::msg("memory witness rejects compare"));
    }
    let right = vm.pop()?;
    let left = vm.pop()?;
    let equal = crate::run::data::value_equal(vm, left, right)?;
    vm.push(Value::from_bool(equal));
    Ok(())
}

pub(super) fn dispose_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let parameter = u64::try_from(vm.read_index()?)
        .map_err(|_| Error::msg("memory dispose witness parameter exceeds u64"))?;
    let binding = vm
        .frames
        .last()
        .and_then(|frame| {
            frame
                .memory_witnesses
                .iter()
                .find(|binding| binding.parameter == parameter)
        })
        .ok_or_else(|| Error::msg("memory dispose witness is missing"))?;
    let witness = vm
        .chunk
        .memory_witnesses()
        .get(
            usize::try_from(binding.witness)
                .map_err(|_| Error::msg("memory dispose witness slot exceeds host usize"))?,
        )
        .ok_or_else(|| Error::msg("memory dispose witness slot is stale"))?;
    if witness
        .facts
        .operations
        .binary_search(&lkjscript_core::MemoryWitnessOperation::Dispose)
        .is_err()
    {
        return Err(Error::msg("memory witness rejects dispose"));
    }
    let source = vm.pop()?;
    if matches!(
        witness.value_kind,
        lkjscript_core::MemoryWitnessValueKind::Structural(_)
    ) {
        let (key, record) = invocation(vm)?.owner(source)?;
        invocation_mut(vm)?
            .runtime
            .dispose_owner(key, record.value_type)
            .map_err(map_value_error)?;
        invocation_mut(vm)?.owners.remove(&key.get());
    } else if witness.value_kind == lkjscript_core::MemoryWitnessValueKind::Unsupported {
        return Err(Error::msg("memory witness cannot dispose owner"));
    }
    vm.push(Value::UNIT);
    Ok(())
}

pub(super) fn independent_owner<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let parameter = u64::try_from(vm.read_index()?)
        .map_err(|_| Error::msg("memory independent-owner witness parameter exceeds u64"))?;
    let binding = vm
        .frames
        .last()
        .and_then(|frame| {
            frame
                .memory_witnesses
                .iter()
                .find(|binding| binding.parameter == parameter)
        })
        .ok_or_else(|| Error::msg("memory independent-owner witness is missing"))?;
    let witness =
        vm.chunk
            .memory_witnesses()
            .get(usize::try_from(binding.witness).map_err(|_| {
                Error::msg("memory independent-owner witness slot exceeds host usize")
            })?)
            .ok_or_else(|| Error::msg("memory independent-owner witness slot is stale"))?;
    if witness
        .facts
        .operations
        .binary_search(&lkjscript_core::MemoryWitnessOperation::IndependentOwner)
        .is_err()
    {
        return Err(Error::msg("memory witness rejects independent-owner"));
    }
    let value_kind = witness.value_kind;
    let source = vm.pop()?;
    match value_kind {
        lkjscript_core::MemoryWitnessValueKind::Structural(_) => {
            let (key, record) = invocation(vm)?.owner(source)?;
            let copied = invocation_mut(vm)?
                .runtime
                .independent_owner(key, record.value_type)
                .map_err(map_value_error)?;
            let copied = invocation_mut(vm)?.register_owner(
                copied,
                record.representation,
                record.value_type,
            )?;
            vm.push(copied);
        }
        lkjscript_core::MemoryWitnessValueKind::Unsupported => {
            return Err(Error::msg("memory witness cannot create independent owner"));
        }
        _ => vm.push(source),
    }
    Ok(())
}
