fn validate_resource_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee.parameter_resources.get(index).copied().flatten();
        match (expected, actual) {
            (Some(expected), Kind::Resource(actual)) if expected == actual => {}
            (Some(_), _) => {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "typed resource call argument does not match declared kind",
                ));
            }
            (None, Kind::Resource(_) | Kind::ResourceResult(_)) => {
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

fn validate_unique_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee.parameter_uniques.get(index).copied().flatten();
        let valid = match (expected, actual) {
            (Some(crate::UniqueValueKind::Bytes), Kind::Bytes(_))
            | (Some(crate::UniqueValueKind::ByteVector), Kind::ByteVector(_))
            | (
                Some(crate::UniqueValueKind::ByteSlice),
                Kind::ByteSlice {
                    mutable: false,
                    used: true,
                    ..
                },
            )
            | (
                Some(crate::UniqueValueKind::ByteSliceMut),
                Kind::ByteSlice {
                    mutable: true,
                    used: true,
                    ..
                },
            ) => true,
            (
                None,
                Kind::Bytes(_)
                | Kind::BytesBorrow { .. }
                | Kind::ByteVector(_)
                | Kind::ByteSlice { .. },
            ) => false,
            (Some(_), _) => false,
            (None, _) => true,
        };
        if !valid {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                "unique call argument does not match exact parameter metadata",
            ));
        }
    }
    Ok(())
}
