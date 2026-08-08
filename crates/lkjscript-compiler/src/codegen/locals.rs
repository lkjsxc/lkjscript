mod cleanup;
mod coloring;
mod storage;
use crate::codegen::*;
use cleanup::cleanup_values;
pub(in crate::codegen) use storage::{LocalMetadata, LocalProducerKind};

pub(in crate::codegen) struct LocalAllocation {
    pub(in crate::codegen) slots: HashMap<ValueId, usize>,
    pub(in crate::codegen) metadata: HashMap<ValueId, LocalMetadata>,
}

pub(in crate::codegen) fn allocate_locals(
    function: &Function,
    chunk: &Chunk,
) -> Result<LocalAllocation> {
    let local_metadata = storage::collect_local_metadata(function, chunk)?;
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
    let value_count = local_metadata.len();
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
            if let InstructionKind::StructuralCopy { value, .. }
            | InstructionKind::MemoryWitnessIndependentOwner { value, .. } = instruction.kind
            {
                add_edge(&mut interference, instruction.id, value)?;
            }
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
    let slots = coloring::color_locals(function, &local_metadata, interference)?;
    Ok(LocalAllocation {
        slots,
        metadata: local_metadata,
    })
}
