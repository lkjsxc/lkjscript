use super::*;

pub(crate) fn nonowned_affine_values(function: &Function) -> HashSet<ValueId> {
    let mut values = HashSet::new();
    for block in &function.blocks {
        for parameter in &block.parameters {
            if parameter.owner_place.is_none() {
                values.insert(parameter.id);
            }
        }
        for instruction in &block.instructions {
            if matches!(
                instruction.kind,
                InstructionKind::Constant(crate::Constant::StaticBytes(_))
                    | InstructionKind::Borrow { .. }
            ) && instruction.ty == SsaType::Bytes
                || matches!(
                    instruction.kind,
                    InstructionKind::Runtime {
                        operation: crate::RuntimeOp::StdinHandle,
                        ..
                    }
                )
            {
                values.insert(instruction.id);
            }
        }
    }
    let mut dependents: BTreeMap<ValueId, Vec<ValueId>> = BTreeMap::new();
    let mut rejected = Vec::new();
    for block in &function.blocks {
        if block.id == function.entry {
            continue;
        }
        for (index, parameter) in block.parameters.iter().enumerate() {
            if parameter.owner_place.is_some() {
                continue;
            }
            for predecessor in &function.blocks {
                for argument in edge_arguments_to(&predecessor.terminator, block.id, index) {
                    dependents.entry(argument).or_default().push(parameter.id);
                    if !values.contains(&argument) {
                        rejected.push(parameter.id);
                    }
                }
            }
        }
    }
    while let Some(value) = rejected.pop() {
        if values.remove(&value) {
            rejected.extend(dependents.get(&value).into_iter().flatten().copied());
        }
    }
    values
}

fn edge_arguments_to(terminator: &Terminator, target: BlockId, index: usize) -> Vec<ValueId> {
    match terminator {
        Terminator::Branch {
            target: actual,
            arguments,
        } if *actual == target => arguments.get(index).copied().into_iter().collect(),
        Terminator::ConditionalBranch {
            true_target,
            true_arguments,
            false_target,
            false_arguments,
            ..
        } => {
            let mut values = Vec::with_capacity(2);
            if *true_target == target {
                values.extend(true_arguments.get(index).copied());
            }
            if *false_target == target {
                values.extend(false_arguments.get(index).copied());
            }
            values
        }
        _ => Vec::new(),
    }
}
