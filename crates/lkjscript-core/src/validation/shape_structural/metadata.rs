fn validate_representation(
    chunk: &Chunk,
    representation: &StructuralRepresentationMetadata,
) -> Result<()> {
    let ty = chunk
        .structural_types
        .get(representation.type_id.index())
        .filter(|ty| ty.id == representation.type_id)
        .ok_or_else(|| Error::msg("bytecode structural representation has a missing type"))?;
    if ty.layout != representation.layout {
        return Err(Error::msg(
            "bytecode structural representation has a stale layout",
        ));
    }
    let valid_storage = matches!(
        (representation.category, representation.storage),
        (
            StructuralValueCategory::Owner,
            crate::StructuralStorage::Static | crate::StructuralStorage::Unique
        ) | (
            StructuralValueCategory::View,
            crate::StructuralStorage::Stack
        ) | (
            StructuralValueCategory::Destination,
            crate::StructuralStorage::CallerDestination
        )
    );
    if !valid_storage {
        return Err(Error::msg(
            "bytecode structural representation has an invalid category/storage pair",
        ));
    }
    Ok(())
}

fn validate_destination(chunk: &Chunk, destination: &StructuralDestinationMetadata) -> Result<()> {
    let representation = lookup_representation(chunk, destination.representation)?;
    let owner = lookup_representation(chunk, destination.owner_representation)?;
    if representation.category != StructuralValueCategory::Destination
        || owner.category != StructuralValueCategory::Owner
        || representation.type_id != owner.type_id
        || representation.layout != owner.layout
    {
        return Err(Error::msg(
            "bytecode structural destination owner metadata is inconsistent",
        ));
    }
    let fields = layout_fields(chunk, representation.layout, destination.active_variant)?;
    if fields != destination.fields {
        return Err(Error::msg(
            "bytecode structural destination field metadata is stale",
        ));
    }
    for field in &destination.fields {
        validate_field(chunk, field)?;
    }
    Ok(())
}

fn validate_field(chunk: &Chunk, field: &StructuralFieldMetadata) -> Result<()> {
    match field.route {
        StructuralFieldRoute::Structural(id) => {
            let ty = chunk
                .structural_types
                .get(id.index())
                .filter(|ty| ty.id == id)
                .ok_or_else(|| {
                    Error::msg(
                        "bytecode structural field references a missing structural type",
                    )
                })?;
            if field.identity != ty.identity || field.runtime_type != Some(ty.runtime_type) {
                return Err(Error::msg(
                    "bytecode structural field has stale exact type metadata",
                ));
            }
        }
        StructuralFieldRoute::Copy => {
            if !matches!(
                field.runtime_type.map(|ty| ty.kind),
                Some(
                    crate::StructuralKind::Unit
                        | crate::StructuralKind::Bool
                        | crate::StructuralKind::I64
                        | crate::StructuralKind::F64
                        | crate::StructuralKind::Static
                )
            ) {
                return Err(Error::msg(
                    "bytecode structural copy field lacks exact runtime type metadata",
                ));
            }
        }
        StructuralFieldRoute::Unique => {
            if !matches!(
                field.runtime_type.map(|ty| ty.kind),
                Some(crate::StructuralKind::Bytes | crate::StructuralKind::ByteVector)
            ) {
                return Err(Error::msg(
                    "bytecode structural unique field lacks exact runtime type metadata",
                ));
            }
        }
        StructuralFieldRoute::Resource => {
            if field.runtime_type.is_some() {
                return Err(Error::msg(
                    "bytecode structural resource field carries value-runtime metadata",
                ));
            }
        }
        StructuralFieldRoute::LegacyHeap => {
            return Err(Error::msg(
                "bytecode structural metadata mixes a legacy heap route",
            ));
        }
    }
    Ok(())
}

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
    chunk
        .structural_destinations
        .get(id.index())
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
        .ok_or_else(|| Error::msg(format!("bytecode {category} overflow")))
}
