fn validate_layouts_and_types(chunk: &Chunk) -> Result<usize> {
    let mut bytes = usize::from(chunk.memory_plan.is_some()) * 32;
    for (index, layout) in chunk.structural_layouts.iter().enumerate() {
        if layout.id.index() != index {
            return Err(Error::msg("bytecode structural LayoutIds are not dense"));
        }
        bytes = add(bytes, 35, "structural metadata byte size")?;
        let fields = match &layout.kind {
            StructuralLayoutKind::String | StructuralLayoutKind::Path => Vec::new(),
            StructuralLayoutKind::Product { fields, .. } => fields.iter().collect(),
            StructuralLayoutKind::Enum { variants, .. } => {
                let mut tags = std::collections::HashSet::with_capacity(variants.len());
                let mut ids = std::collections::HashSet::with_capacity(variants.len());
                let mut fields = Vec::new();
                for variant in variants {
                    if !tags.insert(variant.physical_tag) || !ids.insert(variant.variant) {
                        return Err(Error::msg(
                            "bytecode structural enum variants duplicate identity or physical tag",
                        ));
                    }
                    fields.extend(&variant.fields);
                    bytes = add(bytes, 36, "structural metadata byte size")?;
                }
                fields
            }
        };
        if fields.len() > MAX_STRUCTURAL_LAYOUT_FIELDS {
            return Err(Error::msg(
                "bytecode structural layout field count exceeds the exact bound",
            ));
        }
        for field in fields {
            validate_field(chunk, field)?;
            bytes = add(bytes, 59, "structural metadata byte size")?;
        }
    }
    for (index, ty) in chunk.structural_types.iter().enumerate() {
        if ty.id.index() != index {
            return Err(Error::msg("bytecode structural TypeIds are not dense"));
        }
        let layout = chunk
            .structural_layouts
            .get(ty.layout.index())
            .filter(|layout| layout.id == ty.layout)
            .ok_or_else(|| Error::msg("bytecode structural type references a missing layout"))?;
        let matches = matches!(
            (&ty.kind, &layout.kind),
            (
                crate::StructuralTypeKind::String,
                StructuralLayoutKind::String
            ) | (crate::StructuralTypeKind::Path, StructuralLayoutKind::Path)
        ) || matches!(
            (&ty.kind, &layout.kind),
            (
                crate::StructuralTypeKind::Product(left),
                StructuralLayoutKind::Product { product: right, .. }
            ) if left == right
        ) || matches!(
            (&ty.kind, &layout.kind),
            (
                crate::StructuralTypeKind::Enum(left),
                StructuralLayoutKind::Enum { enum_id: right, .. }
            ) if left == right
        );
        if !matches {
            return Err(Error::msg(
                "bytecode structural type and layout identities disagree",
            ));
        }
        let runtime_kind_matches = matches!(
            (ty.kind, ty.runtime_type.kind),
            (crate::StructuralTypeKind::String, crate::StructuralKind::String)
                | (crate::StructuralTypeKind::Path, crate::StructuralKind::Path)
                | (
                    crate::StructuralTypeKind::Product(_),
                    crate::StructuralKind::Product
                )
                | (crate::StructuralTypeKind::Enum(_), crate::StructuralKind::Enum)
        );
        if !runtime_kind_matches {
            return Err(Error::msg(
                "bytecode structural type has invalid value-runtime metadata",
            ));
        }
        bytes = add(bytes, 93, "structural metadata byte size")?;
    }
    Ok(bytes)
}
