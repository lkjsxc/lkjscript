use crate::optimize::passes::*;
use crate::{verify, Program, VerifiedProgram};

pub fn normalize_baseline(program: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let folded = constant_fold_and_propagate(program)
        .map_err(|error| crate::IrError::new(format!("constant fold: {error}")))?;
    let copied = copy_propagate(&folded)
        .map_err(|error| crate::IrError::new(format!("copy propagation: {error}")))?;
    let simplified = simplify_branches(&copied)
        .map_err(|error| crate::IrError::new(format!("branch simplify: {error}")))?;
    let reachable = unreachable_blocks(&simplified)
        .map_err(|error| crate::IrError::new(format!("reachability: {error}")))?;
    let forwarded = empty_block_forwarding(&reachable)
        .map_err(|error| crate::IrError::new(format!("block forwarding: {error}")))?;
    let dead = effect_aware_dce(&forwarded)
        .map_err(|error| crate::IrError::new(format!("effect DCE: {error}")))?;
    let direct = direct_call_resolution(&dead)
        .map_err(|error| crate::IrError::new(format!("direct calls: {error}")))?;
    canonical_block_order(&direct)
        .map_err(|error| crate::IrError::new(format!("block order: {error}")))
}

pub(crate) fn finish(program: Program) -> crate::Result<VerifiedProgram> {
    verify(program)
}
