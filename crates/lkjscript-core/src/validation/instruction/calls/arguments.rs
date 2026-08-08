fn validate_structural_arguments(
    chunk: &Chunk,
    callee: &FunctionProto,
    arguments: &[Kind],
    caller: &FunctionProto,
    instruction: DecodedInstruction,
) -> Result<Vec<(u64, crate::StructuralRepresentationId)>> {
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
                _,
                Kind::StructuralOwner { representation, owner, .. },
            ) if callee.parameter_requires_independent_owner(index) => {
                if consuming_owners.contains(&owner) {
                    return Err(instruction_error(
                        caller,
                        instruction.op(),
                        instruction.offset(),
                        "one structural owner is duplicated across consuming call arguments",
                    ));
                }
                consuming_owners.push(owner);
                bind_structural_variable(&mut variables, variable, representation)
            }
            (
                None,
                Some(variable),
                false,
                Kind::StructuralOwner { representation, .. },
            ) if witness_observer_parameter(callee, variable) => {
                bind_structural_variable(&mut variables, variable, representation)
            }
            (
                None,
                Some(variable),
                false,
                Kind::StructuralOwnerRef { representation, .. },
            ) if copy_structural_representation(chunk, representation)
                || callee.parameter_requires_independent_owner(index)
                || witness_observer_parameter(callee, variable) => {
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
                    Kind::StructuralOwner {
                        representation: actual,
                        ..
                    } | Kind::StructuralOwnerRef {
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

include!("arguments/helpers.rs");
