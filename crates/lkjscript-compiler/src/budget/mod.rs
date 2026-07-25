mod hir;
mod source;
mod ssa;

use lkjscript_core::{BudgetLedger, Error, ResourceCategory, Result};

pub(crate) use hir::charge_hir;
pub(crate) use source::charge_source;
pub(crate) use ssa::charge_ssa;

fn charge(ledger: &mut BudgetLedger, category: ResourceCategory, amount: u64) -> Result<()> {
    ledger
        .charge(category, amount)
        .map_err(Error::compiler_resource)
}

fn charge_usize(
    ledger: &mut BudgetLedger,
    category: ResourceCategory,
    amount: usize,
) -> Result<()> {
    let amount = u64::try_from(amount)
        .map_err(|_| Error::msg(format!("{} count exceeds u64", category.as_str())))?;
    charge(ledger, category, amount)
}
