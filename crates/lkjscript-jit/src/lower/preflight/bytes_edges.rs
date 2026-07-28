use super::*;

pub(super) fn connect_bytes_call(
    program: &lkjscript_ir::Program,
    caller: &Function,
    callee: FunctionId,
    arguments: &[ValueId],
    output: BytesNode,
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    let target = source_function(program, callee)?;
    let entry_index = target
        .entry
        .index()
        .ok_or_else(|| bytes_mode_error(callee))?;
    let entry = target
        .blocks
        .get(entry_index)
        .ok_or_else(|| bytes_mode_error(callee))?;
    for (argument, parameter) in arguments.iter().zip(&entry.parameters) {
        if parameter.ty == SsaType::Bytes {
            connect_bytes(
                caller.id,
                BytesNode::Value(caller.id, *argument),
                BytesNode::Value(callee, parameter.id),
                indexes,
                sets,
            )?;
        }
    }
    if target.signature.result.as_ref() == &SsaType::Bytes {
        connect_bytes(caller.id, output, BytesNode::Result(callee), indexes, sets)?;
    }
    Ok(())
}

pub(super) fn connect_bytes_terminator(
    _program: &lkjscript_ir::Program,
    function: &Function,
    terminator: &Terminator,
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    match terminator {
        Terminator::Return(value) if function.signature.result.as_ref() == &SsaType::Bytes => {
            connect_bytes(
                function.id,
                BytesNode::Value(function.id, *value),
                BytesNode::Result(function.id),
                indexes,
                sets,
            )?;
        }
        Terminator::Branch { target, arguments } => {
            connect_bytes_edge(function, *target, arguments, indexes, sets)?;
        }
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => {
            connect_bytes_edge(function, *true_target, true_arguments, indexes, sets)?;
            connect_bytes_edge(function, *false_target, false_arguments, indexes, sets)?;
        }
        _ => {}
    }
    Ok(())
}

fn connect_bytes_edge(
    function: &Function,
    target: lkjscript_ir::BlockId,
    arguments: &[ValueId],
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    let index = target
        .index()
        .ok_or_else(|| bytes_mode_error(function.id))?;
    let block = function
        .blocks
        .get(index)
        .ok_or_else(|| bytes_mode_error(function.id))?;
    for (argument, parameter) in arguments.iter().zip(&block.parameters) {
        if parameter.ty == SsaType::Bytes {
            connect_bytes(
                function.id,
                BytesNode::Value(function.id, *argument),
                BytesNode::Value(function.id, parameter.id),
                indexes,
                sets,
            )?;
        }
    }
    Ok(())
}

pub(super) fn assign_if_bytes(
    function: FunctionId,
    value: ValueId,
    mode: BytesMode,
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    let node = BytesNode::Value(function, value);
    if indexes.contains_key(&node) {
        assign_bytes(function, node, mode, indexes, sets)?;
    }
    Ok(())
}

pub(super) fn assign_bytes(
    function: FunctionId,
    node: BytesNode,
    mode: BytesMode,
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    sets.assign(indexes[&node], mode)
        .map_err(|()| bytes_mode_error(function))
}

pub(super) fn connect_bytes(
    function: FunctionId,
    left: BytesNode,
    right: BytesNode,
    indexes: &HashMap<BytesNode, usize>,
    sets: &mut BytesSets,
) -> Result<(), LoweringError> {
    sets.union(indexes[&left], indexes[&right])
        .map_err(|()| bytes_mode_error(function))
}
