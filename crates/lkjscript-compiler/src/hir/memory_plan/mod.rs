use lkjscript_core::Result;

use crate::hir;

mod encoding;
mod executable_witness;
mod model;
mod producer;
mod verifier;

pub use model::*;

use encoding::compute_plan_id;
pub(crate) use executable_witness::{executable_dependencies, executable_facts};

pub(crate) fn memory_type_identity(ty: &MemoryType) -> Result<[u8; 32]> {
    encoding::compute_type_id(ty)
}

pub const MAX_MEMORY_PLAN_FUNCTIONS: u64 = 4_096;
pub const MAX_MEMORY_PLAN_EXPRESSIONS: u64 = 16_384;
pub const MAX_MEMORY_PLAN_ENTRIES: u64 = 65_536;
pub const MAX_MEMORY_PLAN_USES: u64 = 65_536;
pub const MAX_MEMORY_PLAN_LOANS: u64 = 16_384;
pub const MAX_MEMORY_PLAN_CONSTANTS: u64 = 16_384;
pub const MAX_MEMORY_PLAN_CALLS: u64 = 16_384;
pub const MAX_MEMORY_PLAN_OBLIGATIONS: u64 = 32_768;
pub const MAX_MEMORY_PLAN_TYPE_NODES: u64 = 16_384;
pub const MAX_MEMORY_PLAN_WITNESSES: u64 = 16_384;
pub const MAX_MEMORY_WITNESS_PARAMETERS: u64 = 16;
pub const MAX_MEMORY_WITNESS_ARGUMENTS: u64 = 16;
pub const MAX_MEMORY_PLAN_TYPE_EDGES: u64 = 65_536;
pub const MAX_MEMORY_PLAN_SCC_WORK: u64 = 65_536;
pub const MAX_MEMORY_PLAN_AGGREGATE_FIELDS: u64 = 65_536;
pub const MAX_MEMORY_PLAN_AGGREGATE_VARIANTS: u64 = 16_384;
pub const MAX_MEMORY_PLAN_DESTINATIONS: u64 = 16_384;
pub const MAX_MEMORY_PLAN_BORROW_SCOPES: u64 = 16_384;
pub const MAX_MEMORY_PLAN_DROP_PATHS: u64 = 32_768;
pub const MAX_MEMORY_PLAN_VERIFIER_STEPS: u64 = 262_144;

/// Opaque proof that one exact HIR program has a complete independently
/// verified memory plan. SSA construction accepts only this wrapper.
pub(crate) struct MemoryVerifiedHir<'a> {
    hir: &'a hir::Program,
    plan: HirMemoryPlan,
}

impl<'a> MemoryVerifiedHir<'a> {
    pub(crate) fn hir(&self) -> &'a hir::Program {
        self.hir
    }

    pub(crate) fn plan(&self) -> &HirMemoryPlan {
        &self.plan
    }
}

pub(crate) fn verify_hir_memory(program: &hir::Program) -> Result<MemoryVerifiedHir<'_>> {
    let mut plan = producer::derive(program)?;
    let verifier_steps = verifier::verify(program, &plan)?;
    plan.work.verifier_steps = verifier_steps;
    // Verifier work is evidence, not a semantic input, so the content identity
    // remains stable before and after independent verification.
    plan.id = compute_plan_id(&plan)?;
    Ok(MemoryVerifiedHir { hir: program, plan })
}

#[cfg(test)]
mod tests;
