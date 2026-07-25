use crate::semantic::codec::error;
use crate::semantic::schema::{
    AnalysisLevel, ApplyMode, Charges, DiagnosticsResult, OperationRequest, ProtocolError,
    ProtocolErrorCode, ResponseResult, TransactionResult,
};
use crate::semantic::{operations, transaction};
use crate::source::ValidatedSourceTree;

pub(crate) fn dispatch(
    tree: &ValidatedSourceTree,
    operation: OperationRequest,
    charges: &mut Charges,
) -> Result<ResponseResult, ProtocolError> {
    match operation {
        OperationRequest::Snapshot {
            expected_repository_identity,
        } => {
            if expected_repository_identity
                .is_some_and(|expected| expected != tree.revision().to_hex())
            {
                return Err(error(
                    ProtocolErrorCode::StaleRevision,
                    "repository identity does not match",
                ));
            }
            Ok(ResponseResult::Snapshot {
                snapshot: Box::new(operations::snapshot::build(tree)),
            })
        }
        OperationRequest::ReadEntity {
            revision,
            declaration_key,
            entity_fingerprint,
        } => {
            check_revision(tree, &revision)?;
            Ok(ResponseResult::ReadEntity {
                entity: Box::new(operations::entity::read(
                    tree,
                    &declaration_key,
                    entity_fingerprint.as_deref(),
                )?),
            })
        }
        OperationRequest::QueryNode { revision, node } => {
            check_revision(tree, &revision)?;
            Ok(ResponseResult::QueryNode {
                query: Box::new(operations::query::query(tree, node)?),
            })
        }
        OperationRequest::Diagnostics { revision, analysis } => {
            check_revision(tree, &revision)?;
            let diagnostics =
                operations::diagnostics::collect(tree, matches!(analysis, AnalysisLevel::Hir));
            charges.work_units = charges.work_units.saturating_add(diagnostics.len() as u64);
            Ok(ResponseResult::Diagnostics {
                result: Box::new(DiagnosticsResult {
                    complete: true,
                    diagnostics,
                }),
            })
        }
        OperationRequest::ApplyTransaction {
            mode,
            base_revision,
            file_preconditions,
            operations,
        } => {
            check_revision(tree, &base_revision)?;
            let staged = transaction::stage(tree, &operations, &file_preconditions)?;
            charges.work_units = charges
                .work_units
                .saturating_add(staged.tree.nodes().len() as u64)
                .saturating_add(staged.sources.len() as u64);
            super::engine::check_charges(charges)?;
            if mode == ApplyMode::Publish {
                transaction::publish(&staged, tree.root_path())?;
            }
            Ok(ResponseResult::ApplyTransaction {
                transaction: Box::new(TransactionResult {
                    mode: if mode == ApplyMode::Publish {
                        "publish"
                    } else {
                        "preview"
                    }
                    .to_string(),
                    base_revision,
                    new_revision: staged.tree.revision().to_hex(),
                    changed_sources: staged.changes,
                    semantic_diff: staged.identities.clone(),
                    identities: staged.identities,
                    diagnostics: Vec::new(),
                }),
            })
        }
    }
}

fn check_revision(tree: &ValidatedSourceTree, revision: &str) -> Result<(), ProtocolError> {
    if revision == tree.revision().to_hex() {
        Ok(())
    } else {
        Err(error(
            ProtocolErrorCode::StaleRevision,
            format!("base revision {revision:?} is stale"),
        ))
    }
}
