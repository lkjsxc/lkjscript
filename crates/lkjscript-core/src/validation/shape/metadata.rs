fn function_metadata_bytes(chunk: &Chunk) -> Result<(usize, usize)> {
    let mut metadata_bytes = super::entry_capabilities::metadata_bytes(chunk)?;
    let global_prototype_bytes = chunk
        .global_prototypes
        .len()
        .checked_mul(9)
        .ok_or_else(|| Error::host("bytecode metadata byte size overflow"))?;
    metadata_bytes = checked_add(metadata_bytes, global_prototype_bytes, "metadata byte size")?;
    metadata_bytes = checked_add(
        metadata_bytes,
        prototype_metadata_bytes(&chunk.main)?,
        "metadata byte size",
    )?;
    let mut encoded_bytes = chunk.main.code.len();
    for proto in &chunk.protos {
        metadata_bytes = checked_add(metadata_bytes, proto.name.len(), "metadata byte size")?;
        metadata_bytes = checked_add(
            metadata_bytes,
            prototype_metadata_bytes(proto)?,
            "metadata byte size",
        )?;
        encoded_bytes = checked_add(encoded_bytes, proto.code.len(), "encoded byte size")?;
    }
    Ok((metadata_bytes, encoded_bytes))
}

fn prototype_metadata_bytes(proto: &FunctionProto) -> Result<usize> {
    let mut bytes = 0;
    for length in [
        proto.parameter_resources.len(),
        proto.parameter_resource_places.len(),
        proto.parameter_uniques.len(),
        proto.parameter_unique_places.len(),
        proto.parameter_structurals.len(),
        proto.parameter_structural_places.len(),
        proto.parameter_copy_kinds.len(),
    ] {
        bytes = checked_add(bytes, length, "metadata byte size")?;
    }
    let type_variable_bytes = proto
        .parameter_type_variables
        .len()
        .checked_mul(3)
        .ok_or_else(|| Error::host("bytecode metadata byte size overflow"))?;
    bytes = checked_add(bytes, type_variable_bytes, "metadata byte size")?;
    let region_product_bytes = proto
        .parameter_region_products
        .len()
        .checked_mul(3)
        .ok_or_else(|| Error::host("bytecode metadata byte size overflow"))?;
    bytes = checked_add(bytes, region_product_bytes, "metadata byte size")?;
    bytes = checked_add(
        bytes,
        usize::from(proto.return_copy_kind.is_some()),
        "metadata byte size",
    )?;
    bytes = checked_add(
        bytes,
        usize::from(proto.return_region_product.is_some())
            .checked_mul(3)
            .ok_or_else(|| Error::host("bytecode metadata byte size overflow"))?,
        "metadata byte size",
    )?;
    bytes = checked_add(bytes, witness_metadata_bytes(proto)?, "metadata byte size")?;
    bytes = checked_add(bytes, 6, "metadata byte size")?;
    checked_add(
        bytes,
        failure_metadata_bytes(proto)?,
        "metadata byte size",
    )
}

fn witness_metadata_bytes(proto: &FunctionProto) -> Result<usize> {
    let mut bytes = 0;
    for requirement in &proto.memory_witness_parameters {
        bytes = checked_add(bytes, 3, "memory witness metadata byte size")?;
        bytes = checked_add(
            bytes,
            requirement.operations.len(),
            "memory witness metadata byte size",
        )?;
    }
    for call in &proto.call_witnesses {
        bytes = checked_add(bytes, 16, "memory witness metadata byte size")?;
        let binding_bytes = call
            .bindings
            .len()
            .checked_mul(4)
            .ok_or_else(|| Error::host("bytecode memory witness metadata byte size overflow"))?;
        bytes = checked_add(bytes, binding_bytes, "memory witness metadata byte size")?;
    }
    Ok(bytes)
}
