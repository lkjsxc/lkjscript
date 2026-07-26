use crate::source::{Expr, ValidatedSourceTree};
use lkjscript_core::{BudgetAuthority, BudgetLedger, Error, ResourceCategory, Result};

use super::{checked_add, count_usize, reserve};

#[derive(Default)]
struct SourceCharges {
    bytes: u64,
    units: u64,
    imports: u64,
    tokens: u64,
    nodes: u64,
    declarations: u64,
    parser_work: u64,
    validation_work: u64,
    path_work: u64,
}

/// Measure the immutable validated tree after fixed-limit parsing, then reserve
/// its aggregate shape before any HIR construction.
pub(crate) fn reserve_source_shape(
    tree: &ValidatedSourceTree,
    ledger: &mut BudgetLedger,
) -> Result<()> {
    let charges = measure(tree)?;
    for (authority, category, amount) in [
        (
            BudgetAuthority::SourceLoading,
            ResourceCategory::SourceBytes,
            charges.bytes,
        ),
        (
            BudgetAuthority::SourceLoading,
            ResourceCategory::SourceUnits,
            charges.units,
        ),
        (
            BudgetAuthority::SourceLoading,
            ResourceCategory::ImportEdges,
            charges.imports,
        ),
        (
            BudgetAuthority::Parsing,
            ResourceCategory::Tokens,
            charges.tokens,
        ),
        (
            BudgetAuthority::SchemaValidation,
            ResourceCategory::SchemaNodes,
            charges.nodes,
        ),
        (
            BudgetAuthority::SchemaValidation,
            ResourceCategory::TopLevelDeclarations,
            charges.declarations,
        ),
        (
            BudgetAuthority::Parsing,
            ResourceCategory::ParserWork,
            charges.parser_work,
        ),
        (
            BudgetAuthority::SchemaValidation,
            ResourceCategory::ValidationWork,
            charges.validation_work,
        ),
        (
            BudgetAuthority::SourceLoading,
            ResourceCategory::PathWork,
            charges.path_work,
        ),
    ] {
        reserve(ledger, authority, category, amount)?;
    }
    super::source_match::reserve_matches(tree, ledger)?;
    let (enums, variants, fields) = enum_shape_counts(tree)?;
    reserve(
        ledger,
        BudgetAuthority::Hir,
        ResourceCategory::EnumDeclarations,
        enums,
    )?;
    reserve(
        ledger,
        BudgetAuthority::Hir,
        ResourceCategory::EnumVariants,
        variants,
    )?;
    reserve(
        ledger,
        BudgetAuthority::Hir,
        ResourceCategory::VariantFields,
        fields,
    )?;
    let recursion = if enums == 0 {
        0
    } else {
        count_usize(
            ResourceCategory::EnumRecursionWork,
            crate::hir::ENUM_RECURSION_MAX_WORK,
        )?
    };
    reserve(
        ledger,
        BudgetAuthority::Hir,
        ResourceCategory::EnumRecursionWork,
        recursion,
    )
}

fn measure(tree: &ValidatedSourceTree) -> Result<SourceCharges> {
    let mut charges = SourceCharges::default();
    for file in tree.files() {
        checked_add(
            &mut charges.bytes,
            file.exact_source_len,
            ResourceCategory::SourceBytes,
        )?;
        checked_add(&mut charges.units, 1, ResourceCategory::SourceUnits)?;
        checked_add(&mut charges.path_work, 1, ResourceCategory::PathWork)?;
        let tokens = count_usize(ResourceCategory::Tokens, file.tokens.len())?;
        checked_add(&mut charges.tokens, tokens, ResourceCategory::Tokens)?;
        checked_add(
            &mut charges.parser_work,
            tokens,
            ResourceCategory::ParserWork,
        )?;
        for form in &file.forms {
            if matches!(form, Expr::Call { name, .. } if name == "import") {
                checked_add(&mut charges.imports, 1, ResourceCategory::ImportEdges)?;
                checked_add(&mut charges.path_work, 1, ResourceCategory::PathWork)?;
            }
        }
    }
    charges.nodes = count_usize(ResourceCategory::SchemaNodes, tree.nodes().len())?;
    charges.declarations = count_usize(
        ResourceCategory::TopLevelDeclarations,
        tree.declarations().len(),
    )?;
    checked_add(
        &mut charges.parser_work,
        charges.nodes,
        ResourceCategory::ParserWork,
    )?;
    charges.validation_work = charges
        .nodes
        .checked_add(charges.declarations)
        .ok_or_else(|| Error::msg("validation_work count overflow"))?;
    Ok(charges)
}

fn enum_shape_counts(tree: &ValidatedSourceTree) -> Result<(u64, u64, u64)> {
    let mut counts = (0_u64, 0_u64, 0_u64);
    for form in tree.files().iter().flat_map(|file| &file.forms) {
        let Expr::Call { name, args } = form else {
            continue;
        };
        if name != "enum" {
            continue;
        }
        counts.0 = counts
            .0
            .checked_add(1)
            .ok_or_else(|| Error::msg("enum declaration count overflow"))?;
        let Some(Expr::Call { name, args: items }) = args.last() else {
            continue;
        };
        if name != "variants" {
            continue;
        }
        checked_add(
            &mut counts.1,
            count_usize(ResourceCategory::EnumVariants, items.len())?,
            ResourceCategory::EnumVariants,
        )?;
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
            if name == "fields" {
                checked_add(
                    &mut counts.2,
                    count_usize(ResourceCategory::VariantFields, members.len())?,
                    ResourceCategory::VariantFields,
                )?;
            }
        }
    }
    Ok(counts)
}
