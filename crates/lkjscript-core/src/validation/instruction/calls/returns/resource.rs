fn consume_resource_return(state: &mut State, returned: Kind) {
    let owner = match returned {
        Kind::Resource { owner, .. } | Kind::ResourceResult { owner, .. } => owner,
        _ => return,
    };
    if owner == 0 {
        return;
    }
    for local in &mut state.locals {
        if matches!(
            local,
            Some(Kind::Resource { owner: actual, .. } | Kind::ResourceResult { owner: actual, .. })
                if *actual == owner
        ) {
            *local = None;
        }
    }
}

fn validate_resource_return(
    proto: &FunctionProto,
    actual: Kind,
    instruction: DecodedInstruction,
    is_main: bool,
) -> Result<()> {
    if is_main
        && matches!(
            actual,
            Kind::Resource { .. } | Kind::ResourceResult { .. }
        )
    {
        return Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resources cannot escape from main bytecode",
        ));
    }
    let valid = match (proto.return_resource, actual) {
        (
            Some(crate::ResourceReturnKind::Resource(expected)),
            Kind::Resource { kind, owner },
        ) => expected == kind && owner != 0,
        (
            Some(crate::ResourceReturnKind::Result(expected)),
            Kind::ResourceResult { kind, owner },
        ) => expected == kind && owner != 0,
        (None, Kind::Resource { .. } | Kind::ResourceResult { .. }) => false,
        (None, _) => true,
        (Some(_), _) => false,
    };
    if valid {
        Ok(())
    } else {
        Err(instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "typed resource return does not match declared kind",
        ))
    }
}
