fn validate_resource_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let mut owners = Vec::new();
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee.parameter_resources.get(index).copied().flatten();
        let owning = callee
            .parameter_resource_places
            .get(index)
            .is_some_and(Option::is_some);
        match (expected, actual) {
            (
                Some(expected),
                Kind::Resource {
                    kind: actual,
                    owner,
                },
            ) if expected == actual && (!owning || owner != 0) => {
                if owning {
                    if owners.contains(&owner) {
                        return Err(instruction_error(
                            caller,
                            instruction.op(),
                            instruction.offset(),
                            "one resource owner is duplicated across consuming call arguments",
                        ));
                    }
                    owners.push(owner);
                }
            }
            (Some(_), _) => {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "typed resource call argument does not match declared kind",
                ));
            }
            (None, Kind::Resource { .. } | Kind::ResourceResult { .. }) => {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "typed resource call argument lacks parameter metadata",
                ));
            }
            (None, _) => {}
        }
    }
    Ok(())
}

fn consume_resource_arguments(
    state: &mut State,
    callee: &FunctionProto,
    arguments: &[Kind],
) {
    for (index, argument) in arguments.iter().enumerate() {
        if !callee
            .parameter_resource_places
            .get(index)
            .is_some_and(Option::is_some)
        {
            continue;
        }
        let Kind::Resource { owner, .. } = argument else {
            continue;
        };
        if *owner == 0 {
            continue;
        }
        for local in &mut state.locals {
            if matches!(local, Some(Kind::Resource { owner: actual, .. }) if actual == owner) {
                *local = None;
            }
        }
    }
}
