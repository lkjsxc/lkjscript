use super::*;
use std::num::NonZeroU64;

pub(super) fn core_type(
    value_type: StructuralTypeIdentity,
) -> Result<StructuralType, NativeServiceError> {
    let layout = NonZeroU64::new(value_type.layout())
        .map(LayoutIdentity::new)
        .ok_or(NativeServiceError::Trap)?;
    let semantic_type = NonZeroU64::new(value_type.semantic_type())
        .map(SemanticTypeIdentity::new)
        .ok_or(NativeServiceError::Trap)?;
    Ok(StructuralType::new(
        layout,
        semantic_type,
        core_kind(value_type.kind()),
    ))
}

pub(super) const fn core_kind(kind: lkjscript_native::StructuralKind) -> StructuralKind {
    match kind {
        lkjscript_native::StructuralKind::Unit => StructuralKind::Unit,
        lkjscript_native::StructuralKind::Bool => StructuralKind::Bool,
        lkjscript_native::StructuralKind::I64 => StructuralKind::I64,
        lkjscript_native::StructuralKind::F64 => StructuralKind::F64,
        lkjscript_native::StructuralKind::String => StructuralKind::String,
        lkjscript_native::StructuralKind::Path => StructuralKind::Path,
        lkjscript_native::StructuralKind::Bytes => StructuralKind::Bytes,
        lkjscript_native::StructuralKind::ByteVector => StructuralKind::ByteVector,
        lkjscript_native::StructuralKind::Product => StructuralKind::Product,
        lkjscript_native::StructuralKind::Enum => StructuralKind::Enum,
        lkjscript_native::StructuralKind::Static => StructuralKind::Static,
    }
}

pub(super) fn owner_key(
    owner: NativeStructuralOwner,
) -> Result<StructuralValueKey, NativeServiceError> {
    StructuralValueKey::from_word(owner.opaque_word()).ok_or(NativeServiceError::Trap)
}

pub(super) fn view_key(
    view: NativeStructuralView,
) -> Result<StructuralViewKey, NativeServiceError> {
    StructuralViewKey::from_word(view.opaque_word()).ok_or(NativeServiceError::Trap)
}

pub(super) fn destination_key(
    destination: NativeStructuralDestination,
) -> Result<StructuralDestinationKey, NativeServiceError> {
    StructuralDestinationKey::from_word(destination.opaque_word()).ok_or(NativeServiceError::Trap)
}

pub(super) fn payload(bytes: Vec<u8>, kind: StructuralPayloadKind) -> SemanticPayload {
    match kind {
        StructuralPayloadKind::String => SemanticPayload::String(bytes),
        StructuralPayloadKind::Path => SemanticPayload::Path(bytes),
        StructuralPayloadKind::Bytes => SemanticPayload::Bytes(bytes),
        StructuralPayloadKind::ByteVector => SemanticPayload::ByteVector(bytes),
    }
}

pub(super) fn node_bytes(node: StructuralNode<'_>) -> Result<&[u8], NativeServiceError> {
    match node.payload() {
        StructuralNodeView::Bytes(bytes) => Ok(bytes),
        _ => Err(NativeServiceError::Trap),
    }
}
