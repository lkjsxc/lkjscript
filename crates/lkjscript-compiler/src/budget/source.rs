use lkjscript_core::{BudgetLedger, ResourceCategory, Result};

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
    charge_usize(ledger, ResourceCategory::ValidationWork, tree.nodes().len())?;
    charge_usize(
        ledger,
        ResourceCategory::ValidationWork,
        tree.declarations().len(),
    )
}
