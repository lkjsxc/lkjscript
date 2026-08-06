mod response;

use std::path::Path;

use crate::semantic::codec::{error, PreparedResponse};
use crate::semantic::operations;
use crate::semantic::schema::{
    Charges, ProtocolError, ProtocolErrorCode, Request, Response, ResponseResult,
};
use response::{base_response, error_response, prepare};

pub(crate) struct EngineOutcome {
    pub prepared: PreparedResponse,
    pub publication: Option<super::transaction::StagedTransaction>,
    pub guard: Option<super::transaction::PublicationGuard>,
}

pub(crate) fn execute_request(
    request: Request,
    request_bytes: usize,
) -> Result<EngineOutcome, ProtocolError> {
    let policy = super::charges::BoundaryPolicy::default();
    let source_byte_policy = crate::source::SourceBytePolicy::limited(policy.source_bytes);
    let root = Path::new(&request.root);
    let request_charge = Charges {
        request_bytes: u64::try_from(request_bytes).unwrap_or(u64::MAX),
        ..Charges::default()
    };
    let guard = match super::transaction::begin(root) {
        Ok(guard) => guard,
        Err(failure) => return prepare(error_response(None, request_charge, failure), None, None),
    };
    let tree = match crate::source::load_for_protocol(root, source_byte_policy) {
        Ok(tree) => tree,
        Err(failure) => {
            let diagnostic = operations::diagnostics::source_failure(&failure).map(Box::new);
            let code = if failure.category() == crate::source::DiagnosticCategory::ResourceLimit {
                ProtocolErrorCode::ResourceLimit
            } else {
                ProtocolErrorCode::SourceLoad
            };
            let response = Response {
                result: ResponseResult::Error {
                    error: Box::new(error(code, failure.render_human())),
                    diagnostic,
                },
                ..base_response(None, request_charge)
            };
            return prepare(response, None, guard);
        }
    };
    if let Err(message) = operations::holes::validate::source_holes(&tree) {
        let response = error_response(
            Some(tree.revision().to_hex()),
            request_charge,
            error(ProtocolErrorCode::ValidationFailed, message),
        );
        return prepare(response, None, guard);
    }
    let mut charges = match super::charges::measure(&tree, request_bytes, &request.operation) {
        Ok(charges) => charges,
        Err(failure) => {
            let response = error_response(Some(tree.revision().to_hex()), request_charge, failure);
            return prepare(response, None, guard);
        }
    };
    let revision = tree.revision().to_hex();
    match super::dispatch::dispatch(&tree, request.operation, &mut charges, source_byte_policy) {
        Ok(dispatched) => {
            let response_revision = match &dispatched.result {
                ResponseResult::ApplyTransaction { transaction } => {
                    transaction.new_revision.clone()
                }
                _ => revision,
            };
            let response = Response {
                result: dispatched.result,
                ..base_response(Some(response_revision), charges)
            };
            prepare(response, dispatched.publication, guard)
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
            let mut response = error_response(Some(revision), charges, failure);
            if let ResponseResult::Error {
                diagnostic: slot, ..
            } = &mut response.result
            {
                *slot = diagnostic.map(Box::new);
            }
            prepare(response, None, guard)
        }
    }
}
