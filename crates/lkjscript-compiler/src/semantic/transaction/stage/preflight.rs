use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetLedger, ResourceCategory};

use crate::semantic::schema::{
    ExpressionCounts, ProtocolError, ProtocolErrorCode, TransactionOperation,
};

pub(super) fn reserve(
    tree: &crate::source::ValidatedSourceTree,
    operations: &[TransactionOperation],
    ledger: &mut BudgetLedger,
) -> Result<(), ProtocolError> {
    let mut request = ledger.scope(BudgetAuthority::SemanticRequest);
    let mut transaction = request
        .child(BudgetAuthority::Transaction)
        .map_err(failure)?;
    transaction
        .reserve(ResourceCategory::Transactions, 1, BudgetCause::Request)
        .map_err(failure)?
        .commit();
    let operation_count = u64::try_from(operations.len()).map_err(|_| overflow())?;
    transaction
        .reserve(
            ResourceCategory::TransactionOperations,
            operation_count,
            BudgetCause::Request,
        )
        .map_err(failure)?
        .commit();
    let current_nodes = u64::try_from(tree.nodes().len()).map_err(|_| overflow())?;
    let (added_nodes, added_bytes) = additions(operations, current_nodes)?;
    let impact = current_nodes
        .checked_add(added_nodes)
        .ok_or_else(overflow)?;
    transaction
        .reserve(
            ResourceCategory::TransactionImpactNodes,
            impact,
            BudgetCause::Request,
        )
        .map_err(failure)?
        .commit();
    transaction
        .reserve(
            ResourceCategory::StagedPublicationNodes,
            impact,
            BudgetCause::Request,
        )
        .map_err(failure)?
        .commit();
    let source_bytes = tree
        .files()
        .iter()
        .try_fold(0_u64, |total, file| {
            total.checked_add(file.exact_source_len)
        })
        .ok_or_else(overflow)?;
    let staged_bytes = source_bytes.checked_add(added_bytes).ok_or_else(overflow)?;
    transaction
        .reserve(
            ResourceCategory::StagedPublicationBytes,
            staged_bytes,
            BudgetCause::Request,
        )
        .map_err(failure)?
        .commit();
    Ok(())
}

fn additions(
    operations: &[TransactionOperation],
    current_nodes: u64,
) -> Result<(u64, u64), ProtocolError> {
    let mut nodes = 0_u64;
    let mut bytes = 0_u64;
    for operation in operations {
        let mut counts = ExpressionCounts::default();
        match operation {
            TransactionOperation::ReplaceExpression { expression, .. }
            | TransactionOperation::FillHole { expression, .. } => {
                expression.measure(1, &mut counts)
            }
            TransactionOperation::InsertHole {
                hole_identity,
                goal,
                ..
            }
            | TransactionOperation::RefineHole {
                hole_identity,
                goal,
                ..
            } => {
                counts.nodes = 5;
                counts.string_bytes = u64::try_from(
                    hole_identity
                        .len()
                        .saturating_add(goal.as_ref().map_or(0, String::len)),
                )
                .map_err(|_| overflow())?;
            }
            TransactionOperation::RenameDeclaration { new_name, .. } => {
                let name_bytes = u64::try_from(new_name.len()).map_err(|_| overflow())?;
                counts.string_bytes = name_bytes.checked_mul(current_nodes).ok_or_else(overflow)?;
            }
            TransactionOperation::DeleteHole { .. } => {}
        }
        nodes = nodes.checked_add(counts.nodes).ok_or_else(overflow)?;
        let structural = counts.nodes.checked_mul(32).ok_or_else(overflow)?;
        bytes = bytes
            .checked_add(structural)
            .and_then(|value| value.checked_add(counts.string_bytes))
            .ok_or_else(overflow)?;
    }
    Ok((nodes, bytes))
}

fn failure(error: lkjscript_core::BudgetError) -> ProtocolError {
    crate::semantic::codec::budget_error(error)
}

fn overflow() -> ProtocolError {
    crate::semantic::codec::error(
        ProtocolErrorCode::ResourceLimit,
        "transaction pre-allocation charge overflow",
    )
}
