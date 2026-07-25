use crate::optimize::passes::*;
use crate::{verify, Program, VerifiedProgram};

pub fn normalize_baseline(program: &VerifiedProgram) -> crate::Result<VerifiedProgram> {
    let folded = constant_fold_and_propagate(program)?;
    let copied = copy_propagate(&folded)?;
    let simplified = simplify_branches(&copied)?;
    let reachable = unreachable_blocks(&simplified)?;
    let forwarded = empty_block_forwarding(&reachable)?;
    let dead = effect_aware_dce(&forwarded)?;
    let direct = direct_call_resolution(&dead)?;
    canonical_block_order(&direct)
}

pub(crate) fn finish(program: Program) -> crate::Result<VerifiedProgram> {
    verify(program)
}
