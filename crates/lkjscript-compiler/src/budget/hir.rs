use lkjscript_core::{BudgetAuthority, BudgetLedger, Error, ResourceCategory, Result};

use super::{
    checked_add, count_usize,
    hir_expression::measure_expressions,
    reserve,
    type_charge::{measure_type, TypeCharges},
};
use crate::hir::Program;

#[derive(Default)]
pub(super) struct HirCharges {
    pub(super) functions: u64,
    pub(super) expressions: u64,
    pub(super) product_fields: u64,
    pub(super) traits: u64,
    pub(super) ownership_expressions: u64,
    pub(super) ownership_retained: u64,
    pub(super) types: TypeCharges,
}

/// Measure immutable ownership-checked HIR and reserve its exact charged input
/// shape before effects or SSA construction allocate their target state.
pub(crate) fn reserve_ssa_input(program: &Program, ledger: &mut BudgetLedger) -> Result<()> {
    let charges = measure(program)?;
    for (authority, category, amount) in [
        (
            BudgetAuthority::SsaConstruction,
            ResourceCategory::HirFunctions,
            charges.functions,
        ),
        (
            BudgetAuthority::SsaConstruction,
            ResourceCategory::HirExpressions,
            charges.expressions,
        ),
        (
            BudgetAuthority::TypeAnalysis,
            ResourceCategory::ProductFields,
            charges.product_fields,
        ),
        (
            BudgetAuthority::TypeAnalysis,
            ResourceCategory::TypeNesting,
            charges.types.nesting,
        ),
        (
            BudgetAuthority::TypeAnalysis,
            ResourceCategory::TypeWork,
            charges.types.work,
        ),
        (
            BudgetAuthority::TraitAnalysis,
            ResourceCategory::TraitWork,
            charges.traits,
        ),
        (
            BudgetAuthority::OwnershipAnalysis,
            ResourceCategory::OwnershipExpressions,
            charges.ownership_expressions,
        ),
        (
            BudgetAuthority::OwnershipAnalysis,
            ResourceCategory::OwnershipRetainedState,
            charges.ownership_retained,
        ),
    ] {
        reserve(ledger, authority, category, amount)?;
    }
    Ok(())
}

fn measure(program: &Program) -> Result<HirCharges> {
    let mut charges = HirCharges {
        functions: count_usize(ResourceCategory::HirFunctions, program.functions.len())?
            .checked_add(1)
            .ok_or_else(|| Error::msg("hir_functions count overflow"))?,
        traits: count_usize(ResourceCategory::TraitWork, program.traits.len())?,
        ..HirCharges::default()
    };
    checked_add(
        &mut charges.traits,
        count_usize(ResourceCategory::TraitWork, program.implementations.len())?,
        ResourceCategory::TraitWork,
    )?;
    for product in &program.products {
        checked_add(
            &mut charges.product_fields,
            count_usize(ResourceCategory::ProductFields, product.fields.len())?,
            ResourceCategory::ProductFields,
        )?;
        for field in &product.fields {
            measure_type(&field.ty, &mut charges.types)?;
        }
    }
    for field in program.enums.iter().flat_map(|definition| {
        definition
            .variants
            .iter()
            .flat_map(|variant| &variant.fields)
    }) {
        measure_type(&field.ty, &mut charges.types)?;
    }
    for binding in &program.bindings {
        measure_type(&binding.ty, &mut charges.types)?;
    }
    measure_type(&program.main.return_type, &mut charges.types)?;
    checked_add(
        &mut charges.ownership_retained,
        u64::from(program.main.local_count),
        ResourceCategory::OwnershipRetainedState,
    )?;
    measure_expressions(program, &mut charges)?;
    Ok(charges)
}
