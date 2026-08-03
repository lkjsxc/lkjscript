use super::*;

pub(super) fn exact_layout_identity(
    function: FunctionId,
    layouts: &LayoutInterner,
    ty: &SsaType,
) -> Result<LayoutIdentity, LoweringError> {
    layouts.identity(ty).ok_or_else(|| {
        LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} has no supported structural layout identity"),
        )
    })
}

pub(in crate::lower) fn require_resource_island_type(
    function: FunctionId,
    ty: &SsaType,
) -> Result<(), LoweringError> {
    if matches!(
        ty,
        SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Capability(_)
            | SsaType::Resource(_)
    ) {
        Ok(())
    } else {
        Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} is reachable from a collector-free resource group"),
        ))
    }
}

pub(in crate::lower) fn require_structural_island_type(
    function: FunctionId,
    ty: &SsaType,
    layouts: &LayoutInterner,
) -> Result<(), LoweringError> {
    if matches!(
        ty,
        SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::TypeParameter(_)
            | SsaType::StructuralDestination(_)
    ) || layouts.structural().selected(ty)
        || matches!(ty, SsaType::List(element) if structural_list_element(element, layouts))
    {
        Ok(())
    } else {
        Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} is unsupported without structural metadata in a structural group"),
        ))
    }
}

fn structural_list_element(ty: &SsaType, layouts: &LayoutInterner) -> bool {
    matches!(
        ty,
        SsaType::Unit | SsaType::Bool | SsaType::I64 | SsaType::F64
    ) || layouts.structural().selected(ty)
        || matches!(ty, SsaType::List(element) if structural_list_element(element, layouts))
}

pub(in crate::lower) fn require_unique_island_type(
    function: FunctionId,
    ty: &SsaType,
) -> Result<(), LoweringError> {
    if matches!(
        ty,
        SsaType::Unit
            | SsaType::Bool
            | SsaType::I64
            | SsaType::F64
            | SsaType::Bytes
            | SsaType::ByteVector
            | SsaType::ByteSlice
            | SsaType::ByteSliceMut
    ) {
        Ok(())
    } else {
        Err(LoweringError::new(
            LoweringFailureCode::UnsupportedType,
            Some(function),
            format!("type {ty:?} is reachable from a collector-free unique group"),
        ))
    }
}
