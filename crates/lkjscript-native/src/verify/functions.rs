use super::*;

pub(super) fn verify_function(
    function: &FunctionPlan,
    signatures: &[(FunctionId, Signature)],
) -> Result<u64, VerificationError> {
    if function.blocks.is_empty() {
        return Err(VerificationError::EmptyFunction(function.id));
    }
    let entry = function
        .entry
        .ok_or(VerificationError::MissingEntry(function.id))?;
    block_index(function, entry)?;

    for (index, fact) in function.values.iter().enumerate() {
        if fact.id.function != function.id || fact.id.index as usize != index {
            return Err(VerificationError::InvalidValue(fact.id));
        }
        if let ValueDefinition::Parameter(parameter) = fact.definition {
            if function.signature.parameters().get(parameter).copied() != Some(fact.value_type) {
                return Err(VerificationError::InvalidValue(fact.id));
            }
        }
    }
    for (index, local) in function.locals.iter().enumerate() {
        if local.id.function != function.id || local.id.index as usize != index {
            return Err(VerificationError::InvalidLocal(local.id));
        }
    }

    let mut successors = vec![Vec::new(); function.blocks.len()];
    let mut predecessors = vec![Vec::new(); function.blocks.len()];
    for block in &function.blocks {
        let block_index_value = block_index(function, block.id)?;
        let terminator = block
            .terminator
            .as_ref()
            .ok_or(VerificationError::MissingTerminator(block.id))?;
        let targets: Vec<BlockId> = match terminator {
            Terminator::Branch(target) => vec![*target],
            Terminator::BranchIf {
                when_true,
                when_false,
                ..
            } => vec![*when_true, *when_false],
            Terminator::Return(_)
            | Terminator::Trap { .. }
            | Terminator::Exit(_)
            | Terminator::Outcome(_) => Vec::new(),
        };
        for target in targets {
            let target_index = block_index(function, target)?;
            successors[block_index_value].push(target_index);
            predecessors[target_index].push(block_index_value);
        }
    }

    let entry_index = block_index(function, entry)?;
    let mut reachable = vec![false; function.blocks.len()];
    let mut pending = vec![entry_index];
    while let Some(index) = pending.pop() {
        if reachable[index] {
            continue;
        }
        reachable[index] = true;
        pending.extend(successors[index].iter().copied());
    }
    for (index, is_reachable) in reachable.iter().enumerate() {
        if !is_reachable {
            return Err(VerificationError::UnreachableBlock(
                function.blocks[index].id,
            ));
        }
    }

    let value_count = function.values.len();
    let local_count = function.locals.len();
    let mut in_values = vec![vec![true; value_count]; function.blocks.len()];
    let mut in_locals = vec![vec![true; local_count]; function.blocks.len()];
    let parameter_count = function.signature.parameters().len();
    for value in &mut in_values[entry_index] {
        *value = false;
    }
    for index in 0..parameter_count {
        if let Some(value) = in_values[entry_index].get_mut(index) {
            *value = true;
        }
    }
    for local in &mut in_locals[entry_index] {
        *local = false;
    }

    loop {
        let mut changed = false;
        for block_index_value in 0..function.blocks.len() {
            if block_index_value == entry_index {
                continue;
            }
            let mut next_values = vec![true; value_count];
            let mut next_locals = vec![true; local_count];
            for predecessor in &predecessors[block_index_value] {
                let (out_values, out_locals) = transfer_sets(
                    function,
                    *predecessor,
                    &in_values[*predecessor],
                    &in_locals[*predecessor],
                );
                intersect(&mut next_values, &out_values);
                intersect(&mut next_locals, &out_locals);
            }
            if next_values != in_values[block_index_value] {
                in_values[block_index_value] = next_values;
                changed = true;
            }
            if next_locals != in_locals[block_index_value] {
                in_locals[block_index_value] = next_locals;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    let mut work = u64::try_from(function.blocks.len())
        .map_err(|_| VerificationError::LimitExceeded("work units"))?;
    for (block_index_value, block) in function.blocks.iter().enumerate() {
        let mut available_values = in_values[block_index_value].clone();
        let mut initialized_locals = in_locals[block_index_value].clone();
        for instruction in &block.instructions {
            verify_instruction(
                function,
                instruction,
                signatures,
                &available_values,
                &initialized_locals,
            )?;
            let output_index = value_index(function, instruction.output)?;
            let fact = &function.values[output_index];
            if fact.value_type != instruction.output_type
                || !matches!(fact.definition, ValueDefinition::Instruction(id) if id == block.id)
            {
                return Err(VerificationError::InvalidValue(instruction.output));
            }
            available_values[output_index] = true;
            if let Operation::WriteLocal(local, _) = instruction.operation {
                initialized_locals[local_index(function, local)?] = true;
            }
            work = work
                .checked_add(1)
                .ok_or(VerificationError::LimitExceeded("work units"))?;
        }
        let terminator = block
            .terminator
            .as_ref()
            .ok_or(VerificationError::MissingTerminator(block.id))?;
        for operand in terminator.operands() {
            require_available(function, operand, &available_values)?;
        }
        verify_terminator(function, terminator)?;
        work = work
            .checked_add(1)
            .ok_or(VerificationError::LimitExceeded("work units"))?;
    }
    Ok(work)
}

pub(super) fn transfer_sets(
    function: &FunctionPlan,
    block_index_value: usize,
    in_values: &[bool],
    in_locals: &[bool],
) -> (Vec<bool>, Vec<bool>) {
    let mut values = in_values.to_vec();
    let mut locals = in_locals.to_vec();
    for instruction in &function.blocks[block_index_value].instructions {
        if let Some(value) = values.get_mut(instruction.output.index as usize) {
            *value = true;
        }
        if let Operation::WriteLocal(local, _) = instruction.operation {
            if let Some(value) = locals.get_mut(local.index as usize) {
                *value = true;
            }
        }
    }
    (values, locals)
}

pub(super) fn intersect(target: &mut [bool], source: &[bool]) {
    for (target_item, source_item) in target.iter_mut().zip(source) {
        *target_item &= *source_item;
    }
}
