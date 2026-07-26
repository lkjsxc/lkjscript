use lkjscript_core::{BudgetAuthority, BudgetLedger, ResourceCategory, Result};
use lkjscript_ir::{Terminator, VerifiedProgram};

use super::{checked_add, count_usize, reserve};

#[derive(Default)]
struct SsaCharges {
    functions: u64,
    blocks: u64,
    values: u64,
    edges: u64,
    frame_states: u64,
}

/// Measure immutable normalized SSA and reserve its exact charged input shape
/// before bytecode construction allocates chunks, prototypes, code, or links.
pub(crate) fn reserve_bytecode_input(
    program: &VerifiedProgram,
    ledger: &mut BudgetLedger,
) -> Result<()> {
    let mut charges = SsaCharges::default();
    for function in &program.program().functions {
        checked_add(&mut charges.functions, 1, ResourceCategory::SsaFunctions)?;
        for block in &function.blocks {
            checked_add(&mut charges.blocks, 1, ResourceCategory::SsaBlocks)?;
            checked_add(
                &mut charges.values,
                count_usize(ResourceCategory::SsaValues, block.parameters.len())?,
                ResourceCategory::SsaValues,
            )?;
            checked_add(
                &mut charges.values,
                count_usize(ResourceCategory::SsaValues, block.instructions.len())?,
                ResourceCategory::SsaValues,
            )?;
            if block.metadata.frame_state.is_some() {
                checked_add(
                    &mut charges.frame_states,
                    1,
                    ResourceCategory::SsaFrameStates,
                )?;
            }
            for instruction in &block.instructions {
                if instruction.metadata.frame_state.is_some() {
                    checked_add(
                        &mut charges.frame_states,
                        1,
                        ResourceCategory::SsaFrameStates,
                    )?;
                }
            }
            let edges = match block.terminator {
                Terminator::Branch { .. } => 1,
                Terminator::ConditionalBranch { .. } => 2,
                _ => 0,
            };
            checked_add(&mut charges.edges, edges, ResourceCategory::SsaEdges)?;
        }
    }
    for (category, amount) in [
        (ResourceCategory::SsaFunctions, charges.functions),
        (ResourceCategory::SsaBlocks, charges.blocks),
        (ResourceCategory::SsaValues, charges.values),
        (ResourceCategory::SsaEdges, charges.edges),
        (ResourceCategory::SsaFrameStates, charges.frame_states),
    ] {
        reserve(ledger, BudgetAuthority::Bytecode, category, amount)?;
    }
    Ok(())
}
