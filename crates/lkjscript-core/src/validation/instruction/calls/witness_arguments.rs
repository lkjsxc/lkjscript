fn validate_memory_witness_arguments(
    chunk: &Chunk,
    caller: &FunctionProto,
    callee_index: u64,
    callee: &FunctionProto,
    arguments: &[Kind],
    instruction: DecodedInstruction,
) -> Result<Vec<crate::MemoryWitnessBinding>> {
    let instruction_offset = u64::try_from(instruction.offset()).map_err(|_| {
        instruction_error(
            caller,
            instruction.op(),
            instruction.offset(),
            "call offset exceeds u64",
        )
    })?;
    let site = caller
        .call_witnesses
        .binary_search_by_key(&instruction_offset, |site| site.offset)
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
        let witness_index = usize::try_from(binding.witness).map_err(|_| {
            instruction_error(
                caller,
                instruction.op(),
                instruction.offset(),
                "generic call witness slot exceeds host usize",
            )
        })?;
        let witness = chunk
            .memory_witnesses
            .get(witness_index)
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
                    .is_none_or(|actual| {
                        !witness_argument_matches(chunk, witness.value_kind, *actual)
                    })
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

fn witness_argument_matches(
    chunk: &Chunk,
    kind: crate::MemoryWitnessValueKind,
    actual: Kind,
) -> bool {
    match kind {
        crate::MemoryWitnessValueKind::Unit => actual == Kind::Unit,
        crate::MemoryWitnessValueKind::Bool => actual == Kind::Bool,
        crate::MemoryWitnessValueKind::I64 => actual == Kind::I64,
        crate::MemoryWitnessValueKind::F64 => actual == Kind::F64,
        crate::MemoryWitnessValueKind::List => actual == Kind::List,
        crate::MemoryWitnessValueKind::Structural(expected) => {
            let actual = match actual {
                Kind::StructuralOwner { representation, .. }
                | Kind::StructuralOwnerRef { representation, .. } => representation,
                _ => return false,
            };
            same_witness_representation(chunk, expected, actual)
        },
        crate::MemoryWitnessValueKind::Unsupported => false,
    }
}

fn same_witness_representation(
    chunk: &Chunk,
    left: crate::StructuralRepresentationId,
    right: crate::StructuralRepresentationId,
) -> bool {
    let Some(left) = chunk.structural_representations.get_structural(left) else { return false };
    let Some(right) = chunk.structural_representations.get_structural(right) else { return false };
    left.type_id == right.type_id
        && left.witness == right.witness
        && left.witness_group == right.witness_group
        && left.witness_member == right.witness_member
        && left.layout == right.layout
        && left.category == crate::StructuralValueCategory::Owner
        && right.category == crate::StructuralValueCategory::Owner
}
