#![allow(clippy::unwrap_used)]

use super::*;

fn nested_charge(ledger: &mut BudgetLedger, amount: u64) -> Result<(), ResourceDiagnostic> {
    ledger.charge(ResourceCategory::ParserWork, amount)
}

#[test]
fn zero_exact_limit_and_limit_plus_one_are_checked() {
    let category = ResourceCategory::ParserWork;
    let zero = ResourceProfile::default().lowered(category, 0).unwrap();
    let mut zero_ledger = BudgetLedger::new(zero);
    zero_ledger.charge(category, 0).unwrap();
    let zero_error = zero_ledger.charge(category, 1).unwrap_err();
    assert_eq!(zero_error.before, 0);
    assert_eq!(zero_error.increment, 1);
    assert_eq!(zero_error.limit, 0);

    let limit = ResourceProfile::default().ceilings().limit(category);
    let mut exact = BudgetLedger::default();
    exact.charge(category, limit).unwrap();
    assert_eq!(exact.used(category), limit);
    let plus_one = exact.charge(category, 1).unwrap_err();
    assert_eq!(plus_one.before, limit);
    assert_eq!(plus_one.increment, 1);
    assert_eq!(exact.used(category), limit);
}

#[test]
fn arithmetic_overflow_is_rejected_before_mutation() {
    let category = ResourceCategory::SsaValues;
    let mut ledger = BudgetLedger::default();
    ledger.used[category.index()] = u64::MAX;
    let error = ledger.charge(category, 1).unwrap_err();
    assert_eq!(error.before, u64::MAX);
    assert_eq!(error.increment, 1);
    assert_eq!(ledger.used(category), u64::MAX);
}

#[test]
fn nested_operations_share_usage_without_reset() {
    let mut ledger = BudgetLedger::default();
    nested_charge(&mut ledger, 2).unwrap();
    nested_charge(&mut ledger, 3).unwrap();
    assert_eq!(ledger.used(ResourceCategory::ParserWork), 5);
    assert_eq!(
        ledger.usage().used(ResourceCategory::ParserWork),
        ledger.used(ResourceCategory::ParserWork)
    );
}

#[test]
fn diagnostics_are_structured_and_deterministic() {
    let category = ResourceCategory::ProtocolResponseBytes;
    let profile = ResourceProfile::new(crate::ResourceProfileName::Deterministic)
        .lowered(category, 4)
        .unwrap();
    let mut first = BudgetLedger::new(profile);
    let mut second = BudgetLedger::new(profile);
    first.charge(category, 3).unwrap();
    second.charge(category, 3).unwrap();
    let first_error = first.charge(category, 2).unwrap_err();
    let second_error = second.charge(category, 2).unwrap_err();
    assert_eq!(first_error, second_error);
    assert_eq!(first_error.category, category);
    assert_eq!(first_error.limit, 4);
    assert_eq!(first_error.before, 3);
    assert_eq!(first_error.increment, 2);
    assert_eq!(first_error.to_string(), second_error.to_string());
}
