pub(in crate::run) fn call_return_type_variable_representation<J: RuntimeTier>(
    vm: &Vm<'_, J>,
    proto: &lkjscript_core::FunctionProto,
    arguments: &[Value],
) -> Result<Option<StructuralRepresentationId>> {
    let Some(return_variable) = proto.return_type_variable else {
        return Ok(None);
    };
    let mut result = None;
    for (index, variable) in proto.parameter_type_variables.iter().enumerate() {
        if *variable != Some(return_variable) {
            continue;
        }
        let Some(value) = arguments.get(index).copied() else {
            return Err(Error::msg("type-variable call argument is missing"));
        };
        if value.as_structural_root().is_none() {
            continue;
        }
        let (_, owner) = invocation(vm)?.owner(value)?;
        if result.is_some_and(|existing| existing != owner.representation) {
            return Err(Error::msg("type-variable structural arguments disagree at runtime"));
        }
        result = Some(owner.representation);
    }
    if let Some(expected) = result {
        for (index, variable) in proto.parameter_type_variables.iter().enumerate() {
            if *variable != Some(return_variable) {
                continue;
            }
            let value = arguments
                .get(index)
                .copied()
                .ok_or_else(|| Error::msg("type-variable call argument is missing"))?;
            let (_, owner) = invocation(vm)?.owner(value)?;
            if owner.representation != expected {
                return Err(Error::msg("type-variable call arguments disagree at runtime"));
            }
        }
    }
    Ok(result)
}

pub(in crate::run) fn initialize_call_places(
    chunk: &ValidatedChunk,
    structural: Option<&StructuralInvocation>,
    proto: &lkjscript_core::FunctionProto,
    arguments: &[Value],
    places: &mut [unique::RuntimePlace],
) -> Result<()> {
    for (index, representation) in proto.parameter_structurals.iter().copied().enumerate() {
        let Some(representation) = representation else {
            continue;
        };
        let Some(place) = proto
            .parameter_structural_places
            .get(index)
            .copied()
            .flatten()
        else {
            continue;
        };
        let value = arguments
            .get(index)
            .copied()
            .ok_or_else(|| Error::msg("structural call argument is missing"))?;
        let structural =
            structural.ok_or_else(|| Error::msg("structural call lacks invocation registry"))?;
        let (key, record) = structural.owner(value)?;
        let expected = representation_type(chunk, representation, StructuralValueCategory::Owner)?;
        if record.value_type != expected
            || !same_representation_type(chunk, record.representation, representation)?
        {
            return Err(Error::msg(
                "structural call argument representation mismatch",
            ));
        }
        let target = places
            .get_mut(usize::from(place))
            .ok_or_else(|| Error::msg("structural call parameter place is out of range"))?;
        if !matches!(target, unique::RuntimePlace::Inactive) {
            return Err(Error::msg("duplicate call parameter owner place"));
        }
        *target = unique::RuntimePlace::Active {
            owner: Some(key.get()),
            transferred: None,
        };
    }
    Ok(())
}

pub(in crate::run) fn commit_call_arguments<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    arguments: &[Value],
    proto: &lkjscript_core::FunctionProto,
) -> Result<()> {
    for (index, (value, representation)) in arguments
        .iter()
        .copied()
        .zip(&proto.parameter_structurals)
        .enumerate()
    {
        if representation.is_some()
            && proto
                .parameter_structural_places
                .get(index)
                .is_some_and(Option::is_some)
        {
            commit_handoff(vm, value)?;
        }
    }
    Ok(())
}

fn cleanup_copy_roots<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    returned: Value,
    entry: bool,
) -> Result<()> {
    if entry {
        let returned = returned.as_structural_root().map(StructuralValueKey::get);
        let mut roots: Vec<_> = invocation(vm)?
            .owners
            .keys()
            .copied()
            .filter(|word| Some(*word) != returned)
            .collect();
        roots.sort_unstable();
        for word in roots {
            let key = StructuralValueKey::from_word(word)
                .ok_or_else(|| Error::msg("registered structural owner key is malformed"))?;
            adapter::drop_registered_owner(vm, Value::from_structural_root(key))?;
        }
        return Ok(());
    }
    let frame = vm
        .frames
        .last()
        .ok_or_else(|| Error::msg("copy-root cleanup requires an active frame"))?;
    let caller = (!entry).then(|| vm.stack[..frame.stack_base].to_vec());
    let start = if entry { 0 } else { frame.locals_base };
    let mut roots = Vec::new();
    for value in vm.stack[start..].iter().copied() {
        if value == returned
            || caller.as_ref().is_some_and(|caller| caller.contains(&value))
            || roots.contains(&value)
        {
            continue;
        }
        let Some(key) = value.as_structural_root() else {
            continue;
        };
        let Some(record) = invocation(vm)?.owners.get(&key.get()) else {
            continue;
        };
        let copy = vm
            .chunk
            .structural_representations()
            .get(record.representation.index())
            .and_then(|representation| {
                vm.chunk
                    .structural_types()
                    .get(representation.type_id.index())
            })
            .is_some_and(|ty| {
                matches!(
                    ty.mode,
                    lkjscript_core::StructuralTypeMode::Copy
                        | lkjscript_core::StructuralTypeMode::Immutable
                )
            });
        if copy {
            roots.push(value);
        }
    }
    for root in roots {
        adapter::drop_registered_owner(vm, root)?;
    }
    Ok(())
}

fn restore_value(stack: &mut [Value], index: usize, value: Value) -> Result<()> {
    let target = stack
        .get_mut(index)
        .ok_or_else(|| Error::msg("structural handoff lost its source local"))?;
    if !target.is_invalid() && *target != value {
        return Err(Error::msg(
            "structural handoff source local was reused before restoration",
        ));
    }
    *target = value;
    Ok(())
}
