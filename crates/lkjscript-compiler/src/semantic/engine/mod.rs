mod response;

use std::path::Path;

use lkjscript_core::{BudgetAuthority, BudgetCause, BudgetLedger, Limits, ResourceCategory};

use crate::semantic::codec::{budget_error, error, PreparedResponse};
use crate::semantic::operations;
use crate::semantic::schema::{
    Charges, ProtocolError, ProtocolErrorCode, Request, Response, ResponseResult,
};
use response::{base_response, error_response, prepare};

pub(crate) struct EngineOutcome {
    pub prepared: PreparedResponse,
    pub publication: Option<super::transaction::StagedTransaction>,
    pub guard: Option<super::transaction::PublicationGuard>,
    pub budget: Option<Box<lkjscript_core::BudgetError>>,
}

pub(crate) fn execute_request_with_ledger(
    request: Request,
    request_bytes: usize,
    ledger: &mut BudgetLedger,
) -> Result<EngineOutcome, ProtocolError> {
    let profile = request.profile;
    let limits = super::charges::ProtocolLimits::for_core(ledger.profile());
    let root = Path::new(&request.root);
    let request_charge = Charges {
        request_bytes: u64::try_from(request_bytes).unwrap_or(u64::MAX),
        ..Charges::default()
    };
    let guard = match super::transaction::begin(root) {
        Ok(guard) => guard,
        Err(failure) => {
            return prepare(
                error_response(profile, None, request_charge, failure, ledger),
                None,
                None,
                ledger,
            )
        }
    };
    let tree = match crate::source::load_for_protocol(
        root,
        &Limits::default(),
        limits.source_bytes,
        limits.source_units,
    ) {
        Ok(tree) => tree,
        Err(failure) => {
            let diagnostic = operations::diagnostics::source_failure(&failure).map(Box::new);
            let response = Response {
                result: ResponseResult::Error {
                    error: Box::new(error(ProtocolErrorCode::SourceLoad, failure.render_human())),
                    diagnostic,
                },
                ..base_response(profile, None, request_charge, ledger)
            };
            return prepare(response, None, guard, ledger);
        }
    };
    if let Err(message) = operations::holes::validate::source_holes(&tree) {
        let response = error_response(
            profile,
            Some(tree.revision().to_hex()),
            request_charge,
            error(ProtocolErrorCode::ValidationFailed, message),
            ledger,
        );
        return prepare(response, None, guard, ledger);
    }
    let mut charges = match super::charges::measure(&tree, request_bytes, &request.operation) {
        Ok(charges) => charges,
        Err(failure) => {
            let response = error_response(
                profile,
                Some(tree.revision().to_hex()),
                request_charge,
                failure,
                ledger,
            );
            return prepare(response, None, guard, ledger);
        }
    };
    if let Err(failure) = reserve_tree(&tree, &charges, ledger) {
        return prepare(
            error_response(
                profile,
                Some(tree.revision().to_hex()),
                charges,
                budget_error(failure),
                ledger,
            ),
            None,
            guard,
            ledger,
        );
    }
    if let Err(failure) = limits.check_charges(&charges) {
        return prepare(
            error_response(
                profile,
                Some(tree.revision().to_hex()),
                charges,
                failure,
                ledger,
            ),
            None,
            guard,
            ledger,
        );
    }
    let revision = tree.revision().to_hex();
    match super::dispatch::dispatch(&tree, request.operation, &mut charges, limits, ledger) {
        Ok(dispatched) => {
            let response_revision = match &dispatched.result {
                ResponseResult::ApplyTransaction { transaction } => {
                    transaction.new_revision.clone()
                }
                _ => revision,
            };
            let response = Response {
                result: dispatched.result,
                ..base_response(profile, Some(response_revision), charges, ledger)
            };
            prepare(response, dispatched.publication, guard, ledger)
        }
        Err(failure) => {
            let diagnostic = failure.diagnostic.as_deref().cloned().or_else(|| {
                matches!(
                    failure.code,
                    ProtocolErrorCode::StaleRevision | ProtocolErrorCode::PreconditionFailed
                )
                .then(|| {
                    operations::diagnostics::stale(
                        tree.root_origin().logical_path(),
                        &failure.message,
                    )
                })
            });
            let mut response = error_response(profile, Some(revision), charges, failure, ledger);
            if let ResponseResult::Error {
                diagnostic: slot, ..
            } = &mut response.result
            {
                *slot = diagnostic.map(Box::new);
            }
            prepare(response, None, guard, ledger)
        }
    }
}

#[allow(
    clippy::result_large_err,
    reason = "tree preflight preserves the fixed nonallocating budget prefix"
)]
fn reserve_tree(
    tree: &crate::source::ValidatedSourceTree,
    charges: &Charges,
    ledger: &mut BudgetLedger,
) -> Result<(), lkjscript_core::BudgetError> {
    for (category, amount) in [
        (ResourceCategory::SourceBytes, charges.source_bytes),
        (ResourceCategory::SourceUnits, charges.source_units),
        (ResourceCategory::SchemaNodes, charges.source_nodes),
        (ResourceCategory::ValidationWork, charges.work_units),
    ] {
        super::budget::reserve(
            ledger,
            if category == ResourceCategory::ValidationWork {
                BudgetAuthority::SchemaValidation
            } else {
                BudgetAuthority::SourceLoading
            },
            category,
            amount,
            BudgetCause::SemanticNode(u64::try_from(tree.nodes().len()).unwrap_or(u64::MAX)),
        )?;
    }
    Ok(())
}
