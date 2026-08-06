use lkjscript_core::{validate_chunk, Error, Limits, Result};
use lkjscript_ir::{derive_memory_inventory, verify_memory_inventory, SsaMemoryInventory};

use crate::codegen::compile_program;
use crate::ssa::lower_program_with_metrics;
use crate::ExecutableProgram;

pub(super) fn compile_analyzed(
    analyzed: &crate::hir::Program,
    limits: &Limits,
    provenance: impl FnOnce(
        &crate::HirMemoryPlan,
    ) -> Result<crate::package::program::PreparationProvenance>,
) -> Result<ExecutableProgram> {
    let memory_verified = crate::memory_plan::verify_hir_memory(analyzed)?;
    let (ssa, _) = lower_program_with_metrics(&memory_verified)?;
    let memory_plan = memory_verified.plan().clone();
    let memory_inventory = checked_memory_inventory(&ssa)?;
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    let provenance = provenance(&memory_plan)?;
    let (prepared, ssa, bytecode) =
        crate::package::program::bind(ssa, bytecode, &memory_plan, provenance, &limits.validation)?;
    Ok(ExecutableProgram {
        prepared,
        bytecode,
        ssa,
        memory_plan,
        memory_inventory,
        bytecode_links,
    })
}

pub(super) fn checked_memory_inventory(
    ssa: &lkjscript_ir::VerifiedProgram,
) -> Result<SsaMemoryInventory> {
    let inventory = derive_memory_inventory(ssa);
    verify_memory_inventory(ssa, &inventory).map_err(|error| Error::msg(error.to_string()))?;
    Ok(inventory)
}
