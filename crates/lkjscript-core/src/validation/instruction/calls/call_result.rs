fn call_return_kind(
    chunk: &Chunk,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    structural_variables: &[(u64, crate::StructuralRepresentationId)],
    memory_witnesses: &[crate::MemoryWitnessBinding],
) -> Result<Kind> {
    if let Some(representation) = proto.return_structural {
        return structural_call_result(proto, instruction, representation);
    }
    if let Some(variable) = proto.return_type_variable {
        if !proto.memory_witness_parameters.is_empty() {
            let binding = memory_witnesses
                .iter()
                .find(|binding| binding.parameter == variable)
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "generic return is missing its hidden memory witness",
                    )
                })?;
            let witness_index = usize::try_from(binding.witness).map_err(|_| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "generic return memory witness slot exceeds host usize",
                )
            })?;
            let witness = chunk
                .memory_witnesses
                .get(witness_index)
                .ok_or_else(|| {
                    instruction_error(
                        proto,
                        instruction.op(),
                        instruction.offset(),
                        "generic return memory witness slot is invalid",
                    )
                })?;
            return witness_call_result(proto, instruction, witness.value_kind);
        }
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
        let owner = call_result_identity(proto, instruction, 1, "unique")?;
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

fn witness_call_result(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    kind: crate::MemoryWitnessValueKind,
) -> Result<Kind> {
    Ok(match kind {
        crate::MemoryWitnessValueKind::Unit => Kind::Unit,
        crate::MemoryWitnessValueKind::Bool => Kind::Bool,
        crate::MemoryWitnessValueKind::I64 => Kind::I64,
        crate::MemoryWitnessValueKind::F64 => Kind::F64,
        crate::MemoryWitnessValueKind::List => Kind::List,
        crate::MemoryWitnessValueKind::Structural(representation) => {
            return structural_call_result(proto, instruction, representation)
        }
        crate::MemoryWitnessValueKind::Unsupported => {
            return Err(instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "generic return witness has no executable value route",
            ))
        }
    })
}

fn structural_call_result(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    representation: crate::StructuralRepresentationId,
) -> Result<Kind> {
    Ok(Kind::StructuralOwner {
        representation,
        owner: call_result_identity(proto, instruction, 2, "structural")?,
        active_variant: None,
    })
}

fn call_result_identity(
    _proto: &FunctionProto,
    instruction: DecodedInstruction,
    sequence: u8,
    _category: &str,
) -> Result<OwnerIdentity> {
    Ok(OwnerIdentity::instruction(instruction.offset(), sequence))
}
