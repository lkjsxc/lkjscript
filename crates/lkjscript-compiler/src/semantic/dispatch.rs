use crate::semantic::codec::error;
use crate::semantic::schema::{
    AnalysisLevel, ApplyMode, Charges, DiagnosticsResult, OperationRequest, ProtocolError,
    ProtocolErrorCode, ResponseResult, TransactionResult,
};
use crate::semantic::{operations, transaction};
use crate::source::ValidatedSourceTree;

pub(crate) struct DispatchResult {
    pub result: ResponseResult,
    pub publication: Option<transaction::StagedTransaction>,
}

pub(crate) fn dispatch(
    tree: &ValidatedSourceTree,
    operation: OperationRequest,
    charges: &mut Charges,
    limits: super::charges::ProtocolLimits,
) -> Result<DispatchResult, ProtocolError> {
    let mut publication = None;
    let result = match operation {
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
            ResponseResult::Snapshot {
                snapshot: Box::new(operations::snapshot::build(tree)),
            }
        }
        OperationRequest::ReadEntity {
            revision,
            declaration_key,
            entity_fingerprint,
        } => {
            check_revision(tree, &revision)?;
            ResponseResult::ReadEntity {
                entity: Box::new(operations::entity::read(
                    tree,
                    &declaration_key,
                    entity_fingerprint.as_deref(),
                )?),
            }
        }
        OperationRequest::QueryNode { revision, node } => {
            check_revision(tree, &revision)?;
            ResponseResult::QueryNode {
                query: Box::new(operations::query::query(tree, node)?),
            }
        }
        OperationRequest::Diagnostics { revision, analysis } => {
            check_revision(tree, &revision)?;
            let diagnostics =
                operations::diagnostics::collect(tree, matches!(analysis, AnalysisLevel::Hir));
            add_work(charges, diagnostics.len())?;
            ResponseResult::Diagnostics {
                result: Box::new(DiagnosticsResult {
                    complete: true,
                    diagnostics,
                }),
            }
        }
        OperationRequest::ApplyTransaction {
            mode,
            base_revision,
            file_preconditions,
            operations,
        } => {
            check_revision(tree, &base_revision)?;
            let staged = transaction::stage(tree, &operations, &file_preconditions)?;
            add_staged_source(charges, &staged.tree)?;
            add_work(charges, staged.tree.nodes().len())?;
            add_work(charges, staged.sources.len())?;
            let result = ResponseResult::ApplyTransaction {
                transaction: Box::new(TransactionResult {
                    mode: if mode == ApplyMode::Publish {
                        "publish"
                    } else {
                        "preview"
                    }
                    .to_string(),
                    base_revision,
                    new_revision: staged.tree.revision().to_hex(),
                    changed_sources: staged.changes.clone(),
                    semantic_diff: staged.identities.clone(),
                    identities: staged.identities.clone(),
                    diagnostics: Vec::new(),
                }),
            };
            if mode == ApplyMode::Publish {
                publication = Some(staged);
            }
            result
        }
    };
    limits.check_charges(charges)?;
    Ok(DispatchResult {
        result,
        publication,
    })
}

fn add_staged_source(
    charges: &mut Charges,
    tree: &ValidatedSourceTree,
) -> Result<(), ProtocolError> {
    let bytes = tree
        .files()
        .iter()
        .try_fold(0u64, |total, file| total.checked_add(file.exact_source_len));
    let bytes = bytes.ok_or_else(work_overflow)?;
    let units = u64::try_from(tree.files().len()).map_err(|_| work_overflow())?;
    let nodes = u64::try_from(tree.nodes().len()).map_err(|_| work_overflow())?;
    charges.source_bytes = charges
        .source_bytes
        .checked_add(bytes)
        .ok_or_else(work_overflow)?;
    charges.source_units = charges
        .source_units
        .checked_add(units)
        .ok_or_else(work_overflow)?;
    charges.source_nodes = charges
        .source_nodes
        .checked_add(nodes)
        .ok_or_else(work_overflow)?;
    Ok(())
}

fn add_work(charges: &mut Charges, increment: usize) -> Result<(), ProtocolError> {
    let increment = u64::try_from(increment).map_err(|_| work_overflow())?;
    charges.work_units = charges
        .work_units
        .checked_add(increment)
        .ok_or_else(work_overflow)?;
    Ok(())
}

fn work_overflow() -> ProtocolError {
    error(
        ProtocolErrorCode::ResourceLimit,
        "protocol aggregate charge overflow",
    )
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
