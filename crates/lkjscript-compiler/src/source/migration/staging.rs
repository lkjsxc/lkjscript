use lkjscript_core::{
    BudgetAuthority, BudgetCause, BudgetLedger, ResourceCategory, ResourceProfile,
};

use crate::source::{
    DiagnosticCategory, SourceDiagnostic, SourceFile, SourceResult, SourceSpan, ValidatedSourceTree,
};

use super::super::edition::EDITION_MARKER;

pub(super) fn reserve(tree: &ValidatedSourceTree, profile: ResourceProfile) -> SourceResult<()> {
    let (bytes, nodes) = measure(tree)?;
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

pub(super) fn insertion(file: &SourceFile) -> (usize, &'static str) {
    let offset = file.syntax.first().map_or(file.exact_source.len(), |node| {
        node.span.start().byte() as usize
    });
    if offset > 0 && file.exact_source.as_bytes().get(offset - 1) != Some(&b'\n') {
        (offset, "\nedition/\n2\n/edition\n")
    } else {
        (offset, EDITION_MARKER)
    }
}

fn measure(tree: &ValidatedSourceTree) -> SourceResult<(u64, u64)> {
    let mut bytes = 0_u64;
    for file in tree.files() {
        let inserted = u64::try_from(insertion(file).1.len()).map_err(|_| overflow(tree))?;
        bytes = bytes
            .checked_add(file.exact_source_len)
            .and_then(|value| value.checked_add(inserted))
            .ok_or_else(|| overflow(tree))?;
    }
    let added = u64::try_from(tree.files().len())
        .map_err(|_| overflow(tree))?
        .checked_mul(2)
        .ok_or_else(|| overflow(tree))?;
    let nodes = u64::try_from(tree.nodes().len())
        .map_err(|_| overflow(tree))?
        .checked_add(added)
        .ok_or_else(|| overflow(tree))?;
    Ok((bytes, nodes))
}

fn budget(tree: &ValidatedSourceTree, error: lkjscript_core::BudgetError) -> SourceDiagnostic {
    diagnostic(tree, error.to_string())
}

fn overflow(tree: &ValidatedSourceTree) -> SourceDiagnostic {
    diagnostic(tree, "migration staging charge overflow")
}

fn diagnostic(tree: &ValidatedSourceTree, message: impl Into<String>) -> SourceDiagnostic {
    SourceDiagnostic::new(
        "LKJ-SRC-MIGRATION-LIMIT",
        DiagnosticCategory::ResourceLimit,
        message,
        tree.root_origin().clone(),
        SourceSpan::zero(),
    )
}
