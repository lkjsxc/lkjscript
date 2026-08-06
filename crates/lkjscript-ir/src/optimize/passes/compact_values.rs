mod rewrite;
pub(crate) use rewrite::rewrite_function_values;

use std::collections::{HashMap, HashSet};

use crate::{
    CallTarget, FailureCleanupAction, FailureCleanupId, FailureCleanupNode, FailureCleanupRoots,
    FrameState, InstructionKind, Program, Terminator, ValueId,
};

pub(crate) fn compact_values(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        compact_failure_cleanups(function)?;
        let mut mapping = HashMap::new();
        let mut next = 0_u64;
        for block in &function.blocks {
            for parameter in &block.parameters {
                mapping.insert(parameter.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value identity exceeds u64"))?;
            }
            for instruction in &block.instructions {
                mapping.insert(instruction.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value identity exceeds u64"))?;
            }
        }
        for block in &mut function.blocks {
            for parameter in &mut block.parameters {
                parameter.id = mapped_value(&mapping, parameter.id)?;
            }
            for instruction in &mut block.instructions {
                instruction.id = mapped_value(&mapping, instruction.id)?;
            }
        }
        let mut missing_value = None;
        rewrite_function_values(function, |value| match mapping.get(&value).copied() {
            Some(mapped) => mapped,
            None => {
                missing_value.get_or_insert(value);
                value
            }
        });
        if let Some(value) = missing_value {
            return Err(crate::IrError::new(format!(
                "pass lost failure-cleanup SSA value {}",
                value.raw()
            )));
        }
    }
    super::compact_places(program)
}

fn compact_failure_cleanups(function: &mut crate::Function) -> crate::Result<()> {
    let mut used = HashSet::new();
    let mut pending = Vec::new();
    for block in &function.blocks {
        pending.extend(
            block
                .metadata
                .failure_cleanup
                .into_iter()
                .flat_map(FailureCleanupRoots::ids),
        );
        for instruction in &block.instructions {
            pending.extend(
                instruction
                    .metadata
                    .failure_cleanup
                    .into_iter()
                    .flat_map(FailureCleanupRoots::ids),
            );
        }
    }
    while let Some(id) = pending.pop() {
        if !used.insert(id) {
            continue;
        }
        let node = function
            .failure_cleanups
            .get(id.index().unwrap_or(usize::MAX))
            .ok_or_else(|| crate::IrError::new("pass found an invalid cleanup root"))?;
        pending.extend(node.next);
    }

    let mut mapping = HashMap::with_capacity(used.len());
    let mut nodes = Vec::with_capacity(used.len());
    for (index, node) in function.failure_cleanups.iter().copied().enumerate() {
        let old = FailureCleanupId::new(
            u64::try_from(index)
                .map_err(|_| crate::IrError::new("failure-cleanup node count exceeds u64"))?,
        );
        if !used.contains(&old) {
            continue;
        }
        let next = node
            .next
            .map(|next| {
                mapping
                    .get(&next)
                    .copied()
                    .ok_or_else(|| crate::IrError::new("pass lost cleanup chain tail"))
            })
            .transpose()?;
        let id = FailureCleanupId::new(
            u64::try_from(nodes.len())
                .map_err(|_| crate::IrError::new("failure-cleanup node count exceeds u64"))?,
        );
        mapping.insert(old, id);
        nodes.push(FailureCleanupNode {
            action: node.action,
            next,
        });
    }
    for block in &mut function.blocks {
        remap_failure_cleanup(&mut block.metadata.failure_cleanup, &mapping)?;
        for instruction in &mut block.instructions {
            remap_failure_cleanup(&mut instruction.metadata.failure_cleanup, &mapping)?;
        }
    }
    function.failure_cleanups = nodes;
    Ok(())
}

fn remap_failure_cleanup(
    cleanup: &mut Option<FailureCleanupRoots>,
    mapping: &HashMap<FailureCleanupId, FailureCleanupId>,
) -> crate::Result<()> {
    let Some(roots) = cleanup else {
        return Ok(());
    };
    for id in [&mut roots.loans, &mut roots.unplaced, &mut roots.places]
        .into_iter()
        .flatten()
    {
        *id = mapping
            .get(id)
            .copied()
            .ok_or_else(|| crate::IrError::new("pass lost failure-cleanup chain"))?;
    }
    Ok(())
}

pub(crate) fn mapped_value(
    mapping: &HashMap<ValueId, ValueId>,
    id: ValueId,
) -> crate::Result<ValueId> {
    mapping
        .get(&id)
        .copied()
        .ok_or_else(|| crate::IrError::new(format!("pass lost SSA value {}", id.raw())))
}
