fn validate_unique_exit_state(
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
    if state
        .locals
        .iter()
        .filter_map(|slot| *slot)
        .any(|kind| match kind {
            Kind::Bytes(_) | Kind::ByteVector(_) => true,
            Kind::BytesBorrow { owner, .. } | Kind::ByteSlice { owner, .. } => owner & 0xf000_0000 != 0x9000_0000,
            _ => false,
        })
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "function return has an untransferred owner or unended loan",
        ));
    }
    Ok(())
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
    Ok(resource_return_kind(proto.return_resource))
}

fn resource_return_kind(kind: Option<crate::ResourceReturnKind>) -> Kind {
    match kind {
        Some(crate::ResourceReturnKind::Resource(kind)) => Kind::Resource(kind),
        Some(crate::ResourceReturnKind::Result(kind)) => Kind::ResourceResult(kind),
        None => Kind::Any,
    }
}

fn validate_resource_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
    is_main: bool,
) -> Result<()> {
    if is_main && matches!(actual, Kind::Resource(_) | Kind::ResourceResult(_)) {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resources cannot escape from main bytecode",
        ));
    }
    let expected = resource_return_kind(proto.return_resource);
    match (proto.return_resource, expected == actual, actual) {
        (Some(_), true, _) => Ok(()),
        (Some(_), false, _) => Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resource return does not match declared kind",
        )),
        (None, _, Kind::Resource(_) | Kind::ResourceResult(_)) => Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resource return lacks function metadata",
        )),
        (None, _, _) => Ok(()),
    }
}
