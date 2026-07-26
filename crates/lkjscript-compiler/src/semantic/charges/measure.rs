use crate::semantic::codec::error;
use crate::semantic::schema::{Charges, OperationRequest, ProtocolError, ProtocolErrorCode};

pub(crate) fn measure(
    tree: &crate::source::ValidatedSourceTree,
    bytes: usize,
    operation: &OperationRequest,
) -> Result<Charges, ProtocolError> {
    let source_bytes = tree
        .files()
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.exact_source_len))
        .ok_or_else(overflow)?;
    let operations = match operation {
        OperationRequest::ApplyTransaction { operations, .. } => {
            u64::try_from(operations.len()).map_err(|_| overflow())?
        }
        _ => 0,
    };
    let hole_count = crate::semantic::tree::source_nodes(tree)
        .iter()
        .filter(
            |node| matches!(&node.kind, crate::source::SyntaxKind::Call { name } if name == "hole"),
        )
        .count();
    let hole_count = u64::try_from(hole_count).map_err(|_| overflow())?;
    let request_bytes = u64::try_from(bytes).map_err(|_| overflow())?;
    let source_units = u64::try_from(tree.files().len()).map_err(|_| overflow())?;
    let source_nodes = u64::try_from(tree.nodes().len()).map_err(|_| overflow())?;
    let traversal_multiplier = match operation {
        OperationRequest::ApplyTransaction { .. } => {
            operations.checked_add(4).ok_or_else(overflow)?
        }
        OperationRequest::Diagnostics {
            analysis: crate::semantic::schema::AnalysisLevel::Hir,
            ..
        } => 3,
        _ => 1,
    };
    let traversal_work = source_nodes
        .checked_mul(traversal_multiplier)
        .ok_or_else(overflow)?;
    let operation_work = operations.checked_mul(16).ok_or_else(overflow)?;
    let work_units = traversal_work
        .checked_add(operation_work)
        .ok_or_else(overflow)?;
    Ok(Charges {
        request_bytes,
        source_bytes,
        source_units,
        source_nodes,
        operations,
        work_units,
        hole_count,
        transactions: u64::from(operations > 0),
        transaction_operations: operations,
        hole_candidates: 0,
        hole_search_work: 0,
        legal_actions: 0,
        transaction_impact_nodes: 0,
        staged_publication_bytes: 0,
        staged_publication_nodes: 0,
        output_bytes: 0,
    })
}

fn overflow() -> ProtocolError {
    error(
        ProtocolErrorCode::ResourceLimit,
        "protocol aggregate charge overflow",
    )
}
