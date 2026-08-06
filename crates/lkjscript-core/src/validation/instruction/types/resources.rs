pub(super) fn resource_kind(
    kind: crate::ResourceKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    Ok(Kind::Resource {
        kind,
        owner: if matches!(
            kind,
            crate::ResourceKind::InputStream | crate::ResourceKind::OutputStream
        ) {
            OwnerIdentity::None
        } else {
            resource_owner(proto, instruction)?
        },
    })
}

pub(super) fn resource_result_kind(
    kind: crate::ResourceKind,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Kind> {
    Ok(Kind::ResourceResult {
        kind,
        owner: resource_owner(proto, instruction)?,
    })
}

pub(super) const fn is_affine_resource(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Resource { owner, .. } | Kind::ResourceResult { owner, .. }
            if !owner.is_none()
    )
}

fn resource_owner(
    _proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<OwnerIdentity> {
    Ok(OwnerIdentity::instruction(instruction.offset(), 1))
}
