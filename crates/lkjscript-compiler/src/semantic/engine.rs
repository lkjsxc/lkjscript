use std::path::Path;

use lkjscript_core::Limits;

use crate::semantic::codec::{self, error, MAX_SCHEMA_NODES, MAX_WORK_UNITS};
use crate::semantic::operations;
use crate::semantic::schema::{
    Charges, OperationRequest, ProtocolError, ProtocolErrorCode, Request, Response, ResponseResult,
};
use crate::semantic::{SCHEMA, VERSION};

pub(crate) fn execute_request(
    request: Request,
    request_bytes: usize,
) -> Result<Vec<u8>, ProtocolError> {
    let profile = request.profile;
    let loaded = crate::source::load(Path::new(&request.root), &Limits::default());
    let tree = match loaded {
        Ok(tree) => tree,
        Err(failure) => {
            let diagnostic = operations::diagnostics::source_failure(&failure).map(Box::new);
            return codec::encode_response(Response {
                schema: SCHEMA.to_string(),
                version: VERSION,
                compiler_build: compiler_build(),
                profile,
                revision: None,
                charges: Charges {
                    request_bytes: request_bytes as u64,
                    ..Charges::default()
                },
                result: ResponseResult::Error {
                    error: Box::new(error(ProtocolErrorCode::SourceLoad, failure.render_human())),
                    diagnostic,
                },
            });
        }
    };
    let mut charges = charges(&tree, request_bytes, &request.operation);
    if let Err(failure) = check_charges(&charges) {
        return encode_error(
            profile,
            Some(tree.revision().to_hex()),
            charges,
            failure,
            None,
        );
    }
    let revision = tree.revision().to_hex();
    match super::dispatch::dispatch(&tree, request.operation, &mut charges) {
        Ok(result) => {
            let response_revision = match &result {
                ResponseResult::ApplyTransaction { transaction } => {
                    transaction.new_revision.clone()
                }
                _ => revision,
            };
            let encoded = codec::encode_response(Response {
                schema: SCHEMA.to_string(),
                version: VERSION,
                compiler_build: compiler_build(),
                profile,
                revision: Some(response_revision.clone()),
                charges: charges.clone(),
                result,
            });
            encoded.or_else(|failure| {
                encode_error(profile, Some(response_revision), charges, failure, None)
            })
        }
        Err(failure) => {
            let diagnostic = if matches!(
                failure.code,
                ProtocolErrorCode::StaleRevision | ProtocolErrorCode::PreconditionFailed
            ) {
                Some(operations::diagnostics::stale(
                    tree.root_origin().logical_path(),
                    &failure.message,
                ))
            } else {
                None
            };
            encode_error(profile, Some(revision), charges, failure, diagnostic)
        }
    }
}

pub(super) fn check_charges(charges: &Charges) -> Result<(), ProtocolError> {
    if charges.source_nodes > MAX_SCHEMA_NODES || charges.work_units > MAX_WORK_UNITS {
        return Err(error(
            ProtocolErrorCode::ResourceLimit,
            "loaded source closure exceeds the standard protocol profile",
        ));
    }
    Ok(())
}

fn charges(
    tree: &crate::source::ValidatedSourceTree,
    bytes: usize,
    operation: &OperationRequest,
) -> Charges {
    let source_bytes = tree.files().iter().map(|file| file.exact_source_len).sum();
    let operations = match operation {
        OperationRequest::ApplyTransaction { operations, .. } => operations.len() as u64,
        _ => 0,
    };
    let source_nodes = tree.nodes().len() as u64;
    Charges {
        request_bytes: bytes as u64,
        source_bytes,
        source_units: tree.files().len() as u64,
        source_nodes,
        operations,
        work_units: source_nodes.saturating_add(operations.saturating_mul(16)),
        output_bytes: 0,
    }
}

fn encode_error(
    profile: crate::semantic::schema::ResourceProfile,
    revision: Option<String>,
    charges: Charges,
    failure: ProtocolError,
    diagnostic: Option<crate::semantic::schema::DiagnosticRecord>,
) -> Result<Vec<u8>, ProtocolError> {
    codec::encode_response(Response {
        schema: SCHEMA.to_string(),
        version: VERSION,
        compiler_build: compiler_build(),
        profile,
        revision,
        charges,
        result: ResponseResult::Error {
            error: Box::new(failure),
            diagnostic: diagnostic.map(Box::new),
        },
    })
}

fn compiler_build() -> String {
    format!("lkjscript-compiler-{}", env!("CARGO_PKG_VERSION"))
}
