pub(crate) fn supports_list_element_equality(ty: &SsaType) -> bool {
    match ty {
        SsaType::List(item) => supports_list_element_equality(item),
        other => supports_value_equality(other),
    }
}

pub(crate) fn supports_value_equality(ty: &SsaType) -> bool {
    match ty {
        SsaType::Unit
        | SsaType::Bool
        | SsaType::I64
        | SsaType::F64
        | SsaType::Str
        | SsaType::Path
        | SsaType::Symbol => true,
        SsaType::Enum { arguments, .. } => arguments.iter().all(supports_value_equality),
        _ => false,
    }
}

pub(crate) fn signature_contains_ownership(program: &Program, signature: &Signature) -> bool {
    signature
        .parameters
        .iter()
        .any(|ty| contains_ownership_type(program, ty))
        || contains_ownership_type(program, &signature.result)
}

pub(crate) fn contains_ownership_type(program: &Program, ty: &SsaType) -> bool {
    if program.memory.is_owned(ty) {
        return true;
    }
    match ty {
        SsaType::Bytes
        | SsaType::ByteVector
        | SsaType::ByteSlice
        | SsaType::ByteSliceMut
        | SsaType::StructuralDestination(_) => true,
        SsaType::List(inner) => contains_ownership_type(program, inner),
        SsaType::Enum { arguments, .. } => arguments
            .iter()
            .any(|argument| contains_ownership_type(program, argument)),
        SsaType::Function(signature) => signature_contains_ownership(program, signature),
        _ => false,
    }
}

pub(crate) fn is_byte_vector(ty: &SsaType) -> bool {
    matches!(ty, SsaType::ByteVector)
}

pub(crate) fn is_owned_value(program: &Program, ty: &SsaType) -> bool {
    is_byte_vector(ty)
        || matches!(
            ty,
            SsaType::Bytes | SsaType::Resource(_) | SsaType::StructuralDestination(_)
        )
        || program.memory.is_owned(ty)
}

pub(crate) fn expected_drop_glue(
    program: &Program,
    ty: &SsaType,
) -> Option<crate::DropGlueIdentity> {
    let builtin = match ty {
        SsaType::Bytes => Some(crate::DropGlueIdentity::Bytes),
        SsaType::ByteVector => Some(crate::DropGlueIdentity::ByteVector),
        SsaType::Resource(kind) => Some(crate::DropGlueIdentity::Resource(*kind)),
        SsaType::StructuralDestination(type_id) => {
            let item = program.memory.types.get(type_id.index()?)?;
            Some(crate::DropGlueIdentity::Structural(
                crate::StructuralDropGlueIdentity::Destination {
                    type_id: *type_id,
                    layout: item.layout,
                },
            ))
        }
        _ => None,
    };
    builtin.or_else(|| structural_drop_glue(program, ty))
}

pub(crate) fn is_affine(program: &Program, ty: &SsaType) -> bool {
    is_owned_value(program, ty) || matches!(ty, SsaType::ByteSliceMut)
}

fn structural_drop_glue(program: &Program, ty: &SsaType) -> Option<crate::DropGlueIdentity> {
    let item = program.memory.type_for(ty)?;
    if item.mode == crate::StructuralTypeMode::Copy {
        return None;
    }
    let layout = program.memory.layouts.get(item.layout.index()?)?;
    let identity = match &layout.kind {
        crate::StructuralLayoutKind::String => crate::StructuralDropGlueIdentity::String {
            type_id: item.id,
            layout: item.layout,
        },
        crate::StructuralLayoutKind::Path => crate::StructuralDropGlueIdentity::Path {
            type_id: item.id,
            layout: item.layout,
        },
        crate::StructuralLayoutKind::Product { product, .. } => {
            crate::StructuralDropGlueIdentity::Product {
                type_id: item.id,
                product: *product,
                layout: item.layout,
            }
        }
        crate::StructuralLayoutKind::Enum {
            enum_id,
            runtime_layout,
            ..
        } => crate::StructuralDropGlueIdentity::Enum {
            type_id: item.id,
            enum_id: *enum_id,
            layout: item.layout,
            runtime_layout: *runtime_layout,
        },
    };
    Some(crate::DropGlueIdentity::Structural(identity))
}
