use lkjscript_core::{BudgetLedger, ResourceCategory, Result};
use lkjscript_ir::{Terminator, VerifiedProgram};

use super::{charge, charge_usize};

/// Exact post-normalization accounting. SSA construction and verification keep
/// their existing implementation bounds; this check precedes bytecode lowering
/// and prevents publication of an over-profile executable.
pub(crate) fn charge_ssa(program: &VerifiedProgram, ledger: &mut BudgetLedger) -> Result<()> {
    for function in &program.program().functions {
        charge(ledger, ResourceCategory::SsaFunctions, 1)?;
        for block in &function.blocks {
            charge(ledger, ResourceCategory::SsaBlocks, 1)?;
            charge_usize(ledger, ResourceCategory::SsaValues, block.parameters.len())?;
            charge_usize(
                ledger,
                ResourceCategory::SsaValues,
                block.instructions.len(),
            )?;
            if block.metadata.frame_state.is_some() {
                charge(ledger, ResourceCategory::SsaFrameStates, 1)?;
            }
            for instruction in &block.instructions {
                if instruction.metadata.frame_state.is_some() {
                    charge(ledger, ResourceCategory::SsaFrameStates, 1)?;
                }
            }
            let edges = match block.terminator {
                Terminator::Branch { .. } => 1,
                Terminator::ConditionalBranch { .. } => 2,
                _ => 0,
            };
            charge(ledger, ResourceCategory::SsaEdges, edges)?;
        }
    }
    Ok(())
}
