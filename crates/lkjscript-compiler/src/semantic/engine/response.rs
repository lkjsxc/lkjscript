use crate::semantic::codec::{self, error};
use crate::semantic::schema::{
    Charges, ProtocolError, ProtocolErrorCode, Response, ResponseResult,
};
use crate::semantic::{CONTRACT, SCHEMA};

use super::EngineOutcome;

pub(super) fn prepare(
    response: Response,
    publication: Option<crate::semantic::transaction::StagedTransaction>,
    guard: Option<crate::semantic::transaction::PublicationGuard>,
    response_limit: usize,
) -> Result<EngineOutcome, ProtocolError> {
    let prepared = codec::prepare_response(response, response_limit)?;
    Ok(EngineOutcome {
        prepared,
        publication,
        guard,
    })
}

pub(super) fn error_response(
    revision: Option<String>,
    charges: Charges,
    failure: ProtocolError,
) -> Response {
    Response {
        result: ResponseResult::Error {
            error: Box::new(failure),
            diagnostic: None,
        },
        ..base_response(revision, charges)
    }
}

pub(super) fn base_response(revision: Option<String>, charges: Charges) -> Response {
    Response {
        schema: SCHEMA.to_string(),
        contract: CONTRACT.to_hex(),
        compiler_build: format!("lkjscript-compiler-{}", env!("CARGO_PKG_VERSION")),
        revision,
        charges,
        result: ResponseResult::Error {
            error: Box::new(error(
                ProtocolErrorCode::ValidationFailed,
                "uninitialized response",
            )),
            diagnostic: None,
        },
    }
}
