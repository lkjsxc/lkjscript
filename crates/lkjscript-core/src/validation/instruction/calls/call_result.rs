fn call_return_kind(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    structural_variables: &[(u16, crate::StructuralRepresentationId)],
) -> Result<Kind> {
    if let Some(representation) = proto.return_structural {
        return structural_call_result(proto, instruction, representation);
    }
    if let Some(variable) = proto.return_type_variable {
        if let Some((_, representation)) = structural_variables
            .iter()
            .find(|(index, _)| *index == variable)
        {
            return structural_call_result(proto, instruction, *representation);
        }
    }
    if let Some(product) = proto.return_region_product {
        return Ok(Kind::RegionProduct(product));
    }
    if let Some(kind) = proto.return_copy_kind {
        return copy_result_kind(kind).ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "copy call-result metadata is invalid",
            )
        });
    }
    if let Some(kind) = proto.return_unique {
        let owner = call_result_identity(proto, instruction, 0x4000_0001, "unique")?;
        return Ok(match kind {
            crate::UniqueValueKind::Bytes => Kind::Bytes(owner),
            crate::UniqueValueKind::ByteVector => Kind::ByteVector(owner),
            crate::UniqueValueKind::ByteSlice | crate::UniqueValueKind::ByteSliceMut => {
                return Err(instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "byte-view returns are forbidden",
                ));
            }
        });
    }
    match proto.return_resource {
        Some(crate::ResourceReturnKind::Resource(kind)) => {
            resource_kind(kind, proto, instruction)
        }
        Some(crate::ResourceReturnKind::Result(kind)) => {
            resource_result_kind(kind, proto, instruction)
        }
        None => Ok(Kind::Any),
    }
}

fn structural_call_result(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    representation: crate::StructuralRepresentationId,
) -> Result<Kind> {
    Ok(Kind::StructuralOwner {
        representation,
        owner: call_result_identity(proto, instruction, 0x5000_0001, "structural")?,
        active_variant: None,
    })
}

fn call_result_identity(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    base: u32,
    category: &str,
) -> Result<u32> {
    u32::try_from(instruction.offset())
        .ok()
        .and_then(|offset| offset.checked_add(base))
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                &format!("{category} call-result identity overflow"),
            )
        })
}
