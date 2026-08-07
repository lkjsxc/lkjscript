pub(in crate::run) fn teardown(vm: &mut Vm<'_>) -> Result<()> {
    cleanup_list_owners(vm)?;
    cleanup_host_owners(vm)?;
    let Some(structural) = vm.structural.as_ref() else {
        return Ok(());
    };
    if structural.is_empty() && structural.runtime.verify_empty().is_ok() {
        return Ok(());
    }
    let retained = structural.runtime.verify_empty().err();
    cleanup_all(vm)?;
    invocation(vm)?
        .runtime
        .verify_empty()
        .map_err(map_value_error)?;
    Err(Error::msg(format!(
        "structural invocation retained live state before emergency cleanup: {}",
        retained.map_or_else(|| "registry mismatch".to_owned(), |error| error.to_string())
    )))
}

fn cleanup_list_owners(vm: &mut Vm<'_>) -> Result<()> {
    let mut owners: Vec<_> = invocation(vm)?
        .list_owners
        .iter()
        .map(|(word, record)| (*word, record.value_type()))
        .collect();
    owners.sort_unstable_by_key(|(word, _)| *word);
    for (word, value_type) in owners {
        let key = StructuralValueKey::from_word(word)
            .ok_or_else(|| Error::msg("segmented-list owner key is malformed"))?;
        invocation_mut(vm)?
            .runtime
            .dispose_owner(key, value_type)
            .map(|_| ())
            .map_err(map_value_error)?;
        invocation_mut(vm)?.list_owners.remove(&word);
    }
    Ok(())
}

fn cleanup_host_owners(vm: &mut Vm<'_>) -> Result<()> {
    let owners: Vec<_> = invocation(vm)?
        .host_owners
        .iter()
        .map(|(word, value_type)| (*word, *value_type))
        .collect();
    for (word, value_type) in owners {
        let key = StructuralValueKey::from_word(word)
            .ok_or_else(|| Error::msg("host structural owner key is malformed"))?;
        invocation_mut(vm)?
            .runtime
            .dispose_owner(key, value_type)
            .map(|_| ())
            .map_err(map_value_error)?;
        invocation_mut(vm)?.host_owners.remove(&word);
    }
    Ok(())
}

pub(in crate::run) fn prepare_exit(vm: &mut Vm<'_>) -> Result<()> {
    cleanup_all(vm)?;
    cleanup_host_owners(vm)?;
    invocation(vm)?
        .runtime
        .verify_empty()
        .map_err(map_value_error)
}

fn cleanup_all(vm: &mut Vm<'_>) -> Result<()> {
    adapter::cleanup_all_adapters(vm)?;
    let mut view_words: Vec<_> = invocation(vm)?.views.keys().copied().collect();
    view_words.sort_unstable();
    for word in view_words {
        let key = StructuralViewKey::from_word(word)
            .ok_or_else(|| Error::msg("structural cleanup view key is malformed"))?;
        invocation_mut(vm)?
            .runtime
            .end_view(key)
            .map_err(map_value_error)?;
        invocation_mut(vm)?.views.remove(&word);
    }
    let mut destination_words: Vec<_> = invocation(vm)?.destinations.keys().copied().collect();
    destination_words.sort_unstable();
    for word in destination_words {
        let key = StructuralDestinationKey::from_word(word)
            .ok_or_else(|| Error::msg("structural cleanup destination key is malformed"))?;
        invocation_mut(vm)?
            .runtime
            .abort_destination(key)
            .map_err(map_value_error)?;
        invocation_mut(vm)?.destinations.remove(&word);
    }
    cleanup_list_owners(vm)?;
    let mut owners: Vec<_> = invocation(vm)?
        .owners
        .iter()
        .map(|(word, record)| (*word, record.value_type))
        .collect();
    owners.sort_unstable_by_key(|(word, _)| *word);
    for (word, value_type) in owners {
        let key = StructuralValueKey::from_word(word)
            .ok_or_else(|| Error::msg("structural cleanup owner key is malformed"))?;
        invocation_mut(vm)?
            .runtime
            .dispose_owner(key, value_type)
            .map(|_| ())
            .map_err(map_value_error)?;
        invocation_mut(vm)?.owners.remove(&word);
    }
    Ok(())
}
