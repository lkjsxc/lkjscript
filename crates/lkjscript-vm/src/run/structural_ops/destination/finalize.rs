use lkjscript_core::StructuralStorage;

fn finish<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected = StructuralDestinationId::new(vm.read_u64()?);
    let destination_value = vm.pop()?;
    let (destination, record) = invocation(vm)?.destination(destination_value)?;
    if record.destination != expected {
        return Err(Error::msg(
            "structural destination finish metadata is stale",
        ));
    }
    let metadata = destination_metadata(vm.chunk, expected)?;
    let owner_representation = metadata.owner_representation;
    let owner_type = representation_type(
        vm.chunk,
        owner_representation,
        StructuralValueCategory::Owner,
    )?;
    if owner_type != record.value_type {
        return Err(Error::msg(
            "structural destination finish owner type mismatch",
        ));
    }
    let storage = representation(vm.chunk, owner_representation)?.storage;
    let owner = if storage == StructuralStorage::SealedRegion {
        invocation_mut(vm)?.runtime.finish_destination_sealed(destination)
            .map(|sealed| sealed.owner)
    } else {
        invocation_mut(vm)?.runtime.finish_destination(destination)
    }
    .map_err(map_value_error)?;
    invocation_mut(vm)?.destinations.remove(&destination.get());
    let value = invocation_mut(vm)?.register_owner(owner, owner_representation, owner_type)?;
    vm.push(value);
    Ok(())
}

fn abort<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let expected = StructuralDestinationId::new(vm.read_u64()?);
    let destination_value = vm.pop()?;
    let (destination, record) = invocation(vm)?.destination(destination_value)?;
    if record.destination != expected {
        return Err(Error::msg("structural destination abort metadata is stale"));
    }
    invocation_mut(vm)?
        .runtime
        .abort_destination(destination)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.destinations.remove(&destination.get());
    vm.push(Value::UNIT);
    Ok(())
}

fn charge_construction<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    if vm.logical_aggregate_constructions >= vm.config.max_logical_aggregate_constructions {
        return Err(Error::resource(
            ResourceLimitKind::LogicalAggregateConstructions,
            "logical aggregate construction limit exceeded before structural destination",
        ));
    }
    vm.logical_aggregate_constructions = vm.logical_aggregate_constructions.saturating_add(1);
    Ok(())
}

fn destination_metadata(
    chunk: &ValidatedChunk,
    id: StructuralDestinationId,
) -> Result<&StructuralDestinationMetadata> {
    id.index()
        .and_then(|index| chunk.structural_destinations().get(index))
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("structural destination metadata is stale"))
}
