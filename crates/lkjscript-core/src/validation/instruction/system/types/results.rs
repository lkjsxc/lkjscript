fn preferred_result_type(chunk: &Chunk, proto: &FunctionProto) -> Option<crate::StructuralTypeId> {
    proto.return_structural.and_then(|representation| {
        chunk
            .structural_representations
            .get_structural(representation)
            .map(|item| item.type_id)
    })
}

fn find_result_type(
    chunk: &Chunk,
    preferred: Option<crate::StructuralTypeId>,
    system_error: bool,
    allow_empty_success: bool,
    matches: impl Fn(&crate::StructuralFieldMetadata) -> bool,
) -> Option<crate::StructuralTypeId> {
    let candidate = |ty: &crate::StructuralTypeMetadata| {
        let crate::StructuralTypeKind::Enum(enum_id) = ty.kind else {
            return false;
        };
        if enum_id.bytes() != crate::RESULT_ID {
            return false;
        }
        let Some(layout) = chunk.structural_layouts.get_structural(ty.layout) else {
            return false;
        };
        let crate::StructuralLayoutKind::Enum { variants, .. } = &layout.kind else {
            return false;
        };
        let fields = variants
            .iter()
            .flat_map(|variant| variant.fields.first())
            .collect::<Vec<_>>();
        let has_system_error = fields.iter().any(|field| {
            let crate::StructuralFieldRoute::Structural(type_id) = field.route else {
                return false;
            };
            chunk
                .structural_types
                .get_structural(type_id)
                .is_some_and(|ty| {
                    matches!(
                        ty.kind,
                        crate::StructuralTypeKind::Enum(enum_id)
                            if enum_id.bytes() == crate::SYSTEM_ERROR_ID
                    )
                })
        });
        let success_matches = fields.into_iter().any(&matches)
            || (allow_empty_success && variants.iter().any(|variant| variant.fields.is_empty()));
        has_system_error == system_error && success_matches
    };
    preferred
        .and_then(|preferred| {
            chunk
                .structural_types
                .get_structural(preferred)
                .filter(|ty| candidate(ty))
                .map(|ty| ty.id)
        })
        .or_else(|| {
            chunk
                .structural_types
                .iter()
                .find(|ty| candidate(ty))
                .map(|ty| ty.id)
        })
}

fn structural_result_representation(
    chunk: &Chunk,
    type_id: Option<crate::StructuralTypeId>,
) -> Option<crate::StructuralRepresentationId> {
    let type_id = type_id?;
    chunk
        .structural_representations
        .iter()
        .find(|item| {
            item.type_id == type_id && item.category == crate::StructuralValueCategory::Owner
        })
        .map(|item| item.id)
}
