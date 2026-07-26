use lkjscript_core::{
    BudgetAuthority, BudgetCause, BudgetLedger, Error, ResourceCategory, ResourceDiagnostic, Result,
};

use crate::source::{Expr, ValidatedSourceTree};

use super::{charge, charge_usize};

/// Exact post-parse accounting. Edition 1 and Foundation V1 bounds protect the
/// parser allocations; these aggregate checks run before HIR allocation.
pub(crate) fn charge_source(tree: &ValidatedSourceTree, ledger: &mut BudgetLedger) -> Result<()> {
    for file in tree.files() {
        charge(ledger, ResourceCategory::SourceBytes, file.exact_source_len)?;
        charge(ledger, ResourceCategory::SourceUnits, 1)?;
        charge(ledger, ResourceCategory::PathWork, 1)?;
        charge_usize(ledger, ResourceCategory::Tokens, file.tokens.len())?;
        charge_usize(ledger, ResourceCategory::ParserWork, file.tokens.len())?;
        for form in &file.forms {
            if matches!(form, Expr::Call { name, .. } if name == "import") {
                charge(ledger, ResourceCategory::ImportEdges, 1)?;
                charge(ledger, ResourceCategory::PathWork, 1)?;
            }
        }
    }
    charge_usize(ledger, ResourceCategory::SchemaNodes, tree.nodes().len())?;
    charge_usize(ledger, ResourceCategory::ParserWork, tree.nodes().len())?;
    charge_usize(
        ledger,
        ResourceCategory::TopLevelDeclarations,
        tree.declarations().len(),
    )?;
    let (enums, variants, fields) = enum_shape_counts(tree)?;
    // These exact source-shape reservations complete before any enum HIR
    // declaration, variant, field, or recursion-validation state is allocated.
    reserve_enum(ledger, ResourceCategory::EnumDeclarations, enums)?;
    reserve_enum(ledger, ResourceCategory::EnumVariants, variants)?;
    reserve_enum(ledger, ResourceCategory::VariantFields, fields)?;
    let recursion = if enums == 0 {
        0
    } else {
        u64::try_from(crate::hir::ENUM_RECURSION_MAX_WORK)
            .map_err(|_| Error::msg("enum recursion preallocation count overflow"))?
    };
    reserve_enum(ledger, ResourceCategory::EnumRecursionWork, recursion)?;
    charge_usize(ledger, ResourceCategory::ValidationWork, tree.nodes().len())?;
    charge_usize(
        ledger,
        ResourceCategory::ValidationWork,
        tree.declarations().len(),
    )
}

fn reserve_enum(ledger: &mut BudgetLedger, category: ResourceCategory, amount: u64) -> Result<()> {
    ledger
        .charge_with_authority(
            Some(BudgetAuthority::Hir),
            category,
            amount,
            BudgetCause::Request,
        )
        .map_err(|error| {
            Error::compiler_resource(ResourceDiagnostic {
                profile: error.profile,
                category: error.category,
                limit: error.limit,
                before: error.observed,
                increment: error.attempted,
            })
        })
}

fn enum_shape_counts(tree: &ValidatedSourceTree) -> Result<(u64, u64, u64)> {
    let mut declarations = 0_u64;
    let mut variants = 0_u64;
    let mut fields = 0_u64;
    for file in tree.files() {
        for form in &file.forms {
            let Expr::Call { name, args } = form else {
                continue;
            };
            if name != "enum" {
                continue;
            }
            declarations = declarations
                .checked_add(1)
                .ok_or_else(|| lkjscript_core::Error::msg("enum declaration count overflow"))?;
            let Some(Expr::Call { name, args: items }) = args.last() else {
                continue;
            };
            if name != "variants" {
                continue;
            }
            variants =
                variants
                    .checked_add(u64::try_from(items.len()).map_err(|_| {
                        lkjscript_core::Error::msg("enum variant count exceeds u64")
                    })?)
                    .ok_or_else(|| lkjscript_core::Error::msg("enum variant count overflow"))?;
            for item in items {
                let Expr::Call { name, args } = item else {
                    continue;
                };
                if name != "variant" {
                    continue;
                }
                let Some(Expr::Call {
                    name,
                    args: members,
                }) = args.get(1)
                else {
                    continue;
                };
                if name != "fields" {
                    continue;
                }
                fields = fields
                    .checked_add(u64::try_from(members.len()).map_err(|_| {
                        lkjscript_core::Error::msg("variant field count exceeds u64")
                    })?)
                    .ok_or_else(|| lkjscript_core::Error::msg("variant field count overflow"))?;
            }
        }
    }
    Ok((declarations, variants, fields))
}
