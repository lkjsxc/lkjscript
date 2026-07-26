use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetError, BudgetLedger, ResourceCategory};

#[allow(
    clippy::result_large_err,
    reason = "budget rejection must preserve its fixed nonallocating journal prefix"
)]
pub(crate) fn reserve(
    ledger: &mut BudgetLedger,
    authority: BudgetAuthority,
    category: ResourceCategory,
    amount: u64,
    cause: BudgetCause,
) -> Result<(), BudgetError> {
    let mut request = ledger.scope(BudgetAuthority::SemanticRequest);
    request
        .child(authority)?
        .reserve(category, amount, cause)?
        .commit();
    Ok(())
}

pub(crate) fn profile_matches(
    selected: crate::semantic::schema::ResourceProfile,
    ledger: &BudgetLedger,
) -> bool {
    selected.core().name() == ledger.profile().name()
}
