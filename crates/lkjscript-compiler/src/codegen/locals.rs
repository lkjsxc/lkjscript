mod cleanup;
use crate::codegen::*;
use cleanup::cleanup_values;
pub(in crate::codegen) fn allocate_locals(function: &Function) -> Result<HashMap<ValueId, u8>> {
    let entry = function
        .blocks
        .iter()
        .find(|block| block.id == function.entry)
        .ok_or_else(|| Error::msg("SSA function entry block is missing"))?;
    let mut value_types = HashMap::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            value_types.insert(parameter.id, parameter.ty.clone());
        }
        for instruction in &block.instructions {
            value_types.insert(instruction.id, instruction.ty.clone());
        }
    }
    let mut uses = HashMap::new();
    let mut definitions = HashMap::new();
    for block in &function.blocks {
        let mut block_uses = HashSet::new();
        let mut block_definitions: HashSet<ValueId> = block
            .parameters
            .iter()
            .map(|parameter| parameter.id)
            .collect();
        for instruction in &block.instructions {
            for operand in cleanup_values(function, instruction.metadata.failure_cleanup)? {
                if !block_definitions.contains(&operand) {
                    block_uses.insert(operand);
                }
            }
            for operand in instruction.kind.operands() {
                if !block_definitions.contains(&operand) {
                    block_uses.insert(operand);
                }
            }
            block_definitions.insert(instruction.id);
        }
        for operand in block
            .terminator
            .operands()
            .into_iter()
            .chain(cleanup_values(function, block.metadata.failure_cleanup)?)
        {
            if !block_definitions.contains(&operand) {
                block_uses.insert(operand);
            }
        }
        uses.insert(block.id, block_uses);
        definitions.insert(block.id, block_definitions);
    }
    let mut live_in: HashMap<BlockId, HashSet<ValueId>> = function
        .blocks
        .iter()
        .map(|block| (block.id, HashSet::new()))
        .collect();
    let mut live_out = live_in.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for block in function.blocks.iter().rev() {
            let mut next_out = HashSet::new();
            for successor in bytecode_successors(&block.terminator) {
                if let Some(successor_live) = live_in.get(&successor) {
                    next_out.extend(successor_live);
                }
            }
            let mut next_in = uses.get(&block.id).cloned().unwrap_or_default();
            let block_definitions = definitions.get(&block.id).cloned().unwrap_or_default();
            next_in.extend(
                next_out
                    .iter()
                    .copied()
                    .filter(|value| !block_definitions.contains(value)),
            );
            if live_out.get(&block.id) != Some(&next_out) {
                live_out.insert(block.id, next_out);
                changed = true;
            }
            if live_in.get(&block.id) != Some(&next_in) {
                live_in.insert(block.id, next_in);
                changed = true;
            }
        }
    }
    let value_count = value_types.len();
    let mut interference = vec![HashSet::new(); value_count];
    for block in &function.blocks {
        let mut live = live_out.get(&block.id).cloned().unwrap_or_default();
        live.extend(block.terminator.operands());
        live.extend(cleanup_values(function, block.metadata.failure_cleanup)?);
        for instruction in block.instructions.iter().rev() {
            live.extend(cleanup_values(
                function,
                instruction.metadata.failure_cleanup,
            )?);
            add_interference(&mut interference, instruction.id, &live)?;
            live.remove(&instruction.id);
            live.extend(instruction.kind.operands());
        }
        let parameters: Vec<ValueId> = block
            .parameters
            .iter()
            .map(|parameter| parameter.id)
            .collect();
        for parameter in &parameters {
            add_interference(&mut interference, *parameter, &live)?;
        }
        for (index, left) in parameters.iter().enumerate() {
            for right in parameters.iter().skip(index.saturating_add(1)) {
                add_edge(&mut interference, *left, *right)?;
            }
        }
    }
    let mut colors: Vec<Option<usize>> = vec![None; value_count];
    let mut color_types: Vec<SsaType> = Vec::new();
    for (slot, parameter) in entry.parameters.iter().enumerate() {
        let index = parameter
            .id
            .index()
            .ok_or_else(|| Error::msg("SSA entry parameter ValueId exceeds usize"))?;
        let Some(color) = colors.get_mut(index) else {
            return Err(Error::msg("SSA entry parameter ValueId is out of range"));
        };
        *color = Some(slot);
        color_types.push(parameter.ty.clone());
    }

    let mut order: Vec<ValueId> = value_types.keys().copied().collect();
    order.sort_by(|left, right| {
        let left_degree = left
            .index()
            .and_then(|index| interference.get(index))
            .map_or(0, HashSet::len);
        let right_degree = right
            .index()
            .and_then(|index| interference.get(index))
            .map_or(0, HashSet::len);
        right_degree.cmp(&left_degree).then_with(|| left.cmp(right))
    });
    for value in order {
        let index = value
            .index()
            .ok_or_else(|| Error::msg("SSA ValueId exceeds usize during local allocation"))?;
        if colors.get(index).copied().flatten().is_some() {
            continue;
        }
        let ty = value_types
            .get(&value)
            .ok_or_else(|| Error::msg("SSA local allocation lost a value type"))?;
        let neighbors = interference
            .get(index)
            .ok_or_else(|| Error::msg("SSA local interference metadata is inconsistent"))?;
        let color = color_types
            .iter()
            .enumerate()
            .find(|(candidate, candidate_type)| {
                *candidate_type == ty
                    && neighbors.iter().all(|neighbor| {
                        neighbor
                            .index()
                            .and_then(|index| colors.get(index))
                            .copied()
                            .flatten()
                            != Some(*candidate)
                    })
            })
            .map(|(candidate, _)| candidate)
            .unwrap_or_else(|| {
                color_types.push(ty.clone());
                color_types.len().saturating_sub(1)
            });
        let Some(destination) = colors.get_mut(index) else {
            return Err(Error::msg("SSA local color destination is out of range"));
        };
        *destination = Some(color);
    }

    if color_types.len() > usize::from(u8::MAX) {
        return Err(Error::msg(format!(
            "SSA function {} requires {} bytecode locals after liveness allocation; limit is 255",
            function.name,
            color_types.len()
        )));
    }
    let mut slots = HashMap::with_capacity(value_count);
    for (raw, color) in colors.into_iter().enumerate() {
        let value = ValueId::new(
            u32::try_from(raw).map_err(|_| Error::msg("SSA local ValueId exceeds u32"))?,
        );
        let color = color.ok_or_else(|| Error::msg("SSA value did not receive a local color"))?;
        let slot =
            u8::try_from(color).map_err(|_| Error::msg("SSA bytecode local color exceeds u8"))?;
        slots.insert(value, slot);
    }
    Ok(slots)
}
