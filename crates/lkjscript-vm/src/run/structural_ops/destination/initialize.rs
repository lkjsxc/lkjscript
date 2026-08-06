fn initialize_structural_owner<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    destination: StructuralDestinationKey,
    field: usize,
    source: Value,
    metadata: StructuralFieldMetadata,
) -> Result<()> {
    let expected = metadata
        .runtime_type
        .ok_or_else(|| Error::msg("structural field lacks exact runtime type"))?;
    let (owner, record) = invocation(vm)?.owner(source)?;
    if record.value_type != expected {
        return Err(Error::msg(
            "structural destination owner field type mismatch",
        ));
    }
    invocation_mut(vm)?
        .runtime
        .initialize_value(destination, field, source)
        .map_err(map_value_error)?;
    invocation_mut(vm)?.owners.remove(&owner.get());
    Ok(())
}

fn initialize_unique_owner<J: RuntimeTier>(
    vm: &mut Vm<'_, J>,
    destination: StructuralDestinationKey,
    field: usize,
    source: Value,
    metadata: StructuralFieldMetadata,
) -> Result<()> {
    let expected = metadata
        .runtime_type
        .ok_or_else(|| Error::msg("unique structural field lacks exact runtime type"))?;
    let (bytes, dynamic) = if let Some(index) = source.as_static_bytes() {
        if expected.kind != StructuralKind::Bytes {
            return Err(Error::msg("static bytes field has the wrong runtime kind"));
        }
        let bytes = vm
            .chunk
            .constant(index)
            .and_then(|constant| match constant {
                lkjscript_core::Constant::StaticBytes(bytes) => Some(bytes.to_vec()),
                _ => None,
            })
            .ok_or_else(|| Error::msg("static bytes field constant is stale"))?;
        (bytes, false)
    } else {
        vm.unique.ensure_any_unloaned(source)?;
        let bytes = vm.unique.copy_owner_bytes(source)?;
        let kind_matches = (source.as_bytes_key().is_some()
            && expected.kind == StructuralKind::Bytes)
            || (source.as_byte_vector_key().is_some()
                && expected.kind == StructuralKind::ByteVector);
        if !kind_matches {
            return Err(Error::msg(
                "unique structural field has the wrong runtime kind",
            ));
        }
        (bytes, true)
    };
    let payload = match expected.kind {
        StructuralKind::Bytes => SemanticPayload::Bytes(bytes),
        StructuralKind::ByteVector => SemanticPayload::ByteVector(bytes),
        _ => {
            return Err(Error::msg(
                "unsupported unique structural field runtime kind",
            ))
        }
    };
    invocation_mut(vm)?
        .runtime
        .initialize_node(destination, field, SemanticValue::new(expected, payload))
        .map_err(|failure| map_value_error(failure.error))?;
    if dynamic {
        vm.unique.drop_owner(source)?;
    }
    Ok(())
}
