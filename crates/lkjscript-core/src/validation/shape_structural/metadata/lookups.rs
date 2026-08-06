fn lookup_representation(
    chunk: &Chunk,
    id: crate::StructuralRepresentationId,
) -> Result<&StructuralRepresentationMetadata> {
    chunk
        .structural_representations
        .get(id.index())
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("bytecode structural representation reference is stale"))
}

fn lookup_destination(
    chunk: &Chunk,
    id: crate::StructuralDestinationId,
) -> Result<&StructuralDestinationMetadata> {
    id.index()
        .and_then(|index| chunk.structural_destinations.get(index))
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("bytecode structural destination reference is stale"))
}

fn layout_fields(
    chunk: &Chunk,
    id: crate::StructuralLayoutId,
    active_variant: Option<crate::VariantId>,
) -> Result<Vec<StructuralFieldMetadata>> {
    let layout = chunk
        .structural_layouts
        .get(id.index())
        .filter(|item| item.id == id)
        .ok_or_else(|| Error::msg("bytecode structural layout reference is stale"))?;
    match &layout.kind {
        StructuralLayoutKind::String | StructuralLayoutKind::Path => {
            if active_variant.is_some() {
                return Err(Error::msg(
                    "bytecode structural leaf destination has an active variant",
                ));
            }
            Ok(Vec::new())
        }
        StructuralLayoutKind::Product { fields, .. } => {
            if active_variant.is_some() {
                return Err(Error::msg(
                    "bytecode structural product destination has an active variant",
                ));
            }
            Ok(fields.clone())
        }
        StructuralLayoutKind::Enum { variants, .. } => {
            let active = active_variant.ok_or_else(|| {
                Error::msg("bytecode structural enum destination has no active variant")
            })?;
            variants
                .iter()
                .find(|variant| variant.variant == active)
                .map(|variant| variant.fields.clone())
                .ok_or_else(|| Error::msg("bytecode structural enum active variant is missing"))
        }
    }
}

fn add(left: usize, right: usize, category: &str) -> Result<usize> {
    left.checked_add(right)
        .ok_or_else(|| Error::host(format!("bytecode {category} overflow")))
}
