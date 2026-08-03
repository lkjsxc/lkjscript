use lkjscript_core::{validate_chunk, BudgetLedger, Error, Limits, Result};
use lkjscript_ir::{derive_memory_inventory, verify_memory_inventory, SsaMemoryInventory};

use crate::codegen::compile_program;
use crate::ssa::lower_program_with_budget;
use crate::ExecutableProgram;

pub(super) fn compile_analyzed(
    analyzed: &crate::hir::Program,
    limits: &Limits,
    ledger: &mut BudgetLedger,
    provenance: impl FnOnce(
        &crate::HirMemoryPlan,
    ) -> Result<crate::package::program::PreparationProvenance>,
) -> Result<ExecutableProgram> {
    let memory_verified = crate::memory_plan::verify_hir_memory(analyzed)?;
    let ssa = lower_program_with_budget(&memory_verified, ledger)?;
    let memory_plan = memory_verified.plan().clone();
    let memory_inventory = checked_memory_inventory(&ssa)?;
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    let provenance = provenance(&memory_plan)?;
    let (prepared, ssa, bytecode) = crate::package::program::bind(
        ssa,
        bytecode,
        &memory_plan,
        ledger.profile().identity(),
        provenance,
        &limits.validation,
    )?;
    Ok(ExecutableProgram {
        prepared,
        bytecode,
        ssa,
        memory_plan,
        memory_inventory,
        bytecode_links,
        profile: ledger.profile().identity(),
    })
}

pub(super) fn checked_memory_inventory(
    ssa: &lkjscript_ir::VerifiedProgram,
) -> Result<SsaMemoryInventory> {
    let inventory = derive_memory_inventory(ssa);
    verify_memory_inventory(ssa, &inventory).map_err(|error| Error::msg(error.to_string()))?;
    Ok(inventory)
}

pub(super) fn finish<T>(result: Result<T>, ledger: &mut BudgetLedger) -> Result<T> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => match crate::budget::reserve_diagnostic(ledger) {
            Ok(()) => Err(error),
            Err(_) if error.budget_error().is_some() => Err(error),
            Err(diagnostic_error) => Err(diagnostic_error),
        },
    }
}
