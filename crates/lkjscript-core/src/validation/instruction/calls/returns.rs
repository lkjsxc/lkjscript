fn validate_unique_exit_state(
    chunk: &Chunk,
    state: &State,
    proto: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    if state
        .unique_places
        .iter()
        .any(|place| !matches!(place, super::super::UniquePlaceState::Inactive))
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "function return has an active byte-vector place",
        ));
    }
    if !state.structural_destinations.is_empty() {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "function return has an incomplete structural destination",
        ));
    }
    if state
        .locals
        .iter()
        .filter_map(|slot| *slot)
        .any(|kind| match kind {
            Kind::Resource { owner, .. } | Kind::ResourceResult { owner, .. } => owner != 0,
            Kind::Bytes(_) | Kind::ByteVector(_) => true,
            Kind::StructuralOwner { representation, .. } => chunk
                .structural_representations
                .get(representation.index())
                .and_then(|representation| {
                    chunk.structural_types.get(representation.type_id.index())
                })
                .is_none_or(|ty| ty.mode != crate::StructuralTypeMode::Copy),
            Kind::BytesBorrow { owner, .. } | Kind::ByteSlice { owner, .. } => owner & 0xf000_0000 != 0x9000_0000,
            Kind::StructuralOwnerRef { .. } => false,
            Kind::StructuralView { .. } | Kind::StructuralDestination { .. } => true,
            _ => false,
        })
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            &format!(
                "function return has an untransferred owner or unended loan: {:?}",
                state
                    .locals
                    .iter()
                    .enumerate()
                    .filter_map(|(slot, kind)| kind.map(|kind| (slot, kind)))
                    .collect::<Vec<_>>(),
            ),
        ));
    }
    Ok(())
}

fn validate_structural_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
) -> Result<()> {
    let valid = match (proto.return_structural, actual) {
        (
            Some(expected),
            Kind::StructuralOwner {
                representation, ..
            },
        ) => expected == representation,
        (
            None,
            Kind::StructuralOwner { .. }
            | Kind::StructuralOwnerRef { .. }
            | Kind::StructuralView { .. }
            | Kind::StructuralDestination { .. },
        ) => false,
        (Some(_), _) => false,
        (None, _) => true,
    };
    if valid {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "structural return does not match exact function metadata",
        ))
    }
}

fn validate_unique_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
) -> Result<()> {
    let valid = match (proto.return_unique, actual) {
        (Some(crate::UniqueValueKind::Bytes), Kind::StaticBytes | Kind::Bytes(_)) => true,
        (Some(crate::UniqueValueKind::ByteVector), Kind::ByteVector(_)) => true,
        (Some(_), _) => false,
        (
            None,
            Kind::Bytes(_)
            | Kind::BytesBorrow { .. }
            | Kind::ByteVector(_)
            | Kind::ByteSlice { .. },
        ) => false,
        (None, _) => true,
    };
    if valid {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "unique return does not match exact function metadata",
        ))
    }
}

fn call_return_kind(proto: &FunctionProto, instruction: DecodedInstruction) -> Result<Kind> {
    if let Some(representation) = proto.return_structural {
        let owner = u32::try_from(instruction.offset())
            .ok()
            .and_then(|offset| offset.checked_add(0x5000_0001))
            .ok_or_else(|| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "structural call-result identity overflow",
                )
            })?;
        return Ok(Kind::StructuralOwner {
            representation,
            owner,
            active_variant: None,
        });
    }
    if let Some(kind) = proto.return_unique {
        let owner = u32::try_from(instruction.offset())
            .ok()
            .and_then(|offset| offset.checked_add(0x4000_0001))
            .ok_or_else(|| {
                instruction_error(
                    proto,
                    instruction.op(),
                    instruction.offset(),
                    "unique call-result identity overflow",
                )
            })?;
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

include!("returns/resource.rs");
