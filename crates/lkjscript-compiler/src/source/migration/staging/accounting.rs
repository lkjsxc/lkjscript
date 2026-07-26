use lkjscript_core::{
    BudgetAuthority, BudgetCause, BudgetLedger, ResourceCategory, ResourceProfile,
};

use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceResult, SourceSpan, ValidatedSourceTree,
};

use super::rewriting::{insertion, CONVERSION_CLOSE, CONVERSION_OPEN};
use super::ConversionInsertion;

pub(in crate::source::migration) fn reserve(
    tree: &ValidatedSourceTree,
    conversions: &[ConversionInsertion],
    profile: ResourceProfile,
) -> SourceResult<()> {
    let (bytes, nodes) = measure(tree, conversions)?;
    let operation_count = u64::try_from(tree.files().len()).map_err(|_| overflow(tree))?;
    let mut ledger = BudgetLedger::new(profile);
    let mut request = ledger.scope(BudgetAuthority::SemanticRequest);
    let mut transaction = request
        .child(BudgetAuthority::Transaction)
        .map_err(|error| budget(tree, error))?;
    for (category, amount) in [
        (ResourceCategory::Transactions, 1),
        (ResourceCategory::TransactionOperations, operation_count),
        (ResourceCategory::TransactionImpactNodes, nodes),
        (ResourceCategory::StagedPublicationNodes, nodes),
        (ResourceCategory::StagedPublicationBytes, bytes),
    ] {
        transaction
            .reserve(category, amount, BudgetCause::Request)
            .map_err(|error| budget(tree, error))?
            .commit();
    }
    Ok(())
}

fn measure(
    tree: &ValidatedSourceTree,
    conversions: &[ConversionInsertion],
) -> SourceResult<(u64, u64)> {
    let conversion_bytes = u64::try_from(CONVERSION_OPEN.len() + CONVERSION_CLOSE.len())
        .map_err(|_| overflow(tree))?;
    let mut bytes = 0_u64;
    for (index, file) in tree.files().iter().enumerate() {
        let inserted = u64::try_from(insertion(file).1.len()).map_err(|_| overflow(tree))?;
        let count = u64::try_from(conversions.iter().filter(|site| site.file == index).count())
            .map_err(|_| overflow(tree))?;
        bytes = bytes
            .checked_add(file.exact_source_len)
            .and_then(|value| value.checked_add(inserted))
            .and_then(|value| value.checked_add(count.checked_mul(conversion_bytes)?))
            .ok_or_else(|| overflow(tree))?;
    }
    let marker_nodes = u64::try_from(tree.files().len())
        .map_err(|_| overflow(tree))?
        .checked_mul(2)
        .ok_or_else(|| overflow(tree))?;
    let conversion_nodes = u64::try_from(conversions.len()).map_err(|_| overflow(tree))?;
    let nodes = u64::try_from(tree.nodes().len())
        .map_err(|_| overflow(tree))?
        .checked_add(marker_nodes)
        .and_then(|value| value.checked_add(conversion_nodes))
        .ok_or_else(|| overflow(tree))?;
    Ok((bytes, nodes))
}

fn budget(tree: &ValidatedSourceTree, error: lkjscript_core::BudgetError) -> SourceDiagnostic {
    diagnostic(tree, "LKJ-SRC-MIGRATION-LIMIT", error.to_string())
}

fn overflow(tree: &ValidatedSourceTree) -> SourceDiagnostic {
    diagnostic(
        tree,
        "LKJ-SRC-MIGRATION-LIMIT",
        "migration staging charge overflow",
    )
}

fn diagnostic(
    tree: &ValidatedSourceTree,
    code: &'static str,
    message: impl Into<String>,
) -> SourceDiagnostic {
    SourceDiagnostic::new(
        code,
        DiagnosticCategory::ResourceLimit,
        message,
        tree.root_origin().clone(),
        SourceSpan::zero(),
    )
}
