use crate::codegen::*;

pub(in crate::codegen) fn add_interference(
    interference: &mut [HashSet<ValueId>],
    definition: ValueId,
    live: &HashSet<ValueId>,
) -> Result<()> {
    for value in live {
        if *value != definition {
            add_edge(interference, definition, *value)?;
        }
    }
    Ok(())
}

pub(in crate::codegen) fn add_edge(
    interference: &mut [HashSet<ValueId>],
    left: ValueId,
    right: ValueId,
) -> Result<()> {
    let left_index = left
        .index()
        .ok_or_else(|| Error::msg("SSA interference ValueId exceeds usize"))?;
    let right_index = right
        .index()
        .ok_or_else(|| Error::msg("SSA interference ValueId exceeds usize"))?;
    let Some(left_edges) = interference.get_mut(left_index) else {
        return Err(Error::msg("SSA interference ValueId is out of range"));
    };
    left_edges.insert(right);
    let Some(right_edges) = interference.get_mut(right_index) else {
        return Err(Error::msg("SSA interference ValueId is out of range"));
    };
    right_edges.insert(left);
    Ok(())
}

pub(in crate::codegen) fn bytecode_successors(terminator: &Terminator) -> Vec<BlockId> {
    match terminator {
        Terminator::Branch { target, .. } => vec![*target],
        Terminator::ConditionalBranch {
            true_target,
            false_target,
            ..
        } => vec![*true_target, *false_target],
        _ => Vec::new(),
    }
}
