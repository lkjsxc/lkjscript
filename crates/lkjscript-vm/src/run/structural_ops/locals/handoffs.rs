pub(in crate::run) fn commit_handoff(
    vm: &mut Vm<'_>,
    value: Value,
) -> Result<()> {
    let Some(structural) = vm.structural.as_mut() else {
        return Ok(());
    };
    if let Some(key) = value.as_structural_root() {
        if let Some(record) = structural.owners.get_mut(&key.get()) {
            record.taken_from = None;
        } else if !structural.host_owners.contains_key(&key.get()) {
            return Err(Error::msg("committed structural owner is not registered"));
        }
    } else if let Some(word) = value.as_structural_destination() {
        let record = structural
            .destinations
            .get_mut(&word)
            .ok_or_else(|| Error::msg("committed structural destination is not registered"))?;
        record.taken_from = None;
    }
    Ok(())
}

pub(in crate::run) fn restore_handoffs(vm: &mut Vm<'_>) -> Result<()> {
    let Some(structural) = vm.structural.as_ref() else {
        return Ok(());
    };
    let owners: Vec<_> = structural
        .owners
        .iter()
        .filter_map(|(word, record)| record.taken_from.map(|index| (*word, index)))
        .collect();
    let destinations: Vec<_> = structural
        .destinations
        .iter()
        .filter_map(|(word, record)| record.taken_from.map(|index| (*word, index)))
        .collect();
    for (word, index) in &owners {
        restore_value(
            &mut vm.stack,
            *index,
            StructuralValueKey::from_word(*word)
                .map(Value::from_structural_root)
                .ok_or_else(|| Error::msg("structural owner handoff key is malformed"))?,
        )?;
    }
    for (word, index) in &destinations {
        restore_value(
            &mut vm.stack,
            *index,
            StructuralDestinationKey::from_word(*word)
                .map(Value::from_structural_destination)
                .ok_or_else(|| Error::msg("structural destination handoff key is malformed"))?,
        )?;
    }
    let structural = invocation_mut(vm)?;
    for (word, _) in owners {
        if let Some(record) = structural.owners.get_mut(&word) {
            record.taken_from = None;
        }
    }
    for (word, _) in destinations {
        if let Some(record) = structural.destinations.get_mut(&word) {
            record.taken_from = None;
        }
    }
    Ok(())
}
