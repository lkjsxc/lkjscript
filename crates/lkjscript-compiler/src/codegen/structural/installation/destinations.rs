fn install_structural_destinations(chunk: &mut Chunk) -> Result<()> {
    let mut owners = HashMap::new();
    for item in &chunk.structural_representations {
        if item.category == BytecodeStructuralValueCategory::Owner {
            let key = (
                item.type_id,
                item.witness,
                item.witness_group,
                item.witness_member,
                item.layout,
                item.storage,
                item.route,
            );
            if owners.insert(key, item.id).is_some() {
                return Err(Error::msg("structural owner representation is ambiguous"));
            }
        }
    }
    let representations: Vec<_> = chunk
        .structural_representations
        .iter()
        .filter(|item| item.category == BytecodeStructuralValueCategory::Destination)
        .copied()
        .collect();
    for representation in representations {
        let owner_representation = owners
            .get(&(
                representation.type_id,
                representation.witness,
                representation.witness_group,
                representation.witness_member,
                representation.layout,
                representation.storage,
                representation.route,
            ))
            .copied()
            .ok_or_else(|| Error::msg("structural destination has no owner representation"))?;
        let layout = chunk
            .structural_layouts
            .get_structural(representation.layout)
            .ok_or_else(|| Error::msg("structural destination layout is missing"))?;
        let mut candidates: Vec<(Option<BytecodeVariantId>, Vec<StructuralFieldMetadata>)> =
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
        candidates.sort_by_key(|(active_variant, _)| *active_variant);
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
