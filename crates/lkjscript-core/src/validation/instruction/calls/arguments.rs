fn validate_structural_arguments(
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<()> {
    let mut consuming_owners = Vec::new();
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee
            .parameter_structurals
            .get(index)
            .copied()
            .flatten();
        let owning = callee
            .parameter_structural_places
            .get(index)
            .is_some_and(Option::is_some);
        let valid = match (expected, owning, actual) {
            (
                Some(expected),
                true,
                Kind::StructuralOwner {
                    representation,
                    owner,
                    ..
                },
            ) => {
                if consuming_owners.contains(&owner) {
                    return Err(instruction_error(
                        caller,
                        instruction.op(),
                        instruction.offset(),
                        "one structural owner is duplicated across consuming call arguments",
                    ));
                }
                consuming_owners.push(owner);
                expected == representation
            }
            (
                Some(expected),
                false,
                Kind::StructuralOwnerRef { representation, .. },
            ) => expected == representation,
            (
                None,
                _,
                Kind::StructuralOwner { .. }
                | Kind::StructuralOwnerRef { .. }
                | Kind::StructuralView { .. }
                | Kind::StructuralDestination { .. },
            ) => false,
            (Some(_), _, _) => false,
            (None, _, _) => true,
        };
        if !valid {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                &format!(
                    concat!(
                        "structural call argument {index} kind {actual:?} does not match ",
                        "exact parameter metadata {expected:?}",
                    ),
                    index = index,
                    actual = actual,
                    expected = expected,
                ),
            ));
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
