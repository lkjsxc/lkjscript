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
            0
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
        Kind::Resource { owner, .. } | Kind::ResourceResult { owner, .. } if owner != 0
    )
}

fn resource_owner(proto: &FunctionProto, instruction: DecodedInstruction) -> Result<u32> {
    u32::try_from(instruction.offset())
        .ok()
        .and_then(|offset| offset.checked_add(0x6000_0001))
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "resource owner identity overflow",
            )
        })
}
