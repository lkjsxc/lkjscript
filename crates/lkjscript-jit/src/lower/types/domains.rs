use super::*;

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
            | SsaType::StructuralDestination(_)
    ) || layouts.structural().selected(ty)
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
