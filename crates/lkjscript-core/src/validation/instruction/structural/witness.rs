fn witness_compare(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let parameter = u64::try_from(instruction_operand(proto, instruction)?)
        .map_err(|_| crate::Error::msg("memory witness parameter exceeds u64"))?;
    let requirement = proto
        .memory_witness_parameters
        .iter()
        .find(|requirement| requirement.parameter == parameter)
        .ok_or_else(|| instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "memory compare parameter is missing",
        ))?;
    if !requirement
        .operations
        .contains(&lkjscript_contracts::MemoryWitnessOperation::Compare)
    {
        return fail(proto, instruction, "memory witness does not authorize compare");
    }
    for _ in 0..2 {
        match pop(state, proto, instruction)? {
            Kind::StructuralOwner { .. }
            | Kind::StructuralOwnerRef { .. }
            | Kind::Any => {}
            _ => return fail(proto, instruction, "memory compare expects witness values"),
        }
    }
    state.stack.push(Kind::Bool);
    Ok(())
}

fn witness_dispose(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let parameter = u64::try_from(instruction_operand(proto, instruction)?)
        .map_err(|_| crate::Error::msg("memory witness parameter exceeds u64"))?;
    let requirement = proto
        .memory_witness_parameters
        .iter()
        .find(|requirement| requirement.parameter == parameter)
        .ok_or_else(|| {
            instruction_error(
                proto,
                instruction.op(),
                instruction.offset(),
                "memory dispose parameter is missing",
            )
        })?;
    if !requirement
        .operations
        .contains(&lkjscript_contracts::MemoryWitnessOperation::Dispose)
    {
        return fail(proto, instruction, "memory witness does not authorize dispose");
    }
    match pop(state, proto, instruction)? {
        Kind::StructuralOwner { .. } | Kind::Any => state.stack.push(Kind::Unit),
        _ => return fail(proto, instruction, "memory dispose expects an owned value"),
    }
    Ok(())
}

fn witness_independent_owner(
    proto: &FunctionProto,
    instruction: DecodedInstruction,
    state: &mut State,
) -> Result<()> {
    let parameter = u64::try_from(instruction_operand(proto, instruction)?)
        .map_err(|_| crate::Error::msg("memory witness parameter exceeds u64"))?;
    let requirement = proto.memory_witness_parameters.iter()
        .find(|requirement| requirement.parameter == parameter)
        .ok_or_else(|| instruction_error(
            proto,
            instruction.op(),
            instruction.offset(),
            "memory independent-owner parameter is missing",
        ))?;
    if !requirement.operations.contains(
        &lkjscript_contracts::MemoryWitnessOperation::IndependentOwner,
    ) {
        return fail(
            proto,
            instruction,
            "memory witness does not authorize independent-owner",
        );
    }
    let input = pop(state, proto, instruction)?;
    match input {
        Kind::StructuralOwnerRef { representation, .. }
        | Kind::StructuralOwner { representation, .. } => {
            state.stack.push(Kind::StructuralOwner {
                representation,
                owner: fresh_identity(proto, instruction, 7)?,
                active_variant: None,
            });
        }
        Kind::Any => state.stack.push(Kind::Any),
        _ => return fail(
            proto,
            instruction,
            "memory independent-owner expects witness-compatible value",
        ),
    }
    Ok(())
}
