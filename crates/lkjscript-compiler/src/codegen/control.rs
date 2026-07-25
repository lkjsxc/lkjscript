use crate::codegen::*;

pub(in crate::codegen) fn tail_path_returns(
    function: &Function,
    terminator: &Terminator,
    value: ValueId,
) -> bool {
    match terminator {
        Terminator::Return(returned) => *returned == value,
        Terminator::Branch { target, arguments } => {
            follow_empty_tail_path(function, *target, arguments, value, &mut HashSet::new())
        }
        _ => false,
    }
}

pub(in crate::codegen) fn follow_empty_tail_path(
    function: &Function,
    target: BlockId,
    arguments: &[ValueId],
    value: ValueId,
    visited: &mut HashSet<BlockId>,
) -> bool {
    if !visited.insert(target) {
        return false;
    }
    let Some(block) = function.blocks.iter().find(|block| block.id == target) else {
        return false;
    };
    if !block.instructions.is_empty() || block.parameters.len() != arguments.len() {
        return false;
    }
    let substitutions: HashMap<ValueId, ValueId> = block
        .parameters
        .iter()
        .zip(arguments)
        .map(|(parameter, argument)| (parameter.id, *argument))
        .collect();
    let resolve = |candidate: ValueId| substitutions.get(&candidate).copied().unwrap_or(candidate);
    match &block.terminator {
        Terminator::Return(returned) => resolve(*returned) == value,
        Terminator::Branch { target, arguments } => {
            let arguments: Vec<ValueId> = arguments
                .iter()
                .map(|argument| resolve(*argument))
                .collect();
            follow_empty_tail_path(function, *target, &arguments, value, visited)
        }
        _ => false,
    }
}
