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
            .saturating_add(chunk.main.parameter_structural_places.len()),
        "metadata byte size",
    )?;
    metadata_bytes = checked_add(metadata_bytes, 3, "metadata byte size")?;
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
                .saturating_add(proto.parameter_structural_places.len()),
            "metadata byte size",
        )?;
        metadata_bytes = checked_add(metadata_bytes, 3, "metadata byte size")?;
        metadata_bytes = checked_add(
            metadata_bytes,
            failure_metadata_bytes(proto)?,
            "metadata byte size",
        )?;
        encoded_bytes = checked_add(encoded_bytes, proto.code.len(), "encoded byte size")?;
    }
    Ok((metadata_bytes, encoded_bytes))
}
