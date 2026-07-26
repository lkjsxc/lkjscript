use super::*;
use crate::{
    compile_source_with_ledger, BudgetAuthority, BudgetLedger, ResourceCategory, ResourceProfile,
};

fn phase_error(
    category: ResourceCategory,
    exact: u64,
    authority: BudgetAuthority,
) -> lkjscript_core::Error {
    let source = unit_main("");
    let exact_profile = ResourceProfile::default().lowered(category, exact).unwrap();
    compile_source_with_profile(
        &source,
        "phase-exact.lkjscript",
        &Limits::default(),
        exact_profile,
    )
    .unwrap();

    let plus_one_profile = ResourceProfile::default()
        .lowered(category, exact - 1)
        .unwrap();
    let compile = || {
        compile_source_with_profile(
            &source,
            "phase-plus-one.lkjscript",
            &Limits::default(),
            plus_one_profile,
        )
        .unwrap_err()
    };
    let first = compile();
    let second = compile();
    assert_eq!(first, second);
    let rejection = first.budget_error().unwrap();
    assert_eq!(rejection.category, category);
    assert_eq!(rejection.authority, Some(authority));
    assert_eq!(rejection.limit, exact - 1);
    assert_eq!(rejection.observed, 0);
    assert_eq!(rejection.attempted, exact);
    assert!(!rejection.allocated_before_rejection);
    assert_eq!(rejection.prefix(), second.budget_error().unwrap().prefix());
    first
}

#[test]
fn ssa_construction_preflights_exact_hir_input_and_plus_one() {
    let error = phase_error(
        ResourceCategory::HirFunctions,
        1,
        BudgetAuthority::SsaConstruction,
    );
    let prefix = error.budget_error().unwrap().prefix();
    assert!(prefix.committed(ResourceCategory::SourceBytes) > 0);
}

#[test]
fn bytecode_construction_preflights_exact_ssa_input_and_plus_one() {
    let error = phase_error(ResourceCategory::SsaFunctions, 1, BudgetAuthority::Bytecode);
    let prefix = error.budget_error().unwrap().prefix();
    assert_eq!(prefix.committed(ResourceCategory::HirFunctions), 1);
}

#[test]
fn budget_rejection_prefix_survives_diagnostic_exhaustion() {
    let profile = ResourceProfile::default()
        .lowered(ResourceCategory::HirFunctions, 0)
        .unwrap()
        .lowered(ResourceCategory::Diagnostics, 0)
        .unwrap();
    let error = compile_source_with_profile(
        &unit_main(""),
        "preserved.lkjscript",
        &Limits::default(),
        profile,
    )
    .unwrap_err();
    let rejection = error.budget_error().unwrap();
    assert_eq!(rejection.category, ResourceCategory::HirFunctions);
    assert_eq!(rejection.authority, Some(BudgetAuthority::SsaConstruction));
}

#[test]
fn public_compile_and_validate_apis_accumulate_one_outer_ledger() {
    let source = canonical_source(&unit_main(""));
    let profile = ResourceProfile::default()
        .lowered(ResourceCategory::SourceUnits, 2)
        .unwrap();
    let run = || {
        let mut ledger = BudgetLedger::new(profile);
        crate::validate_source_with_ledger(
            &source,
            "outer.lkjscript",
            &Limits::default(),
            &mut ledger,
        )
        .unwrap();
        let program =
            compile_source_with_ledger(&source, "outer.lkjscript", &Limits::default(), &mut ledger)
                .unwrap();
        assert_eq!(program.profile(), profile.identity());
        assert_eq!(ledger.used(ResourceCategory::SourceUnits), 2);
        let error = crate::validate_source_with_ledger(
            &source,
            "outer.lkjscript",
            &Limits::default(),
            &mut ledger,
        )
        .unwrap_err();
        assert_eq!(ledger.used(ResourceCategory::SourceUnits), 2);
        error
    };
    let error = run();
    let repeated = run();
    assert_eq!(error, repeated);
    let rejection = error.budget_error().unwrap();
    assert_eq!(rejection.authority, Some(BudgetAuthority::SourceLoading));
    assert_eq!(rejection.limit, 0);
    assert_eq!(rejection.attempted, 1);
    assert_eq!(
        rejection.prefix().committed(ResourceCategory::SourceUnits),
        2
    );
    assert_eq!(
        rejection.prefix(),
        repeated.budget_error().unwrap().prefix()
    );
}
