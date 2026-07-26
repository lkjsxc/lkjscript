mod hir;
mod hir_expression;
mod source;
mod source_match;
mod ssa;
mod type_charge;

use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetLedger, Error, ResourceCategory, Result};

pub(crate) use hir::reserve_ssa_input;
pub(crate) use source::reserve_source_shape;
pub(crate) use ssa::reserve_bytecode_input;

pub(crate) fn reserve_diagnostic(ledger: &mut BudgetLedger) -> Result<()> {
    reserve(
        ledger,
        BudgetAuthority::Diagnostics,
        ResourceCategory::Diagnostics,
        1,
    )
}

fn reserve(
    ledger: &mut BudgetLedger,
    authority: BudgetAuthority,
    category: ResourceCategory,
    amount: u64,
) -> Result<()> {
    if amount == 0 {
        return Ok(());
    }
    ledger
        .charge_with_authority(Some(authority), category, amount, BudgetCause::Request)
        .map_err(Error::budget)
}

fn count_usize(category: ResourceCategory, amount: usize) -> Result<u64> {
    u64::try_from(amount)
        .map_err(|_| Error::msg(format!("{} count exceeds u64", category.as_str())))
}

fn checked_add(total: &mut u64, amount: u64, category: ResourceCategory) -> Result<()> {
    *total = total
        .checked_add(amount)
        .ok_or_else(|| Error::msg(format!("{} count overflow", category.as_str())))?;
    Ok(())
}
