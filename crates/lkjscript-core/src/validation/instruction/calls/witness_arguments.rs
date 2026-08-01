fn validate_memory_witness_arguments(
    chunk: &Chunk,
    caller: &FunctionProto,
    callee_index: u32,
    callee: &FunctionProto,
    arguments: &[Kind],
    instruction: DecodedInstruction,
) -> Result<Vec<crate::MemoryWitnessBinding>> {
    let site = caller
        .call_witnesses
        .binary_search_by_key(
            &u32::try_from(instruction.offset()).unwrap_or(u32::MAX),
            |site| site.offset,
        )
        .ok()
        .and_then(|index| caller.call_witnesses.get(index));
    if callee.memory_witness_parameters.is_empty() {
        if site.is_some() {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                "monomorphic call carries hidden memory witness metadata",
            ));
        }
        return Ok(Vec::new());
    }
    let site = site.ok_or_else(|| {
        instruction_error(
            caller,
            instruction.op(),
            instruction.offset(),
            "generic call is missing hidden memory witness metadata",
        )
    })?;
    if site.callee != callee_index || site.bindings.len() != callee.memory_witness_parameters.len() {
        return Err(instruction_error(
            caller,
            instruction.op(),
            instruction.offset(),
            "generic call witness callee or count mismatch",
        ));
    }
    for (requirement, binding) in callee
        .memory_witness_parameters
        .iter()
        .zip(&site.bindings)
    {
        let witness = chunk
            .memory_witnesses
            .get(usize::from(binding.witness))
            .ok_or_else(|| {
                instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "generic call witness slot is out of range",
                )
            })?;
        if binding.parameter != requirement.parameter
            || requirement
                .operations
                .iter()
                .any(|operation| witness.facts.operations.binary_search(operation).is_err())
        {
            return Err(instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                "generic call witness operation mismatch",
            ));
        }
        for (index, variable) in callee.parameter_type_variables.iter().enumerate() {
            if *variable == Some(requirement.parameter)
                && arguments
                    .get(index)
                    .is_none_or(|actual| !witness_argument_matches(witness.value_kind, *actual))
            {
                return Err(instruction_error(
                    caller,
                    instruction.op(),
                    instruction.offset(),
                    "generic call witness type does not match argument",
                ));
            }
        }
    }
    Ok(site.bindings.clone())
}

fn witness_argument_matches(kind: crate::MemoryWitnessValueKind, actual: Kind) -> bool {
    match kind {
        crate::MemoryWitnessValueKind::Unit => actual == Kind::Unit,
        crate::MemoryWitnessValueKind::Bool => actual == Kind::Bool,
        crate::MemoryWitnessValueKind::I64 => actual == Kind::I64,
        crate::MemoryWitnessValueKind::F64 => actual == Kind::F64,
        crate::MemoryWitnessValueKind::List => actual == Kind::List,
        crate::MemoryWitnessValueKind::Structural(expected) => matches!(
            actual,
            Kind::StructuralOwner { representation, .. }
                | Kind::StructuralOwnerRef { representation, .. }
                if representation == expected
        ),
        crate::MemoryWitnessValueKind::Unsupported => false,
    }
}
