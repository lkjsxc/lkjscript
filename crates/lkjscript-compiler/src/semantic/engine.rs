use std::path::Path;

use lkjscript_core::Limits;

use crate::semantic::codec::{self, error};
use crate::semantic::operations;
use crate::semantic::schema::{
    Charges, ProtocolError, ProtocolErrorCode, Request, Response, ResponseResult,
};
use crate::semantic::{SCHEMA, VERSION};

pub(crate) fn execute_request(
    request: Request,
    request_bytes: usize,
) -> Result<Vec<u8>, ProtocolError> {
    let profile = request.profile;
    let protocol_limits = super::charges::ProtocolLimits::for_profile(profile);
    let root = Path::new(&request.root);
    let request_charge = Charges {
        request_bytes: u64::try_from(request_bytes).unwrap_or(u64::MAX),
        ..Charges::default()
    };
    let _publication_guard = match super::transaction::begin(root) {
        Ok(guard) => guard,
        Err(failure) => return encode_error(profile, None, request_charge, failure, None),
    };
    let loaded = crate::source::load_for_protocol(
        root,
        &Limits::default(),
        protocol_limits.source_bytes,
        protocol_limits.source_units,
    );
    let tree = match loaded {
        Ok(tree) => tree,
        Err(failure) => {
            let diagnostic = operations::diagnostics::source_failure(&failure).map(Box::new);
            return codec::encode_response(Response {
                schema: SCHEMA.to_string(),
                version: VERSION,
                compiler_build: compiler_build(),
                profile,
                profile_identity: super::charges::identity(profile),
                limits: protocol_limits.record(),
                revision: None,
                charges: request_charge,
                result: ResponseResult::Error {
                    error: Box::new(error(ProtocolErrorCode::SourceLoad, failure.render_human())),
                    diagnostic,
                },
            });
        }
    };
    if let Err(message) = operations::holes::validate::source_holes(&tree) {
        return encode_error(
            profile,
            Some(tree.revision().to_hex()),
            request_charge,
            error(ProtocolErrorCode::ValidationFailed, message),
            None,
        );
    }
    let mut charges = match super::charges::measure(&tree, request_bytes, &request.operation) {
        Ok(charges) => charges,
        Err(failure) => {
            return encode_error(
                profile,
                Some(tree.revision().to_hex()),
                request_charge,
                failure,
                None,
            )
        }
    };
    if let Err(failure) = protocol_limits.check_charges(&charges) {
        return encode_error(
            profile,
            Some(tree.revision().to_hex()),
            charges,
            failure,
            None,
        );
    }
    let revision = tree.revision().to_hex();
    match super::dispatch::dispatch(
        &tree,
        request.operation,
        &mut charges,
        protocol_limits,
        profile,
    ) {
        Ok(dispatched) => {
            let response_revision = match &dispatched.result {
                ResponseResult::ApplyTransaction { transaction } => {
                    transaction.new_revision.clone()
                }
                _ => revision.clone(),
            };
            let encoded = codec::encode_response(Response {
                schema: SCHEMA.to_string(),
                version: VERSION,
                compiler_build: compiler_build(),
                profile,
                profile_identity: super::charges::identity(profile),
                limits: protocol_limits.record(),
                revision: Some(response_revision.clone()),
                charges: charges.clone(),
                result: dispatched.result,
            });
            let encoded = match encoded {
                Ok(bytes) => bytes,
                Err(failure) => {
                    return encode_error(profile, Some(response_revision), charges, failure, None)
                }
            };
            if let Some(publication) = dispatched.publication {
                if let Err(failure) = super::transaction::publish(&publication, root) {
                    return encode_error(profile, Some(revision), charges, failure, None);
                }
            }
            Ok(encoded)
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
            encode_error(profile, Some(revision), charges, failure, diagnostic)
        }
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
        profile_identity: super::charges::identity(profile),
        limits: super::charges::ProtocolLimits::for_profile(profile).record(),
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
