fn install_structural_destinations(chunk: &mut Chunk) -> Result<()> {
    let representations: Vec<_> = chunk
        .structural_representations
        .iter()
        .filter(|item| item.category == BytecodeStructuralValueCategory::Destination)
        .copied()
        .collect();
    for representation in representations {
        let owner_representation = chunk
            .structural_representations
            .iter()
            .find(|item| {
                item.type_id == representation.type_id
                    && item.witness == representation.witness
                    && item.witness_group == representation.witness_group
                    && item.witness_member == representation.witness_member
                    && item.layout == representation.layout
                    && item.category == BytecodeStructuralValueCategory::Owner
                    && item.storage == representation.storage
                    && item.route == representation.route
            })
            .map(|item| item.id)
            .ok_or_else(|| Error::msg("structural destination has no owner representation"))?;
        let layout = chunk
            .structural_layouts
            .get(representation.layout.index())
            .ok_or_else(|| Error::msg("structural destination layout is missing"))?;
        let candidates: Vec<(Option<BytecodeVariantId>, Vec<StructuralFieldMetadata>)> =
            match &layout.kind {
                BytecodeStructuralLayoutKind::String | BytecodeStructuralLayoutKind::Path => {
                    vec![(None, Vec::new())]
                }
                BytecodeStructuralLayoutKind::Product { fields, .. } => {
                    vec![(None, fields.clone())]
                }
                BytecodeStructuralLayoutKind::Enum { variants, .. } => variants
                    .iter()
                    .map(|variant| (Some(variant.variant), variant.fields.clone()))
                    .collect(),
            };
        for (active_variant, fields) in candidates {
            let raw = u64::try_from(chunk.structural_destinations.len())
                .map_err(|_| Error::msg("bytecode structural destination index exceeds u64"))?;
            chunk
                .structural_destinations
                .push(StructuralDestinationMetadata {
                    id: StructuralDestinationId::new(raw),
                    representation: representation.id,
                    owner_representation,
                    active_variant,
                    fields,
                });
        }
    }
    Ok(())
}
