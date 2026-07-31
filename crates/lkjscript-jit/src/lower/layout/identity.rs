use super::*;

pub(in crate::lower) fn scalar_structural_type(
    _plan: lkjscript_ir::MemoryPlanId,
    ty: &SsaType,
) -> Option<lkjscript_native::StructuralTypeIdentity> {
    let (_layout, kind) = match ty {
        SsaType::Unit => (
            ValueType::Unit.layout_identity().get(),
            lkjscript_native::StructuralKind::Unit,
        ),
        SsaType::Bool => (
            ValueType::Bool.layout_identity().get(),
            lkjscript_native::StructuralKind::Bool,
        ),
        SsaType::I64 => (
            ValueType::I64.layout_identity().get(),
            lkjscript_native::StructuralKind::I64,
        ),
        SsaType::F64 => (
            ValueType::F64.layout_identity().get(),
            lkjscript_native::StructuralKind::F64,
        ),
        _ => return None,
    };
    let runtime_type = lkjscript_ir::runtime_structural_type(None, ty)
        .ok()
        .flatten()?;
    Some(lkjscript_native::StructuralTypeIdentity::new(
        runtime_type.layout.get(),
        runtime_type.semantic_type.get(),
        kind,
        true,
    ))
}

pub(in crate::lower) fn identity_word(bytes: &[u8]) -> u64 {
    let digest = lkjscript_core::sha256(bytes);
    u64::from_le_bytes(digest[..8].try_into().unwrap_or([0; 8])).max(1)
}

pub(in crate::lower) fn invalid_structural(detail: &str) -> LoweringError {
    LoweringError::new(LoweringFailureCode::InvalidFunction, None, detail)
}
