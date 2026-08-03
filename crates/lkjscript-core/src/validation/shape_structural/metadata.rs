fn validate_representation(
    chunk: &Chunk,
    representation: &StructuralRepresentationMetadata,
) -> Result<()> {
    let ty = chunk
        .structural_types
        .get(representation.type_id.index())
        .filter(|ty| ty.id == representation.type_id)
        .ok_or_else(|| Error::msg("bytecode structural representation has a missing type"))?;
    if ty.layout != representation.layout || ty.witness != representation.witness {
        return Err(Error::msg(
            "bytecode structural representation has stale type/witness metadata",
        ));
    }
    if let Some(witness) = chunk.memory_witnesses.iter()
        .find(|item| item.id == representation.witness)
    {
        if witness.group != representation.witness_group
            || witness.ordinal != representation.witness_member
        {
            return Err(Error::msg(
                "bytecode structural representation group/member is stale",
            ));
        }
    } else if !chunk.memory_witnesses.is_empty() {
        return Err(Error::msg(
            "bytecode structural representation witness is missing",
        ));
    }
    if chunk.structural_representations.iter().any(|item| {
        item.id != representation.id
            && item.type_id == representation.type_id
            && item.witness == representation.witness
            && item.witness_group == representation.witness_group
            && item.witness_member == representation.witness_member
            && item.layout == representation.layout
            && item.category == representation.category
            && item.storage == representation.storage
            && item.route == representation.route
    }) {
        return Err(Error::msg("bytecode structural representation tuple is duplicated"));
    }
    let valid_storage = matches!(
        (representation.category, representation.storage),
        (StructuralValueCategory::Owner,
            crate::StructuralStorage::Static
                | crate::StructuralStorage::UniqueStructural
                | crate::StructuralStorage::OrdinaryRegion
                | crate::StructuralStorage::SealedRegion
                | crate::StructuralStorage::ExternalResource)
        | (StructuralValueCategory::View,
            crate::StructuralStorage::Stack | crate::StructuralStorage::BorrowedView)
        | (StructuralValueCategory::Destination,
            crate::StructuralStorage::CallerDestination
                | crate::StructuralStorage::UniqueStructural
                | crate::StructuralStorage::OrdinaryRegion
                | crate::StructuralStorage::SealedRegion)
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
        || representation.witness != owner.witness
        || representation.witness_group != owner.witness_group
        || representation.witness_member != owner.witness_member
        || representation.layout != owner.layout
        || representation.storage != owner.storage
        || representation.route != owner.route
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

include!("metadata/lookups.rs");
