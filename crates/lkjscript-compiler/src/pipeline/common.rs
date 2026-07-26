use lkjscript_core::{validate_chunk, BudgetLedger, Limits, Result};

use crate::codegen::compile_program;
use crate::ssa::lower_program_with_budget;
use crate::ExecutableProgram;

pub(super) fn compile_analyzed(
    analyzed: &crate::hir::Program,
    limits: &Limits,
    ledger: &mut BudgetLedger,
) -> Result<ExecutableProgram> {
    let ssa = lower_program_with_budget(analyzed, ledger)?;
    let (chunk, bytecode_links) = compile_program(&ssa)?;
    let bytecode = validate_chunk(chunk, &limits.validation)?;
    Ok(ExecutableProgram {
        bytecode,
        ssa,
        bytecode_links,
        profile: ledger.profile().identity(),
    })
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
