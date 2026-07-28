mod rewrite;
pub(crate) use rewrite::rewrite_function_values;

use std::collections::{HashMap, HashSet};

use crate::{
    CallTarget, FailureCleanupAction, FailureCleanupId, FrameState, InstructionKind, Program,
    Terminator, ValueId,
};

pub(crate) fn compact_values(program: &mut Program) -> crate::Result<()> {
    for function in &mut program.functions {
        compact_failure_cleanups(function)?;
        let mut mapping = HashMap::new();
        let mut next = 0_u32;
        for block in &function.blocks {
            for parameter in &block.parameters {
                mapping.insert(parameter.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value count exceeds u32"))?;
            }
            for instruction in &block.instructions {
                mapping.insert(instruction.id, ValueId::new(next));
                next = next
                    .checked_add(1)
                    .ok_or_else(|| crate::IrError::new("SSA value count exceeds u32"))?;
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
    for block in &function.blocks {
        used.extend(block.metadata.failure_cleanup);
        for instruction in &block.instructions {
            used.extend(instruction.metadata.failure_cleanup);
        }
    }
    let mut mapping = HashMap::new();
    let mut plans = Vec::with_capacity(used.len());
    for plan in &function.failure_cleanups {
        if used.contains(&plan.id) {
            let raw = u32::try_from(plans.len())
                .map_err(|_| crate::IrError::new("failure-cleanup plan count exceeds u32"))?;
            let id = FailureCleanupId::new(raw);
            mapping.insert(plan.id, id);
            let mut plan = plan.clone();
            plan.id = id;
            plans.push(plan);
        }
    }
    for block in &mut function.blocks {
        remap_failure_cleanup(&mut block.metadata.failure_cleanup, &mapping)?;
        for instruction in &mut block.instructions {
            remap_failure_cleanup(&mut instruction.metadata.failure_cleanup, &mapping)?;
        }
    }
    function.failure_cleanups = plans;
    Ok(())
}

fn remap_failure_cleanup(
    cleanup: &mut Option<FailureCleanupId>,
    mapping: &HashMap<FailureCleanupId, FailureCleanupId>,
) -> crate::Result<()> {
    if let Some(id) = cleanup {
        *id = mapping
            .get(id)
            .copied()
            .ok_or_else(|| crate::IrError::new("pass lost failure-cleanup plan"))?;
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
