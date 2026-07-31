fn validate_structural_arguments(
    chunk: &Chunk,
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Vec<(u16, crate::StructuralRepresentationId)>> {
    let mut consuming_owners = Vec::new();
    let mut variables = Vec::new();
    for (index, actual) in arguments.iter().copied().enumerate() {
        let expected = callee
            .parameter_structurals
            .get(index)
            .copied()
            .flatten();
        let variable = callee
            .parameter_type_variables
            .get(index)
            .copied()
            .flatten();
        let owning = callee
            .parameter_structural_places
            .get(index)
            .is_some_and(Option::is_some);
        let valid = match (expected, variable, owning, actual) {
            (
                Some(expected),
                None,
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
                None,
                false,
                Kind::StructuralOwnerRef { representation, .. },
            ) => expected == representation,
            (
                None,
                Some(variable),
                false,
                Kind::StructuralOwnerRef { representation, .. },
            ) if copy_structural_representation(chunk, representation) => {
                bind_structural_variable(&mut variables, variable, representation)
            }
            (
                None,
                _,
                _,
                Kind::StructuralOwner { .. }
                | Kind::StructuralOwnerRef { .. }
                | Kind::StructuralView { .. }
                | Kind::StructuralDestination { .. },
            ) => false,
            (Some(_), _, _, _) => false,
            (None, _, _, _) => true,
        };
        if !valid {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                &format!(
                    concat!(
                        "structural call argument {index} kind {actual:?} does not match ",
                        "exact parameter metadata {expected:?} or variable {variable:?}",
                    ),
                    index = index,
                    actual = actual,
                    expected = expected,
                    variable = variable,
                ),
            ));
        }
    }
    for &(variable, representation) in &variables {
        for (index, parameter) in callee.parameter_type_variables.iter().enumerate() {
            if *parameter != Some(variable) {
                continue;
            }
            let matches = arguments.get(index).is_some_and(|actual| {
                matches!(
                    actual,
                    Kind::StructuralOwnerRef {
                        representation: actual,
                        ..
                    } if *actual == representation
                )
            });
            if !matches {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "repeated type-variable call arguments disagree on structural type",
                ));
            }
        }
    }
    Ok(variables)
}

fn bind_structural_variable(
    variables: &mut Vec<(u16, crate::StructuralRepresentationId)>,
    variable: u16,
    representation: crate::StructuralRepresentationId,
) -> bool {
    if let Some((_, existing)) = variables.iter().find(|(index, _)| *index == variable) {
        return *existing == representation;
    }
    variables.push((variable, representation));
    true
}

fn copy_structural_representation(
    chunk: &Chunk,
    representation: crate::StructuralRepresentationId,
) -> bool {
    chunk
        .structural_representations
        .get(representation.index())
        .filter(|item| item.id == representation)
        .and_then(|item| chunk.structural_types.get(item.type_id.index()))
        .is_some_and(|item| item.mode == crate::StructuralTypeMode::Copy)
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
