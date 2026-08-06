fn independent_owner_parameter(callee: &FunctionProto, variable: u64) -> bool {
    callee
        .memory_witness_parameters
        .iter()
        .find(|requirement| requirement.parameter == variable)
        .is_some_and(|requirement| {
            requirement.operations.contains(
                &lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
            ) && requirement
                .operations
                .contains(&lkjscript_contracts::MemoryWitnessOperation::Dispose)
        })
}

fn witness_observer_parameter(callee: &FunctionProto, variable: u64) -> bool {
    callee
        .memory_witness_parameters
        .iter()
        .find(|requirement| requirement.parameter == variable)
        .is_some_and(|requirement| {
            requirement
                .operations
                .contains(&lkjscript_contracts::MemoryWitnessOperation::Compare)
        })
}

fn bind_structural_variable(
    variables: &mut Vec<(u64, crate::StructuralRepresentationId)>,
    variable: u64,
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
