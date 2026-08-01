fn function_metadata_bytes(chunk: &Chunk) -> Result<(usize, usize)> {
    let mut metadata_bytes = super::entry_capabilities::metadata_bytes(chunk)?;
    let global_prototype_bytes = chunk
        .global_prototypes
        .len()
        .checked_mul(5)
        .ok_or_else(|| Error::msg("bytecode metadata byte size overflow"))?;
    metadata_bytes = checked_add(metadata_bytes, global_prototype_bytes, "metadata byte size")?;
    metadata_bytes = checked_add(
        metadata_bytes,
        chunk
            .main
            .parameter_resources
            .len()
            .saturating_add(chunk.main.parameter_resource_places.len()),
        "metadata byte size",
    )?;
    metadata_bytes = checked_add(
        metadata_bytes,
        chunk
            .main
            .parameter_uniques
            .len()
            .saturating_add(chunk.main.parameter_unique_places.len())
            .saturating_add(chunk.main.parameter_structurals.len())
            .saturating_add(chunk.main.parameter_structural_places.len())
            .saturating_add(chunk.main.parameter_type_variables.len().saturating_mul(3))
            .saturating_add(chunk.main.parameter_copy_kinds.len())
            .saturating_add(usize::from(chunk.main.return_copy_kind.is_some()))
            .saturating_add(chunk.main.parameter_region_products.len().saturating_mul(3))
            .saturating_add(usize::from(chunk.main.return_region_product.is_some()) * 3),
        "metadata byte size",
    )?;
    metadata_bytes = checked_add(
        metadata_bytes,
        witness_metadata_bytes(&chunk.main),
        "metadata byte size",
    )?;
    metadata_bytes = checked_add(metadata_bytes, 6, "metadata byte size")?;
    metadata_bytes = checked_add(
        metadata_bytes,
        failure_metadata_bytes(&chunk.main)?,
        "metadata byte size",
    )?;
    let mut encoded_bytes = chunk.main.code.len();
    for proto in &chunk.protos {
        metadata_bytes = checked_add(metadata_bytes, proto.name.len(), "metadata byte size")?;
        metadata_bytes = checked_add(
            metadata_bytes,
            proto
                .parameter_resources
                .len()
                .saturating_add(proto.parameter_resource_places.len()),
            "metadata byte size",
        )?;
        metadata_bytes = checked_add(
            metadata_bytes,
            proto
                .parameter_uniques
                .len()
                .saturating_add(proto.parameter_unique_places.len())
                .saturating_add(proto.parameter_structurals.len())
                .saturating_add(proto.parameter_structural_places.len())
                .saturating_add(proto.parameter_type_variables.len().saturating_mul(3))
                .saturating_add(proto.parameter_copy_kinds.len())
                .saturating_add(usize::from(proto.return_copy_kind.is_some()))
                .saturating_add(proto.parameter_region_products.len().saturating_mul(3))
                .saturating_add(usize::from(proto.return_region_product.is_some()) * 3),
            "metadata byte size",
        )?;
        metadata_bytes = checked_add(
            metadata_bytes,
            witness_metadata_bytes(proto),
            "metadata byte size",
        )?;
        metadata_bytes = checked_add(metadata_bytes, 6, "metadata byte size")?;
        metadata_bytes = checked_add(
            metadata_bytes,
            failure_metadata_bytes(proto)?,
            "metadata byte size",
        )?;
        encoded_bytes = checked_add(encoded_bytes, proto.code.len(), "encoded byte size")?;
    }
    Ok((metadata_bytes, encoded_bytes))
}

fn witness_metadata_bytes(proto: &FunctionProto) -> usize {
    let requirements = proto
        .memory_witness_parameters
        .iter()
        .map(|item| 3usize.saturating_add(item.operations.len()))
        .fold(0usize, usize::saturating_add);
    let calls = proto
        .call_witnesses
        .iter()
        .map(|item| 8usize.saturating_add(item.bindings.len().saturating_mul(4)))
        .fold(0usize, usize::saturating_add);
    requirements.saturating_add(calls)
}
